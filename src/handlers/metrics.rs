//! Metrics endpoint handler for Prometheus scraping.
//!
//! This module provides the `/metrics` endpoint handler that formats and returns
//! process metrics in Prometheus text format.

use ahash::AHashMap as HashMap;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, instrument, warn};

use crate::cache::ProcMem;
use crate::process::classify_process_with_config;
use crate::state::SharedState;
use crate::system;

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

/// Handler for the /metrics endpoint.
#[instrument(skip(state))]
pub async fn metrics_handler(State(state): State<SharedState>) -> Result<String, MetricsError> {
    let start = Instant::now();
    debug!("Processing /metrics request");

    // Wait for cache to be available (not currently updating)
    loop {
        let cache_guard = state.cache.read().await;
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

            // Get uptime for this scrape cycle (constant for all metrics)
            let uptime_seconds = state.health_stats.get_uptime_seconds().to_string();

            let cfg = &state.config;
            let enable_rss = cfg.enable_rss.unwrap_or(true);
            let enable_pss = cfg.enable_pss.unwrap_or(true);
            let enable_uss = cfg.enable_uss.unwrap_or(true);
            let enable_cpu = cfg.enable_cpu.unwrap_or(true);

            // Aggregation map
            let mut groups: HashMap<(Arc<str>, Arc<str>), Vec<&ProcMem>> = HashMap::new();
            let mut exported_count = 0usize;

            // Enforce an overall limit for processes classified as "other".
            let mut other_exported = 0usize;
            let other_limit = state.config.top_n_others.unwrap_or(10);

            // Populate per-process metrics + prepare aggregation
            for p in &processes_vec {
                if let Some((group, subgroup)) =
                    classify_process_with_config(&p.name, &state.config)
                {
                    // If this is the "other" group, enforce the configured per-group limit.
                    if group.as_ref().eq_ignore_ascii_case("other") {
                        if other_exported >= other_limit {
                            continue;
                        }
                        other_exported += 1;
                    }

                    exported_count += 1;
                    let pid_str = p.pid.to_string();

                    state.metrics.set_for_process(
                        &pid_str,
                        &p.name,
                        group.as_ref(),
                        subgroup.as_ref(),
                        p.rss,
                        p.pss,
                        p.uss,
                        p.cpu_percent as f64,
                        p.cpu_time_seconds as f64,
                        &state.config,
                        &uptime_seconds,
                    );

                    groups.entry((group, subgroup)).or_default().push(p);
                }
            }

            state.processes_total.set(exported_count as f64);
            state.scrape_duration.set(start.elapsed().as_secs_f64());

            // Aggregated sums and Top-N metrics per subgroup
            for ((group, subgroup), mut list) in groups {
                let mut rss_sum: u64 = 0;
                let mut pss_sum: u64 = 0;
                let mut uss_sum: u64 = 0;
                let mut cpu_percent_sum: f64 = 0.0;
                let mut cpu_time_sum: f64 = 0.0;

                for p in &list {
                    rss_sum += p.rss;
                    pss_sum += p.pss;
                    uss_sum += p.uss;
                    cpu_percent_sum += p.cpu_percent as f64;
                    cpu_time_sum += p.cpu_time_seconds as f64;
                }

                let group_ref: &str = group.as_ref();
                let subgroup_ref: &str = subgroup.as_ref();

                // Set aggregation metrics (respect enable_* flags)
                if enable_rss {
                    state
                        .metrics
                        .agg_rss_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(rss_sum as f64);
                }
                if enable_pss {
                    state
                        .metrics
                        .agg_pss_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(pss_sum as f64);
                }
                if enable_uss {
                    state
                        .metrics
                        .agg_uss_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(uss_sum as f64);
                }
                if enable_cpu {
                    state
                        .metrics
                        .agg_cpu_percent_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(cpu_percent_sum);
                    state
                        .metrics
                        .agg_cpu_time_sum
                        .with_label_values(&[group_ref, subgroup_ref, &uptime_seconds])
                        .set(cpu_time_sum);
                }

                // Sort by USS for Top-N selection
                list.sort_by_key(|p| std::cmp::Reverse(p.uss));

                let is_other_group = group_ref.eq_ignore_ascii_case("other")
                    || group_ref.eq_ignore_ascii_case("others")
                    || subgroup_ref.eq_ignore_ascii_case("other")
                    || subgroup_ref.eq_ignore_ascii_case("others");

                let top_subgroup = state.config.top_n_subgroup.unwrap_or(3);
                let top_others = state.config.top_n_others.unwrap_or(10);
                let limit = if is_other_group {
                    std::cmp::max(1, top_others)
                } else {
                    std::cmp::max(1, top_subgroup)
                };

                let rss_total = rss_sum as f64;
                let pss_total = pss_sum as f64;
                let uss_total = uss_sum as f64;
                let cpu_total = cpu_time_sum;

                for (rank, p) in list.iter().take(limit).enumerate() {
                    let pid_s = p.pid.to_string();
                    let rank_s = (rank + 1).to_string();
                    let name_s = p.name.as_str();

                    // Absolute Top-N values
                    if enable_rss {
                        state
                            .metrics
                            .top_rss
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.rss as f64);
                    }
                    if enable_pss {
                        state
                            .metrics
                            .top_pss
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.pss as f64);
                    }
                    if enable_uss {
                        state
                            .metrics
                            .top_uss
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.uss as f64);
                    }
                    if enable_cpu {
                        state
                            .metrics
                            .top_cpu_percent
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.cpu_percent as f64);
                        state
                            .metrics
                            .top_cpu_time
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(p.cpu_time_seconds as f64);
                    }

                    // Percentage-of-subgroup values
                    if enable_cpu && cpu_total > 0.0 {
                        let pct = (p.cpu_time_seconds as f64 / cpu_total) * 100.0;
                        state
                            .metrics
                            .top_cpu_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }

                    if enable_rss && rss_total > 0.0 {
                        let pct = (p.rss as f64 / rss_total) * 100.0;
                        state
                            .metrics
                            .top_rss_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }

                    if enable_pss && pss_total > 0.0 {
                        let pct = (p.pss as f64 / pss_total) * 100.0;
                        state
                            .metrics
                            .top_pss_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }

                    if enable_uss && uss_total > 0.0 {
                        let pct = (p.uss as f64 / uss_total) * 100.0;
                        state
                            .metrics
                            .top_uss_percent_of_subgroup
                            .with_label_values(&[
                                group_ref,
                                subgroup_ref,
                                &rank_s,
                                &pid_s,
                                name_s,
                                &uptime_seconds,
                            ])
                            .set(pct);
                    }
                }
            }

            // Update system-wide metrics
            match system::read_load_average() {
                Ok(load_avg) => {
                    // Set load metrics
                    state.metrics.set_system_load_metrics(
                        load_avg.one_min,
                        load_avg.five_min,
                        load_avg.fifteen_min,
                    );
                }
                Err(e) => {
                    warn!("Failed to read load average: {}", e);
                }
            }

            // Set new extended memory metrics
            match system::read_extended_memory_info() {
                Ok(mem_info) => {
                    state.metrics.set_system_memory_metrics(
                        mem_info.total_bytes,
                        mem_info.available_bytes,
                    );
                }
                Err(e) => {
                    warn!("Failed to read extended memory info: {}", e);
                }
            }

            // Set CPU usage ratio metrics
            match state.system_cpu_cache.calculate_usage_ratios() {
                Ok(cpu_ratios) => {
                    state.metrics.set_system_cpu_usage_ratios(&cpu_ratios);
                }
                Err(e) => {
                    warn!("Failed to calculate CPU usage ratios: {}", e);
                }
            }

            // Encode metrics in Prometheus text format
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

            // Record metrics request statistics
            let request_duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            state.health_stats.record_metrics_endpoint_call();
            state
                .health_stats
                .record_request_duration(request_duration_ms);
            state.health_stats.record_http_request();
            state.health_stats.record_cache_hit();

            debug!(
                "Metrics request completed: {} processes (exported {}), {} bytes, {:.3}ms",
                processes_vec.len(),
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
