# Use Cases

This guide covers common use cases for the Herakles Process Memory Exporter.

## Database Monitoring

### PostgreSQL Monitoring

Monitor PostgreSQL processes for memory usage and connection pooling issues.

**Configuration:**

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 30

search_mode: "include"
search_subgroups:
  - postgres

top_n_subgroup: 20
min_uss_kb: 1024
```

**Key Queries:**

```promql
# Total PostgreSQL memory
herakles_proc_mem_group_rss_bytes_sum{subgroup="postgres"}

# Individual backend processes
herakles_proc_mem_rss_bytes{subgroup="postgres"}

# Connection count (approximate by process count)
count(herakles_proc_mem_uss_bytes{subgroup="postgres", name="postgres"})

# Memory per connection (average)
herakles_proc_mem_group_rss_bytes_sum{subgroup="postgres"}
  / count(herakles_proc_mem_uss_bytes{subgroup="postgres"})
```

**Alerts:**

```yaml
- alert: PostgresHighMemoryPerConnection
  expr: |
    herakles_proc_mem_group_rss_bytes_sum{subgroup="postgres"}
    / count(herakles_proc_mem_uss_bytes{subgroup="postgres"})
    > 104857600  # > 100MB per connection
  for: 10m
  annotations:
    summary: "PostgreSQL connections using excessive memory"
```

### MySQL/MariaDB Monitoring

```yaml
search_mode: "include"
search_subgroups:
  - mysql

top_n_subgroup: 10
```

```promql
# MySQL memory usage
herakles_proc_mem_group_rss_bytes_sum{subgroup="mysql"}

# Memory growth
rate(herakles_proc_mem_rss_bytes{subgroup="mysql"}[1h])
```

### Redis Monitoring

Monitor Redis for memory-related issues.

```yaml
search_mode: "include"
search_subgroups:
  - redis

top_n_subgroup: 5
```

```promql
# Redis memory (compare with Redis INFO memory)
herakles_proc_mem_rss_bytes{subgroup="redis"}

# Redis memory efficiency (RSS vs used_memory)
# Combine with redis_exporter metrics
herakles_proc_mem_rss_bytes{subgroup="redis", name="redis-server"}
  / on(instance) redis_memory_used_bytes
```

## Container Host Monitoring

### Kubernetes Node Monitoring

Monitor container runtime and Kubernetes components.

**Configuration:**

```yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 60

search_mode: "include"
search_groups:
  - container
search_subgroups:
  - kubelet
  - containerd
  - prometheus

top_n_subgroup: 10
top_n_others: 20
```

**Key Queries:**

```promql
# Container runtime memory
herakles_proc_mem_rss_bytes{group="container"}

# Kubelet memory
herakles_proc_mem_rss_bytes{subgroup="kubelet"}

# Total container overhead
sum(herakles_proc_mem_rss_bytes{group="container"})
```

### Docker Host Monitoring

```yaml
search_mode: "include"
search_subgroups:
  - docker
  - containerd

top_n_subgroup: 10
```

```promql
# Docker daemon memory
herakles_proc_mem_rss_bytes{name=~"dockerd|containerd"}

# Memory overhead trend
rate(herakles_proc_mem_rss_bytes{name="dockerd"}[1h])
```

## Java Application Monitoring

### JVM Process Monitoring

Monitor Java applications for heap and metaspace issues.

**Custom Subgroups:**

```toml
# /etc/herakles/subgroups.toml
subgroups = [
  { group = "java", subgroup = "spring-boot", cmdline_matches = [
    "org.springframework.boot.loader",
  ] },
  { group = "java", subgroup = "tomcat", cmdline_matches = [
    "org.apache.catalina.startup.Bootstrap",
  ] },
  { group = "java", subgroup = "kafka", matches = ["kafka"] },
  { group = "java", subgroup = "elasticsearch", matches = ["elasticsearch"] },
]
```

**Configuration:**

```yaml
search_mode: "include"
search_groups:
  - java
  - web

