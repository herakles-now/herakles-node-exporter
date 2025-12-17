//! Herakles Process Memory Exporter Library
//!
//! This library provides health monitoring functionality for internal buffer
//! fill levels. It is designed to be framework-agnostic, allowing downstream
//! projects to integrate the health monitoring into their preferred web framework.
//!
//! # Features
//!
//! - **Buffer Health Monitoring**: Track fill levels of internal buffers
//! - **Configurable Thresholds**: Set warning and critical thresholds
//! - **Flexible Status Logic**: Support for both "larger is better" and "smaller is better" buffers
//! - **Thread-Safe Updates**: Atomic operations for efficient cross-thread updates
//!
//! # Usage
//!
//! ```rust
//! use herakles_proc_mem_exporter::{AppConfig, HealthState, BufferHealthConfig};
//!
//! // Create configuration
//! let config = AppConfig::default();
//!
//! // Create health state
//! let health_state = HealthState::new(config);
//!
//! // Update buffer values
//! health_state.update_io_buffer_kb(100);
//! health_state.update_smaps_buffer_kb(200);
//! health_state.update_smaps_rollup_buffer_kb(50);
//!
//! // Get health status
//! let response = health_state.get_health();
//! println!("Overall status: {}", response.overall_status);
//!
//! for buffer in &response.buffers {
//!     println!("{}: {}% ({})", buffer.name, buffer.fill_percent, buffer.status);
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `health-actix`: Enables actix-web integration example (see examples/health_server.rs)

pub mod health;
pub mod health_config;

// Re-export main types for convenience
pub use health::{BufferHealth, HealthResponse, HealthState};
pub use health_config::{AppConfig, BufferHealthConfig};
