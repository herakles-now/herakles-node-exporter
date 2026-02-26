# Metrics Overview

This document describes all metrics exported by the Herakles Process Memory Exporter.

## Per-Process Metrics

These metrics are exported for each monitored process.

### Memory Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_mem_process_rss_bytes` | Gauge | Resident Set Size - Total memory currently in RAM |
| `herakles_mem_process_pss_bytes` | Gauge | Proportional Set Size - Memory accounting for shared pages |
| `herakles_mem_process_uss_bytes` | Gauge | Unique Set Size - Memory unique to this process |

**Labels:**
- `pid` - Process ID
- `name` - Process name (from /proc/pid/comm)
- `group` - Classification group (e.g., "db", "web")
- `subgroup` - Classification subgroup (e.g., "postgres", "nginx")

**Example output:**

```
# HELP herakles_mem_process_rss_bytes Resident Set Size per process in bytes
# TYPE herakles_mem_process_rss_bytes gauge
herakles_mem_process_rss_bytes{pid="1234",name="postgres",group="db",subgroup="postgres"} 524288000
herakles_mem_process_rss_bytes{pid="1235",name="postgres",group="db",subgroup="postgres"} 262144000
herakles_mem_process_rss_bytes{pid="5678",name="nginx",group="web",subgroup="nginx"} 104857600

# HELP herakles_mem_process_pss_bytes Proportional Set Size per process in bytes
# TYPE herakles_mem_process_pss_bytes gauge
herakles_mem_process_pss_bytes{pid="1234",name="postgres",group="db",subgroup="postgres"} 419430400
herakles_mem_process_pss_bytes{pid="1235",name="postgres",group="db",subgroup="postgres"} 209715200

# HELP herakles_mem_process_uss_bytes Unique Set Size per process in bytes
# TYPE herakles_mem_process_uss_bytes gauge
herakles_mem_process_uss_bytes{pid="1234",name="postgres",group="db",subgroup="postgres"} 314572800
herakles_mem_process_uss_bytes{pid="1235",name="postgres",group="db",subgroup="postgres"} 157286400
```

### CPU Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_cpu_process_usage_percent` | Gauge | CPU usage percentage (delta over last scan) |
| `herakles_cpu_process_time_seconds` | Gauge | Total CPU time used since process start |

**Example output:**

```
# HELP herakles_cpu_process_usage_percent CPU usage per process in percent (delta over last scan)
# TYPE herakles_cpu_process_usage_percent gauge
herakles_cpu_process_usage_percent{pid="1234",name="postgres",group="db",subgroup="postgres"} 12.5
herakles_cpu_process_usage_percent{pid="5678",name="nginx",group="web",subgroup="nginx"} 2.3

# HELP herakles_cpu_process_time_seconds Total CPU time used per process
# TYPE herakles_cpu_process_time_seconds gauge
herakles_cpu_process_time_seconds{pid="1234",name="postgres",group="db",subgroup="postgres"} 3456.78
herakles_cpu_process_time_seconds{pid="5678",name="nginx",group="web",subgroup="nginx"} 789.12
```

## Aggregated Metrics per Subgroup

These metrics provide totals for each group/subgroup combination.

### Memory Aggregates

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_mem_group_rss_bytes` | Gauge | Sum of RSS bytes per subgroup |
| `herakles_mem_group_pss_bytes` | Gauge | Sum of PSS bytes per subgroup |
| `herakles_mem_group_uss_bytes` | Gauge | Sum of USS bytes per subgroup |
| `herakles_mem_group_swap_bytes` | Gauge | Sum of swap usage per subgroup |

### CPU Aggregates

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_cpu_group_usage_percent_sum` | Gauge | Sum of CPU percent per subgroup |
| `herakles_cpu_group_time_seconds_sum` | Gauge | Sum of CPU time per subgroup |

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_cpu_group_usage_ratio` | Gauge | CPU usage ratio per subgroup (0.0 to 1.0) |
| `herakles_cpu_group_seconds_total` | Gauge | Total CPU time seconds per subgroup |

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `mode` - CPU mode (user, for cpu_group_seconds_total)

**Example output:**

```
# HELP herakles_mem_group_rss_bytes Sum of RSS bytes per subgroup
# TYPE herakles_mem_group_rss_bytes gauge
herakles_mem_group_rss_bytes{group="db",subgroup="postgres"} 2147483648
herakles_mem_group_rss_bytes{group="db",subgroup="mysql"} 1073741824
herakles_mem_group_rss_bytes{group="web",subgroup="nginx"} 419430400

