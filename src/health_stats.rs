//! Health statistics and monitoring for the exporter.
//!
//! This module provides types and functionality for tracking exporter health,
//! including scan performance, cache statistics, and HTTP request metrics.

use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock as StdRwLock};
use std::time::{Instant, SystemTime};

/// Running statistics for a single metric.
#[derive(Clone, Copy, Default)]
pub struct RunningStat {
    count: u64,
    sum: f64,
    min: f64,
    max: f64,
    last: f64,
}

impl RunningStat {
    pub fn add(&mut self, value: f64) {
        if self.count == 0 {
            self.min = value;
            self.max = value;
            self.last = value;
            self.sum = value;
            self.count = 1;
            return;
        }
        self.count += 1;
        self.sum += value;
        self.last = value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / (self.count as f64)
        }
    }
}

/// Thread-safe wrapper for running statistics.
#[derive(Default)]
pub struct Stat {
    inner: Mutex<RunningStat>,
}

impl Stat {
    pub fn add_sample(&self, value: f64) {
        if let Ok(mut s) = self.inner.lock() {
            s.add(value);
        }
    }

    pub fn snapshot(&self) -> (f64, f64, f64, f64, u64) {
        if let Ok(s) = self.inner.lock() {
            (s.last, s.avg(), s.max, s.min, s.count)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0)
        }
    }
}

/// Thread-safe circular buffer for tracking HTTP request timestamps.
pub struct RequestTimestamps {
    inner: Mutex<VecDeque<Instant>>,
}

impl Default for RequestTimestamps {
    fn default() -> Self {
        Self {
            inner: Mutex::new(VecDeque::with_capacity(1024)),
        }
    }
}

impl RequestTimestamps {
    pub fn record(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.push_back(Instant::now());
            // Keep only last 10 minutes of timestamps to avoid unbounded growth
            let cutoff = Instant::now() - std::time::Duration::from_secs(600);
            while guard.front().is_some_and(|&t| t < cutoff) {
                guard.pop_front();
            }
        }
    }

    pub fn count_last_minute(&self) -> u64 {
        if let Ok(guard) = self.inner.lock() {
            let cutoff = Instant::now() - std::time::Duration::from_secs(60);
            guard.iter().filter(|&&t| t >= cutoff).count() as u64
        } else {
            0
        }
    }
}

/// Comprehensive health statistics for the exporter.
pub struct HealthStats {
    // Existing fields
    pub scanned_processes: Stat,
    pub scan_duration_seconds: Stat,
    pub cache_update_duration_seconds: Stat,
    pub total_scans: AtomicU64,

    // Scan performance
    pub scan_success_count: AtomicU64,
    pub scan_failure_count: AtomicU64,
    pub used_subgroups: Stat,

    // Cache performance
    pub cache_size: Stat,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,

    // HTTP server stats
    pub http_request_timestamps: RequestTimestamps,
    pub request_duration_ms: Stat,
    pub label_cardinality: Stat,
    pub metrics_endpoint_calls: AtomicU64,

    // Exporter resources
    pub exporter_memory_mb: Stat,
    pub exporter_cpu_percent: Stat,

    // Timing
    pub start_time: Instant,
    pub last_scan_time: StdRwLock<Option<Instant>>,
}

impl Default for HealthStats {
    fn default() -> Self {
        Self {
            scanned_processes: Stat::default(),
            scan_duration_seconds: Stat::default(),
            cache_update_duration_seconds: Stat::default(),
            total_scans: AtomicU64::new(0),
            scan_success_count: AtomicU64::new(0),
            scan_failure_count: AtomicU64::new(0),
            used_subgroups: Stat::default(),
            cache_size: Stat::default(),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            http_request_timestamps: RequestTimestamps::default(),
            request_duration_ms: Stat::default(),
            label_cardinality: Stat::default(),
            metrics_endpoint_calls: AtomicU64::new(0),
            exporter_memory_mb: Stat::default(),
            exporter_cpu_percent: Stat::default(),
            start_time: Instant::now(),
            last_scan_time: StdRwLock::new(None),
        }
    }
}

