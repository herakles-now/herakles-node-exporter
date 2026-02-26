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
- **eBPF-based Per-Process I/O Tracking** (optional): Network and disk I/O per process, TCP connection states
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
| `herakles_group_memory_*` | Aggregated memory metrics per subgroup | group, subgroup |
| `herakles_mem_top_process_*` | Top-N memory metrics per subgroup | group, subgroup, rank, pid, comm |
| `herakles_group_cpu_*` | Aggregated CPU metrics per subgroup | group, subgroup |
| `herakles_cpu_top_process_*` | Top-N CPU metrics per subgroup | group, subgroup, rank, pid, comm |

### Top Process Metrics

Track the highest resource-consuming processes within each group/subgroup. See [Top Process Metrics](wiki/Top-Process-Metrics.md) for detailed documentation.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_top_cpu_process_usage_ratio` | Gauge | Top-3 processes by CPU usage (0.0 to 1.0) |
| `herakles_top_cpu_process_seconds_total` | Counter | Cumulative CPU time for top-3 CPU processes |
| `herakles_top_mem_process_rss_bytes` | Gauge | Top-3 processes by RSS memory |
| `herakles_top_mem_process_pss_bytes` | Gauge | Top-3 processes by PSS memory |
| `herakles_top_blkio_process_read_bytes_total` | Counter | Top-3 processes by disk read bytes |
| `herakles_top_blkio_process_write_bytes_total` | Counter | Top-3 processes by disk write bytes |
| `herakles_top_net_process_rx_bytes_total` | Counter | Top-3 processes by network RX bytes (eBPF) |
| `herakles_top_net_process_tx_bytes_total` | Counter | Top-3 processes by network TX bytes (eBPF) |

### System Memory Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_memory_total_bytes` | Total system memory in bytes | - |
| `herakles_system_memory_available_bytes` | Available system memory in bytes | - |
| `herakles_system_memory_used_ratio` | Memory used ratio (0.0 to 1.0) | - |
| `herakles_system_memory_cached_bytes` | Page cache memory in bytes | - |
| `herakles_system_memory_buffers_bytes` | Buffer cache memory in bytes | - |
| `herakles_system_swap_used_ratio` | Swap used ratio (0.0 to 1.0) | - |
| `herakles_system_memory_psi_wait_seconds_total` | Memory pressure stall total seconds | - |

### System CPU Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_cpu_usage_ratio` | CPU usage ratio per core and total | cpu |
| `herakles_system_cpu_idle_ratio` | CPU idle ratio per core and total | cpu |
| `herakles_system_cpu_iowait_ratio` | CPU IO-wait ratio per core and total | cpu |
| `herakles_system_cpu_steal_ratio` | CPU steal time ratio per core and total | cpu |
| `herakles_system_cpu_load_1` | System load average over 1 minute | - |
| `herakles_system_cpu_load_5` | System load average over 5 minutes | - |
| `herakles_system_cpu_load_15` | System load average over 15 minutes | - |
| `herakles_system_cpu_psi_wait_seconds_total` | CPU pressure stall total seconds | - |

### Disk I/O Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_disk_read_bytes_total` | Total number of bytes read | device |
| `herakles_system_disk_write_bytes_total` | Total number of bytes written | device |
| `herakles_system_disk_io_time_seconds_total` | Total seconds spent doing I/Os | device |
| `herakles_system_disk_queue_depth` | Current I/O queue depth | device |
| `herakles_system_disk_psi_wait_seconds_total` | Disk pressure stall total seconds | - |

### Filesystem Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_filesystem_size_bytes` | Filesystem size in bytes | device, mountpoint, fstype |
| `herakles_system_filesystem_avail_bytes` | Filesystem space available to non-root users | device, mountpoint, fstype |
| `herakles_system_filesystem_files` | Filesystem total file nodes (inodes) | device, mountpoint, fstype |
| `herakles_system_filesystem_files_free` | Filesystem total free file nodes | device, mountpoint, fstype |

### Network Interface Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_net_rx_bytes_total` | Network device bytes received | iface |
| `herakles_system_net_tx_bytes_total` | Network device bytes transmitted | iface |
| `herakles_system_net_rx_errors_total` | Network device receive errors | iface |
| `herakles_system_net_tx_errors_total` | Network device transmit errors | iface |
| `herakles_system_net_drops_total` | Network device drops | iface, direction |

