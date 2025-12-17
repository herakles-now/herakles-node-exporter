# Troubleshooting

This guide covers common issues and solutions for the Herakles Process Memory Exporter.

## Common Issues and Solutions

### Permission Denied

**Problem:** Exporter fails with "Permission denied" errors when reading `/proc`.

**Symptoms:**
```
Error: Permission denied reading /proc/1234/smaps
```

**Solutions:**

1. **Run as root (not recommended for production):**
   ```bash
   sudo herakles-proc-mem-exporter
   ```

2. **Add capability (recommended):**
   ```bash
   sudo setcap cap_dac_read_search+ep /usr/local/bin/herakles-proc-mem-exporter
   ```

3. **Use systemd with capabilities:**
   ```ini
   [Service]
   User=prometheus
   CapabilityBoundingSet=CAP_DAC_READ_SEARCH
   AmbientCapabilities=CAP_DAC_READ_SEARCH
   ```

4. **Container deployment:**
   ```yaml
   # docker-compose.yml
   services:
     herakles-exporter:
       cap_add:
         - DAC_READ_SEARCH
       volumes:
         - /proc:/host/proc:ro
   ```

### No Processes Found

**Problem:** Metrics show zero processes or empty output.

**Diagnosis:**
```bash
# Check system requirements
herakles-proc-mem-exporter check --all

# Check with debug logging
herakles-proc-mem-exporter --log-level debug
```

**Common Causes:**

1. **Filter too restrictive:**
   ```yaml
   # Problem: Only processes > 10GB USS included
   min_uss_kb: 10485760
   
   # Solution: Lower threshold
   min_uss_kb: 1024
   ```

2. **Search filter mismatch:**
   ```yaml
   # Problem: No matching groups
   search_mode: "include"
   search_groups:
     - nonexistent-group
   
   # Solution: Check available groups
   herakles-proc-mem-exporter subgroups
   ```

3. **All processes filtered as "other":**
   ```yaml
   # Problem: Disabled other group with no matches
   disable_others: true
   search_mode: "include"
   search_groups:
     - custom-group  # No processes match
   
   # Solution: Add custom subgroups or disable filter
   disable_others: false
   ```

### High Cardinality

**Problem:** Prometheus complaining about too many time series.

**Symptoms:**
```
err="add 150000 samples: out of order sample"
level=warn msg="Reached 1M active series limit"
```

**Solutions:**

1. **Reduce Top-N settings:**
   ```yaml
   top_n_subgroup: 3      # Only top 3 per subgroup
   top_n_others: 5        # Limit "other" group
   ```

2. **Increase USS threshold:**
   ```yaml
   min_uss_kb: 10240      # Only processes with >= 10MB USS
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
   enable_rss: true
   enable_pss: false      # Disable if not needed
   enable_uss: true
   enable_cpu: false      # Disable if not needed
   ```

### Slow Scrapes

**Problem:** Prometheus scrapes timing out or taking too long.

**Symptoms:**
```
level=warn msg="Scrape of target failed" err="context deadline exceeded"
```

**Solutions:**

1. **Increase cache TTL:**
   ```yaml
   cache_ttl: 120         # Cache for 2 minutes
   ```

2. **Adjust Prometheus timeout:**
   ```yaml
   scrape_configs:
     - job_name: 'herakles-proc-mem'
       scrape_timeout: 60s  # Increase timeout
       scrape_interval: 120s
   ```

3. **Reduce process count:**
   ```yaml
   max_processes: 500     # Limit total processes
   min_uss_kb: 5120       # Skip small processes
   ```

4. **Increase parallelism:**
   ```yaml
   parallelism: 8         # More threads
   ```

5. **Check disk I/O:**
   ```bash
   # Check if /proc reads are slow
   time cat /proc/1/smaps_rollup
   ```

### Cache Issues

**Problem:** Stale metrics or cache not updating.

**Symptoms:**
- Metrics not changing
- Health endpoint shows failed updates
- `herakles_proc_mem_cache_update_success = 0`

**Diagnosis:**
```bash
# Check health endpoint
curl http://localhost:9215/health

# Look for cache errors
herakles-proc-mem-exporter --log-level debug
```

**Solutions:**

1. **Check for errors in logs:**
   ```bash
   journalctl -u herakles-proc-mem-exporter -f
   ```

2. **Restart the exporter:**
   ```bash
   sudo systemctl restart herakles-proc-mem-exporter
   ```

3. **Verify /proc accessibility:**
   ```bash
   ls -la /proc/self/smaps_rollup
   ```

## Debug Mode

### Enable Debug Logging

```bash
# CLI flag
herakles-proc-mem-exporter --log-level debug

# Or via config
log_level: "debug"

# Environment variable
RUST_LOG=debug herakles-proc-mem-exporter
```

### Trace Logging

For maximum verbosity:
```bash
herakles-proc-mem-exporter --log-level trace
```

### Debug Endpoints

Enable pprof for performance profiling:
```yaml
enable_pprof: true
```

## Log Analysis

### Key Log Messages

