# Architecture

This document provides a technical overview of the Herakles Process Memory Exporter architecture.

## Code Structure Overview

```
herakles-proc-mem-exporter/
├── src/
│   └── main.rs           # All application code in single file
├── data/
│   └── subgroups.toml    # Built-in process classification rules
├── Cargo.toml            # Rust dependencies and metadata
├── Cargo.lock            # Locked dependency versions
└── README.md             # Documentation
```

## Main Components

### HTTP Server (Axum)

The exporter uses Axum as its HTTP framework, providing:
- Lightweight, high-performance async HTTP server
- Request routing for `/metrics`, `/health`, `/config`, `/subgroups`
- Shared state management

```rust
// Router setup
let app = Router::new()
    .route("/metrics", get(metrics_handler))
    .route("/health", get(health_handler))
    .route("/config", get(config_handler))
    .route("/subgroups", get(subgroups_handler))
    .with_state(state);
```

### Background Cache Updater

A dedicated Tokio task updates the metrics cache periodically:

```rust
tokio::spawn(async move {
    let mut interval = interval(Duration::from_secs(cache_ttl));
    loop {
        interval.tick().await;
        update_cache(&state).await;
    }
});
```

**Benefits:**
- Decouples expensive /proc scanning from HTTP requests
- Consistent response times for /metrics endpoint
- Configurable update frequency via `cache_ttl`

### Process Scanner

Scans the `/proc` filesystem for process directories:

```rust
fn collect_proc_entries(root: &str, max: Option<usize>) -> Vec<ProcEntry> {
    // Read /proc directory
    // Filter for numeric directories (PIDs)
    // Verify smaps/smaps_rollup availability
    // Return list of process entries
}
```

**Optimizations:**
- Early filtering (skip non-numeric directories)
- Limit maximum processes (`max_processes` config)
- Check for memory metrics availability before processing

### Memory Parser

Two parsing strategies based on kernel version:

#### smaps_rollup (Fast Path - Kernel 4.14+)

```rust
fn parse_smaps_rollup(path: &Path, buf_kb: usize) -> Result<(u64, u64, u64), Error> {
    // Read aggregated memory metrics
    // Parse Rss, Pss, Private_Clean, Private_Dirty
    // USS = Private_Clean + Private_Dirty
}
```

#### smaps (Fallback - Older Kernels)

```rust
fn parse_smaps(path: &Path, buf_kb: usize) -> Result<(u64, u64, u64), Error> {
    // Read full memory map
    // Sum values across all mappings
    // More expensive but available on all kernels
}
```

### CPU Metrics Calculator

Calculates CPU usage using delta between samples:

```rust
fn get_cpu_stat_for_pid(
    pid: u32,
    proc_path: &Path,
    cache: &RwLock<HashMap<u32, CpuEntry>>,
) -> CpuStat {
    // Read current CPU time from /proc/<pid>/stat
    // Calculate delta from previous sample
    // Compute percentage: (delta_cpu / delta_time) * 100
}
```

**Key Details:**
- Uses system clock ticks (CLK_TCK) for time conversion
- Maintains per-process cache for delta calculation
- CPU percent represents usage since last sample

### Classification Engine

Matches process names against subgroup patterns:

```rust
fn classify_process_with_config(
    process_name: &str, 
    cfg: &Config
) -> Option<(Arc<str>, Arc<str>)> {
    // Match against SUBGROUPS map
    // Apply include/exclude filters
    // Handle "other" group logic
    // Return (group, subgroup) or None
}
```

**Pattern Matching:**
- Exact name matches from `matches` array
- Command line patterns from `cmdline_matches`
- Fallback to "other/other" for unmatched processes

### Prometheus Metrics Registry

Custom metrics registered with Prometheus library:

```rust
struct MemoryMetrics {
    // Per-process metrics
    rss: GaugeVec,
    pss: GaugeVec,
    uss: GaugeVec,
    cpu_usage: GaugeVec,
    cpu_time: GaugeVec,
    
    // Aggregated per-subgroup
    agg_rss_sum: GaugeVec,
    agg_pss_sum: GaugeVec,
    // ...
    
    // Top-N per subgroup
    top_rss: GaugeVec,
    top_pss: GaugeVec,
    // ...
}
```

## Data Flow Diagram