### TCP Connection State Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_tcp_connections_established` | TCP connections in ESTABLISHED state | - |
| `herakles_system_tcp_connections_syn_sent` | TCP connections in SYN_SENT state | - |
| `herakles_system_tcp_connections_syn_recv` | TCP connections in SYN_RECV state | - |
| `herakles_system_tcp_connections_fin_wait1` | TCP connections in FIN_WAIT1 state | - |
| `herakles_system_tcp_connections_fin_wait2` | TCP connections in FIN_WAIT2 state | - |
| `herakles_system_tcp_connections_time_wait` | TCP connections in TIME_WAIT state | - |
| `herakles_system_tcp_connections_close` | TCP connections in CLOSE state | - |
| `herakles_system_tcp_connections_close_wait` | TCP connections in CLOSE_WAIT state | - |
| `herakles_system_tcp_connections_last_ack` | TCP connections in LAST_ACK state | - |
| `herakles_system_tcp_connections_listen` | TCP connections in LISTEN state | - |
| `herakles_system_tcp_connections_closing` | TCP connections in CLOSING state | - |

### Hardware and Host Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_cpu_temp_celsius` | CPU temperature in Celsius | sensor |
| `herakles_system_uptime_seconds` | System uptime in seconds | - |
| `herakles_system_boot_time_seconds` | System boot time as Unix timestamp | - |
| `herakles_system_uname_info` | System uname information (always 1) | sysname, release, version, machine |

### Kernel and Runtime Metrics

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_system_context_switches_total` | Total number of context switches | - |
| `herakles_system_forks_total` | Total number of process forks | - |
| `herakles_system_open_fds` | Number of open file descriptors | state |
| `herakles_system_entropy_bits` | Available entropy in bits | - |

### Group and Subgroup Metrics

Aggregated metrics per process group and subgroup (always available):

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_group_memory_rss_bytes` | Aggregated RSS memory per subgroup | group, subgroup |
| `herakles_group_memory_pss_bytes` | Aggregated PSS memory per subgroup | group, subgroup |
| `herakles_group_memory_swap_bytes` | Aggregated swap usage per subgroup | group, subgroup |
| `herakles_group_cpu_usage_ratio` | Aggregated CPU usage ratio per subgroup | group, subgroup |
| `herakles_group_cpu_seconds_total` | Aggregated CPU time per subgroup and mode | group, subgroup, mode |
| `herakles_group_blkio_read_bytes_total` | Aggregated disk read bytes per subgroup | group, subgroup |
| `herakles_group_blkio_write_bytes_total` | Aggregated disk write bytes per subgroup | group, subgroup |
| `herakles_group_blkio_read_syscalls_total` | Aggregated disk read syscalls per subgroup | group, subgroup |
| `herakles_group_blkio_write_syscalls_total` | Aggregated disk write syscalls per subgroup | group, subgroup |
| `herakles_group_net_rx_bytes_total` | Aggregated network RX bytes per subgroup (eBPF) | group, subgroup |
| `herakles_group_net_tx_bytes_total` | Aggregated network TX bytes per subgroup (eBPF) | group, subgroup |
| `herakles_group_net_connections_total` | Aggregated network connections per subgroup (eBPF) | group, subgroup, proto |

### eBPF Performance Metrics

