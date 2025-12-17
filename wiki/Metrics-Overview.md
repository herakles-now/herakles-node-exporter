# Metrics Overview

This document describes all metrics exported by the Herakles Process Memory Exporter.

## Per-Process Metrics

These metrics are exported for each monitored process.

### Memory Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_proc_mem_rss_bytes` | Gauge | Resident Set Size - Total memory currently in RAM |
| `herakles_proc_mem_pss_bytes` | Gauge | Proportional Set Size - Memory accounting for shared pages |
| `herakles_proc_mem_uss_bytes` | Gauge | Unique Set Size - Memory unique to this process |

**Labels:**
- `pid` - Process ID
- `name` - Process name (from /proc/pid/comm)
- `group` - Classification group (e.g., "db", "web")
- `subgroup` - Classification subgroup (e.g., "postgres", "nginx")

**Example output:**

```
# HELP herakles_proc_mem_rss_bytes Resident Set Size per process in bytes
# TYPE herakles_proc_mem_rss_bytes gauge
herakles_proc_mem_rss_bytes{pid="1234",name="postgres",group="db",subgroup="postgres"} 524288000
herakles_proc_mem_rss_bytes{pid="1235",name="postgres",group="db",subgroup="postgres"} 262144000
herakles_proc_mem_rss_bytes{pid="5678",name="nginx",group="web",subgroup="nginx"} 104857600

# HELP herakles_proc_mem_pss_bytes Proportional Set Size per process in bytes
# TYPE herakles_proc_mem_pss_bytes gauge
herakles_proc_mem_pss_bytes{pid="1234",name="postgres",group="db",subgroup="postgres"} 419430400
herakles_proc_mem_pss_bytes{pid="1235",name="postgres",group="db",subgroup="postgres"} 209715200

# HELP herakles_proc_mem_uss_bytes Unique Set Size per process in bytes
# TYPE herakles_proc_mem_uss_bytes gauge
herakles_proc_mem_uss_bytes{pid="1234",name="postgres",group="db",subgroup="postgres"} 314572800
herakles_proc_mem_uss_bytes{pid="1235",name="postgres",group="db",subgroup="postgres"} 157286400
```

### CPU Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_proc_mem_cpu_percent` | Gauge | CPU usage percentage (delta over last scan) |
| `herakles_proc_mem_cpu_time_seconds` | Gauge | Total CPU time used since process start |

**Example output:**

```
# HELP herakles_proc_mem_cpu_percent CPU usage per process in percent (delta over last scan)
# TYPE herakles_proc_mem_cpu_percent gauge
herakles_proc_mem_cpu_percent{pid="1234",name="postgres",group="db",subgroup="postgres"} 12.5
herakles_proc_mem_cpu_percent{pid="5678",name="nginx",group="web",subgroup="nginx"} 2.3

# HELP herakles_proc_mem_cpu_time_seconds Total CPU time used per process
# TYPE herakles_proc_mem_cpu_time_seconds gauge
herakles_proc_mem_cpu_time_seconds{pid="1234",name="postgres",group="db",subgroup="postgres"} 3456.78
herakles_proc_mem_cpu_time_seconds{pid="5678",name="nginx",group="web",subgroup="nginx"} 789.12
```

## Aggregated Metrics per Subgroup

These metrics provide totals for each group/subgroup combination.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_proc_mem_group_rss_bytes_sum` | Gauge | Sum of RSS bytes per subgroup |
| `herakles_proc_mem_group_pss_bytes_sum` | Gauge | Sum of PSS bytes per subgroup |
| `herakles_proc_mem_group_uss_bytes_sum` | Gauge | Sum of USS bytes per subgroup |
| `herakles_proc_mem_group_cpu_percent_sum` | Gauge | Sum of CPU percent per subgroup |
| `herakles_proc_mem_group_cpu_time_seconds_sum` | Gauge | Sum of CPU time per subgroup |

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup

**Example output:**

```
# HELP herakles_proc_mem_group_rss_bytes_sum Sum of RSS bytes per subgroup
# TYPE herakles_proc_mem_group_rss_bytes_sum gauge
herakles_proc_mem_group_rss_bytes_sum{group="db",subgroup="postgres"} 2147483648
herakles_proc_mem_group_rss_bytes_sum{group="db",subgroup="mysql"} 1073741824
herakles_proc_mem_group_rss_bytes_sum{group="web",subgroup="nginx"} 419430400

