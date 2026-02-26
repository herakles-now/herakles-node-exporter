# Installation Guide

This guide covers all installation methods for the Herakles Process Memory Exporter.

## Prerequisites

- **Linux**: Kernel 4.14+ recommended (for `smaps_rollup` support)
- **Rust**: 1.70+ (for building from source)
- **Permissions**: Read access to `/proc` filesystem

### Check System Requirements

```bash
# After installation, verify system compatibility
herakles-node-exporter check --all
```

## Method 1: From Source

### Release Build

```bash
# Clone the repository
git clone https://github.com/cansp-dev/herakles-node-exporter.git
cd herakles-node-exporter

# Build optimized release binary
cargo build --release

# The binary is located at:
ls -la target/release/herakles-node-exporter

# Install system-wide
sudo cp target/release/herakles-node-exporter /usr/local/bin/
sudo chmod +x /usr/local/bin/herakles-node-exporter

# Verify installation
herakles-node-exporter --version
```

### Development Build

```bash
# Build with debug symbols
cargo build

# Run directly from source
cargo run -- --help

# Run with specific options
cargo run -- -p 9215 --log-level debug
```

## Method 2: Debian/Ubuntu Package

### Build the Package

```bash
# Install cargo-deb if not present
cargo install cargo-deb

# Build .deb package
cargo deb

# The package is created at:
ls -la target/debian/herakles-node-exporter_*.deb
```

### Install the Package

```bash
# Install the .deb package
sudo dpkg -i target/debian/herakles-node-exporter_*.deb

# Or with apt (handles dependencies)
sudo apt install ./target/debian/herakles-node-exporter_*.deb
```

### Package Contents

The Debian package installs:
- `/usr/bin/herakles-node-exporter` - Main binary
- `/etc/herakles-node-exporter/herakles-node-exporter.yaml` - Config file
- `/lib/systemd/system/herakles-node-exporter.service` - Systemd service

## Method 3: Docker

### Build Docker Image

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/herakles-node-exporter /usr/local/bin/
EXPOSE 9215
ENTRYPOINT ["herakles-node-exporter"]
```

```bash
# Build the image
docker build -t herakles-node-exporter:latest .
```

### Run Container

```bash
# Basic run (requires /proc access)
docker run -d \
  --name herakles-exporter \
  -p 9215:9215 \
  -v /proc:/host/proc:ro \
  herakles-node-exporter

# With custom config
docker run -d \
  --name herakles-exporter \
  -p 9215:9215 \
  -v /proc:/host/proc:ro \
  -v $(pwd)/config.yaml:/etc/herakles/config.yaml:ro \
  herakles-node-exporter -c /etc/herakles/config.yaml

# With environment variables
docker run -d \
  --name herakles-exporter \
  -p 9215:9215 \
  -v /proc:/host/proc:ro \
  -e RUST_LOG=info \
  herakles-node-exporter
```

## Method 4: Docker Compose

### Basic Setup

```yaml
# docker-compose.yml
version: '3.8'

services:
  herakles-exporter:
    image: herakles-node-exporter:latest
    build: .
    container_name: herakles-exporter
    ports:
      - "9215:9215"
    volumes:
      - /proc:/host/proc:ro
    restart: unless-stopped
```

### Full Stack with Prometheus & Grafana

```yaml
# docker-compose.yml
version: '3.8'

services:
  herakles-exporter:
    image: herakles-node-exporter:latest
    build: .
    container_name: herakles-exporter
    ports:
      - "9215:9215"
    volumes:
      - /proc:/host/proc:ro
      - ./config.yaml:/etc/herakles/config.yaml:ro
    command: ["-c", "/etc/herakles/config.yaml"]
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    depends_on:
      - herakles-exporter
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    depends_on:
      - prometheus
    restart: unless-stopped

volumes:
  prometheus-data:
  grafana-data:
```

## Systemd Service Setup

### Create Service File

```bash
# Create service file
sudo tee /etc/systemd/system/herakles-node-exporter.service << 'EOF'
[Unit]
Description=Herakles Process Memory Exporter
Documentation=https://github.com/cansp-dev/herakles-node-exporter
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=prometheus
Group=prometheus
ExecStart=/usr/local/bin/herakles-node-exporter -c /etc/herakles/config.yaml
Restart=always
RestartSec=5
TimeoutStopSec=30

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadOnlyPaths=/
ReadWritePaths=/var/log

# Capability to read /proc
CapabilityBoundingSet=CAP_DAC_READ_SEARCH
AmbientCapabilities=CAP_DAC_READ_SEARCH

[Install]
WantedBy=multi-user.target
EOF
```

### Enable and Start Service

```bash
# Create dedicated user
sudo useradd -r -s /sbin/nologin prometheus

# Create config directory
sudo mkdir -p /etc/herakles
sudo chown prometheus:prometheus /etc/herakles

# Create minimal config
sudo tee /etc/herakles/config.yaml << 'EOF'
port: 9215
bind: "0.0.0.0"
cache_ttl: 30
log_level: "info"
EOF

# Reload systemd
sudo systemctl daemon-reload

# Enable and start service
sudo systemctl enable herakles-node-exporter
sudo systemctl start herakles-node-exporter

# Check status
sudo systemctl status herakles-node-exporter
```

## Post-Installation

### 1. Verify System Check

```bash
herakles-node-exporter check --all
```

Expected output:
```
🔍 Herakles Process Memory Exporter - System Check
===================================================

📁 Checking /proc filesystem...
   ✅ /proc filesystem accessible
   ✅ Can read 5 process entries

💾 Checking memory metrics accessibility...
   ✅ smaps_rollup available (fast path)
   ✅ Memory parsing successful: RSS=50MB, PSS=45MB, USS=40MB

⚙️  Checking configuration...
   ✅ Configuration is valid

📊 Checking subgroups configuration...
   ✅ 140 subgroups loaded

📋 Summary:
   ✅ All checks passed - system is ready
```

### 2. Verify Configuration

```bash
# Show effective configuration
herakles-node-exporter --show-config

# Validate configuration
herakles-node-exporter --check-config
```

### 3. Test Metrics Collection

```bash
# Start exporter in foreground
herakles-node-exporter --log-level debug

# In another terminal, fetch metrics
curl http://localhost:9215/metrics | head -50

# Check health endpoint
curl http://localhost:9215/health
```

## Troubleshooting Installation

### Permission Denied

```bash
# Error: Permission denied reading /proc/*/smaps
# Solution: Run with appropriate capabilities
sudo setcap cap_dac_read_search+ep /usr/local/bin/herakles-node-exporter
```

### Port Already in Use

```bash
# Check what's using port 9215
sudo lsof -i :9215

# Use a different port
herakles-node-exporter -p 9216
```

### Rust Build Errors

```bash
# Ensure Rust is up to date
rustup update stable

# Clean and rebuild
cargo clean
cargo build --release
```

### Missing smaps_rollup

```bash
# Check kernel version (4.14+ required for smaps_rollup)
uname -r

# The exporter will fall back to smaps if smaps_rollup is unavailable
# Performance may be reduced on older kernels
```

## Next Steps

- [Configure the exporter](Configuration.md)
- [Set up Prometheus integration](Prometheus-Integration.md)
- [Understand the metrics](Metrics-Overview.md)

## 🔗 Project & Support

Project: https://github.com/cansp-dev/herakles-node-exporter — More info: https://www.herakles.now — Support: exporter@herakles.now
