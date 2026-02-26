//! Root endpoint handler for the landing page.
//!
//! This module provides the `/` endpoint handler that displays
//! a landing page with all available endpoints and descriptions.

use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use tracing::{debug, instrument};

use crate::handlers::health::FOOTER_TEXT;
use crate::state::SharedState;

/// Handler for the root `/` endpoint.
#[instrument(skip(state))]
pub async fn root_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing / request");
    state.health_stats.record_http_request();

    let version = env!("CARGO_PKG_VERSION");

    // Calculate actual uptime from service start time
    let uptime_secs = state.start_time.elapsed().as_secs();
    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let seconds = uptime_secs % 60;
    let uptime_str = format!("{}h {}m {}s", hours, minutes, seconds);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Herakles Node Exporter</title>
    <style>
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
            margin: 0; 
            padding: 20px; 
            background: #f5f5f5; 
            line-height: 1.6;
        }}
        .container {{ 
            max-width: 900px; 
            margin: 0 auto; 
            background: white; 
            padding: 40px; 
            border-radius: 8px; 
            box-shadow: 0 2px 8px rgba(0,0,0,0.1); 
        }}
        h1 {{ 
            color: #333; 
            border-bottom: 3px solid #007bff; 
            padding-bottom: 15px; 
            margin-bottom: 10px;
        }}
        .subtitle {{
            color: #666;
            font-size: 1.1em;
            margin-bottom: 30px;
        }}
        h2 {{ 
            color: #555; 
            margin-top: 35px; 
            margin-bottom: 15px;
        }}
        .info {{ 
            background: #e9ecef; 
            padding: 15px; 
            border-radius: 4px; 
            margin: 20px 0;
            display: flex;
            justify-content: space-around;
            flex-wrap: wrap;
        }}
        .info-item {{
            margin: 10px;
        }}
        .info-label {{ 
            font-weight: 600; 
            color: #555; 
            display: block;
            font-size: 0.9em;
        }}
        .info-value {{ 
            font-size: 1.2em; 
            color: #007bff; 
        }}
        .endpoint-list {{
            list-style: none;
            padding: 0;
        }}
        .endpoint-list li {{
            margin: 20px 0;
            padding: 15px;
            background: #f8f9fa;
            border-left: 4px solid #007bff;
            border-radius: 4px;
        }}
        .endpoint-list a {{
            color: #007bff;
            text-decoration: none;
            font-weight: 600;
            font-size: 1.1em;
        }}
        .endpoint-list a:hover {{
            text-decoration: underline;
        }}
        .endpoint-desc {{
            color: #666;
            margin-top: 5px;
        }}
        .footer {{ 
            margin-top: 40px; 
            padding-top: 20px; 
            border-top: 1px solid #ddd; 
            color: #666; 
            font-size: 0.9em; 
            text-align: center;
        }}
        code {{
            background: #e9ecef;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
        }}
    </style>
</head>
<body>
<div class="container">
    <h1>Herakles Node Exporter</h1>
    <p class="subtitle">High-performance process, subgroup and node-level metrics exporter</p>

    <div class="info">
        <div class="info-item">
            <span class="info-label">Version</span>
            <span class="info-value">{version}</span>
        </div>
        <div class="info-item">
            <span class="info-label">Uptime</span>
            <span class="info-value">{uptime}</span>
        </div>
    </div>

    <h2>Available Endpoints</h2>
    <ul class="endpoint-list">
        <li>
            <a href="/metrics">/metrics</a>
            <div class="endpoint-desc">Prometheus-compatible metrics endpoint</div>
        </li>
        <li>
            <a href="/health">/health</a>
            <div class="endpoint-desc">Exporter internal health & performance statistics (text)</div>
        </li>
        <li>
            <a href="/docs">/docs</a>
            <div class="endpoint-desc">HTML documentation about metrics & concepts</div>
        </li>
        <li>
            <a href="/html/details">/html/details</a>
            <div class="endpoint-desc">Interactive HTML view: subgroup history, ringbuffer data</div>
        </li>
        <li>
            <a href="/html/health">/html/health</a>
            <div class="endpoint-desc">HTML view of exporter health</div>
        </li>
        <li>
            <a href="/html/config">/html/config</a>
            <div class="endpoint-desc">Active runtime configuration (read-only)</div>
        </li>
        <li>
            <a href="/subgroups">/subgroups</a>
            <div class="endpoint-desc">List of detected groups & subgroups (text/json)</div>
        </li>
    </ul>

    <div class="footer">
        <p>{footer}</p>
    </div>
</div>
</body>
</html>"#,
        version = version,
        uptime = uptime_str,
        footer = FOOTER_TEXT
    );

    Html(html)
}
