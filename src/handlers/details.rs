//! Details endpoint handler.
//!
//! This module provides the `/details` endpoint handler that explains exceptional
//! behavior by comparing live processes against historical baselines.
//! High-cardinality data (PIDs, full command lines) is intentionally exposed here
//! to help identify anomalies that cannot be safely represented as metrics.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::cache::ProcMem;
use crate::handlers::health::FOOTER_TEXT;
use crate::process::classifier::classify_process_raw;
use crate::state::SharedState;

/// Query parameters for the details endpoint.
#[derive(Deserialize, Debug)]
pub struct DetailsQuery {
    pub subgroup: Option<String>,
}

/// Baseline metrics calculated from ringbuffer history.
#[derive(Debug, Clone)]
struct BaselineMetrics {
    min_rss: u64,
    avg_rss: u64,
    max_rss: u64,
    min_pss: u64,
    avg_pss: u64,
    max_pss: u64,
    min_uss: u64,
    avg_uss: u64,
    max_uss: u64,
    history_count: usize,
    time_window_minutes: u64,
}

/// Live snapshot data for a single subgroup.
#[derive(Debug, Clone)]
struct SubgroupSnapshot {
    process_count: usize,
    total_rss: u64,
    total_pss: u64,
    total_uss: u64,
    oldest_uptime_seconds: f64,
    all_processes: Vec<ProcessInfo>,
}

/// Information about a single process for display.
#[derive(Debug, Clone)]
struct ProcessInfo {
    pid: u32,
    name: String,
    rss: u64,
    pss: u64,
    uss: u64,
    cpu_percent: f32,
    uptime_seconds: f64,
    read_bytes: u64,
    write_bytes: u64,
}

/// An outlier process that significantly deviates from baseline.
#[derive(Debug, Clone)]
struct OutlierProcess {
    pid: u32,
    name: String,
    uptime_seconds: f64,
    rss: u64,
    pss: u64,
    uss: u64,
    rss_ratio: f64,
    pss_ratio: f64,
    uss_ratio: f64,
    read_bytes: u64,
    write_bytes: u64,
}

/// Computes live snapshot for all subgroups from the current cache.
async fn compute_live_snapshots(
    state: &SharedState,
) -> HashMap<String, SubgroupSnapshot> {
    let cache = state.cache.read().await;
    let system_uptime = crate::system::read_uptime().unwrap_or(0.0);

    // Group processes by subgroup
    let mut subgroup_procs: HashMap<String, Vec<ProcMem>> = HashMap::new();

    for proc in cache.processes.values() {
        let (group, subgroup) = classify_process_raw(&proc.name);
        let key = format!("{}:{}", group, subgroup);
        subgroup_procs
            .entry(key)
            .or_insert_with(Vec::new)
            .push(proc.clone());
    }

    // Compute snapshot for each subgroup
    let mut snapshots = HashMap::new();

    for (subgroup_key, procs) in subgroup_procs {
        if procs.is_empty() {
            continue;
        }

        let process_count = procs.len();
        let total_rss: u64 = procs.iter().map(|p| p.rss).sum();
        let total_pss: u64 = procs.iter().map(|p| p.pss).sum();
        let total_uss: u64 = procs.iter().map(|p| p.uss).sum();

        // Find oldest process (min start_time_seconds)
        let min_start_time = procs
            .iter()
            .map(|p| p.start_time_seconds)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        let oldest_uptime_seconds = system_uptime - min_start_time;

        // Convert all processes to ProcessInfo
        let all_processes: Vec<ProcessInfo> = procs
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                rss: p.rss,
                pss: p.pss,
                uss: p.uss,
                cpu_percent: p.cpu_percent,
                uptime_seconds: system_uptime - p.start_time_seconds,
                read_bytes: p.read_bytes,
                write_bytes: p.write_bytes,
            })
            .collect();

        snapshots.insert(
            subgroup_key,
            SubgroupSnapshot {
                process_count,
                total_rss,
                total_pss,
                total_uss,
                oldest_uptime_seconds,
                all_processes,
            },
        );
    }

    snapshots
}