# HELP herakles_cpu_group_usage_ratio CPU usage ratio per subgroup
# TYPE herakles_cpu_group_usage_ratio gauge
herakles_cpu_group_usage_ratio{group="db",subgroup="postgres"} 0.456
herakles_cpu_group_usage_ratio{group="web",subgroup="nginx"} 0.123

# HELP herakles_cpu_group_seconds_total Total CPU time seconds per subgroup
# TYPE herakles_cpu_group_seconds_total gauge
herakles_cpu_group_seconds_total{group="db",subgroup="postgres",mode="user"} 12345.67
herakles_cpu_group_seconds_total{group="web",subgroup="nginx",mode="user"} 6789.12
```

## Top-N Metrics per Subgroup

These metrics show the top N processes by USS within each subgroup.

### Memory Top-N Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_mem_top_process_rss_bytes` | Gauge | Top-N RSS per subgroup |
| `herakles_mem_top_process_pss_bytes` | Gauge | Top-N PSS per subgroup |
| `herakles_mem_top_process_uss_bytes` | Gauge | Top-N USS per subgroup |

### CPU Top-N Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_cpu_top_process_usage_percent` | Gauge | Top-N CPU percent per subgroup |
| `herakles_cpu_top_process_time_seconds` | Gauge | Top-N CPU time per subgroup |
| `herakles_cpu_top_process_usage_ratio` | Gauge | Top-3 CPU usage ratio per subgroup (0.0 to 1.0) |
| `herakles_cpu_top_process_seconds_total` | Gauge | Top-3 CPU time seconds per subgroup |

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position (1, 2, 3, ...)
- `pid` - Process ID
- `comm` - Process name (from /proc/[pid]/comm) - used in new metrics
- `name` - Process name - used in legacy metrics
- `mode` - CPU mode (user, for cpu_top_process_seconds_total)

**Example output:**

```
# HELP herakles_mem_top_process_uss_bytes Top-N USS per subgroup
# TYPE herakles_mem_top_process_uss_bytes gauge
herakles_mem_top_process_uss_bytes{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 314572800
herakles_mem_top_process_uss_bytes{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres"} 157286400
herakles_mem_top_process_uss_bytes{group="db",subgroup="postgres",rank="3",pid="1236",comm="postgres"} 104857600

# HELP herakles_cpu_top_process_usage_ratio Top-3 CPU usage ratio per subgroup
# TYPE herakles_cpu_top_process_usage_ratio gauge
herakles_cpu_top_process_usage_ratio{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 0.125
herakles_cpu_top_process_usage_ratio{group="web",subgroup="nginx",rank="1",pid="5678",comm="nginx"} 0.023

# HELP herakles_cpu_top_process_seconds_total Top-3 CPU time seconds per subgroup
# TYPE herakles_cpu_top_process_seconds_total gauge
herakles_cpu_top_process_seconds_total{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres",mode="user"} 3456.78
herakles_cpu_top_process_seconds_total{group="web",subgroup="nginx",rank="1",pid="5678",comm="nginx",mode="user"} 789.12
```

## Percentage-of-Subgroup Metrics

These metrics show each top-N process as a percentage of the subgroup total.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_mem_top_process_rss_percent_of_subgroup` | Gauge | RSS as % of subgroup total |
| `herakles_mem_top_process_pss_percent_of_subgroup` | Gauge | PSS as % of subgroup total |
| `herakles_mem_top_process_uss_percent_of_subgroup` | Gauge | USS as % of subgroup total |
| `herakles_cpu_top_process_percent_of_subgroup` | Gauge | CPU time as % of subgroup total |

**Example output:**

```
# HELP herakles_mem_top_process_uss_percent_of_subgroup Top-N USS as percentage of subgroup total
# TYPE herakles_mem_top_process_uss_percent_of_subgroup gauge
herakles_mem_top_process_uss_percent_of_subgroup{group="db",subgroup="postgres",rank="1",pid="1234",comm="postgres"} 54.5
herakles_mem_top_process_uss_percent_of_subgroup{group="db",subgroup="postgres",rank="2",pid="1235",comm="postgres"} 27.3
herakles_mem_top_process_uss_percent_of_subgroup{group="db",subgroup="postgres",rank="3",pid="1236",comm="postgres"} 18.2
```

## System-Wide Metrics

These metrics provide information about overall system resources.

### Memory System Metrics (Renamed & Enhanced)

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_mem_system_total_bytes` | Gauge | Total system memory in bytes |
| `herakles_mem_system_available_bytes` | Gauge | Available system memory in bytes |
| `herakles_mem_system_used_ratio` | Gauge | Memory used ratio (0.0 to 1.0) |
| `herakles_mem_system_cached_bytes` | Gauge | Page cache memory in bytes |
| `herakles_mem_system_buffers_bytes` | Gauge | Buffer cache memory in bytes |
| `herakles_mem_system_swap_used_ratio` | Gauge | Swap used ratio (0.0 to 1.0) |
| `herakles_mem_system_psi_wait_seconds_total` | Gauge | Memory Pressure Stall Information (PSI) total seconds |