Self-monitoring metrics for the eBPF subsystem (requires `ebpf` feature):

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_ebpf_events_processed_total` | Total eBPF events processed | - |
| `herakles_ebpf_events_dropped_total` | Total eBPF events dropped | - |
| `herakles_ebpf_maps_count` | Number of active eBPF maps | - |
| `herakles_ebpf_cpu_seconds_total` | CPU time spent in eBPF programs | - |

### eBPF-based Process I/O Metrics (Optional Feature)

When the `ebpf` feature is enabled and eBPF is configured, these additional metrics provide per-process I/O tracking:

| Metric | Description | Labels |
|--------|-------------|--------|
| `herakles_net_process_bytes_total` | TCP/UDP bytes per process from eBPF | pid, comm, group, subgroup, direction (rx/tx) |
| `herakles_net_process_packets_total` | TCP/UDP packets per process from eBPF | pid, comm, group, subgroup, direction (rx/tx) |
| `herakles_net_process_dropped_total` | Dropped packets per process from eBPF | pid, comm, group, subgroup |
| `herakles_io_process_bytes_total` | Block I/O bytes per process/device from eBPF | pid, comm, device, group, subgroup, direction (read/write) |
| `herakles_io_process_iops_total` | I/O operations per process/device from eBPF | pid, comm, device, group, subgroup, direction (read/write) |

**eBPF Requirements:**
- Linux kernel >= 4.18 (for CO-RE eBPF support with BTF)
- BTF (BPF Type Format) support: `/sys/kernel/btf/vmlinux` must exist
- CAP_BPF + CAP_PERFMON capabilities (or root access)
- Build dependencies: clang >= 10, llvm >= 10, libbpf-dev, linux-headers, bpftool
- Compile with `--features ebpf` flag

**Building with eBPF support:**

```bash
# Install required system packages (Ubuntu/Debian)
sudo apt-get install -y clang llvm libbpf-dev linux-headers-$(uname -r) bpftool

# Build with eBPF feature
cargo build --release --features ebpf

# The build process will automatically:
# 1. Generate vmlinux.h from your kernel's BTF information
# 2. Compile the eBPF C programs to BPF bytecode
# 3. Embed the compiled programs into the Rust binary
```

**Configuration:**

eBPF is **enabled by default** in the configuration. To disable it, create a configuration file:

```yaml
# config.yaml
enable_ebpf: false
```

And run with:
```bash
herakles-node-exporter --config config.yaml
```

You can also use the command-line to see the current configuration:
```bash
# Show the default configuration (eBPF enabled)
herakles-node-exporter --show-config --no-config
```

**Graceful Degradation:**

If eBPF initialization fails (missing kernel support, insufficient permissions, or disabled feature), the exporter continues with all standard metrics. eBPF metrics will simply be absent from the output. Check the logs for eBPF status:

```bash
# eBPF successfully initialized:
INFO ✅ eBPF programs loaded and attached successfully
INFO    - Network RX/TX tracking enabled
INFO    - Block I/O tracking enabled
INFO    - TCP state tracking enabled

# eBPF not available:
WARN ⚠️  Failed to initialize eBPF: [reason] - running without eBPF metrics
```

**Note:** When eBPF is not available or fails to initialize, the exporter gracefully continues with all standard metrics.


## 📦 Installation

### Building

#### Standard Build (eBPF enabled by default)

eBPF-based process I/O tracking is now enabled by default:

```bash
# Clone the repository
git clone https://github.com/cansp-dev/herakles-node-exporter.git
cd herakles-node-exporter

# Install build dependencies (Debian/Ubuntu)
sudo apt-get install -y clang llvm libbpf-dev linux-headers-$(uname -r) bpftool

# Build with automatic binary copying (eBPF included automatically)
make release

# Or use the build script
./build.sh --release

# Or use cargo directly (manual copy needed)
cargo build --release

# The binary is automatically copied to binary/herakles-node-exporter
# when using make or build.sh

# Run with required capabilities
sudo setcap cap_bpf,cap_perfmon+ep binary/herakles-node-exporter
./binary/herakles-node-exporter

# Install to /usr/local/bin
sudo cp binary/herakles-node-exporter /usr/local/bin/
```

**System Requirements for eBPF:**
- Linux kernel ≥ 4.18
- BTF support: `/sys/kernel/btf/vmlinux` must exist
- Capabilities: `CAP_BPF` + `CAP_PERFMON` (or root)

**Graceful Degradation:**
If eBPF initialization fails, the exporter continues with standard metrics and logs a warning.

#### Building Without eBPF

To build without eBPF support (smaller binary, no eBPF build dependencies):

```bash
# Using make
make release CARGOFLAGS="--no-default-features"

# Using build script
./build.sh --release --no-default-features

