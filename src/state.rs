//! Application state management for the exporter.
//!
//! This module defines the shared application state that is passed
//! to HTTP handlers and used by the background cache update task.

use ahash::AHashMap as HashMap;
use herakles_node_exporter::HealthState;
use prometheus::{Gauge, Registry};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Instant;
use tokio::sync::RwLock;

use crate::cache::MetricsCache;
use crate::config::Config;
use crate::ebpf::EbpfManager;
use crate::health_stats::HealthStats;
use crate::metrics::MemoryMetrics;
use crate::process::{BufferConfig, CpuEntry};
use crate::ringbuffer_manager::RingbufferManager;
use crate::system::CpuStatsCache;

/// Type alias for shared application state.
pub type SharedState = Arc<AppState>;

/// Global application state shared across requests and background tasks.
pub struct AppState {
    pub registry: Registry,
    pub metrics: MemoryMetrics,
    pub scrape_duration: Gauge,
    pub processes_total: Gauge,
    pub cache_update_duration: Gauge,
    pub cache_update_success: Gauge,
    pub cache_updating: Gauge,
    pub cache: Arc<RwLock<MetricsCache>>,
    pub config: Arc<Config>,
    pub buffer_config: BufferConfig,
    pub cpu_cache: StdRwLock<HashMap<u32, CpuEntry>>,
    pub health_stats: Arc<HealthStats>,
    /// Health state for buffer monitoring.
    pub health_state: Arc<HealthState>,
    /// CPU statistics cache for calculating usage ratios.
    pub system_cpu_cache: CpuStatsCache,
    /// eBPF manager for process I/O tracking (optional).
    pub ebpf: Option<Arc<EbpfManager>>,
    /// Ringbuffer manager for historical metrics tracking.
    pub ringbuffer_manager: Arc<RingbufferManager>,
    /// Server start time for uptime calculation.
    pub start_time: Instant,
}
