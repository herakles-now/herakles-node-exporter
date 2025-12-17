//! HTTP endpoint handlers for the exporter.
//!
//! This module provides handlers for all HTTP endpoints:
//! - `/metrics`: Prometheus metrics endpoint
//! - `/health`: Health check endpoint
//! - `/config`: Configuration display endpoint
//! - `/subgroups`: Subgroups display endpoint
//! - `/doc`: Documentation endpoint

pub mod config;
pub mod doc;
pub mod health;
pub mod metrics;
pub mod subgroups;

// Re-export handlers
pub use config::config_handler;
pub use doc::doc_handler;
pub use health::health_handler;
pub use metrics::metrics_handler;
pub use subgroups::subgroups_handler;
