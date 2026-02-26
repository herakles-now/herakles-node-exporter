//! Details endpoint handler - Time-based forensic view.
//!
//! This module provides the `/details` endpoint handler with intelligent uptime-aware
//! filtering and three temporal zones for forensic analysis.
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
use tracing::{debug, instrument};

use crate::cache::ProcMem;
use crate::handlers::health::FOOTER_TEXT;
use crate::process::classifier::classify_process_raw;
use crate::ringbuffer::RingbufferEntry;
use crate::state::SharedState;

// Temporal zone thresholds
const LIVE_PHASE_SECONDS: f64 = 300.0; // 5 minutes
const STABILIZATION_PHASE_SECONDS: f64 = 3600.0; // 60 minutes

// Anomaly severity thresholds
const SEVERITY_MINOR: f64 = 1.2; // ℹ️  Minor deviation
const SEVERITY_MODERATE: f64 = 1.5; // ⚠️  Moderate deviation
const SEVERITY_CRITICAL: f64 = 2.0; // 🔥 Critical deviation

const MAX_OUTLIERS_DISPLAY: usize = 10;
const MAX_DISPLAYED_SUBGROUPS: usize = 20;

/// Query parameters for the details endpoint.
#[derive(Deserialize, Debug)]
pub struct DetailsQuery {
    pub subgroup: Option<String>,
}

/// Temporal phase classification for a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemporalPhase {
    Newborn,       // uptime < history_window - Don't compare to baseline
    Live,          // 0-5 minutes
    Stabilization, // 5-60 minutes
    Historical,    // >60 minutes
}

/// Information about a single process with temporal context.
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
    phase: TemporalPhase,
}

/// Metric value with timestamp for peak tracking.
#[derive(Debug, Clone)]
struct MetricWithTimestamp {
    value: u64,
    timestamp: i64,
}

/// Min/Max/Avg triplet with timestamps for stabilization phase.
#[derive(Debug, Clone)]
struct MetricTriplet {
    min: MetricWithTimestamp,
    max: MetricWithTimestamp,
    avg: u64,
}

/// Process anomaly with severity and details.
#[derive(Debug, Clone)]
struct ProcessAnomaly {
    pid: u32,
    name: String,
    uptime_seconds: f64,
    phase: TemporalPhase,

    // Current values
    current_rss: u64,
    current_pss: u64,
    current_uss: u64,
    current_cpu: f32,

    // Comparison baseline (5-min rolling avg for Live, longterm for Historical)
    baseline_rss: u64,
    baseline_pss: u64,
    baseline_uss: u64,

    // Deviation ratios
    rss_ratio: f64,
    pss_ratio: f64,
    uss_ratio: f64,

    // Growth rates (MB/sec over last hour)
    rss_growth_rate: Option<f64>,

    // I/O metrics
    read_bytes: u64,
    write_bytes: u64,
    #[allow(dead_code)] // Future enhancement for 5-minute delta tracking
    io_delta_5min: Option<(u64, u64)>, // (read_delta, write_delta)

    // Severity
    severity: AnomalySeverity,
}

/// Severity levels for anomalies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AnomalySeverity {
    Normal,
    Minor,    // 1.2x
    Moderate, // 1.5x
    Critical, // 2.0x
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

