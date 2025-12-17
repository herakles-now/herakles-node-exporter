//! Documentation endpoint handler.
//!
//! This module provides the `/doc` endpoint handler that displays
//! comprehensive documentation for the exporter.

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use tracing::{debug, instrument};

use crate::handlers::health::FOOTER_TEXT;
use crate::state::SharedState;

/// Handler for the /doc endpoint.
#[instrument(skip(state))]
pub async fn doc_handler(State(state): State<SharedState>) -> impl IntoResponse {
    debug!("Processing /doc request");

    // Track HTTP request
    state.health_stats.record_http_request();

    let version = env!("CARGO_PKG_VERSION");
    let doc = format!(
        r#"HERAKLES PROCESS MEMORY EXPORTER - DOCUMENTATION
================================================

VERSION: {}
DESCRIPTION: Prometheus exporter for per-process RSS/PSS/USS and CPU metrics

HTTP ENDPOINTS
--------------
GET /metrics     - Prometheus metrics endpoint
GET /health      - Health check with internal statistics (plain text)
GET /config      - Current configuration (plain text)
GET /subgroups   - Loaded subgroups overview (plain text)
GET /doc         - This documentation (plain text)

AVAILABLE METRICS
-----------------
herakles_proc_mem_rss_bytes              - Resident Set Size per process
herakles_proc_mem_pss_bytes              - Proportional Set Size per process
herakles_proc_mem_uss_bytes              - Unique Set Size per process
herakles_proc_mem_cpu_percent            - CPU usage per process
herakles_proc_mem_cpu_time_seconds       - Total CPU time per process

herakles_proc_mem_group_*_sum            - Aggregated metrics per subgroup
herakles_proc_mem_top_*                  - Top-N metrics per subgroup

CONFIGURATION
-------------
Config file locations (in order):
1. CLI specified: -c /path/to/config.yaml
2. Current directory: ./herakles-proc-mem-exporter.yaml
3. User config: ~/.config/herakles/config.yaml
4. System config: /etc/herakles/config.yaml

Key configuration options:
- port: HTTP listen port (default: 9215)
- bind: Bind address (default: 0.0.0.0)
- cache_ttl: Cache TTL in seconds (default: 30)
- min_uss_kb: Minimum USS threshold (default: 0)
- top_n_subgroup: Top-N processes per subgroup (default: 3)
- top_n_others: Top-N processes for "other" group (default: 10)

TLS/SSL Configuration:
- enable_tls: Enable HTTPS (default: false)
- tls_cert_path: Path to TLS certificate (PEM format)
- tls_key_path: Path to TLS private key (PEM format)

CLI COMMANDS
------------
herakles-proc-mem-exporter                    - Start the exporter
herakles-proc-mem-exporter check --all        - Validate system requirements
herakles-proc-mem-exporter config -o config.yaml - Generate config file
herakles-proc-mem-exporter test               - Test metrics collection
herakles-proc-mem-exporter subgroups          - List available subgroups
herakles-proc-mem-exporter --help             - Show all CLI options

EXAMPLE USAGE
-------------
# Start exporter
herakles-proc-mem-exporter

# Start exporter with TLS
herakles-proc-mem-exporter --enable-tls --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem

# View this documentation
curl http://localhost:9215/doc

# Get metrics
curl http://localhost:9215/metrics

# Check health
curl http://localhost:9215/health

EXAMPLE PROMQL QUERIES
----------------------
# Top 10 processes by USS memory
topk(10, herakles_proc_mem_uss_bytes)

# Memory usage by group
sum by (group) (herakles_proc_mem_rss_bytes)

# CPU usage by subgroup
sum by (group, subgroup) (herakles_proc_mem_cpu_percent)

# Process count per subgroup
count by (group, subgroup) (herakles_proc_mem_uss_bytes)

PROMETHEUS SCRAPE CONFIG
------------------------
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s

MORE INFORMATION
----------------
GitHub: https://github.com/herakles-io/herakles-proc-mem-exporter
Documentation: See /config and /subgroups endpoints for runtime info

{}
"#,
        version, FOOTER_TEXT
    );

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        doc,
    )
}
