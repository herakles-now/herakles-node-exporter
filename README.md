# Herakles Node Exporter

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Prometheus](https://img.shields.io/badge/prometheus-exporter-red.svg)](https://prometheus.io)

A high-performance Prometheus exporter for comprehensive Linux system monitoring. Provides detailed per-process memory and CPU metrics, system-wide resource metrics, disk I/O statistics, filesystem usage, and network interface statistics with intelligent process classification.

## 🚀 Key Features

- **Per-Process Memory Metrics**: RSS (Resident Set Size), PSS (Proportional Set Size), USS (Unique Set Size)
- **CPU Metrics**: Per-process CPU percentage and total CPU time
- **System Metrics**: Memory, CPU, load averages, and pressure stall information (PSI)
- **Disk I/O Metrics**: Read/write operations, bytes transferred, I/O time statistics per device
- **Filesystem Metrics**: Size, available space, inode statistics per mount point
- **Network Metrics**: Bytes, packets, errors, and drops per network interface
- **Intelligent Process Classification**: 140+ built-in subgroups for automatic process categorization
- **Top-N Metrics**: Track top memory/CPU consumers per subgroup
- **High Performance**: Background caching, parallel processing, optimized `/proc` parsing
- **Flexible Configuration**: YAML/JSON/TOML config files, CLI overrides, environment variables
- **Production Ready**: Graceful shutdown, health endpoints, comprehensive logging

## 📊 Metrics Overview

### Process Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_mem_process_rss_bytes` | Resident Set Size per process | pid, name, group, subgroup |
| `herakles_mem_process_pss_bytes` | Proportional Set Size per process | pid, name, group, subgroup |
| `herakles_mem_process_uss_bytes` | Unique Set Size per process | pid, name, group, subgroup |
| `herakles_cpu_process_usage_percent` | CPU usage percentage | pid, name, group, subgroup |
| `herakles_cpu_process_time_seconds` | Total CPU time used | pid, name, group, subgroup |
| `herakles_mem_group_*` | Aggregated memory metrics per subgroup | group, subgroup |
| `herakles_mem_top_process_*` | Top-N memory metrics per subgroup | group, subgroup, rank, pid, comm |
| `herakles_cpu_group_*` | Aggregated CPU metrics per subgroup | group, subgroup |
| `herakles_cpu_top_process_*` | Top-N CPU metrics per subgroup | group, subgroup, rank, pid, comm |

### System Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_mem_system_total_bytes` | Total system memory in bytes | - |
| `herakles_mem_system_available_bytes` | Available system memory in bytes | - |
| `herakles_mem_system_used_ratio` | Memory used ratio (0.0 to 1.0) | - |
| `herakles_mem_system_cached_bytes` | Page cache memory in bytes | - |
| `herakles_mem_system_buffers_bytes` | Buffer cache memory in bytes | - |
| `herakles_mem_system_swap_used_ratio` | Swap used ratio (0.0 to 1.0) | - |
| `herakles_mem_system_psi_wait_seconds_total` | Memory pressure stall total seconds | - |
| `herakles_mem_group_swap_bytes` | Swap usage per subgroup | group, subgroup |
| `herakles_cpu_system_usage_ratio` | CPU usage ratio per core and total | cpu |
| `herakles_cpu_system_idle_ratio` | CPU idle ratio per core and total | cpu |
| `herakles_cpu_system_iowait_ratio` | CPU IO-wait ratio per core and total | cpu |
| `herakles_cpu_system_steal_ratio` | CPU steal time ratio per core and total | cpu |
| `herakles_cpu_system_load_1` | System load average over 1 minute | - |
| `herakles_cpu_system_load_5` | System load average over 5 minutes | - |
| `herakles_cpu_system_load_15` | System load average over 15 minutes | - |
| `herakles_cpu_system_psi_wait_seconds_total` | CPU pressure stall total seconds | - |

### Disk I/O Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_disk_reads_completed_total` | Total number of reads completed successfully | device |
| `herakles_disk_reads_merged_total` | Total number of reads merged | device |
| `herakles_disk_read_bytes_total` | Total number of bytes read successfully | device |
| `herakles_disk_read_time_seconds_total` | Total seconds spent reading | device |
| `herakles_disk_writes_completed_total` | Total number of writes completed successfully | device |
| `herakles_disk_writes_merged_total` | Total number of writes merged | device |
| `herakles_disk_written_bytes_total` | Total number of bytes written successfully | device |
| `herakles_disk_write_time_seconds_total` | Total seconds spent writing | device |
| `herakles_disk_io_now` | Number of I/Os currently in progress | device |
| `herakles_disk_io_time_seconds_total` | Total seconds spent doing I/Os | device |
| `herakles_disk_io_time_weighted_seconds_total` | Weighted seconds spent doing I/Os | device |

### Filesystem Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_filesystem_size_bytes` | Filesystem size in bytes | device, mountpoint, fstype |
| `herakles_filesystem_free_bytes` | Filesystem free space in bytes | device, mountpoint, fstype |
| `herakles_filesystem_avail_bytes` | Filesystem space available to non-root users | device, mountpoint, fstype |
| `herakles_filesystem_files` | Filesystem total file nodes (inodes) | device, mountpoint, fstype |
| `herakles_filesystem_files_free` | Filesystem total free file nodes | device, mountpoint, fstype |

### Network Interface Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_network_receive_bytes_total` | Network device bytes received | device |
| `herakles_network_receive_packets_total` | Network device packets received | device |
| `herakles_network_receive_errs_total` | Network device receive errors | device |
| `herakles_network_receive_drop_total` | Network device receive drops | device |
| `herakles_network_transmit_bytes_total` | Network device bytes transmitted | device |
| `herakles_network_transmit_packets_total` | Network device packets transmitted | device |
| `herakles_network_transmit_errs_total` | Network device transmit errors | device |
| `herakles_network_transmit_drop_total` | Network device transmit drops | device |


## 📦 Installation

### From Source (Release Build)

```bash
# Clone the repository
git clone https://github.com/cansp-dev/herakles-node-exporter.git
cd herakles-node-exporter

# Build release binary
cargo build --release

# Install to /usr/local/bin
sudo cp target/release/herakles-node-exporter /usr/local/bin/
```

### From Source (Development Build)

```bash
cargo build
./target/debug/herakles-node-exporter --help
```

### Debian/Ubuntu Package

```bash
# Install cargo-deb if not present
cargo install cargo-deb

# Build .deb package
cargo deb

# Install the package
sudo dpkg -i target/debian/herakles-node-exporter_*.deb
```

### Docker

```bash
# Build Docker image
docker build -t herakles-node-exporter .

# Run container
docker run -d \
  --name herakles-exporter \
  -p 9215:9215 \
  -v /proc:/host/proc:ro \
  herakles-node-exporter
```

## ⚡ Quick Start

```bash
# Start with default settings (port 9215)
herakles-node-exporter

# Start with custom port
herakles-node-exporter -p 9216

# Start with config file
herakles-node-exporter -c /etc/herakles/config.yaml

# Check system requirements
herakles-node-exporter check --all

# View current configuration
herakles-node-exporter --show-config
```

## ⚙️ Configuration

### Configuration File Locations

The exporter searches for configuration files in the following order:
1. CLI specified: `-c /path/to/config.yaml`
2. Current directory: `./herakles-node-exporter.yaml`
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
herakles-node-exporter config --format yaml --commented -o config.yaml

# Generate minimal JSON config
herakles-node-exporter config --format json -o config.json
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
herakles-node-exporter \
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
herakles-node-exporter \
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
  herakles-node-exporter \
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
herakles-node-exporter subgroups

# Filter by group
herakles-node-exporter subgroups --group db

# Show detailed matching rules
herakles-node-exporter subgroups --verbose
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
herakles-node-exporter test

# Run multiple iterations with verbose output
herakles-node-exporter test -n 5 --verbose
```

### Generate Synthetic Test Data

```bash
# Generate test data file
herakles-node-exporter generate-testdata -o testdata.json

# Run exporter with test data
herakles-node-exporter -t testdata.json
```

### Verify Installation

```bash
# Check system requirements
herakles-node-exporter check --all

# Validate configuration
herakles-node-exporter --check-config

# Test metrics endpoint
curl http://localhost:9215/metrics | head -50
```

## 🐳 Docker Compose

```yaml
version: '3.8'

services:
  herakles-exporter:
    image: herakles-node-exporter:latest
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
ExecStart=/usr/bin/herakles-node-exporter -c /etc/herakles/config.yaml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable herakles-node-exporter
sudo systemctl start herakles-node-exporter
sudo systemctl status herakles-node-exporter
```

## 📈 Example PromQL Queries

### Process Metrics
```promql
# Top 10 processes by USS memory
topk(10, herakles_mem_process_uss_bytes)

# Memory usage by group
sum by (group) (herakles_mem_process_rss_bytes)

# CPU usage by subgroup
sum by (group, subgroup) (herakles_cpu_process_usage_percent)

# Memory growth rate (per minute)
rate(herakles_mem_process_rss_bytes[5m]) * 60

# Process count per subgroup
count by (group, subgroup) (herakles_mem_process_uss_bytes)
```

### Disk I/O Metrics
```promql
# Disk read/write rate in bytes per second
rate(herakles_disk_read_bytes_total[5m])
rate(herakles_disk_written_bytes_total[5m])

# Disk I/O operations per second
rate(herakles_disk_reads_completed_total[5m])
rate(herakles_disk_writes_completed_total[5m])

# Disk I/O utilization (percentage of time with I/O in progress)
rate(herakles_disk_io_time_seconds_total[5m]) * 100

# Average I/O wait time
rate(herakles_disk_io_time_weighted_seconds_total[5m]) / 
  (rate(herakles_disk_reads_completed_total[5m]) + rate(herakles_disk_writes_completed_total[5m]))
```

### Filesystem Metrics
```promql
# Filesystem usage percentage
(herakles_filesystem_size_bytes - herakles_filesystem_free_bytes) / herakles_filesystem_size_bytes * 100

# Filesystem available space in GB
herakles_filesystem_avail_bytes / 1024 / 1024 / 1024

# Filesystems with less than 10% free space
(herakles_filesystem_free_bytes / herakles_filesystem_size_bytes) < 0.1

# Inode usage percentage
(herakles_filesystem_files - herakles_filesystem_files_free) / herakles_filesystem_files * 100
```

### Network Metrics
```promql
# Network traffic rate in bytes per second
rate(herakles_network_receive_bytes_total[5m])
rate(herakles_network_transmit_bytes_total[5m])

# Network packet rate
rate(herakles_network_receive_packets_total[5m])
rate(herakles_network_transmit_packets_total[5m])

# Network error rate
rate(herakles_network_receive_errs_total[5m])
rate(herakles_network_transmit_errs_total[5m])

# Total network bandwidth usage
sum(rate(herakles_network_receive_bytes_total[5m])) + 
  sum(rate(herakles_network_transmit_bytes_total[5m]))
```


## 🔧 CLI Reference

```
herakles-node-exporter [OPTIONS] [COMMAND]

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
use herakles_node_exporter::{AppConfig, BufferHealthConfig, HealthState};

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

Project: https://github.com/cansp-dev/herakles-node-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
