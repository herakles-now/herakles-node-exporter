//! herakles-proc-mem-exporter - version 0.1.0
//!
//! Professional memory metrics exporter with tracing logging.
//! This is the main entry point that initializes the server and handles subcommands.

mod cache;
mod cli;
mod commands;
mod config;
mod handlers;
mod health_stats;
mod metrics;
mod process;
mod state;
mod system;

use ahash::AHashMap as HashMap;
use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use herakles_proc_mem_exporter::{AppConfig as HealthAppConfig, BufferHealthConfig, HealthState};
use prometheus::{Gauge, Registry};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Instant;
use tokio::{
    net::TcpListener,
    signal,
    sync::{Notify, RwLock},
    time::{interval, Duration},
};
use tracing::{debug, error, info, instrument, warn, Level};

use cache::{MetricsCache, ProcMem};
use cli::{Args, Commands, LogLevel};
use commands::{
    command_check, command_config, command_generate_testdata, command_subgroups, command_test,
};
use config::{
    resolve_config, show_config, validate_effective_config, Config, DEFAULT_BIND_ADDR,
    DEFAULT_CACHE_TTL, DEFAULT_PORT,
};
use handlers::{config_handler, doc_handler, health_handler, metrics_handler, subgroups_handler};
use health_stats::HealthStats;
use metrics::MemoryMetrics;
use process::{
    classify_process_raw, collect_proc_entries, get_cpu_stat_for_pid, parse_memory_for_process,
    read_process_name, should_include_process, BufferConfig, CLK_TCK, MAX_IO_BUFFER_BYTES,
    MAX_SMAPS_BUFFER_BYTES, MAX_SMAPS_ROLLUP_BUFFER_BYTES,
};
use state::{AppState, SharedState};
use system::CpuStatsCache;

// Re-export load_test_data_from_file for use in update_cache
use commands::generate::load_test_data_from_file;

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

/// Reads the exporter's own memory and CPU usage from /proc/self.
fn read_self_resources() -> (f64, f64) {
    let memory_mb = read_self_memory_mb().unwrap_or(0.0);
    let cpu_percent = read_self_cpu_percent().unwrap_or(0.0);
    (memory_mb, cpu_percent)
}

/// Reads the exporter's RSS memory usage from /proc/self/status.
fn read_self_memory_mb() -> Option<f64> {
    let content = fs::read_to_string("/proc/self/status").ok()?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            let kb: u64 = value.split_whitespace().next()?.parse().ok()?;
            return Some(kb as f64 / 1024.0);
        }
    }
    None
}

/// Reads the exporter's CPU usage from /proc/self/stat.
fn read_self_cpu_percent() -> Option<f64> {
    let content = fs::read_to_string("/proc/self/stat").ok()?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() <= 14 {
        return None;
    }

    let utime: f64 = parts[13].parse().ok()?;
    let stime: f64 = parts[14].parse().ok()?;
    let total_ticks = utime + stime;

    let uptime_content = fs::read_to_string("/proc/uptime").ok()?;
    let uptime_seconds: f64 = uptime_content.split_whitespace().next()?.parse().ok()?;

    if uptime_seconds > 0.0 {
        let cpu_time_seconds = total_ticks / *CLK_TCK;
        Some((cpu_time_seconds / uptime_seconds) * 100.0)
    } else {
        None
    }
}