impl HealthStats {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn record_scan(
        &self,
        scanned: u64,
        scan_duration_seconds: f64,
        cache_update_duration_seconds: f64,
    ) {
        self.scanned_processes.add_sample(scanned as f64);
        self.scan_duration_seconds.add_sample(scan_duration_seconds);
        self.cache_update_duration_seconds
            .add_sample(cache_update_duration_seconds);
        self.total_scans.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_scan_success(&self) {
        self.scan_success_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_scan_failure(&self) {
        self.scan_failure_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_used_subgroups(&self, count: u64) {
        self.used_subgroups.add_sample(count as f64);
    }

    pub fn record_cache_size(&self, size: u64) {
        self.cache_size.add_sample(size as f64);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss. Currently unused but kept for API completeness.
    #[allow(dead_code)]
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_http_request(&self) {
        self.http_request_timestamps.record();
    }

    pub fn record_request_duration(&self, duration_ms: f64) {
        self.request_duration_ms.add_sample(duration_ms);
    }

    pub fn record_label_cardinality(&self, count: u64) {
        self.label_cardinality.add_sample(count as f64);
    }

    pub fn record_metrics_endpoint_call(&self) {
        self.metrics_endpoint_calls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_exporter_resources(&self, memory_mb: f64, cpu_percent: f64) {
        self.exporter_memory_mb.add_sample(memory_mb);
        self.exporter_cpu_percent.add_sample(cpu_percent);
    }

    pub fn update_last_scan_time(&self) {
        if let Ok(mut guard) = self.last_scan_time.write() {
            *guard = Some(Instant::now());
        }
    }

    pub fn get_scan_success_rate(&self) -> f64 {
        let success = self.scan_success_count.load(Ordering::Relaxed);
        let failure = self.scan_failure_count.load(Ordering::Relaxed);
        let total = success + failure;
        if total == 0 {
            100.0
        } else {
            (success as f64 / total as f64) * 100.0
        }
    }

    pub fn get_cache_hit_ratio(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            100.0 // Default to 100% when no cache operations have occurred
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }

    pub fn get_uptime_hours(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64() / 3600.0
    }

    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub fn get_last_scan_time_str(&self) -> String {
        // Time constants for formatting
        const SECS_PER_DAY: u64 = 86400;
        const SECS_PER_HOUR: u64 = 3600;
        const SECS_PER_MINUTE: u64 = 60;

        if let Ok(guard) = self.last_scan_time.read() {
            if let Some(last_scan) = *guard {
                // Calculate time since epoch by using SystemTime
                let elapsed_since_scan = last_scan.elapsed();
                let now = SystemTime::now();
                if let Ok(duration) = now.duration_since(SystemTime::UNIX_EPOCH) {
                    let scan_time_secs = duration
                        .as_secs()
                        .saturating_sub(elapsed_since_scan.as_secs());
                    let hours = (scan_time_secs % SECS_PER_DAY) / SECS_PER_HOUR;
                    let minutes = (scan_time_secs % SECS_PER_HOUR) / SECS_PER_MINUTE;
                    let seconds = scan_time_secs % SECS_PER_MINUTE;
                    return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
                }
            }
        }
        "N/A".to_string()
    }

    pub fn render_table(&self) -> String {
        let (sc_cur, sc_avg, sc_max, sc_min, _sc_count) = self.scanned_processes.snapshot();
        let (sd_cur, sd_avg, sd_max, sd_min, _sd_count) = self.scan_duration_seconds.snapshot();
        let (cu_cur, cu_avg, cu_max, cu_min, _cu_count) =
            self.cache_update_duration_seconds.snapshot();
        let total = self.total_scans.load(Ordering::Relaxed);

        // New metrics snapshots
        let (ug_cur, ug_avg, ug_max, ug_min, _) = self.used_subgroups.snapshot();
        let (cs_cur, cs_avg, cs_max, cs_min, _) = self.cache_size.snapshot();
        let (rd_cur, rd_avg, rd_max, rd_min, _) = self.request_duration_ms.snapshot();
        let (lc_cur, lc_avg, lc_max, lc_min, _) = self.label_cardinality.snapshot();
        let (em_cur, em_avg, em_max, em_min, _) = self.exporter_memory_mb.snapshot();
        let (ec_cur, ec_avg, ec_max, ec_min, _) = self.exporter_cpu_percent.snapshot();

        let scan_success_rate = self.get_scan_success_rate();
        let cache_hit_ratio = self.get_cache_hit_ratio();
        let http_requests_last_minute = self.http_request_timestamps.count_last_minute();
        let metrics_calls = self.metrics_endpoint_calls.load(Ordering::Relaxed);
        let uptime_hours = self.get_uptime_hours();
        let last_scan = self.get_last_scan_time_str();

        let left_col = 26usize;
        let col_w = 12usize;

        let mut out = String::new();

        writeln!(out, "HEALTH ENDPOINT - EXPORTER INTERNAL STATS").ok();
        writeln!(out, "==========================================").ok();
        writeln!(out).ok();

        // Header
        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "",
            "current",
            "average",
            "max",
            "min",
            left = left_col,
            col = col_w
        )
        .ok();

        // SCAN PERFORMANCE section
        writeln!(out).ok();
        writeln!(out, "SCAN PERFORMANCE").ok();
        writeln!(out, "-----------------").ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "scanned_processes",
            format!("{:.0}", sc_cur),
            format!("{:.1}", sc_avg),
            format!("{:.0}", sc_max),
            format!("{:.0}", sc_min),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "scan_duration (s)",
            format!("{:.3}", sd_cur),
            format!("{:.3}", sd_avg),
            format!("{:.3}", sd_max),
            format!("{:.3}", sd_min),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "scan_success_rate (%)",
            format!("{:.1}", scan_success_rate),
            format!("{:.1}", scan_success_rate),
            format!("{:.1}", scan_success_rate),
            format!("{:.1}", scan_success_rate),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "used_subgroups",
            format!("{:.0}", ug_cur),
            format!("{:.1}", ug_avg),
            format!("{:.0}", ug_max),
            format!("{:.0}", ug_min),
            left = left_col,
            col = col_w
        )
        .ok();

        // CACHE PERFORMANCE section
        writeln!(out).ok();
        writeln!(out, "CACHE PERFORMANCE").ok();
        writeln!(out, "------------------").ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "cache_update_duration (s)",
            format!("{:.3}", cu_cur),
            format!("{:.3}", cu_avg),
            format!("{:.3}", cu_max),
            format!("{:.3}", cu_min),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "cache_hit_ratio (%)",
            format!("{:.1}", cache_hit_ratio),
            format!("{:.1}", cache_hit_ratio),
            format!("{:.1}", cache_hit_ratio),
            format!("{:.1}", cache_hit_ratio),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "cache_size",
            format!("{:.0}", cs_cur),
            format!("{:.1}", cs_avg),
            format!("{:.0}", cs_max),
            format!("{:.0}", cs_min),
            left = left_col,
            col = col_w
        )
        .ok();

        // HTTP SERVER section
        writeln!(out).ok();
        writeln!(out, "HTTP SERVER").ok();
        writeln!(out, "-----------").ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "http_requests_last_minute",
            format!("{}", http_requests_last_minute),
            "N/A",
            "N/A",
            "N/A",
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "avg_request_duration (ms)",
            format!("{:.1}", rd_cur),
            format!("{:.1}", rd_avg),
            format!("{:.1}", rd_max),
            format!("{:.1}", rd_min),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "label_cardinality_total",
            format!("{:.0}", lc_cur),
            format!("{:.1}", lc_avg),
            format!("{:.0}", lc_max),
            format!("{:.0}", lc_min),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "metrics_endpoint_calls",
            format!("{}", metrics_calls),
            "N/A",
            "N/A",
            "N/A",
            left = left_col,
            col = col_w
        )
        .ok();

        // EXPORTER RESOURCES section
        writeln!(out).ok();
        writeln!(out, "EXPORTER RESOURCES").ok();
        writeln!(out, "-------------------").ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "exporter_memory_usage (MB)",
            format!("{:.1}", em_cur),
            format!("{:.1}", em_avg),
            format!("{:.1}", em_max),
            format!("{:.1}", em_min),
            left = left_col,
            col = col_w
        )
        .ok();

        writeln!(
            out,
            "{:left$} | {:^col$} | {:^col$} | {:^col$} | {:^col$}",
            "exporter_cpu_usage (%)",
            format!("{:.1}", ec_cur),
            format!("{:.1}", ec_avg),
            format!("{:.1}", ec_max),
            format!("{:.1}", ec_min),
            left = left_col,
            col = col_w
        )
        .ok();

        // Summary line
        writeln!(out).ok();
        writeln!(
            out,
            "number of done scans: {} | last scan: {} | uptime: {:.1}h",
            total, last_scan, uptime_hours
        )
        .ok();

        out
    }
}
