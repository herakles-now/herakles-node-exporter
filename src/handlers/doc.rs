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
GET /details     - Ringbuffer statistics and history (plain text)
                   Query params: ?subgroup=<name>

AVAILABLE METRICS
-----------------
herakles_mem_process_rss_bytes           - Resident Set Size per process
herakles_mem_process_pss_bytes           - Proportional Set Size per process
herakles_mem_process_uss_bytes           - Unique Set Size per process
herakles_cpu_process_usage_percent       - CPU usage per process
herakles_cpu_process_time_seconds        - Total CPU time per process

herakles_group_memory_*                  - Aggregated memory metrics per subgroup
herakles_group_cpu_*                     - Aggregated CPU metrics per subgroup
herakles_mem_top_process_*               - Top-N memory metrics per subgroup
herakles_cpu_top_process_*               - Top-N CPU metrics per subgroup
herakles_system_memory_*                 - System-wide memory metrics
herakles_system_cpu_*                    - System-wide CPU metrics
herakles_system_disk_*                   - System-wide disk metrics
herakles_system_net_*                    - System-wide network metrics
herakles_group_blkio_*                   - Group block I/O metrics
herakles_group_net_*                     - Group network metrics
herakles_exporter_*                      - Internal exporter metrics

CONFIGURATION
-------------
Config file locations (in order):
1. CLI specified: -c /path/to/config.yaml
2. Current directory: ./herakles-node-exporter.yaml
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
herakles-node-exporter                    - Start the exporter
herakles-node-exporter check --all        - Validate system requirements
herakles-node-exporter config -o config.yaml - Generate config file
herakles-node-exporter test               - Test metrics collection
herakles-node-exporter subgroups          - List available subgroups
herakles-node-exporter --help             - Show all CLI options

EXAMPLE USAGE
-------------
# Start exporter
herakles-node-exporter

# Start exporter with TLS
herakles-node-exporter --enable-tls --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem

# View this documentation
curl http://localhost:9215/doc

# Get metrics
curl http://localhost:9215/metrics

# Check health
curl http://localhost:9215/health

EXAMPLE PROMQL QUERIES
----------------------
# Top 10 processes by USS memory
topk(10, herakles_mem_process_uss_bytes)

# Memory usage by group
sum by (group) (herakles_mem_process_rss_bytes)

# CPU usage by subgroup
sum by (group, subgroup) (herakles_cpu_process_usage_percent)

# Process count per subgroup
count by (group, subgroup) (herakles_mem_process_uss_bytes)

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
GitHub: https://github.com/cansp-dev/herakles-node-exporter
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
