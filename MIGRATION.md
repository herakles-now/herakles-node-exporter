# Migration Guide: Metric Naming Updates

This document provides a mapping guide for users upgrading to the new consistent metric naming convention.

## Overview

All Prometheus metrics now follow the convention: **`herakles_<module>_<scope>_<signal>[_<detail>]`**

- **Modules**: `mem` (memory), `cpu` (CPU), `exporter` (internal metrics)
- **Scopes**: `process`, `group`, `top`, `system`
- **Signals**: `rss`, `pss`, `uss`, `usage`, `time`, etc.
- **Details**: `_bytes`, `_seconds`, `_percent`, `_ratio`, etc.

## Metric Name Changes

### Per-Process Metrics

| Old Metric Name | New Metric Name | Notes |
|----------------|-----------------|-------|
| `herakles_proc_mem_rss_bytes` | `herakles_mem_process_rss_bytes` | Resident Set Size per process |
| `herakles_proc_mem_pss_bytes` | `herakles_mem_process_pss_bytes` | Proportional Set Size per process |
| `herakles_proc_mem_uss_bytes` | `herakles_mem_process_uss_bytes` | Unique Set Size per process |
| `herakles_proc_mem_cpu_percent` | `herakles_cpu_process_usage_percent` | CPU usage per process |
| `herakles_proc_mem_cpu_time_seconds` | `herakles_cpu_process_time_seconds` | Total CPU time per process |

### Group Aggregated Metrics

| Old Metric Name | New Metric Name | Notes |
|----------------|-----------------|-------|
| `herakles_mem_group_rss_bytes` | `herakles_mem_group_rss_bytes` | ✓ Already correct |
| `herakles_mem_group_pss_bytes` | `herakles_mem_group_pss_bytes` | ✓ Already correct |
| `herakles_mem_group_uss_bytes` | `herakles_mem_group_uss_bytes` | ✓ Already correct |
| `herakles_mem_group_swap_bytes` | `herakles_mem_group_swap_bytes` | ✓ Already correct |
| `herakles_proc_mem_group_cpu_percent_sum` | `herakles_cpu_group_usage_percent_sum` | CPU percent sum per subgroup |
| `herakles_proc_mem_group_cpu_time_seconds_sum` | `herakles_cpu_group_time_seconds_sum` | CPU time sum per subgroup |

### Top-N Process Metrics

| Old Metric Name | New Metric Name | Notes |
|----------------|-----------------|-------|
| `herakles_mem_top_process_rss_bytes` | `herakles_mem_top_process_rss_bytes` | ✓ Already correct |
| `herakles_mem_top_process_pss_bytes` | `herakles_mem_top_process_pss_bytes` | ✓ Already correct |
| `herakles_mem_top_process_uss_bytes` | `herakles_mem_top_process_uss_bytes` | ✓ Already correct |
| `herakles_proc_mem_top_cpu_percent` | `herakles_cpu_top_process_usage_percent` | Top-N CPU percent |
| `herakles_proc_mem_top_cpu_time_seconds` | `herakles_cpu_top_process_time_seconds` | Top-N CPU time |

### Percentage-of-Subgroup Metrics

| Old Metric Name | New Metric Name | Notes |
|----------------|-----------------|-------|
| `herakles_proc_mem_top_cpu_percent_of_subgroup` | `herakles_cpu_top_process_percent_of_subgroup` | CPU as % of subgroup |
| `herakles_proc_mem_top_rss_percent_of_subgroup` | `herakles_mem_top_process_rss_percent_of_subgroup` | RSS as % of subgroup |
| `herakles_proc_mem_top_pss_percent_of_subgroup` | `herakles_mem_top_process_pss_percent_of_subgroup` | PSS as % of subgroup |
| `herakles_proc_mem_top_uss_percent_of_subgroup` | `herakles_mem_top_process_uss_percent_of_subgroup` | USS as % of subgroup |

### System Metrics

