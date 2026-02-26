//! Cache update logic for the metrics exporter.
//!
//! This module provides the cache update functionality that can be triggered
//! both by the background periodic task and on-demand by the metrics endpoint.

use ahash::AHashMap as HashMap;
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tracing::{debug, error, info, instrument, warn};

use crate::cache::ProcMem;
use crate::commands::generate::load_test_data_from_file;
use crate::process::{
    classify_process_raw, collect_proc_entries, get_cpu_stat_for_pid, parse_memory_for_process,
    parse_start_time_seconds, read_block_io, read_process_name, read_vmswap,
    should_include_process, MAX_IO_BUFFER_BYTES, MAX_SMAPS_BUFFER_BYTES,
    MAX_SMAPS_ROLLUP_BUFFER_BYTES,
};
use crate::ringbuffer::{RingbufferEntry, TopProcessInfo};
use crate::state::SharedState;
use crate::system;

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

/// Reads the exporter's own memory and CPU usage from /proc/self.
fn read_self_resources() -> (f64, f64) {
    let memory_mb = read_self_memory_mb().unwrap_or(0.0);
    let cpu_percent = read_self_cpu_percent().unwrap_or(0.0);
    (memory_mb, cpu_percent)
}

/// Reads the exporter's RSS memory usage from /proc/self/status.
fn read_self_memory_mb() -> Option<f64> {
    let content = std::fs::read_to_string("/proc/self/status").ok()?;
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
    use crate::process::CLK_TCK;

    let content = std::fs::read_to_string("/proc/self/stat").ok()?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() <= 14 {
        return None;
    }

    let utime: f64 = parts[13].parse().ok()?;
    let stime: f64 = parts[14].parse().ok()?;
    let total_ticks = utime + stime;

    let uptime_content = std::fs::read_to_string("/proc/uptime").ok()?;
    let uptime_seconds: f64 = uptime_content.split_whitespace().next()?.parse().ok()?;

    if uptime_seconds > 0.0 {
        let cpu_time_seconds = total_ticks / *CLK_TCK;
        Some((cpu_time_seconds / uptime_seconds) * 100.0)
    } else {
        None
    }
}

/// Cache update function.
/// This function can be called both by the background periodic task and on-demand.
#[instrument(skip(state))]
pub async fn update_cache(state: &SharedState) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Check if an update is already in progress - if so, serve stale cache
    // This is important for on-demand updates triggered by multiple concurrent /metrics requests
    {
        let mut cache = state.cache.write().await;
        if cache.is_updating {
            debug!("Cache update already in progress, serving stale cache");
            return Ok(());
        }
        cache.is_updating = true;
        cache.update_success = false;
        state.cache_updating.set(1.0);
        debug!("Cache marked as updating (old snapshot still available)");
    }

    info!("Starting cache update");

    // Get current timestamp for rate calculations
    let current_time = chrono::Utc::now().timestamp() as f64;

    let min_uss_bytes = state.config.min_uss_kb.unwrap_or(0) * 1024;

    let included_count = AtomicUsize::new(0);
    let skipped_count = AtomicUsize::new(0);

    // Clone previous cache for I/O rate delta calculation
    let previous_cache: HashMap<u32, ProcMem> = {
        let cache = state.cache.read().await;
        cache.processes.clone()
    };

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
                        state.health_stats.record_proc_read_error();
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

                let parse_start = Instant::now();
                match parse_memory_for_process(&entry.proc_path, &state.buffer_config) {
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
                        let (read_bytes, write_bytes) = read_block_io(&entry.proc_path).unwrap_or((0, 0));

                        // Get previous I/O values from cache (if exists)
                        let (last_read_bytes, last_write_bytes, last_rx_bytes, last_tx_bytes, last_update_time) =
                            if let Some(prev) = previous_cache.get(&entry.pid) {
                                // Use previous values as baseline for rate calculation
                                (prev.read_bytes, prev.write_bytes, prev.rx_bytes, prev.tx_bytes, prev.last_update_time)
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
                for stat in net_stats {
                    if let Some(proc) = results.iter_mut().find(|p| p.pid == stat.pid) {
                        // Get previous network I/O from cache
                        let (last_rx, last_tx, _last_time) =
                            if let Some(prev) = previous_cache.get(&stat.pid) {
                                (prev.rx_bytes, prev.tx_bytes, prev.last_update_time)
                            } else {
                                // First time seeing network stats for this process
                                (stat.rx_bytes, stat.tx_bytes, current_time)
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
        processes_by_subgroup
            .entry(key)
            .or_insert_with(Vec::new)
            .push(p);
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

        // Total CPU usage for this subgroup (sum of all processes)
        let cpu_percent = agg_data.cpu_percent_sum as f32;

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
            cpu_percent,
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
            cpu_percent
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

    info!(
        "Cache update completed: {} processes (subgroup filters applied at scrape), {} total scanned, {:.2}ms",
        results.len(),
        final_included + final_skipped,
        start.elapsed().as_secs_f64() * 1000.0
    );

    Ok(())
}