# HELP herakles_proc_mem_group_cpu_percent_sum Sum of CPU percent per subgroup
# TYPE herakles_proc_mem_group_cpu_percent_sum gauge
herakles_proc_mem_group_cpu_percent_sum{group="db",subgroup="postgres"} 45.6
herakles_proc_mem_group_cpu_percent_sum{group="web",subgroup="nginx"} 12.3
```

## Top-N Metrics per Subgroup

These metrics show the top N processes by USS within each subgroup.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_proc_mem_top_rss_bytes` | Gauge | Top-N RSS per subgroup |
| `herakles_proc_mem_top_pss_bytes` | Gauge | Top-N PSS per subgroup |
| `herakles_proc_mem_top_uss_bytes` | Gauge | Top-N USS per subgroup |
| `herakles_proc_mem_top_cpu_percent` | Gauge | Top-N CPU percent per subgroup |
| `herakles_proc_mem_top_cpu_time_seconds` | Gauge | Top-N CPU time per subgroup |

**Labels:**
- `group` - Classification group
- `subgroup` - Classification subgroup
- `rank` - Ranking position (1, 2, 3, ...)
- `pid` - Process ID
- `name` - Process name

**Example output:**

```
# HELP herakles_proc_mem_top_uss_bytes Top-N USS per subgroup
# TYPE herakles_proc_mem_top_uss_bytes gauge
herakles_proc_mem_top_uss_bytes{group="db",subgroup="postgres",rank="1",pid="1234",name="postgres"} 314572800
herakles_proc_mem_top_uss_bytes{group="db",subgroup="postgres",rank="2",pid="1235",name="postgres"} 157286400
herakles_proc_mem_top_uss_bytes{group="db",subgroup="postgres",rank="3",pid="1236",name="postgres"} 104857600
```

## Percentage-of-Subgroup Metrics

These metrics show each top-N process as a percentage of the subgroup total.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_proc_mem_top_rss_percent_of_subgroup` | Gauge | RSS as % of subgroup total |
| `herakles_proc_mem_top_pss_percent_of_subgroup` | Gauge | PSS as % of subgroup total |
| `herakles_proc_mem_top_uss_percent_of_subgroup` | Gauge | USS as % of subgroup total |
| `herakles_proc_mem_top_cpu_percent_of_subgroup` | Gauge | CPU time as % of subgroup total |

**Example output:**

```
# HELP herakles_proc_mem_top_uss_percent_of_subgroup Top-N USS as percentage of subgroup total
# TYPE herakles_proc_mem_top_uss_percent_of_subgroup gauge
herakles_proc_mem_top_uss_percent_of_subgroup{group="db",subgroup="postgres",rank="1",pid="1234",name="postgres"} 54.5
herakles_proc_mem_top_uss_percent_of_subgroup{group="db",subgroup="postgres",rank="2",pid="1235",name="postgres"} 27.3
herakles_proc_mem_top_uss_percent_of_subgroup{group="db",subgroup="postgres",rank="3",pid="1236",name="postgres"} 18.2
```

## Exporter Internal Metrics

These metrics provide observability into the exporter itself.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_proc_mem_scrape_duration_seconds` | Gauge | Time spent serving /metrics request |
| `herakles_proc_mem_processes_total` | Gauge | Number of processes currently exported |
| `herakles_proc_mem_cache_update_duration_seconds` | Gauge | Time spent updating the cache |
| `herakles_proc_mem_cache_update_success` | Gauge | Last cache update success (1) or failure (0) |
| `herakles_proc_mem_cache_updating` | Gauge | Cache update in progress (1) or idle (0) |

**Example output:**

```
# HELP herakles_proc_mem_scrape_duration_seconds Time spent serving /metrics request
# TYPE herakles_proc_mem_scrape_duration_seconds gauge
herakles_proc_mem_scrape_duration_seconds 0.015

# HELP herakles_proc_mem_processes_total Number of processes currently exported
# TYPE herakles_proc_mem_processes_total gauge
herakles_proc_mem_processes_total 156

# HELP herakles_proc_mem_cache_update_duration_seconds Time spent updating the process metrics cache
# TYPE herakles_proc_mem_cache_update_duration_seconds gauge
herakles_proc_mem_cache_update_duration_seconds 0.234

# HELP herakles_proc_mem_cache_update_success Whether the last cache update was successful
# TYPE herakles_proc_mem_cache_update_success gauge
herakles_proc_mem_cache_update_success 1
```

## System Metrics

These metrics provide system-wide resource information.

| Metric | Type | Description |
|--------|------|-------------|
| `herakles_system_memory_total_bytes` | Gauge | Total system memory in bytes (MemTotal from /proc/meminfo) |
| `herakles_system_memory_available_bytes` | Gauge | Available system memory in bytes (MemAvailable from /proc/meminfo) |
| `herakles_system_memory_used_ratio` | Gauge | Memory used ratio: 1 - (available / total), value between 0.0 and 1.0 |
| `herakles_system_cpu_usage_ratio` | Gauge | CPU usage ratio per core and total, calculated from /proc/stat deltas |
| `herakles_system_load1` | Gauge | System load average over 1 minute |
| `herakles_system_load5` | Gauge | System load average over 5 minutes |
| `herakles_system_load15` | Gauge | System load average over 15 minutes |

