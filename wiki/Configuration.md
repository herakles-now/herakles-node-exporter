# Configuration Reference

This document provides a complete reference for all configuration options in the Herakles Process Memory Exporter.

## Configuration Sources and Priority

Configuration values are loaded and merged in the following order (later sources override earlier ones):

1. **Built-in defaults** - Hardcoded default values
2. **System config file** - `/etc/herakles/config.yaml`
3. **User config file** - `~/.config/herakles/config.yaml`
4. **Local config file** - `./herakles-proc-mem-exporter.yaml`
5. **CLI-specified config** - `-c /path/to/config.yaml`
6. **CLI flags** - Command-line arguments override all config files
7. **Environment variables** - (Future support planned)

## Configuration File Locations

The exporter searches for configuration files in the following locations:

| Location | Description |
|----------|-------------|
| `/etc/herakles/config.yaml` | System-wide configuration |
| `~/.config/herakles/config.yaml` | User-specific configuration |
| `./herakles-proc-mem-exporter.yaml` | Local directory configuration |
| CLI `-c /path/to/file` | Explicitly specified configuration |

## Supported Formats

- **YAML** (`.yaml`, `.yml`)
- **JSON** (`.json`)
- **TOML** (`.toml`)

## Complete Configuration Reference

### Server Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `port` | integer | `9215` | HTTP listen port |
| `bind` | string | `"0.0.0.0"` | Bind IP address |

```yaml
port: 9215
bind: "0.0.0.0"
```

### Metrics Collection

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `min_uss_kb` | integer | `0` | Minimum USS in KB to include a process |
| `include_names` | list | `null` | Include only processes matching these names |
| `exclude_names` | list | `null` | Exclude processes matching these names |
| `parallelism` | integer | `null` | Number of parallel threads (null = auto) |
| `max_processes` | integer | `null` | Maximum number of processes to scan |

```yaml
min_uss_kb: 1024          # Only include processes with >= 1MB USS
include_names:
  - postgres
  - nginx
  - java
exclude_names:
  - kworker
  - migration
parallelism: 4            # Use 4 threads for parallel processing
max_processes: 500        # Limit to 500 processes max
```

### Performance Tuning

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `cache_ttl` | integer | `30` | Cache metrics for N seconds |
| `io_buffer_kb` | integer | `256` | Buffer size in KB for generic /proc readers |
| `smaps_buffer_kb` | integer | `512` | Buffer size in KB for smaps parsing |
| `smaps_rollup_buffer_kb` | integer | `256` | Buffer size in KB for smaps_rollup parsing |

```yaml
cache_ttl: 60              # Update cache every 60 seconds
io_buffer_kb: 256
smaps_buffer_kb: 512
smaps_rollup_buffer_kb: 256
```

**Performance Recommendations:**

| System Size | cache_ttl | parallelism | Recommendation |
|-------------|-----------|-------------|----------------|
| Small (<100 processes) | 30s | 2 | Default settings work well |
| Medium (100-500 processes) | 60s | 4 | Increase cache TTL |
| Large (500+ processes) | 120s | 8+ | Aggressive caching recommended |

### Classification / Search Engine

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `search_mode` | string | `null` | Filter mode: "include" or "exclude" |
| `search_groups` | list | `null` | List of group names to include/exclude |
| `search_subgroups` | list | `null` | List of subgroup names to include/exclude |
| `disable_others` | boolean | `false` | Skip "other/unknown" processes completely |
| `top_n_subgroup` | integer | `3` | Top-N processes to export per subgroup |
| `top_n_others` | integer | `10` | Top-N processes for "other" group |

```yaml
# Include mode - only export these groups
search_mode: "include"
search_groups:
  - db
  - web
  - container
search_subgroups:
  - prometheus
  - grafana

# Control cardinality
top_n_subgroup: 5
top_n_others: 20
disable_others: false
```

```yaml
# Exclude mode - export everything except these
search_mode: "exclude"
search_groups:
  - system
  - kernel
```

### Metrics Flags

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_rss` | boolean | `true` | Export RSS (Resident Set Size) metrics |
| `enable_pss` | boolean | `true` | Export PSS (Proportional Set Size) metrics |
| `enable_uss` | boolean | `true` | Export USS (Unique Set Size) metrics |
| `enable_cpu` | boolean | `true` | Export CPU metrics |

```yaml
enable_rss: true
enable_pss: true
enable_uss: true
enable_cpu: true
```

### Feature Flags

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_health` | boolean | `true` | Enable /health endpoint |
| `enable_telemetry` | boolean | `true` | Enable internal exporter_* metrics |
| `enable_default_collectors` | boolean | `true` | Enable default collectors |
| `enable_pprof` | boolean | `false` | Enable /debug/pprof endpoints |

```yaml
enable_health: true
enable_telemetry: true
enable_default_collectors: true
enable_pprof: false        # Enable for debugging only
```

### TLS/SSL Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_tls` | boolean | `false` | Enable HTTPS/TLS |
| `tls_cert_path` | string | `null` | Path to TLS certificate (PEM format) |
| `tls_key_path` | string | `null` | Path to TLS private key (PEM format) |

```yaml
# Enable TLS for secure connections
enable_tls: true
tls_cert_path: "/etc/herakles/certs/server.crt"
tls_key_path: "/etc/herakles/certs/server.key"
```

**TLS CLI Options:**