# Using cargo directly
cargo build --release --no-default-features
```

#### Build Options

The project provides three ways to build with automatic binary copying to `binary/`:

1. **Makefile** (recommended for CI/CD):
   ```bash
   make build          # Debug build
   make release        # Release build
   make build CARGOFLAGS="--no-default-features"  # Custom flags
   ```

2. **Build Script** (convenient wrapper):
   ```bash
   ./build.sh          # Debug build
   ./build.sh --release --no-default-features
   ```

3. **Cargo directly** (manual copy needed):
   ```bash
   cargo build --release
   # Manual copy: cp target/release/herakles-node-exporter binary/
   ```

The `binary/` directory is automatically created and excluded from git. The binary is named `herakles-node-exporter` regardless of the build profile.

#### Troubleshooting eBPF Build

If eBPF compilation fails:

1. **Check kernel headers**:
   ```bash
   ls -la /sys/kernel/btf/vmlinux
   sudo apt-get install linux-headers-$(uname -r)
   ```

2. **Check clang version** (needs ≥10):
   ```bash
   clang --version
   ```

3. **Verify bpftool**:
   ```bash
   bpftool version
   ```

4. **Manual vmlinux.h generation**:
   ```bash
   cd src/ebpf/bpf
   bpftool btf dump file /sys/kernel/btf/vmlinux format c > vmlinux.h
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

## 🔧 System-wide Installation

The exporter can be installed system-wide with systemd service integration.

### ⚠️ Important: Running as Root

For full system monitoring, herakles-node-exporter must run as **root** or with specific capabilities:

#### Recommended: Run as Root

```bash
sudo herakles-node-exporter install
```

This ensures the exporter can:
- Read `/proc/<pid>/smaps_rollup` for all processes (including root-owned)
- Access eBPF maps in `/sys/fs/bpf/`
- Monitor all system processes without permission errors

The systemd service is configured to run as root with appropriate security capabilities.

#### Check Requirements

Before starting, verify your system meets all requirements:

```bash
herakles-node-exporter check-requirements --ebpf
```

Expected output:
```
🔍 Checking Runtime Requirements
================================

✅ Running as root (uid=0)
✅ /proc access: Can read all processes
✅ eBPF requirements validated
✅ All requirements met - ready for production!
```

#### Troubleshooting Permission Issues

If you see "Permission denied" errors in logs:

1. **Check effective user**: `ps aux | grep herakles-node-exporter`
2. **Should show**: `root ... herakles-node-exporter`
3. **If not root**: The service will automatically run as root after a fresh install
4. **Check logs**: `journalctl -u herakles-node-exporter -f`

**Expected behavior:**
- Service starts as root, does NOT drop privileges
- Log shows: `ℹ️  User 'herakles' not found - continuing as root`
- All 345+ processes are monitored successfully

### Install

The `install` command sets up a production-ready installation:

```bash
# Install system-wide (requires root)
sudo herakles-node-exporter install

# Install without starting the service
sudo herakles-node-exporter install --no-service

# Force reinstall over existing installation
sudo herakles-node-exporter install --force
```

**Installation includes:**
- Binary at `/opt/herakles/bin/herakles-node-exporter`
- Configuration at `/etc/herakles/herakles-node-exporter.yaml`
- systemd service at `/etc/systemd/system/herakles-node-exporter.service` (runs as root)
- Runtime directories with proper permissions (owned by root)

**After installation:**
```bash
# Check service status
systemctl status herakles-node-exporter

# View logs
journalctl -u herakles-node-exporter -f

# Access metrics
curl http://localhost:9215/metrics
```

### Uninstall

The `uninstall` command cleanly removes the installation:

```bash
# Uninstall (requires root, prompts for confirmation)
sudo herakles-node-exporter uninstall

# Uninstall without confirmation
sudo herakles-node-exporter uninstall --yes
```

**Uninstallation removes:**
- systemd service (stopped and disabled)
- Binary from `/opt/herakles/bin/`
- Configuration from `/etc/herakles/`
- All installation directories (`/opt/herakles/`, `/var/lib/herakles/`, etc.)
- BPF maps from `/sys/fs/bpf/herakles/`

**Note:** No system user is created during installation, so nothing needs to be removed.

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

## 🔬 eBPF Configuration (Optional Advanced Feature)

The exporter supports optional eBPF-based per-process I/O tracking for advanced monitoring scenarios. This feature requires special kernel support and is disabled by default.

### Prerequisites