/// Cache update function.
#[instrument(skip(state))]
async fn update_cache(state: &SharedState) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    info!("Starting cache update");

    // Mark cache as updating
    {
        let mut cache = state.cache.write().await;
        cache.is_updating = true;
        cache.update_success = false;
        state.cache_updating.set(1.0);
        debug!("Cache marked as updating (old snapshot still available)");
    }

    let min_uss_bytes = state.config.min_uss_kb.unwrap_or(0) * 1024;

    use std::sync::atomic::AtomicUsize;
    let included_count = AtomicUsize::new(0);
    let skipped_count = AtomicUsize::new(0);

    let results: Vec<ProcMem> = if let Some(test_file) = &state.config.test_data_file {
        info!("Using test data from file: {}", test_file.display());

        let test_data = match load_test_data_from_file(test_file) {
            Ok(data) => data,
            Err(err_msg) => {
                error!("Failed to load test data: {}", err_msg);
                state.health_stats.record_scan_failure();
                {
                    let mut cache = state.cache.write().await;
                    cache.is_updating = false;
                    state.cache_updating.set(0.0);
                }
                return Err(err_msg.into());
            }
        };

        info!("Loaded {} test processes", test_data.processes.len());

        test_data
            .processes
            .into_iter()
            .filter_map(|tp| {
                if !should_include_process(&tp.name, &state.config) {
                    debug!("Skipping process {}: filtered by name config", tp.name);
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                if tp.uss < min_uss_bytes {
                    debug!(
                        "Skipping process {}: USS {} bytes below threshold {} bytes",
                        tp.name, tp.uss, min_uss_bytes
                    );
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                debug!(
                    "Including test process {}: {} (RSS: {} MB, PSS: {} MB, USS: {} MB, CPU: {:.6}%)",
                    tp.pid,
                    tp.name,
                    tp.rss / 1024 / 1024,
                    tp.pss / 1024 / 1024,
                    tp.uss / 1024 / 1024,
                    tp.cpu_percent
                );

                included_count.fetch_add(1, Ordering::Relaxed);
                Some(ProcMem::from(tp))
            })
            .collect()
    } else {
        let entries = collect_proc_entries("/proc", state.config.max_processes);
        debug!("Collected {} process entries from /proc", entries.len());

        entries
            .par_iter()
            .filter_map(|entry| {
                let name = match read_process_name(&entry.proc_path) {
                    Some(name) => name,
                    None => {
                        debug!("Skipping process {}: could not read name", entry.pid);
                        skipped_count.fetch_add(1, Ordering::Relaxed);
                        return None;
                    }
                };

                if !should_include_process(&name, &state.config) {
                    debug!("Skipping process {}: filtered by name config", name);
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                let cpu = get_cpu_stat_for_pid(entry.pid, &entry.proc_path, &state.cpu_cache);

                match parse_memory_for_process(&entry.proc_path, &state.buffer_config) {
                    Ok((rss, pss, uss)) => {
                        if uss < min_uss_bytes {
                            debug!(
                                "Skipping process {}: USS {} bytes below threshold {} bytes",
                                name, uss, min_uss_bytes
                            );
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                            return None;
                        }

                        debug!(
                            "Including process {}: {} (RSS: {} MB, PSS: {} MB, USS: {} MB, CPU: {:.6}%)",
                            entry.pid,
                            name,
                            rss / 1024 / 1024,
                            pss / 1024 / 1024,
                            uss / 1024 / 1024,
                            cpu.cpu_percent
                        );

                        included_count.fetch_add(1, Ordering::Relaxed);
                        Some(ProcMem {
                            pid: entry.pid,
                            name,
                            rss,
                            pss,
                            uss,
                            cpu_percent: cpu.cpu_percent as f32,
                            cpu_time_seconds: cpu.cpu_time_seconds as f32,
                        })
                    }
                    Err(e) => {
                        debug!("Skipping process {}: failed to parse memory: {}", name, e);
                        skipped_count.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                }
            })
            .collect()
    };

    let final_included = included_count.load(Ordering::Relaxed);
    let final_skipped = skipped_count.load(Ordering::Relaxed);

    debug!(
        "Process filtering completed: {} included, {} skipped",
        final_included, final_skipped
    );

    if results.is_empty() {
        warn!("No processes matched filters after sorting");
    }

    // Update cache with new data
    {
        let mut cache = state.cache.write().await;
        cache.processes.clear();
        for p in &results {
            cache.processes.insert(p.pid, p.clone());
        }

        cache.update_duration_seconds = start.elapsed().as_secs_f64();
        cache.update_success = true;
        cache.last_updated = Some(start);
        cache.is_updating = false;

        state.cache_updating.set(0.0);
    }

    state.cache_ready.notify_waiters();

    // Count unique subgroups
    let mut used_subgroups_set: HashSet<(Arc<str>, Arc<str>)> = HashSet::new();
    for p in &results {
        let (group, subgroup) = classify_process_raw(&p.name);
        used_subgroups_set.insert((group, subgroup));
    }
    let subgroups_count = used_subgroups_set.len() as u64;

    let scanned = results.len() as u64;
    let scan_duration = start.elapsed().as_secs_f64();
    state
        .health_stats
        .record_scan(scanned, scan_duration, scan_duration);

    state.health_stats.record_scan_success();
    state.health_stats.record_used_subgroups(subgroups_count);
    state.health_stats.record_cache_size(scanned);
    state.health_stats.update_last_scan_time();

    // Update buffer usage
    let io_usage_kb = MAX_IO_BUFFER_BYTES.load(Ordering::Relaxed).div_ceil(1024);
    let smaps_usage_kb = MAX_SMAPS_BUFFER_BYTES
        .load(Ordering::Relaxed)
        .div_ceil(1024);
    let smaps_rollup_usage_kb = MAX_SMAPS_ROLLUP_BUFFER_BYTES
        .load(Ordering::Relaxed)
        .div_ceil(1024);

    state.health_state.update_io_buffer_kb(io_usage_kb as usize);
    state
        .health_state
        .update_smaps_buffer_kb(smaps_usage_kb as usize);
    state
        .health_state
        .update_smaps_rollup_buffer_kb(smaps_rollup_usage_kb as usize);

    let (exporter_mem_mb, exporter_cpu_pct) = read_self_resources();
    state
        .health_stats
        .record_exporter_resources(exporter_mem_mb, exporter_cpu_pct);

    info!(
        "Cache update completed: {} processes (subgroup filters applied at scrape), {} total scanned, {:.2}ms",
        results.len(),
        final_included + final_skipped,
        start.elapsed().as_secs_f64() * 1000.0
    );

    Ok(())
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
        let config = resolve_config(&args)?;
        if let Err(e) = validate_effective_config(&config) {
            eprintln!("❌ Configuration invalid: {}", e);
            std::process::exit(1);
        }

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
        };
    }

    // Load configuration for main server mode
    let config = resolve_config(&args)?;

    if let Err(e) = validate_effective_config(&config) {
        eprintln!("❌ Configuration invalid: {}", e);
        std::process::exit(1);
    }

    setup_logging(&config, &args);

    info!("Starting herakles-proc-mem-exporter");

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
        "herakles_proc_mem_scrape_duration_seconds",
        "Time spent serving /metrics request (reading from cache)",
    )?;
    let processes_total = Gauge::new(
        "herakles_proc_mem_processes_total",
        "Number of processes currently exported by herakles-proc-mem-exporter",
    )?;
    let cache_update_duration = Gauge::new(
        "herakles_proc_mem_cache_update_duration_seconds",
        "Time spent updating the process metrics cache in background",
    )?;
    let cache_update_success = Gauge::new(
        "herakles_proc_mem_cache_update_success",
        "Whether the last cache update was successful (1) or failed (0)",
    )?;
    let cache_updating = Gauge::new(
        "herakles_proc_mem_cache_updating",
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
        cache_ready: Arc::new(Notify::new()),
        system_cpu_cache: CpuStatsCache::new(),
    });

    // Perform initial cache population
    info!("Performing initial cache update");
    if let Err(e) = update_cache(&state).await {
        error!("Initial cache update failed: {}", e);
    } else {
        info!("Initial cache update completed successfully");
    }

    // Start background cache refresh task
    let bg_state = state.clone();
    let ttl = Duration::from_secs(state.config.cache_ttl.unwrap_or(DEFAULT_CACHE_TTL));

    let background_task = tokio::spawn(async move {
        let mut int = interval(ttl);
        debug!(
            "Background cache update task started with {}s interval",
            ttl.as_secs()
        );

        loop {
            int.tick().await;
            debug!("Starting scheduled cache update");
            if let Err(e) = update_cache(&bg_state).await {
                error!("Scheduled cache update failed: {}", e);
            } else {
                debug!("Scheduled cache update completed");
            }
        }
    });

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

    let mut app = Router::new().route("/metrics", get(metrics_handler));

    if config.enable_health.unwrap_or(true) {
        app = app.route("/health", get(health_handler));
    }

    app = app
        .route("/config", get(config_handler))
        .route("/subgroups", get(subgroups_handler))
        .route("/doc", get(doc_handler));

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
            "herakles-proc-mem-exporter listening on https://{}:{}",
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
            "herakles-proc-mem-exporter listening on http://{}:{}",
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

    background_task.abort();
    let _ = background_task.await;

    info!("herakles-proc-mem-exporter stopped gracefully");
    Ok(())
}