/// Historical I/O event (past spike now idle).
#[allow(dead_code)] // Future enhancement for historical I/O event tracking
#[derive(Debug, Clone)]
struct HistoricalIoEvent {
    pid: u32,
    name: String,
    peak_read_bytes: u64,
    peak_write_bytes: u64,
    last_active_timestamp: i64,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Calculate process uptime from system uptime and process start time.
fn calculate_process_uptime(system_uptime: f64, process_start_time: f64) -> f64 {
    system_uptime - process_start_time
}

/// Determine temporal phase based on uptime and history window.
fn classify_temporal_phase(uptime_seconds: f64, history_window_seconds: u64) -> TemporalPhase {
    if uptime_seconds < history_window_seconds as f64 {
        TemporalPhase::Newborn
    } else if uptime_seconds < LIVE_PHASE_SECONDS {
        TemporalPhase::Live
    } else if uptime_seconds < STABILIZATION_PHASE_SECONDS {
        TemporalPhase::Stabilization
    } else {
        TemporalPhase::Historical
    }
}

/// Calculate 5-minute rolling average for a metric from ringbuffer history.
/// Returns None if insufficient data.
fn get_5min_rolling_avg(
    history: &[RingbufferEntry],
    interval_seconds: u64,
    extract_value: impl Fn(&RingbufferEntry) -> u64,
) -> Option<u64> {
    if history.is_empty() {
        return None;
    }

    // Calculate how many entries cover 5 minutes
    let entries_in_5min = (300 / interval_seconds).max(1) as usize;

    // Take the most recent entries up to 5 minutes
    let entries_to_use = history.len().min(entries_in_5min);
    let recent_entries = &history[history.len() - entries_to_use..];

    let sum: u64 = recent_entries.iter().map(&extract_value).sum();
    Some(sum / entries_to_use as u64)
}

/// Extract min/max/avg with timestamps for a metric (stabilization phase).
fn extract_min_max_avg_with_timestamps(
    history: &[RingbufferEntry],
    extract_value: impl Fn(&RingbufferEntry) -> u64,
) -> Option<MetricTriplet> {
    if history.is_empty() {
        return None;
    }

    let mut min_entry = &history[0];
    let mut max_entry = &history[0];
    let mut sum: u64 = 0;

    for entry in history {
        let value = extract_value(entry);
        sum += value;

        if extract_value(entry) < extract_value(min_entry) {
            min_entry = entry;
        }
        if extract_value(entry) > extract_value(max_entry) {
            max_entry = entry;
        }
    }

    let avg = sum / history.len() as u64;

    Some(MetricTriplet {
        min: MetricWithTimestamp {
            value: extract_value(min_entry),
            timestamp: min_entry.timestamp,
        },
        max: MetricWithTimestamp {
            value: extract_value(max_entry),
            timestamp: max_entry.timestamp,
        },
        avg,
    })
}

/// Calculate I/O delta over the last 5 minutes.
/// Returns (read_delta, write_delta) or None if insufficient history.
///
/// TODO: This function is currently a stub because RingbufferEntry doesn't store I/O data.
/// To implement this properly, we would need to extend RingbufferEntry to track I/O metrics
/// or maintain a separate I/O history tracking structure.
#[allow(dead_code)] // Future enhancement for 5-minute I/O delta calculation
fn calculate_io_delta_5min(
    _current_read: u64,
    _current_write: u64,
    _history: &[RingbufferEntry],
    _interval_seconds: u64,
) -> Option<(u64, u64)> {
    // Note: RingbufferEntry doesn't have I/O data, so we can't calculate delta from current structure
    // This would require adding I/O tracking to the ringbuffer entries
    None
}

/// Calculate long-term average (for Historical phase).
fn calculate_longterm_avg(
    history: &[RingbufferEntry],
    extract_value: impl Fn(&RingbufferEntry) -> u64,
) -> Option<u64> {
    if history.is_empty() {
        return None;
    }

    let sum: u64 = history.iter().map(&extract_value).sum();
    Some(sum / history.len() as u64)
}

/// Calculate growth rate (MB/sec) over the last hour.
fn calculate_growth_rate(
    current_value: u64,
    history: &[RingbufferEntry],
    interval_seconds: u64,
    extract_value: impl Fn(&RingbufferEntry) -> u64,
) -> Option<f64> {
    if history.is_empty() {
        return None;
    }

    // Calculate how many entries cover 1 hour
    let entries_in_hour = (3600 / interval_seconds).max(1) as usize;

    if history.len() < entries_in_hour {
        return None; // Not enough history
    }

    // Get value from 1 hour ago
    let index_1h_ago = history.len() - entries_in_hour;
    let value_1h_ago = extract_value(&history[index_1h_ago]);

    // Calculate growth rate in bytes per second
    let delta_bytes = current_value.saturating_sub(value_1h_ago) as f64;
    let delta_seconds = 3600.0;

    Some(delta_bytes / delta_seconds)
}

/// Detect anomaly severity based on deviation ratio.
fn detect_anomaly_severity(deviation_ratio: f64) -> AnomalySeverity {
    if deviation_ratio >= SEVERITY_CRITICAL {
        AnomalySeverity::Critical
    } else if deviation_ratio >= SEVERITY_MODERATE {
        AnomalySeverity::Moderate
    } else if deviation_ratio >= SEVERITY_MINOR {
        AnomalySeverity::Minor
    } else {
        AnomalySeverity::Normal
    }
}

/// Format severity with emoji.
fn format_severity(severity: AnomalySeverity) -> &'static str {
    match severity {
        AnomalySeverity::Critical => "🔥 Critical",
        AnomalySeverity::Moderate => "⚠️  Moderate",
        AnomalySeverity::Minor => "ℹ️  Minor",
        AnomalySeverity::Normal => "Normal",
    }
}

// ============================================================================
// SNAPSHOT AND ANALYSIS FUNCTIONS
// ============================================================================