| Message | Meaning | Action |
|---------|---------|--------|
| `Starting cache update` | Normal operation | None |
| `Cache update completed: N processes` | Success | None |
| `Failed to read memory for process X` | Process exited during scan | Normal, can ignore |
| `Permission denied reading /proc/X/smaps` | Permission issue | Check capabilities |
| `No processes matched filters` | Filters too restrictive | Review configuration |

### Log Levels

| Level | Use Case |
|-------|----------|
| error | Production - only errors |
| warn | Production - warnings and errors |
| info | Default - normal operation |
| debug | Troubleshooting - detailed operation |
| trace | Development - maximum detail |

## Health Endpoint Interpretation

### Accessing Health

```bash
curl http://localhost:9215/health
```

### Sample Output

```
OK

HEALTH ENDPOINT - EXPORTER INTERNAL STATS
==========================================

                           |    current   |    average   |      max     |      min     

SCAN PERFORMANCE
-----------------
scanned_processes          |          156 |        154.3 |          158 |          150
scan_duration (s)          |        0.045 |        0.043 |        0.089 |        0.038
scan_success_rate (%)      |        100.0 |        100.0 |        100.0 |        100.0
used_subgroups             |           23 |         22.8 |           24 |           22

CACHE PERFORMANCE
------------------
cache_update_duration (s)  |        0.046 |        0.044 |        0.091 |        0.039
cache_hit_ratio (%)        |        100.0 |        100.0 |        100.0 |        100.0
cache_size                 |          156 |        154.3 |          158 |          150

number of done scans: 450 | last scan: 14:32:15 | uptime: 2.5h
```

### Health Metrics Explained

| Metric | Normal Range | Warning If |
|--------|--------------|------------|
| scanned_processes | Varies | 0 or very low |
| scan_duration | < 1s | > 5s |
| scan_success_rate | 100% | < 100% |
| cache_hit_ratio | 100% | < 90% |
| cache_update_duration | < 2s | > 10s |

## Performance Profiling

### Using Built-in Metrics

```promql
# Cache update time
herakles_proc_mem_cache_update_duration_seconds

# Scrape duration
herakles_proc_mem_scrape_duration_seconds

# Process count
herakles_proc_mem_processes_total
```

### System-Level Profiling

```bash
# CPU usage
top -p $(pgrep herakles-proc-mem)

# Memory usage
ps -o pid,vsz,rss,comm -p $(pgrep herakles-proc-mem)

# File descriptor usage
ls /proc/$(pgrep herakles-proc-mem)/fd | wc -l
```

### /proc I/O Testing

```bash
# Test smaps_rollup read time
time for i in $(seq 1 100); do cat /proc/self/smaps_rollup > /dev/null; done

# Test smaps read time (slower)
time for i in $(seq 1 10); do cat /proc/self/smaps > /dev/null; done
```

## System Requirements Verification

### Check Command

```bash
herakles-proc-mem-exporter check --all
```

### Expected Output

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

### Troubleshooting Check Failures

| Check | Failure | Solution |
|-------|---------|----------|
| /proc filesystem | Not accessible | Mount /proc or check container config |
| smaps_rollup | Not available | Upgrade kernel to 4.14+ or use smaps fallback |
| Memory parsing | Failed | Check permissions, try running as root |
| Configuration | Invalid | Run `--check-config` for details |
| Subgroups | Empty | Check subgroups.toml file |

## Frequently Asked Questions

### Why are some processes not showing up?

1. Process exited before scanning completed
2. USS below `min_uss_kb` threshold
3. Filtered by `search_mode` configuration
4. Classified as "other" and `disable_others: true`

### Why is memory higher than expected?

- RSS includes shared memory (libraries)
- Use USS for actual unique memory
- PSS accounts for shared pages proportionally

### Why is CPU percentage sometimes > 100%?

- Multi-threaded processes can use multiple CPU cores
- CPU% > 100% means using more than one core equivalent

### How do I reduce memory usage of the exporter itself?

```yaml
# Reduce buffer sizes
io_buffer_kb: 128
smaps_buffer_kb: 256
smaps_rollup_buffer_kb: 128

# Limit process count
max_processes: 200

# Increase cache TTL
cache_ttl: 120
```

### How do I add monitoring for custom processes?

Create `/etc/herakles/subgroups.toml`:
```toml
subgroups = [
  { group = "myapp", subgroup = "api", matches = ["myapp-api"] },
]
```

## Getting Help

1. **Check logs first:**
   ```bash
   journalctl -u herakles-proc-mem-exporter --since "1 hour ago"
   ```

2. **Enable debug mode:**
   ```bash
   herakles-proc-mem-exporter --log-level debug
   ```

3. **Run system check:**
   ```bash
   herakles-proc-mem-exporter check --all
   ```

4. **Check configuration:**
   ```bash
   herakles-proc-mem-exporter --check-config
   herakles-proc-mem-exporter --show-config
   ```

5. **Open GitHub issue:**
   - Include config (sanitized)
   - Include relevant logs
   - Include system information (kernel version, etc.)

## Next Steps

- [Architecture overview](Architecture.md)
- [Performance tuning](Performance-Tuning.md)
- [Configuration reference](Configuration.md)

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