```
                                    ┌─────────────────────────────────────────┐
                                    │           Background Task               │
                                    │         (every cache_ttl)               │
                                    └────────────────┬────────────────────────┘
                                                     │
                                                     ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                  Cache Update Flow                                   │
│                                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ Scan /proc   │───▶│ Read Memory  │───▶│ Calculate    │───▶│ Classify     │      │
│  │ directories  │    │ (smaps/      │    │ CPU Metrics  │    │ Processes    │      │
│  │              │    │ smaps_rollup)│    │ (delta)      │    │              │      │
│  └──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                       │              │
│                                                                       ▼              │
│                                                              ┌──────────────┐       │
│                                                              │ Apply        │       │
│                                                              │ Filters      │       │
│                                                              │ (min_uss,    │       │
│                                                              │  include,    │       │
│                                                              │  exclude)    │       │
│                                                              └──────┬───────┘       │
│                                                                     │               │
│                                                                     ▼               │
│                                                              ┌──────────────┐       │
│                                                              │ Update       │       │
│                                                              │ Metrics      │       │
│                                                              │ Cache        │       │
│                                                              └──────────────┘       │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
                                                                       │
                                                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                   /metrics Request                                   │
│                                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ HTTP Request │───▶│ Read from    │───▶│ Format       │───▶│ Return       │      │
│  │ /metrics     │    │ Cache        │    │ Prometheus   │    │ Response     │      │
│  │              │    │              │    │ Text         │    │              │      │
│  └──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

## Concurrency Model

### Tokio Runtime

The exporter uses Tokio for async operations:
- HTTP server runs on Tokio runtime
- Background cache updater as spawned task
- Async I/O for HTTP responses

### Rayon for Parallelism

Process scanning uses Rayon for parallel processing:

```rust
entries
    .par_iter()
    .filter_map(|entry| {
        // Parse memory for each process in parallel
        parse_memory_for_process(&entry.proc_path, &buffer_config)
    })
    .collect()
```

**Benefits:**
- Parallelizes /proc reads across CPU cores
- Configurable thread count via `parallelism`
- Automatic work stealing for load balancing

### Lock Strategy

```
┌────────────────────────────────────────────────────────────────┐
│                       Locking Strategy                          │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Metrics Cache (tokio::RwLock<MetricsCache>)                   │
│  ├─ Write lock: During cache update only                       │
│  └─ Read lock: During /metrics request                         │
│                                                                 │
│  CPU Cache (std::sync::RwLock<HashMap<u32, CpuEntry>>)        │
│  ├─ Write lock: Per-PID update during scan                     │
│  └─ Read lock: Per-PID read during scan                        │
│                                                                 │
│  Health Stats (Mutex for individual stats)                     │
│  └─ Short locks for atomic stat updates                        │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

## Error Handling Strategy

### Graceful Degradation

- Process disappeared: Skip silently
- Permission denied: Log warning, skip process
- Parse error: Log debug, skip process
- Cache update failure: Keep serving stale data

### Error Propagation

```rust
// Errors in background tasks are logged, not propagated
if let Err(e) = update_cache(&state).await {
    error!("Cache update failed: {}", e);
    // Continue running, serve stale cache
}
```

### Health Reporting

- `cache_update_success` metric tracks failures
- Health endpoint shows update status
- Logs capture detailed error information

## Memory Management

### Shared Strings (Arc<str>)

Process names and classification labels use `Arc<str>` for:
- Reduced memory allocation
- Efficient cloning for concurrent access
- Immutable shared references

### Buffer Pooling

Configurable buffer sizes for file reading:
```rust
struct BufferConfig {
    io_kb: usize,           // Generic readers
    smaps_kb: usize,        // Full smaps parsing
    smaps_rollup_kb: usize, // smaps_rollup parsing
}
```

### Cache Size Control

- `max_processes` limits total processes scanned
- `min_uss_kb` filters out small processes
- `top_n_*` controls metric cardinality

## Key Design Decisions

### Single Binary

All code in `main.rs` for:
- Simple deployment (single binary)
- Easy compilation and distribution
- No module complexity for small codebase

### Background Caching

Benefits:
- Consistent /metrics response time
- Decouples scan cost from scrape frequency
- Allows longer scans without timeout issues

### Dual Parser Strategy

- smaps_rollup preferred (faster)
- smaps as fallback (compatibility)
- Automatic selection based on availability

### Subgroup Classification

- Built-in patterns for common software
- Extensible via external TOML files
- Support for both name and cmdline matching

## Performance Characteristics

| Operation | Typical Time | Notes |
|-----------|--------------|-------|
| Cache update (100 procs) | 20-50ms | Parallel, smaps_rollup |
| Cache update (1000 procs) | 200-500ms | Parallel, smaps_rollup |
| /metrics response | 5-20ms | From cache |
| Memory per process | ~200 bytes | In cache |
| Base memory footprint | ~10-20MB | Varies by process count |

## Next Steps

- [Testing documentation](Testing.md)
- [Contributing guidelines](Contributing.md)
- [Performance tuning](Performance-Tuning.md)

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