### CPU System Metrics (Renamed & Enhanced)

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_cpu_system_usage_ratio` | Gauge | CPU usage ratio per core and total (0.0 to 1.0) |
| `herakles_cpu_system_idle_ratio` | Gauge | CPU idle ratio per core and total (0.0 to 1.0) |
| `herakles_cpu_system_iowait_ratio` | Gauge | CPU IO-wait ratio per core and total (0.0 to 1.0) |
| `herakles_cpu_system_steal_ratio` | Gauge | CPU steal time ratio per core and total (0.0 to 1.0) |
| `herakles_cpu_system_load_1` | Gauge | System load average over 1 minute |
| `herakles_cpu_system_load_5` | Gauge | System load average over 5 minutes |
| `herakles_cpu_system_load_15` | Gauge | System load average over 15 minutes |
| `herakles_cpu_system_psi_wait_seconds_total` | Gauge | CPU Pressure Stall Information (PSI) total seconds |

**Labels:**
- `cpu` - CPU identifier (e.g., "cpu" for total, "cpu0", "cpu1" for individual cores)

**Example output:**

```
# HELP herakles_mem_system_total_bytes Total system memory in bytes
# TYPE herakles_mem_system_total_bytes gauge
herakles_mem_system_total_bytes 16777216000

# HELP herakles_mem_system_cached_bytes Page cache memory in bytes
# TYPE herakles_mem_system_cached_bytes gauge
herakles_mem_system_cached_bytes 4194304000

# HELP herakles_mem_system_swap_used_ratio Swap used ratio
# TYPE herakles_mem_system_swap_used_ratio gauge
herakles_mem_system_swap_used_ratio 0.15

# HELP herakles_cpu_system_usage_ratio CPU usage ratio per core and total
# TYPE herakles_cpu_system_usage_ratio gauge
herakles_cpu_system_usage_ratio{cpu="cpu"} 0.45
herakles_cpu_system_usage_ratio{cpu="cpu0"} 0.60
herakles_cpu_system_usage_ratio{cpu="cpu1"} 0.30

# HELP herakles_cpu_system_idle_ratio CPU idle ratio per core and total
# TYPE herakles_cpu_system_idle_ratio gauge
herakles_cpu_system_idle_ratio{cpu="cpu"} 0.50
herakles_cpu_system_idle_ratio{cpu="cpu0"} 0.35
herakles_cpu_system_idle_ratio{cpu="cpu1"} 0.65

# HELP herakles_cpu_system_load_1 System load average over 1 minute
# TYPE herakles_cpu_system_load_1 gauge
herakles_cpu_system_load_1 1.52

# HELP herakles_cpu_system_psi_wait_seconds_total CPU Pressure Stall Information total seconds
# TYPE herakles_cpu_system_psi_wait_seconds_total gauge
herakles_cpu_system_psi_wait_seconds_total 12.345678
```

## Exporter Internal Metrics

These metrics provide observability into the exporter itself.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_exporter_scrape_duration_seconds` | Gauge | Time spent serving /metrics request |
| `herakles_exporter_processes_total` | Gauge | Number of processes currently exported |
| `herakles_exporter_cache_update_duration_seconds` | Gauge | Time spent updating the cache |
| `herakles_exporter_cache_update_success` | Gauge | Last cache update success (1) or failure (0) |
| `herakles_exporter_cache_updating` | Gauge | Cache update in progress (1) or idle (0) |

**Example output:**

