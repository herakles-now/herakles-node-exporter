//! HTML endpoint handlers for human-friendly inspection and debugging.
//!
//! This module provides HTML views for the existing /details data,
//! using only in-memory data structures. No new calculations or state changes.

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use std::sync::atomic::Ordering;
use tracing::{debug, instrument};

use crate::cache::ProcMem;
use crate::handlers::health::FOOTER_TEXT;
use crate::process::classify_process_raw;
use crate::state::SharedState;

/// CPU percentage scaling factor (must match the constant in main.rs).
const CPU_SCALE_FACTOR: f32 = 1000.0;

/// CPU heatmap thresholds
const CPU_CRITICAL_THRESHOLD: f32 = 80.0;
const CPU_HIGH_THRESHOLD: f32 = 50.0;
const CPU_MEDIUM_THRESHOLD: f32 = 20.0;

/// I/O rates calculated from process deltas.
#[derive(Debug, Clone, Copy)]
struct IoRates {
    read_bytes_per_sec: f64,
    write_bytes_per_sec: f64,
    rx_bytes_per_sec: f64,
    tx_bytes_per_sec: f64,
}

/// Calculate I/O rates from process metrics.
fn calculate_io_rates(proc: &ProcMem, current_time: f64) -> IoRates {
    let time_delta = current_time - proc.last_update_time;

    // Handle edge cases: no previous data or invalid time delta
    if time_delta <= 0.0 || proc.last_update_time == 0.0 {
        return IoRates {
            read_bytes_per_sec: 0.0,
            write_bytes_per_sec: 0.0,
            rx_bytes_per_sec: 0.0,
            tx_bytes_per_sec: 0.0,
        };
    }

    // Calculate deltas (handle counter wraps with saturating_sub)
    let read_delta = proc.read_bytes.saturating_sub(proc.last_read_bytes);
    let write_delta = proc.write_bytes.saturating_sub(proc.last_write_bytes);
    let rx_delta = proc.rx_bytes.saturating_sub(proc.last_rx_bytes);
    let tx_delta = proc.tx_bytes.saturating_sub(proc.last_tx_bytes);

    // Calculate rates (bytes per second)
    let read_rate = read_delta as f64 / time_delta;
    let write_rate = write_delta as f64 / time_delta;
    let rx_rate = rx_delta as f64 / time_delta;
    let tx_rate = tx_delta as f64 / time_delta;

    IoRates {
        read_bytes_per_sec: read_rate,
        write_bytes_per_sec: write_rate,
        rx_bytes_per_sec: rx_rate,
        tx_bytes_per_sec: tx_rate,
    }
}

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
        h4 {{ color: #777; margin-top: 20px; }}
        h5 {{ color: #888; }}
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
        
        /* Collapsible sections styling */
        details {{ 
            border: 1px solid #ddd; 
            border-radius: 4px; 
            margin: 10px 0; 
            padding: 0;
            background: #f9f9f9; 
        }}
        summary {{ 
            cursor: pointer; 
            font-weight: 600; 
            padding: 15px; 
            background: #007bff; 
            color: white; 
            border-radius: 4px;
            user-select: none;
        }}
        summary:hover {{ 
            background: #0056b3; 
        }}
        .subgroup-content {{ 
            padding: 20px; 
            margin-top: 0;
            background: white;
        }}
        
        /* Sortable process table styling */
        .sortable-process-table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }}
        
        .sortable-process-table th {{
            background: #007bff;
            color: white;
            padding: 12px;
            text-align: left;
            cursor: pointer;
            user-select: none;
            position: relative;
            font-weight: 600;
        }}
        
        .sortable-process-table th:hover {{
            background: #0056b3;
        }}
        
        .sortable-process-table th.sorted-desc::after {{
            content: ' ▼';
            position: absolute;
            right: 8px;
        }}
        
        .sortable-process-table th.sorted-asc::after {{
            content: ' ▲';
            position: absolute;
            right: 8px;
        }}
        
        .sortable-process-table td {{
            padding: 10px;
            border-bottom: 1px solid #ddd;
        }}
        
        .sortable-process-table tr:hover {{
            background: #f8f9fa;
        }}
        
        .rank {{
            font-size: 1.2em;
            text-align: center;
            width: 40px;
        }}
        
        /* CPU heatmap colors */
        .cpu-critical {{
            background: #ff4444 !important;
            color: white !important;
            font-weight: bold !important;
        }}
        
        .cpu-high {{
            background: #ffaa44 !important;
        }}
        
        .cpu-medium {{
            background: #ffff88 !important;
        }}
        
        .cpu-low {{
            background: transparent;
        }}
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

/// Render interactive HTML table for a specific subgroup.
async fn render_interactive_table(state: SharedState, subgroup_name: &str) -> Html<String> {
    use chrono::{Local, TimeZone};

    let cache = state.cache.read().await;
    let current_timestamp = chrono::Utc::now().timestamp();

    // Parse subgroup_name once (format: "group:subgroup")
    let subgroup_parts: Vec<&str> = subgroup_name.split(':').collect();
    if subgroup_parts.len() != 2 {
        return Html(format!(
            r#"<!DOCTYPE html><html><body><h1>Error</h1><p>Invalid subgroup format. Expected "group:subgroup"</p></body></html>"#
        ));
    }
    let expected_group = subgroup_parts[0];
    let expected_subgroup = subgroup_parts[1];

    // Collect all processes for the subgroup
    let mut processes: Vec<&ProcMem> = Vec::new();
    for proc in cache.processes.values() {
        let (group, subgroup) = classify_process_raw(&proc.name);

        if group.as_ref() == expected_group && subgroup.as_ref() == expected_subgroup {
            processes.push(proc);
        }
    }

    // Sort by CPU descending (default)
    processes.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Generate HTML
    let mut html = String::new();

    html.push_str(&format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Details: {}</title>
  <style>
    body {{
      font-family: 'Segoe UI', Tahoma, sans-serif;
      background: #f5f5f5;
      padding: 20px;
      margin: 0;
    }}
    .container {{
      max-width: 1600px;
      margin: 0 auto;
      background: white;
      padding: 30px;
      border-radius: 8px;
      box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    }}
    h1 {{
      color: #333;
      border-bottom: 3px solid #007bff;
      padding-bottom: 10px;
      margin-bottom: 20px;
    }}
    .auto-refresh {{
      float: right;
      font-size: 0.9em;
      color: #666;
      font-weight: normal;
    }}
    #searchBox {{
      width: 300px;
      padding: 10px;
      margin-bottom: 15px;
      border: 1px solid #ddd;
      border-radius: 4px;
      font-size: 14px;
    }}
    #processTable {{
      width: 100%;
      border-collapse: collapse;
      margin-top: 20px;
    }}
    #processTable th {{
      background: #007bff;
      color: white;
      padding: 12px;
      text-align: left;
      cursor: pointer;
      user-select: none;
      position: relative;
      font-weight: 600;
    }}
    #processTable th:hover {{
      background: #0056b3;
    }}
    #processTable th.sorted-desc::after {{
      content: ' ▼';
      position: absolute;
      right: 8px;
    }}
    #processTable th.sorted-asc::after {{
      content: ' ▲';
      position: absolute;
      right: 8px;
    }}
    #processTable td {{
      padding: 10px;
      border-bottom: 1px solid #ddd;
    }}
    #processTable tr:hover {{
      background: #f8f9fa;
    }}
    .cpu-critical {{
      background: #ff4444 !important;
      color: white !important;
      font-weight: bold !important;
    }}
    .cpu-high {{
      background: #ffaa44 !important;
    }}
    .cpu-medium {{
      background: #ffff88 !important;
    }}
    .rank {{
      font-size: 1.2em;
      text-align: center;
      width: 50px;
    }}
    .back-link {{
      display: inline-block;
      margin-bottom: 15px;
      color: #007bff;
      text-decoration: none;
    }}
    .back-link:hover {{
      text-decoration: underline;
    }}
  </style>
