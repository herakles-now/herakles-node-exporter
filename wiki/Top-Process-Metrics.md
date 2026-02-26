# Top Process Metrics

This document describes the top-N process metrics exported by the Herakles Node Exporter. These metrics track the highest resource-consuming processes within each group/subgroup combination, providing visibility into the most impactful processes in your system.

## Overview

The top process metrics system identifies and tracks the top-3 processes by resource consumption within each group/subgroup. These metrics help you quickly identify resource bottlenecks and monitor the most significant processes in your infrastructure.

**Key Features:**
- Tracks top-3 processes per group/subgroup by different resource dimensions (CPU, Memory, Disk I/O, Network I/O)
- Includes both current usage (gauges) and cumulative totals (counters)
- Provides process identification via PID and command name
- Rankings update dynamically as resource usage changes

## CPU Metrics

### herakles_top_cpu_process_usage_ratio

**Type:** Gauge  
**Description:** Top-3 processes by CPU usage within each group/subgroup, expressed as a ratio (0.0 to 1.0)  
**Data Source:** `/proc/[pid]/stat`

This metric shows the CPU usage percentage of the most CPU-intensive processes. A value of 0.5 means the process is using 50% of a single CPU core. Values can exceed 1.0 on multi-core systems.

**Labels:**
- `group` - Classification group (e.g., "db", "web", "container")
- `subgroup` - Classification subgroup (e.g., "postgres", "nginx", "docker")
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`

**Example output:**
```prometheus
# HELP herakles_top_cpu_process_usage_ratio Top-3 processes by CPU usage ratio (0.0 to 1.0) within group/subgroup
# TYPE herakles_top_cpu_process_usage_ratio gauge
herakles_top_cpu_process_usage_ratio{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 0.856
herakles_top_cpu_process_usage_ratio{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres"} 0.432
herakles_top_cpu_process_usage_ratio{group="db",subgroup="postgres",rank="3",pid="1236",comm="postgres"} 0.178
herakles_top_cpu_process_usage_ratio{group="web",subgroup="nginx",rank="1",pid="5678",comm="nginx"} 0.234
```

**Example PromQL Queries:**
```promql
# Find all rank-1 (highest CPU) processes across all subgroups
herakles_top_cpu_process_usage_ratio{rank="1"}

# Identify processes using more than 80% of a CPU core
herakles_top_cpu_process_usage_ratio > 0.8

# CPU usage of top database processes
herakles_top_cpu_process_usage_ratio{group="db"}
```

### herakles_top_cpu_process_seconds_total

**Type:** Counter  
**Description:** Cumulative CPU time (in seconds) for the top-3 CPU-consuming processes within each group/subgroup  
**Data Source:** `/proc/[pid]/stat`

This counter metric tracks the total CPU time consumed by top processes since they started. Useful for identifying long-running processes and analyzing CPU consumption trends over time.

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`
- `mode` - CPU mode: "user" or "system"

**Example output:**
```prometheus
# HELP herakles_top_cpu_process_seconds_total Cumulative CPU time in seconds for top-3 CPU processes within group/subgroup
# TYPE herakles_top_cpu_process_seconds_total counter
herakles_top_cpu_process_seconds_total{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres",mode="user"} 45678.92
herakles_top_cpu_process_seconds_total{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres",mode="system"} 3456.78
herakles_top_cpu_process_seconds_total{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres",mode="user"} 23456.12
herakles_top_cpu_process_seconds_total{group="web",subgroup="nginx",rank="1",pid="5678",comm="nginx",mode="user"} 12345.67
```

**Example PromQL Queries:**
```promql
# CPU time growth rate for top processes (seconds per second)
rate(herakles_top_cpu_process_seconds_total[5m])

# Total CPU time consumed by top-3 database processes
sum(herakles_top_cpu_process_seconds_total{group="db"})

# User vs system time for top nginx process
herakles_top_cpu_process_seconds_total{subgroup="nginx",rank="1"}
```

## Memory Metrics

### herakles_top_mem_process_rss_bytes

**Type:** Gauge  
**Description:** Resident Set Size (RSS) in bytes for the top-3 memory-consuming processes within each group/subgroup  
**Data Source:** `/proc/[pid]/statm`

RSS represents the portion of a process's memory that is held in RAM. This includes all stack and heap memory, shared libraries that are currently in RAM, and memory-mapped files.

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`

**Example output:**
```prometheus
# HELP herakles_top_mem_process_rss_bytes Top-3 processes by Resident Set Size (RSS) within group/subgroup
# TYPE herakles_top_mem_process_rss_bytes gauge
herakles_top_mem_process_rss_bytes{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 2147483648
herakles_top_mem_process_rss_bytes{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres"} 1073741824
herakles_top_mem_process_rss_bytes{group="db",subgroup="postgres",rank="3",pid="1236",comm="postgres"} 536870912
herakles_top_mem_process_rss_bytes{group="container",subgroup="docker",rank="1",pid="9012",comm="dockerd"} 419430400
```

**Example PromQL Queries:**
```promql
# Top memory consumers in GB
herakles_top_mem_process_rss_bytes / 1024 / 1024 / 1024

# Memory usage of rank-1 processes across all subgroups
herakles_top_mem_process_rss_bytes{rank="1"}

# Identify processes using more than 1GB
herakles_top_mem_process_rss_bytes > 1073741824
```

### herakles_top_mem_process_pss_bytes

**Type:** Gauge  
**Description:** Proportional Set Size (PSS) in bytes for the top-3 memory-consuming processes within each group/subgroup  
**Data Source:** `/proc/[pid]/smaps_rollup`

PSS is a more accurate memory accounting metric than RSS. It divides the size of shared memory pages proportionally among the processes sharing them. PSS provides a better estimate of a process's actual memory footprint.

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`

**Example output:**
```prometheus
# HELP herakles_top_mem_process_pss_bytes Top-3 processes by Proportional Set Size (PSS) within group/subgroup
# TYPE herakles_top_mem_process_pss_bytes gauge
herakles_top_mem_process_pss_bytes{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 1932735283
herakles_top_mem_process_pss_bytes{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres"} 966367641
herakles_top_mem_process_pss_bytes{group="db",subgroup="postgres",rank="3",pid="1236",comm="postgres"} 483183820
```

**Example PromQL Queries:**
```promql
# Compare RSS vs PSS for shared memory analysis
herakles_top_mem_process_rss_bytes - herakles_top_mem_process_pss_bytes

# Total PSS for top-3 database processes
sum(herakles_top_mem_process_pss_bytes{group="db"})

# PSS in MB for all rank-1 processes
herakles_top_mem_process_pss_bytes{rank="1"} / 1024 / 1024
```

## Block I/O Metrics

### herakles_top_blkio_process_read_bytes_total

**Type:** Counter  
**Description:** Cumulative bytes read from block devices by the top-3 I/O-intensive processes within each group/subgroup  
**Data Source:** `/proc/[pid]/io`

Tracks the total number of bytes read from block storage devices (disks, SSDs) by processes. This counter includes all read operations regardless of caching.

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`

**Example output:**
```prometheus
# HELP herakles_top_blkio_process_read_bytes_total Cumulative bytes read from block devices by top-3 processes within group/subgroup
# TYPE herakles_top_blkio_process_read_bytes_total counter
herakles_top_blkio_process_read_bytes_total{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 549755813888
herakles_top_blkio_process_read_bytes_total{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres"} 274877906944
herakles_top_blkio_process_read_bytes_total{group="backup",subgroup="bacula",rank="1",pid="3456",comm="bacula-fd"} 1099511627776
```

**Example PromQL Queries:**
```promql
# Disk read rate in bytes per second
rate(herakles_top_blkio_process_read_bytes_total[5m])

# Disk read rate in MB/s
rate(herakles_top_blkio_process_read_bytes_total[5m]) / 1024 / 1024

# Top database read I/O consumers
herakles_top_blkio_process_read_bytes_total{group="db",rank="1"}

# Identify processes with high read rates (> 100 MB/s)
rate(herakles_top_blkio_process_read_bytes_total[1m]) > 104857600
```

### herakles_top_blkio_process_write_bytes_total

**Type:** Counter  
**Description:** Cumulative bytes written to block devices by the top-3 I/O-intensive processes within each group/subgroup  
**Data Source:** `/proc/[pid]/io`

Tracks the total number of bytes written to block storage devices. This is crucial for monitoring write-heavy workloads and identifying processes causing disk wear on SSDs.

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`

**Example output:**
```prometheus
# HELP herakles_top_blkio_process_write_bytes_total Cumulative bytes written to block devices by top-3 processes within group/subgroup
# TYPE herakles_top_blkio_process_write_bytes_total counter
herakles_top_blkio_process_write_bytes_total{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 824633720832
herakles_top_blkio_process_write_bytes_total{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres"} 412316860416
herakles_top_blkio_process_write_bytes_total{group="logging",subgroup="elasticsearch",rank="1",pid="7890",comm="java"} 2199023255552
```

**Example PromQL Queries:**
```promql
# Disk write rate in bytes per second
rate(herakles_top_blkio_process_write_bytes_total[5m])

# Total write I/O across all top processes in GB
sum(herakles_top_blkio_process_write_bytes_total) / 1024 / 1024 / 1024

# Write-heavy processes (> 50 MB/s)
rate(herakles_top_blkio_process_write_bytes_total[1m]) > 52428800

# Compare read vs write I/O
rate(herakles_top_blkio_process_write_bytes_total[5m]) / rate(herakles_top_blkio_process_read_bytes_total[5m])
```

## Network Metrics

These metrics require **eBPF support** to be enabled. See the [eBPF Configuration](../README.md#-ebpf-configuration-optional-advanced-feature) section for setup requirements.

### herakles_top_net_process_rx_bytes_total

**Type:** Counter  
**Description:** Cumulative bytes received over the network by the top-3 network-intensive processes within each group/subgroup  
**Data Source:** eBPF packet capture

Tracks network receive (RX) traffic at the process level using eBPF tracing. This provides visibility into which processes are consuming network bandwidth for incoming data.

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`

**Example output:**
```prometheus
# HELP herakles_top_net_process_rx_bytes_total Cumulative bytes received by top-3 network processes within group/subgroup
# TYPE herakles_top_net_process_rx_bytes_total counter
herakles_top_net_process_rx_bytes_total{group="web",subgroup="nginx",rank="1",pid="5678",comm="nginx"} 10995116277760
herakles_top_net_process_rx_bytes_total{group="web",subgroup="nginx",rank="2",pid="5679",comm="nginx"} 5497558138880
herakles_top_net_process_rx_bytes_total{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 2748779069440
```

**Example PromQL Queries:**
```promql
# Network RX rate in bytes per second
rate(herakles_top_net_process_rx_bytes_total[5m])

# Network RX rate in Mbps (megabits per second)
rate(herakles_top_net_process_rx_bytes_total[5m]) * 8 / 1000000

# Top network receivers across all groups
topk(5, rate(herakles_top_net_process_rx_bytes_total[5m]))

# High-bandwidth receivers (> 100 Mbps)
rate(herakles_top_net_process_rx_bytes_total[1m]) * 8 > 100000000
```

### herakles_top_net_process_tx_bytes_total

**Type:** Counter  
**Description:** Cumulative bytes transmitted over the network by the top-3 network-intensive processes within each group/subgroup  
**Data Source:** eBPF packet capture

Tracks network transmit (TX) traffic at the process level. Essential for monitoring data egress, identifying bandwidth-heavy applications, and understanding network usage patterns.

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position: "1" (highest), "2", "3"
- `pid` - Process ID
- `comm` - Process command name from `/proc/[pid]/comm`

**Example output:**
```prometheus
# HELP herakles_top_net_process_tx_bytes_total Cumulative bytes transmitted by top-3 network processes within group/subgroup
# TYPE herakles_top_net_process_tx_bytes_total counter
herakles_top_net_process_tx_bytes_total{group="web",subgroup="nginx",rank="1",pid="5678",comm="nginx"} 21990232555520
herakles_top_net_process_tx_bytes_total{group="web",subgroup="nginx",rank="2",pid="5679",comm="nginx"} 10995116277760
herakles_top_net_process_tx_bytes_total{group="messaging",subgroup="kafka",rank="1",pid="2345",comm="java"} 8796093022208
```

**Example PromQL Queries:**
```promql
# Network TX rate in bytes per second
rate(herakles_top_net_process_tx_bytes_total[5m])

# Total network bandwidth (RX + TX) per process
rate(herakles_top_net_process_rx_bytes_total[5m]) + rate(herakles_top_net_process_tx_bytes_total[5m])

# Identify asymmetric traffic patterns
rate(herakles_top_net_process_tx_bytes_total[5m]) / rate(herakles_top_net_process_rx_bytes_total[5m])

# Top web servers by outbound traffic
topk(3, rate(herakles_top_net_process_tx_bytes_total{group="web"}[5m]))
```

## Label Reference

All top process metrics share a common label structure:

| Label | Description | Example Values |
|-------|-------------|----------------|
| `group` | High-level process classification | `db`, `web`, `container`, `monitoring`, `backup` |
| `subgroup` | Specific process type within group | `postgres`, `mysql`, `nginx`, `docker`, `prometheus` |
| `rank` | Position in top-N ranking (1 = highest) | `1`, `2`, `3` |
| `pid` | Process ID | `1234`, `5678` |
| `comm` | Process command name from `/proc/[pid]/comm` | `postgres`, `nginx`, `dockerd`, `java` |
| `mode` | CPU mode (CPU metrics only) | `user`, `system` |

## Data Sources

Understanding where metrics come from helps with troubleshooting and performance considerations:

| Data Source | Metrics | Overhead | Privileges Required |
|-------------|---------|----------|---------------------|
| `/proc/[pid]/stat` | CPU usage, CPU time | Very low | Read access to `/proc` |
| `/proc/[pid]/statm` | RSS | Very low | Read access to `/proc` |
| `/proc/[pid]/smaps_rollup` | PSS | Low | Read access to `/proc` |
| `/proc/[pid]/io` | Block I/O bytes | Low | Read access to `/proc` (some kernels require root) |
| eBPF | Network RX/TX | Low-Medium | CAP_BPF + CAP_PERFMON or root |

**Note:** eBPF-based network metrics require:
- Linux kernel ≥ 4.18 with BTF support
- eBPF feature enabled at compile time (`--features ebpf`)
- eBPF feature enabled in configuration
- Required capabilities or root access

## Configuration

Control the behavior of top process metrics through the exporter configuration:

```yaml
# config.yaml

# Number of top processes to track per subgroup (default: 3)
top_n_subgroup: 3

# Number of top processes to track for uncategorized ("other") group
top_n_others: 10

# Minimum memory threshold to consider a process (in KB)
# Processes below this threshold are excluded from rankings
min_uss_kb: 1024

# eBPF network metrics (requires eBPF support)
enable_ebpf: true
enable_ebpf_network: true
enable_ebpf_disk: true
```

### Cardinality Considerations

Top-N metrics can contribute to high cardinality in Prometheus. Consider:

**Per metric cardinality:** `groups × subgroups × top_n × metric_types`

Example calculations:
- 10 subgroups × 3 top processes × 8 metric types = ~240 time series
- 50 subgroups × 3 top processes × 8 metric types = ~1,200 time series
- 100 subgroups × 5 top processes × 8 metric types = ~4,000 time series

**Optimization strategies:**
1. Limit `top_n_subgroup` to 3 (default)
2. Use `search_groups` to focus on specific groups
3. Set `min_uss_kb` to filter out small processes
4. Disable unused groups with `search_mode: "include"`

## Use Cases

### Capacity Planning

Monitor resource consumption trends of top processes:

```promql
# Memory growth rate for top processes
rate(herakles_top_mem_process_rss_bytes{rank="1"}[6h])

# CPU usage trend for top database processes
avg_over_time(herakles_top_cpu_process_usage_ratio{group="db",rank="1"}[24h])
```

### Performance Troubleshooting

Identify resource bottlenecks quickly:

```promql
# Processes with sustained high CPU usage
herakles_top_cpu_process_usage_ratio > 0.8

# Processes with high I/O wait
rate(herakles_top_blkio_process_write_bytes_total[5m]) > 104857600
```

### Resource Allocation

Understand which processes need more resources:

```promql
# Top memory consumers requiring attention
topk(10, herakles_top_mem_process_rss_bytes)

# Network bandwidth distribution
sum by (subgroup) (rate(herakles_top_net_process_tx_bytes_total[5m]))
```

### SLA Monitoring

Track critical process metrics for service level objectives:

```promql
# Database process availability (exists in top-3)
count(herakles_top_mem_process_rss_bytes{subgroup="postgres"})

# Web server performance (CPU efficiency)
avg(herakles_top_cpu_process_usage_ratio{group="web"})
```

## Alerting Examples

### High CPU Usage Alert

```yaml
- alert: HighCPUTopProcess
  expr: herakles_top_cpu_process_usage_ratio > 0.9
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Process {{ $labels.comm }} (PID {{ $labels.pid }}) has high CPU usage"
    description: "CPU usage is {{ $value | humanizePercentage }} in {{ $labels.group }}/{{ $labels.subgroup }}"
```

### Memory Growth Alert

```yaml
- alert: RapidMemoryGrowth
  expr: rate(herakles_top_mem_process_rss_bytes[5m]) * 3600 > 1073741824
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Process {{ $labels.comm }} memory growing rapidly"
    description: "Memory growing at {{ $value | humanize }}B/hour"
```

### Disk I/O Saturation Alert

```yaml
- alert: HighDiskIOProcess
  expr: rate(herakles_top_blkio_process_write_bytes_total[5m]) > 524288000
  for: 5m
  labels:
    severity: info
  annotations:
    summary: "Process {{ $labels.comm }} has high disk write rate"
    description: "Writing {{ $value | humanize }}B/s to disk"
```

### Network Bandwidth Alert

```yaml
- alert: HighNetworkBandwidth
  expr: rate(herakles_top_net_process_tx_bytes_total[5m]) > 104857600
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Process {{ $labels.comm }} using high network bandwidth"
    description: "Transmitting {{ $value | humanize }}B/s"
```

## Comparison with Aggregated Metrics

The exporter provides multiple views of process metrics:

| View | Metrics Pattern | Use Case |
|------|----------------|----------|
| **Per-Process** | `herakles_*_process_*` | Detailed analysis of specific processes |
| **Top-N** | `herakles_top_*_process_*` | Quick identification of resource bottlenecks |
| **Group Aggregates** | `herakles_*_group_*` | High-level resource usage by service type |

**Example workflow:**
1. Use **group aggregates** to identify which service type (e.g., database) is consuming resources
2. Use **top-N metrics** to quickly find the specific high-impact processes
3. Use **per-process metrics** for detailed analysis of identified processes

```promql
# Step 1: Identify high-memory group
topk(5, herakles_mem_group_rss_bytes)

# Step 2: Find top consumers in that group
herakles_top_mem_process_rss_bytes{group="db"}

# Step 3: Detailed analysis of specific process
herakles_mem_process_rss_bytes{pid="1234"}
```

## Performance Considerations

### Metric Collection Overhead

Top-N metrics add minimal overhead because:
- Rankings are computed from data already collected for per-process metrics
- Only top-N processes are exposed (not all processes)
- Ranking is performed in-memory during metric export

### Update Frequency

Rankings update on each scrape based on current resource usage. Processes may move in and out of top-N between scrapes.

### Caching

The exporter caches metric data between scrapes:

```yaml
# config.yaml
cache_ttl: 60  # Cache metrics for 60 seconds
```

This reduces CPU overhead for frequent scrapes while maintaining reasonable freshness.

## Troubleshooting

### Missing eBPF Network Metrics

If `herakles_top_net_process_*` metrics are missing:

1. **Check eBPF is enabled:**
   ```bash
   curl http://localhost:9215/health | grep -i ebpf
   ```

2. **Verify kernel support:**
   ```bash
   uname -r  # Should be >= 4.18
   ls /sys/kernel/btf/vmlinux  # Should exist
   ```

3. **Check permissions:**
   ```bash
   # Run with capabilities
   sudo setcap cap_bpf,cap_perfmon=ep /usr/local/bin/herakles-node-exporter
   ```

4. **Review logs:**
   ```bash
   journalctl -u herakles-node-exporter | grep -i ebpf
   ```

### Rankings Change Frequently

If process rankings are unstable:

1. **Increase scrape interval** to smooth out short-term fluctuations
2. **Use rate() or avg_over_time()** in queries for more stable trends
3. **Focus on rank="1"** which tends to be more stable

### High Cardinality Issues

If Prometheus shows cardinality problems:

1. **Reduce top_n_subgroup:**
   ```yaml
   top_n_subgroup: 2  # Track only top 2 instead of 3
   ```

2. **Filter groups:**
   ```yaml
   search_mode: "include"
   search_groups: ["db", "web"]  # Only track critical groups
   ```

3. **Drop unused metrics in Prometheus:**
   ```yaml
   metric_relabel_configs:
     - source_labels: [__name__]
       regex: 'herakles_top_blkio.*'
       action: drop
   ```

## Related Documentation

- [Metrics Overview](Metrics-Overview.md) - Complete list of all metrics
- [Subgroups System](Subgroups-System.md) - How process classification works
- [Configuration](Configuration.md) - Configuration options
- [Prometheus Integration](Prometheus-Integration.md) - Integration guide
- [Alerting Examples](Alerting-Examples.md) - More alert examples
- [Performance Tuning](Performance-Tuning.md) - Optimization strategies
