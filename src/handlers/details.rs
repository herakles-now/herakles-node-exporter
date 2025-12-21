//! Details endpoint handler.
//!
//! This module provides the `/details` endpoint handler that displays
//! ringbuffer statistics and historical metrics for subgroups.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::fmt::Write;
use tracing::{debug, instrument};

use crate::handlers::health::FOOTER_TEXT;
use crate::state::SharedState;

/// Query parameters for the details endpoint.
#[derive(Deserialize, Debug)]
pub struct DetailsQuery {
    pub subgroup: Option<String>,
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

    // If subgroup specified, show history
    if let Some(subgroup_name) = params.subgroup {
        if let Some(history) = state
            .ringbuffer_manager
            .get_subgroup_history(&subgroup_name)
        {
            writeln!(out, "HISTORY: {}", subgroup_name).ok();
            writeln!(out, "=================================").ok();
            writeln!(
                out,
                "{:>19} | {:>10} | {:>10} | {:>10} | {:>8} | {:>10}",
                "Timestamp", "RSS (KB)", "PSS (KB)", "USS (KB)", "CPU %", "CPU Time"
            )
            .ok();
            writeln!(out, "{}", "-".repeat(88)).ok();

            for entry in history {
                let dt = chrono::NaiveDateTime::from_timestamp_opt(entry.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| entry.timestamp.to_string());

                writeln!(
                    out,
                    "{:>19} | {:>10} | {:>10} | {:>10} | {:>7.1}% | {:>9.2}s",
                    dt,
                    entry.rss_kb,
                    entry.pss_kb,
                    entry.uss_kb,
                    entry.cpu_percent,
                    entry.cpu_time_seconds
                )
                .ok();
            }
        } else {
            writeln!(out, "Subgroup '{}' not found", subgroup_name).ok();
        }
    } else {
        // List all subgroups
        writeln!(out, "AVAILABLE SUBGROUPS").ok();
        writeln!(out, "===================").ok();
        let mut subgroups = state.ringbuffer_manager.get_all_subgroups();
        subgroups.sort();
        for sg in subgroups {
            writeln!(out, "  - {} (use ?subgroup={} to view history)", sg, sg).ok();
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
