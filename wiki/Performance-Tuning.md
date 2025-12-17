# Performance Tuning

This guide covers optimization strategies for the Herakles Process Memory Exporter.

## Understanding Cache TTL

The `cache_ttl` setting controls how often the exporter scans `/proc` for process metrics.

### How Caching Works

```
┌─────────────────────────────────────────────────────────────────┐
│                     Background Cache Update                      │
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │ Scan     │    │ Parse    │    │ Calculate│    │ Update   │  │
│  │ /proc    │───▶│ Memory   │───▶│ CPU      │───▶│ Cache    │  │
│  │ entries  │    │ (smaps)  │    │ Metrics  │    │          │  │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘  │
│                                                                  │
│                     Runs every cache_ttl seconds                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     /metrics Request                             │
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐                   │
│  │ Read     │    │ Format   │    │ Return   │                   │
│  │ from     │───▶│ Prometheus│───▶│ Response │                  │
│  │ Cache    │    │ Text     │    │          │                   │
│  └──────────┘    └──────────┘    └──────────┘                   │
│                                                                  │
│                     Instant response from cache                  │
└─────────────────────────────────────────────────────────────────┘
```

### Choosing Cache TTL

| System Size | Recommended TTL | Notes |
|-------------|-----------------|-------|
| < 50 processes | 15-30s | Fast updates, minimal overhead |
| 50-200 processes | 30-60s | Balanced performance |
| 200-500 processes | 60-90s | Reduce scan frequency |
| 500-1000 processes | 90-120s | Prioritize performance |
| > 1000 processes | 120-300s | Consider filtering |

```yaml
# Low process count
cache_ttl: 30

# High process count
cache_ttl: 120
```

## Buffer Size Optimization

Buffer sizes affect memory parsing performance.

### Buffer Types

| Buffer | Default | Purpose |
|--------|---------|---------|
| `io_buffer_kb` | 256 | Generic /proc file reading |
| `smaps_buffer_kb` | 512 | Full smaps file parsing |
| `smaps_rollup_buffer_kb` | 256 | smaps_rollup parsing (fast path) |

### Optimization Guidelines

```yaml
# For systems with large process memory maps
smaps_buffer_kb: 1024      # Increase for large processes
smaps_rollup_buffer_kb: 512

# For memory-constrained systems
smaps_buffer_kb: 256
smaps_rollup_buffer_kb: 128
```

## Parallelism Configuration

Control the number of threads used for parallel processing.

### Configuration

```yaml
# Auto-detect (recommended for most cases)
parallelism: null

# Manual setting
parallelism: 4
```

### Recommendations

| CPU Cores | Recommended parallelism |
|-----------|------------------------|
| 1-2 | 1-2 |
| 4 | 2-4 |
| 8 | 4-6 |
| 16+ | 8 |

**Note**: Higher parallelism can increase contention on `/proc` filesystem access.

## Limiting Cardinality with Top-N

High cardinality (many unique label combinations) can cause:
- Increased Prometheus memory usage
- Slower queries
- Higher storage costs

### Top-N Configuration

```yaml
# Only export top N processes per subgroup
top_n_subgroup: 3      # For defined subgroups
top_n_others: 10       # For "other" group
```

### Cardinality Calculation

```
Base series per process = 5 (rss, pss, uss, cpu_percent, cpu_time)
Top-N series per subgroup = 10 (5 metrics × 2 for percent_of_subgroup)
Aggregated series per subgroup = 5

Total series ≈ (process_count × 5) + (subgroup_count × 15)
```

### Reducing Cardinality

```yaml
# Aggressive cardinality reduction
top_n_subgroup: 3
top_n_others: 5
min_uss_kb: 10240        # Only processes with >= 10MB USS
disable_others: true     # Skip unclassified processes
```

## Using Search Filters

Filter processes to reduce overhead and focus on relevant metrics.

### Include Mode (Recommended)

```yaml
search_mode: "include"
search_groups:
  - db
  - web
  - container
search_subgroups:
  - prometheus
  - grafana
```

### Exclude Mode

```yaml
search_mode: "exclude"
search_groups:
  - system
  - kernel
```

### Benefits

- Faster scanning (fewer processes to examine)
- Lower cardinality
- Focused metrics

## Memory Parser Selection

The exporter automatically selects the best parser.