1. **Linux Kernel**: >= 4.18 with BTF support
2. **BTF**: `/sys/kernel/btf/vmlinux` must exist
3. **Capabilities**: CAP_BPF + CAP_PERFMON (or run as root)
4. **Build Tools** (compile time):
   - clang >= 10
   - llvm >= 10
   - libbpf-dev
   - linux-headers

### Build with eBPF Support

```bash
# Install build dependencies (Ubuntu/Debian)
sudo apt-get install clang llvm libbpf-dev linux-headers-$(uname -r)

# Build with eBPF feature enabled
cargo build --release --features ebpf

# Or for development
cargo build --features ebpf
```

### Enable eBPF via Configuration File

```yaml
# /etc/herakles/config.yaml
port: 9215
bind: "0.0.0.0"

# eBPF Configuration
enable_ebpf: true                  # Master eBPF enable switch
enable_ebpf_network: true          # Enable per-process network I/O tracking
enable_ebpf_disk: true             # Enable per-process disk I/O tracking
enable_tcp_tracking: true          # Enable TCP connection state tracking
```

### Run with Required Capabilities

```bash
# Option 1: Run as root (not recommended for production)
sudo herakles-node-exporter

# Option 2: Grant specific capabilities (recommended)
sudo setcap 'cap_bpf,cap_perfmon=+ep' /usr/local/bin/herakles-node-exporter
herakles-node-exporter

# Option 3: Run in Docker with capabilities
docker run -d \
  --name herakles-exporter \
  --cap-add CAP_BPF \
  --cap-add CAP_PERFMON \
  -p 9215:9215 \
  -v /proc:/host/proc:ro \
  -v /sys/kernel/btf:/sys/kernel/btf:ro \
  herakles-node-exporter --enable-ebpf
```

### Verify eBPF Status

```bash
# Check if BTF is available
ls -l /sys/kernel/btf/vmlinux

# Check kernel version
uname -r

# Start with debug logging to see eBPF initialization
herakles-node-exporter --log-level debug
```

### Graceful Degradation

The exporter is designed to work without eBPF:
- If eBPF initialization fails (old kernel, missing BTF, insufficient permissions), the exporter logs a warning and continues
- All standard metrics (memory, CPU, system-level disk/network) continue to work normally
- eBPF metrics are simply not exported when unavailable
- No impact on performance or reliability of non-eBPF features

### Current Implementation Status

**✅ eBPF Integration Status**: The eBPF integration is **fully implemented**. The following features are active when the `ebpf` feature is compiled in and eBPF initializes successfully:
- Real-time per-process network I/O tracking via `net_stats_map` (bytes RX/TX, packets, drops)
- Real-time per-process block I/O tracking via `blkio_stats_map` (bytes read/write, syscall counts)
- TCP connection state tracking via `tcp_state_map`
- Aggregated I/O and network metrics per group/subgroup
- eBPF performance self-monitoring (`herakles_ebpf_*` metrics)

The eBPF programs are compiled from `src/ebpf/bpf/process_io.bpf.o` and embedded into the binary at build time.

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
rate(herakles_system_disk_read_bytes_total[5m])
rate(herakles_system_disk_write_bytes_total[5m])

# Disk I/O utilization (percentage of time with I/O in progress)
rate(herakles_system_disk_io_time_seconds_total[5m]) * 100

# Current I/O queue depth per device
herakles_system_disk_queue_depth
```

### Filesystem Metrics
```promql
# Filesystem usage percentage
(herakles_system_filesystem_size_bytes - herakles_system_filesystem_avail_bytes) / herakles_system_filesystem_size_bytes * 100

# Filesystem available space in GB
herakles_system_filesystem_avail_bytes / 1024 / 1024 / 1024

# Filesystems with less than 10% available space
(herakles_system_filesystem_avail_bytes / herakles_system_filesystem_size_bytes) < 0.1

# Inode usage percentage
(herakles_system_filesystem_files - herakles_system_filesystem_files_free) / herakles_system_filesystem_files * 100
```

### Network Metrics
```promql
# Network traffic rate in bytes per second
rate(herakles_system_net_rx_bytes_total[5m])
rate(herakles_system_net_tx_bytes_total[5m])

# Network error rate
rate(herakles_system_net_rx_errors_total[5m])
rate(herakles_system_net_tx_errors_total[5m])

