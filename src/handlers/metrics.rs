//! Metrics endpoint handler for Prometheus scraping.
//!
//! This module provides the `/metrics` endpoint handler that formats and returns
//! system and group-level metrics in Prometheus text format according to the German specification.
//! NO per-process or Top-N metrics are exported.

use ahash::AHashMap as HashMap;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};
use std::time::Instant;
use tracing::{debug, error, instrument, warn};

use crate::cache::ProcMem;
use crate::collectors;
use crate::process::classify_process_with_config;
use crate::state::SharedState;
use crate::system;

/// Buffer capacity for metrics encoding.
const BUFFER_CAP: usize = 512 * 1024;

/// Error type for metrics endpoint failures.
#[derive(Debug)]
pub enum MetricsError {
    EncodingFailed,
}

impl IntoResponse for MetricsError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics",
        )
            .into_response()
    }
}

/// Aggregated metrics for a group/subgroup.
#[derive(Default, Debug)]
struct GroupMetrics {
    rss_sum: u64,
    pss_sum: u64,
    swap_sum: u64,
    cpu_percent_sum: f64,
    cpu_time_total_sum: f64,
}

/// Handler for the /metrics endpoint.
#[instrument(skip(state))]
pub async fn metrics_handler(State(state): State<SharedState>) -> Result<String, MetricsError> {
    let start = Instant::now();
    debug!("Processing /metrics request");

    // Wait for cache to be available (not currently updating)
    loop {
        // Measure lock wait time
        let lock_wait_start = Instant::now();
        let cache_guard = state.cache.read().await;
        let lock_wait_ms = lock_wait_start.elapsed().as_secs_f64() * 1000.0;
        state
            .health_stats
            .record_lock_wait_duration_ms(lock_wait_ms);

        if !cache_guard.is_updating {
            let processes_vec: Vec<ProcMem> = cache_guard.processes.values().cloned().collect();
            let meta = (
                cache_guard.update_duration_seconds,
                cache_guard.update_success,
                cache_guard.is_updating,
            );

            drop(cache_guard);

            // Update cache metadata metrics
            state.cache_update_duration.set(meta.0);
            state
                .cache_update_success
                .set(if meta.1 { 1.0 } else { 0.0 });
            state.cache_updating.set(if meta.2 { 1.0 } else { 0.0 });

            // Reset metrics before populating with fresh data
            state.metrics.reset();

            let config_guard = state.config();
            let cfg = &*config_guard;
            let enable_rss = cfg.enable_rss.unwrap_or(true);
            let enable_pss = cfg.enable_pss.unwrap_or(true);
            let enable_cpu = cfg.enable_cpu.unwrap_or(true);

            // ========== PHASE 1: Aggregate processes by (group, subgroup) ==========
            let mut group_aggregations: HashMap<(String, String), GroupMetrics> = HashMap::new();
            let mut exported_count = 0usize;

            for p in &processes_vec {
                if let Some((group, subgroup)) = classify_process_with_config(&p.name, cfg) {
                    exported_count += 1;

                    let entry = group_aggregations
                        .entry((group.to_string(), subgroup.to_string()))
                        .or_default();

                    entry.rss_sum += p.rss;
                    entry.pss_sum += p.pss;
                    entry.swap_sum += p.vmswap;
                    entry.cpu_percent_sum += p.cpu_percent as f64;
                    entry.cpu_time_total_sum += p.cpu_time_seconds as f64;
                }
            }

            state.processes_total.set(exported_count as f64);

            // ========== PHASE 2: Export Group-Level Metrics ==========
            for ((group, subgroup), metrics) in group_aggregations {
                // Memory Group Metrics
                if enable_rss {
                    state
                        .metrics
                        .group_memory_rss_bytes
                        .with_label_values(&[&group, &subgroup])
                        .set(metrics.rss_sum as f64);
                }

                if enable_pss {
                    state
                        .metrics
                        .group_memory_pss_bytes
                        .with_label_values(&[&group, &subgroup])
                        .set(metrics.pss_sum as f64);
                }

                state
                    .metrics
                    .group_memory_swap_bytes
                    .with_label_values(&[&group, &subgroup])
                    .set(metrics.swap_sum as f64);

                // CPU Group Metrics
                if enable_cpu {
                    // Convert CPU percentage to ratio (0.0-1.0)
                    let cpu_ratio = metrics.cpu_percent_sum / 100.0;
                    state
                        .metrics
                        .group_cpu_usage_ratio
                        .with_label_values(&[&group, &subgroup])
                        .set(cpu_ratio);

                    state
                        .metrics
                        .group_cpu_seconds_total
                        .with_label_values(&[group.as_str(), subgroup.as_str(), "total"])
                        .set(metrics.cpu_time_total_sum);
                }
            }

            // ========== PHASE 2.5: Block I/O Group Metrics (from eBPF) ==========
            #[cfg(feature = "ebpf")]
            if let Some(ebpf) = &state.ebpf {
                match ebpf.read_process_blkio_stats() {
                    Ok(blkio_stats) => {
                        // Aggregate per (group, subgroup)
                        // Tuple format: (read_bytes, write_bytes, read_ops, write_ops)
                        let mut blkio_groups: HashMap<(String, String), (u64, u64, u64, u64)> =
                            HashMap::new();

                        for stat in blkio_stats {
                            let (group, subgroup) =
                                crate::process::classify_process_raw(&stat.comm);
                            let entry = blkio_groups
                                .entry((group.to_string(), subgroup.to_string()))
                                .or_insert((0, 0, 0, 0));

                            entry.0 += stat.read_bytes;
                            entry.1 += stat.write_bytes;
                            entry.2 += stat.read_ops;
                            entry.3 += stat.write_ops;
                        }

                        for ((group, subgroup), (read_bytes, write_bytes, read_ops, write_ops)) in
                            blkio_groups
                        {
                            state
                                .metrics
                                .group_blkio_read_bytes_total
                                .with_label_values(&[&group, &subgroup])
                                .set(read_bytes as f64);
                            state
                                .metrics
                                .group_blkio_write_bytes_total
                                .with_label_values(&[&group, &subgroup])
                                .set(write_bytes as f64);
                            state
                                .metrics
                                .group_blkio_read_syscalls_total
                                .with_label_values(&[&group, &subgroup])
                                .set(read_ops as f64);
                            state
                                .metrics
                                .group_blkio_write_syscalls_total
                                .with_label_values(&[&group, &subgroup])
                                .set(write_ops as f64);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read eBPF block I/O statistics: {}", e);
                    }
                }
            }

            // ========== PHASE 3: System-Level CPU Metrics ==========
            match state.system_cpu_cache.calculate_usage_ratios() {
                Ok(cpu_ratios) => {
                    // Get the "cpu" (total) values for system ratios
                    if let Some(&usage_ratio) = cpu_ratios.usage.get("cpu") {
                        state.metrics.system_cpu_usage_ratio.set(usage_ratio);
                    }
                    if let Some(&idle_ratio) = cpu_ratios.idle.get("cpu") {
                        state.metrics.system_cpu_idle_ratio.set(idle_ratio);
                    }
                    if let Some(&iowait_ratio) = cpu_ratios.iowait.get("cpu") {
                        state.metrics.system_cpu_iowait_ratio.set(iowait_ratio);
                    }
                    if let Some(&steal_ratio) = cpu_ratios.steal.get("cpu") {
                        state.metrics.system_cpu_steal_ratio.set(steal_ratio);
                    }
                }
                Err(e) => {
                    warn!("Failed to calculate CPU ratios: {}", e);
                }
            }

            // Load averages
            match system::read_load_average() {
                Ok(load_avg) => {
                    state.metrics.system_cpu_load_1.set(load_avg.one_min);
                    state.metrics.system_cpu_load_5.set(load_avg.five_min);
                    state.metrics.system_cpu_load_15.set(load_avg.fifteen_min);
                }
                Err(e) => {
                    warn!("Failed to read load average: {}", e);
                }
            }

            // ========== PHASE 4: System-Level Memory Metrics ==========
            match system::read_extended_memory_info() {
                Ok(mem_info) => {
                    state
                        .metrics
                        .system_memory_total_bytes
                        .set(mem_info.total_bytes as f64);
                    state
                        .metrics
                        .system_memory_available_bytes
                        .set(mem_info.available_bytes as f64);
                    state
                        .metrics
                        .system_memory_cached_bytes
                        .set(mem_info.cached_bytes as f64);
                    state
                        .metrics
                        .system_memory_buffers_bytes
                        .set(mem_info.buffers_bytes as f64);

                    // Calculate memory used ratio
                    if mem_info.total_bytes > 0 {
                        let mem_used_ratio = (mem_info.total_bytes - mem_info.available_bytes)
                            as f64
                            / mem_info.total_bytes as f64;
                        state.metrics.system_memory_used_ratio.set(mem_used_ratio);
                    }

                    // Calculate swap used ratio
                    if mem_info.swap_total_bytes > 0 {
                        let swap_used_ratio = (mem_info.swap_total_bytes - mem_info.swap_free_bytes)
                            as f64
                            / mem_info.swap_total_bytes as f64;
                        state.metrics.system_swap_used_ratio.set(swap_used_ratio);
                    } else {
                        state.metrics.system_swap_used_ratio.set(0.0);
                    }
                }
                Err(e) => {
                    warn!("Failed to read memory info: {}", e);
                }
            }

            // ========== PHASE 5: System-Level Disk Metrics ==========
            match collectors::diskstats::read_diskstats() {
                Ok(diskstats) => {
                    for (device, stats) in diskstats {
                        // Read bytes
                        state
                            .metrics
                            .system_disk_read_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.sectors_read as f64 * 512.0);

                        // Write bytes
                        state
                            .metrics
                            .system_disk_write_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.sectors_written as f64 * 512.0);

                        // I/O time in seconds (convert from milliseconds)
                        state
                            .metrics
                            .system_disk_io_time_seconds_total
                            .with_label_values(&[&device])
                            .set(stats.time_io_ms as f64 / 1000.0);

                        // Queue depth (I/Os in progress)
                        state
                            .metrics
                            .system_disk_queue_depth
                            .with_label_values(&[&device])
                            .set(stats.ios_in_progress as f64);
                    }
                }
                Err(e) => {
                    warn!("Failed to read disk statistics: {}", e);
                }
            }

            // ========== PHASE 6: System-Level Network Metrics ==========
            match collectors::netdev::read_netdev_stats() {
                Ok(netdevs) => {
                    for (device, stats) in netdevs {
                        // RX bytes
                        state
                            .metrics
                            .system_net_rx_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.receive_bytes as f64);

                        // TX bytes
                        state
                            .metrics
                            .system_net_tx_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.transmit_bytes as f64);

                        // RX errors
                        state
                            .metrics
                            .system_net_rx_errors_total
                            .with_label_values(&[&device])
                            .set(stats.receive_errs as f64);

                        // TX errors
                        state
                            .metrics
                            .system_net_tx_errors_total
                            .with_label_values(&[&device])
                            .set(stats.transmit_errs as f64);

                        // RX drops
                        state
                            .metrics
                            .system_net_drops_total
                            .with_label_values(&[device.as_str(), "rx"])
                            .set(stats.receive_drop as f64);

                        // TX drops
                        state
                            .metrics
                            .system_net_drops_total
                            .with_label_values(&[device.as_str(), "tx"])
                            .set(stats.transmit_drop as f64);
                    }
                }
                Err(e) => {
                    warn!("Failed to read network device statistics: {}", e);
                }
            }

            // ========== PHASE 6.5: System-Level Filesystem Metrics ==========
            if cfg.enable_filesystem_collector.unwrap_or(true) {
                match collectors::filesystem::read_filesystem_stats() {
                    Ok(filesystems) => {
                        for fs in filesystems {
                            state
                                .metrics
                                .system_filesystem_avail_bytes
                                .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                                .set(fs.available_bytes as f64);

                            state
                                .metrics
                                .system_filesystem_size_bytes
                                .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                                .set(fs.size_bytes as f64);

                            state
                                .metrics
                                .system_filesystem_files
                                .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                                .set(fs.files_total as f64);

                            state
                                .metrics
                                .system_filesystem_files_free
                                .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                                .set(fs.files_free as f64);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read filesystem statistics: {}", e);
                    }
                }
            }

            // ========== PHASE 7: Hardware/Host Metrics ==========
            // Thermal sensors
            if cfg.enable_thermal_collector.unwrap_or(true) {
                match collectors::thermal::collect_temperatures() {
                    Ok(temperatures) => {
                        for (sensor, temp) in temperatures {
                            state
                                .metrics
                                .system_cpu_temp_celsius
                                .with_label_values(&[&sensor])
                                .set(temp);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read thermal sensors: {}", e);
                    }
                }
            }

            // Uptime
            match system::read_uptime() {
                Ok(uptime) => {
                    state.metrics.system_uptime_seconds.set(uptime);
                }
                Err(e) => {
                    warn!("Failed to read system uptime: {}", e);
                }
            }

            // Boot time, context switches, and forks from /proc/stat
            match system::read_stat_counters() {
                Ok((boot_time, context_switches, forks)) => {
                    state.metrics.system_boot_time_seconds.set(boot_time as f64);
                    state
                        .metrics
                        .system_context_switches_total
                        .set(context_switches as f64);
                    state.metrics.system_forks_total.set(forks as f64);
                }
                Err(e) => warn!("Failed to read stat counters: {}", e),
            }

            // Uname info
            match system::read_uname_info() {
                Ok((sysname, release, version, machine)) => {
                    state
                        .metrics
                        .system_uname_info
                        .with_label_values(&[&sysname, &release, &version, &machine])
                        .set(1.0);
                }
                Err(e) => warn!("Failed to read uname info: {}", e),
            }

            // ========== PHASE 8: Kernel/Runtime Metrics ==========
            // File descriptors
            match system::read_system_fd_stats() {
                Ok((open_fds, _unused_fds, max_fds)) => {
                    state
                        .metrics
                        .system_open_fds
                        .with_label_values(&["allocated"])
                        .set(open_fds as f64);
                    state
                        .metrics
                        .system_open_fds
                        .with_label_values(&["max"])
                        .set(max_fds as f64);
                }
                Err(e) => {
                    warn!("Failed to read system FD stats: {}", e);
                }
            }

            // Entropy
            match system::read_entropy() {
                Ok(entropy) => {
                    state.metrics.system_entropy_bits.set(entropy as f64);
                }
                Err(e) => warn!("Failed to read entropy: {}", e),
            }

            // ========== PHASE 9: PSI (Pressure Stall Information) Metrics ==========
            if let Ok(cpu_psi) = system::read_psi_some_total("/proc/pressure/cpu") {
                state.metrics.system_cpu_psi_wait_seconds_total.set(cpu_psi);
            }
            if let Ok(mem_psi) = system::read_psi_some_total("/proc/pressure/memory") {
                state
                    .metrics
                    .system_memory_psi_wait_seconds_total
                    .set(mem_psi);
            }
            if let Ok(io_psi) = system::read_psi_some_total("/proc/pressure/io") {
                state.metrics.system_disk_psi_wait_seconds_total.set(io_psi);
            }

            // ========== PHASE 10: eBPF Group Network Metrics (if available) ==========
            #[cfg(feature = "ebpf")]
            if let Some(ebpf) = &state.ebpf {
                match ebpf.read_process_net_stats() {
                    Ok(net_stats) => {
                        // Aggregated per (group, subgroup)
                        let mut net_groups: HashMap<(String, String), (u64, u64)> = HashMap::new();

                        for stat in net_stats {
                            let (group, subgroup) =
                                crate::process::classify_process_raw(&stat.comm);
                            let entry = net_groups
                                .entry((group.to_string(), subgroup.to_string()))
                                .or_insert((0, 0));

                            entry.0 += stat.rx_bytes;
                            entry.1 += stat.tx_bytes;
                        }

                        for ((group, subgroup), (rx, tx)) in net_groups {
                            state
                                .metrics
                                .group_net_rx_bytes_total
                                .with_label_values(&[&group, &subgroup])
                                .set(rx as f64);

                            state
                                .metrics
                                .group_net_tx_bytes_total
                                .with_label_values(&[&group, &subgroup])
                                .set(tx as f64);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read eBPF network statistics: {}", e);
                    }
                }

                // NOTE: Group network connections tracking requires eBPF-based
                // connection state tracking which is not yet implemented.
                // The metric group_net_connections_total{proto="tcp/udp"} will be
                // added in a future enhancement.
            }

            // ========== PHASE 10.5: TCP Connection Statistics (eBPF) ==========
            #[cfg(feature = "ebpf")]
            if let Some(ebpf) = &state.ebpf {
                if cfg.enable_tcp_tracking.unwrap_or(true) {
                    match ebpf.read_tcp_stats() {
                        Ok(tcp_stats) => {
                            state
                                .metrics
                                .system_tcp_connections_established
                                .set(tcp_stats.established as f64);
                            state
                                .metrics
                                .system_tcp_connections_syn_sent
                                .set(tcp_stats.syn_sent as f64);
                            state
                                .metrics
                                .system_tcp_connections_syn_recv
                                .set(tcp_stats.syn_recv as f64);
                            state
                                .metrics
                                .system_tcp_connections_fin_wait1
                                .set(tcp_stats.fin_wait1 as f64);
                            state
                                .metrics
                                .system_tcp_connections_fin_wait2
                                .set(tcp_stats.fin_wait2 as f64);
                            state
                                .metrics
                                .system_tcp_connections_time_wait
                                .set(tcp_stats.time_wait as f64);
                            state
                                .metrics
                                .system_tcp_connections_close
                                .set(tcp_stats.close as f64);
                            state
                                .metrics
                                .system_tcp_connections_close_wait
                                .set(tcp_stats.close_wait as f64);
                            state
                                .metrics
                                .system_tcp_connections_last_ack
                                .set(tcp_stats.last_ack as f64);
                            state
                                .metrics
                                .system_tcp_connections_listen
                                .set(tcp_stats.listen as f64);
                            state
                                .metrics
                                .system_tcp_connections_closing
                                .set(tcp_stats.closing as f64);
                        }
                        Err(e) => {
                            warn!("Failed to read TCP connection statistics: {}", e);
                        }
                    }
                }
            }

            // ========== PHASE 11: Encode and Return Metrics ==========
            let serialize_start = Instant::now();
            let families = state.registry.gather();

            // Calculate label cardinality
            let mut label_count: u64 = 0;
            for family in &families {
                for metric in family.get_metric() {
                    label_count += metric.get_label().len() as u64;
                }
            }
            state.health_stats.record_label_cardinality(label_count);

            let mut buffer = Vec::with_capacity(BUFFER_CAP);
            let encoder = TextEncoder::new();

            if encoder.encode(&families, &mut buffer).is_err() {
                error!("Failed to encode Prometheus metrics");
                return Err(MetricsError::EncodingFailed);
            }

            let serialization_ms = serialize_start.elapsed().as_secs_f64() * 1000.0;
            state
                .health_stats
                .record_serialization_duration_ms(serialization_ms);

            // Record response size
            let response_size_kb = buffer.len() as f64 / 1024.0;
            state
                .health_stats
                .record_metrics_response_size_kb(response_size_kb);

            // Count time series
            let time_series_count =
                families.iter().map(|f| f.get_metric().len()).sum::<usize>() as u64;
            state
                .health_stats
                .record_total_time_series(time_series_count);

            // Record metrics request statistics
            let request_duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            state.health_stats.record_metrics_endpoint_call();
            state
                .health_stats
                .record_request_duration(request_duration_ms);
            state.health_stats.record_http_request();
            state.health_stats.record_cache_hit();

            state.scrape_duration.set(start.elapsed().as_secs_f64());

            debug!(
                "Metrics request completed: {} processes, {} bytes, {:.3}ms",
                exported_count,
                buffer.len(),
                request_duration_ms
            );

            return String::from_utf8(buffer).map_err(|_| MetricsError::EncodingFailed);
        }

        drop(cache_guard);
        // Wait for notification that cache update is complete
        state.cache_ready.notified().await;
    }
}
