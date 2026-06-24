//! herakles-node-exporter - version 0.1.0
//!
//! Professional memory metrics exporter with tracing logging.
//! This is the main entry point that initializes the server and handles subcommands.

mod cache;
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
mod state;
mod system;

use ahash::AHashMap as HashMap;
use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use herakles_node_exporter::{AppConfig as HealthAppConfig, BufferHealthConfig, HealthState};
use prometheus::{Gauge, Registry};
use rayon::prelude::*;
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
    command_check, command_config, command_generate_testdata, command_install, command_subgroups,
    command_test, command_uninstall,
};
use config::{
    resolve_config, show_config, validate_effective_config, Config, DEFAULT_BIND_ADDR,
    DEFAULT_CACHE_TTL, DEFAULT_PORT,
};
use handlers::{
    config_handler, details_handler, doc_handler, health_handler, html_config_handler,
    html_dashboard_handler, html_details_handler, html_docs_handler, html_health_handler,
    html_index_handler, html_subgroups_handler, metrics_handler, root_handler, subgroups_handler,
};
use health_stats::HealthStats;
use metrics::MemoryMetrics;
use process::{
    classify_process_raw, collect_proc_entries, get_cpu_stat_for_pid, is_kernel_thread,
    parse_memory_for_process, parse_start_time_seconds, read_block_io, read_process_name,
    read_vmswap, should_include_process, CLK_TCK, MAX_IO_BUFFER_BYTES, MAX_SMAPS_BUFFER_BYTES,
    MAX_SMAPS_ROLLUP_BUFFER_BYTES, SUBGROUPS,
};
use ringbuffer::{RingbufferEntry, TopProcessInfo};
use ringbuffer_manager::RingbufferManager;
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

/// CPU percentage scaling factor to preserve precision in u32 storage.
/// CPU percent values are multiplied by this factor before storing,
/// and divided by this factor when displaying.
const CPU_SCALE_FACTOR: f32 = 1000.0;

/// Aggregated metrics data for a subgroup.
struct AggregatedData {
    rss_sum: u64,
    pss_sum: u64,
    uss_sum: u64,
    cpu_percent_sum: f64,
    cpu_time_sum: f64,
    process_count: usize,
}

/// Helper function to extract top-3 processes from a slice.
fn extract_top_3<F, V>(procs: &[&ProcMem], compare_fn: F, value_fn: V) -> [TopProcessInfo; 3]
where
    F: Fn(&ProcMem, &ProcMem) -> std::cmp::Ordering,
    V: Fn(&ProcMem) -> u32,
{
    let mut sorted: Vec<&ProcMem> = procs.to_vec();
    sorted.sort_by(|a, b| compare_fn(a, b));

    [
        if !sorted.is_empty() {
            TopProcessInfo::new(sorted[0].pid, value_fn(sorted[0]), &sorted[0].name)
        } else {
            TopProcessInfo::default()
        },
        if sorted.len() > 1 {
            TopProcessInfo::new(sorted[1].pid, value_fn(sorted[1]), &sorted[1].name)
        } else {
            TopProcessInfo::default()
        },
        if sorted.len() > 2 {
            TopProcessInfo::new(sorted[2].pid, value_fn(sorted[2]), &sorted[2].name)
        } else {
            TopProcessInfo::default()
        },
    ]
}

