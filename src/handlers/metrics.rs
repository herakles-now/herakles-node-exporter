//! Metrics endpoint handler for Prometheus scraping.
//!
//! This module provides the `/metrics` endpoint handler that formats and returns
//! process metrics in Prometheus text format.

use ahash::AHashMap as HashMap;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, instrument, warn};

use crate::cache::ProcMem;
use crate::collectors;
use crate::process::{classify_process_raw, classify_process_with_config};
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

            // Get uptime for this scrape cycle (constant for all metrics)
            let uptime_seconds = state.health_stats.get_uptime_seconds().to_string();

            let cfg = &state.config;
            let enable_rss = cfg.enable_rss.unwrap_or(true);
            let enable_pss = cfg.enable_pss.unwrap_or(true);
            let enable_uss = cfg.enable_uss.unwrap_or(true);
            let enable_cpu = cfg.enable_cpu.unwrap_or(true);

            // Aggregation map
            let mut groups: HashMap<(Arc<str>, Arc<str>), Vec<&ProcMem>> = HashMap::new();
            let mut exported_count = 0usize;

            // Enforce an overall limit for processes classified as "other".
            let mut other_exported = 0usize;
            let other_limit = state.config.top_n_others.unwrap_or(10);

            // Populate aggregation (no longer exporting per-process metrics)
            for p in &processes_vec {
                if let Some((group, subgroup)) =
                    classify_process_with_config(&p.name, &state.config)
                {
                    // If this is the "other" group, enforce the configured per-group limit.
                    if group.as_ref().eq_ignore_ascii_case("other") {
                        if other_exported >= other_limit {
                            continue;
                        }
                        other_exported += 1;
                    }

                    exported_count += 1;

                    // Removed: per-process metric export - no longer setting individual process metrics
                    // Data collection continues but metrics are not exported to /metrics endpoint

                    groups.entry((group, subgroup)).or_default().push(p);
                }
            }

            state.processes_total.set(exported_count as f64);
            state.scrape_duration.set(start.elapsed().as_secs_f64());

            // Aggregated sums and Top-N metrics per subgroup
            for ((group, subgroup), mut list) in groups {
                let mut rss_sum: u64 = 0;
                let mut pss_sum: u64 = 0;
                let mut uss_sum: u64 = 0;
                let mut cpu_percent_sum: f64 = 0.0;
                let mut cpu_time_sum: f64 = 0.0;
                let mut swap_sum: u64 = 0;

                for p in &list {
                    rss_sum += p.rss;
                    pss_sum += p.pss;
                    uss_sum += p.uss;
                    cpu_percent_sum += p.cpu_percent as f64;
                    cpu_time_sum += p.cpu_time_seconds as f64;
                    swap_sum += p.vmswap;
                }

                let group_ref: &str = group.as_ref();
                let subgroup_ref: &str = subgroup.as_ref();

                // Set new subgroup-level aggregated metrics (without uptime label)
                if enable_rss {
                    state
                        .metrics
                        .mem_rss_subgroup_bytes
                        .with_label_values(&[subgroup_ref])
                        .set(rss_sum as f64);
                }
                if enable_pss {
                    state
                        .metrics
                        .mem_pss_subgroup_bytes
                        .with_label_values(&[subgroup_ref])
                        .set(pss_sum as f64);
                }
                if enable_uss {
                    state
                        .metrics
                        .mem_uss_subgroup_bytes
                        .with_label_values(&[subgroup_ref])
                        .set(uss_sum as f64);
                }
                state
                    .metrics
                    .mem_swap_subgroup_bytes
                    .with_label_values(&[subgroup_ref])
                    .set(swap_sum as f64);

                if enable_cpu {
                    state
                        .metrics
                        .cpu_usage_subgroup_percent
                        .with_label_values(&[subgroup_ref])
                        .set(cpu_percent_sum);

                    // Note: CPU iowait at subgroup level is not currently tracked per-process
                    // Set to 0 for now as a placeholder
                    state
                        .metrics
                        .cpu_iowait_subgroup_percent
                        .with_label_values(&[subgroup_ref])
                        .set(0.0);
                }

                // TODO: Calculate subgroup-level I/O and network rates
                // These require tracking previous values and calculating deltas per subgroup
                // For now, set to 0 as placeholders
                state
                    .metrics
                    .io_read_subgroup_bytes_per_second
                    .with_label_values(&[subgroup_ref])
                    .set(0.0);
                state
                    .metrics
                    .io_write_subgroup_bytes_per_second
                    .with_label_values(&[subgroup_ref])
                    .set(0.0);
                state
                    .metrics
                    .net_rx_subgroup_bytes_per_second
                    .with_label_values(&[subgroup_ref])
                    .set(0.0);
                state
                    .metrics
                    .net_tx_subgroup_bytes_per_second
                    .with_label_values(&[subgroup_ref])
                    .set(0.0);

                // Set subgroup metadata metrics
                state
                    .metrics
                    .subgroup_info
                    .with_label_values(&[group_ref, subgroup_ref])
                    .set(1.0);

                // Calculate oldest uptime in the subgroup
                // The oldest process is the one with the earliest start_time_seconds (smallest value)
                // which translates to the maximum uptime: max(system_uptime - start_time_seconds)
                let system_uptime = system::read_uptime().unwrap_or(0.0);

                let oldest_uptime = if list.is_empty() {
                    0.0
                } else {
                    list.iter()
                        .map(|p| system_uptime - p.start_time_seconds)
                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(0.0)
                };

                state
                    .metrics
                    .subgroup_oldest_uptime_seconds
                    .with_label_values(&[subgroup_ref])
                    .set(oldest_uptime);

                // Alert armed status (not currently implemented, default to 0)
                state
                    .metrics
                    .subgroup_alert_armed
                    .with_label_values(&[subgroup_ref])
                    .set(0.0);

                // Set new Top-3 metrics (separate metrics for top1, top2, top3)
                // Sort by RSS for RSS Top-3
                let mut rss_sorted_list = list.clone();
                rss_sorted_list.sort_by_key(|p| std::cmp::Reverse(p.rss));

                if enable_rss && rss_sorted_list.len() >= 1 {
                    let p = &rss_sorted_list[0];
                    state
                        .metrics
                        .mem_rss_subgroup_top1_bytes
                        .with_label_values(&[subgroup_ref])
                        .set(p.rss as f64);
                    state
                        .metrics
                        .mem_rss_subgroup_top1_comm
                        .with_label_values(&[subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_rss && rss_sorted_list.len() >= 2 {
                    let p = &rss_sorted_list[1];
                    state
                        .metrics
                        .mem_rss_subgroup_top2_bytes
                        .with_label_values(&[subgroup_ref])
                        .set(p.rss as f64);
                    state
                        .metrics
                        .mem_rss_subgroup_top2_comm
                        .with_label_values(&[subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_rss && rss_sorted_list.len() >= 3 {
                    let p = &rss_sorted_list[2];
                    state
                        .metrics
                        .mem_rss_subgroup_top3_bytes
                        .with_label_values(&[subgroup_ref])
                        .set(p.rss as f64);
                    state
                        .metrics
                        .mem_rss_subgroup_top3_comm
                        .with_label_values(&[subgroup_ref, &p.name])
                        .set(1.0);
                }

                // Sort by CPU percent for CPU Top-3
                let mut cpu_sorted_list = list.clone();
                cpu_sorted_list.sort_by(|a, b| {
                    b.cpu_percent
                        .partial_cmp(&a.cpu_percent)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                if enable_cpu && cpu_sorted_list.len() >= 1 {
                    let p = &cpu_sorted_list[0];
                    state
                        .metrics
                        .cpu_usage_subgroup_top1_percent
                        .with_label_values(&[subgroup_ref])
                        .set(p.cpu_percent as f64);
                    state
                        .metrics
                        .cpu_usage_subgroup_top1_comm
                        .with_label_values(&[subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_cpu && cpu_sorted_list.len() >= 2 {
                    let p = &cpu_sorted_list[1];
                    state
                        .metrics
                        .cpu_usage_subgroup_top2_percent
                        .with_label_values(&[subgroup_ref])
                        .set(p.cpu_percent as f64);
                    state
                        .metrics
                        .cpu_usage_subgroup_top2_comm
                        .with_label_values(&[subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_cpu && cpu_sorted_list.len() >= 3 {
                    let p = &cpu_sorted_list[2];
                    state
                        .metrics
                        .cpu_usage_subgroup_top3_percent
                        .with_label_values(&[subgroup_ref])
                        .set(p.cpu_percent as f64);
                    state
                        .metrics
                        .cpu_usage_subgroup_top3_comm
                        .with_label_values(&[subgroup_ref, &p.name])
                        .set(1.0);
                }
            }

            // Group Core Metrics - Aggregate all processes by (group, subgroup)
            let mut group_aggregations: HashMap<(String, String), (u64, u64, u64, f64, f64, f64)> =
                HashMap::new();

            for p in &processes_vec {
                if let Some((group, subgroup)) =
                    classify_process_with_config(&p.name, &state.config)
                {
                    let entry = group_aggregations
                        .entry((group.to_string(), subgroup.to_string()))
                        .or_insert((0, 0, 0, 0.0, 0.0, f64::MAX));

                    entry.0 += p.rss; // RSS
                    entry.1 += p.pss; // PSS
                    entry.2 += p.uss; // USS
                    entry.3 += p.cpu_percent as f64; // CPU %
                    entry.4 += p.cpu_time_seconds as f64; // CPU time
                    entry.5 = entry.5.min(p.start_time_seconds); // Oldest start time
                }
            }

            let system_uptime = system::read_uptime().unwrap_or(0.0);

            for ((group, subgroup), (rss, pss, uss, cpu_pct, cpu_time, oldest_start)) in
                group_aggregations
            {
                if enable_rss {
                    state
                        .metrics
                        .group_memory_rss_bytes_sum
                        .with_label_values(&[&group, &subgroup])
                        .set(rss as f64);
                }

                if enable_pss {
                    state
                        .metrics
                        .group_memory_pss_bytes_sum
                        .with_label_values(&[&group, &subgroup])
                        .set(pss as f64);
                }

                if enable_uss {
                    state
                        .metrics
                        .group_memory_uss_bytes_sum
                        .with_label_values(&[&group, &subgroup])
                        .set(uss as f64);
                }

                if enable_cpu {
                    state
                        .metrics
                        .group_cpu_usage_percent_sum
                        .with_label_values(&[&group, &subgroup])
                        .set(cpu_pct);

                    state
                        .metrics
                        .group_cpu_time_seconds_sum
                        .with_label_values(&[&group, &subgroup])
                        .set(cpu_time);

                    // Uptime = NOW - oldest_start_time
                    let uptime = if oldest_start < f64::MAX {
                        system_uptime - oldest_start
                    } else {
                        0.0
                    };
                    state
                        .metrics
                        .group_cpu_uptime_oldest_process_seconds
                        .with_label_values(&[&group, &subgroup])
                        .set(uptime);
                }
            }

            // Set node-level metrics
            // Uptime
            match system::read_uptime() {
                Ok(uptime) => {
                    state.metrics.node_uptime_seconds.set(uptime);
                    state.metrics.system_uptime_seconds.set(uptime);
                }
                Err(e) => {
                    warn!("Failed to read system uptime: {}", e);
                }
            }

            // Read boot time, context switches, and forks from /proc/stat
            match system::read_stat_counters() {
                Ok((boot_time, context_switches, forks)) => {
                    state.metrics.system_boot_time_seconds.set(boot_time as f64);
                    state.metrics.system_context_switches_total.set(context_switches as f64);
                    state.metrics.system_forks_total.set(forks as f64);
                }
                Err(e) => warn!("Failed to read stat counters: {}", e),
            }

            // Read entropy
            match system::read_entropy() {
                Ok(entropy) => {
                    state.metrics.system_entropy_bits.set(entropy as f64);
                }
                Err(e) => warn!("Failed to read entropy: {}", e),
            }

            // Read PSI (Pressure Stall Information)
            if let Ok(cpu_psi) = system::read_psi_some_total("/proc/pressure/cpu") {
                state.metrics.system_cpu_psi_wait_seconds_total.set(cpu_psi);
            }
            if let Ok(mem_psi) = system::read_psi_some_total("/proc/pressure/memory") {
                state.metrics.system_memory_psi_wait_seconds_total.set(mem_psi);
            }
            if let Ok(io_psi) = system::read_psi_some_total("/proc/pressure/io") {
                state.metrics.system_disk_psi_wait_seconds_total.set(io_psi);
            }

            // File descriptors
            match system::read_system_fd_stats() {
                Ok((open_fds, _unused_fds, max_fds)) => {
                    state.metrics.node_fd_open.set(open_fds as f64);
                    state.metrics.node_fd_kernel_max.set(max_fds as f64);
                    state.metrics.system_open_fds.set(open_fds as f64);
                    if max_fds > 0 {
                        let used_ratio = open_fds as f64 / max_fds as f64;
                        state.metrics.node_fd_used_ratio.set(used_ratio);
                    } else {
                        state.metrics.node_fd_used_ratio.set(0.0);
                    }
                }
                Err(e) => {
                    warn!("Failed to read system FD stats: {}", e);
                }
            }

            // Load averages
            match system::read_load_average() {
                Ok(load_avg) => {
                    state.metrics.node_load1.set(load_avg.one_min);
                    state.metrics.node_load5.set(load_avg.five_min);
                    state.metrics.node_load15.set(load_avg.fifteen_min);
                }
                Err(e) => {
                    warn!("Failed to read load average: {}", e);
                }
            }

            // Memory metrics
            match system::read_extended_memory_info() {
                Ok(mem_info) => {
                    state
                        .metrics
                        .node_mem_total_bytes
                        .set(mem_info.total_bytes as f64);
                    state
                        .metrics
                        .node_mem_available_bytes
                        .set(mem_info.available_bytes as f64);
                    state
                        .metrics
                        .node_mem_cached_bytes
                        .set(mem_info.cached_bytes as f64);
                    state
                        .metrics
                        .node_mem_buffers_bytes
                        .set(mem_info.buffers_bytes as f64);
                    state
                        .metrics
                        .node_mem_swap_total_bytes
                        .set(mem_info.swap_total_bytes as f64);

                    // Calculate used memory
                    let used_bytes = mem_info
                        .total_bytes
                        .saturating_sub(mem_info.available_bytes);
                    state.metrics.node_mem_used_bytes.set(used_bytes as f64);

                    // Calculate used swap
                    let swap_used_bytes = mem_info
                        .swap_total_bytes
                        .saturating_sub(mem_info.swap_free_bytes);
                    state
                        .metrics
                        .node_mem_swap_used_bytes
                        .set(swap_used_bytes as f64);
                }
                Err(e) => {
                    warn!("Failed to read extended memory info: {}", e);
                }
            }

            // CPU metrics (aggregate across all cores)
            match state.system_cpu_cache.calculate_usage_ratios() {
                Ok(cpu_ratios) => {
                    // Get the "cpu" (total) values
                    if let Some(&usage_ratio) = cpu_ratios.usage.get("cpu") {
                        state
                            .metrics
                            .node_cpu_usage_percent
                            .set(usage_ratio * 100.0);
                    }
                    if let Some(&iowait_ratio) = cpu_ratios.iowait.get("cpu") {
                        state
                            .metrics
                            .node_cpu_iowait_percent
                            .set(iowait_ratio * 100.0);
                    }
                    if let Some(&steal_ratio) = cpu_ratios.steal.get("cpu") {
                        state
                            .metrics
                            .node_cpu_steal_percent
                            .set(steal_ratio * 100.0);
                    }
                }
                Err(e) => {
                    warn!("Failed to calculate CPU usage ratios: {}", e);
                }
            }

            // System Ratios
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
                    warn!("Failed to calculate CPU ratios for system metrics: {}", e);
                }
            }

            // Memory Ratios
            match system::read_extended_memory_info() {
                Ok(mem_info) => {
                    if mem_info.total_bytes > 0 {
                        let mem_used_ratio = (mem_info.total_bytes - mem_info.available_bytes)
                            as f64
                            / mem_info.total_bytes as f64;
                        state.metrics.system_memory_used_ratio.set(mem_used_ratio);
                    }

                    if mem_info.swap_total_bytes > 0 {
                        let swap_used_ratio = (mem_info.swap_total_bytes
                            - mem_info.swap_free_bytes)
                            as f64
                            / mem_info.swap_total_bytes as f64;
                        state
                            .metrics
                            .system_memory_swap_used_ratio
                            .set(swap_used_ratio);
                    } else {
                        state.metrics.system_memory_swap_used_ratio.set(0.0);
                    }
                }
                Err(e) => {
                    warn!("Failed to read memory info for ratios: {}", e);
                }
            }

            // Disk Device-Level Metrics
            match collectors::diskstats::read_diskstats() {
                Ok(diskstats) => {
                    for (device, stats) in diskstats {
                        state
                            .metrics
                            .system_disk_reads_completed_total
                            .with_label_values(&[&device])
                            .set(stats.reads_completed as f64);

                        state
                            .metrics
                            .system_disk_read_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.sectors_read as f64 * 512.0);

                        state
                            .metrics
                            .system_disk_writes_completed_total
                            .with_label_values(&[&device])
                            .set(stats.writes_completed as f64);

                        state
                            .metrics
                            .system_disk_write_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.sectors_written as f64 * 512.0);

                        state
                            .metrics
                            .system_disk_io_now
                            .with_label_values(&[&device])
                            .set(stats.ios_in_progress as f64);
                    }
                }
                Err(e) => {
                    warn!("Failed to read disk statistics: {}", e);
                }
            }

            // Filesystem Metrics
            match collectors::filesystem::read_filesystem_stats() {
                Ok(filesystems) => {
                    for fs in filesystems {
                        state
                            .metrics
                            .filesystem_avail_bytes
                            .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                            .set(fs.available_bytes as f64);

                        state
                            .metrics
                            .filesystem_size_bytes
                            .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                            .set(fs.size_bytes as f64);

                        state
                            .metrics
                            .filesystem_files
                            .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                            .set(fs.files_total as f64);

                        state
                            .metrics
                            .filesystem_files_free
                            .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                            .set(fs.files_free as f64);
                    }
                }
                Err(e) => {
                    warn!("Failed to read filesystem statistics: {}", e);
                }
            }

            // Network Device-Level Metrics
            match collectors::netdev::read_netdev_stats() {
                Ok(netdevs) => {
                    for (device, stats) in netdevs {
                        state
                            .metrics
                            .system_net_receive_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.receive_bytes as f64);

                        state
                            .metrics
                            .system_net_transmit_bytes_total
                            .with_label_values(&[&device])
                            .set(stats.transmit_bytes as f64);

                        state
                            .metrics
                            .system_net_receive_packets_total
                            .with_label_values(&[&device])
                            .set(stats.receive_packets as f64);

                        state
                            .metrics
                            .system_net_receive_errs_total
                            .with_label_values(&[&device])
                            .set(stats.receive_errs as f64);

                        state
                            .metrics
                            .system_net_receive_drop_total
                            .with_label_values(&[&device])
                            .set(stats.receive_drop as f64);
                    }
                }
                Err(e) => {
                    warn!("Failed to read network device statistics: {}", e);
                }
            }

            // eBPF Group Network Aggregation
            if let Some(ebpf) = &state.ebpf {
                match ebpf.read_process_net_stats() {
                    Ok(net_stats) => {
                        // Aggregated per (group, subgroup)
                        let mut net_groups: HashMap<(String, String), (u64, u64, u64, u64)> =
                            HashMap::new();

                        for stat in net_stats {
                            let (group, subgroup) = classify_process_raw(&stat.comm);
                            let entry = net_groups
                                .entry((group.to_string(), subgroup.to_string()))
                                .or_insert((0, 0, 0, 0));

                            entry.0 += stat.rx_bytes;
                            entry.1 += stat.tx_bytes;
                            entry.2 += stat.rx_packets + stat.tx_packets;
                            entry.3 += stat.dropped;
                        }

                        for ((group, subgroup), (rx, tx, packets, dropped)) in net_groups {
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

                            state
                                .metrics
                                .group_net_packets_total
                                .with_label_values(&[&group, &subgroup])
                                .set(packets as f64);

                            state
                                .metrics
                                .group_net_dropped_total
                                .with_label_values(&[&group, &subgroup])
                                .set(dropped as f64);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read eBPF network statistics: {}", e);
                    }
                }
            }

            // eBPF Group I/O Aggregation
            if let Some(ebpf) = &state.ebpf {
                match ebpf.read_process_blkio_stats() {
                    Ok(blkio_stats) => {
                        // Aggregated per (group, subgroup)
                        let mut io_groups: HashMap<(String, String), (u64, u64)> = HashMap::new();

                        for stat in blkio_stats {
                            let (group, subgroup) = classify_process_raw(&stat.comm);
                            let entry = io_groups
                                .entry((group.to_string(), subgroup.to_string()))
                                .or_insert((0, 0));

                            entry.0 += stat.read_bytes;
                            entry.1 += stat.write_bytes;
                        }

                        for ((group, subgroup), (read, write)) in io_groups {
                            state
                                .metrics
                                .group_blkio_read_bytes_total
                                .with_label_values(&[&group, &subgroup])
                                .set(read as f64);

                            state
                                .metrics
                                .group_blkio_write_bytes_total
                                .with_label_values(&[&group, &subgroup])
                                .set(write as f64);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read eBPF block I/O statistics: {}", e);
                    }
                }
            }

            // Thermal Sensor metrics
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

            // TODO: Calculate node-level I/O rates (bytes and IOPS per second)
            // These require tracking previous values and calculating deltas
            // For now, set to 0 as placeholders
            state.metrics.node_io_read_bytes_per_second.set(0.0);
            state.metrics.node_io_write_bytes_per_second.set(0.0);
            state.metrics.node_io_read_iops_per_second.set(0.0);
            state.metrics.node_io_write_iops_per_second.set(0.0);

            // TODO: Calculate node-level network rates (bytes and packets per second)
            // These require tracking previous values and calculating deltas
            // For now, set to 0 as placeholders
            state.metrics.node_net_rx_bytes_per_second.set(0.0);
            state.metrics.node_net_tx_bytes_per_second.set(0.0);
            state
                .metrics
                .node_net_rx_dropped_packets_per_second
                .set(0.0);
            state
                .metrics
                .node_net_tx_dropped_packets_per_second
                .set(0.0);
            state.metrics.node_net_rx_error_packets_per_second.set(0.0);
            state.metrics.node_net_tx_error_packets_per_second.set(0.0);

            // TODO: Set subgroup-level I/O and network rates
            // These also require tracking and calculating deltas per subgroup
            // For now, they will be set to 0 in the subgroup loop above or left unset

            // Encode metrics in Prometheus text format
            // Measure serialization time
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

            debug!(
                "Metrics request completed: {} processes (exported {}), {} bytes, {:.3}ms",
                processes_vec.len(),
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
