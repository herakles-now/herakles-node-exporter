//! Health monitoring module for internal buffer fill levels.
//!
//! This module provides types and functionality for monitoring the health status
//! of internal buffers, allowing users to decide whether "larger is better" for
//! each buffer type.
//!
//! # Usage
//!
//! ```rust
//! use herakles_proc_mem_exporter::{AppConfig, HealthState};
//!
//! // Create health state with configuration
//! let config = AppConfig::default();
//! let health_state = HealthState::new(config);
//!
//! // Update buffer values as they change
//! health_state.update_io_buffer_kb(128);
//! health_state.update_smaps_buffer_kb(256);
//! health_state.update_smaps_rollup_buffer_kb(100);
//!
//! // Get current health status
//! let response = health_state.get_health();
//! println!("Overall status: {}", response.overall_status);
//! ```

use crate::health_config::{AppConfig, BufferHealthConfig};
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Health status for a single buffer.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BufferHealth {
    /// Name of the buffer (e.g., "io_buffer_kb").
    pub name: String,
    /// Configured capacity of the buffer in kilobytes.
    pub capacity_kb: usize,
    /// Current usage of the buffer in kilobytes.
    pub current_kb: usize,
    /// Current fill percentage (0.0 to 100.0).
    pub fill_percent: f64,
    /// Whether larger fill percentages are considered healthy.
    pub larger_is_better: bool,
    /// Health status: "ok", "warn", or "critical".
    pub status: String,
}

/// Health response containing status for all buffers.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// Health status for each buffer.
    pub buffers: Vec<BufferHealth>,
    /// Overall health status: "ok", "warn", or "critical".
    /// This is the worst status among all buffers.
    pub overall_status: String,
}

/// Thread-safe state for tracking buffer health.
///
/// Uses atomic operations for efficient cross-thread updates.
pub struct HealthState {
    io_buffer_kb: Arc<AtomicUsize>,
    smaps_buffer_kb: Arc<AtomicUsize>,
    smaps_rollup_buffer_kb: Arc<AtomicUsize>,
    config: Arc<AppConfig>,
}

impl HealthState {
    /// Creates a new HealthState with the given configuration.
    pub fn new(config: AppConfig) -> Self {
        Self {
            io_buffer_kb: Arc::new(AtomicUsize::new(0)),
            smaps_buffer_kb: Arc::new(AtomicUsize::new(0)),
            smaps_rollup_buffer_kb: Arc::new(AtomicUsize::new(0)),
            config: Arc::new(config),
        }
    }

    /// Updates the current IO buffer usage in kilobytes.
    pub fn update_io_buffer_kb(&self, value_kb: usize) {
        self.io_buffer_kb.store(value_kb, Ordering::Relaxed);
    }

    /// Updates the current smaps buffer usage in kilobytes.
    pub fn update_smaps_buffer_kb(&self, value_kb: usize) {
        self.smaps_buffer_kb.store(value_kb, Ordering::Relaxed);
    }

    /// Updates the current smaps_rollup buffer usage in kilobytes.
    pub fn update_smaps_rollup_buffer_kb(&self, value_kb: usize) {
        self.smaps_rollup_buffer_kb
            .store(value_kb, Ordering::Relaxed);
    }

    /// Gets the current IO buffer usage in kilobytes.
    pub fn get_io_buffer_kb(&self) -> usize {
        self.io_buffer_kb.load(Ordering::Relaxed)
    }

    /// Gets the current smaps buffer usage in kilobytes.
    pub fn get_smaps_buffer_kb(&self) -> usize {
        self.smaps_buffer_kb.load(Ordering::Relaxed)
    }

    /// Gets the current smaps_rollup buffer usage in kilobytes.
    pub fn get_smaps_rollup_buffer_kb(&self) -> usize {
        self.smaps_rollup_buffer_kb.load(Ordering::Relaxed)
    }

    /// Returns the current health status for all buffers.
    pub fn get_health(&self) -> HealthResponse {
        let io_health = self.compute_buffer_health(
            "io_buffer_kb",
            self.io_buffer_kb.load(Ordering::Relaxed),
            &self.config.io_buffer,
        );

        let smaps_health = self.compute_buffer_health(
            "smaps_buffer_kb",
            self.smaps_buffer_kb.load(Ordering::Relaxed),
            &self.config.smaps_buffer,
        );

        let smaps_rollup_health = self.compute_buffer_health(
            "smaps_rollup_buffer_kb",
            self.smaps_rollup_buffer_kb.load(Ordering::Relaxed),
            &self.config.smaps_rollup_buffer,
        );

        let buffers = vec![io_health, smaps_health, smaps_rollup_health];

        // Determine overall status (worst of all buffers)
        let overall_status = buffers
            .iter()
            .map(|b| status_priority(&b.status))
            .max()
            .map(priority_to_status)
            .unwrap_or_else(|| "ok".to_string());

        HealthResponse {
            buffers,
            overall_status,
        }
    }

