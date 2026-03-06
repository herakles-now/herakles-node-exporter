# HTTP Endpoints Reference

This document describes all HTTP endpoints exposed by the Herakles Node Exporter.  
By default, the exporter listens on **port `9215`** and binds to **`0.0.0.0`**.

---

## Overview

| Endpoint | Method | Format | Description |
|----------|--------|--------|-------------|
| [`/`](#-root) | GET | HTML | Landing page with uptime info and endpoint list |
| [`/metrics`](#-metrics) | GET | Text (Prometheus) | Prometheus metrics scrape endpoint |
| [`/health`](#-health) | GET | Plain text | Exporter health statistics and buffer status |
| [`/config`](#-config) | GET | Plain text | Active runtime configuration (read-only) |
| [`/subgroups`](#-subgroups) | GET | Plain text / JSON | Detected process groups and subgroups |
| [`/doc`](#-doc) | GET | Plain text | Full text documentation for metrics and concepts |
| [`/docs`](#-docs--htmldocs) | GET | HTML | HTML documentation page |
| [`/details`](#-details) | GET | Plain text | Ring buffer statistics and scrape history |
| [`/html`](#-html--html) | GET | HTML | HTML index / navigation page |
| [`/html/details`](#-htmldetails) | GET | HTML | Interactive view of subgroup history and ring buffer |
| [`/html/subgroups`](#-htmlsubgroups) | GET | HTML | HTML view of detected groups and subgroups |
| [`/html/health`](#-htmlhealth) | GET | HTML | HTML view of exporter health |
| [`/html/config`](#-htmlconfig) | GET | HTML | HTML view of active runtime configuration |

> **Note:** The `/health` endpoint can be disabled via `enable_health: false` in the configuration.

---

## Endpoint Details

### `/` — Root

- **Method:** `GET`
- **Format:** HTML
- **Authentication:** None
- **Cache:** No

The landing page of the exporter. Displays:
- Current exporter version
- Uptime since start
- A clickable list of all available endpoints with short descriptions

**Example:**

```bash
curl http://localhost:9215/
```

---

### `/metrics` — Metrics

- **Method:** `GET`
- **Format:** Prometheus text exposition format (`text/plain; version=0.0.4`)
- **Authentication:** None
- **Cache:** Yes — respects `cache_ttl` (default: `30` seconds)

The primary endpoint scraped by Prometheus. Returns all collected metrics including:

- `herakles_mem_*` — per-process and per-group memory metrics (RSS, PSS, USS)
- `herakles_cpu_*` — per-process and per-group CPU metrics
- `herakles_group_*` — aggregated group-level metrics
- `herakles_exporter_*` — internal exporter telemetry (scrape duration, cache hits, errors)
- `herakles_top_*` — top-N process metrics per subgroup
- `herakles_net_*` / `herakles_group_net_*` — network metrics (if enabled)

**Example:**

```bash
curl http://localhost:9215/metrics
curl http://localhost:9215/metrics | grep herakles_mem_
```

**Prometheus scrape configuration:**

```yaml
scrape_configs:
  - job_name: 'herakles-node-exporter'
    static_configs:
      - targets: ['localhost:9215']
    scrape_interval: 60s
    scrape_timeout: 30s
```

> See [Metrics Overview](Metrics-Overview.md) for a full description of all metrics.

---

### `/health` — Health

- **Method:** `GET`
- **Format:** Plain text
- **Authentication:** None
- **Enabled by default:** Yes (disable via `enable_health: false`)

Returns human-readable health and performance statistics of the exporter itself, including:

- Total HTTP requests served
- Exporter uptime
- Buffer sizes and usage (io buffer, smaps buffer, smaps_rollup buffer)
- Last scrape duration
- Cache status

**Example:**

```bash
curl http://localhost:9215/health
```

> For a visual HTML version, use [`/html/health`](#-htmlhealth).

---

### `/config` — Configuration

- **Method:** `GET`
- **Format:** Plain text
- **Authentication:** None

Displays the **active runtime configuration** of the exporter as a formatted text table. This is a read-only view — no configuration changes can be made via this endpoint.

Shows all resolved configuration values including:
- Server settings (bind address, port)
- Cache TTL
- Filtering settings (min_uss_kb, search_mode, top_n_*)
- Feature flags (enable_health, enable_telemetry, …)
- TLS status

**Example:**

```bash
curl http://localhost:9215/config
```

> For a visual HTML version, use [`/html/config`](#-htmlconfig).

---

### `/subgroups` — Subgroups

- **Method:** `GET`
- **Format:** Plain text (optionally JSON)
- **Authentication:** None

Lists all process **groups** and **subgroups** currently detected by the exporter's classification engine. Useful for verifying that your process filter configuration (`search_groups`, `search_subgroups`) is working as expected.

**Example:**

```bash
curl http://localhost:9215/subgroups
```

> See [Subgroups System](Subgroups-System.md) for a full explanation of the group/subgroup concept.

---

### `/doc` — Documentation (Text)

- **Method:** `GET`
- **Format:** Plain text
- **Authentication:** None

Returns a comprehensive plain-text reference document covering:
- All available metrics and their labels
- Configuration options
- CLI commands
- Example queries and usage

Useful for quick access without a browser.

**Example:**

```bash
curl http://localhost:9215/doc
curl http://localhost:9215/doc | less
```

> For the rendered HTML version, use [`/docs`](#-docs--htmldocs).

---

### `/docs` / `/html/docs` — Documentation (HTML)

- **Method:** `GET`
- **Format:** HTML
- **Authentication:** None

Same content as `/doc` but rendered as a styled HTML page, suitable for reading in a browser.

**Example:**

```bash
curl http://localhost:9215/docs
```

---

### `/details` — Ring Buffer Details (Text)

- **Method:** `GET`
- **Format:** Plain text
- **Authentication:** None

Exposes internal ring buffer statistics and scrape history data. Useful for:
- Diagnosing scrape latency trends
- Inspecting per-subgroup historical data
- Performance debugging

**Example:**

```bash
curl http://localhost:9215/details
```

> For the interactive HTML view, use [`/html/details`](#-htmldetails).

---

### `/html` / `/html/` — HTML Index

- **Method:** `GET`
- **Format:** HTML
- **Authentication:** None

The HTML navigation index — a browser-friendly starting page with links to all `/html/*` sub-pages.

**Example:**

```bash
curl http://localhost:9215/html
```

---

### `/html/details` — Ring Buffer Details (HTML)

- **Method:** `GET`
- **Format:** HTML
- **Authentication:** None

An interactive HTML view of subgroup history and ring buffer data. Allows visual inspection of scrape timings, per-group trends, and internal buffer usage in the browser.

**Example:**

```bash
curl http://localhost:9215/html/details
# Or open in browser: http://localhost:9215/html/details
```

---

### `/html/subgroups` — Subgroups (HTML)

- **Method:** `GET`
- **Format:** HTML
- **Authentication:** None

HTML-rendered view of the detected groups and subgroups (same data as `/subgroups`), formatted for browser inspection.

---

### `/html/health` — Health (HTML)

- **Method:** `GET`
- **Format:** HTML
- **Authentication:** None

HTML-rendered view of the exporter health statistics (same data as `/health`), formatted for browser inspection.

---

### `/html/config` — Configuration (HTML)

- **Method:** `GET`
- **Format:** HTML
- **Authentication:** None

HTML-rendered view of the active runtime configuration (same data as `/config`), formatted for browser inspection.

---

## TLS / HTTPS

When TLS is enabled (`enable_tls: true`), **all endpoints** above are served over HTTPS instead of HTTP. The port does not change automatically — configure it explicitly via `port` if needed.

```bash
# With TLS enabled
curl https://localhost:9215/metrics --cacert /path/to/ca.crt
```

> See [Configuration Reference](Configuration.md) for TLS setup details.

---

## Conditional Endpoints

Some endpoints can be disabled via configuration:

| Endpoint | Config Flag | Default |
|----------|-------------|---------|
| `/health`, `/html/health` | `enable_health` | `true` (enabled) |
| `herakles_exporter_*` metrics | `enable_telemetry` | `true` (enabled) |

---

## Quick Reference

```bash
# Prometheus metrics
curl http://localhost:9215/metrics

# Health check
curl http://localhost:9215/health

# Active configuration
curl http://localhost:9215/config

# Detected subgroups
curl http://localhost:9215/subgroups

# Text documentation
curl http://localhost:9215/doc

# Ring buffer / scrape history
curl http://localhost:9215/details
```

---

## Related Pages

- [Configuration Reference](Configuration.md)
- [Metrics Overview](Metrics-Overview.md)
- [Subgroups System](Subgroups-System.md)
- [Prometheus Integration](Prometheus-Integration.md)
- [Troubleshooting](Troubleshooting.md)

---

## 🔗 Project & Support

Project: https://github.com/cansp-dev/herakles-node-exporter — More info: https://www.herakles.now — Support: exporter@herakles.now