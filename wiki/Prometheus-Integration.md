# Prometheus Integration

This guide covers how to integrate the Herakles Process Memory Exporter with Prometheus.

## Scrape Configuration

### Basic Configuration

Add the exporter to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s
```

### HTTPS/TLS Configuration

When TLS is enabled on the exporter, configure Prometheus to scrape via HTTPS:

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s
    scheme: https
    tls_config:
      # For private/custom CA certificates (not publicly trusted)
      ca_file: /etc/prometheus/certs/ca.crt
      
      # Optional: Client certificate authentication
      # cert_file: /etc/prometheus/certs/client.crt
      # key_file: /etc/prometheus/certs/client.key
```

**Note:** For certificates signed by publicly trusted CAs, no additional `tls_config` is needed - just set `scheme: https`.

**For Self-Signed Certificates (Testing Only):**

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s
    scheme: https
    tls_config:
      insecure_skip_verify: true  # Only for testing!
```

**For Production with Multiple TLS Targets:**

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets:
          - 'server1:9215'
          - 'server2:9215'
          - 'server3:9215'
    scrape_interval: 60s
    scrape_timeout: 30s
    scheme: https
    tls_config:
      ca_file: /etc/prometheus/certs/herakles-ca.crt
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
        regex: '(.+):.*'
        replacement: '${1}'
```

### Multiple Targets

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets:
          - 'server1:9215'
          - 'server2:9215'
          - 'server3:9215'
    scrape_interval: 60s
    scrape_timeout: 30s
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
        regex: '(.+):.*'
        replacement: '${1}'
```

### With Relabeling

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
        labels:
          environment: production
          datacenter: dc1
    
    # Add hostname label from instance
    relabel_configs:
      - source_labels: [__address__]
        target_label: hostname
        regex: '(.+):.*'
        replacement: '${1}'
    
    # Drop high-cardinality labels if needed
    metric_relabel_configs:
      - source_labels: [pid]
        regex: '.*'
        action: labeldrop  # Removes pid label to reduce cardinality
```

## Service Discovery

### File-Based Service Discovery

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    file_sd_configs:
      - files:
          - '/etc/prometheus/targets/herakles-*.json'
    scrape_interval: 60s
```

```json
# /etc/prometheus/targets/herakles-production.json
[
  {
    "targets": ["server1:9215", "server2:9215"],
    "labels": {
      "environment": "production",
      "team": "platform"
    }
  }
]
```

### Kubernetes Service Discovery

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      # Only scrape pods with annotation prometheus.io/scrape=true
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      
      # Use annotation for port
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_port]
        action: replace
        target_label: __address__
        regex: (.+)
        replacement: ${__meta_kubernetes_pod_ip}:${1}
      
      # Add namespace and pod name labels
      - source_labels: [__meta_kubernetes_namespace]
        target_label: kubernetes_namespace
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: kubernetes_pod_name
```

### Consul Service Discovery

```yaml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    consul_sd_configs:
      - server: 'consul.service.consul:8500'
        services: ['herakles-node-exporter']
    relabel_configs:
      - source_labels: [__meta_consul_node]
        target_label: instance
```

## Recording Rules

### Aggregation Rules

Create recording rules for commonly used aggregations:

```yaml
# rules/herakles-recording.yml
groups:
  - name: herakles-proc-mem-aggregation
    interval: 60s
    rules:
      # Total memory by group
      - record: herakles:proc_mem_rss_bytes_by_group:sum
        expr: sum by (group) (herakles_mem_process_rss_bytes)
      
      # Total memory by subgroup
      - record: herakles:proc_mem_rss_bytes_by_subgroup:sum
        expr: sum by (group, subgroup) (herakles_mem_process_rss_bytes)
      
      # Process count by group
      - record: herakles:proc_count_by_group:count
        expr: count by (group) (herakles_mem_process_uss_bytes)
      
      # Average memory per process by subgroup
      - record: herakles:proc_mem_rss_bytes_by_subgroup:avg
        expr: avg by (group, subgroup) (herakles_mem_process_rss_bytes)
      
      # Total CPU by group
      - record: herakles:proc_cpu_percent_by_group:sum
        expr: sum by (group) (herakles_cpu_process_usage_percent)
      
      # Memory growth rate
      - record: herakles:proc_mem_rss_bytes:rate5m
        expr: rate(herakles_mem_process_rss_bytes[5m])
```

### Pre-calculated Alerting Metrics