top_n_subgroup: 15
min_uss_kb: 102400  # 100MB minimum
```

**Key Queries:**

```promql
# Total JVM memory
sum by (subgroup) (herakles_proc_mem_rss_bytes{group="java"})

# JVM memory as percentage of system memory
sum(herakles_proc_mem_rss_bytes{group="java"}) 
  / on() node_memory_MemTotal_bytes * 100

# Detect memory growth (potential leak)
deriv(herakles_proc_mem_rss_bytes{group="java"}[1h]) > 0
```

## Memory Leak Detection

### Detecting Memory Leaks

Identify processes with consistent memory growth.

**Configuration:**

```yaml
cache_ttl: 60
min_uss_kb: 10240  # Focus on significant processes
top_n_subgroup: 10
```

**Detection Queries:**

```promql
# Processes with consistent 1-hour growth
deriv(herakles_proc_mem_rss_bytes[1h]) > 1048576  # > 1MB/s growth

# Memory growth percentage over 24 hours
(herakles_proc_mem_rss_bytes - herakles_proc_mem_rss_bytes offset 24h)
  / herakles_proc_mem_rss_bytes offset 24h * 100
  > 20  # > 20% growth

# Long-term growth trend
predict_linear(herakles_proc_mem_rss_bytes[6h], 86400)
  > herakles_proc_mem_rss_bytes * 2  # Will double in 24h
```

**Alerting:**

```yaml
groups:
  - name: memory-leak-detection
    rules:
      - alert: PossibleMemoryLeak
        expr: |
          deriv(herakles_proc_mem_rss_bytes[6h]) > 524288  # > 512KB/s
          and
          herakles_proc_mem_rss_bytes > 1073741824  # > 1GB
        for: 1h
        labels:
          severity: warning
        annotations:
          summary: "Possible memory leak in {{ $labels.name }}"
          description: |
            Process {{ $labels.name }} (PID {{ $labels.pid }}) 
            is growing at {{ $value | humanize1024 }}/s
```

## Capacity Planning

### Memory Capacity Analysis

Plan for future memory requirements.

**Queries:**

```promql
# Current memory usage by group
sum by (group) (herakles_proc_mem_rss_bytes)

# Predict memory in 7 days
predict_linear(sum(herakles_proc_mem_rss_bytes)[7d:1h], 604800)

# Memory growth rate (per day)
deriv(sum(herakles_proc_mem_rss_bytes)[7d]) * 86400

# Days until memory exhaustion
(node_memory_MemAvailable_bytes 
  - sum(herakles_proc_mem_rss_bytes))
  / (deriv(sum(herakles_proc_mem_rss_bytes)[7d]) * 86400)
```

### Process Growth Tracking

```promql
# Process count trend
count(herakles_proc_mem_uss_bytes) 
  - count(herakles_proc_mem_uss_bytes offset 7d)

# Average memory per process over time
sum(herakles_proc_mem_rss_bytes) 
  / count(herakles_proc_mem_uss_bytes)
```

## Cost Optimization

### Identifying Memory Waste

Find over-provisioned or inefficient processes.

**Queries:**

```promql
# Large processes with low CPU (potential over-provisioning)
herakles_proc_mem_rss_bytes > 4294967296  # > 4GB
and
herakles_proc_mem_cpu_percent < 5

# High shared memory ratio (potential for optimization)
(herakles_proc_mem_rss_bytes - herakles_proc_mem_uss_bytes) 
  / herakles_proc_mem_rss_bytes * 100 > 50

# Dormant processes (high memory, no CPU)
herakles_proc_mem_rss_bytes > 1073741824  # > 1GB
and
rate(herakles_proc_mem_cpu_time_seconds[1h]) == 0
```

### Resource Right-Sizing

```promql
# Memory headroom per subgroup
(herakles_proc_mem_rss_bytes - herakles_proc_mem_uss_bytes)
  / herakles_proc_mem_rss_bytes * 100

