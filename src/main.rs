//! herakles-node-exporter - version 0.1.0
//!
//! Professional memory metrics exporter with tracing logging.
//! This is the main entry point that initializes the server and handles subcommands.

mod cache;
mod cache_updater;
mod cli;
mod collectors;
mod commands;
mod config;
mod ebpf;
mod handlers;
mod health_stats;
mod metrics;
mod process;
mod ringbuffer;
mod ringbuffer_manager;
mod startup_checks;
mod state;
mod system;

use ahash::AHashMap as HashMap;
use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use herakles_node_exporter::{AppConfig as HealthAppConfig, BufferHealthConfig, HealthState};
use prometheus::{Gauge, Registry};
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Instant;
use tokio::{
    net::TcpListener,
    signal,
    sync::RwLock,
};
use tracing::{debug, error, info, warn, Level};
use nix::unistd::{geteuid, setgid, setgroups, setuid, Group, User};

use cache::MetricsCache;
use cli::{Args, Commands, LogLevel};
use commands::{
    command_check, command_config, command_generate_testdata, command_install, command_subgroups,
    command_test, command_uninstall,
};
use config::{
    resolve_config, show_config, validate_effective_config, Config, DEFAULT_BIND_ADDR, DEFAULT_PORT,
};
use handlers::{
    config_handler, details_handler, doc_handler, health_handler, html_config_handler,
    html_details_handler, html_docs_handler, html_health_handler, html_index_handler,
    html_subgroups_handler, metrics_handler, root_handler, subgroups_handler,
};
use health_stats::HealthStats;
use metrics::MemoryMetrics;
use process::{BufferConfig, SUBGROUPS};
use ringbuffer_manager::RingbufferManager;
use state::{AppState, SharedState};
use system::CpuStatsCache;

/// Initializes tracing logging subsystem with configured log level.
fn setup_logging(_config: &Config, args: &Args) {
    let log_level = match args.log_level {
        LogLevel::Off => Level::ERROR,
        LogLevel::Error => Level::ERROR,
        LogLevel::Warn => Level::WARN,
        LogLevel::Info => Level::INFO,
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Trace => Level::TRACE,
    };

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("Logging initialized with level: {:?}", args.log_level);
}

/// Resolve effective buffer sizes (CLI > config > defaults).
fn resolve_buffer_config(cfg: &Config, args: &Args) -> BufferConfig {
    let io_kb = args
        .io_buffer_kb
        .unwrap_or_else(|| cfg.io_buffer_kb.unwrap_or(256));
    let smaps_kb = args
        .smaps_buffer_kb
        .unwrap_or_else(|| cfg.smaps_buffer_kb.unwrap_or(512));
    let smaps_rollup_kb = args
        .smaps_rollup_buffer_kb
        .unwrap_or_else(|| cfg.smaps_rollup_buffer_kb.unwrap_or(256));

    BufferConfig {
        io_kb,
        smaps_kb,
        smaps_rollup_kb,
    }
}

/// Helper function to load and validate configuration.
/// Exits the process with error code 1 if validation fails.
fn load_validated_config(args: &Args) -> Result<Config, Box<dyn std::error::Error>> {
    let config = resolve_config(args)?;
    if let Err(e) = validate_effective_config(&config) {
        eprintln!("❌ Configuration invalid: {}", e);
        std::process::exit(1);
    }
    Ok(config)
}

/// Wrapper function to call cache updater from background task.
async fn update_cache(state: &SharedState) -> Result<(), Box<dyn std::error::Error>> {
    cache_updater::update_cache(state).await
}