### Parser Types

| Parser | Kernel Version | Performance | Accuracy |
|--------|---------------|-------------|----------|
| smaps_rollup | 4.14+ | Fast | High |
| smaps | Any | Slower | High |

### Verification

```bash
# Check if smaps_rollup is available
herakles-proc-mem-exporter check --memory
```

Output:
```
💾 Checking memory metrics accessibility...
   ✅ smaps_rollup available (fast path)
```

## Benchmark Results

Typical scan times on different system sizes:

| Processes | Parser | Cache Update Time | Memory Usage |
|-----------|--------|-------------------|--------------|
| 50 | smaps_rollup | ~10ms | ~5MB |
| 100 | smaps_rollup | ~20ms | ~8MB |
| 200 | smaps_rollup | ~40ms | ~12MB |
| 500 | smaps_rollup | ~100ms | ~25MB |
| 1000 | smaps_rollup | ~200ms | ~45MB |

*Benchmarks on AMD Ryzen 7 3700X, NVMe SSD*

### Running Your Own Benchmark

```bash
# Test mode with timing
herakles-proc-mem-exporter test -n 10 --verbose

# Check health endpoint for stats
curl http://localhost:9215/health
```

## Recommendations by System Size

### Small Systems (< 100 processes)

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 30
parallelism: 2
top_n_subgroup: 5
top_n_others: 15
```

### Medium Systems (100-500 processes)

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 60
parallelism: 4
min_uss_kb: 1024
top_n_subgroup: 3
top_n_others: 10

# Focus on important groups
search_mode: "include"
search_groups:
  - db
  - web
  - container
  - monitoring
```

### Large Systems (500+ processes)

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 120
parallelism: 8
min_uss_kb: 10240
max_processes: 1000
top_n_subgroup: 3
top_n_others: 5

# Strict filtering
search_mode: "include"
search_groups:
  - db
  - web
search_subgroups:
  - postgres
  - nginx
disable_others: true

# Increased buffers
smaps_buffer_kb: 1024
smaps_rollup_buffer_kb: 512

# Minimal logging
log_level: "warn"
```

### Container Host / Kubernetes Node

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 60
parallelism: 4

# Focus on container-related processes
search_mode: "include"
search_groups:
  - container
  - monitoring
search_subgroups:
  - kubelet
  - containerd

top_n_subgroup: 10
top_n_others: 20
```

## Monitoring Exporter Performance

### Health Endpoint

```bash
curl http://localhost:9215/health
```

Check these metrics:
- `scan_duration (s)` - Time to scan /proc
- `cache_update_duration (s)` - Full cache update time
- `scanned_processes` - Number of processes scanned

### Internal Metrics

```promql
# Cache update duration
herakles_proc_mem_cache_update_duration_seconds

# Number of exported processes
herakles_proc_mem_processes_total

# Scrape duration (time to serve /metrics)
herakles_proc_mem_scrape_duration_seconds
```

### Alerting on Performance

```yaml
groups:
  - name: herakles-performance
    rules:
      - alert: HeraklesSlowCacheUpdate
        expr: herakles_proc_mem_cache_update_duration_seconds > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Herakles cache update is slow"
          description: "Cache update takes {{ $value }}s (threshold: 5s)"

      - alert: HeraklesCacheUpdateFailed
        expr: herakles_proc_mem_cache_update_success == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Herakles cache update is failing"
```

## Troubleshooting Performance Issues

### Slow Cache Updates

1. **Increase cache_ttl**: Reduce update frequency
2. **Use search filters**: Focus on fewer processes
3. **Check disk I/O**: `/proc` reads can be slow under load
4. **Reduce parallelism**: High parallelism can cause contention

### High Memory Usage

1. **Reduce buffer sizes**: Lower `smaps_buffer_kb`
2. **Limit process count**: Use `max_processes`
3. **Filter processes**: Use `search_mode: "include"`

### High Cardinality

1. **Reduce top_n settings**: Lower `top_n_subgroup` and `top_n_others`
2. **Increase min_uss_kb**: Filter small processes
3. **Disable unused metrics**: Set `enable_pss: false` etc.

## Next Steps

- [Set up alerting](Alerting-Examples.md)
- [Common use cases](Use-Cases.md)
- [Troubleshooting guide](Troubleshooting.md)