```yaml
groups:
  - name: herakles-proc-mem-alerting
    interval: 30s
    rules:
      # Top 10 memory consumers
      - record: herakles:proc_mem_top10_rss:bytes
        expr: topk(10, herakles_mem_process_rss_bytes)
      
      # Processes exceeding thresholds
      - record: herakles:proc_high_memory:count
        expr: count(herakles_mem_process_rss_bytes > 1073741824)  # > 1GB
      
      # Processes with high CPU
      - record: herakles:proc_high_cpu:count
        expr: count(herakles_cpu_process_usage_percent > 80)
```

## Common PromQL Queries

### Memory Analysis

```promql
# Top 10 processes by RSS memory
topk(10, herakles_mem_process_rss_bytes)

# Top 10 processes by USS (unique) memory
topk(10, herakles_mem_process_uss_bytes)

# Memory usage by group (pie chart)
sum by (group) (herakles_mem_process_rss_bytes)

# Memory usage by subgroup
sum by (group, subgroup) (herakles_mem_process_rss_bytes)

# Memory as percentage of total (requires node_exporter)
sum by (group) (herakles_mem_process_rss_bytes) 
  / on() group_left() node_memory_MemTotal_bytes * 100

# Memory growth rate (bytes/minute)
rate(herakles_mem_process_rss_bytes[5m]) * 60

# Memory growth percentage over 1 hour
(herakles_mem_process_rss_bytes - herakles_mem_process_rss_bytes offset 1h) 
  / herakles_mem_process_rss_bytes offset 1h * 100

# Shared memory ratio (RSS - USS) / RSS
(herakles_mem_process_rss_bytes - herakles_mem_process_uss_bytes) 
  / herakles_mem_process_rss_bytes * 100
```

### CPU Analysis

```promql
# Top 10 processes by CPU usage
topk(10, herakles_cpu_process_usage_percent)

# CPU usage by group
sum by (group) (herakles_cpu_process_usage_percent)

# CPU usage by subgroup
sum by (group, subgroup) (herakles_cpu_process_usage_percent)

# CPU time rate (seconds/second)
rate(herakles_cpu_process_time_seconds[5m])

# Processes with CPU > 50%
herakles_cpu_process_usage_percent > 50

# Average CPU per subgroup
avg by (group, subgroup) (herakles_cpu_process_usage_percent)
```

### Process Discovery

```promql
# Count of processes per group
count by (group) (herakles_mem_process_uss_bytes)

# Count of processes per subgroup
count by (group, subgroup) (herakles_mem_process_uss_bytes)

# All processes in a specific subgroup
herakles_mem_process_uss_bytes{subgroup="postgres"}

# Processes by name pattern
herakles_mem_process_uss_bytes{name=~".*worker.*"}

# New processes (appeared in last hour)
herakles_mem_process_uss_bytes unless herakles_mem_process_uss_bytes offset 1h
```

### Capacity Planning

```promql
# Projected memory in 24 hours
herakles_mem_process_rss_bytes 
  + (deriv(herakles_mem_process_rss_bytes[6h]) * 86400)

# Memory headroom (requires node_exporter)
node_memory_MemAvailable_bytes 
  - sum(herakles_mem_process_rss_bytes)

# Days until memory exhaustion
node_memory_MemAvailable_bytes 
  / (deriv(sum(herakles_mem_process_rss_bytes)[24h]) * 86400)
```

### Aggregated Subgroup Queries

```promql
# Use pre-aggregated metrics for efficiency
herakles_mem_group_rss_bytes

# Top subgroups by memory
topk(10, herakles_mem_group_rss_bytes)

# CPU time sum per subgroup
herakles_cpu_group_time_seconds_sum
```

## Grafana Dashboard Template

Here's a basic Grafana dashboard JSON template:

```json
{
  "annotations": {
    "list": []
  },
  "editable": true,
  "fiscalYearStartMonth": 0,
  "graphTooltip": 0,
  "id": null,
  "links": [],
  "liveNow": false,
  "panels": [
    {
      "datasource": {
        "type": "prometheus",
        "uid": "${datasource}"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "palette-classic"
          },
          "custom": {
            "hideFrom": {
              "legend": false,
              "tooltip": false,
              "viz": false
            }
          },
          "mappings": [],
          "unit": "bytes"
        },
        "overrides": []
      },
      "gridPos": {
        "h": 8,
        "w": 12,
        "x": 0,
        "y": 0
      },
      "id": 1,
      "options": {
        "legend": {
          "displayMode": "table",
          "placement": "right",
          "showLegend": true
        },
        "pieType": "pie",
        "reduceOptions": {
          "calcs": [
            "lastNotNull"
          ],
          "fields": "",
          "values": false
        },
        "tooltip": {
          "mode": "single",
          "sort": "none"
        }
      },
      "title": "Memory by Group",
      "type": "piechart",
      "targets": [
        {
          "expr": "sum by (group) (herakles_mem_process_rss_bytes)",
          "legendFormat": "{{group}}"
        }
      ]
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "${datasource}"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "palette-classic"
          },
          "mappings": [],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "green",
                "value": null
              }
            ]
          },
          "unit": "bytes"
        },
        "overrides": []
      },
      "gridPos": {
        "h": 8,
        "w": 12,
        "x": 12,
        "y": 0
      },
      "id": 2,
      "options": {
        "colorMode": "value",
        "graphMode": "area",
        "justifyMode": "auto",
        "orientation": "auto",
        "reduceOptions": {
          "calcs": [
            "lastNotNull"
          ],
          "fields": "",
          "values": false
        },
        "textMode": "auto"
      },
      "title": "Total RSS Memory",
      "type": "stat",
      "targets": [
        {
          "expr": "sum(herakles_mem_process_rss_bytes)",
          "legendFormat": "Total RSS"
        }
      ]
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "${datasource}"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "palette-classic"
          },
          "custom": {
            "axisCenteredZero": false,
            "axisColorMode": "text",
            "axisLabel": "",
            "axisPlacement": "auto",
            "barAlignment": 0,
            "drawStyle": "line",
            "fillOpacity": 10,
            "gradientMode": "none",
            "hideFrom": {
              "legend": false,
              "tooltip": false,
              "viz": false
            },
            "lineInterpolation": "linear",
            "lineWidth": 1,
            "pointSize": 5,
            "scaleDistribution": {
              "type": "linear"
            },
            "showPoints": "never",
            "spanNulls": false,
            "stacking": {
              "group": "A",
              "mode": "none"
            },
            "thresholdsStyle": {
              "mode": "off"
            }
          },
          "mappings": [],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "green",
                "value": null
              }
            ]
          },
          "unit": "bytes"
        },
        "overrides": []
      },
      "gridPos": {
        "h": 8,
        "w": 24,
        "x": 0,
        "y": 8
      },
      "id": 3,
      "options": {
        "legend": {
          "calcs": [
            "lastNotNull",
            "max"
          ],
          "displayMode": "table",
          "placement": "right",
          "showLegend": true
        },
        "tooltip": {
          "mode": "multi",
          "sort": "desc"
        }
      },
      "title": "Top 10 Processes by RSS",
      "type": "timeseries",
      "targets": [
        {
          "expr": "topk(10, herakles_mem_process_rss_bytes)",
          "legendFormat": "{{name}} (pid={{pid}})"
        }
      ]
    }
  ],
  "refresh": "30s",
  "schemaVersion": 38,
  "style": "dark",
  "tags": [
    "herakles",
    "memory",
    "processes"
  ],
  "templating": {
    "list": [
      {
        "current": {},
        "hide": 0,
        "includeAll": false,
        "multi": false,
        "name": "datasource",
        "options": [],
        "query": "prometheus",
        "refresh": 1,
        "regex": "",
        "skipUrlSync": false,
        "type": "datasource"
      }
    ]
  },
  "time": {
    "from": "now-1h",
    "to": "now"
  },
  "title": "Herakles Process Memory",
  "uid": "herakles-proc-mem",
  "version": 1
}
```

## Best Practices

### Scrape Intervals

| Use Case | Recommended Interval |
|----------|---------------------|
| Development | 15s |
| Standard monitoring | 60s |
| Large scale (1000+ processes) | 120s |
| Debugging | 10s |

### Reducing Cardinality

```yaml
# prometheus.yml - Drop PID label
metric_relabel_configs:
  - source_labels: [__name__]
    regex: 'herakles_(mem|cpu|exporter)_.*'
    action: keep
  - source_labels: [pid]
    regex: '.*'
    action: labeldrop
```

### Storage Optimization

```yaml
# Use recording rules for frequently used aggregations
groups:
  - name: herakles-storage-optimization
    rules:
      # Pre-aggregate to reduce storage
      - record: herakles:memory_by_group:sum
        expr: sum by (group) (herakles_mem_process_rss_bytes)
```

## Next Steps

- [Set up alerting rules](Alerting-Examples.md)
- [Performance tuning for large deployments](Performance-Tuning.md)
- [Common use cases](Use-Cases.md)
