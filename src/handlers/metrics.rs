//! Metrics endpoint handler for Prometheus scraping.
//!
//! This module provides the `/metrics` endpoint handler that formats and returns
//! system and group-level metrics in Prometheus text format according to the German specification.
//! NO per-process or Top-N metrics are exported.

mod ebpf_export;
mod system_export;

use ahash::AHashMap as HashMap;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};
use std::time::Instant;
use tracing::{debug, error, instrument};

use crate::cache::ProcMem;
use crate::process::classify_process_with_config;
use crate::state::SharedState;

/// Buffer capacity for metrics encoding.
const BUFFER_CAP: usize = 512 * 1024;

/// Error type for metrics endpoint failures.
#[derive(Debug)]
pub enum MetricsError {
    EncodingFailed,
}

impl IntoResponse for MetricsError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics",
        )
            .into_response()
    }
}

/// Aggregated metrics for a group/subgroup.
#[derive(Default, Debug)]
struct GroupMetrics {
    rss_sum: u64,
    pss_sum: u64,
    swap_sum: u64,
    cpu_percent_sum: f64,
    cpu_time_total_sum: f64,
}

/// Handler for the /metrics endpoint.
#[instrument(skip(state))]
pub async fn metrics_handler(State(state): State<SharedState>) -> Result<String, MetricsError> {
    let start = Instant::now();
    debug!("Processing /metrics request");

    // Wait for cache to be available (not currently updating)
    loop {
        // Measure lock wait time
        let lock_wait_start = Instant::now();
        let cache_guard = state.cache.read().await;
        let lock_wait_ms = lock_wait_start.elapsed().as_secs_f64() * 1000.0;
        state
            .health_stats
            .record_lock_wait_duration_ms(lock_wait_ms);

        if !cache_guard.is_updating {
            let processes_vec: Vec<ProcMem> = cache_guard.processes.values().cloned().collect();
            let meta = (
                cache_guard.update_duration_seconds,
                cache_guard.update_success,
                cache_guard.is_updating,
            );

            drop(cache_guard);

            // Update cache metadata metrics
            state.cache_update_duration.set(meta.0);
            state
                .cache_update_success
                .set(if meta.1 { 1.0 } else { 0.0 });
            state.cache_updating.set(if meta.2 { 1.0 } else { 0.0 });

            // Reset metrics before populating with fresh data
            state.metrics.reset();

            let config_guard = state.config();
            let cfg = &*config_guard;
            let enable_rss = cfg.enable_rss.unwrap_or(true);
            let enable_pss = cfg.enable_pss.unwrap_or(true);
            let enable_cpu = cfg.enable_cpu.unwrap_or(true);

            // ========== PHASE 1: Aggregate processes by (group, subgroup) ==========
            let mut group_aggregations: HashMap<(String, String), GroupMetrics> = HashMap::new();
            let mut exported_count = 0usize;

            for p in &processes_vec {
                if let Some((group, subgroup)) = classify_process_with_config(&p.name, cfg) {
                    exported_count += 1;

                    let entry = group_aggregations
                        .entry((group.to_string(), subgroup.to_string()))
                        .or_default();

                    entry.rss_sum += p.rss;
                    entry.pss_sum += p.pss;
                    entry.swap_sum += p.vmswap;
                    entry.cpu_percent_sum += p.cpu_percent as f64;
                    entry.cpu_time_total_sum += p.cpu_time_seconds as f64;
                }
            }

            state.processes_total.set(exported_count as f64);

            // ========== PHASE 2: Export Group-Level Metrics ==========
            for ((group, subgroup), metrics) in group_aggregations {
                // Memory Group Metrics
                if enable_rss {
                    state
                        .metrics
                        .group_memory_rss_bytes
                        .with_label_values(&[&group, &subgroup])
                        .set(metrics.rss_sum as f64);
                }

                if enable_pss {
                    state
                        .metrics
                        .group_memory_pss_bytes
                        .with_label_values(&[&group, &subgroup])
                        .set(metrics.pss_sum as f64);
                }

                state
                    .metrics
                    .group_memory_swap_bytes
                    .with_label_values(&[&group, &subgroup])
                    .set(metrics.swap_sum as f64);

                // CPU Group Metrics
                if enable_cpu {
                    // Convert CPU percentage to ratio (0.0-1.0)
                    let cpu_ratio = metrics.cpu_percent_sum / 100.0;
                    state
                        .metrics
                        .group_cpu_usage_ratio
                        .with_label_values(&[&group, &subgroup])
                        .set(cpu_ratio);

                    state
                        .metrics
                        .group_cpu_seconds_total
                        .with_label_values(&[group.as_str(), subgroup.as_str(), "total"])
                        .set(metrics.cpu_time_total_sum);
                }
            }

            ebpf_export::export_blkio_metrics(&state);
            system_export::export_system_metrics(
                &state,
                cfg.enable_filesystem_collector.unwrap_or(true),
                cfg.enable_thermal_collector.unwrap_or(true),
            );

            ebpf_export::export_network_metrics(&state);
            ebpf_export::export_tcp_metrics(&state, cfg.enable_tcp_tracking.unwrap_or(true));

            // ========== PHASE 11: Encode and Return Metrics ==========
            let serialize_start = Instant::now();
            let families = state.registry.gather();

            // Calculate label cardinality
            let mut label_count: u64 = 0;
            for family in &families {
                for metric in family.get_metric() {
                    label_count += metric.get_label().len() as u64;
                }
            }
            state.health_stats.record_label_cardinality(label_count);

            let mut buffer = Vec::with_capacity(BUFFER_CAP);
            let encoder = TextEncoder::new();

            if encoder.encode(&families, &mut buffer).is_err() {
                error!("Failed to encode Prometheus metrics");
                return Err(MetricsError::EncodingFailed);
            }

            let serialization_ms = serialize_start.elapsed().as_secs_f64() * 1000.0;
            state
                .health_stats
                .record_serialization_duration_ms(serialization_ms);

            // Record response size
            let response_size_kb = buffer.len() as f64 / 1024.0;
            state
                .health_stats
                .record_metrics_response_size_kb(response_size_kb);

            // Count time series
            let time_series_count =
                families.iter().map(|f| f.get_metric().len()).sum::<usize>() as u64;
            state
                .health_stats
                .record_total_time_series(time_series_count);

            // Record metrics request statistics
            let request_duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            state.health_stats.record_metrics_endpoint_call();
            state
                .health_stats
                .record_request_duration(request_duration_ms);
            state.health_stats.record_http_request();
            state.health_stats.record_cache_hit();

            state.scrape_duration.set(start.elapsed().as_secs_f64());

            debug!(
                "Metrics request completed: {} processes, {} bytes, {:.3}ms",
                exported_count,
                buffer.len(),
                request_duration_ms
            );

            return String::from_utf8(buffer).map_err(|_| MetricsError::EncodingFailed);
        }

        drop(cache_guard);
        // Wait for notification that cache update is complete
        state.cache_ready.notified().await;
    }
}
