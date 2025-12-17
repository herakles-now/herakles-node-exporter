//! Application state management for the exporter.
//!
//! This module defines the shared application state that is passed
//! to HTTP handlers and used by the background cache update task.

use ahash::AHashMap as HashMap;
use herakles_proc_mem_exporter::HealthState;
use prometheus::{Gauge, Registry};
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::{Notify, RwLock};

use crate::cache::MetricsCache;
use crate::config::Config;
use crate::health_stats::HealthStats;
use crate::metrics::MemoryMetrics;
use crate::process::{BufferConfig, CpuEntry};
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
    /// Notification for cache update completion.
    pub cache_ready: Arc<Notify>,
    /// CPU statistics cache for calculating usage ratios.
    pub system_cpu_cache: CpuStatsCache,
}
