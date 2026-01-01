//! HTML endpoint handlers for human-friendly inspection and debugging.
//!
//! This module provides HTML views for the existing /details data,
//! using only in-memory data structures. No new calculations or state changes.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::atomic::Ordering;
use tracing::{debug, instrument};

use crate::cache::ProcMem;
use crate::handlers::health::FOOTER_TEXT;
use crate::process::classify_process_raw;
use crate::state::SharedState;

/// Query parameters for HTML details endpoint.
#[derive(Deserialize, Debug)]
pub struct HtmlDetailsQuery {
    pub subgroup: Option<String>,
}

/// Query parameters for HTML subgroups endpoint (for sorting).
#[derive(Deserialize, Debug)]
pub struct HtmlSubgroupsQuery {
    pub sort: Option<String>, // "rss" or "cpu"
}

/// Generate HTML header with title and navigation.
fn html_header(title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - Herakles Node Exporter</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .container {{ max-width: 1400px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 3px solid #007bff; padding-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; }}
        h3 {{ color: #666; }}
        nav {{ background: #007bff; padding: 15px; border-radius: 4px; margin-bottom: 20px; }}
        nav a {{ color: white; text-decoration: none; margin-right: 20px; font-weight: 500; }}
        nav a:hover {{ text-decoration: underline; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th {{ background: #007bff; color: white; padding: 12px; text-align: left; font-weight: 600; }}
        td {{ padding: 10px; border-bottom: 1px solid #ddd; }}
        tr:hover {{ background: #f8f9fa; }}
        .metric {{ display: inline-block; margin: 10px 20px 10px 0; padding: 10px 15px; background: #e9ecef; border-radius: 4px; }}
        .metric-label {{ font-weight: 600; color: #555; }}
        .metric-value {{ font-size: 1.2em; color: #007bff; }}
        .footer {{ margin-top: 40px; padding-top: 20px; border-top: 1px solid #ddd; color: #666; font-size: 0.9em; }}
        .status-ok {{ color: #28a745; font-weight: 600; }}
        .status-warn {{ color: #ffc107; font-weight: 600; }}
        .status-error {{ color: #dc3545; font-weight: 600; }}
        a {{ color: #007bff; text-decoration: none; }}
        a:hover {{ text-decoration: underline; }}
        .info-box {{ background: #d1ecf1; border: 1px solid #bee5eb; border-radius: 4px; padding: 15px; margin: 20px 0; }}
        code {{ background: #f8f9fa; padding: 2px 6px; border-radius: 3px; font-family: 'Courier New', monospace; }}
    </style>
</head>
<body>
<div class="container">
<nav>
    <a href="/html/">Home</a>
    <a href="/html/details">Details</a>
    <a href="/html/subgroups">Subgroups</a>
    <a href="/html/health">Health</a>
    <a href="/html/config">Config</a>
    <a href="/html/docs">Docs</a>
</nav>
"#
    )
}

/// Generate HTML footer.
fn html_footer() -> String {
    format!(
        r#"<div class="footer">
    <p>{}</p>
</div>
</div>
</body>
</html>"#,
        FOOTER_TEXT
    )
}

/// Format bytes to human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Helper function to render top-N processes table in HTML
fn render_top_processes_table<F>(
    html: &mut String,
    title: &str,
    processes: &[&ProcMem],
    value_fn: F,
    value_header: &str,
) where
    F: Fn(&ProcMem) -> String,
{
    html.push_str(&format!("<h4>{}</h4>\n", title));
    html.push_str("<table>\n");
    html.push_str(&format!(
        "<tr><th>Rank</th><th>PID</th><th>Name</th><th>{}</th></tr>\n",
        value_header
    ));

    for (rank, proc) in processes.iter().take(3).enumerate() {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            rank + 1,
            proc.pid,
            proc.name,
            value_fn(proc)
        ));
    }

    html.push_str("</table>\n");
}

/// Handler for /html/ (landing page).
#[instrument(skip(state))]
pub async fn html_index_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /html/ request");
    state.health_stats.record_http_request();

    let stats = state.ringbuffer_manager.get_stats();
    
    // Calculate uptime from service start time
    let uptime_secs = state.start_time.elapsed().as_secs();
    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let seconds = uptime_secs % 60;
    let uptime_str = format!("{}h {}m {}s", hours, minutes, seconds);

    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    let mut html = html_header("Home");
    html.push_str("<h1>Herakles Node Exporter</h1>\n");
    html.push_str("<p>Human-friendly HTML views for inspection and debugging</p>\n");

    html.push_str("<h2>Overview</h2>\n");
    html.push_str(r#"<div class="metric"><span class="metric-label">Version:</span> <span class="metric-value">0.1.0</span></div>"#);
    html.push_str(&format!(
        r#"<div class="metric"><span class="metric-label">Hostname:</span> <span class="metric-value">{}</span></div>"#,
        hostname
    ));
    html.push_str(&format!(
        r#"<div class="metric"><span class="metric-label">Uptime:</span> <span class="metric-value">{}</span></div>"#,
        uptime_str
    ));
    html.push_str(&format!(
        r#"<div class="metric"><span class="metric-label">Subgroups:</span> <span class="metric-value">{}</span></div>"#,
        stats.total_subgroups
    ));
    html.push_str(&format!(
        r#"<div class="metric"><span class="metric-label">Ringbuffer RAM:</span> <span class="metric-value">{} / {} MB</span></div>"#,
        stats.estimated_ram_bytes / (1024 * 1024),
        stats.max_memory_mb
    ));

    html.push_str("<h2>Quick Links</h2>\n");
    html.push_str("<ul>\n");
    html.push_str(r#"<li><a href="/html/details">Details</a> - Ringbuffer statistics and subgroup history</li>"#);
    html.push_str(r#"<li><a href="/html/subgroups">Subgroups</a> - All subgroups with current metrics</li>"#);
    html.push_str(r#"<li><a href="/html/health">Health</a> - Exporter health and buffer status</li>"#);
    html.push_str(r#"<li><a href="/html/config">Config</a> - Current configuration</li>"#);
    html.push_str(r#"<li><a href="/html/docs">Docs</a> - Documentation and FAQ</li>"#);
    html.push_str("</ul>\n");

    html.push_str(&html_footer());
    Html(html)
}

/// Handler for /html/details.
#[instrument(skip(state))]
pub async fn html_details_handler(
    State(state): State<SharedState>,
    Query(params): Query<HtmlDetailsQuery>,
) -> impl IntoResponse {
    debug!("Processing /html/details request");
    state.health_stats.record_http_request();

    let cache = state.cache.read().await;
    let stats = state.ringbuffer_manager.get_stats();

    let mut html = html_header("Details");
    html.push_str("<h1>Details</h1>\n");

    // Show ringbuffer configuration
    html.push_str("<h2>Ringbuffer Configuration</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Setting</th><th>Value</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>Max Memory</td><td>{} MB</td></tr>\n",
        stats.max_memory_mb
    ));
    html.push_str(&format!(
        "<tr><td>Entry Size</td><td>{} bytes</td></tr>\n",
        stats.entry_size_bytes
    ));
    html.push_str(&format!(
        "<tr><td>Interval</td><td>{} seconds</td></tr>\n",
        stats.interval_seconds
    ));
    html.push_str(&format!(
        "<tr><td>Entries per Subgroup</td><td>{}</td></tr>\n",
        stats.entries_per_subgroup
    ));
    html.push_str(&format!(
        "<tr><td>Total Subgroups</td><td>{}</td></tr>\n",
        stats.total_subgroups
    ));
    html.push_str(&format!(
        "<tr><td>Estimated RAM</td><td>{}</td></tr>\n",
        format_bytes(stats.estimated_ram_bytes as u64)
    ));
    html.push_str(&format!(
        "<tr><td>History Duration</td><td>{} seconds ({} minutes)</td></tr>\n",
        stats.history_seconds,
        stats.history_seconds / 60
    ));
    html.push_str("</table>\n");

    if let Some(subgroup_name) = params.subgroup {
        // Show specific subgroup details
        if let Some(history) = state.ringbuffer_manager.get_subgroup_history(&subgroup_name) {
            html.push_str(&format!("<h2>Subgroup: {}</h2>\n", subgroup_name));

            // Calculate current aggregated values from cache
            let mut subgroup_processes: Vec<&ProcMem> = Vec::new();
            for proc in cache.processes.values() {
                let (_, sg) = classify_process_raw(&proc.name);
                let key = format!("{}:{}", classify_process_raw(&proc.name).0, sg);
                if key == subgroup_name {
                    subgroup_processes.push(proc);
                }
            }

            if !subgroup_processes.is_empty() {
                html.push_str("<h3>Current Aggregated Values</h3>\n");
                
                let total_rss: u64 = subgroup_processes.iter().map(|p| p.rss).sum();
                let total_pss: u64 = subgroup_processes.iter().map(|p| p.pss).sum();
                let total_uss: u64 = subgroup_processes.iter().map(|p| p.uss).sum();
                let total_cpu: f64 = subgroup_processes.iter().map(|p| p.cpu_percent as f64).sum();

                // Find oldest uptime
                let oldest_uptime = subgroup_processes
                    .iter()
                    .map(|p| p.start_time_seconds)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);

                html.push_str("<table>\n");
                html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
                html.push_str(&format!(
                    "<tr><td>Process Count</td><td>{}</td></tr>\n",
                    subgroup_processes.len()
                ));
                html.push_str(&format!(
                    "<tr><td>Total RSS</td><td>{}</td></tr>\n",
                    format_bytes(total_rss)
                ));
                html.push_str(&format!(
                    "<tr><td>Total PSS</td><td>{}</td></tr>\n",
                    format_bytes(total_pss)
                ));
                html.push_str(&format!(
                    "<tr><td>Total USS</td><td>{}</td></tr>\n",
                    format_bytes(total_uss)
                ));
                html.push_str(&format!(
                    "<tr><td>Total CPU Usage</td><td>{:.2}%</td></tr>\n",
                    total_cpu
                ));
                html.push_str(&format!(
                    "<tr><td>Oldest Process Start</td><td>{:.2}s</td></tr>\n",
                    oldest_uptime
                ));
                html.push_str("</table>\n");
                
                // Add Top Process Metrics section
                html.push_str("<h3>Top Process Metrics</h3>\n");
                html.push_str(r#"<div class="info-box">Data sources: /proc/[pid]/stat (CPU), /proc/[pid]/statm (RSS), /proc/[pid]/smaps_rollup (PSS), /proc/[pid]/io (Block I/O)</div>"#);
                html.push_str("\n");
                
                // Top-3 by CPU Usage (custom rendering for CPU with two columns)
                html.push_str("<h4>Top-3 Processes by CPU Usage</h4>\n");
                html.push_str("<table>\n");
                html.push_str("<tr><th>Rank</th><th>PID</th><th>Name</th><th>CPU %</th><th>CPU Time (s)</th></tr>\n");
                
                let mut sorted_by_cpu: Vec<_> = subgroup_processes.iter().map(|&p| p).collect();
                sorted_by_cpu.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
                for (rank, proc) in sorted_by_cpu.iter().take(3).enumerate() {
                    html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}%</td><td>{:.2}</td></tr>\n",
                        rank + 1,
                        proc.pid,
                        proc.name,
                        proc.cpu_percent,
                        proc.cpu_time_seconds
                    ));
                }
                html.push_str("</table>\n");
                
                // Top-3 by Memory (RSS)
                let mut sorted_by_rss: Vec<_> = subgroup_processes.iter().map(|&p| p).collect();
                sorted_by_rss.sort_by(|a, b| b.rss.cmp(&a.rss));
                render_top_processes_table(
                    &mut html,
                    "Top-3 Processes by Memory (RSS)",
                    &sorted_by_rss,
                    |p| format_bytes(p.rss),
                    "RSS",
                );
                
                // Top-3 by Memory (PSS)
                let mut sorted_by_pss: Vec<_> = subgroup_processes.iter().map(|&p| p).collect();
                sorted_by_pss.sort_by(|a, b| b.pss.cmp(&a.pss));
                render_top_processes_table(
                    &mut html,
                    "Top-3 Processes by Memory (PSS)",
                    &sorted_by_pss,
                    |p| format_bytes(p.pss),
                    "PSS",
                );
                
                // Top-3 by Block I/O Read
                let mut sorted_by_read: Vec<_> = subgroup_processes.iter().map(|&p| p).collect();
                sorted_by_read.sort_by(|a, b| b.read_bytes.cmp(&a.read_bytes));
                render_top_processes_table(
                    &mut html,
                    "Top-3 Processes by Block I/O Read",
                    &sorted_by_read,
                    |p| if p.read_bytes > 0 { format_bytes(p.read_bytes) } else { "N/A".to_string() },
                    "Read Bytes",
                );
                
                // Top-3 by Block I/O Write
                let mut sorted_by_write: Vec<_> = subgroup_processes.iter().map(|&p| p).collect();
                sorted_by_write.sort_by(|a, b| b.write_bytes.cmp(&a.write_bytes));
                render_top_processes_table(
                    &mut html,
                    "Top-3 Processes by Block I/O Write",
                    &sorted_by_write,
                    |p| if p.write_bytes > 0 { format_bytes(p.write_bytes) } else { "N/A".to_string() },
                    "Write Bytes",
                );
                
                // Note about network metrics
                html.push_str(r#"<div class="info-box"><strong>Note:</strong> Network metrics (RX/TX) require eBPF support. See <a href="/html/docs">documentation</a> for setup.</div>"#);
                html.push_str("\n");
            }

            // Show ringbuffer history
            html.push_str("<h3>Ringbuffer History</h3>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Timestamp</th><th>RSS (KB)</th><th>PSS (KB)</th><th>USS (KB)</th><th>CPU %</th><th>CPU Time (s)</th></tr>\n");

            for entry in history {
                let dt = chrono::NaiveDateTime::from_timestamp_opt(entry.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| entry.timestamp.to_string());

                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.1}</td><td>{:.2}</td></tr>\n",
                    dt, entry.rss_kb, entry.pss_kb, entry.uss_kb, entry.cpu_percent, entry.cpu_time_seconds
                ));
            }

            html.push_str("</table>\n");

            // Show top-1 process details if available
            if let Some(top_proc) = subgroup_processes.iter().max_by_key(|p| p.rss) {
                html.push_str("<h3>Top Process (by RSS)</h3>\n");
                html.push_str("<table>\n");
                html.push_str("<tr><th>PID</th><th>Name</th><th>RSS</th><th>PSS</th><th>USS</th><th>CPU %</th></tr>\n");
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td></tr>\n",
                    top_proc.pid,
                    top_proc.name,
                    format_bytes(top_proc.rss),
                    format_bytes(top_proc.pss),
                    format_bytes(top_proc.uss),
                    top_proc.cpu_percent
                ));
                html.push_str("</table>\n");
            }
        } else {
            html.push_str(&format!("<p>Subgroup '{}' not found.</p>\n", subgroup_name));
        }
    } else {
        // List all available subgroups
        html.push_str("<h2>Available Subgroups</h2>\n");
        html.push_str("<p>Click a subgroup to view its history and details.</p>\n");

        let mut subgroups = state.ringbuffer_manager.get_all_subgroups();
        subgroups.sort();

        html.push_str("<ul>\n");
        for sg in subgroups {
            html.push_str(&format!(
                r#"<li><a href="/html/details?subgroup={}">{}</a></li>"#,
                sg, sg
            ));
        }
        html.push_str("</ul>\n");
    }

    html.push_str(&html_footer());
    Html(html)
}

/// Handler for /html/subgroups.
#[instrument(skip(state))]
pub async fn html_subgroups_handler(
    State(state): State<SharedState>,
    Query(params): Query<HtmlSubgroupsQuery>,
) -> impl IntoResponse {
    debug!("Processing /html/subgroups request");
    state.health_stats.record_http_request();

    let cache = state.cache.read().await;

    // Aggregate data by subgroup
    let mut subgroup_data: std::collections::HashMap<String, (u64, u64, u64, f64, usize)> =
        std::collections::HashMap::new();

    for proc in cache.processes.values() {
        let (group, subgroup) = classify_process_raw(&proc.name);
        let key = format!("{}:{}", group, subgroup);

        let entry = subgroup_data.entry(key).or_insert((0, 0, 0, 0.0, 0));
        entry.0 += proc.rss;
        entry.1 += proc.pss;
        entry.2 += proc.uss;
        entry.3 += proc.cpu_percent as f64;
        entry.4 += 1;
    }

    // Convert to vector for sorting
    let mut subgroups: Vec<_> = subgroup_data.into_iter().collect();

    // Sort based on query parameter
    match params.sort.as_deref() {
        Some("rss") => subgroups.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)),
        Some("cpu") => subgroups.sort_by(|a, b| b.1 .3.partial_cmp(&a.1 .3).unwrap()),
        _ => subgroups.sort_by(|a, b| a.0.cmp(&b.0)), // Default: alphabetical
    }

    let mut html = html_header("Subgroups");
    html.push_str("<h1>Subgroups</h1>\n");
    html.push_str("<p>All active subgroups with current metrics. Click column headers to sort.</p>\n");

    html.push_str(r#"<div style="margin: 20px 0;">
        <a href="/html/subgroups">Alphabetical</a> | 
        <a href="/html/subgroups?sort=rss">Sort by RSS</a> | 
        <a href="/html/subgroups?sort=cpu">Sort by CPU</a>
    </div>"#);

    html.push_str("<table>\n");
    html.push_str("<tr><th>Subgroup</th><th>Process Count</th><th>RSS</th><th>PSS</th><th>USS</th><th>CPU %</th></tr>\n");

    for (subgroup_key, (rss, pss, uss, cpu, count)) in subgroups {
        html.push_str(&format!(
            r#"<tr><td><a href="/html/details?subgroup={}">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td></tr>"#,
            subgroup_key,
            subgroup_key,
            count,
            format_bytes(rss),
            format_bytes(pss),
            format_bytes(uss),
            cpu
        ));
        html.push_str("\n");
    }

    html.push_str("</table>\n");
    html.push_str(&html_footer());
    Html(html)
}

/// Handler for /html/health.
#[instrument(skip(state))]
pub async fn html_health_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /html/health request");
    state.health_stats.record_http_request();

    let cache = state.cache.read().await;
    let buffer_health = state.health_state.get_health();

    let status = if cache.update_success && cache.last_updated.is_some() {
        "OK"
    } else {
        "ERROR"
    };

    let mut html = html_header("Health");
    html.push_str("<h1>Health Status</h1>\n");

    let status_class = if status == "OK" {
        "status-ok"
    } else {
        "status-error"
    };
    html.push_str(&format!(
        r#"<p class="{}">Status: {}</p>"#,
        status_class, status
    ));

    // Scan Performance
    html.push_str("<h2>Scan Performance</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");

    let total_scans = state.health_stats.total_scans.load(Ordering::Relaxed);
    let successful_scans = state
        .health_stats
        .scan_success_count
        .load(Ordering::Relaxed);
    let failed_scans = state
        .health_stats
        .scan_failure_count
        .load(Ordering::Relaxed);
    let (_, avg_duration, _, _, _) = state.health_stats.scan_duration_seconds.snapshot();
    let (_, avg_processes, _, _, _) = state.health_stats.scanned_processes.snapshot();

    html.push_str(&format!(
        "<tr><td>Total Scans</td><td>{}</td></tr>\n",
        total_scans
    ));
    html.push_str(&format!(
        "<tr><td>Successful Scans</td><td>{}</td></tr>\n",
        successful_scans
    ));
    html.push_str(&format!(
        "<tr><td>Failed Scans</td><td>{}</td></tr>\n",
        failed_scans
    ));
    html.push_str(&format!(
        "<tr><td>Avg Duration</td><td>{:.2}ms</td></tr>\n",
        avg_duration * 1000.0
    ));
    html.push_str(&format!(
        "<tr><td>Avg Processes Scanned</td><td>{:.0}</td></tr>\n",
        avg_processes
    ));
    html.push_str("</table>\n");

    // Cache Stats
    html.push_str("<h2>Cache Statistics</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>Cached Processes</td><td>{}</td></tr>\n",
        cache.processes.len()
    ));
    html.push_str(&format!(
        "<tr><td>Last Updated</td><td>{}</td></tr>\n",
        cache
            .last_updated
            .map(|t| format!("{:.2}s ago", t.elapsed().as_secs_f64()))
            .unwrap_or_else(|| "Never".to_string())
    ));
    html.push_str(&format!(
        "<tr><td>Update Duration</td><td>{:.2}ms</td></tr>\n",
        cache.update_duration_seconds * 1000.0
    ));
    html.push_str("</table>\n");

    // Buffer Health
    html.push_str("<h2>Buffer Health</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Buffer</th><th>Usage (KB)</th><th>Capacity (KB)</th><th>Status</th></tr>\n");

    for buffer in &buffer_health.buffers {
        let status_class = match buffer.status.as_str() {
            "healthy" => "status-ok",
            "warning" => "status-warn",
            "critical" => "status-error",
            _ => "",
        };
        html.push_str(&format!(
            r#"<tr><td>{}</td><td>{}</td><td>{}</td><td class="{}">{}</td></tr>"#,
            buffer.name, buffer.current_kb, buffer.capacity_kb, status_class, buffer.status
        ));
        html.push_str("\n");
    }

    html.push_str("</table>\n");
    html.push_str(&format!(
        "<p><strong>Overall Buffer Status:</strong> <span class=\"{}\">{}</span></p>\n",
        match buffer_health.overall_status.as_str() {
            "healthy" => "status-ok",
            "warning" => "status-warn",
            "critical" => "status-error",
            _ => "",
        },
        buffer_health.overall_status
    ));

    // Error Statistics
    html.push_str("<h2>Error Statistics</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Error Type</th><th>Count</th></tr>\n");

    let parse_errors = state.health_stats.parsing_errors.load(Ordering::Relaxed);
    let read_errors = state.health_stats.proc_read_errors.load(Ordering::Relaxed);
    let permission_denied = state
        .health_stats
        .permission_denied_count
        .load(Ordering::Relaxed);

    html.push_str(&format!(
        "<tr><td>Parse Errors</td><td>{}</td></tr>\n",
        parse_errors
    ));
    html.push_str(&format!(
        "<tr><td>Read Errors</td><td>{}</td></tr>\n",
        read_errors
    ));
    html.push_str(&format!(
        "<tr><td>Permission Denied</td><td>{}</td></tr>\n",
        permission_denied
    ));
    html.push_str("</table>\n");

    // eBPF Stats (if available)
    if let Some(ref ebpf_manager) = state.ebpf {
        let perf_stats = ebpf_manager.get_performance_stats();
        if perf_stats.enabled {
            html.push_str("<h2>eBPF Statistics</h2>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
            html.push_str(&format!(
                "<tr><td>Events per Second</td><td>{:.2}</td></tr>\n",
                perf_stats.events_per_sec
            ));
            html.push_str(&format!(
                "<tr><td>Lost Events</td><td>{}</td></tr>\n",
                perf_stats.lost_events_total
            ));
            html.push_str(&format!(
                "<tr><td>Map Usage</td><td>{:.2}%</td></tr>\n",
                perf_stats.map_usage_percent
            ));
            html.push_str(&format!(
                "<tr><td>CPU Overhead</td><td>{:.2}%</td></tr>\n",
                perf_stats.cpu_overhead_percent
            ));
            html.push_str("</table>\n");
        }
    }

    html.push_str(&html_footer());
    Html(html)
}

/// Handler for /html/config.
#[instrument(skip(state))]
pub async fn html_config_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /html/config request");
    state.health_stats.record_http_request();

    let cfg = &state.config;

    let mut html = html_header("Configuration");
    html.push_str("<h1>Configuration</h1>\n");
    html.push_str(r#"<div class="info-box">Read-only view of active configuration. Secrets are not exposed.</div>"#);

    // Server Configuration
    html.push_str("<h2>Server Configuration</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Setting</th><th>Value</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>Bind Address</td><td>{}</td></tr>\n",
        cfg.bind
            .as_deref()
            .unwrap_or(crate::config::DEFAULT_BIND_ADDR)
    ));
    html.push_str(&format!(
        "<tr><td>Port</td><td>{}</td></tr>\n",
        cfg.port.unwrap_or(crate::config::DEFAULT_PORT)
    ));
    html.push_str(&format!(
        "<tr><td>Cache TTL</td><td>{} seconds</td></tr>\n",
        cfg.cache_ttl
            .unwrap_or(crate::config::DEFAULT_CACHE_TTL)
    ));
    html.push_str("</table>\n");

    // Ringbuffer Configuration
    html.push_str("<h2>Ringbuffer Settings</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Setting</th><th>Value</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>Max Memory</td><td>{} MB</td></tr>\n",
        cfg.ringbuffer.max_memory_mb
    ));
    html.push_str(&format!(
        "<tr><td>Interval</td><td>{} seconds</td></tr>\n",
        cfg.ringbuffer.interval_seconds
    ));
    html.push_str(&format!(
        "<tr><td>Min Entries per Subgroup</td><td>{}</td></tr>\n",
        cfg.ringbuffer.min_entries_per_subgroup
    ));
    html.push_str(&format!(
        "<tr><td>Max Entries per Subgroup</td><td>{}</td></tr>\n",
        cfg.ringbuffer.max_entries_per_subgroup
    ));
    html.push_str("</table>\n");

    // Metrics Collection
    html.push_str("<h2>Metrics Collection</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Setting</th><th>Value</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>Min USS</td><td>{} KB</td></tr>\n",
        cfg.min_uss_kb.unwrap_or(0)
    ));
    html.push_str(&format!(
        "<tr><td>Include Names</td><td>{}</td></tr>\n",
        cfg.include_names
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "none".to_string())
    ));
    html.push_str(&format!(
        "<tr><td>Exclude Names</td><td>{}</td></tr>\n",
        cfg.exclude_names
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "none".to_string())
    ));
    html.push_str(&format!(
        "<tr><td>Max Processes</td><td>{}</td></tr>\n",
        cfg.max_processes
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unlimited".to_string())
    ));
    html.push_str("</table>\n");

    // Feature Flags
    html.push_str("<h2>Feature Flags</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Feature</th><th>Enabled</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>Health Endpoint</td><td>{}</td></tr>\n",
        if cfg.enable_health.unwrap_or(true) {
            "✓"
        } else {
            "✗"
        }
    ));
    html.push_str(&format!(
        "<tr><td>eBPF</td><td>{}</td></tr>\n",
        if cfg.enable_ebpf.unwrap_or(false) {
            "✓"
        } else {
            "✗"
        }
    ));
    html.push_str(&format!(
        "<tr><td>TLS</td><td>{}</td></tr>\n",
        if cfg.enable_tls.unwrap_or(false) {
            "✓"
        } else {
            "✗"
        }
    ));
    html.push_str(&format!(
        "<tr><td>RSS Metrics</td><td>{}</td></tr>\n",
        if cfg.enable_rss.unwrap_or(true) {
            "✓"
        } else {
            "✗"
        }
    ));
    html.push_str(&format!(
        "<tr><td>PSS Metrics</td><td>{}</td></tr>\n",
        if cfg.enable_pss.unwrap_or(true) {
            "✓"
        } else {
            "✗"
        }
    ));
    html.push_str(&format!(
        "<tr><td>USS Metrics</td><td>{}</td></tr>\n",
        if cfg.enable_uss.unwrap_or(true) {
            "✓"
        } else {
            "✗"
        }
    ));
    html.push_str(&format!(
        "<tr><td>CPU Metrics</td><td>{}</td></tr>\n",
        if cfg.enable_cpu.unwrap_or(true) {
            "✓"
        } else {
            "✗"
        }
    ));
    html.push_str("</table>\n");

    // Performance Tuning
    html.push_str("<h2>Performance Tuning</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Buffer</th><th>Size (KB)</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>I/O Buffer</td><td>{}</td></tr>\n",
        cfg.io_buffer_kb.unwrap_or(256)
    ));
    html.push_str(&format!(
        "<tr><td>smaps Buffer</td><td>{}</td></tr>\n",
        cfg.smaps_buffer_kb.unwrap_or(512)
    ));
    html.push_str(&format!(
        "<tr><td>smaps_rollup Buffer</td><td>{}</td></tr>\n",
        cfg.smaps_rollup_buffer_kb.unwrap_or(256)
    ));
    html.push_str("</table>\n");

    html.push_str(&html_footer());
    Html(html)
}

/// Handler for /html/docs.
#[instrument(skip(state))]
pub async fn html_docs_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /html/docs request");
    state.health_stats.record_http_request();

    let mut html = html_header("Documentation");
    html.push_str("<h1>Documentation</h1>\n");

    // Mental Model
    html.push_str("<h2>Mental Model</h2>\n");
    html.push_str(r#"<div class="info-box">
        <p><strong>Node:</strong> A physical or virtual machine running the exporter.</p>
        <p><strong>Subgroup:</strong> A logical grouping of processes based on their name patterns. For example, all Java processes might be grouped under <code>java:java</code>.</p>
        <p><strong>Process:</strong> An individual running process on the system.</p>
    </div>"#);

    // What Metrics Represent
    html.push_str("<h2>What Metrics Represent</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Metric</th><th>Description</th></tr>\n");
    html.push_str("<tr><td><strong>RSS</strong></td><td>Resident Set Size - Total physical memory used by a process (includes shared memory)</td></tr>\n");
    html.push_str("<tr><td><strong>PSS</strong></td><td>Proportional Set Size - RSS with shared memory divided proportionally across processes</td></tr>\n");
    html.push_str("<tr><td><strong>USS</strong></td><td>Unique Set Size - Memory unique to a process (not shared)</td></tr>\n");
    html.push_str("<tr><td><strong>CPU %</strong></td><td>CPU usage percentage for the process or subgroup</td></tr>\n");
    html.push_str("<tr><td><strong>CPU Time</strong></td><td>Cumulative CPU time consumed by the process</td></tr>\n");
    html.push_str("</table>\n");

    // What /details Shows
    html.push_str("<h2>What /details Shows</h2>\n");
    html.push_str(r#"<div class="info-box">
        <p>The <code>/details</code> endpoint (both text and HTML versions) displays:</p>
        <ul>
            <li><strong>Ringbuffer Configuration:</strong> Memory limits, intervals, and capacity</li>
            <li><strong>Available Subgroups:</strong> List of all active process subgroups</li>
            <li><strong>Subgroup History:</strong> Time-series data for a specific subgroup showing how metrics evolve over time</li>
        </ul>
    </div>"#);

    // Purpose of Ringbuffers
    html.push_str("<h2>Purpose of Ringbuffers</h2>\n");
    html.push_str(r#"<div class="info-box">
        <p>Ringbuffers store historical metrics for each subgroup in a fixed-size circular buffer. This allows:</p>
        <ul>
            <li><strong>Trend Analysis:</strong> See how memory and CPU usage change over time</li>
            <li><strong>Memory Efficiency:</strong> Fixed memory usage regardless of runtime duration</li>
            <li><strong>No External Dependencies:</strong> Historical data kept in-process without external storage</li>
        </ul>
    </div>"#);

    // Why Ringbuffers are RAM-Limited
    html.push_str("<h2>Why Ringbuffers are RAM-Limited</h2>\n");
    html.push_str(r#"<div class="info-box">
        <p>Ringbuffers use a fixed amount of RAM to prevent unbounded memory growth. The <code>max_memory_mb</code> setting controls the total RAM budget. This is divided across all subgroups to provide a predictable memory footprint.</p>
        <p>As new data arrives, the oldest entries are overwritten. This ensures the exporter itself remains lightweight.</p>
    </div>"#);

    // Warm-up vs Memory Leak
    html.push_str("<h2>Warm-up vs Memory Leak Behavior</h2>\n");
    html.push_str(r#"<div class="info-box">
        <p><strong>Warm-up:</strong> When the exporter starts, memory usage grows as ringbuffers fill with data. This is normal and expected.</p>
        <p><strong>Steady State:</strong> Once ringbuffers are full, memory usage stabilizes. New data overwrites old data in a circular fashion.</p>
        <p><strong>Memory Leak:</strong> If memory continues to grow indefinitely after warm-up, that would indicate a leak (not expected behavior).</p>
    </div>"#);

    // Meaning of other:unknown
    html.push_str("<h2>Meaning of other:unknown</h2>\n");
    html.push_str(r#"<div class="info-box">
        <p>The <code>other:unknown</code> subgroup contains processes that don't match any known classification patterns. This is a catch-all category for unrecognized processes.</p>
        <p>If you see important processes in <code>other:unknown</code>, consider adding classification rules for them in your configuration.</p>
    </div>"#);

    // FAQ
    html.push_str("<h2>FAQ for Operators</h2>\n");
    html.push_str("<h3>How do I add a new subgroup?</h3>\n");
    html.push_str(r#"<p>Subgroups are defined in the exporter's built-in classification rules. To customize, modify the configuration file or source code (see <code>/config</code> for current settings).</p>"#);

    html.push_str("<h3>Why is my exporter using X MB of RAM?</h3>\n");
    html.push_str(r#"<p>Check the ringbuffer configuration. The <code>estimated_ram_bytes</code> in <code>/details</code> shows expected usage. Additional overhead comes from process metadata and cache.</p>"#);

    html.push_str("<h3>Can I export historical data from ringbuffers?</h3>\n");
    html.push_str(r#"<p>No. Ringbuffers are for in-process inspection only. Use Prometheus to scrape <code>/metrics</code> for long-term storage.</p>"#);

    html.push_str("<h3>What's the difference between /details and /metrics?</h3>\n");
    html.push_str(r#"<p><strong>/metrics:</strong> Prometheus-formatted current state for scraping by monitoring systems.</p>
    <p><strong>/details:</strong> Human-readable historical data for debugging and inspection.</p>"#);

    html.push_str(&html_footer());
    Html(html)
}
