//! Details endpoint handler.
//!
//! This module provides the `/details` endpoint handler that displays
//! ringbuffer statistics, historical metrics, and live process snapshots for subgroups.

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

/// Live snapshot data for a single subgroup.
#[derive(Debug, Clone)]
struct SubgroupSnapshot {
    process_count: usize,
    total_rss: u64,
    total_pss: u64,
    total_uss: u64,
    oldest_uptime_seconds: f64,
    top_processes_by_rss: Vec<ProcessInfo>,
    top_processes_by_cpu: Vec<ProcessInfo>,
    top_processes_by_pss: Vec<ProcessInfo>,
    top_processes_by_blkio_read: Vec<ProcessInfo>,
    top_processes_by_blkio_write: Vec<ProcessInfo>,
}

/// Information about a single process for display.
#[derive(Debug, Clone)]
struct ProcessInfo {
    pid: u32,
    name: String,
    rss: u64,
    pss: u64,
    cpu_percent: f32,
    uptime_seconds: f64,
    read_bytes: u64,
    write_bytes: u64,
}

/// Computes live snapshot for all subgroups from the current cache.
async fn compute_live_snapshots(
    state: &SharedState,
    top_n: usize,
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

        // Helper function to create ProcessInfo from ProcMem
        let to_process_info = |p: &ProcMem| ProcessInfo {
            pid: p.pid,
            name: p.name.clone(),
            rss: p.rss,
            pss: p.pss,
            cpu_percent: p.cpu_percent,
            uptime_seconds: system_uptime - p.start_time_seconds,
            read_bytes: p.read_bytes,
            write_bytes: p.write_bytes,
        };

        // Create indices and sort them instead of cloning the entire vector
        let mut indices: Vec<usize> = (0..procs.len()).collect();
        
        // Sort by RSS descending
        let mut indices_rss = indices.clone();
        indices_rss.sort_by(|&a, &b| procs[b].rss.cmp(&procs[a].rss));
        let top_processes_by_rss: Vec<ProcessInfo> = indices_rss
            .iter()
            .take(top_n)
            .map(|&i| to_process_info(&procs[i]))
            .collect();

        // Sort by CPU descending
        let mut indices_cpu = indices.clone();
        indices_cpu.sort_by(|&a, &b| {
            procs[b].cpu_percent
                .partial_cmp(&procs[a].cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_processes_by_cpu: Vec<ProcessInfo> = indices_cpu
            .iter()
            .take(top_n)
            .map(|&i| to_process_info(&procs[i]))
            .collect();

        // Sort by PSS descending
        let mut indices_pss = indices.clone();
        indices_pss.sort_by(|&a, &b| procs[b].pss.cmp(&procs[a].pss));
        let top_processes_by_pss: Vec<ProcessInfo> = indices_pss
            .iter()
            .take(top_n)
            .map(|&i| to_process_info(&procs[i]))
            .collect();

        // Sort by Block I/O Read descending
        let mut indices_read = indices.clone();
        indices_read.sort_by(|&a, &b| procs[b].read_bytes.cmp(&procs[a].read_bytes));
        let top_processes_by_blkio_read: Vec<ProcessInfo> = indices_read
            .iter()
            .take(top_n)
            .map(|&i| to_process_info(&procs[i]))
            .collect();

        // Sort by Block I/O Write descending
        let mut indices_write = indices;
        indices_write.sort_by(|&a, &b| procs[b].write_bytes.cmp(&procs[a].write_bytes));
        let top_processes_by_blkio_write: Vec<ProcessInfo> = indices_write
            .iter()
            .take(top_n)
            .map(|&i| to_process_info(&procs[i]))
            .collect();

        snapshots.insert(
            subgroup_key,
            SubgroupSnapshot {
                process_count,
                total_rss,
                total_pss,
                total_uss,
                oldest_uptime_seconds,
                top_processes_by_rss,
                top_processes_by_cpu,
                top_processes_by_pss,
                top_processes_by_blkio_read,
                top_processes_by_blkio_write,
            },
        );
    }

    snapshots
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
fn render_history(out: &mut String, _subgroup_name: &str, history: &[crate::ringbuffer::RingbufferEntry], interval_seconds: u64) {
    if history.is_empty() {
        writeln!(out, "No history available").ok();
        return;
    }

    let history_length = history.len();
    let history_minutes = (history_length as u64 * interval_seconds) / 60;

    // Calculate averages
    let avg_rss_kb: f64 = history.iter().map(|e| e.rss_kb as f64).sum::<f64>() / history_length as f64;
    let avg_pss_kb: f64 = history.iter().map(|e| e.pss_kb as f64).sum::<f64>() / history_length as f64;
    let avg_uss_kb: f64 = history.iter().map(|e| e.uss_kb as f64).sum::<f64>() / history_length as f64;

    let latest_entry = history.last().unwrap();
    let latest_time = chrono::NaiveDateTime::from_timestamp_opt(latest_entry.timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    writeln!(out, "AGGREGATED HISTORY (from ringbuffer):").ok();
    writeln!(out, "  History length:       {} entries ({} minutes)", history_length, history_minutes).ok();
    writeln!(out, "  Latest entry:         {}", latest_time).ok();
    writeln!(out, "  Average RSS:          {}", format_bytes((avg_rss_kb * 1024.0) as u64)).ok();
    writeln!(out, "  Average PSS:          {}", format_bytes((avg_pss_kb * 1024.0) as u64)).ok();
    writeln!(out, "  Average USS:          {}", format_bytes((avg_uss_kb * 1024.0) as u64)).ok();
}

/// Renders live snapshot data for a subgroup.
fn render_snapshot(out: &mut String, snapshot: &SubgroupSnapshot) {
    writeln!(out).ok();
    writeln!(out, "LIVE SNAPSHOT (current):").ok();
    writeln!(out, "  Process count:        {}", snapshot.process_count).ok();
    writeln!(out, "  Total RSS:            {}", format_bytes(snapshot.total_rss)).ok();
    writeln!(out, "  Total PSS:            {}", format_bytes(snapshot.total_pss)).ok();
    writeln!(out, "  Total USS:            {}", format_bytes(snapshot.total_uss)).ok();
    writeln!(out, "  Oldest uptime:        {}", format_uptime(snapshot.oldest_uptime_seconds)).ok();
    // Placeholder for future alert_armed logic - currently always NO
    writeln!(out, "  Alert armed:          NO").ok();

    writeln!(out).ok();
    writeln!(out, "TOP PROCESS METRICS:").ok();
    writeln!(out, "====================").ok();
    
    // Top-3 by CPU Usage
    writeln!(out).ok();
    writeln!(out, "TOP PROCESSES (by CPU):").ok();
    writeln!(
        out,
        "  {:<6} {:<8} {:<16} {:<8} {}",
        "Rank", "PID", "Name", "CPU%", "CPU Time"
    )
    .ok();
    
    for (rank, proc) in snapshot.top_processes_by_cpu.iter().enumerate() {
        writeln!(
            out,
            "  {:<6} {:<8} {:<16} {:>6.1}% {:.2}s",
            rank + 1,
            proc.pid,
            if proc.name.len() > 16 {
                &proc.name[..16]
            } else {
                &proc.name
            },
            proc.cpu_percent,
            proc.uptime_seconds
        )
        .ok();
    }
    
    // Top-3 by Memory (RSS)
    writeln!(out).ok();
    writeln!(out, "TOP PROCESSES (by RSS):").ok();
    writeln!(
        out,
        "  {:<6} {:<8} {:<16} {:<12} {}",
        "Rank", "PID", "Name", "RSS", "Uptime"
    )
    .ok();

    for (rank, proc) in snapshot.top_processes_by_rss.iter().enumerate() {
        writeln!(
            out,
            "  {:<6} {:<8} {:<16} {:<12} {}",
            rank + 1,
            proc.pid,
            if proc.name.len() > 16 {
                &proc.name[..16]
            } else {
                &proc.name
            },
            format_bytes(proc.rss),
            format_uptime(proc.uptime_seconds)
        )
        .ok();
    }
    
    // Top-3 by Memory (PSS)
    writeln!(out).ok();
    writeln!(out, "TOP PROCESSES (by PSS):").ok();
    writeln!(
        out,
        "  {:<6} {:<8} {:<16} {:<12} {}",
        "Rank", "PID", "Name", "PSS", "Uptime"
    )
    .ok();

    for (rank, proc) in snapshot.top_processes_by_pss.iter().enumerate() {
        writeln!(
            out,
            "  {:<6} {:<8} {:<16} {:<12} {}",
            rank + 1,
            proc.pid,
            if proc.name.len() > 16 {
                &proc.name[..16]
            } else {
                &proc.name
            },
            format_bytes(proc.pss),
            format_uptime(proc.uptime_seconds)
        )
        .ok();
    }
    
    // Top-3 by Block I/O Read
    writeln!(out).ok();
    writeln!(out, "TOP PROCESSES (by Block I/O Read):").ok();
    writeln!(
        out,
        "  {:<6} {:<8} {:<16} {}",
        "Rank", "PID", "Name", "Read Bytes"
    )
    .ok();

    for (rank, proc) in snapshot.top_processes_by_blkio_read.iter().enumerate() {
        writeln!(
            out,
            "  {:<6} {:<8} {:<16} {}",
            rank + 1,
            proc.pid,
            if proc.name.len() > 16 {
                &proc.name[..16]
            } else {
                &proc.name
            },
            if proc.read_bytes > 0 {
                format_bytes(proc.read_bytes)
            } else {
                "N/A".to_string()
            }
        )
        .ok();
    }
    
    // Top-3 by Block I/O Write
    writeln!(out).ok();
    writeln!(out, "TOP PROCESSES (by Block I/O Write):").ok();
    writeln!(
        out,
        "  {:<6} {:<8} {:<16} {}",
        "Rank", "PID", "Name", "Write Bytes"
    )
    .ok();

    for (rank, proc) in snapshot.top_processes_by_blkio_write.iter().enumerate() {
        writeln!(
            out,
            "  {:<6} {:<8} {:<16} {}",
            rank + 1,
            proc.pid,
            if proc.name.len() > 16 {
                &proc.name[..16]
            } else {
                &proc.name
            },
            if proc.write_bytes > 0 {
                format_bytes(proc.write_bytes)
            } else {
                "N/A".to_string()
            }
        )
        .ok();
    }
    
    writeln!(out).ok();
    writeln!(out, "Note: Block I/O data from /proc/[pid]/io").ok();
    writeln!(out, "      Network metrics require eBPF support (see /html/docs)").ok();
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
    let top_n = state.config.details_top_n.unwrap_or(5);

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
    let snapshots = compute_live_snapshots(&state, top_n).await;

    // If subgroup specified, show detailed view
    if let Some(subgroup_name) = params.subgroup {
        writeln!(out, "SUBGROUP: {}", subgroup_name).ok();
        writeln!(out, "=====================").ok();
        writeln!(out).ok();

        // Show historical data if available
        if let Some(history) = state
            .ringbuffer_manager
            .get_subgroup_history(&subgroup_name)
        {
            render_history(&mut out, &subgroup_name, &history, stats.interval_seconds);
        } else {
            writeln!(out, "AGGREGATED HISTORY (from ringbuffer):").ok();
            writeln!(out, "  No history available").ok();
        }

        // Show live snapshot if available
        if let Some(snapshot) = snapshots.get(&subgroup_name) {
            render_snapshot(&mut out, snapshot);
        } else {
            writeln!(out).ok();
            writeln!(out, "LIVE SNAPSHOT (current):").ok();
            writeln!(out, "  No processes found").ok();
        }
    } else {
        // List all subgroups with live snapshots
        writeln!(out, "AVAILABLE SUBGROUPS").ok();
        writeln!(out, "===================").ok();
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
                // Placeholder for future alert_armed logic - currently always NO
                writeln!(out, "  Alert armed:      NO").ok();
            }

            writeln!(out).ok();
            writeln!(out, "  Use ?subgroup={} to view detailed history and top processes", subgroup_name).ok();
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
