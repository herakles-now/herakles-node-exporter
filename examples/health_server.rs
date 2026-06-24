//! Health Server Example using axum
//!
//! This example demonstrates how to expose the HealthState API via an HTTP
//! endpoint using axum.
//!
//! # Running the example
//!
//! ```bash
//! cargo run --example health_server
//! ```
//!
//! Then access the health endpoint:
//! ```bash
//! curl http://localhost:8080/health
//! ```

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use herakles_node_exporter::{AppConfig, BufferHealthConfig, HealthState};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    health_state: Arc<HealthState>,
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.health_state.get_health())
}

async fn index() -> impl IntoResponse {
    "Health Server Example\n\nEndpoints:\n  GET /health - Buffer health status"
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Starting health server example on http://127.0.0.1:8080");

    // Create configuration with custom settings
    let config = AppConfig {
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
    };

    // Create health state
    let health_state = Arc::new(HealthState::new(config));

    // Simulate some buffer usage
    health_state.update_io_buffer_kb(100);
    health_state.update_smaps_buffer_kb(200);
    health_state.update_smaps_rollup_buffer_kb(50);

    println!("Health state initialized with sample buffer values");
    println!("  io_buffer_kb: 100");
    println!("  smaps_buffer_kb: 200");
    println!("  smaps_rollup_buffer_kb: 50");
    println!();
    println!("Access the health endpoint: curl http://127.0.0.1:8080/health");

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health_handler))
        .with_state(AppState { health_state });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await
}