# Network drop rate (by direction)
rate(herakles_system_net_drops_total[5m])

# Total network bandwidth usage
sum(rate(herakles_system_net_rx_bytes_total[5m])) + 
  sum(rate(herakles_system_net_tx_bytes_total[5m]))
```

### Group Metrics
```promql
# Memory usage aggregated per subgroup
herakles_group_memory_rss_bytes

# CPU usage aggregated per subgroup
herakles_group_cpu_usage_ratio

# Block I/O per subgroup
rate(herakles_group_blkio_read_bytes_total[5m])
rate(herakles_group_blkio_write_bytes_total[5m])

# Network I/O per subgroup (requires eBPF)
rate(herakles_group_net_rx_bytes_total[5m])
rate(herakles_group_net_tx_bytes_total[5m])
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
      --enable-ebpf                  Enable eBPF-based per-process I/O tracking
      --enable-ebpf-network          Enable eBPF-based per-process network I/O tracking
      --disable-ebpf-network         Disable eBPF-based per-process network I/O tracking
      --enable-ebpf-disk             Enable eBPF-based per-process disk I/O tracking
      --disable-ebpf-disk            Disable eBPF-based per-process disk I/O tracking
      --enable-tcp-tracking          Enable TCP connection state tracking via eBPF
      --disable-tcp-tracking         Disable TCP connection state tracking via eBPF
  -h, --help                         Print help
  -V, --version                      Print version
```

## 🔍 eBPF Troubleshooting

### Common eBPF Issues and Solutions

**1. eBPF fails to initialize**

Check the logs for specific errors:
```bash
# Look for eBPF initialization messages
herakles-node-exporter | grep -i ebpf
```

Common causes:
- **Missing BTF support**: Verify `/sys/kernel/btf/vmlinux` exists
  ```bash
  ls -lh /sys/kernel/btf/vmlinux
  # If missing: rebuild kernel with CONFIG_DEBUG_INFO_BTF=y
  ```

- **Insufficient permissions**: Run with CAP_BPF + CAP_PERFMON or root
  ```bash
  # As root
  sudo herakles-node-exporter --enable-ebpf
  
  # Or with capabilities
  sudo setcap cap_bpf,cap_perfmon=ep /usr/local/bin/herakles-node-exporter
  herakles-node-exporter --enable-ebpf
  ```

- **Old kernel**: Requires Linux >= 4.18 for BTF support
  ```bash
  uname -r
  # Upgrade if kernel version < 4.18
  ```

**2. Build fails with eBPF feature**

Ensure all build dependencies are installed:
```bash
# Ubuntu/Debian
sudo apt-get install -y clang llvm libbpf-dev linux-headers-$(uname -r) bpftool

# Fedora/RHEL
sudo dnf install -y clang llvm libbpf-devel kernel-devel bpftool

# Arch Linux
sudo pacman -S clang llvm libbpf linux-headers bpf
```

**3. eBPF metrics are missing**

Verify eBPF is enabled in configuration:
```yaml
enable_ebpf: true
```

Check the `/health` endpoint for eBPF status:
```bash
curl http://localhost:9215/health | grep -i ebpf
```

**4. Performance issues with eBPF**

Monitor eBPF performance statistics via `/health` endpoint:
- `events_per_sec`: Event processing rate
- `map_usage_percent`: BPF map utilization
- `lost_events`: Events dropped due to buffer overruns

If overhead is too high, consider:
- Disabling specific eBPF features (e.g., `disable-ebpf-network`)
- Reducing `top_n_subgroup` to limit per-process tracking
- Increasing system resources

## 📚 Documentation

For detailed documentation, see the [Wiki](wiki/Home.md):

- [Installation Guide](wiki/Installation.md)
- [Configuration Reference](wiki/Configuration.md)
- [Metrics Overview](wiki/Metrics-Overview.md)
- [Top Process Metrics](wiki/Top-Process-Metrics.md) - **Detailed guide for top-N resource metrics**
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

- Michael Moll <exporter@herakles.now> - [Herakles](https://herakles.now)

## 🔗 Project & Support

Project: https://github.com/cansp-dev/herakles-node-exporter — More info: https://www.herakles.now — Support: exporter@herakles.now
