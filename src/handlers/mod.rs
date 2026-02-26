//! HTTP endpoint handlers for the exporter.
//!
//! This module provides handlers for all HTTP endpoints:
//! - `/metrics`: Prometheus metrics endpoint
//! - `/health`: Health check endpoint
//! - `/config`: Configuration display endpoint
//! - `/subgroups`: Subgroups display endpoint
//! - `/doc`: Documentation endpoint
//! - `/details`: Ringbuffer statistics and history endpoint
//! - `/html/*`: HTML endpoints for human-friendly inspection

pub mod config;
pub mod details;
pub mod doc;
pub mod health;
pub mod html;
pub mod metrics;
pub mod root;
pub mod subgroups;

// Re-export handlers
pub use config::config_handler;
pub use details::details_handler;
pub use doc::doc_handler;
pub use health::health_handler;
pub use html::{
    html_config_handler, html_details_handler, html_docs_handler, html_health_handler,
    html_index_handler, html_subgroups_handler,
};
pub use metrics::metrics_handler;
pub use root::root_handler;
pub use subgroups::subgroups_handler;
