//! Health check endpoint handler.
//!
//! This module provides the `/health` endpoint handler that returns
//! exporter health statistics and buffer status.

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use herakles_proc_mem_exporter::HealthResponse;
use std::fmt::Write as FmtWrite;
use tracing::{debug, instrument};

use crate::state::SharedState;

/// Footer text for human-readable HTTP endpoints.
pub const FOOTER_TEXT: &str = "Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io";

/// Handler for the /health endpoint.
#[instrument(skip(state))]
pub async fn health_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /health request");

    // Track HTTP request for health endpoint
    state.health_stats.record_http_request();

    let cache = state.cache.read().await;

    // Derive HTTP status from cache state
    let status = if cache.update_success && cache.last_updated.is_some() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    // Short status message for human-readable heading
    let message = if cache.is_updating {
        "OK - Cache updating"
    } else if cache.update_success {
        "OK"
    } else {
        "Cache update failed"
    };

    // Render plain-text table from HealthStats
    let table = state.health_stats.render_table();

    // Get buffer health and render it
    let buffer_health = state.health_state.get_health();
    let buffer_section = render_buffer_health(&buffer_health);

    debug!("Health check: {} - {}", status, message);
    (
        status,
        [("Content-Type", "text/plain; charset=utf-8")],
        format!("{message}\n\n{table}\n{buffer_section}\n{FOOTER_TEXT}"),
    )
}

/// Renders buffer health information as a plain-text table.
fn render_buffer_health(health: &HealthResponse) -> String {
    let mut out = String::new();
    writeln!(out, "BUFFER HEALTH").ok();
    writeln!(out, "=============").ok();
    writeln!(out).ok();
    writeln!(
        out,
        "{:25} | {:>10} | {:>12} | {:>10}",
        "Buffer", "Usage (KB)", "Capacity (KB)", "Status"
    )
    .ok();
    writeln!(out, "{}", "-".repeat(66)).ok();

    for buffer in &health.buffers {
        writeln!(
            out,
            "{:25} | {:>10} | {:>12} | {:>10}",
            buffer.name, buffer.current_kb, buffer.capacity_kb, buffer.status
        )
        .ok();
    }

    writeln!(out).ok();
    writeln!(out, "Overall Buffer Status: {}", health.overall_status).ok();
    out
}