| Old Metric Name | New Metric Name | Notes |
|----------------|-----------------|-------|
| `herakles_mem_system_*` | `herakles_mem_system_*` | ✓ Already correct |
| `herakles_cpu_system_*` | `herakles_cpu_system_*` | ✓ Already correct |

### Internal Exporter Metrics

| Old Metric Name | New Metric Name | Notes |
|----------------|-----------------|-------|
| `herakles_proc_mem_scrape_duration_seconds` | `herakles_exporter_scrape_duration_seconds` | Metrics scrape duration |
| `herakles_proc_mem_processes_total` | `herakles_exporter_processes_total` | Number of processes exported |
| `herakles_proc_mem_cache_update_duration_seconds` | `herakles_exporter_cache_update_duration_seconds` | Cache update duration |
| `herakles_proc_mem_cache_update_success` | `herakles_exporter_cache_update_success` | Cache update success status |
| `herakles_proc_mem_cache_updating` | `herakles_exporter_cache_updating` | Cache update in progress flag |

## Updating Your Queries

### Basic Pattern Replacement

For most cases, you can update your PromQL queries by replacing prefixes:

```promql
# OLD
herakles_proc_mem_rss_bytes
herakles_proc_mem_cpu_percent

# NEW
herakles_mem_process_rss_bytes
herakles_cpu_process_usage_percent
```

### Example Query Migrations

#### Memory Queries

```promql
# Top 10 processes by memory
# OLD: topk(10, herakles_proc_mem_uss_bytes)
# NEW:
topk(10, herakles_mem_process_uss_bytes)

# Memory by group
# OLD: sum by (group) (herakles_proc_mem_rss_bytes)
# NEW:
sum by (group) (herakles_mem_process_rss_bytes)
```

#### CPU Queries

```promql
# Top 10 processes by CPU
# OLD: topk(10, herakles_proc_mem_cpu_percent)
# NEW:
topk(10, herakles_cpu_process_usage_percent)

# CPU by subgroup
# OLD: sum by (group, subgroup) (herakles_proc_mem_cpu_percent)
# NEW:
sum by (group, subgroup) (herakles_cpu_process_usage_percent)
```

#### Process Discovery

```promql
# Count processes per group
# OLD: count by (group) (herakles_proc_mem_uss_bytes)
# NEW:
count by (group) (herakles_mem_process_uss_bytes)
```

## Updating Prometheus Recording Rules

If you have recording rules using the old metric names, update them as follows:

```yaml
# OLD
groups:
  - name: herakles_memory
    rules:
      - record: job:herakles_rss_bytes:sum
        expr: sum(herakles_proc_mem_rss_bytes)

# NEW
groups:
  - name: herakles_memory
    rules:
      - record: job:herakles_rss_bytes:sum
        expr: sum(herakles_mem_process_rss_bytes)
```

## Updating Alerting Rules

Update your alerting rules to use the new metric names:

```yaml
# OLD
groups:
  - name: herakles_alerts
    rules:
      - alert: HighMemoryUsage
        expr: herakles_proc_mem_rss_bytes > 4294967296
        for: 5m

# NEW
groups:
  - name: herakles_alerts
    rules:
      - alert: HighMemoryUsage
        expr: herakles_mem_process_rss_bytes > 4294967296
        for: 5m
```

## Updating Grafana Dashboards

### Manual Update

1. Open your Grafana dashboard
2. Edit each panel
3. Update the metric names in queries
4. Save the dashboard

### Dashboard JSON Update

Alternatively, export your dashboard JSON and use find/replace:

```bash
# Replace per-process metrics
sed -i 's/herakles_proc_mem_rss_bytes/herakles_mem_process_rss_bytes/g' dashboard.json
sed -i 's/herakles_proc_mem_pss_bytes/herakles_mem_process_pss_bytes/g' dashboard.json
sed -i 's/herakles_proc_mem_uss_bytes/herakles_mem_process_uss_bytes/g' dashboard.json
sed -i 's/herakles_proc_mem_cpu_percent/herakles_cpu_process_usage_percent/g' dashboard.json

# Replace group metrics
sed -i 's/herakles_proc_mem_group_cpu_percent_sum/herakles_cpu_group_usage_percent_sum/g' dashboard.json

# Replace exporter metrics
sed -i 's/herakles_proc_mem_scrape_duration_seconds/herakles_exporter_scrape_duration_seconds/g' dashboard.json
sed -i 's/herakles_proc_mem_processes_total/herakles_exporter_processes_total/g' dashboard.json
```

