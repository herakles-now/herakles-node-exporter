//! Health Server Example using actix-web
//!
//! This example demonstrates how to expose the HealthState API via an HTTP
//! endpoint using actix-web.
//!
//! # Running the example
//!
//! ```bash
//! cargo run --example health_server --features health-actix
//! ```
//!
//! Then access the health endpoint:
//! ```bash
//! curl http://localhost:8080/health
//! ```

#[cfg(feature = "health-actix")]
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

#[cfg(feature = "health-actix")]
use herakles_proc_mem_exporter::{AppConfig, BufferHealthConfig, HealthState};

#[cfg(feature = "health-actix")]
use std::sync::Arc;

#[cfg(feature = "health-actix")]
struct AppState {
    health_state: Arc<HealthState>,
}

#[cfg(feature = "health-actix")]
#[get("/health")]
async fn health_handler(data: web::Data<AppState>) -> impl Responder {
    let response = data.health_state.get_health();
    HttpResponse::Ok().json(response)
}

#[cfg(feature = "health-actix")]
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .body("Health Server Example\n\nEndpoints:\n  GET /health - Buffer health status")
}

#[cfg(feature = "health-actix")]
#[actix_web::main]
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

    let app_state = web::Data::new(AppState { health_state });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(index)
            .service(health_handler)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(not(feature = "health-actix"))]
fn main() {
    eprintln!("This example requires the 'health-actix' feature.");
    eprintln!("Run with: cargo run --example health_server --features health-actix");
    std::process::exit(1);
}