```bash
herakles-proc-mem-exporter \
  --enable-tls \
  --tls-cert /path/to/server.crt \
  --tls-key /path/to/server.key
```

**Generate Self-Signed Certificate (Testing Only):**

```bash
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout server.key -out server.crt \
  -days 365 -subj "/CN=localhost"
```

**Production TLS Setup:**

For production environments, use certificates from a trusted Certificate Authority (CA):

1. **Obtain certificates** from your CA or use Let's Encrypt
2. **Store certificates** securely with appropriate permissions:
   ```bash
   sudo mkdir -p /etc/herakles/certs
   sudo chmod 700 /etc/herakles/certs
   sudo cp server.crt server.key /etc/herakles/certs/
   sudo chmod 600 /etc/herakles/certs/server.key
   ```
3. **Configure the exporter**:
   ```yaml
   enable_tls: true
   tls_cert_path: "/etc/herakles/certs/server.crt"
   tls_key_path: "/etc/herakles/certs/server.key"
   ```

### Logging

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `log_level` | string | `"info"` | Log level: off, error, warn, info, debug, trace |
| `enable_file_logging` | boolean | `false` | Enable logging to file |
| `log_file` | string | `null` | Log file path (null = stderr) |

```yaml
log_level: "info"
enable_file_logging: true
log_file: "/var/log/herakles/exporter.log"
```

### Test Data

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `test_data_file` | string | `null` | Path to JSON test data file |

```yaml
test_data_file: "/path/to/testdata.json"
```

## Commands to Show/Validate Config

```bash
# Show effective merged configuration
herakles-proc-mem-exporter --show-config

# Show in different formats
herakles-proc-mem-exporter --show-config --config-format yaml
herakles-proc-mem-exporter --show-config --config-format json
herakles-proc-mem-exporter --show-config --config-format toml

# Show only user config file
herakles-proc-mem-exporter --show-user-config

# Validate configuration
herakles-proc-mem-exporter --check-config

# Generate configuration template
herakles-proc-mem-exporter config --format yaml --commented -o config.yaml
```

## Example Configurations

### Minimal Configuration

```yaml
port: 9215
bind: "0.0.0.0"
```

### Standard Production

```yaml
# Server settings
port: 9215
bind: "0.0.0.0"

# Performance
cache_ttl: 60
parallelism: 4

# Metrics filtering
min_uss_kb: 1024
top_n_subgroup: 5
top_n_others: 20

# Logging
log_level: "info"
enable_health: true
enable_telemetry: true
```

### High-Performance (Large Systems)

```yaml
# Server settings
port: 9215
bind: "0.0.0.0"

# Aggressive caching for high process count
cache_ttl: 120
parallelism: 8

# Limit cardinality
min_uss_kb: 10240        # Only processes with >= 10MB USS
top_n_subgroup: 3
top_n_others: 10
max_processes: 1000

# Buffer optimization
smaps_buffer_kb: 1024
smaps_rollup_buffer_kb: 512

# Minimal logging
log_level: "warn"
enable_pprof: false
```

### Development

```yaml
port: 9215
bind: "127.0.0.1"        # Localhost only

cache_ttl: 10            # Fast refresh for testing

log_level: "debug"
enable_pprof: true       # Enable profiling

# No filtering for development
min_uss_kb: 0
top_n_subgroup: 10
top_n_others: 50
```

### Database-Focused Monitoring

```yaml
port: 9215
bind: "0.0.0.0"

# Only monitor database processes
search_mode: "include"
search_groups:
  - db
search_subgroups:
  - postgres
  - mysql
  - mongodb
  - redis
  - elasticsearch

# Detailed metrics for fewer processes
top_n_subgroup: 10
disable_others: true      # Skip non-database processes

cache_ttl: 30
log_level: "info"
```

### Container Host Monitoring

```yaml
port: 9215
bind: "0.0.0.0"

# Focus on container-related processes
search_mode: "include"
search_groups:
  - container
  - monitoring
search_subgroups:
  - kubelet
  - containerd
  - dockerd

# Include all container processes
top_n_subgroup: 20
top_n_others: 50

cache_ttl: 30
```

## Environment Variables

The exporter respects standard Rust logging environment variables:

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Set log level (overrides config) |
| `RUST_BACKTRACE` | Enable backtraces (1 = enabled) |

```bash
RUST_LOG=debug herakles-proc-mem-exporter
```

## Configuration Best Practices

### 1. Start with Defaults

Begin with minimal configuration and add options as needed:

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 30
```

### 2. Control Cardinality

High cardinality can cause Prometheus performance issues:

```yaml
# Limit the number of series
top_n_subgroup: 5        # Only top 5 per subgroup
top_n_others: 10         # Limit "other" processes
min_uss_kb: 1024         # Skip small processes
```

### 3. Use Search Filters

Focus on processes you care about:

```yaml
search_mode: "include"
search_groups:
  - db
  - web
  - container
```

### 4. Adjust Cache TTL

Balance freshness vs. performance:

```yaml
# Low process count (< 100): shorter TTL
cache_ttl: 15

# High process count (> 500): longer TTL
cache_ttl: 120
```

### 5. Enable Debug Mode Temporarily

For troubleshooting:

```yaml
log_level: "debug"
enable_pprof: true
```

## Next Steps

- [Understand the metrics](Metrics-Overview.md)
- [Set up Prometheus integration](Prometheus-Integration.md)
- [Performance tuning guide](Performance-Tuning.md)

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