/// Calculates baseline metrics from ringbuffer history.
fn calculate_baseline(history: &[crate::ringbuffer::RingbufferEntry], interval_seconds: u64) -> Option<BaselineMetrics> {
    if history.is_empty() {
        return None;
    }

    let count = history.len();
    let time_window_minutes = (count as u64 * interval_seconds) / 60;

    // Calculate min/avg/max for RSS
    let rss_values: Vec<u64> = history.iter().map(|e| e.rss_kb * 1024).collect();
    let min_rss = *rss_values.iter().min().unwrap_or(&0);
    let max_rss = *rss_values.iter().max().unwrap_or(&0);
    let avg_rss = rss_values.iter().sum::<u64>() / count as u64;

    // Calculate min/avg/max for PSS
    let pss_values: Vec<u64> = history.iter().map(|e| e.pss_kb * 1024).collect();
    let min_pss = *pss_values.iter().min().unwrap_or(&0);
    let max_pss = *pss_values.iter().max().unwrap_or(&0);
    let avg_pss = pss_values.iter().sum::<u64>() / count as u64;

    // Calculate min/avg/max for USS
    let uss_values: Vec<u64> = history.iter().map(|e| e.uss_kb * 1024).collect();
    let min_uss = *uss_values.iter().min().unwrap_or(&0);
    let max_uss = *uss_values.iter().max().unwrap_or(&0);
    let avg_uss = uss_values.iter().sum::<u64>() / count as u64;

    Some(BaselineMetrics {
        min_rss,
        avg_rss,
        max_rss,
        min_pss,
        avg_pss,
        max_pss,
        min_uss,
        avg_uss,
        max_uss,
        history_count: count,
        time_window_minutes,
    })
}