**Labels for CPU metrics:**
- `cpu` - CPU identifier ("cpu" for total, "cpu0", "cpu1", etc. for individual cores)

**Example output:**

```
# HELP herakles_system_memory_total_bytes Total system memory in bytes
# TYPE herakles_system_memory_total_bytes gauge
herakles_system_memory_total_bytes 16777216000

# HELP herakles_system_memory_available_bytes Available system memory in bytes
# TYPE herakles_system_memory_available_bytes gauge
herakles_system_memory_available_bytes 8388608000

# HELP herakles_system_memory_used_ratio Memory used ratio
# TYPE herakles_system_memory_used_ratio gauge
herakles_system_memory_used_ratio 0.5

# HELP herakles_system_cpu_usage_ratio CPU usage ratio per core and total
# TYPE herakles_system_cpu_usage_ratio gauge
herakles_system_cpu_usage_ratio{cpu="cpu"} 0.35
herakles_system_cpu_usage_ratio{cpu="cpu0"} 0.40
herakles_system_cpu_usage_ratio{cpu="cpu1"} 0.30
herakles_system_cpu_usage_ratio{cpu="cpu2"} 0.35
herakles_system_cpu_usage_ratio{cpu="cpu3"} 0.35

# HELP herakles_system_load1 System load average over 1 minute
# TYPE herakles_system_load1 gauge
herakles_system_load1 1.25

# HELP herakles_system_load5 System load average over 5 minutes
# TYPE herakles_system_load5 gauge
herakles_system_load5 1.50

# HELP herakles_system_load15 System load average over 15 minutes
# TYPE herakles_system_load15 gauge
herakles_system_load15 1.75
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
topk(10, herakles_proc_mem_uss_bytes)

# Memory usage by group
sum by (group) (herakles_proc_mem_rss_bytes)

# Memory usage by subgroup (using aggregated metrics)
herakles_proc_mem_group_rss_bytes_sum

# Percentage of total memory per group
sum by (group) (herakles_proc_mem_rss_bytes) 
  / ignoring(group) group_left sum(herakles_proc_mem_rss_bytes) * 100
```

### CPU Analysis

```promql
# Top 10 processes by CPU
topk(10, herakles_proc_mem_cpu_percent)

# CPU usage by group
sum by (group) (herakles_proc_mem_cpu_percent)

# Processes using more than 50% CPU
herakles_proc_mem_cpu_percent > 50
```

### Process Discovery

```promql
# Count of processes per group
count by (group) (herakles_proc_mem_uss_bytes)

# Count of processes per subgroup
count by (group, subgroup) (herakles_proc_mem_uss_bytes)

# All postgres processes
herakles_proc_mem_uss_bytes{subgroup="postgres"}
```

### Capacity Planning

```promql
# Memory growth rate (bytes per minute)
rate(herakles_proc_mem_rss_bytes[5m]) * 60

# Predict memory usage in 1 hour
herakles_proc_mem_rss_bytes + (rate(herakles_proc_mem_rss_bytes[1h]) * 3600)
```

### Alerting Queries

```promql
# High memory usage (> 80% of total)
herakles_proc_mem_rss_bytes > 0.8 * node_memory_MemTotal_bytes

# Process CPU spike
rate(herakles_proc_mem_cpu_time_seconds[5m]) > 0.9

# Unusual process count
abs(count(herakles_proc_mem_uss_bytes) - count(herakles_proc_mem_uss_bytes offset 1h)) > 10
```

### System Monitoring Queries

```promql
# System memory usage percentage
herakles_system_memory_used_ratio * 100

# Available memory in GB
herakles_system_memory_available_bytes / 1024 / 1024 / 1024

# Average CPU usage across all cores
herakles_system_cpu_usage_ratio{cpu="cpu"}

# Individual core CPU usage
herakles_system_cpu_usage_ratio{cpu=~"cpu[0-9]+"}

# System load normalized by available cores
herakles_system_load1 / count(herakles_system_cpu_usage_ratio{cpu=~"cpu[0-9]+"})

# System under high load (load1 > number of cores)
herakles_system_load1 > count(herakles_system_cpu_usage_ratio{cpu=~"cpu[0-9]+"})
```

## Next Steps

- [Understand the subgroups system](Subgroups-System.md)
- [Configure Prometheus integration](Prometheus-Integration.md)
- [Set up alerting](Alerting-Examples.md)
