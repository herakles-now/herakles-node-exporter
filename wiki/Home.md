# Herakles Process Memory Exporter - Wiki

Welcome to the Herakles Process Memory Exporter documentation! This wiki provides comprehensive information about installation, configuration, and usage of the exporter.

## 🚀 Quick Navigation

| Section | Description |
|---------|-------------|
| [Installation](Installation.md) | How to install the exporter |
| [Configuration](Configuration.md) | Complete configuration reference |
| [Metrics Overview](Metrics-Overview.md) | Understanding the exported metrics |
| [Subgroups System](Subgroups-System.md) | Process classification system |
| [Prometheus Integration](Prometheus-Integration.md) | Scrape config and PromQL queries |
| [Performance Tuning](Performance-Tuning.md) | Optimization guide |
| [Alerting Examples](Alerting-Examples.md) | AlertManager rules |
| [Use Cases](Use-Cases.md) | Common use case scenarios |
| [Troubleshooting](Troubleshooting.md) | Problem solving guide |
| [Architecture](Architecture.md) | Technical overview |
| [Testing](Testing.md) | Testing documentation |
| [Contributing](Contributing.md) | How to contribute |

## ⚡ Quick Start

### 1. Install the Exporter

```bash
# From source
git clone https://github.com/herakles-io/herakles-proc-mem-exporter.git
cd herakles-proc-mem-exporter
cargo build --release
sudo cp target/release/herakles-proc-mem-exporter /usr/local/bin/
```

### 2. Create Configuration

```yaml
# /etc/herakles/config.yaml
port: 9215
bind: "0.0.0.0"
cache_ttl: 30

# Optional: Filter by process groups
search_mode: "include"
search_groups:
  - db
  - web
  - container

# Limit cardinality
top_n_subgroup: 5
top_n_others: 10
```

### 3. Start the Exporter

```bash
herakles-proc-mem-exporter -c /etc/herakles/config.yaml
```

### 4. Configure Prometheus

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s
```

### 5. Verify Metrics

```bash
curl http://localhost:9215/metrics | grep herakles_proc_mem
```

## 📖 Quick Reference

For a quick overview of all available features, you can access the documentation endpoint:

```bash
curl http://localhost:9215/doc
```

This provides instant access to:
- All HTTP endpoints
- Available metrics
- Configuration options
- Example queries

## 📊 Sample Output

```
# HELP herakles_proc_mem_rss_bytes Resident Set Size per process in bytes
# TYPE herakles_proc_mem_rss_bytes gauge
herakles_proc_mem_rss_bytes{pid="1234",name="postgres",group="db",subgroup="postgres"} 524288000
herakles_proc_mem_rss_bytes{pid="5678",name="nginx",group="web",subgroup="nginx"} 104857600

# HELP herakles_proc_mem_group_rss_bytes_sum Sum of RSS bytes per subgroup
# TYPE herakles_proc_mem_group_rss_bytes_sum gauge
herakles_proc_mem_group_rss_bytes_sum{group="db",subgroup="postgres"} 2147483648
herakles_proc_mem_group_rss_bytes_sum{group="web",subgroup="nginx"} 419430400
```

## 🔗 Useful Links

- [GitHub Repository](https://github.com/herakles-io/herakles-proc-mem-exporter)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Dashboards](https://grafana.com/grafana/dashboards/)

## 📝 Getting Help

- Check the [Troubleshooting](Troubleshooting.md) guide
- Open an issue on [GitHub](https://github.com/herakles-io/herakles-proc-mem-exporter/issues)
- Contact: proc-mem@herakles.io

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
