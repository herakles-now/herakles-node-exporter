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
        state.health_stats.record_lock_wait_duration_ms(lock_wait_ms);

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

                // Set aggregation metrics (respect enable_* flags)
                if enable_rss {
                    state
                        .metrics
                        .agg_rss_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(rss_sum as f64);
                }
                if enable_pss {
                    state
                        .metrics
                        .agg_pss_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(pss_sum as f64);
                }
                if enable_uss {
                    state
                        .metrics
                        .agg_uss_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(uss_sum as f64);
                }
                if enable_cpu {
                    state
                        .metrics
                        .agg_cpu_percent_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(cpu_percent_sum);
                    state
                        .metrics
                        .agg_cpu_time_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(cpu_time_sum);

                    // New CPU group metrics (without uptime label)
                    // CPU usage ratio: cpu_percent / 100 to get 0-1 range
                    state
                        .metrics
                        .cpu_group_usage_ratio
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(cpu_percent_sum / 100.0);

                    // CPU seconds total with mode=user (we don't track kernel time separately)
                    state
                        .metrics
                        .cpu_group_seconds_total
                        .with_label_values(&[group_ref, subgroup_ref, "user"])
                        .set(cpu_time_sum);
                }

                // Set memory group swap metric
                state
                    .metrics
                    .mem_group_swap_bytes
                    .with_label_values(&[group_ref, subgroup_ref])
                    .set(swap_sum as f64);

                // Set new subgroup-level aggregated metrics (without uptime label)
                if enable_rss {
                    state
                        .metrics
                        .mem_rss_subgroup_bytes
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(rss_sum as f64);
                }
                if enable_pss {
                    state
                        .metrics
                        .mem_pss_subgroup_bytes
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(pss_sum as f64);
                }
                if enable_uss {
                    state
                        .metrics
                        .mem_uss_subgroup_bytes
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(uss_sum as f64);
                }
                state
                    .metrics
                    .mem_swap_subgroup_bytes
                    .with_label_values(&[group_ref, subgroup_ref])
                    .set(swap_sum as f64);

                if enable_cpu {
                    state
                        .metrics
                        .cpu_usage_subgroup_percent
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(cpu_percent_sum);
                    
                    // Note: CPU iowait at subgroup level is not currently tracked per-process
                    // Set to 0 for now as a placeholder
                    state
                        .metrics
                        .cpu_iowait_subgroup_percent
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(0.0);
                }

                // Set subgroup metadata metrics
                state
                    .metrics
                    .subgroup_info
                    .with_label_values(&[group_ref, subgroup_ref])
                    .set(1.0);

                // Oldest uptime in subgroup - not currently tracked in ProcMem
                // Set to 0 as a placeholder for now
                state
                    .metrics
                    .subgroup_oldest_uptime_seconds
                    .with_label_values(&[group_ref, subgroup_ref])
                    .set(0.0);

                // Alert armed status (not currently implemented, default to 0)
                state
                    .metrics
                    .subgroup_alert_armed
                    .with_label_values(&[group_ref, subgroup_ref])
                    .set(0.0);

                // Sort by USS for Top-N selection
                list.sort_by_key(|p| std::cmp::Reverse(p.uss));

                let is_other_group = group_ref.eq_ignore_ascii_case("other")
                    || group_ref.eq_ignore_ascii_case("others")
                    || subgroup_ref.eq_ignore_ascii_case("other")
                    || subgroup_ref.eq_ignore_ascii_case("others");

                let top_subgroup = state.config.top_n_subgroup.unwrap_or(3);
                let top_others = state.config.top_n_others.unwrap_or(10);
                let limit = if is_other_group {
                    std::cmp::max(1, top_others)
                } else {
                    std::cmp::max(1, top_subgroup)
                };

                let rss_total = rss_sum as f64;
                let pss_total = pss_sum as f64;
                let uss_total = uss_sum as f64;
                let cpu_total = cpu_time_sum;

                for (rank, p) in list.iter().take(limit).enumerate() {
                    let pid_s = p.pid.to_string();
                    let rank_s = (rank + 1).to_string();
                    let name_s = p.name.as_str();

                    // Absolute Top-N values
                    if enable_rss {
                        state
                            .metrics
                            .top_rss
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.rss as f64);
                    }
                    if enable_pss {
                        state
                            .metrics
                            .top_pss
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.pss as f64);
                    }
                    if enable_uss {
                        state
                            .metrics
                            .top_uss
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.uss as f64);
                    }
                    if enable_cpu {
                        state
                            .metrics
                            .top_cpu_percent
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.cpu_percent as f64);
                        state
                            .metrics
                            .top_cpu_time
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.cpu_time_seconds as f64);
                    }

                    // Percentage-of-subgroup values
                    if enable_cpu && cpu_total > 0.0 {
                        let pct = (p.cpu_time_seconds as f64 / cpu_total) * 100.0;
                        state
                            .metrics
                            .top_cpu_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }

                    if enable_rss && rss_total > 0.0 {
                        let pct = (p.rss as f64 / rss_total) * 100.0;
                        state
                            .metrics
                            .top_rss_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }

                    if enable_pss && pss_total > 0.0 {
                        let pct = (p.pss as f64 / pss_total) * 100.0;
                        state
                            .metrics
                            .top_pss_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }

                    if enable_uss && uss_total > 0.0 {
                        let pct = (p.uss as f64 / uss_total) * 100.0;
                        state
                            .metrics
                            .top_uss_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }

                    // New CPU top process metrics (without uptime label)
                    if enable_cpu {
                        // CPU usage ratio (0-1 range)
                        state
                            .metrics
                            .cpu_top_process_usage_ratio
                            .with_label_values(&[group_ref, subgroup_ref, &rank_s, &pid_s, name_s])
                            .set(p.cpu_percent as f64 / 100.0);

                        // CPU seconds total with mode=user
                        state
                            .metrics
                            .cpu_top_process_seconds_total
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                "user",
                            ])
                            .set(p.cpu_time_seconds as f64);
                    }
                }

                // Set new Top-3 metrics (separate metrics for top1, top2, top3)
                // Sort by RSS for RSS Top-3
                let mut rss_sorted_list = list.clone();
                rss_sorted_list.sort_by_key(|p| std::cmp::Reverse(p.rss));
                
                if enable_rss && rss_sorted_list.len() >= 1 {
                    let p = &rss_sorted_list[0];
                    state.metrics.mem_rss_subgroup_top1_bytes
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.rss as f64);
                    state.metrics.mem_rss_subgroup_top1_pid
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.pid as f64);
                    state.metrics.mem_rss_subgroup_top1_comm
                        .with_label_values(&[group_ref, subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_rss && rss_sorted_list.len() >= 2 {
                    let p = &rss_sorted_list[1];
                    state.metrics.mem_rss_subgroup_top2_bytes
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.rss as f64);
                    state.metrics.mem_rss_subgroup_top2_pid
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.pid as f64);
                    state.metrics.mem_rss_subgroup_top2_comm
                        .with_label_values(&[group_ref, subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_rss && rss_sorted_list.len() >= 3 {
                    let p = &rss_sorted_list[2];
                    state.metrics.mem_rss_subgroup_top3_bytes
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.rss as f64);
                    state.metrics.mem_rss_subgroup_top3_pid
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.pid as f64);
                    state.metrics.mem_rss_subgroup_top3_comm
                        .with_label_values(&[group_ref, subgroup_ref, &p.name])
                        .set(1.0);
                }

                // Sort by CPU percent for CPU Top-3
                let mut cpu_sorted_list = list.clone();
                cpu_sorted_list.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
                
                if enable_cpu && cpu_sorted_list.len() >= 1 {
                    let p = &cpu_sorted_list[0];
                    state.metrics.cpu_usage_subgroup_top1_percent
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.cpu_percent as f64);
                    state.metrics.cpu_usage_subgroup_top1_pid
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.pid as f64);
                    state.metrics.cpu_usage_subgroup_top1_comm
                        .with_label_values(&[group_ref, subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_cpu && cpu_sorted_list.len() >= 2 {
                    let p = &cpu_sorted_list[1];
                    state.metrics.cpu_usage_subgroup_top2_percent
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.cpu_percent as f64);
                    state.metrics.cpu_usage_subgroup_top2_pid
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.pid as f64);
                    state.metrics.cpu_usage_subgroup_top2_comm
                        .with_label_values(&[group_ref, subgroup_ref, &p.name])
                        .set(1.0);
                }
                if enable_cpu && cpu_sorted_list.len() >= 3 {
                    let p = &cpu_sorted_list[2];
                    state.metrics.cpu_usage_subgroup_top3_percent
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.cpu_percent as f64);
                    state.metrics.cpu_usage_subgroup_top3_pid
                        .with_label_values(&[group_ref, subgroup_ref])
                        .set(p.pid as f64);
                    state.metrics.cpu_usage_subgroup_top3_comm
                        .with_label_values(&[group_ref, subgroup_ref, &p.name])
                        .set(1.0);
                }
            }

            // Set node-level metrics
            // Uptime
            match system::read_uptime() {
                Ok(uptime) => {
                    state.metrics.node_uptime_seconds.set(uptime);
                }
                Err(e) => {
                    warn!("Failed to read system uptime: {}", e);
                }
            }

            // File descriptors
            match system::read_system_fd_stats() {
                Ok((open_fds, _unused_fds, max_fds)) => {
                    state.metrics.node_fd_open.set(open_fds as f64);
                    state.metrics.node_fd_max.set(max_fds as f64);
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

            // Update system-wide metrics
            match system::read_load_average() {
                Ok(load_avg) => {
                    // Set load metrics
                    state.metrics.set_system_load_metrics(
                        load_avg.one_min,
                        load_avg.five_min,
                        load_avg.fifteen_min,
                    );
                }
                Err(e) => {
                    warn!("Failed to read load average: {}", e);
                }
            }

            // Set new extended memory metrics
            match system::read_extended_memory_info() {
                Ok(mem_info) => {
                    state.metrics.set_system_memory_metrics(
                        mem_info.total_bytes,
                        mem_info.available_bytes,
                        mem_info.cached_bytes,
                        mem_info.buffers_bytes,
                        mem_info.swap_total_bytes,
                        mem_info.swap_free_bytes,
                    );
                }
                Err(e) => {
                    warn!("Failed to read extended memory info: {}", e);
                }
            }

            // Set CPU usage ratio metrics (including idle, iowait, steal)
            match state.system_cpu_cache.calculate_usage_ratios() {
                Ok(cpu_ratios) => {
                    state.metrics.set_system_cpu_usage_ratios(&cpu_ratios);
                }
                Err(e) => {
                    warn!("Failed to calculate CPU usage ratios: {}", e);
                }
            }

            // Set PSI (Pressure Stall Information) metrics
            let cpu_psi_total = system::read_psi_some_total("/proc/pressure/cpu").unwrap_or(0.0);
            let memory_psi_total =
                system::read_psi_some_total("/proc/pressure/memory").unwrap_or(0.0);
            state
                .metrics
                .set_psi_metrics(cpu_psi_total, memory_psi_total);

            // Collect and update disk statistics
            match crate::collectors::diskstats::read_diskstats() {
                Ok(disk_stats) => {
                    for (device, stats) in disk_stats.iter() {
                        state.metrics.update_disk_stats(device, stats);
                    }
                }
                Err(e) => {
                    warn!("Failed to read disk statistics: {}", e);
                }
            }

            // Collect and update filesystem statistics
            match crate::collectors::filesystem::read_filesystem_stats() {
                Ok(fs_stats) => {
                    for stats in fs_stats.iter() {
                        state.metrics.update_filesystem_stats(stats);
                    }
                }
                Err(e) => {
                    warn!("Failed to read filesystem statistics: {}", e);
                }
            }

            // Collect and update network interface statistics
            match crate::collectors::netdev::read_netdev_stats() {
                Ok(net_stats) => {
                    for (interface, stats) in net_stats.iter() {
                        state.metrics.update_network_stats(interface, stats);
                    }
                }
                Err(e) => {
                    warn!("Failed to read network statistics: {}", e);
                }
            }

            // Collect and update eBPF-based metrics (if enabled)
            if let Some(ref ebpf) = state.ebpf {
                if ebpf.is_enabled() {
                    // Read eBPF data once
                    let net_stats_opt = if state.config.enable_ebpf_network.unwrap_or(true) {
                        match ebpf.read_process_net_stats() {
                            Ok(stats) => {
                                if !stats.is_empty() {
                                    debug!("Collected {} process network I/O stats from eBPF", stats.len());
                                    state.metrics.update_process_network_metrics(&stats);
                                    Some(stats)
                                } else {
                                    Some(Vec::new())
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read eBPF process network stats: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    };

                    let blkio_stats_opt = if state.config.enable_ebpf_disk.unwrap_or(true) {
                        match ebpf.read_process_blkio_stats() {
                            Ok(stats) => {
                                if !stats.is_empty() {
                                    debug!("Collected {} process block I/O stats from eBPF", stats.len());
                                    state.metrics.update_process_blkio_metrics(&stats);
                                    Some(stats)
                                } else {
                                    Some(Vec::new())
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read eBPF process block I/O stats: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    };

                    // Collect TCP connection statistics
                    if state.config.enable_tcp_tracking.unwrap_or(true) {
                        match ebpf.read_tcp_stats() {
                            Ok(tcp_stats) => {
                                state.metrics.update_tcp_metrics(&tcp_stats);
                            }
                            Err(e) => {
                                warn!("Failed to read eBPF TCP stats: {}", e);
                            }
                        }
                    }

                    // Aggregate I/O metrics by subgroup and update top-N (reuse collected data)
                    if net_stats_opt.is_some() || blkio_stats_opt.is_some() {
                        let net_stats = net_stats_opt.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
                        let blkio_stats = blkio_stats_opt.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
                        
                        // Calculate aggregations
                        let (net_agg, blkio_agg) = crate::ebpf::aggregate_io_by_subgroup(net_stats, blkio_stats);
                        state.metrics.update_io_aggregations(&net_agg, &blkio_agg);
                        
                        // Calculate top-N processes
                        let top_n = state.config.top_n_subgroup.unwrap_or(3);
                        let (top_net, top_blkio) = crate::ebpf::calculate_top_io_processes(net_stats, blkio_stats, top_n);
                        state.metrics.update_top_io_processes(&top_net, &top_blkio);
                    }
                }
            }

            // Collect I/O PSI metric
            match crate::collectors::diskstats::read_psi_io() {
                Ok(psi_io) => {
                    state.metrics.system_io_psi_wait_seconds_total.set(psi_io);
                }
                Err(e) => {
                    debug!("Failed to read I/O PSI (may not be available on this system): {}", e);
                }
            }

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
            state.health_stats.record_serialization_duration_ms(serialization_ms);

            // Record response size
            let response_size_kb = buffer.len() as f64 / 1024.0;
            state.health_stats.record_metrics_response_size_kb(response_size_kb);

            // Count time series
            let time_series_count = families.iter()
                .map(|f| f.get_metric().len())
                .sum::<usize>() as u64;
            state.health_stats.record_total_time_series(time_series_count);

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