Then import the updated dashboard JSON back into Grafana.

## Verification

After upgrading, verify your metrics are being collected correctly:

```bash
# Check new metric names are present
curl -s http://localhost:9215/metrics | grep -E "herakles_(mem|cpu|exporter)_"

# Verify specific metrics
curl -s http://localhost:9215/metrics | grep "herakles_mem_process_rss_bytes"
curl -s http://localhost:9215/metrics | grep "herakles_cpu_process_usage_percent"
curl -s http://localhost:9215/metrics | grep "herakles_exporter_scrape_duration_seconds"
```

## Support

If you encounter any issues during migration:

1. Check the [Metrics Overview](wiki/Metrics-Overview.md) documentation
2. Open an issue on [GitHub](https://github.com/cansp-dev/herakles-node-exporter/issues)
3. Join our community discussions

## Rollback

If you need to rollback to the old metric names temporarily, use a previous version of the exporter until your queries and dashboards are updated.

## New eBPF-Based Metrics (Optional Feature)

Starting from version 0.1.0 with eBPF support, the following new metrics are available when eBPF is enabled:

### eBPF Process I/O Metrics

These metrics provide per-process I/O tracking via eBPF and are only available when:
- Built with `--features ebpf`
- `enable_ebpf: true` in configuration
- Kernel >= 4.18 with BTF support
- Running with CAP_BPF + CAP_PERFMON capabilities

| Metric Name | Description | Labels |
|------------|-------------|--------|
| `herakles_net_process_bytes_total` | TCP/UDP bytes per process | pid, comm, group, subgroup, direction (rx/tx) |
| `herakles_net_process_packets_total` | TCP/UDP packets per process | pid, comm, group, subgroup, direction (rx/tx) |
| `herakles_net_process_dropped_total` | Dropped packets per process | pid, comm, group, subgroup |
| `herakles_io_process_bytes_total` | Block I/O bytes per process/device | pid, comm, device, group, subgroup, direction (read/write) |
| `herakles_io_process_iops_total` | I/O operations per process/device | pid, comm, device, group, subgroup, direction (read/write) |
| `node_tcp_connections` | TCP connections by state | state |
| `herakles_io_group_read_bytes_total` | Aggregated disk reads per subgroup | group, subgroup |
| `herakles_io_group_write_bytes_total` | Aggregated disk writes per subgroup | group, subgroup |
| `herakles_net_group_rx_bytes_total` | Aggregated network RX per subgroup | group, subgroup |
| `herakles_net_group_tx_bytes_total` | Aggregated network TX per subgroup | group, subgroup |
| `herakles_io_top_process_bytes` | Top-N disk I/O processes | group, subgroup, rank, pid, comm, op (read/write) |
| `herakles_net_top_process_bytes` | Top-N network I/O processes | group, subgroup, rank, pid, comm, dir (rx/tx) |
| `herakles_io_system_psi_wait_seconds_total` | I/O Pressure Stall Information | - |

### Configuration

To enable eBPF metrics, add to your configuration file:

```yaml
# Enable eBPF features
enable_ebpf: true
enable_ebpf_network: true
enable_ebpf_disk: true
enable_tcp_tracking: true
```

### Graceful Degradation

If eBPF cannot be initialized (old kernel, missing BTF, insufficient permissions):
- The exporter logs a warning and continues
- All standard metrics continue to work normally
- eBPF metrics are simply not exported
- No impact on existing functionality

See the README.md for detailed eBPF setup instructions, prerequisites, and troubleshooting.
