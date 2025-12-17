//! Configuration types for buffer health monitoring.
//!
//! This module provides configuration structures for tracking buffer fill levels
//! and determining their health status.

use serde::Deserialize;

/// Configuration for a single buffer's health monitoring.
///
/// Each buffer can be configured with its capacity, whether larger fill levels
/// are considered better, and optional threshold percentages for warning and
/// critical status levels.
#[derive(Debug, Clone, Deserialize)]
pub struct BufferHealthConfig {
    /// Maximum capacity of the buffer in kilobytes.
    pub capacity_kb: usize,
    /// If true, higher fill percentages are considered healthy (e.g., cache buffers).
    /// If false, lower fill percentages are considered healthy (e.g., overflow buffers).
    pub larger_is_better: bool,
    /// Optional warning threshold as a percentage (0-100).
    /// For `larger_is_better=false`: warn when fill_percent > warn_percent
    /// For `larger_is_better=true`: warn when fill_percent < warn_percent
    pub warn_percent: Option<f64>,
    /// Optional critical threshold as a percentage (0-100).
    /// For `larger_is_better=false`: critical when fill_percent > critical_percent
    /// For `larger_is_better=true`: critical when fill_percent < critical_percent
    pub critical_percent: Option<f64>,
}

impl Default for BufferHealthConfig {
    fn default() -> Self {
        Self {
            capacity_kb: 256,
            larger_is_better: false,
            warn_percent: None,
            critical_percent: None,
        }
    }
}

/// Application-wide buffer health configuration.
///
/// Groups the configuration for all three internal buffers:
/// - `io_buffer`: General IO buffer for /proc readers
/// - `smaps_buffer`: Buffer for /proc/<pid>/smaps parsing
/// - `smaps_rollup_buffer`: Buffer for /proc/<pid>/smaps_rollup parsing
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// Configuration for the general IO buffer.
    pub io_buffer: BufferHealthConfig,
    /// Configuration for the smaps buffer.
    pub smaps_buffer: BufferHealthConfig,
    /// Configuration for the smaps_rollup buffer.
    pub smaps_rollup_buffer: BufferHealthConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            io_buffer: BufferHealthConfig {
                capacity_kb: 256,
                larger_is_better: false,
                warn_percent: Some(80.0),
                critical_percent: Some(95.0),
            },
            smaps_buffer: BufferHealthConfig {
                capacity_kb: 512,
                larger_is_better: false,
                warn_percent: Some(80.0),
                critical_percent: Some(95.0),
            },
            smaps_rollup_buffer: BufferHealthConfig {
                capacity_kb: 256,
                larger_is_better: false,
                warn_percent: Some(80.0),
                critical_percent: Some(95.0),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_health_config_default() {
        let config = BufferHealthConfig::default();
        assert_eq!(config.capacity_kb, 256);
        assert!(!config.larger_is_better);
        assert!(config.warn_percent.is_none());
        assert!(config.critical_percent.is_none());
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.io_buffer.capacity_kb, 256);
        assert_eq!(config.smaps_buffer.capacity_kb, 512);
        assert_eq!(config.smaps_rollup_buffer.capacity_kb, 256);
    }
}