/// Computes live snapshot for all subgroups from the current cache.
async fn compute_live_snapshots(
    state: &SharedState,
    history_window_seconds: u64,
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

        // Convert all processes to ProcessInfo with temporal phase
        let all_processes: Vec<ProcessInfo> = procs
            .iter()
            .map(|p| {
                let uptime = calculate_process_uptime(system_uptime, p.start_time_seconds);
                let phase = classify_temporal_phase(uptime, history_window_seconds);

                ProcessInfo {
                    pid: p.pid,
                    name: p.name.clone(),
                    rss: p.rss,
                    pss: p.pss,
                    uss: p.uss,
                    cpu_percent: p.cpu_percent,
                    uptime_seconds: uptime,
                    read_bytes: p.read_bytes,
                    write_bytes: p.write_bytes,
                    phase,
                }
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

/// Analyze processes and identify anomalies by temporal phase.
fn analyze_anomalies(
    snapshot: &SubgroupSnapshot,
    history: &[RingbufferEntry],
    interval_seconds: u64,
) -> Vec<ProcessAnomaly> {
    let mut anomalies = Vec::new();

    for proc in &snapshot.all_processes {
        // Skip newborn processes - they don't have enough history
        if proc.phase == TemporalPhase::Newborn {
            continue;
        }

        let anomaly = match proc.phase {
            TemporalPhase::Live => analyze_live_phase(proc, history, interval_seconds),
            TemporalPhase::Stabilization => {
                analyze_stabilization_phase(proc, history, interval_seconds)
            }
            TemporalPhase::Historical => analyze_historical_phase(proc, history, interval_seconds),
            TemporalPhase::Newborn => continue, // Already checked above
        };

        if let Some(a) = anomaly {
            if a.severity > AnomalySeverity::Normal {
                anomalies.push(a);
            }
        }
    }

    // Sort by severity (highest first)
    anomalies.sort_by(|a, b| b.severity.cmp(&a.severity));

    anomalies
}

/// Analyze a process in Live phase (0-5 minutes).
/// Compare against 5-minute rolling average.
fn analyze_live_phase(
    proc: &ProcessInfo,
    history: &[RingbufferEntry],
    interval_seconds: u64,
) -> Option<ProcessAnomaly> {
    // Get 5-minute rolling averages
    let baseline_rss = get_5min_rolling_avg(history, interval_seconds, |e| e.rss_kb * 1024)?;
    let baseline_pss = get_5min_rolling_avg(history, interval_seconds, |e| e.pss_kb * 1024)?;
    let baseline_uss = get_5min_rolling_avg(history, interval_seconds, |e| e.uss_kb * 1024)?;

    // Calculate ratios
    let rss_ratio = if baseline_rss > 0 {
        proc.rss as f64 / baseline_rss as f64
    } else {
        0.0
    };
    let pss_ratio = if baseline_pss > 0 {
        proc.pss as f64 / baseline_pss as f64
    } else {
        0.0
    };
    let uss_ratio = if baseline_uss > 0 {
        proc.uss as f64 / baseline_uss as f64
    } else {
        0.0
    };

    let max_ratio = rss_ratio.max(pss_ratio).max(uss_ratio);
    let severity = detect_anomaly_severity(max_ratio);

    Some(ProcessAnomaly {
        pid: proc.pid,
        name: proc.name.clone(),
        uptime_seconds: proc.uptime_seconds,
        phase: proc.phase,
        current_rss: proc.rss,
        current_pss: proc.pss,
        current_uss: proc.uss,
        current_cpu: proc.cpu_percent,
        baseline_rss,
        baseline_pss,
        baseline_uss,
        rss_ratio,
        pss_ratio,
        uss_ratio,
        rss_growth_rate: None, // Not applicable for Live phase
        read_bytes: proc.read_bytes,
        write_bytes: proc.write_bytes,
        io_delta_5min: None, // TODO: Calculate when I/O history is available
        severity,
    })
}

/// Analyze a process in Stabilization phase (5-60 minutes).
/// Look for pattern deviations.
fn analyze_stabilization_phase(
    proc: &ProcessInfo,
    history: &[RingbufferEntry],
    _interval_seconds: u64,
) -> Option<ProcessAnomaly> {
    // Get long-term averages for comparison
    let baseline_rss = calculate_longterm_avg(history, |e| e.rss_kb * 1024)?;
    let baseline_pss = calculate_longterm_avg(history, |e| e.pss_kb * 1024)?;
    let baseline_uss = calculate_longterm_avg(history, |e| e.uss_kb * 1024)?;

    // Calculate ratios
    let rss_ratio = if baseline_rss > 0 {
        proc.rss as f64 / baseline_rss as f64
    } else {
        0.0
    };
    let pss_ratio = if baseline_pss > 0 {
        proc.pss as f64 / baseline_pss as f64
    } else {
        0.0
    };
    let uss_ratio = if baseline_uss > 0 {
        proc.uss as f64 / baseline_uss as f64
    } else {
        0.0
    };

    let max_ratio = rss_ratio.max(pss_ratio).max(uss_ratio);
    let severity = detect_anomaly_severity(max_ratio);

    Some(ProcessAnomaly {
        pid: proc.pid,
        name: proc.name.clone(),
        uptime_seconds: proc.uptime_seconds,
        phase: proc.phase,
        current_rss: proc.rss,
        current_pss: proc.pss,
        current_uss: proc.uss,
        current_cpu: proc.cpu_percent,
        baseline_rss,
        baseline_pss,
        baseline_uss,
        rss_ratio,
        pss_ratio,
        uss_ratio,
        rss_growth_rate: None, // Calculate growth rate for this phase
        read_bytes: proc.read_bytes,
        write_bytes: proc.write_bytes,
        io_delta_5min: None,
        severity,
    })
}

/// Analyze a process in Historical phase (>60 minutes).
/// Compare against long-term trend.
fn analyze_historical_phase(
    proc: &ProcessInfo,
    history: &[RingbufferEntry],
    interval_seconds: u64,
) -> Option<ProcessAnomaly> {
    // Get long-term averages
    let baseline_rss = calculate_longterm_avg(history, |e| e.rss_kb * 1024)?;
    let baseline_pss = calculate_longterm_avg(history, |e| e.pss_kb * 1024)?;
    let baseline_uss = calculate_longterm_avg(history, |e| e.uss_kb * 1024)?;

    // Calculate ratios
    let rss_ratio = if baseline_rss > 0 {
        proc.rss as f64 / baseline_rss as f64
    } else {
        0.0
    };
    let pss_ratio = if baseline_pss > 0 {
        proc.pss as f64 / baseline_pss as f64
    } else {
        0.0
    };
    let uss_ratio = if baseline_uss > 0 {
        proc.uss as f64 / baseline_uss as f64
    } else {
        0.0
    };

    let max_ratio = rss_ratio.max(pss_ratio).max(uss_ratio);
    let severity = detect_anomaly_severity(max_ratio);

    // Calculate growth rate (important for detecting memory leaks)
    let rss_growth_rate =
        calculate_growth_rate(proc.rss, history, interval_seconds, |e| e.rss_kb * 1024);

    Some(ProcessAnomaly {
        pid: proc.pid,
        name: proc.name.clone(),
        uptime_seconds: proc.uptime_seconds,
        phase: proc.phase,
        current_rss: proc.rss,
        current_pss: proc.pss,
        current_uss: proc.uss,
        current_cpu: proc.cpu_percent,
        baseline_rss,
        baseline_pss,
        baseline_uss,
        rss_ratio,
        pss_ratio,
        uss_ratio,
        rss_growth_rate,
        read_bytes: proc.read_bytes,
        write_bytes: proc.write_bytes,
        io_delta_5min: None,
        severity,
    })
}

// ============================================================================
// FORMATTING AND OUTPUT FUNCTIONS
// ============================================================================

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

/// Formats seconds as human-readable uptime (e.g., "4m 32s", "47h 32m", "2d 5h").
fn format_uptime(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", total_seconds)
    }
}

/// Format timestamp as HH:MM:SS.
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{Local, TimeZone};
    let dt = Local.timestamp_opt(timestamp, 0).single();
    match dt {
        Some(dt) => dt.format("%H:%M:%S").to_string(),
        None => format!("@{}", timestamp),
    }
}

/// Format growth rate in MB/min or MB/sec.
fn format_growth_rate(bytes_per_sec: f64) -> String {
    let mb_per_sec = bytes_per_sec / (1024.0 * 1024.0);
    let mb_per_min = mb_per_sec * 60.0;

    if mb_per_min.abs() >= 1.0 {
        format!("{:+.1} MB/min", mb_per_min)
    } else if mb_per_sec.abs() >= 0.01 {
        format!("{:+.2} MB/sec", mb_per_sec)
    } else {
        format!("{:+.3} MB/sec", mb_per_sec)
    }
}

/// Render newborn processes (those with uptime < history_window).
fn render_newborn_processes(out: &mut String, snapshot: &SubgroupSnapshot) {
    let newborns: Vec<_> = snapshot
        .all_processes
        .iter()
        .filter(|p| p.phase == TemporalPhase::Newborn)
        .collect();

    if newborns.is_empty() {
        return;
    }

    writeln!(out).ok();
    writeln!(out, "🆕 NEWBORN PROCESSES").ok();
    writeln!(out, "====================").ok();
    writeln!(
        out,
        "These processes are too young to have reliable baseline comparison."
    )
    .ok();
    writeln!(out).ok();

    for proc in newborns.iter().take(MAX_OUTLIERS_DISPLAY) {
        writeln!(
            out,
            "  PID {}  |  {}  |  uptime: {}",
            proc.pid,
            proc.name,
            format_uptime(proc.uptime_seconds)
        )
        .ok();
        writeln!(
            out,
            "    RSS: {}  PSS: {}  USS: {}",
            format_bytes(proc.rss),
            format_bytes(proc.pss),
            format_bytes(proc.uss)
        )
        .ok();
        writeln!(out).ok();
    }
}

/// Render Live Phase (0-5 minutes) anomalies.
fn render_live_phase(
    out: &mut String,
    anomalies: &[ProcessAnomaly],
    history: &[RingbufferEntry],
    interval_seconds: u64,
) {
    let live_anomalies: Vec<_> = anomalies
        .iter()
        .filter(|a| a.phase == TemporalPhase::Live)
        .collect();

    if live_anomalies.is_empty() {
        return;
    }

    writeln!(out).ok();
    writeln!(out, "🔴 LIVE PHASE (0-5 minutes)").ok();
    writeln!(out, "---------------------------").ok();
    writeln!(
        out,
        "{} process(es) with current anomalies detected.",
        live_anomalies.len()
    )
    .ok();
    writeln!(out).ok();

    for anomaly in live_anomalies.iter().take(MAX_OUTLIERS_DISPLAY) {
        writeln!(
            out,
            "Process: {} (PID {}) | Uptime: {}",
            anomaly.name,
            anomaly.pid,
            format_uptime(anomaly.uptime_seconds)
        )
        .ok();
        writeln!(out).ok();

        // RSS comparison
        if anomaly.rss_ratio >= SEVERITY_MINOR {
            writeln!(
                out,
                "  Current RSS:    {}",
                format_bytes(anomaly.current_rss)
            )
            .ok();
            writeln!(
                out,
                "  5min avg RSS:   {}  (↑ {:.1}x)  {}",
                format_bytes(anomaly.baseline_rss),
                anomaly.rss_ratio,
                format_severity(detect_anomaly_severity(anomaly.rss_ratio))
            )
            .ok();

            // Calculate growth rate
            if let Some(rate) =
                calculate_growth_rate(anomaly.current_rss, history, interval_seconds, |e| {
                    e.rss_kb * 1024
                })
            {
                if rate > 0.0 {
                    writeln!(out, "  Growth rate:    {}", format_growth_rate(rate)).ok();
                }
            }
            writeln!(out).ok();
        }

        // PSS comparison
        if anomaly.pss_ratio >= SEVERITY_MINOR {
            writeln!(
                out,
                "  Current PSS:    {}",
                format_bytes(anomaly.current_pss)
            )
            .ok();
            writeln!(
                out,
                "  5min avg PSS:   {}  (↑ {:.1}x)  {}",
                format_bytes(anomaly.baseline_pss),
                anomaly.pss_ratio,
                format_severity(detect_anomaly_severity(anomaly.pss_ratio))
            )
            .ok();
            writeln!(out).ok();
        }

        // USS comparison
        if anomaly.uss_ratio >= SEVERITY_MINOR {
            writeln!(
                out,
                "  Current USS:    {}",
                format_bytes(anomaly.current_uss)
            )
            .ok();
            writeln!(
                out,
                "  5min avg USS:   {}  (↑ {:.1}x)  {}",
                format_bytes(anomaly.baseline_uss),
                anomaly.uss_ratio,
                format_severity(detect_anomaly_severity(anomaly.uss_ratio))
            )
            .ok();
            writeln!(out).ok();
        }

        // I/O if significant
        if anomaly.read_bytes > 0 || anomaly.write_bytes > 0 {
            writeln!(
                out,
                "  Block I/O read:  {}",
                format_bytes(anomaly.read_bytes)
            )
            .ok();
            writeln!(
                out,
                "  Block I/O write: {}",
                format_bytes(anomaly.write_bytes)
            )
            .ok();
            writeln!(out).ok();
        }
    }
}

/// Render Stabilization Phase (5-60 minutes) anomalies.
fn render_stabilization_phase(
    out: &mut String,
    anomalies: &[ProcessAnomaly],
    history: &[RingbufferEntry],
) {
    let stab_anomalies: Vec<_> = anomalies
        .iter()
        .filter(|a| a.phase == TemporalPhase::Stabilization)
        .collect();

    if stab_anomalies.is_empty() {
        return;
    }

    writeln!(out).ok();
    writeln!(out, "🟡 STABILIZATION PHASE (5-60 minutes)").ok();
    writeln!(out, "--------------------------------------").ok();
    writeln!(
        out,
        "{} process(es) showing pattern deviation.",
        stab_anomalies.len()
    )
    .ok();
    writeln!(out).ok();

    for anomaly in stab_anomalies.iter().take(MAX_OUTLIERS_DISPLAY) {
        writeln!(
            out,
            "Process: {} (PID {}) | Uptime: {}",
            anomaly.name,
            anomaly.pid,
            format_uptime(anomaly.uptime_seconds)
        )
        .ok();
        writeln!(out).ok();

        // Show triplets for RSS
        if let Some(triplet) = extract_min_max_avg_with_timestamps(history, |e| e.rss_kb * 1024) {
            writeln!(out, "  RSS:").ok();
            writeln!(
                out,
                "    Min:   {}  (@ {})",
                format_bytes(triplet.min.value),
                format_timestamp(triplet.min.timestamp)
            )
            .ok();
            writeln!(
                out,
                "    Max:   {}  (@ {})  ← Peak moment",
                format_bytes(triplet.max.value),
                format_timestamp(triplet.max.timestamp)
            )
            .ok();
            writeln!(out, "    Avg:   {}", format_bytes(triplet.avg)).ok();

            if anomaly.rss_ratio >= SEVERITY_MINOR {
                writeln!(
                    out,
                    "    Current vs Avg: {:.1}x  {}",
                    anomaly.rss_ratio,
                    format_severity(anomaly.severity)
                )
                .ok();
            }
            writeln!(out).ok();
        }

        // Show CPU if available (current value only, no historical triplet)
        if anomaly.current_cpu > 0.0 {
            writeln!(out, "  CPU:").ok();
            writeln!(out, "    Current: {:.1}%", anomaly.current_cpu).ok();
            writeln!(out).ok();
        }
    }
}

/// Render Historical Phase (>60 minutes) anomalies.
fn render_historical_phase(out: &mut String, anomalies: &[ProcessAnomaly]) {
    let hist_anomalies: Vec<_> = anomalies
        .iter()
        .filter(|a| a.phase == TemporalPhase::Historical)
        .collect();

    if hist_anomalies.is_empty() {
        return;
    }

    writeln!(out).ok();
    writeln!(out, "🟢 HISTORICAL PHASE (> 60 minutes)").ok();
    writeln!(out, "-----------------------------------").ok();
    writeln!(
        out,
        "{} process(es) with long-term trends.",
        hist_anomalies.len()
    )
    .ok();
    writeln!(out).ok();

    for anomaly in hist_anomalies.iter().take(MAX_OUTLIERS_DISPLAY) {
        writeln!(
            out,
            "Process: {} (PID {}) | Uptime: {}",
            anomaly.name,
            anomaly.pid,
            format_uptime(anomaly.uptime_seconds)
        )
        .ok();
        writeln!(out).ok();

        writeln!(
            out,
            "  Long-term avg RSS:  {}  (history)",
            format_bytes(anomaly.baseline_rss)
        )
        .ok();
        writeln!(
            out,
            "  Current RSS:        {}  (↑ {:.2}x longterm avg)  {}",
            format_bytes(anomaly.current_rss),
            anomaly.rss_ratio,
            format_severity(anomaly.severity)
        )
        .ok();
        writeln!(out).ok();

        // Show growth rate if significant
        if let Some(rate) = anomaly.rss_growth_rate {
            if rate > 1024.0 {
                // More than 1 KB/sec growth
                writeln!(out, "  Trend analysis:").ok();
                writeln!(
                    out,
                    "    Growth rate: {}  ← Steady growth pattern",
                    format_growth_rate(rate)
                )
                .ok();

                if rate > 10240.0 {
                    // More than 10 KB/sec
                    writeln!(out, "    ⚠️  Possible memory leak candidate").ok();
                }
                writeln!(out).ok();
            }
        }
    }
}
/// Handler for the /details endpoint.
#[instrument(skip(_state))]
pub async fn details_handler(
    State(_state): State<SharedState>,
    Query(params): Query<DetailsQuery>,
) -> impl IntoResponse {
    debug!("Processing /details request");

    // Track HTTP request
    _state.health_stats.record_http_request();

    let stats = _state.ringbuffer_manager.get_stats();

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

    // Ringbuffer memory usage statistics
    writeln!(out, "RINGBUFFER MEMORY USAGE").ok();
    writeln!(out, "=======================").ok();

    // Get all subgroup names and their ringbuffer stats
    let subgroups = _state.ringbuffer_manager.get_all_subgroups();
    if !subgroups.is_empty() {
        writeln!(
            out,
            "{:<30} | {:>10} | {:>10} | {:>10}",
            "Subgroup", "Entries", "Capacity", "Fill %"
        )
        .ok();
        writeln!(out, "{}", "-".repeat(66)).ok();

        for subgroup_name in subgroups.iter().take(MAX_DISPLAYED_SUBGROUPS) {
            if let Some(buffer) = _state.ringbuffer_manager.get_subgroup_buffer(subgroup_name) {
                let len = buffer.len();
                let capacity = buffer.capacity();
                let fill_percent = if capacity > 0 {
                    (len as f64 / capacity as f64) * 100.0
                } else {
                    0.0
                };

                writeln!(
                    out,
                    "{:<30} | {:>10} | {:>10} | {:>9.1}%",
                    subgroup_name, len, capacity, fill_percent
                )
                .ok();
            }
        }

        if subgroups.len() > MAX_DISPLAYED_SUBGROUPS {
            writeln!(out, "... and {} more subgroups", subgroups.len() - MAX_DISPLAYED_SUBGROUPS).ok();
        }
    } else {
        writeln!(out, "No subgroups tracked yet").ok();
    }

    writeln!(out).ok();

    // Compute live snapshots for all subgroups
    let snapshots = compute_live_snapshots(&_state, stats.history_seconds).await;

    // If subgroup specified, show detailed temporal zone view
    if let Some(subgroup_name) = params.subgroup {
        writeln!(out, "SUBGROUP: {}", subgroup_name).ok();
        writeln!(out, "=====================").ok();
        writeln!(out).ok();

        // Get live snapshot
        let snapshot_opt = snapshots.get(&subgroup_name);

        // Get historical data if available
        let history_opt = _state
            .ringbuffer_manager
            .get_subgroup_history(&subgroup_name);

        match (history_opt.as_ref(), snapshot_opt) {
            (Some(history), Some(snapshot)) if !history.is_empty() => {
                // Full temporal zone analysis

                // Analyze anomalies by phase
                let anomalies = analyze_anomalies(snapshot, history, stats.interval_seconds);

                // Show newborn processes first (informational)
                render_newborn_processes(&mut out, snapshot);

                if anomalies.is_empty()
                    && snapshot
                        .all_processes
                        .iter()
                        .all(|p| p.phase == TemporalPhase::Newborn)
                {
                    // Only newborn processes exist
                    writeln!(out).ok();
                    writeln!(out, "✅ No mature processes yet").ok();
                    writeln!(out, "==========================").ok();
                    writeln!(out, "All processes are too new for baseline comparison.").ok();
                    writeln!(
                        out,
                        "Check back after {} minutes for temporal analysis.",
                        stats.history_seconds / 60
                    )
                    .ok();
                } else if anomalies.is_empty() {
                    // System is normal
                    writeln!(out).ok();
                    writeln!(out, "✅ No exceptional behavior detected").ok();
                    writeln!(out, "====================================").ok();
                    writeln!(
                        out,
                        "All processes within normal ranges for their age groups."
                    )
                    .ok();
                } else {
                    // Show anomalies by temporal zone
                    render_live_phase(&mut out, &anomalies, history, stats.interval_seconds);
                    render_stabilization_phase(&mut out, &anomalies, history);
                    render_historical_phase(&mut out, &anomalies);
                }
            }
            (_, Some(snapshot)) if snapshot.all_processes.is_empty() => {
                // No processes
                writeln!(out, "No processes currently running in this subgroup.").ok();
            }
            (Some(history), Some(snapshot)) if history.is_empty() => {
                // Empty history, treat as no history
                writeln!(out, "No baseline available yet (insufficient history).").ok();
                writeln!(
                    out,
                    "Collecting data... check back in {} minutes.",
                    stats.history_seconds / 60
                )
                .ok();
                writeln!(out).ok();

                writeln!(out, "Current state:").ok();
                writeln!(out, "  Process count:  {}", snapshot.process_count).ok();
                writeln!(
                    out,
                    "  Total RSS:      {}",
                    format_bytes(snapshot.total_rss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total PSS:      {}",
                    format_bytes(snapshot.total_pss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total USS:      {}",
                    format_bytes(snapshot.total_uss)
                )
                .ok();
            }
            (None, Some(snapshot)) => {
                // No history yet, show live snapshot only
                writeln!(out, "No baseline available yet (insufficient history).").ok();
                writeln!(
                    out,
                    "Collecting data... check back in {} minutes.",
                    stats.history_seconds / 60
                )
                .ok();
                writeln!(out).ok();

                writeln!(out, "Current state:").ok();
                writeln!(out, "  Process count:  {}", snapshot.process_count).ok();
                writeln!(
                    out,
                    "  Total RSS:      {}",
                    format_bytes(snapshot.total_rss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total PSS:      {}",
                    format_bytes(snapshot.total_pss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total USS:      {}",
                    format_bytes(snapshot.total_uss)
                )
                .ok();
            }
            (Some(_), None) | (None, None) => {
                // No live processes
                writeln!(out, "No processes currently running in this subgroup.").ok();
            }
            // Catch-all for any other combinations
            (Some(_), Some(snapshot)) => {
                // Fallback case
                writeln!(out, "Current state:").ok();
                writeln!(out, "  Process count:  {}", snapshot.process_count).ok();
                writeln!(
                    out,
                    "  Total RSS:      {}",
                    format_bytes(snapshot.total_rss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total PSS:      {}",
                    format_bytes(snapshot.total_pss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total USS:      {}",
                    format_bytes(snapshot.total_uss)
                )
                .ok();
            }
        }
    } else {
        // List all subgroups with summary
        writeln!(out, "AVAILABLE SUBGROUPS").ok();
        writeln!(out, "===================").ok();
        writeln!(out).ok();
        writeln!(
            out,
            "This endpoint provides time-based forensic analysis with"
        )
        .ok();
        writeln!(
            out,
            "intelligent uptime-aware filtering and three temporal zones:"
        )
        .ok();
        writeln!(out, "  🔴 Live Phase (0-5 min)").ok();
        writeln!(out, "  🟡 Stabilization Phase (5-60 min)").ok();
        writeln!(out, "  🟢 Historical Phase (>60 min)").ok();
        writeln!(out).ok();
        writeln!(out, "Use ?subgroup=<name> to view detailed analysis.").ok();
        writeln!(out).ok();

        let mut subgroup_names: Vec<String> = snapshots.keys().cloned().collect();
        subgroup_names.sort();

        for subgroup_name in subgroup_names {
            writeln!(out, "SUBGROUP: {}", subgroup_name).ok();
            writeln!(out, "---------------------").ok();

            if let Some(snapshot) = snapshots.get(&subgroup_name) {
                writeln!(out, "  Process count:    {}", snapshot.process_count).ok();
                writeln!(
                    out,
                    "  Total RSS:        {}",
                    format_bytes(snapshot.total_rss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total PSS:        {}",
                    format_bytes(snapshot.total_pss)
                )
                .ok();
                writeln!(
                    out,
                    "  Total USS:        {}",
                    format_bytes(snapshot.total_uss)
                )
                .ok();
                writeln!(
                    out,
                    "  Oldest uptime:    {}",
                    format_uptime(snapshot.oldest_uptime_seconds)
                )
                .ok();

                // Show phase distribution
                let newborn_count = snapshot
                    .all_processes
                    .iter()
                    .filter(|p| p.phase == TemporalPhase::Newborn)
                    .count();
                let live_count = snapshot
                    .all_processes
                    .iter()
                    .filter(|p| p.phase == TemporalPhase::Live)
                    .count();
                let stab_count = snapshot
                    .all_processes
                    .iter()
                    .filter(|p| p.phase == TemporalPhase::Stabilization)
                    .count();
                let hist_count = snapshot
                    .all_processes
                    .iter()
                    .filter(|p| p.phase == TemporalPhase::Historical)
                    .count();

                if newborn_count > 0 || live_count > 0 || stab_count > 0 || hist_count > 0 {
                    writeln!(out, "  Phase distribution:").ok();
                    if newborn_count > 0 {
                        writeln!(out, "    🆕 Newborn: {}", newborn_count).ok();
                    }
                    if live_count > 0 {
                        writeln!(out, "    🔴 Live: {}", live_count).ok();
                    }
                    if stab_count > 0 {
                        writeln!(out, "    🟡 Stabilization: {}", stab_count).ok();
                    }
                    if hist_count > 0 {
                        writeln!(out, "    🟢 Historical: {}", hist_count).ok();
                    }
                }
            }

            writeln!(out).ok();
            writeln!(
                out,
                "  Use ?subgroup={} to view temporal analysis",
                subgroup_name
            )
            .ok();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ringbuffer::{RingbufferEntry, TopProcessInfo};

    #[test]
    fn test_calculate_process_uptime() {
        let system_uptime = 1000.0;
        let process_start = 900.0;
        let uptime = calculate_process_uptime(system_uptime, process_start);
        assert_eq!(uptime, 100.0);
    }

    #[test]
    fn test_classify_temporal_phase() {
        let history_window = 3600; // 1 hour

        // Newborn: uptime < history_window
        assert_eq!(
            classify_temporal_phase(1800.0, history_window),
            TemporalPhase::Newborn
        );
        assert_eq!(
            classify_temporal_phase(3500.0, history_window),
            TemporalPhase::Newborn
        );

        // Live: >= history_window and < 5 minutes (300 seconds)
        // This case is tricky: if history_window > 300, there's no Live phase
        // Let's use a smaller history_window for this test
        let small_history = 60; // 1 minute
        assert_eq!(
            classify_temporal_phase(200.0, small_history),
            TemporalPhase::Live
        );

        // Stabilization: 5-60 minutes (300 - 3600 seconds)
        assert_eq!(
            classify_temporal_phase(1800.0, small_history),
            TemporalPhase::Stabilization
        );

        // Historical: >60 minutes (>3600 seconds)
        assert_eq!(
            classify_temporal_phase(7200.0, small_history),
            TemporalPhase::Historical
        );
        assert_eq!(
            classify_temporal_phase(10000.0, small_history),
            TemporalPhase::Historical
        );
    }

    #[test]
    fn test_detect_anomaly_severity() {
        assert_eq!(detect_anomaly_severity(1.0), AnomalySeverity::Normal);
        assert_eq!(detect_anomaly_severity(1.1), AnomalySeverity::Normal);
        assert_eq!(detect_anomaly_severity(1.2), AnomalySeverity::Minor);
        assert_eq!(detect_anomaly_severity(1.3), AnomalySeverity::Minor);
        assert_eq!(detect_anomaly_severity(1.5), AnomalySeverity::Moderate);
        assert_eq!(detect_anomaly_severity(1.8), AnomalySeverity::Moderate);
        assert_eq!(detect_anomaly_severity(2.0), AnomalySeverity::Critical);
        assert_eq!(detect_anomaly_severity(3.0), AnomalySeverity::Critical);
    }

    #[test]
    fn test_format_severity() {
        assert_eq!(format_severity(AnomalySeverity::Normal), "Normal");
        assert_eq!(format_severity(AnomalySeverity::Minor), "ℹ️  Minor");
        assert_eq!(format_severity(AnomalySeverity::Moderate), "⚠️  Moderate");
        assert_eq!(format_severity(AnomalySeverity::Critical), "🔥 Critical");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.00 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(45.0), "45s");
        assert_eq!(format_uptime(300.0), "5m 0s");
        assert_eq!(format_uptime(3700.0), "1h 1m");
        assert_eq!(format_uptime(90000.0), "1d 1h");
    }

    #[test]
    fn test_get_5min_rolling_avg() {
        let mut history = Vec::new();

        // Create 10 entries spanning 5 minutes (30 second intervals)
        for i in 0..10 {
            history.push(RingbufferEntry {
                timestamp: 1000 + i * 30,
                rss_kb: 100 + i as u64 * 10, // Growing RSS
                pss_kb: 90,
                uss_kb: 80,
                cpu_percent: 5.0,
                cpu_time_seconds: 1.0,
                top_cpu: [TopProcessInfo::default(); 3],
                top_rss: [TopProcessInfo::default(); 3],
                top_pss: [TopProcessInfo::default(); 3],
                _padding: [],
            });
        }

        let avg = get_5min_rolling_avg(&history, 30, |e| e.rss_kb * 1024);
        assert!(avg.is_some());

        // Average of entries 0-9: 100, 110, 120, ... 190
        // Average = (100 + 110 + 120 + 130 + 140 + 150 + 160 + 170 + 180 + 190) / 10 = 145
        let expected_kb = 145u64;
        let expected_bytes = expected_kb * 1024;
        assert_eq!(avg.unwrap(), expected_bytes);
    }

    #[test]
    fn test_extract_min_max_avg_with_timestamps() {
        let mut history = Vec::new();

        // Create entries with varying RSS
        let values = vec![100, 150, 90, 200, 120];
        for (i, val) in values.iter().enumerate() {
            history.push(RingbufferEntry {
                timestamp: 1000 + i as i64 * 60,
                rss_kb: *val,
                pss_kb: 90,
                uss_kb: 80,
                cpu_percent: 5.0,
                cpu_time_seconds: 1.0,
                top_cpu: [TopProcessInfo::default(); 3],
                top_rss: [TopProcessInfo::default(); 3],
                top_pss: [TopProcessInfo::default(); 3],
                _padding: [],
            });
        }

        let triplet = extract_min_max_avg_with_timestamps(&history, |e| e.rss_kb * 1024);
        assert!(triplet.is_some());

        let t = triplet.unwrap();
        assert_eq!(t.min.value, 90 * 1024); // Entry 2
        assert_eq!(t.max.value, 200 * 1024); // Entry 3
        assert_eq!(t.avg, 132 * 1024); // (100+150+90+200+120)/5 = 132
    }

    #[test]
    fn test_calculate_growth_rate() {
        let mut history = Vec::new();

        // Create entries spanning 2 hours with steady growth
        for i in 0..120 {
            history.push(RingbufferEntry {
                timestamp: 1000 + i * 60,     // Every minute
                rss_kb: 1000 + i as u64 * 10, // Growing by 10KB/min
                pss_kb: 90,
                uss_kb: 80,
                cpu_percent: 5.0,
                cpu_time_seconds: 1.0,
                top_cpu: [TopProcessInfo::default(); 3],
                top_rss: [TopProcessInfo::default(); 3],
                top_pss: [TopProcessInfo::default(); 3],
                _padding: [],
            });
        }

        // Current value is at entry 119 (last entry would be 120, so 119 is the most recent)
        let current_value = (1000 + 119 * 10) * 1024; // in bytes
        let rate = calculate_growth_rate(current_value, &history, 60, |e| e.rss_kb * 1024);
        assert!(rate.is_some());

        // Expected: growth from entry 59 (1590KB) to entry 119 (2190KB) = 600KB over 3600 seconds
        // = 600*1024 / 3600 bytes/sec ≈ 170.67 bytes/sec
        let r = rate.unwrap();
        assert!(r > 160.0 && r < 180.0); // Roughly 170 bytes/sec
    }
}