```
# HELP herakles_exporter_scrape_duration_seconds Time spent serving /metrics request
# TYPE herakles_exporter_scrape_duration_seconds gauge
herakles_exporter_scrape_duration_seconds 0.015

# HELP herakles_exporter_processes_total Number of processes currently exported
# TYPE herakles_exporter_processes_total gauge
herakles_exporter_processes_total 156

# HELP herakles_exporter_cache_update_duration_seconds Time spent updating the process metrics cache
# TYPE herakles_exporter_cache_update_duration_seconds gauge
herakles_exporter_cache_update_duration_seconds 0.234

# HELP herakles_exporter_cache_update_success Whether the last cache update was successful
# TYPE herakles_exporter_cache_update_success gauge
herakles_exporter_cache_update_success 1
```

## Label Cardinality Considerations

High cardinality can cause performance issues in Prometheus. Consider these strategies:

### Controlling Cardinality

1. **Limit Top-N metrics:**
   ```yaml
   top_n_subgroup: 3      # Only top 3 per subgroup
   top_n_others: 10       # Limit "other" group
   ```

2. **Filter by USS threshold:**
   ```yaml
   min_uss_kb: 1024       # Skip processes with < 1MB USS
   ```

3. **Use search filters:**
   ```yaml
   search_mode: "include"
   search_groups:
     - db
     - web
   disable_others: true   # Skip unclassified processes
   ```

4. **Disable unused metrics:**
   ```yaml
   enable_rss: false      # If you only need USS
   enable_cpu: false      # If you don't need CPU metrics
   ```

### Estimated Series Count

| Configuration | Approximate Series |
|---------------|-------------------|
| Default (no filtering) | High (depends on process count) |
| Top-N only (3 per subgroup) | ~500-1000 |
| Database focus only | ~50-100 |
| With min_uss_kb: 10240 | Low (major processes only) |

## Example PromQL Queries

### Memory Analysis

```promql
# Top 10 processes by USS
topk(10, herakles_mem_process_uss_bytes)

# Memory usage by group
sum by (group) (herakles_mem_process_rss_bytes)

# Memory usage by subgroup (using aggregated metrics)
herakles_mem_group_rss_bytes

# Percentage of total memory per group
sum by (group) (herakles_mem_process_rss_bytes) 
  / ignoring(group) group_left sum(herakles_mem_process_rss_bytes) * 100
```

### CPU Analysis

```promql
# Top 10 processes by CPU
topk(10, herakles_cpu_process_usage_percent)

# CPU usage by group
sum by (group) (herakles_cpu_process_usage_percent)

# Processes using more than 50% CPU
herakles_cpu_process_usage_percent > 50
```

### Process Discovery

```promql
# Count of processes per group
count by (group) (herakles_mem_process_uss_bytes)

# Count of processes per subgroup
count by (group, subgroup) (herakles_mem_process_uss_bytes)

# All postgres processes
herakles_mem_process_uss_bytes{subgroup="postgres"}
```

### Capacity Planning

```promql
# Memory growth rate (bytes per minute)
rate(herakles_mem_process_rss_bytes[5m]) * 60

# Predict memory usage in 1 hour
herakles_mem_process_rss_bytes + (rate(herakles_mem_process_rss_bytes[1h]) * 3600)
```

### Alerting Queries

```promql
# High memory usage (> 80% of total)
herakles_mem_process_rss_bytes > 0.8 * node_memory_MemTotal_bytes

# Process CPU spike
rate(herakles_cpu_process_time_seconds[5m]) > 0.9

# Unusual process count
abs(count(herakles_mem_process_uss_bytes) - count(herakles_mem_process_uss_bytes offset 1h)) > 10
```

### System Monitoring Queries

```promql
# System memory usage percentage
herakles_mem_system_used_ratio * 100

# Available memory in GB
herakles_mem_system_available_bytes / 1024 / 1024 / 1024

# Average CPU usage across all cores
herakles_cpu_system_usage_ratio{cpu="cpu"}

# Individual core CPU usage
herakles_cpu_system_usage_ratio{cpu=~"cpu[0-9]+"}

# System load normalized by available cores
herakles_cpu_system_load_1 / count(herakles_cpu_system_usage_ratio{cpu=~"cpu[0-9]+"})

# System under high load (load1 > number of cores)
herakles_cpu_system_load_1 > count(herakles_cpu_system_usage_ratio{cpu=~"cpu[0-9]+"})
```

## Next Steps

- [Understand the subgroups system](Subgroups-System.md)
- [Configure Prometheus integration](Prometheus-Integration.md)
- [Set up alerting](Alerting-Examples.md)