# Subgroups by memory efficiency
sort_desc(
  herakles_proc_mem_uss_bytes 
    / herakles_proc_mem_rss_bytes
)
```

## Multi-Tenant Environments

### Tenant Isolation Monitoring

Monitor processes per tenant.

**Custom Subgroups:**

```toml
subgroups = [
  { group = "tenant-acme", subgroup = "api", matches = ["acme-api"] },
  { group = "tenant-acme", subgroup = "worker", matches = ["acme-worker"] },
  
  { group = "tenant-beta", subgroup = "api", matches = ["beta-api"] },
  { group = "tenant-beta", subgroup = "worker", matches = ["beta-worker"] },
]
```

**Configuration:**

```yaml
# Monitor all tenants
search_mode: null  # No filter

top_n_subgroup: 5
```

**Queries:**

```promql
# Memory by tenant
sum by (group) (herakles_proc_mem_rss_bytes{group=~"tenant-.*"})

# Compare tenant resource usage
topk(10, sum by (group) (herakles_proc_mem_rss_bytes{group=~"tenant-.*"}))

# Detect noisy neighbor
sum by (group) (herakles_proc_mem_cpu_percent{group=~"tenant-.*"}) > 100
```

**Alerting:**

```yaml
- alert: TenantExcessiveMemory
  expr: |
    sum by (group) (herakles_proc_mem_rss_bytes{group=~"tenant-.*"}) > 17179869184
  for: 10m
  annotations:
    summary: "Tenant {{ $labels.group }} using excessive memory"
```

## Web Application Stack

### Full Stack Monitoring

Monitor a typical web application stack.

**Configuration:**

```yaml
search_mode: "include"
search_groups:
  - web
  - db
  - cache
  - messaging
  
search_subgroups:
  - nginx
  - postgres
  - redis
  - kafka

top_n_subgroup: 10
```

**Dashboard Queries:**

```promql
# Layer-by-layer memory breakdown
sum by (group) (herakles_proc_mem_rss_bytes{group=~"web|db|cache|messaging"})

# Request processing overhead (proxy/web layer)
herakles_proc_mem_rss_bytes{group="web"}

# Data layer memory
herakles_proc_mem_rss_bytes{group="db"}

# Caching efficiency (cache layer)
herakles_proc_mem_rss_bytes{group="cache"}
```

## Microservices Architecture

### Service-Level Monitoring

**Custom Subgroups:**

```toml
subgroups = [
  { group = "services", subgroup = "user-service", matches = ["user-service", "user-api"] },
  { group = "services", subgroup = "order-service", matches = ["order-service"] },
  { group = "services", subgroup = "payment-service", matches = ["payment-service"] },
  { group = "services", subgroup = "inventory-service", matches = ["inventory-api"] },
  { group = "services", subgroup = "notification-service", matches = ["notification-worker"] },
]
```

**Queries:**

```promql
# Memory per service
sum by (subgroup) (herakles_proc_mem_rss_bytes{group="services"})

# Service replica count
count by (subgroup) (herakles_proc_mem_uss_bytes{group="services"})

# Memory per replica
sum by (subgroup) (herakles_proc_mem_rss_bytes{group="services"})
  / count by (subgroup) (herakles_proc_mem_uss_bytes{group="services"})
```

## Best Practices Summary

| Use Case | Key Configuration | Focus Metrics |
|----------|------------------|---------------|
| Database | `search_subgroups`, low TTL | RSS, connection count |
| Containers | `search_groups: container` | RSS overhead, process count |
| Java Apps | Custom cmdline_matches | RSS growth, USS |
| Leak Detection | Low `min_uss_kb`, recording rules | deriv(), predict_linear() |
| Capacity | Long retention, recording rules | Growth rates, predictions |
| Multi-tenant | Custom tenant groups | Group sums, CPU |

## Next Steps

- [Troubleshooting guide](Troubleshooting.md)
- [Alerting examples](Alerting-Examples.md)
- [Performance tuning](Performance-Tuning.md)

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