</head>
<body>
  <div class="container">
    <a href="/html/details" class="back-link">← Back to All Subgroups</a>
    
    <h1>
      SUBGROUP: {}
      <span class="auto-refresh">Auto-refresh: 30s</span>
    </h1>
    
    <input type="text" id="searchBox" placeholder="Filter by name or PID...">
    
    <table id="processTable">
      <thead>
        <tr>
          <th data-column="rank">#</th>
          <th data-column="pid" onclick="sortTable('pid')">PID</th>
          <th data-column="name" onclick="sortTable('name')">Name</th>
          <th data-column="timestamp" onclick="sortTable('timestamp')">Timestamp</th>
          <th data-column="cpu" onclick="sortTable('cpu')" class="sorted-desc">CPU%</th>
          <th data-column="rss" onclick="sortTable('rss')">RSS</th>
          <th data-column="pss" onclick="sortTable('pss')">PSS</th>
          <th data-column="uss" onclick="sortTable('uss')">USS</th>
          <th data-column="blkio" onclick="sortTable('blkio')">Block IO</th>
          <th data-column="netio" onclick="sortTable('netio')">Net IO</th>
        </tr>
      </thead>
      <tbody>
"#,
        subgroup_name, subgroup_name
    ));

    // Add process rows
    for proc in processes {
        let cpu_class = if proc.cpu_percent > 80.0 {
            "cpu-critical"
        } else if proc.cpu_percent > 50.0 {
            "cpu-high"
        } else if proc.cpu_percent > 20.0 {
            "cpu-medium"
        } else {
            ""
        };

        let timestamp_str = Local
            .timestamp_opt(current_timestamp, 0)
            .single()
            .map(|dt| dt.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| {
                // Timestamp conversion failed - use Unix timestamp as fallback
                format!("{}", current_timestamp)
            });

        let rss_mb = proc.rss as f64 / (1024.0 * 1024.0);
        let pss_mb = proc.pss as f64 / (1024.0 * 1024.0);
        let uss_mb = proc.uss as f64 / (1024.0 * 1024.0);

        // Calculate Block I/O rate (bytes per second)
        // NOTE: Set to 0.0 as proper implementation requires delta calculation between
        // consecutive scrapes (current_io - previous_io) / time_delta. This would need
        // historical tracking in the cache or ringbuffer to calculate the rate accurately.
        let blkio_mb_s = 0.0;

        // Get Network I/O rate from eBPF if available
        let netio_mb_s = if let Some(ref ebpf_manager) = state.ebpf {
            if let Ok(net_stats) = ebpf_manager.read_process_net_stats() {
                net_stats
                    .iter()
                    .find(|s| s.pid == proc.pid)
                    .map(|s| (s.rx_bytes + s.tx_bytes) as f64 / (1024.0 * 1024.0))
                    .unwrap_or(0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        html.push_str(&format!(r#"
        <tr data-cpu="{}" data-rss="{}" data-pss="{}" data-uss="{}" data-blkio="{}" data-netio="{}" data-pid="{}" data-timestamp="{}">
          <td class="rank"></td>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td class="{}">{:.1}%</td>
          <td>{:.1} MB</td>
          <td>{:.1} MB</td>
          <td>{:.1} MB</td>
          <td>{:.2} MB/s</td>
          <td>{:.2} MB/s</td>
        </tr>
"#, 
            proc.cpu_percent, rss_mb, pss_mb, uss_mb, blkio_mb_s, netio_mb_s, proc.pid, current_timestamp,
            proc.pid, proc.name, timestamp_str, cpu_class, proc.cpu_percent,
            rss_mb, pss_mb, uss_mb, blkio_mb_s, netio_mb_s
        ));
    }

    html.push_str(r#"
      </tbody>
    </table>
  </div>
  
  <script>
    let sortConfig = { column: 'cpu', direction: 'desc' };
    
    function sortTable(column) {
      const table = document.getElementById('processTable');
      const tbody = table.querySelector('tbody');
      const rows = Array.from(tbody.querySelectorAll('tr'));
      
      if (sortConfig.column === column) {
        sortConfig.direction = sortConfig.direction === 'desc' ? 'asc' : 'desc';
      } else {
        sortConfig.column = column;
        sortConfig.direction = 'desc';
      }
      
      rows.sort((a, b) => {
        let aVal, bVal;
        
        if (column === 'rank') {
          return 0; // Don't sort rank column
        } else if (column === 'pid') {
          aVal = parseInt(a.dataset.pid);
          bVal = parseInt(b.dataset.pid);
        } else if (column === 'name') {
          aVal = a.cells[2].textContent;
          bVal = b.cells[2].textContent;
        } else if (column === 'timestamp') {
          aVal = parseInt(a.dataset.timestamp);
          bVal = parseInt(b.dataset.timestamp);
        } else {
          aVal = parseFloat(a.dataset[column] || 0);
          bVal = parseFloat(b.dataset[column] || 0);
        }
        
        if (typeof aVal === 'string') {
          return sortConfig.direction === 'desc' ? bVal.localeCompare(aVal) : aVal.localeCompare(bVal);
        }
        
        return sortConfig.direction === 'desc' ? bVal - aVal : aVal - bVal;
      });
      
      rows.forEach(row => tbody.appendChild(row));
      updateRankBadges();
      
      document.querySelectorAll('th').forEach(th => {
        th.classList.remove('sorted-asc', 'sorted-desc');
      });
      const clickedTh = document.querySelector(`th[data-column="${column}"]`);
      if (clickedTh) {
        clickedTh.classList.add(`sorted-${sortConfig.direction}`);
      }
    }
    
    function updateRankBadges() {
      document.querySelectorAll('td.rank').forEach(td => td.textContent = '');
      const rows = Array.from(document.querySelectorAll('#processTable tbody tr')).filter(row => row.style.display !== 'none');
      const badges = ['🥇', '🥈', '🥉'];
      for (let i = 0; i < 3 && i < rows.length; i++) {
        rows[i].querySelector('td.rank').textContent = badges[i];
      }
    }
    
    document.getElementById('searchBox').addEventListener('input', function(e) {
      const query = e.target.value.toLowerCase();
      const rows = document.querySelectorAll('#processTable tbody tr');
      
      rows.forEach(row => {
        const text = row.textContent.toLowerCase();
        row.style.display = text.includes(query) ? '' : 'none';
      });
      
      updateRankBadges();
    });
    
    setInterval(() => {
      location.reload();
    }, 30000);
    
    // Initialize on page load
    updateRankBadges();
  </script>
</body>
</html>
"#);

    Html(html)
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
    html.push_str(
        r#"<li><a href="/html/subgroups">Subgroups</a> - All subgroups with current metrics</li>"#,
    );
    html.push_str(
        r#"<li><a href="/html/health">Health</a> - Exporter health and buffer status</li>"#,
    );
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

    // Check if subgroup parameter is provided for interactive table view
    if let Some(ref subgroup_name) = params.subgroup {
        return render_interactive_table(state, subgroup_name).await;
    }

    let cache = state.cache.read().await;
    let stats = state.ringbuffer_manager.get_stats();

    let mut html = html_header("Details");
    html.push_str("<h1>Details - All Subgroups</h1>\n");

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

    // Add expand/collapse all buttons
    html.push_str(r#"
<div style="margin: 20px 0;">
    <button onclick="expandAll()" style="padding: 10px 20px; margin-right: 10px; cursor: pointer; background: #007bff; color: white; border: none; border-radius: 4px;">Expand All</button>
    <button onclick="collapseAll()" style="padding: 10px 20px; cursor: pointer; background: #6c757d; color: white; border: none; border-radius: 4px;">Collapse All</button>
</div>
<script>
function expandAll() {
    document.querySelectorAll('details').forEach(d => d.open = true);
}
function collapseAll() {
    document.querySelectorAll('details').forEach(d => d.open = false);
}
</script>
"#);

    // Get all subgroups and sort them
    let mut subgroups = state.ringbuffer_manager.get_all_subgroups();
    subgroups.sort();

    html.push_str("<h2>All Subgroups</h2>\n");
    html.push_str(&format!(
        "<p>Showing {} subgroups. Click to expand/collapse details.</p>\n",
        subgroups.len()
    ));

    // Render each subgroup in a collapsible section
    for subgroup_name in subgroups {
        // Get history for this subgroup
        let history = state
            .ringbuffer_manager
            .get_subgroup_history(&subgroup_name);

        // Calculate current aggregated values from cache
        let mut subgroup_processes: Vec<&ProcMem> = Vec::new();
        for proc in cache.processes.values() {
            let (_, sg) = classify_process_raw(&proc.name);
            let key = format!("{}:{}", classify_process_raw(&proc.name).0, sg);
            if key == subgroup_name {
                subgroup_processes.push(proc);
            }
        }

        // Start collapsible section
        html.push_str("<details>\n");
        html.push_str(&format!("<summary>{}</summary>\n", subgroup_name));
        html.push_str(r#"<div class="subgroup-content">"#);
        html.push_str("\n");

        if !subgroup_processes.is_empty() {
            html.push_str("<h3>Current Live Snapshot</h3>\n");

            let total_rss: u64 = subgroup_processes.iter().map(|p| p.rss).sum();
            let total_pss: u64 = subgroup_processes.iter().map(|p| p.pss).sum();
            let total_uss: u64 = subgroup_processes.iter().map(|p| p.uss).sum();
            let total_cpu: f64 = subgroup_processes
                .iter()
                .map(|p| p.cpu_percent as f64)
                .sum();

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
                "<tr><td>Oldest Process Start</td><td>{:.2}s since boot</td></tr>\n",
                oldest_uptime
            ));
            html.push_str("</table>\n");

            // Add sortable table showing ALL processes
            let current_timestamp = chrono::Utc::now().timestamp();

            html.push_str(&format!(
                "<h3>All Processes ({} total) - Click column to sort</h3>\n",
                subgroup_processes.len()
            ));

            // Create safe table ID from subgroup_name
            let table_id = subgroup_name.replace(":", "-");

            html.push_str(&format!(
                r#"<table id="table-{}" class="sortable-process-table">"#,
                table_id
            ));
            html.push_str("\n");

            // Table header
            html.push_str("<thead>\n<tr>\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'rank')">#</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'pid')">PID</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'name')">Name</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'timestamp')">Timestamp</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'cpu')" class="sorted-desc">CPU%</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'rss')">RSS</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'pss')">PSS</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'uss')">USS</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'blkio-read')">Blk Read</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'blkio-write')">Blk Write</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'net-rx')">Net RX</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str(&format!(
                r#"  <th onclick="sortSubgroupTable('{}', 'net-tx')">Net TX</th>"#,
                table_id
            ));
            html.push_str("\n");
            html.push_str("</tr>\n</thead>\n");

            // Table body with all processes
            html.push_str("<tbody>\n");

            // Sort by CPU descending (default)
            let mut sorted_procs = subgroup_processes.clone();
            sorted_procs.sort_by(|a, b| {
                b.cpu_percent
                    .partial_cmp(&a.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Get current time for rate calculations
            let current_time = chrono::Utc::now().timestamp() as f64;

            for proc in sorted_procs {
                // Determine CPU heatmap class
                let cpu_class = if proc.cpu_percent > CPU_CRITICAL_THRESHOLD {
                    "cpu-critical"
                } else if proc.cpu_percent > CPU_HIGH_THRESHOLD {
                    "cpu-high"
                } else if proc.cpu_percent > CPU_MEDIUM_THRESHOLD {
                    "cpu-medium"
                } else {
                    "cpu-low"
                };

                // Format timestamp as HH:MM:SS
                let timestamp_str = {
                    use chrono::{Local, TimeZone};
                    Local
                        .timestamp_opt(current_timestamp, 0)
                        .single()
                        .map(|dt| dt.format("%H:%M:%S").to_string())
                        .unwrap_or_else(|| format!("{}", current_timestamp))
                };

                // Calculate I/O rates using the helper function
                let rates = calculate_io_rates(proc, current_time);

                // Convert to KB for data attributes (to avoid precision issues)
                let rss_kb = proc.rss / 1024;
                let pss_kb = proc.pss / 1024;
                let uss_kb = proc.uss / 1024;

                // Convert rates to KB/s for data attributes
                let blkio_read_kb_s = (rates.read_bytes_per_sec / 1024.0) as u64;
                let blkio_write_kb_s = (rates.write_bytes_per_sec / 1024.0) as u64;
                let net_rx_kb_s = (rates.rx_bytes_per_sec / 1024.0) as u64;
                let net_tx_kb_s = (rates.tx_bytes_per_sec / 1024.0) as u64;

                // Write table row with data attributes for sorting
                html.push_str(&format!(
                    r#"<tr data-cpu="{}" data-rss="{}" data-pss="{}" data-uss="{}" data-blkio-read="{}" data-blkio-write="{}" data-net-rx="{}" data-net-tx="{}" data-pid="{}" data-timestamp="{}" data-name="{}">"#,
                    proc.cpu_percent,
                    rss_kb,
                    pss_kb,
                    uss_kb,
                    blkio_read_kb_s,
                    blkio_write_kb_s,
                    net_rx_kb_s,
                    net_tx_kb_s,
                    proc.pid,
                    current_timestamp,
                    proc.name
                ));
                html.push_str("\n");

                // Rank column (will be populated by JavaScript)
                html.push_str(r#"  <td class="rank"></td>"#);
                html.push_str("\n");

                // PID
                html.push_str(&format!("  <td>{}</td>\n", proc.pid));

                // Name
                html.push_str(&format!("  <td>{}</td>\n", proc.name));

                // Timestamp
                html.push_str(&format!("  <td>{}</td>\n", timestamp_str));

                // CPU with heatmap class
                html.push_str(&format!(
                    r#"  <td class="{}">{:.2}%</td>"#,
                    cpu_class, proc.cpu_percent
                ));
                html.push_str("\n");

                // RSS
                html.push_str(&format!(
                    "  <td>{:.2} MB</td>\n",
                    proc.rss as f64 / (1024.0 * 1024.0)
                ));

                // PSS
                html.push_str(&format!(
                    "  <td>{:.2} MB</td>\n",
                    proc.pss as f64 / (1024.0 * 1024.0)
                ));

                // USS
                html.push_str(&format!(
                    "  <td>{:.2} MB</td>\n",
                    proc.uss as f64 / (1024.0 * 1024.0)
                ));

                // Block Read
                html.push_str(&format!(
                    "  <td>{:.2} MB/s</td>\n",
                    rates.read_bytes_per_sec / (1024.0 * 1024.0)
                ));

                // Block Write
                html.push_str(&format!(
                    "  <td>{:.2} MB/s</td>\n",
                    rates.write_bytes_per_sec / (1024.0 * 1024.0)
                ));

                // Net RX
                html.push_str(&format!(
                    "  <td>{:.2} MB/s</td>\n",
                    rates.rx_bytes_per_sec / (1024.0 * 1024.0)
                ));

                // Net TX
                html.push_str(&format!(
                    "  <td>{:.2} MB/s</td>\n",
                    rates.tx_bytes_per_sec / (1024.0 * 1024.0)
                ));

                html.push_str("</tr>\n");
            }

            html.push_str("</tbody>\n");
            html.push_str("</table>\n");
        } else {
            html.push_str("<p><em>No processes currently in this subgroup.</em></p>\n");
        }

        // Show ringbuffer history with top-N data
        if let Some(history) = history {
            if !history.is_empty() {
                html.push_str("<h3>Historical Ringbuffer Data</h3>\n");
                html.push_str(&format!(
                    "<p>Showing {} historical entries.</p>\n",
                    history.len()
                ));

                // Calculate averages
                let avg_rss = history.iter().map(|e| e.rss_kb).sum::<u64>() / history.len() as u64;
                let avg_pss = history.iter().map(|e| e.pss_kb).sum::<u64>() / history.len() as u64;
                let avg_uss = history.iter().map(|e| e.uss_kb).sum::<u64>() / history.len() as u64;
                let avg_cpu =
                    history.iter().map(|e| e.cpu_percent).sum::<f32>() / history.len() as f32;

                html.push_str("<table>\n");
                html.push_str("<tr><th>Metric</th><th>Average</th><th>Latest</th></tr>\n");
                let latest = &history[history.len() - 1];
                html.push_str(&format!(
                    "<tr><td>RSS</td><td>{} KB</td><td>{} KB</td></tr>\n",
                    avg_rss, latest.rss_kb
                ));
                html.push_str(&format!(
                    "<tr><td>PSS</td><td>{} KB</td><td>{} KB</td></tr>\n",
                    avg_pss, latest.pss_kb
                ));
                html.push_str(&format!(
                    "<tr><td>USS</td><td>{} KB</td><td>{} KB</td></tr>\n",
                    avg_uss, latest.uss_kb
                ));
                html.push_str(&format!(
                    "<tr><td>CPU %</td><td>{:.1}%</td><td>{:.1}%</td></tr>\n",
                    avg_cpu, latest.cpu_percent
                ));
                html.push_str("</table>\n");

                // Show top-N from latest historical entry
                html.push_str("<h4>Historical Top-3 (Latest Entry)</h4>\n");

                // Top-3 by CPU from history
                html.push_str("<h5>By CPU Usage</h5>\n");
                html.push_str("<table>\n");
                html.push_str(
                    "<tr><th>Rank</th><th>PID</th><th>Name</th><th>CPU (scaled)</th></tr>\n",
                );
                for (rank, top) in latest.top_cpu.iter().enumerate() {
                    if top.pid != 0 {
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}%</td></tr>\n",
                            rank + 1,
                            top.pid,
                            top.name_str(),
                            top.value as f32 / CPU_SCALE_FACTOR
                        ));
                    }
                }
                html.push_str("</table>\n");

                // Top-3 by RSS from history
                html.push_str("<h5>By Memory (RSS)</h5>\n");
                html.push_str("<table>\n");
                html.push_str("<tr><th>Rank</th><th>PID</th><th>Name</th><th>RSS</th></tr>\n");
                for (rank, top) in latest.top_rss.iter().enumerate() {
                    if top.pid != 0 {
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{} KB</td></tr>\n",
                            rank + 1,
                            top.pid,
                            top.name_str(),
                            top.value
                        ));
                    }
                }
                html.push_str("</table>\n");

                // Top-3 by PSS from history
                html.push_str("<h5>By Memory (PSS)</h5>\n");
                html.push_str("<table>\n");
                html.push_str("<tr><th>Rank</th><th>PID</th><th>Name</th><th>PSS</th></tr>\n");
                for (rank, top) in latest.top_pss.iter().enumerate() {
                    if top.pid != 0 {
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{} KB</td></tr>\n",
                            rank + 1,
                            top.pid,
                            top.name_str(),
                            top.value
                        ));
                    }
                }
                html.push_str("</table>\n");
            }
        }

        html.push_str("</div>\n");
        html.push_str("</details>\n");
    }

    // Add JavaScript for table sorting
    html.push_str(
        r#"
<script>
// Track sort state per table
let sortStates = {};

function sortSubgroupTable(subgroupId, column) {
  const tableId = 'table-' + subgroupId;
  const table = document.getElementById(tableId);
  if (!table) return;
  
  const tbody = table.querySelector('tbody');
  const rows = Array.from(tbody.querySelectorAll('tr'));
  
  // Initialize sort state
  if (!sortStates[subgroupId]) {
    sortStates[subgroupId] = { column: column, direction: 'desc' };
  }
  
  // Toggle direction if same column
  if (sortStates[subgroupId].column === column) {
    sortStates[subgroupId].direction = 
      sortStates[subgroupId].direction === 'desc' ? 'asc' : 'desc';
  } else {
    sortStates[subgroupId].column = column;
    sortStates[subgroupId].direction = 'desc';
  }
  
  const state = sortStates[subgroupId];
  
  // Don't sort rank column
  if (column === 'rank') {
    return;
  }
  
  // Sort rows
  rows.sort((a, b) => {
    let aVal, bVal;
    
    if (column === 'pid') {
      aVal = parseInt(a.dataset.pid);
      bVal = parseInt(b.dataset.pid);
    } else if (column === 'name') {
      aVal = a.dataset.name;
      bVal = b.dataset.name;
    } else if (column === 'timestamp') {
      aVal = parseInt(a.dataset.timestamp);
      bVal = parseInt(b.dataset.timestamp);
    } else {
      // Convert hyphenated column names to camelCase for dataset access
      // e.g., 'blkio-read' -> 'blkioRead'
      const datasetKey = column.replace(/-([a-z])/g, (g) => g[1].toUpperCase());
      aVal = parseFloat(a.dataset[datasetKey] || 0);
      bVal = parseFloat(b.dataset[datasetKey] || 0);
    }
    
    if (typeof aVal === 'string') {
      return state.direction === 'desc' 
        ? bVal.localeCompare(aVal) 
        : aVal.localeCompare(bVal);
    }
    
    return state.direction === 'desc' ? bVal - aVal : aVal - bVal;
  });
  
  // Update DOM
  rows.forEach(row => tbody.appendChild(row));
  
  // Update rank badges
  updateRankBadges(tableId);
  
  // Update header indicators
  const headers = table.querySelectorAll('th');
  headers.forEach(th => {
    th.classList.remove('sorted-asc', 'sorted-desc');
  });
  
  const columnIndex = getColumnIndex(column);
  if (columnIndex >= 0) {
    headers[columnIndex].classList.add('sorted-' + state.direction);
  }
}

function updateRankBadges(tableId) {
  const table = document.getElementById(tableId);
  const rows = table.querySelectorAll('tbody tr');
  const badges = ['🥇', '🥈', '🥉'];
  
  // Clear all badges
  rows.forEach(row => {
    row.querySelector('.rank').textContent = '';
  });
  
  // Add badges to top 3
  for (let i = 0; i < 3 && i < rows.length; i++) {
    rows[i].querySelector('.rank').textContent = badges[i];
  }
}

function getColumnIndex(column) {
  const map = {
    'rank': 0, 'pid': 1, 'name': 2, 'timestamp': 3,
    'cpu': 4, 'rss': 5, 'pss': 6, 'uss': 7,
    'blkio-read': 8, 'blkio-write': 9, 'net-rx': 10, 'net-tx': 11
  };
  return map[column] || -1;
}

// Initialize: sort all tables by CPU on page load and add badges
document.addEventListener('DOMContentLoaded', function() {
  document.querySelectorAll('.sortable-process-table').forEach(table => {
    const subgroupId = table.id.replace('table-', '');
    // Initialize rank badges for default CPU sort
    updateRankBadges(table.id);
  });
});
</script>
"#,
    );

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
    html.push_str(
        "<p>All active subgroups with current metrics. Click column headers to sort.</p>\n",
    );

    html.push_str(
        r#"<div style="margin: 20px 0;">
        <a href="/html/subgroups">Alphabetical</a> | 
        <a href="/html/subgroups?sort=rss">Sort by RSS</a> | 
        <a href="/html/subgroups?sort=cpu">Sort by CPU</a>
    </div>"#,
    );

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
    html.push_str(
        "<tr><th>Buffer</th><th>Usage (KB)</th><th>Capacity (KB)</th><th>Status</th></tr>\n",
    );

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
        cfg.cache_ttl.unwrap_or(crate::config::DEFAULT_CACHE_TTL)
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
