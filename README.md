# Herakles Process Memory Exporter

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Prometheus](https://img.shields.io/badge/prometheus-exporter-red.svg)](https://prometheus.io)

A high-performance Prometheus exporter for per-process memory and CPU metrics on Linux systems. Provides detailed RSS, PSS, USS memory metrics and CPU usage with intelligent process classification.

## 🚀 Key Features

- **Per-Process Memory Metrics**: RSS (Resident Set Size), PSS (Proportional Set Size), USS (Unique Set Size)
- **CPU Metrics**: Per-process CPU percentage and total CPU time
- **Intelligent Process Classification**: 140+ built-in subgroups for automatic process categorization
- **Top-N Metrics**: Track top memory/CPU consumers per subgroup
- **High Performance**: Background caching, parallel processing, optimized `/proc` parsing
- **Flexible Configuration**: YAML/JSON/TOML config files, CLI overrides, environment variables
- **Production Ready**: Graceful shutdown, health endpoints, comprehensive logging

## 📊 Metrics Overview

### Process Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_proc_mem_rss_bytes` | Resident Set Size per process | pid, name, group, subgroup |
| `herakles_proc_mem_pss_bytes` | Proportional Set Size per process | pid, name, group, subgroup |
| `herakles_proc_mem_uss_bytes` | Unique Set Size per process | pid, name, group, subgroup |
| `herakles_proc_mem_cpu_percent` | CPU usage percentage | pid, name, group, subgroup |
| `herakles_proc_mem_cpu_time_seconds` | Total CPU time used | pid, name, group, subgroup |
| `herakles_proc_mem_group_*_sum` | Aggregated metrics per subgroup | group, subgroup |
| `herakles_proc_mem_top_*` | Top-N metrics per subgroup | group, subgroup, rank, pid, name |

### System Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_memory_total_bytes` | Total system memory in bytes | - |
| `herakles_system_memory_available_bytes` | Available system memory in bytes | - |
| `herakles_system_memory_used_ratio` | Memory used ratio (0.0 to 1.0) | - |
| `herakles_system_cpu_usage_ratio` | CPU usage ratio per core and total | cpu |
| `herakles_system_load1` | System load average over 1 minute | - |
| `herakles_system_load5` | System load average over 5 minutes | - |
| `herakles_system_load15` | System load average over 15 minutes | - |

## 📦 Installation

### From Source (Release Build)

```bash
# Clone the repository
git clone https://github.com/herakles-io/herakles-proc-mem-exporter.git
cd herakles-proc-mem-exporter

# Build release binary
cargo build --release

# Install to /usr/local/bin
sudo cp target/release/herakles-proc-mem-exporter /usr/local/bin/
```

### From Source (Development Build)

```bash
cargo build
./target/debug/herakles-proc-mem-exporter --help
```

### Debian/Ubuntu Package

```bash
# Install cargo-deb if not present
cargo install cargo-deb

# Build .deb package
cargo deb

# Install the package
sudo dpkg -i target/debian/herakles-proc-mem-exporter_*.deb
```

### Docker

```bash
# Build Docker image
docker build -t herakles-proc-mem-exporter .

# Run container
docker run -d \
  --name herakles-exporter \
  -p 9215:9215 \
  -v /proc:/host/proc:ro \
  herakles-proc-mem-exporter
```

## ⚡ Quick Start

```bash
# Start with default settings (port 9215)
herakles-proc-mem-exporter

# Start with custom port
herakles-proc-mem-exporter -p 9216

# Start with config file
herakles-proc-mem-exporter -c /etc/herakles/config.yaml

# Check system requirements
herakles-proc-mem-exporter check --all

# View current configuration
herakles-proc-mem-exporter --show-config
```

## ⚙️ Configuration

### Configuration File Locations

The exporter searches for configuration files in the following order:
1. CLI specified: `-c /path/to/config.yaml`
2. Current directory: `./herakles-proc-mem-exporter.yaml`
3. User config: `~/.config/herakles/config.yaml`
4. System config: `/etc/herakles/config.yaml`

### Minimal Configuration

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 30
```

### Production Configuration

```yaml
# Server settings
port: 9215
bind: "0.0.0.0"

# Performance tuning
cache_ttl: 60
parallelism: 4
io_buffer_kb: 256
smaps_buffer_kb: 512

# Metrics filtering
min_uss_kb: 1024
top_n_subgroup: 5
top_n_others: 20

# Classification
search_mode: "include"
search_groups:
  - db
  - web
  - container

# Feature flags
enable_health: true
enable_telemetry: true
log_level: "info"
```

### High-Performance Configuration

```yaml
port: 9215
bind: "0.0.0.0"

# Aggressive caching
cache_ttl: 120

# Parallel processing
parallelism: 8

# Limit cardinality
top_n_subgroup: 3
top_n_others: 10
min_uss_kb: 10240

# Disable optional features
enable_pprof: false
```

### Generate Configuration Template

```bash
# Generate YAML config with comments
herakles-proc-mem-exporter config --format yaml --commented -o config.yaml

# Generate minimal JSON config
herakles-proc-mem-exporter config --format json -o config.json
```

## 🔒 SSL/TLS Configuration

The exporter supports HTTPS through TLS/SSL configuration.

### Enable TLS via Configuration File

```yaml
# /etc/herakles/config.yaml
port: 9215
bind: "0.0.0.0"

# TLS/SSL Configuration
enable_tls: true
tls_cert_path: "/etc/herakles/certs/server.crt"
tls_key_path: "/etc/herakles/certs/server.key"
```

### Enable TLS via CLI

```bash
herakles-proc-mem-exporter \
  --enable-tls \
  --tls-cert /path/to/server.crt \
  --tls-key /path/to/server.key
```

### Generate Self-Signed Certificate (Testing Only)

```bash
# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout server.key -out server.crt \
  -days 365 -subj "/CN=localhost"

# Start exporter with TLS
herakles-proc-mem-exporter \
  --enable-tls \
  --tls-cert server.crt \
  --tls-key server.key
```

### Docker with TLS

```bash
docker run -d \
  --name herakles-exporter \
  -p 9215:9215 \
  -v /proc:/host/proc:ro \
  -v /path/to/certs:/certs:ro \
  herakles-proc-mem-exporter \
  --enable-tls \
  --tls-cert /certs/server.crt \
  --tls-key /certs/server.key
```

### Prometheus Configuration with HTTPS

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s
    scheme: https
    tls_config:
      # For self-signed certs (testing only):
      # insecure_skip_verify: true
      
      # For private/custom CA certificates:
      ca_file: /path/to/ca.crt
```

## 🏷️ Subgroups System

The exporter automatically classifies processes into groups and subgroups for better organization and analysis.

### Built-in Subgroups

The exporter includes 140+ predefined subgroups covering:

| Group | Subgroups |
|-------|-----------|
| `db` | postgres, mysql, mongodb, oracle, cassandra, redis, clickhouse, etc. |
| `web` | nginx, apache, tomcat, caddy, weblogic, websphere, etc. |
| `container` | docker, containerd, kubelet, podman, crio |
| `monitoring` | prometheus, grafana, alertmanager, zabbix, etc. |
| `backup` | veeam, bacula, netbackup, commvault, etc. |
| `messaging` | kafka, rabbitmq, activemq, nats, etc. |
| `logging` | elasticsearch, logstash, splunk, graylog, etc. |
| `system` | systemd, sshd, cron, postfix, etc. |

### List Available Subgroups

```bash
# List all subgroups
herakles-proc-mem-exporter subgroups

# Filter by group
herakles-proc-mem-exporter subgroups --group db

# Show detailed matching rules
herakles-proc-mem-exporter subgroups --verbose
```

### Custom Subgroups

Create custom subgroups by adding a `subgroups.toml` file:

**Location precedence:**
1. `./subgroups.toml` (current directory)
2. `/etc/herakles/subgroups.toml` (system-wide)

**Example custom subgroups:**

```toml
subgroups = [
  { group = "myapp", subgroup = "api", matches = ["myapp-api", "api-server"] },
  { group = "myapp", subgroup = "worker", matches = ["myapp-worker", "job-processor"] },
  { group = "myapp", subgroup = "frontend", cmdline_matches = ["node.*myapp-frontend"] },
]
```

## 🔌 HTTP Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /metrics` | Prometheus metrics endpoint |
| `GET /health` | Health check with internal stats |
| `GET /config` | Current configuration (HTML) |
| `GET /subgroups` | Loaded subgroups (HTML) |
| `GET /doc` | Documentation in plain text format |

## 📖 Quick Documentation Access

View the complete documentation directly from the command line:

```bash
curl http://localhost:9215/doc
```

This provides a quick reference for:
- Available endpoints
- Metrics overview
- Configuration options
- Example PromQL queries
- CLI commands

### Prometheus Scrape Configuration

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s
```

## 🧪 Testing

### Test Mode

```bash
# Run single test iteration
herakles-proc-mem-exporter test

# Run multiple iterations with verbose output
herakles-proc-mem-exporter test -n 5 --verbose
```

### Generate Synthetic Test Data

```bash
# Generate test data file
herakles-proc-mem-exporter generate-testdata -o testdata.json

# Run exporter with test data
herakles-proc-mem-exporter -t testdata.json
```

### Verify Installation

```bash
# Check system requirements
herakles-proc-mem-exporter check --all

# Validate configuration
herakles-proc-mem-exporter --check-config

# Test metrics endpoint
curl http://localhost:9215/metrics | head -50
```

## 🐳 Docker Compose

```yaml
version: '3.8'

services:
  herakles-exporter:
    image: herakles-proc-mem-exporter:latest
    container_name: herakles-exporter
    ports:
      - "9215:9215"
    volumes:
      - /proc:/host/proc:ro
      - ./config.yaml:/etc/herakles/config.yaml:ro
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    depends_on:
      - herakles-exporter
```

## 🔧 Systemd Service

```ini
[Unit]
Description=Herakles Process Memory Exporter
After=network.target

[Service]
Type=simple
User=prometheus
ExecStart=/usr/bin/herakles-proc-mem-exporter -c /etc/herakles/config.yaml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable herakles-proc-mem-exporter
sudo systemctl start herakles-proc-mem-exporter
sudo systemctl status herakles-proc-mem-exporter
```

## 📈 Example PromQL Queries

```promql
# Top 10 processes by USS memory
topk(10, herakles_proc_mem_uss_bytes)

# Memory usage by group
sum by (group) (herakles_proc_mem_rss_bytes)

# CPU usage by subgroup
sum by (group, subgroup) (herakles_proc_mem_cpu_percent)

# Memory growth rate (per minute)
rate(herakles_proc_mem_rss_bytes[5m]) * 60

# Process count per subgroup
count by (group, subgroup) (herakles_proc_mem_uss_bytes)
```

## 🔧 CLI Reference

```
herakles-proc-mem-exporter [OPTIONS] [COMMAND]

Commands:
  check               Validate configuration and system requirements
  config              Generate configuration files
  test                Test metrics collection
  subgroups           List available process subgroups
  generate-testdata   Generate synthetic test data JSON file

Options:
  -p, --port <PORT>                  HTTP listen port
      --bind <BIND>                  Bind to specific interface/IP
      --log-level <LOG_LEVEL>        Log level [default: info]
  -c, --config <CONFIG>              Config file (YAML/JSON/TOML)
      --no-config                    Disable all config file loading
      --show-config                  Print effective merged config and exit
      --show-user-config             Print loaded user config file and exit
      --config-format <FORMAT>       Output format for --show-config* [default: yaml]
      --check-config                 Validate config and exit
      --cache-ttl <SECONDS>          Cache metrics for N seconds
      --min-uss-kb <KB>              Minimum USS in KB to include process
      --top-n-subgroup <N>           Top-N processes per subgroup
      --top-n-others <N>             Top-N processes for "other" group
  -t, --test-data-file <FILE>        Path to JSON test data file
      --enable-tls                   Enable HTTPS/TLS
      --tls-cert <FILE>              Path to TLS certificate (PEM)
      --tls-key <FILE>               Path to TLS private key (PEM)
  -h, --help                         Print help
  -V, --version                      Print version
```

## 📚 Documentation

For detailed documentation, see the [Wiki](wiki/Home.md):

- [Installation Guide](wiki/Installation.md)
- [Configuration Reference](wiki/Configuration.md)
- [Metrics Overview](wiki/Metrics-Overview.md)
- [Subgroups System](wiki/Subgroups-System.md)
- [Prometheus Integration](wiki/Prometheus-Integration.md)
- [Performance Tuning](wiki/Performance-Tuning.md)
- [Alerting Examples](wiki/Alerting-Examples.md)
- [Troubleshooting](wiki/Troubleshooting.md)
- [Architecture](wiki/Architecture.md)
- [Contributing](wiki/Contributing.md)

## 🔧 Buffer Health Monitoring API

The library provides a health monitoring API for tracking internal buffer fill levels. This allows users to monitor buffer usage and make informed decisions about buffer sizing.

### Usage

```rust
use herakles_proc_mem_exporter::{AppConfig, BufferHealthConfig, HealthState};

// Create configuration with custom thresholds
let config = AppConfig {
    io_buffer: BufferHealthConfig {
        capacity_kb: 256,
        larger_is_better: false,  // Lower fill is better
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
let health_state = HealthState::new(config);

// Update buffer values as they change
health_state.update_io_buffer_kb(100);
health_state.update_smaps_buffer_kb(200);
health_state.update_smaps_rollup_buffer_kb(50);

// Get current health status
let response = health_state.get_health();
println!("Overall status: {}", response.overall_status);

for buffer in &response.buffers {
    println!("{}: {:.1}% ({})", buffer.name, buffer.fill_percent, buffer.status);
}
```

### Feature Flags

- `health-actix`: Enables actix-web integration for exposing health endpoints via HTTP

```bash
# Build with actix-web support
cargo build --features health-actix

# Run the health server example
cargo run --example health_server --features health-actix
```

## 📄 License

This project is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## 👥 Authors

- Michael Moll <proc-mem@herakles.io> - [Herakles IO](https://herakles.io)

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
