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
    pub vmswap: u64,
    pub start_time_seconds: f64, // Process start time (seconds since system boot)
    // Block I/O metrics from /proc/[pid]/io
    pub read_bytes: u64,  // Total bytes read from storage
    pub write_bytes: u64, // Total bytes written to storage
    // Network I/O metrics from eBPF (if available)
    pub rx_bytes: u64, // Total bytes received from network
    pub tx_bytes: u64, // Total bytes transmitted to network
    // Previous I/O values for delta calculation
    pub last_read_bytes: u64,
    pub last_write_bytes: u64,
    pub last_rx_bytes: u64,
    pub last_tx_bytes: u64,
    // Timestamp of last update for rate calculation
    pub last_update_time: f64, // Unix timestamp (seconds)
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
