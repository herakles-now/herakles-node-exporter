//! Application state management for the exporter.
//!
//! This module defines the shared application state that is passed
//! to HTTP handlers and used by the background cache update task.

use ahash::AHashMap as HashMap;
use herakles_node_exporter::HealthState;
use prometheus::{Gauge, Registry};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Instant;
use tokio::sync::{Notify, RwLock};

use crate::cache::MetricsCache;
use crate::cli::Args;
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
    pub processes: Gauge,
    pub cache_update_duration: Gauge,
    pub cache_update_success: Gauge,
    pub cache_updating: Gauge,
    pub database_entries: Gauge,
    pub database_size_bytes: Gauge,
    pub cache: Arc<RwLock<MetricsCache>>,
    pub config: Arc<StdRwLock<Config>>,
    pub buffer_config: StdRwLock<BufferConfig>,
    pub args: Args,
    pub cpu_cache: StdRwLock<HashMap<u32, CpuEntry>>,
    pub health_stats: Arc<HealthStats>,
    /// Health state for buffer monitoring.
    pub health_state: Arc<HealthState>,
    /// Notification for cache update completion.
    pub cache_ready: Arc<Notify>,
    /// CPU statistics cache for calculating usage ratios.
    pub system_cpu_cache: CpuStatsCache,
    /// eBPF manager for process I/O tracking (optional).
    pub ebpf: Option<Arc<EbpfManager>>,
    /// Ringbuffer manager for historical metrics tracking.
    pub ringbuffer_manager: Arc<RingbufferManager>,
    /// Server start time for uptime calculation.
    pub start_time: Instant,
}

impl AppState {
    /// Helper to get a read lock on the configuration
    pub fn config(&self) -> std::sync::RwLockReadGuard<'_, Config> {
        self.config.read().unwrap()
    }

    /// Helper to get a read lock on the buffer configuration
    pub fn buffer_config(&self) -> std::sync::RwLockReadGuard<'_, BufferConfig> {
        self.buffer_config.read().unwrap()
    }

    /// Reloads configuration file and regenerates buffer configuration
    pub fn reload_config(&self) -> Result<(), Box<dyn std::error::Error + '_>> {
        let new_config = crate::config::resolve_config(&self.args)?;
        let new_buffer_config = crate::process::resolve_buffer_config(&new_config, &self.args);

        *self.config.write()? = new_config;
        *self.buffer_config.write()? = new_buffer_config;

        tracing::info!("Exporter configuration reloaded successfully");
        Ok(())
    }
}
