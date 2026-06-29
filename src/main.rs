//! herakles-node-exporter - version 0.2.1
//!
//! Professional memory metrics exporter with tracing logging.
//! This is the main entry point that initializes the server and handles subcommands.

mod cache;
mod cache_update;
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
mod server;
mod state;
mod system;

use ahash::AHashMap as HashMap;
use clap::Parser;
use herakles_node_exporter::{AppConfig as HealthAppConfig, BufferHealthConfig, HealthState};
use prometheus::{Gauge, Registry};
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Instant;
use tokio::sync::{Notify, RwLock};
use tracing::{debug, error, info, warn, Level};

use cache::MetricsCache;
use cache_update::update_cache;
use cli::{Args, Commands, LogLevel};
use commands::{
    command_check, command_config, command_generate_testdata, command_install, command_subgroups,
    command_test, command_uninstall,
};
use config::{
    resolve_config, show_config, validate_effective_config, Config, DEFAULT_BIND_ADDR, DEFAULT_PORT,
};
use health_stats::HealthStats;
use metrics::MemoryMetrics;
use process::SUBGROUPS;
use ringbuffer_manager::RingbufferManager;
use state::AppState;
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
        // Intercept install/uninstall early since they don't require config validation
        match command {
            Commands::Install { no_service, force } => {
                return command_install(*no_service, *force);
            }
            Commands::Uninstall { yes } => {
                return command_uninstall(*yes);
            }
            _ => {}
        }

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
            Commands::Install { .. } => unreachable!("Install handled above"),
            Commands::Uninstall { .. } => unreachable!("Uninstall handled above"),
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

    let bind_ip_str = config
        .bind
        .clone()
        .unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string());
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

    let buffer_config = crate::process::resolve_buffer_config(&config, &args);

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
    let database_entries = Gauge::new(
        "herakles_exporter_database_entries",
        "Total number of entries currently stored in the persistent database",
    )?;
    let database_size_bytes = Gauge::new(
        "herakles_exporter_database_size_bytes",
        "Size of the persistent database on disk in bytes",
    )?;

    registry.register(Box::new(scrape_duration.clone()))?;
    registry.register(Box::new(processes_total.clone()))?;
    registry.register(Box::new(cache_update_duration.clone()))?;
    registry.register(Box::new(cache_update_success.clone()))?;
    registry.register(Box::new(cache_updating.clone()))?;
    registry.register(Box::new(database_entries.clone()))?;
    registry.register(Box::new(database_size_bytes.clone()))?;

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

    // Initialize ringbuffer manager
    let initial_subgroup_count = SUBGROUPS.read().unwrap().len().max(1); // Prevent division by zero
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
        database_entries,
        database_size_bytes,
        cache: Arc::new(RwLock::new(MetricsCache::default())),
        config: Arc::new(StdRwLock::new(config.clone())),
        buffer_config: StdRwLock::new(buffer_config),
        args: args.clone(),
        cpu_cache: StdRwLock::new(HashMap::new()),
        health_stats: health_stats.clone(),
        health_state,
        cache_ready: Arc::new(Notify::new()),
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

    server::run(state, config, &bind_ip_str, port).await
}