/// Cache update function.
#[instrument(skip(state))]
async fn update_cache(state: &SharedState) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    info!("Starting cache update");

    // Get current timestamp for rate calculations
    let current_time = chrono::Utc::now().timestamp() as f64;

    // Mark cache as updating
    {
        let mut cache = state.cache.write().await;
        cache.is_updating = true;
        cache.update_success = false;
        state.cache_updating.set(1.0);
        debug!("Cache marked as updating (old snapshot still available)");
    }

    let (test_data_file, max_processes, min_uss_bytes, config_snapshot, buffer_config_snapshot) = {
        let cfg = state.config();
        let buf_cfg = state.buffer_config();
        (
            cfg.test_data_file.clone(),
            cfg.max_processes,
            cfg.min_uss_kb.unwrap_or(0) * 1024,
            cfg.clone(),
            *buf_cfg,
        )
    };

    use std::sync::atomic::AtomicUsize;
    let included_count = AtomicUsize::new(0);
    let skipped_count = AtomicUsize::new(0);

    // Clone previous cache for delta calculation (before collecting new processes)
    let previous_cache = {
        let cache = state.cache.read().await;
        cache.processes.clone()
    };

    let results: Vec<ProcMem> = if let Some(test_file) = &test_data_file {
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
                if !should_include_process(&tp.name, &config_snapshot) {
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
        let entries = collect_proc_entries("/proc", max_processes);
        debug!("Collected {} process entries from /proc", entries.len());

        entries
            .par_iter()
            .filter_map(|entry| {
                let name = match read_process_name(&entry.proc_path) {
                    Some(name) => name,
                    None => {
                        debug!("Skipping process {}: could not read name", entry.pid);
                        state.health_stats.record_proc_read_error();
                        skipped_count.fetch_add(1, Ordering::Relaxed);
                        return None;
                    }
                };

                if !should_include_process(&name, &config_snapshot) {
                    debug!("Skipping process {}: filtered by name config", name);
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                let cpu = get_cpu_stat_for_pid(entry.pid, &entry.proc_path, &state.cpu_cache);

                let parse_start = Instant::now();
                match parse_memory_for_process(&entry.proc_path, &buffer_config_snapshot) {
                    Ok((rss, pss, uss)) => {
                        let parse_duration_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
                        state.health_stats.record_parsing_duration_ms(parse_duration_ms);

                        if uss < min_uss_bytes {
                            debug!(
                                "Skipping process {}: USS {} bytes below threshold {} bytes",
                                name, uss, min_uss_bytes
                            );
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                            return None;
                        }

                        // Read VmSwap from /proc/[pid]/status
                        let vmswap = read_vmswap(&entry.proc_path).unwrap_or(0);

                        // Read process start time from /proc/[pid]/stat
                        let start_time_seconds = parse_start_time_seconds(&entry.proc_path).unwrap_or(0.0);

                        // Read Block I/O from /proc/[pid]/io
                        let (read_bytes, write_bytes) =
                            read_block_io(&entry.proc_path).unwrap_or((0, 0));

                        // Get previous I/O values from cache (if exists)
                        let (
                            last_read_bytes,
                            last_write_bytes,
                            last_rx_bytes,
                            last_tx_bytes,
                            last_update_time,
                        ) =
                            if let Some(prev) = previous_cache.get(&entry.pid) {
                                // Use previous values as baseline for rate calculation
                                (
                                    prev.read_bytes,
                                    prev.write_bytes,
                                    prev.rx_bytes,
                                    prev.tx_bytes,
                                    prev.last_update_time,
                                )
                            } else {
                                // First time seeing this process - use current values as baseline
                                // This means the first rate calculation will show 0 (expected)
                                (read_bytes, write_bytes, 0, 0, current_time)
                            };

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
                            vmswap,
                            start_time_seconds,
                            read_bytes,
                            write_bytes,
                            rx_bytes: 0,  // Will be filled by eBPF if available
                            tx_bytes: 0,  // Will be filled by eBPF if available
                            last_read_bytes,
                            last_write_bytes,
                            last_rx_bytes,
                            last_tx_bytes,
                            last_update_time,
                        })
                    }
                    Err(e) => {
                        // Kernel threads have no userspace memory, so memory
                        // parsing always fails for them — skip silently rather
                        // than inflating the parsing-error counter.
                        if is_kernel_thread(&entry.proc_path) {
                            debug!("Skipping kernel thread {}", entry.pid);
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                            return None;
                        }
                        let err_msg = e.to_string();
                        debug!("Skipping process {}: failed to parse memory: {}", name, err_msg);
                        state.health_stats.record_parsing_error();
                        // Check if it's a permission denied error
                        if err_msg.contains("Permission denied") || err_msg.contains("permission") {
                            state.health_stats.record_permission_denied();
                        }
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

    // Convert results to mutable vector for eBPF network stats update
    let mut results = results;

    // Update network I/O from eBPF if available
    if let Some(ref ebpf_manager) = state.ebpf {
        match ebpf_manager.read_process_net_stats() {
            Ok(net_stats) => {
                debug!("Read {} network stats from eBPF", net_stats.len());
                let process_indices: HashMap<u32, usize> = results
                    .iter()
                    .enumerate()
                    .map(|(idx, proc)| (proc.pid, idx))
                    .collect();

                for stat in net_stats {
                    if let Some(&idx) = process_indices.get(&stat.pid) {
                        let proc = &mut results[idx];
                        // Get previous network I/O from cache
                        let (last_rx, last_tx) = if let Some(prev) = previous_cache.get(&stat.pid) {
                            (prev.rx_bytes, prev.tx_bytes)
                        } else {
                            // First time seeing network stats for this process
                            (stat.rx_bytes, stat.tx_bytes)
                        };

                        proc.rx_bytes = stat.rx_bytes;
                        proc.tx_bytes = stat.tx_bytes;
                        proc.last_rx_bytes = last_rx;
                        proc.last_tx_bytes = last_tx;
                        // Update last_update_time to current time for rate calculation
                        proc.last_update_time = current_time;
                    }
                }
            }
            Err(e) => {
                debug!("Failed to read eBPF network stats: {}", e);
            }
        }
    } else {
        // No eBPF available - update timestamps for processes that had previous data
        for proc in results.iter_mut() {
            if previous_cache.contains_key(&proc.pid) {
                proc.last_update_time = current_time;
            }
        }
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

    // Count unique subgroups and aggregate metrics for ringbuffer
    // Also collect processes per subgroup for top-N calculation
    let mut aggregated_by_subgroup: HashMap<String, AggregatedData> = HashMap::new();
    let mut processes_by_subgroup: HashMap<String, Vec<&ProcMem>> = HashMap::new();

    for p in &results {
        let (group, subgroup) = classify_process_raw(&p.name);
        let key = format!("{}:{}", group, subgroup);

        let agg = aggregated_by_subgroup
            .entry(key.clone())
            .or_insert(AggregatedData {
                rss_sum: 0,
                pss_sum: 0,
                uss_sum: 0,
                cpu_percent_sum: 0.0,
                cpu_time_sum: 0.0,
                process_count: 0,
            });

        agg.rss_sum += p.rss;
        agg.pss_sum += p.pss;
        agg.uss_sum += p.uss;
        agg.cpu_percent_sum += p.cpu_percent as f64;
        agg.cpu_time_sum += p.cpu_time_seconds as f64;
        agg.process_count += 1;

        // Store process reference for top-N calculation
        processes_by_subgroup.entry(key).or_default().push(p);
    }

    let subgroups_count = aggregated_by_subgroup.len() as u64;

    // Record ringbuffer entries for each subgroup
    let timestamp = chrono::Utc::now().timestamp();
    for (key, agg_data) in &aggregated_by_subgroup {
        // Get top-3 processes for this subgroup
        let procs = processes_by_subgroup
            .get(key)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Calculate average CPU percent
        let avg_cpu_percent = if agg_data.process_count > 0 {
            (agg_data.cpu_percent_sum / agg_data.process_count as f64) as f32
        } else {
            0.0
        };

        // Calculate top-3 by CPU, RSS, and PSS using helper function
        let top_cpu = extract_top_3(
            procs,
            |a, b| {
                b.cpu_percent
                    .partial_cmp(&a.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            },
            |p| (p.cpu_percent * CPU_SCALE_FACTOR) as u32,
        );

        let top_rss = extract_top_3(
            procs,
            |a, b| b.rss.cmp(&a.rss),
            |p| (p.rss / 1024) as u32, // Convert to KB
        );

        let top_pss = extract_top_3(
            procs,
            |a, b| b.pss.cmp(&a.pss),
            |p| (p.pss / 1024) as u32, // Convert to KB
        );

        let entry = RingbufferEntry {
            timestamp,
            rss_kb: agg_data.rss_sum / 1024,
            pss_kb: agg_data.pss_sum / 1024,
            uss_kb: agg_data.uss_sum / 1024,
            cpu_percent: avg_cpu_percent,
            cpu_time_seconds: agg_data.cpu_time_sum as f32,
            top_cpu,
            top_rss,
            top_pss,
            _padding: [],
        };

        state.ringbuffer_manager.record(key, entry);

        debug!(
            "Recorded ringbuffer entry for {}: {} processes, RSS={} KB, CPU={:.1}%",
            key,
            agg_data.process_count,
            agg_data.rss_sum / 1024,
            avg_cpu_percent
        );
    }

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

    // Update FD usage
    if let Ok((open, max)) = system::get_fd_usage() {
        state.health_stats.update_fd_usage(open, max);
    }

    // Update eBPF performance stats
    if let Some(ref ebpf_manager) = state.ebpf {
        let perf_stats = ebpf_manager.get_performance_stats();
        if perf_stats.enabled {
            state
                .health_stats
                .record_ebpf_events_per_sec(perf_stats.events_per_sec);
            state
                .health_stats
                .record_ebpf_lost_events(perf_stats.lost_events_total);
            state
                .health_stats
                .ebpf_map_usage_percent
                .add_sample(perf_stats.map_usage_percent);
            state
                .health_stats
                .ebpf_overhead_cpu_percent
                .add_sample(perf_stats.cpu_overhead_percent);
            // lost_events is cumulative, so just store it
            state
                .health_stats
                .ebpf_lost_events
                .store(perf_stats.lost_events_total, Ordering::Relaxed);
        }
    }

    // Prune the database once per update cycle and update database metrics
    if state.config().ringbuffer.enable_database {
        if let Err(e) = state.ringbuffer_manager.prune_database(false) {
            warn!("Failed to prune database: {}", e);
        }
        let db_stats = state.ringbuffer_manager.get_stats();
        state.database_entries.set(db_stats.db_entries as f64);
        state.database_size_bytes.set(db_stats.db_size_bytes as f64);
    }

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

    let buffer_config = process::resolve_buffer_config(&config, &args);

    // Initialize Prometheus metrics registry
    let registry = Registry::new();
    debug!("Prometheus registry initialized");

    let metrics = MemoryMetrics::new(&registry)?;
    let scrape_duration = Gauge::new(
        "herakles_exporter_scrape_duration_seconds",
        "Time spent serving /metrics request (reading from cache)",
    )?;
    let processes = Gauge::new(
        "herakles_exporter_processes",
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
    registry.register(Box::new(processes.clone()))?;
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
    let initial_subgroup_count = SUBGROUPS.read()?.len().max(1); // Prevent division by zero
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
        processes,
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

    // Start background cache refresh task
    let bg_state = state.clone();
    let ttl = Duration::from_secs(state.config().cache_ttl.unwrap_or(DEFAULT_CACHE_TTL));

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

    #[cfg(unix)]
    {
        let state_clone = state.clone();
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

                // 1. Reload subgroups
                process::reload_subgroups();

                // 2. Reload general config
                match state_clone.reload_config() {
                    Ok(_) => info!("Configuration and subgroups reloaded successfully."),
                    Err(e) => error!("Failed to reload configuration: {}", e),
                }
            }
        });
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

    background_task.abort();
    let _ = background_task.await;

    // Flush the persistent database to disk on graceful shutdown
    if let Err(e) = state.ringbuffer_manager.flush() {
        error!("Failed to flush persistent database: {}", e);
    }

    info!("herakles-node-exporter stopped gracefully");
    Ok(())
}
