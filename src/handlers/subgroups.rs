//! Subgroups endpoint handler.
//!
//! This module provides the `/subgroups` endpoint handler that displays
//! the loaded process subgroups configuration.

use ahash::AHashMap as HashMap;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::fmt::Write as FmtWrite;
use tracing::{debug, instrument};

use crate::handlers::health::FOOTER_TEXT;
use crate::process::SUBGROUPS;
use crate::state::SharedState;

/// Handler for the /subgroups endpoint.
#[instrument(skip(state))]
pub async fn subgroups_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /subgroups request");

    // Track HTTP request
    state.health_stats.record_http_request();

    // Collect unique (group, subgroup) pairs with their associated process name matches
    let mut subgroup_data: HashMap<(String, String), Vec<String>> = HashMap::new();

    for (process_name, (group, subgroup)) in SUBGROUPS.iter() {
        let key = (group.to_string(), subgroup.to_string());
        subgroup_data
            .entry(key)
            .or_default()
            .push(process_name.to_string());
    }

    // Sort by group then subgroup for consistent output
    let mut sorted_entries: Vec<_> = subgroup_data.into_iter().collect();
    sorted_entries.sort_by(|a, b| {
        let group_cmp = a.0 .0.cmp(&b.0 .0);
        if group_cmp == std::cmp::Ordering::Equal {
            a.0 .1.cmp(&b.0 .1)
        } else {
            group_cmp
        }
    });

    // Count unique subgroups
    let unique_subgroups_count = sorted_entries.len();

    let mut out = String::new();

    writeln!(out, "HERAKLES PROC MEM EXPORTER - SUBGROUPS").ok();
    writeln!(out, "======================================").ok();
    writeln!(out).ok();
    writeln!(
        out,
        "Total patterns: {} | Unique subgroups: {}",
        SUBGROUPS.len(),
        unique_subgroups_count
    )
    .ok();
    writeln!(out).ok();

    // Group entries by group name for better readability
    let mut current_group: Option<String> = None;
    for ((group, subgroup), mut matches) in sorted_entries {
        // Print group header when group changes
        if current_group.as_ref() != Some(&group) {
            if current_group.is_some() {
                writeln!(out).ok();
            }
            writeln!(out, "GROUP: {}", group).ok();
            writeln!(out, "{}", "-".repeat(40)).ok();
            current_group = Some(group.clone());
        }

        matches.sort();
        let matches_str = matches.join(", ");
        writeln!(out, "  {:<20} -> {}", subgroup, matches_str).ok();
    }

    writeln!(out).ok();
    writeln!(out, "{FOOTER_TEXT}").ok();

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        out,
    )
}
