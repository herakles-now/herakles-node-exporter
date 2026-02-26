//! Configuration display endpoint handler.
//!
//! This module provides the `/config` endpoint handler that displays
//! the current exporter configuration.

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use std::fmt::Write as FmtWrite;
use tracing::{debug, instrument};

use crate::config::{DEFAULT_BIND_ADDR, DEFAULT_CACHE_TTL, DEFAULT_PORT};
use crate::handlers::health::FOOTER_TEXT;
use crate::state::SharedState;

/// Handler for the /config endpoint.
#[instrument(skip(state))]
pub async fn config_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /config request");

    // Track HTTP request
    state.health_stats.record_http_request();

    let cfg = &state.config;

    let mut out = String::new();

    writeln!(out, "HERAKLES PROC MEM EXPORTER - CONFIGURATION").ok();
    writeln!(out, "==========================================").ok();
    writeln!(out).ok();

    writeln!(out, "SERVER CONFIGURATION").ok();
    writeln!(out, "--------------------").ok();
    writeln!(
        out,
        "bind:                       {}",
        cfg.bind.as_deref().unwrap_or(DEFAULT_BIND_ADDR)
    )
    .ok();
    writeln!(
        out,
        "port:                       {}",
        cfg.port.unwrap_or(DEFAULT_PORT)
    )
    .ok();
    writeln!(
        out,
        "cache_ttl:                  {} seconds",
        cfg.cache_ttl.unwrap_or(DEFAULT_CACHE_TTL)
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "TLS/SSL CONFIGURATION").ok();
    writeln!(out, "---------------------").ok();
    writeln!(
        out,
        "enable_tls:                 {}",
        cfg.enable_tls.unwrap_or(false)
    )
    .ok();
    writeln!(
        out,
        "tls_cert_path:              {}",
        cfg.tls_cert_path.as_deref().unwrap_or("none")
    )
    .ok();
    writeln!(
        out,
        "tls_key_path:               {}",
        cfg.tls_key_path.as_deref().unwrap_or("none")
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "METRICS COLLECTION").ok();
    writeln!(out, "------------------").ok();
    writeln!(
        out,
        "min_uss_kb:                 {}",
        cfg.min_uss_kb.unwrap_or(0)
    )
    .ok();
    writeln!(
        out,
        "include_names:              {}",
        cfg.include_names
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "none".to_string())
    )
    .ok();
    writeln!(
        out,
        "exclude_names:              {}",
        cfg.exclude_names
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "none".to_string())
    )
    .ok();
    writeln!(
        out,
        "parallelism:                {}",
        cfg.parallelism
            .map(|v| v.to_string())
            .unwrap_or_else(|| "auto".to_string())
    )
    .ok();
    writeln!(
        out,
        "max_processes:              {}",
        cfg.max_processes
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unlimited".to_string())
    )
    .ok();
    writeln!(
        out,
        "top_n_subgroup:             {}",
        cfg.top_n_subgroup.unwrap_or(3)
    )
    .ok();
    writeln!(
        out,
        "top_n_others:               {}",
        cfg.top_n_others.unwrap_or(10)
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "PERFORMANCE TUNING").ok();
    writeln!(out, "------------------").ok();
    writeln!(
        out,
        "io_buffer_kb:               {}",
        cfg.io_buffer_kb.unwrap_or(256)
    )
    .ok();
    writeln!(
        out,
        "smaps_buffer_kb:            {}",
        cfg.smaps_buffer_kb.unwrap_or(512)
    )
    .ok();
    writeln!(
        out,
        "smaps_rollup_buffer_kb:     {}",
        cfg.smaps_rollup_buffer_kb.unwrap_or(256)
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "FEATURE FLAGS").ok();
    writeln!(out, "-------------").ok();
    writeln!(
        out,
        "enable_health:              {}",
        cfg.enable_health.unwrap_or(true)
    )
    .ok();
    writeln!(
        out,
        "enable_telemetry:           {}",
        cfg.enable_telemetry.unwrap_or(true)
    )
    .ok();
    writeln!(
        out,
        "enable_default_collectors:  {}",
        cfg.enable_default_collectors.unwrap_or(true)
    )
    .ok();
    writeln!(
        out,
        "enable_pprof:               {}",
        cfg.enable_pprof.unwrap_or(false)
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "METRICS FLAGS").ok();
    writeln!(out, "-------------").ok();
    writeln!(
        out,
        "enable_rss:                 {}",
        cfg.enable_rss.unwrap_or(true)
    )
    .ok();
    writeln!(
        out,
        "enable_pss:                 {}",
        cfg.enable_pss.unwrap_or(true)
    )
    .ok();
    writeln!(
        out,
        "enable_uss:                 {}",
        cfg.enable_uss.unwrap_or(true)
    )
    .ok();
    writeln!(
        out,
        "enable_cpu:                 {}",
        cfg.enable_cpu.unwrap_or(true)
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "CLASSIFICATION").ok();
    writeln!(out, "--------------").ok();
    writeln!(
        out,
        "search_mode:                {}",
        cfg.search_mode.as_deref().unwrap_or("none")
    )
    .ok();
    writeln!(
        out,
        "search_groups:              {}",
        cfg.search_groups
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "none".to_string())
    )
    .ok();
    writeln!(
        out,
        "search_subgroups:           {}",
        cfg.search_subgroups
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "none".to_string())
    )
    .ok();
    writeln!(
        out,
        "disable_others:             {}",
        cfg.disable_others.unwrap_or(false)
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "LOGGING").ok();
    writeln!(out, "-------").ok();
    writeln!(
        out,
        "log_level:                  {}",
        cfg.log_level.as_deref().unwrap_or("info")
    )
    .ok();
    writeln!(
        out,
        "enable_file_logging:        {}",
        cfg.enable_file_logging.unwrap_or(false)
    )
    .ok();
    writeln!(
        out,
        "log_file:                   {}",
        cfg.log_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "TEST DATA").ok();
    writeln!(out, "---------").ok();
    writeln!(
        out,
        "test_data_file:             {}",
        cfg.test_data_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    )
    .ok();
    writeln!(out).ok();
    writeln!(out, "{FOOTER_TEXT}").ok();

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        out,
    )
}
