//! HTTP server, routing, background refresh, and shutdown handling.

use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use tokio::{
    net::TcpListener,
    signal,
    time::{interval, Duration},
};
use tracing::{debug, error, info};

use crate::cache_update::update_cache;
use crate::config::{Config, DEFAULT_CACHE_TTL};
use crate::handlers::{
    config_handler, details_handler, doc_handler, health_handler, html_config_handler,
    html_dashboard_handler, html_details_handler, html_docs_handler, html_health_handler,
    html_index_handler, html_subgroups_handler, metrics_handler, root_handler, subgroups_handler,
};
use crate::state::SharedState;

pub async fn run(
    state: SharedState,
    config: Config,
    bind_ip: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let background_task = spawn_cache_refresh(state.clone());
    let addr: SocketAddr = format!("{}:{}", bind_ip, port).parse()?;
    let app = build_router(&config).with_state(state.clone());
    let shutdown_signal = shutdown_signal();

    spawn_reload_handler(state.clone());

    if config.enable_tls.unwrap_or(false) {
        serve_tls(app, &config, addr, bind_ip, port, shutdown_signal).await?;
    } else {
        serve_plain(app, addr, bind_ip, port, shutdown_signal).await?;
    }

    background_task.abort();
    let _ = background_task.await;

    if let Err(e) = state.ringbuffer_manager.flush() {
        error!("Failed to flush persistent database: {}", e);
    }

    info!("herakles-node-exporter stopped gracefully");
    Ok(())
}

fn spawn_cache_refresh(state: SharedState) -> tokio::task::JoinHandle<()> {
    let ttl = Duration::from_secs(state.config().cache_ttl.unwrap_or(DEFAULT_CACHE_TTL));

    tokio::spawn(async move {
        let mut int = interval(ttl);
        debug!(
            "Background cache update task started with {}s interval",
            ttl.as_secs()
        );

        loop {
            int.tick().await;
            debug!("Starting scheduled cache update");
            if let Err(e) = update_cache(&state).await {
                error!("Scheduled cache update failed: {}", e);
            } else {
                debug!("Scheduled cache update completed");
            }
        }
    })
}

fn build_router(config: &Config) -> Router<SharedState> {
    let mut app = Router::new()
        .route("/", get(root_handler))
        .route("/metrics", get(metrics_handler));

    if config.enable_health.unwrap_or(true) {
        app = app.route("/health", get(health_handler));
    }

    app = app
        .route("/config", get(config_handler))
        .route("/subgroups", get(subgroups_handler))
        .route("/doc", get(doc_handler))
        .route("/docs", get(html_docs_handler))
        .route("/details", get(details_handler))
        .route("/html", get(html_index_handler))
        .route("/html/", get(html_index_handler))
        .route("/html/dashboard", get(html_dashboard_handler))
        .route("/html/details", get(html_details_handler))
        .route("/html/subgroups", get(html_subgroups_handler))
        .route("/html/health", get(html_health_handler))
        .route("/html/config", get(html_config_handler))
        .route("/html/docs", get(html_docs_handler));

    if config.enable_pprof.unwrap_or(false) {
        debug!("Debug endpoints enabled at /debug/pprof");
    }

    app
}

async fn serve_tls(
    app: Router,
    config: &Config,
    addr: SocketAddr,
    bind_ip: &str,
    port: u16,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cert_path = config
        .tls_cert_path
        .as_ref()
        .expect("tls_cert_path should be set when enable_tls is true (validated at startup)");
    let key_path = config
        .tls_key_path
        .as_ref()
        .expect("tls_key_path should be set when enable_tls is true (validated at startup)");

    info!("Loading TLS certificate from: {}", cert_path);
    info!("Loading TLS private key from: {}", key_path);

    let tls_config = RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .map_err(|e| {
            error!("Failed to load TLS configuration: {}", e);
            e
        })?;

    info!(
        "herakles-node-exporter listening on https://{}:{}",
        bind_ip, port
    );

    let server = axum_server::bind_rustls(addr, tls_config).serve(app.into_make_service());

    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Server error: {}", e);
                return Err(e.into());
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, exiting...");
        }
    }

    Ok(())
}

async fn serve_plain(
    app: Router,
    addr: SocketAddr,
    bind_ip: &str,
    port: u16,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    info!(
        "herakles-node-exporter listening on http://{}:{}",
        bind_ip, port
    );

    let server = axum::serve(listener, app);

    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Server error: {}", e);
                return Err(e.into());
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, exiting...");
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C), shutting down gracefully...");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully...");
        }
    }
}

#[cfg(unix)]
fn spawn_reload_handler(state: SharedState) {
    tokio::spawn(async move {
        let mut stream = match signal::unix::signal(signal::unix::SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to install SIGHUP handler: {}", e);
                return;
            }
        };
        info!("SIGHUP signal handler installed (for config/subgroup reloading)");
        while stream.recv().await.is_some() {
            info!("SIGHUP received, reloading configuration and subgroups...");

            crate::process::reload_subgroups();

            match state.reload_config() {
                Ok(_) => info!("Configuration and subgroups reloaded successfully."),
                Err(e) => error!("Failed to reload configuration: {}", e),
            }
        }
    });
}

#[cfg(not(unix))]
fn spawn_reload_handler(_state: SharedState) {}