/// Drop privileges from root to the herakles user after eBPF initialization.
///
/// IMPORTANT: This should only happen if:
/// 1. We are running as root
/// 2. The herakles user exists
/// 3. We have verified /proc access before dropping
///
/// For production monitoring, it's recommended to run as root.
fn drop_privileges() {
    // Check if running as root
    if !geteuid().is_root() {
        debug!("Not running as root, skipping privilege drop");
        return;
    }

    // Lookup herakles user and group
    let user = match User::from_name("herakles") {
        Ok(Some(u)) => u,
        Ok(None) => {
            info!("ℹ️  User 'herakles' not found - continuing as root");
            info!("   This is the recommended configuration for full system monitoring");
            return;
        }
        Err(e) => {
            debug!("Failed to lookup user 'herakles': {} - continuing as root", e);
            return;
        }
    };

    let group = match Group::from_name("herakles") {
        Ok(Some(g)) => g,
        Ok(None) => {
            warn!("Group 'herakles' not found, continuing as root");
            return;
        }
        Err(e) => {
            warn!(
                "Failed to lookup group 'herakles': {} - continuing as root",
                e
            );
            return;
        }
    };

    // Warn before dropping - this will reduce monitoring capabilities
    warn!("⚠️  User 'herakles' found - attempting privilege drop");
    warn!("   Note: This will prevent monitoring of root-owned processes!");
    warn!("   Recommendation: Remove 'herakles' user or run as root for full monitoring");

    // Drop privileges: clear supplementary groups, then group, then user
    if let Err(e) = setgroups(&[group.gid]) {
        warn!(
            "Failed to clear supplementary groups: {} - continuing as root",
            e
        );
        return;
    }

    if let Err(e) = setgid(group.gid) {
        warn!(
            "Failed to drop group privileges to gid={}: {} - continuing as root",
            group.gid, e
        );
        return;
    }

    if let Err(e) = setuid(user.uid) {
        warn!(
            "Failed to drop user privileges to uid={}: {} - continuing as root",
            user.uid, e
        );
        return;
    }

    info!(
        "✅ Privileges dropped to user '{}' (uid={}, gid={})",
        user.name, user.uid, group.gid
    );
    
    // Verify /proc access after drop using metadata check
    if let Err(e) = std::fs::metadata("/proc/1/smaps_rollup") {
        error!("❌ After privilege drop: Cannot access /proc/1/smaps_rollup: {}", e);
        error!("   Only user-owned processes will be monitored!");
        error!("   Recommendation: Reinstall without 'herakles' user or run as root");
    }
}