/// Identifies outlier processes that significantly deviate from baseline.
/// Uses a threshold of 2.5x the per-process average for any metric.
fn identify_outliers(
    snapshot: &SubgroupSnapshot,
    baseline: &BaselineMetrics,
) -> Vec<OutlierProcess> {
    if snapshot.process_count == 0 {
        return Vec::new();
    }

    // Calculate per-process averages from baseline
    let avg_rss_per_proc = baseline.avg_rss / snapshot.process_count.max(1) as u64;
    let avg_pss_per_proc = baseline.avg_pss / snapshot.process_count.max(1) as u64;
    let avg_uss_per_proc = baseline.avg_uss / snapshot.process_count.max(1) as u64;

    let outlier_threshold = 2.5;

    let mut outliers = Vec::new();

    for proc in &snapshot.all_processes {
        // Check if any metric exceeds threshold
        let rss_ratio = if avg_rss_per_proc > 0 {
            proc.rss as f64 / avg_rss_per_proc as f64
        } else {
            0.0
        };
        
        let pss_ratio = if avg_pss_per_proc > 0 {
            proc.pss as f64 / avg_pss_per_proc as f64
        } else {
            0.0
        };
        
        let uss_ratio = if avg_uss_per_proc > 0 {
            proc.uss as f64 / avg_uss_per_proc as f64
        } else {
            0.0
        };

        // A process is an outlier if any metric exceeds the threshold
        if rss_ratio > outlier_threshold || pss_ratio > outlier_threshold || uss_ratio > outlier_threshold {
            outliers.push(OutlierProcess {
                pid: proc.pid,
                name: proc.name.clone(),
                uptime_seconds: proc.uptime_seconds,
                rss: proc.rss,
                pss: proc.pss,
                uss: proc.uss,
                rss_ratio,
                pss_ratio,
                uss_ratio,
                read_bytes: proc.read_bytes,
                write_bytes: proc.write_bytes,
            });
        }
    }

    // Sort by highest ratio of any metric
    outliers.sort_by(|a, b| {
        let max_a = a.rss_ratio.max(a.pss_ratio).max(a.uss_ratio);
        let max_b = b.rss_ratio.max(b.pss_ratio).max(b.uss_ratio);
        max_b.partial_cmp(&max_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    outliers
}

/// Identifies Block I/O outliers (processes with significantly higher I/O than average).
fn identify_io_outliers(
    snapshot: &SubgroupSnapshot,
) -> Vec<OutlierProcess> {
    if snapshot.process_count == 0 {
        return Vec::new();
    }

    // Calculate average I/O per process
    let total_read: u64 = snapshot.all_processes.iter().map(|p| p.read_bytes).sum();
    let total_write: u64 = snapshot.all_processes.iter().map(|p| p.write_bytes).sum();
    
    let avg_read = total_read / snapshot.process_count as u64;
    let avg_write = total_write / snapshot.process_count as u64;

    let io_threshold = 3.0; // Higher threshold for I/O

    let mut outliers = Vec::new();

    for proc in &snapshot.all_processes {
        let read_ratio = if avg_read > 0 {
            proc.read_bytes as f64 / avg_read as f64
        } else if proc.read_bytes > 0 {
            10.0 // If average is 0 but process has I/O, it's an outlier
        } else {
            0.0
        };

        let write_ratio = if avg_write > 0 {
            proc.write_bytes as f64 / avg_write as f64
        } else if proc.write_bytes > 0 {
            10.0
        } else {
            0.0
        };

        // Consider as outlier if either read or write is significantly above average
        if read_ratio > io_threshold || write_ratio > io_threshold {
            outliers.push(OutlierProcess {
                pid: proc.pid,
                name: proc.name.clone(),
                uptime_seconds: proc.uptime_seconds,
                rss: proc.rss,
                pss: proc.pss,
                uss: proc.uss,
                rss_ratio: 0.0,
                pss_ratio: 0.0,
                uss_ratio: 0.0,
                read_bytes: proc.read_bytes,
                write_bytes: proc.write_bytes,
            });
        }
    }

    // Sort by highest I/O
    outliers.sort_by(|a, b| {
        let io_a = a.read_bytes.max(a.write_bytes);
        let io_b = b.read_bytes.max(b.write_bytes);
        io_b.cmp(&io_a)
    });

    outliers
}

/// Formats bytes as human-readable string (KB, MB, GB).
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Formats seconds as human-readable uptime (e.g., "47h 32m", "2d 5h").
fn format_uptime(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", total_seconds)
    }
}

/// Renders historical ringbuffer data for a subgroup.
fn render_baseline(out: &mut String, baseline: &BaselineMetrics) {
    writeln!(out, "BASELINE CONTEXT (historical normal)").ok();
    writeln!(out, "=====================================").ok();
    writeln!(out, "  Time window:       {} minutes ({} entries)", baseline.time_window_minutes, baseline.history_count).ok();
    writeln!(out).ok();
    writeln!(out, "  RSS:  min={:<12} avg={:<12} max={}", 
             format_bytes(baseline.min_rss), 
             format_bytes(baseline.avg_rss), 
             format_bytes(baseline.max_rss)).ok();
    writeln!(out, "  PSS:  min={:<12} avg={:<12} max={}", 
             format_bytes(baseline.min_pss), 
             format_bytes(baseline.avg_pss), 
             format_bytes(baseline.max_pss)).ok();
    writeln!(out, "  USS:  min={:<12} avg={:<12} max={}", 
             format_bytes(baseline.min_uss), 
             format_bytes(baseline.avg_uss), 
             format_bytes(baseline.max_uss)).ok();
}

/// Renders live snapshot data for a subgroup with comparison to baseline.
fn render_snapshot(out: &mut String, snapshot: &SubgroupSnapshot, baseline: Option<&BaselineMetrics>) {
    writeln!(out).ok();
    writeln!(out, "LIVE SNAPSHOT (current state)").ok();
    writeln!(out, "=============================").ok();
    writeln!(out, "  Process count:     {}", snapshot.process_count).ok();
    
    if let Some(base) = baseline {
        writeln!(out, "  Total RSS:         {} (baseline avg: {})", 
                 format_bytes(snapshot.total_rss), 
                 format_bytes(base.avg_rss)).ok();
        writeln!(out, "  Total PSS:         {} (baseline avg: {})", 
                 format_bytes(snapshot.total_pss), 
                 format_bytes(base.avg_pss)).ok();
        writeln!(out, "  Total USS:         {} (baseline avg: {})", 
                 format_bytes(snapshot.total_uss), 
                 format_bytes(base.avg_uss)).ok();
    } else {
        writeln!(out, "  Total RSS:         {}", format_bytes(snapshot.total_rss)).ok();
        writeln!(out, "  Total PSS:         {}", format_bytes(snapshot.total_pss)).ok();
        writeln!(out, "  Total USS:         {}", format_bytes(snapshot.total_uss)).ok();
    }
    
    writeln!(out, "  Oldest uptime:     {}", format_uptime(snapshot.oldest_uptime_seconds)).ok();
}

/// Renders memory outliers section.
fn render_memory_outliers(out: &mut String, outliers: &[OutlierProcess], baseline: &BaselineMetrics) {
    writeln!(out).ok();
    writeln!(out, "⚠ MEMORY OUTLIERS DETECTED").ok();
    writeln!(out, "===========================").ok();
    writeln!(out, "Processes with memory usage significantly above baseline:").ok();
    writeln!(out).ok();

    let avg_rss_per_proc = baseline.avg_rss / outliers.len().max(1) as u64;
    let avg_pss_per_proc = baseline.avg_pss / outliers.len().max(1) as u64;
    let avg_uss_per_proc = baseline.avg_uss / outliers.len().max(1) as u64;

    for outlier in outliers.iter().take(10) {  // Show top 10 outliers max
        writeln!(out, "  PID {}  |  {}  |  uptime: {}", 
                 outlier.pid, 
                 outlier.name,
                 format_uptime(outlier.uptime_seconds)).ok();
        
        if outlier.rss_ratio > 2.5 {
            writeln!(out, "    RSS: {}  (baseline avg/proc: {}, ratio: {:.1}x)", 
                     format_bytes(outlier.rss),
                     format_bytes(avg_rss_per_proc),
                     outlier.rss_ratio).ok();
        }
        
        if outlier.pss_ratio > 2.5 {
            writeln!(out, "    PSS: {}  (baseline avg/proc: {}, ratio: {:.1}x)", 
                     format_bytes(outlier.pss),
                     format_bytes(avg_pss_per_proc),
                     outlier.pss_ratio).ok();
        }
        
        if outlier.uss_ratio > 2.5 {
            writeln!(out, "    USS: {}  (baseline avg/proc: {}, ratio: {:.1}x)", 
                     format_bytes(outlier.uss),
                     format_bytes(avg_uss_per_proc),
                     outlier.uss_ratio).ok();
        }
        
        writeln!(out).ok();
    }
}

/// Renders Block I/O outliers section.
fn render_io_outliers(out: &mut String, io_outliers: &[OutlierProcess], snapshot: &SubgroupSnapshot) {
    writeln!(out).ok();
    writeln!(out, "⚠ BLOCK I/O OUTLIERS DETECTED").ok();
    writeln!(out, "==============================").ok();
    writeln!(out, "Processes with I/O significantly above group average:").ok();
    writeln!(out).ok();

    let total_read: u64 = snapshot.all_processes.iter().map(|p| p.read_bytes).sum();
    let total_write: u64 = snapshot.all_processes.iter().map(|p| p.write_bytes).sum();
    let avg_read = total_read / snapshot.process_count.max(1) as u64;
    let avg_write = total_write / snapshot.process_count.max(1) as u64;

    for outlier in io_outliers.iter().take(10) {  // Show top 10 I/O outliers max
        writeln!(out, "  PID {}  |  {}  |  uptime: {}", 
                 outlier.pid, 
                 outlier.name,
                 format_uptime(outlier.uptime_seconds)).ok();
        
        if outlier.read_bytes > 0 {
            let read_ratio = if avg_read > 0 {
                outlier.read_bytes as f64 / avg_read as f64
            } else {
                0.0
            };
            writeln!(out, "    Read:  {}  (group avg: {}, ratio: {:.1}x)", 
                     format_bytes(outlier.read_bytes),
                     format_bytes(avg_read),
                     read_ratio).ok();
        }
        
        if outlier.write_bytes > 0 {
            let write_ratio = if avg_write > 0 {
                outlier.write_bytes as f64 / avg_write as f64
            } else {
                0.0
            };
            writeln!(out, "    Write: {}  (group avg: {}, ratio: {:.1}x)", 
                     format_bytes(outlier.write_bytes),
                     format_bytes(avg_write),
                     write_ratio).ok();
        }
        
        writeln!(out).ok();
    }
    
    writeln!(out, "Note: Block I/O data from /proc/[pid]/io").ok();
}

/// Handler for the /details endpoint.
#[instrument(skip(state))]
pub async fn details_handler(
    State(state): State<SharedState>,
    Query(params): Query<DetailsQuery>,
) -> impl IntoResponse {
    debug!("Processing /details request");

    // Track HTTP request
    state.health_stats.record_http_request();

    let stats = state.ringbuffer_manager.get_stats();

    let mut out = String::new();

    // Ringbuffer configuration section
    writeln!(out, "RINGBUFFER CONFIGURATION").ok();
    writeln!(out, "========================").ok();
    writeln!(out, "max_memory_mb:            {}", stats.max_memory_mb).ok();
    writeln!(out, "entry_size_bytes:         {}", stats.entry_size_bytes).ok();
    writeln!(out, "interval_seconds:         {}", stats.interval_seconds).ok();
    writeln!(
        out,
        "entries_per_subgroup:     {}",
        stats.entries_per_subgroup
    )
    .ok();
    writeln!(out, "total_subgroups:          {}", stats.total_subgroups).ok();
    writeln!(
        out,
        "estimated_ram_bytes:      {}",
        stats.estimated_ram_bytes
    )
    .ok();
    writeln!(
        out,
        "history_seconds:          {} ({} min)",
        stats.history_seconds,
        stats.history_seconds / 60
    )
    .ok();
    writeln!(out).ok();

    // Compute live snapshots for all subgroups
    let snapshots = compute_live_snapshots(&state).await;

    // If subgroup specified, show detailed anomaly detection view
    if let Some(subgroup_name) = params.subgroup {
        writeln!(out, "SUBGROUP: {}", subgroup_name).ok();
        writeln!(out, "=====================").ok();
        writeln!(out).ok();

        // Get historical baseline if available
        let baseline = state
            .ringbuffer_manager
            .get_subgroup_history(&subgroup_name)
            .and_then(|history| calculate_baseline(&history, stats.interval_seconds));

        // Get live snapshot
        let snapshot_opt = snapshots.get(&subgroup_name);

        match (baseline.as_ref(), snapshot_opt) {
            (Some(base), Some(snapshot)) => {
                // Full anomaly detection: baseline + snapshot + outliers
                render_baseline(&mut out, base);
                render_snapshot(&mut out, snapshot, Some(base));

                // Identify memory outliers
                let outliers = identify_outliers(snapshot, base);
                
                // Identify I/O outliers
                let io_outliers = identify_io_outliers(snapshot);

                // Only show sections if anomalies exist
                if !outliers.is_empty() {
                    render_memory_outliers(&mut out, &outliers, base);
                }

                if !io_outliers.is_empty() {
                    render_io_outliers(&mut out, &io_outliers, snapshot);
                }

                // If no anomalies, say so
                if outliers.is_empty() && io_outliers.is_empty() {
                    writeln!(out).ok();
                    writeln!(out, "✓ NOTHING EXCEPTIONAL TO REPORT").ok();
                    writeln!(out, "=================================").ok();
                    writeln!(out, "All processes are operating within normal parameters.").ok();
                    writeln!(out, "No significant deviations from baseline detected.").ok();
                }
            }
            (None, Some(snapshot)) => {
                // No baseline available yet, just show snapshot
                writeln!(out, "No baseline available yet (insufficient history).").ok();
                render_snapshot(&mut out, snapshot, None);
                
                // Check for I/O outliers even without baseline
                let io_outliers = identify_io_outliers(snapshot);
                if !io_outliers.is_empty() {
                    render_io_outliers(&mut out, &io_outliers, snapshot);
                }
            }
            (Some(base), None) => {
                // Have baseline but no live processes
                render_baseline(&mut out, base);
                writeln!(out).ok();
                writeln!(out, "LIVE SNAPSHOT (current state):").ok();
                writeln!(out, "  No processes currently running in this subgroup.").ok();
            }
            (None, None) => {
                // No data at all
                writeln!(out, "No history or live processes found for this subgroup.").ok();
            }
        }
    } else {
        // List all subgroups with summary
        writeln!(out, "AVAILABLE SUBGROUPS").ok();
        writeln!(out, "===================").ok();
        writeln!(out).ok();
        writeln!(out, "This endpoint explains exceptional behavior by comparing live").ok();
        writeln!(out, "processes against historical baselines. Use ?subgroup=<name> to").ok();
        writeln!(out, "view detailed anomaly detection for a specific subgroup.").ok();
        writeln!(out).ok();

        let mut subgroup_names: Vec<String> = snapshots.keys().cloned().collect();
        subgroup_names.sort();

        for subgroup_name in subgroup_names {
            writeln!(out, "SUBGROUP: {}", subgroup_name).ok();
            writeln!(out, "---------------------").ok();

            if let Some(snapshot) = snapshots.get(&subgroup_name) {
                writeln!(out, "  Process count:    {}", snapshot.process_count).ok();
                writeln!(out, "  Total RSS:        {}", format_bytes(snapshot.total_rss)).ok();
                writeln!(out, "  Total PSS:        {}", format_bytes(snapshot.total_pss)).ok();
                writeln!(out, "  Total USS:        {}", format_bytes(snapshot.total_uss)).ok();
                writeln!(out, "  Oldest uptime:    {}", format_uptime(snapshot.oldest_uptime_seconds)).ok();
            }

            writeln!(out).ok();
            writeln!(out, "  Use ?subgroup={} to view anomaly detection details", subgroup_name).ok();
            writeln!(out).ok();
        }
    }

    writeln!(out).ok();
    writeln!(out, "{}", FOOTER_TEXT).ok();

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        out,
    )
}