    fn compute_buffer_health(
        &self,
        name: &str,
        current_kb: usize,
        config: &BufferHealthConfig,
    ) -> BufferHealth {
        let capacity_kb = config.capacity_kb.max(1); // Avoid division by zero
        let fill_percent = (current_kb as f64) / (capacity_kb as f64) * 100.0;

        let status = evaluate_status(
            fill_percent,
            config.larger_is_better,
            config.warn_percent,
            config.critical_percent,
        );

        BufferHealth {
            name: name.to_string(),
            capacity_kb: config.capacity_kb,
            current_kb,
            fill_percent,
            larger_is_better: config.larger_is_better,
            status,
        }
    }
}

/// Evaluates the health status based on fill percentage and thresholds.
///
/// For `larger_is_better == false`: higher fill_percent is worse
/// - status is "critical" if percent > critical_percent
/// - status is "warn" if percent > warn_percent
/// - otherwise "ok"
///
/// For `larger_is_better == true`: lower fill_percent is worse
/// - status is "critical" if percent < critical_percent
/// - status is "warn" if percent < warn_percent
/// - otherwise "ok"
fn evaluate_status(
    fill_percent: f64,
    larger_is_better: bool,
    warn_percent: Option<f64>,
    critical_percent: Option<f64>,
) -> String {
    if larger_is_better {
        // Lower fill_percent is worse
        if let Some(critical) = critical_percent {
            if fill_percent < critical {
                return "critical".to_string();
            }
        }
        if let Some(warn) = warn_percent {
            if fill_percent < warn {
                return "warn".to_string();
            }
        }
    } else {
        // Higher fill_percent is worse
        if let Some(critical) = critical_percent {
            if fill_percent > critical {
                return "critical".to_string();
            }
        }
        if let Some(warn) = warn_percent {
            if fill_percent > warn {
                return "warn".to_string();
            }
        }
    }
    "ok".to_string()
}

/// Returns a numeric priority for status (higher = worse).
fn status_priority(status: &str) -> u8 {
    match status {
        "ok" => 0,
        "warn" => 1,
        "critical" => 2,
        _ => 0,
    }
}

