# Alerting Examples

This guide provides AlertManager configuration and example alert rules for the Herakles Process Memory Exporter.

## AlertManager Configuration

### Basic AlertManager Setup

```yaml
# alertmanager.yml
global:
  smtp_smarthost: 'smtp.example.org:587'
  smtp_from: 'alerts@example.org'
  smtp_auth_username: 'alerts@example.org'
  smtp_auth_password: 'password'

route:
  group_by: ['alertname', 'group', 'subgroup']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'default'
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'default'
    email_configs:
      - to: 'ops@example.org'

  - name: 'slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#alerts'
        text: '{{ template "slack.default.text" . }}'

  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'your-pagerduty-service-key'

templates:
  - '/etc/alertmanager/templates/*.tmpl'
```

## Alert Rules

### High Memory Usage

```yaml
groups:
  - name: herakles-memory-alerts
    rules:
      # Individual process high memory
      - alert: HeraklesProcessHighMemory
        expr: herakles_mem_process_rss_bytes > 4294967296  # > 4GB
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Process {{ $labels.name }} using high memory"
          description: |
            Process {{ $labels.name }} (PID: {{ $labels.pid }}) 
            is using {{ $value | humanize1024 }} of memory.
            Group: {{ $labels.group }}/{{ $labels.subgroup }}
          runbook_url: "https://wiki.example.org/runbooks/high-memory"

      # Critical memory usage (> 8GB)
      - alert: HeraklesProcessCriticalMemory
        expr: herakles_mem_process_rss_bytes > 8589934592  # > 8GB
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Process {{ $labels.name }} using critical memory"
          description: |
            Process {{ $labels.name }} (PID: {{ $labels.pid }}) 
            is using {{ $value | humanize1024 }} of memory.
            Immediate attention required.

      # Subgroup total memory high
      - alert: HeraklesSubgroupHighMemory
        expr: herakles_mem_group_rss_bytes > 17179869184  # > 16GB
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "Subgroup {{ $labels.subgroup }} using high total memory"
          description: |
            All processes in {{ $labels.group }}/{{ $labels.subgroup }}
            are using {{ $value | humanize1024 }} of total memory.
```

### Memory Growth Detection

```yaml
groups:
  - name: herakles-memory-growth
    rules:
      # Fast memory growth (potential leak)
      - alert: HeraklesProcessMemoryGrowth
        expr: |
          (herakles_mem_process_rss_bytes - herakles_mem_process_rss_bytes offset 1h)
          / herakles_mem_process_rss_bytes offset 1h * 100 > 50
        for: 30m
        labels:
          severity: warning
        annotations:
          summary: "Process {{ $labels.name }} memory growing rapidly"
          description: |
            Process {{ $labels.name }} (PID: {{ $labels.pid }}) 
            memory has grown by {{ $value | printf "%.1f" }}% in the last hour.
            Current: {{ with query "herakles_mem_process_rss_bytes{pid='%s'}" $labels.pid }}
              {{ . | first | value | humanize1024 }}
            {{ end }}

      # Steady memory increase over 6 hours
      - alert: HeraklesProcessMemoryLeak
        expr: |
          deriv(herakles_mem_process_rss_bytes[6h]) > 1048576
        for: 1h
        labels:
          severity: warning
        annotations:
          summary: "Potential memory leak in {{ $labels.name }}"
          description: |
            Process {{ $labels.name }} (PID: {{ $labels.pid }}) 
            is consistently growing at {{ $value | humanize1024 }}/s.
            This may indicate a memory leak.
          runbook_url: "https://wiki.example.org/runbooks/memory-leak"
```

### Subgroup Memory Thresholds

```yaml
groups:
  - name: herakles-subgroup-thresholds
    rules:
      # Database memory thresholds
      - alert: HeraklesPostgresHighMemory
        expr: |
          herakles_mem_group_rss_bytes{subgroup="postgres"} > 34359738368
        for: 10m
        labels:
          severity: warning
          team: database
        annotations:
          summary: "PostgreSQL using {{ $value | humanize1024 }} memory"
          description: |
            PostgreSQL processes are using more than 32GB of memory.
            Consider checking for memory pressure or query issues.

      # Redis memory threshold
      - alert: HeraklesRedisHighMemory
        expr: |
          herakles_mem_group_rss_bytes{subgroup="redis"} > 8589934592
        for: 5m
        labels:
          severity: warning
          team: platform
        annotations:
          summary: "Redis using {{ $value | humanize1024 }} memory"
          description: "Redis memory usage is high. Check maxmemory configuration."

      # Elasticsearch memory threshold
      - alert: HeraklesElasticsearchHighMemory
        expr: |
          herakles_mem_group_rss_bytes{subgroup="elasticsearch"} > 68719476736
        for: 10m
        labels:
          severity: warning
          team: search
        annotations:
          summary: "Elasticsearch using {{ $value | humanize1024 }} memory"
```

### CPU Spike Detection