/// Main application entry point.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Early config resolution for show/check modes
    if args.show_config || args.show_user_config || args.check_config {
        let config = resolve_config(&args)?;

        if args.check_config {
            if let Err(e) = validate_effective_config(&config) {
                eprintln!("❌ Configuration invalid: {}", e);
                std::process::exit(1);
            }
            println!("✅ Configuration is valid");
            return Ok(());
        }

        if args.show_config {
            return show_config(&config, args.config_format, false);
        }

        if args.show_user_config {
            return show_config(&config, args.config_format, true);
        }
    }

    // Handle subcommands
    if let Some(command) = &args.command {
        // Install, Uninstall, and CheckRequirements commands don't need config validation
        match command {
            Commands::Install { no_service, force } => {
                return command_install(*no_service, *force);
            }
            Commands::Uninstall { yes } => {
                return command_uninstall(*yes);
            }
            Commands::CheckRequirements { ebpf } => {
                println!("🔍 Checking Runtime Requirements");
                println!("================================\n");
                
                match startup_checks::validate_requirements(*ebpf) {
                    Ok(_) => {
                        println!("\n✅ All requirements met - ready for production!");
                        std::process::exit(0);
                    }
                    Err(e) => {
                        eprintln!("\n❌ Requirements check failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                // Other commands need config validation
            }
        }

        let config = load_validated_config(&args)?;

        return match command {
            Commands::Check { memory, proc, all } => command_check(*memory, *proc, *all, &config),

            Commands::Config {
                output,
                format,
                commented,
            } => command_config(output.clone(), format.clone(), *commented),

            Commands::Test {
                iterations,
                verbose,
                format,
            } => command_test(*iterations, *verbose, format.clone(), &config),

            Commands::Subgroups { verbose, group } => command_subgroups(*verbose, group.clone()),

            Commands::GenerateTestdata {
                output,
                min_per_subgroup,
                others_count,
            } => {
                command_generate_testdata(output.clone(), *min_per_subgroup, *others_count, &config)
            }

            Commands::Install { .. } => unreachable!("Install handled above"),
            Commands::Uninstall { .. } => unreachable!("Uninstall handled above"),
            Commands::CheckRequirements { .. } => unreachable!("CheckRequirements handled above"),
        };
    }

    // Load configuration for main server mode
    let config = resolve_config(&args)?;

    if let Err(e) = validate_effective_config(&config) {
        eprintln!("❌ Configuration invalid: {}", e);
        std::process::exit(1);
    }

    setup_logging(&config, &args);

    info!("Starting herakles-node-exporter");

    // NEW: Validate runtime requirements BEFORE proceeding
    let enable_ebpf = config.enable_ebpf.unwrap_or(false);
    if let Err(e) = startup_checks::validate_requirements(enable_ebpf) {
        error!("❌ Startup validation failed: {}", e);
        error!("   The exporter will start but may not function correctly!");
        // Continue anyway - don't fail hard
    }

    let bind_ip_str = config.bind.as_deref().unwrap_or(DEFAULT_BIND_ADDR);
    let port = config.port.unwrap_or(DEFAULT_PORT);

    // Configure parallel processing
    if let Some(threads) = config.parallelism {
        if threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build_global()
                .unwrap_or_else(|e| error!("Failed to set rayon thread pool: {}", e));
            debug!("Rayon thread pool configured with {} threads", threads);
        }
    }

    let buffer_config = resolve_buffer_config(&config, &args);

    // Initialize Prometheus metrics registry
    let registry = Registry::new();
    debug!("Prometheus registry initialized");

    let metrics = MemoryMetrics::new(&registry)?;
    let scrape_duration = Gauge::new(
        "herakles_exporter_scrape_duration_seconds",
        "Time spent serving /metrics request (reading from cache)",
    )?;
    let processes_total = Gauge::new(
        "herakles_exporter_processes_total",
        "Number of processes currently exported by herakles-node-exporter",
    )?;
    let cache_update_duration = Gauge::new(
        "herakles_exporter_cache_update_duration_seconds",
        "Time spent updating the process metrics cache in background",
    )?;
    let cache_update_success = Gauge::new(
        "herakles_exporter_cache_update_success",
        "Whether the last cache update was successful (1) or failed (0)",
    )?;
    let cache_updating = Gauge::new(
        "herakles_exporter_cache_updating",
        "Whether cache update is currently in progress (1) or idle (0)",
    )?;

    registry.register(Box::new(scrape_duration.clone()))?;
    registry.register(Box::new(processes_total.clone()))?;
    registry.register(Box::new(cache_update_duration.clone()))?;
    registry.register(Box::new(cache_update_success.clone()))?;
    registry.register(Box::new(cache_updating.clone()))?;

    debug!("All metrics registered successfully");

    let health_stats = Arc::new(HealthStats::new());

    let health_config = HealthAppConfig {
        io_buffer: BufferHealthConfig {
            capacity_kb: buffer_config.io_kb,
            larger_is_better: false,
            warn_percent: Some(80.0),
            critical_percent: Some(95.0),
        },
        smaps_buffer: BufferHealthConfig {
            capacity_kb: buffer_config.smaps_kb,
            larger_is_better: false,
            warn_percent: Some(80.0),
            critical_percent: Some(95.0),
        },
        smaps_rollup_buffer: BufferHealthConfig {
            capacity_kb: buffer_config.smaps_rollup_kb,
            larger_is_better: false,
            warn_percent: Some(80.0),
            critical_percent: Some(95.0),
        },
    };
    let health_state = Arc::new(HealthState::new(health_config));

    // Initialize eBPF manager if enabled
    let ebpf = if config.enable_ebpf.unwrap_or(false) {
        info!("eBPF enabled in configuration, attempting to initialize...");
        match ebpf::EbpfManager::new() {
            Ok(manager) => {
                if manager.is_enabled() {
                    info!("✅ eBPF initialized successfully - process I/O tracking enabled");
                } else {
                    warn!("⚠️  eBPF initialization returned disabled state - running without eBPF metrics");
                    health_stats
                        .ebpf_init_failures
                        .fetch_add(1, Ordering::Relaxed);
                }
                Some(Arc::new(manager))
            }
            Err(e) => {
                warn!(
                    "⚠️  Failed to initialize eBPF: {} - running without eBPF metrics",
                    e
                );
                health_stats
                    .ebpf_init_failures
                    .fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    } else {
        debug!("eBPF disabled in configuration");
        None
    };

    // Drop privileges after eBPF initialization
    // This is safe because:
    // 1. eBPF programs are already loaded and pinned (if enabled)
    // 2. Runtime operations don't require root privileges
    // 3. If eBPF is disabled, we never needed root privileges
    drop_privileges();

    // Initialize ringbuffer manager
    let initial_subgroup_count = SUBGROUPS.len().max(1); // Prevent division by zero
    let ringbuffer_manager = Arc::new(RingbufferManager::new(
        config.ringbuffer.clone(),
        initial_subgroup_count,
    ));
    info!(
        "Ringbuffer manager initialized with {} initial subgroups, {} entries per subgroup",
        initial_subgroup_count,
        ringbuffer_manager.get_stats().entries_per_subgroup
    );

    let state = Arc::new(AppState {
        registry,
        metrics,
        scrape_duration,
        processes_total,
        cache_update_duration,
        cache_update_success,
        cache_updating,
        cache: Arc::new(RwLock::new(MetricsCache::default())),
        config: Arc::new(config.clone()),
        buffer_config,
        cpu_cache: StdRwLock::new(HashMap::new()),
        health_stats: health_stats.clone(),
        health_state,
        system_cpu_cache: CpuStatsCache::new(),
        ebpf,
        ringbuffer_manager,
        start_time: Instant::now(),
    });

    // Perform initial cache population
    info!("Performing initial cache update");
    if let Err(e) = update_cache(&state).await {
        error!("Initial cache update failed: {}", e);
    } else {
        info!("Initial cache update completed successfully");
    }

    info!(
        "Note: No background cache refresh task - updates will be triggered by /metrics requests"
    );

    // Setup graceful shutdown signal handlers
    let shutdown_signal = async {
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
    };

    // Configure HTTP server routes
    let addr: SocketAddr = format!("{}:{}", bind_ip_str, port).parse()?;

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
        .route("/html/details", get(html_details_handler))
        .route("/html/subgroups", get(html_subgroups_handler))
        .route("/html/health", get(html_health_handler))
        .route("/html/config", get(html_config_handler))
        .route("/html/docs", get(html_docs_handler));

    if config.enable_pprof.unwrap_or(false) {
        debug!("Debug endpoints enabled at /debug/pprof");
    }

    let app = app.with_state(state.clone());

    // Check if TLS is enabled
    let enable_tls = config.enable_tls.unwrap_or(false);

    if enable_tls {
        // TLS is enabled - use axum_server with rustls
        // These paths are guaranteed to exist since validate_effective_config() was called earlier
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
            bind_ip_str, port
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
    } else {
        // TLS is disabled - use standard TCP listener
        let listener = TcpListener::bind(addr).await?;
        info!(
            "herakles-node-exporter listening on http://{}:{}",
            bind_ip_str, port
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
    }

    info!("herakles-node-exporter stopped gracefully");
    Ok(())
}