/// Converts a priority number back to a status string.
fn priority_to_status(priority: u8) -> String {
    match priority {
        0 => "ok".to_string(),
        1 => "warn".to_string(),
        2 => "critical".to_string(),
        _ => "ok".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn test_health_state_new() {
        let state = HealthState::new(default_config());
        assert_eq!(state.get_io_buffer_kb(), 0);
        assert_eq!(state.get_smaps_buffer_kb(), 0);
        assert_eq!(state.get_smaps_rollup_buffer_kb(), 0);
    }

    #[test]
    fn test_update_buffers() {
        let state = HealthState::new(default_config());
        state.update_io_buffer_kb(100);
        state.update_smaps_buffer_kb(200);
        state.update_smaps_rollup_buffer_kb(50);

        assert_eq!(state.get_io_buffer_kb(), 100);
        assert_eq!(state.get_smaps_buffer_kb(), 200);
        assert_eq!(state.get_smaps_rollup_buffer_kb(), 50);
    }

    #[test]
    fn test_get_health_ok() {
        let state = HealthState::new(default_config());
        // Default config has capacity 256, 512, 256 with 80% warn and 95% critical
        state.update_io_buffer_kb(100); // ~39% of 256, below 80% warn
        state.update_smaps_buffer_kb(200); // ~39% of 512, below 80% warn
        state.update_smaps_rollup_buffer_kb(50); // ~19.5% of 256, below 80% warn

        let response = state.get_health();
        assert_eq!(response.overall_status, "ok");
        assert_eq!(response.buffers.len(), 3);

        for buffer in &response.buffers {
            assert_eq!(buffer.status, "ok");
        }
    }

    #[test]
    fn test_get_health_warn() {
        let state = HealthState::new(default_config());
        // IO buffer: 85% of 256 = 218 KB, above 80% warn but below 95% critical
        state.update_io_buffer_kb(218);
        state.update_smaps_buffer_kb(100);
        state.update_smaps_rollup_buffer_kb(50);

        let response = state.get_health();
        assert_eq!(response.overall_status, "warn");

        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        assert_eq!(io_buffer.status, "warn");
    }

    #[test]
    fn test_get_health_critical() {
        let state = HealthState::new(default_config());
        // IO buffer: 98% of 256 = 251 KB, above 95% critical
        state.update_io_buffer_kb(251);
        state.update_smaps_buffer_kb(100);
        state.update_smaps_rollup_buffer_kb(50);

        let response = state.get_health();
        assert_eq!(response.overall_status, "critical");

        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        assert_eq!(io_buffer.status, "critical");
    }

    #[test]
    fn test_larger_is_better_ok() {
        // When larger_is_better=true, higher fill is good
        let config = AppConfig {
            io_buffer: BufferHealthConfig {
                capacity_kb: 100,
                larger_is_better: true,
                warn_percent: Some(30.0),
                critical_percent: Some(10.0),
            },
            smaps_buffer: BufferHealthConfig::default(),
            smaps_rollup_buffer: BufferHealthConfig::default(),
        };

        let state = HealthState::new(config);
        state.update_io_buffer_kb(50); // 50% fill, above 30% warn

        let response = state.get_health();
        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        assert_eq!(io_buffer.status, "ok");
    }

    #[test]
    fn test_larger_is_better_warn() {
        // When larger_is_better=true, lower fill is bad
        let config = AppConfig {
            io_buffer: BufferHealthConfig {
                capacity_kb: 100,
                larger_is_better: true,
                warn_percent: Some(30.0),
                critical_percent: Some(10.0),
            },
            smaps_buffer: BufferHealthConfig::default(),
            smaps_rollup_buffer: BufferHealthConfig::default(),
        };

        let state = HealthState::new(config);
        state.update_io_buffer_kb(20); // 20% fill, below 30% warn but above 10% critical

        let response = state.get_health();
        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        assert_eq!(io_buffer.status, "warn");
    }

    #[test]
    fn test_larger_is_better_critical() {
        // When larger_is_better=true, very low fill is critical
        let config = AppConfig {
            io_buffer: BufferHealthConfig {
                capacity_kb: 100,
                larger_is_better: true,
                warn_percent: Some(30.0),
                critical_percent: Some(10.0),
            },
            smaps_buffer: BufferHealthConfig::default(),
            smaps_rollup_buffer: BufferHealthConfig::default(),
        };

        let state = HealthState::new(config);
        state.update_io_buffer_kb(5); // 5% fill, below 10% critical

        let response = state.get_health();
        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        assert_eq!(io_buffer.status, "critical");
    }

    #[test]
    fn test_no_thresholds() {
        // Without thresholds, status is always "ok"
        let config = AppConfig {
            io_buffer: BufferHealthConfig {
                capacity_kb: 100,
                larger_is_better: false,
                warn_percent: None,
                critical_percent: None,
            },
            smaps_buffer: BufferHealthConfig::default(),
            smaps_rollup_buffer: BufferHealthConfig::default(),
        };

        let state = HealthState::new(config);
        state.update_io_buffer_kb(99); // 99% fill

        let response = state.get_health();
        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        assert_eq!(io_buffer.status, "ok");
    }

    #[test]
    fn test_fill_percent_calculation() {
        let config = AppConfig::default();
        let state = HealthState::new(config);
        state.update_io_buffer_kb(128); // 50% of 256

        let response = state.get_health();
        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        assert!((io_buffer.fill_percent - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_zero_capacity_protection() {
        // Zero capacity should not cause division by zero
        let config = AppConfig {
            io_buffer: BufferHealthConfig {
                capacity_kb: 0,
                larger_is_better: false,
                warn_percent: Some(80.0),
                critical_percent: Some(95.0),
            },
            smaps_buffer: BufferHealthConfig::default(),
            smaps_rollup_buffer: BufferHealthConfig::default(),
        };

        let state = HealthState::new(config);
        state.update_io_buffer_kb(50);

        // Should not panic
        let response = state.get_health();
        let io_buffer = response
            .buffers
            .iter()
            .find(|b| b.name == "io_buffer_kb")
            .unwrap();
        // With capacity_kb=0, we use max(1) internally, so fill_percent = 50/1*100 = 5000%
        assert!(io_buffer.fill_percent > 100.0);
    }

    #[test]
    fn test_overall_status_worst() {
        let state = HealthState::new(default_config());
        // One buffer warn, one critical, one ok -> overall should be critical
        state.update_io_buffer_kb(251); // critical
        state.update_smaps_buffer_kb(420); // ~82% warn
        state.update_smaps_rollup_buffer_kb(50); // ok

        let response = state.get_health();
        assert_eq!(response.overall_status, "critical");
    }

    #[test]
    fn test_buffer_health_serialization() {
        let health = BufferHealth {
            name: "test_buffer".to_string(),
            capacity_kb: 100,
            current_kb: 50,
            fill_percent: 50.0,
            larger_is_better: false,
            status: "ok".to_string(),
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("test_buffer"));
        assert!(json.contains("50.0"));
    }
}
