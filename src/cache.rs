//! Cache management for process metrics.
//!
//! This module provides the `MetricsCache` structure for storing process metrics
//! between collection intervals, along with metadata about the cache state.

use ahash::AHashMap as HashMap;
use std::time::Instant;

/// Process memory and CPU metrics collected from /proc.
#[derive(Debug, Clone)]
pub struct ProcMem {
    pub pid: u32,
    pub name: String,
    pub rss: u64,
    pub pss: u64,
    pub uss: u64,
    pub cpu_percent: f32,
    pub cpu_time_seconds: f32,
}

/// Cache state for storing process metrics with update timing information.
#[derive(Clone, Default)]
pub struct MetricsCache {
    pub processes: HashMap<u32, ProcMem>,
    pub last_updated: Option<Instant>,
    pub update_duration_seconds: f64,
    pub update_success: bool,
    pub is_updating: bool,
}