```yaml
groups:
  - name: herakles-cpu-alerts
    rules:
      # High CPU usage
      - alert: HeraklesProcessHighCPU
        expr: herakles_cpu_process_usage_percent > 80
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Process {{ $labels.name }} using {{ $value | printf \"%.1f\" }}% CPU"
          description: |
            Process {{ $labels.name }} (PID: {{ $labels.pid }}) 
            is using more than 80% CPU for over 10 minutes.

      # CPU spike (sudden increase)
      - alert: HeraklesProcessCPUSpike
        expr: |
          herakles_cpu_process_usage_percent > 50
          and
          herakles_cpu_process_usage_percent > (herakles_cpu_process_usage_percent offset 10m) * 3
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "CPU spike detected for {{ $labels.name }}"
          description: |
            Process {{ $labels.name }} CPU usage spiked from 
            {{ with query "herakles_cpu_process_usage_percent{pid='%s'} offset 10m" $labels.pid }}
              {{ . | first | value | printf "%.1f" }}%
            {{ end }} to {{ $value | printf "%.1f" }}%.

      # Subgroup CPU alert
      - alert: HeraklesSubgroupHighCPU
        expr: herakles_cpu_group_usage_percent_sum > 200
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Subgroup {{ $labels.subgroup }} using {{ $value | printf \"%.1f\" }}% total CPU"
```

### Process Count Anomalies

```yaml
groups:
  - name: herakles-process-count
    rules:
      # Too many processes in subgroup
      - alert: HeraklesSubgroupProcessCount
        expr: count by (group, subgroup) (herakles_mem_process_uss_bytes) > 50
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High process count in {{ $labels.subgroup }}"
          description: |
            {{ $labels.group }}/{{ $labels.subgroup }} has {{ $value }} processes.
            This may indicate a fork bomb or misconfiguration.

      # Sudden change in process count
      - alert: HeraklesProcessCountChange
        expr: |
          abs(
            count(herakles_mem_process_uss_bytes) 
            - count(herakles_mem_process_uss_bytes offset 30m)
          ) > 20
        for: 5m
        labels:
          severity: info
        annotations:
          summary: "Significant change in process count"
          description: |
            Process count changed by {{ $value }} in the last 30 minutes.
            Current: {{ with query "count(herakles_mem_process_uss_bytes)" }}
              {{ . | first | value }}
            {{ end }}

      # No processes in expected subgroup
      - alert: HeraklesSubgroupMissing
        expr: |
          absent(herakles_mem_process_uss_bytes{subgroup="postgres"}) == 1
        for: 5m
        labels:
          severity: critical
          team: database
        annotations:
          summary: "No PostgreSQL processes found"
          description: "Expected PostgreSQL processes are not running."
```

### Exporter Health

```yaml
groups:
  - name: herakles-exporter-health
    rules:
      # Cache update failing
      - alert: HeraklesCacheUpdateFailed
        expr: herakles_exporter_cache_update_success == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Herakles exporter cache update failing"
          description: "The process metrics cache has not been updated successfully."

      # Slow cache updates
      - alert: HeraklesSlowCacheUpdate
        expr: herakles_exporter_cache_update_duration_seconds > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Herakles cache update is slow ({{ $value | printf \"%.2f\" }}s)"
          description: "Cache updates are taking longer than expected."

      # Exporter down
      - alert: HeraklesExporterDown
        expr: up{job="herakles-proc-mem"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Herakles exporter is down"
          description: "The Herakles process memory exporter is not responding."
```

## Alert Annotation Templates

### Slack Template

```gotemplate
{{ define "slack.herakles.title" -}}
[{{ .Status | toUpper }}{{ if eq .Status "firing" }}:{{ .Alerts.Firing | len }}{{ end }}] {{ .GroupLabels.alertname }}
{{- end }}

{{ define "slack.herakles.text" -}}
{{ range .Alerts }}
*Alert:* {{ .Annotations.summary }}
*Severity:* {{ .Labels.severity }}
*Details:*
{{ .Annotations.description }}
{{ if .Annotations.runbook_url }}*Runbook:* {{ .Annotations.runbook_url }}{{ end }}
{{ end }}
{{- end }}
```

### Email Template

```gotemplate
{{ define "email.herakles.subject" -}}
[{{ .Status | toUpper }}] {{ .GroupLabels.alertname }} - Herakles Memory Alert
{{- end }}

{{ define "email.herakles.html" -}}
<!DOCTYPE html>
<html>
<head>
  <style>
    .alert { padding: 10px; margin: 10px 0; border-left: 4px solid; }
    .critical { border-color: #dc3545; background: #f8d7da; }
    .warning { border-color: #ffc107; background: #fff3cd; }
  </style>
</head>
<body>
  <h2>Herakles Process Memory Alert</h2>
  {{ range .Alerts }}
  <div class="alert {{ .Labels.severity }}">
    <h3>{{ .Annotations.summary }}</h3>
    <p><strong>Severity:</strong> {{ .Labels.severity }}</p>
    <p><strong>Group:</strong> {{ .Labels.group }}/{{ .Labels.subgroup }}</p>
    <p>{{ .Annotations.description }}</p>
    {{ if .Annotations.runbook_url }}
    <p><a href="{{ .Annotations.runbook_url }}">View Runbook</a></p>
    {{ end }}
  </div>
  {{ end }}
</body>
</html>
{{- end }}
```

## Runbook Links

### Sample Runbook Structure

```yaml
annotations:
  runbook_url: "https://wiki.example.org/runbooks/herakles/{{ $labels.alertname }}"
```

### Example Runbook Topics

| Alert | Runbook Topic |
|-------|---------------|
| HeraklesProcessHighMemory | Process memory investigation |
| HeraklesProcessMemoryLeak | Memory leak debugging |
| HeraklesProcessHighCPU | CPU troubleshooting |
| HeraklesCacheUpdateFailed | Exporter troubleshooting |

## Next Steps

- [Common use cases](Use-Cases.md)
- [Troubleshooting](Troubleshooting.md)
- [Performance tuning](Performance-Tuning.md)
