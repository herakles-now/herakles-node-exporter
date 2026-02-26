# Testing

This guide covers testing approaches for the Herakles Process Memory Exporter.

## Test Mode with Real /proc Data

### Running Test Mode

The exporter includes a test command that performs metrics collection without starting the HTTP server:

```bash
# Single test iteration
herakles-node-exporter test

# Multiple iterations with verbose output
herakles-node-exporter test -n 5 --verbose

# Output in different formats
herakles-node-exporter test --format yaml
herakles-node-exporter test --format json
```

### Sample Output

```
🧪 Herakles Process Memory Exporter - Test Mode
================================================

🔄 Iteration 1/1:
   📁 Found 156 process entries

   Top 10 processes by USS:
   ┌─────────────────────────────────────────────────────────────────────────────┐
   │ PID      NAME                 GROUP        SUBGROUP     RSS      PSS      USS     │
   ├─────────────────────────────────────────────────────────────────────────────┤
   │ 1234     postgres             db           postgres     512MB    450MB    400MB   │
   │ 5678     java                 runtime      java         2.1GB    1.9GB    1.8GB   │
   │ 9012     nginx                web          nginx        156MB    120MB    100MB   │
   └─────────────────────────────────────────────────────────────────────────────┘

   ⏱️  Scan duration: 45.23ms
   📊 Processes exported: 156

✅ Test completed successfully
```

## Generating Synthetic Test Data

### Create Test Data File

Generate a JSON file with synthetic process data:

```bash
# Default settings
herakles-node-exporter generate-testdata -o testdata.json

# Custom parameters
herakles-node-exporter generate-testdata \
  -o testdata.json \
  --min-per-subgroup 10 \
  --others-count 50
```

### Test Data Format

The generated file follows this JSON schema:

```json
{
  "metadata": {
    "generated_at": "2024-01-15T10:30:00Z",
    "generator": "herakles-node-exporter",
    "version": "0.1.0"
  },
  "processes": [
    {
      "pid": 1001,
      "name": "postgres",
      "group": "db",
      "subgroup": "postgres",
      "rss": 536870912,
      "pss": 469762048,
      "uss": 402653184,
      "cpu_percent": 2.5,
      "cpu_time_seconds": 1234.56
    },
    {
      "pid": 1002,
      "name": "nginx",
      "group": "web",
      "subgroup": "nginx",
      "rss": 134217728,
      "pss": 104857600,
      "uss": 83886080,
      "cpu_percent": 0.8,
      "cpu_time_seconds": 567.89
    }
  ]
}
```

## Using Test Data Files

### Running with Test Data

Start the exporter with synthetic data instead of /proc:

```bash
# Via CLI flag
herakles-node-exporter -t testdata.json

# Via config file
# config.yaml
test_data_file: /path/to/testdata.json
```

### Benefits of Test Data

- **Reproducible testing**: Same data every run
- **CI/CD friendly**: No /proc dependency
- **Development**: Test without real processes
- **Documentation**: Create example metrics

### Creating Custom Test Scenarios

#### High Cardinality Test

```json
{
  "processes": [
    // Generate 1000+ processes
    {"pid": 1, "name": "proc1", "group": "test", "subgroup": "load", ...},
    {"pid": 2, "name": "proc2", "group": "test", "subgroup": "load", ...},
    // ...
  ]
}
```

#### Memory Leak Simulation

Create multiple files with increasing memory values:

```bash
# Generate initial state
herakles-node-exporter generate-testdata -o state1.json

# Manually modify values to simulate growth
# state2.json - increase memory values by 10%
# state3.json - increase memory values by 20%
```

## Integration Testing with Prometheus

### Docker Compose Test Setup

```yaml
# docker-compose.test.yml
version: '3.8'

services:
  herakles-exporter:
    build: .
    ports:
      - "9215:9215"
    volumes:
      - ./testdata.json:/testdata.json:ro
    command: ["-t", "/testdata.json"]

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus-test.yml:/etc/prometheus/prometheus.yml:ro
    depends_on:
      - herakles-exporter

  test-runner:
    image: curlimages/curl:latest
    depends_on:
      - prometheus
    command: >
      sh -c "
        sleep 30 &&
        curl -s http://prometheus:9090/api/v1/query?query=herakles_mem_process_rss_bytes |
        grep -q 'postgres' && echo 'Test PASSED' || echo 'Test FAILED'
      "
```

### Prometheus Test Configuration

```yaml
# prometheus-test.yml
global:
  scrape_interval: 10s

scrape_configs:
  - job_name: 'herakles-proc-mem'
    static_configs:
      - targets: ['herakles-exporter:9215']
```

### Running Integration Tests

```bash
# Start test environment
docker-compose -f docker-compose.test.yml up -d

# Wait for data collection
sleep 60

# Query Prometheus
curl 'http://localhost:9090/api/v1/query?query=herakles_mem_process_rss_bytes' | jq .

# Verify specific metrics exist
curl -s 'http://localhost:9090/api/v1/query?query=herakles_mem_group_rss_bytes' | \
  jq '.data.result | length'

# Cleanup
docker-compose -f docker-compose.test.yml down
```

## Load Testing

### Continuous Scrape Test

```bash
# Install hey (HTTP load generator)
go install github.com/rakyll/hey@latest

# Load test /metrics endpoint
hey -z 60s -c 10 http://localhost:9215/metrics

# Expected output shows latency distribution
Summary:
  Total:        60.0234 secs
  Slowest:      0.0892 secs
  Fastest:      0.0089 secs
  Average:      0.0156 secs
  Requests/sec: 640.25
```

### Concurrent Scrape Simulation

```bash
# Simulate multiple Prometheus servers scraping
for i in {1..5}; do
  while true; do
    curl -s http://localhost:9215/metrics > /dev/null
    sleep 10
  done &
done

# Monitor exporter health
watch -n 5 'curl -s http://localhost:9215/health | head -20'
```

### Memory Pressure Test

```bash
# Monitor memory during load
while true; do
  ps -o pid,rss,vsz,comm -p $(pgrep herakles-proc-mem)
  sleep 5
done
```

## CI/CD Integration Examples

### GitHub Actions

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      
      - name: Build
        run: cargo build --release
      
      - name: Run unit tests
        run: cargo test
      
      - name: Generate test data
        run: ./target/release/herakles-node-exporter generate-testdata -o testdata.json
      
      - name: Run with test data
        run: |
          ./target/release/herakles-node-exporter -t testdata.json &
          sleep 5
          curl -f http://localhost:9215/metrics | grep -E "herakles_(mem|cpu|exporter)_"
          curl -f http://localhost:9215/health
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - build
  - test

build:
  stage: build
  image: rust:1.75
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/herakles-node-exporter

test:
  stage: test
  image: rust:1.75
  script:
    - ./target/release/herakles-node-exporter generate-testdata -o testdata.json
    - ./target/release/herakles-node-exporter -t testdata.json &
    - sleep 5
    - curl -f http://localhost:9215/metrics
    - curl -f http://localhost:9215/health
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any
    
    stages {
        stage('Build') {
            steps {
                sh 'cargo build --release'
            }
        }
        
        stage('Test') {
            steps {
                sh '''
                    ./target/release/herakles-node-exporter generate-testdata -o testdata.json
                    ./target/release/herakles-node-exporter -t testdata.json &
                    sleep 5
                    curl -f http://localhost:9215/metrics | grep -cE "herakles_(mem|cpu|exporter)_"
                '''
            }
        }
    }
}
```

## Verification Checks

### Configuration Validation

```bash
# Validate config file
herakles-node-exporter --check-config -c config.yaml

# Expected output on success
✅ Configuration is valid

# Expected output on error
❌ Configuration invalid: search_mode is set to include/exclude, but no search_groups or search_subgroups defined
```

### System Check

```bash
# Full system verification
herakles-node-exporter check --all

# Individual checks
herakles-node-exporter check --memory
herakles-node-exporter check --proc
```

### Metrics Verification

```bash
# Check metrics are being exported
curl -s http://localhost:9215/metrics | grep -E '^herakles_(mem|cpu|exporter)_' | wc -l

# Verify specific metric types
curl -s http://localhost:9215/metrics | grep 'herakles_mem_process_rss_bytes{'
curl -s http://localhost:9215/metrics | grep 'herakles_(mem|cpu)_group_'
curl -s http://localhost:9215/metrics | grep 'herakles_(mem|cpu)_top_process_'
```

### Health Verification

```bash
# Check health endpoint
curl -s http://localhost:9215/health

# Verify cache is updating
curl -s http://localhost:9215/health | grep 'cache_update_duration'

# Monitor over time
watch -n 5 'curl -s http://localhost:9215/health | grep -E "scanned_processes|cache"'
```

## Test Scenarios Checklist

| Scenario | Test Method | Expected Result |
|----------|-------------|-----------------|
| Basic functionality | `test` command | Processes listed |
| Config validation | `--check-config` | Valid or specific error |
| System requirements | `check --all` | All checks pass |
| HTTP endpoints | curl /metrics | Prometheus format output |
| Health reporting | curl /health | Stats table displayed |
| Test data mode | `-t testdata.json` | Uses synthetic data |
| High load | hey/ab tool | Stable latency |
| Memory stability | Long-running + monitoring | No memory growth |

## Next Steps

- [Contributing guidelines](Contributing.md)
- [Architecture overview](Architecture.md)
- [Troubleshooting guide](Troubleshooting.md)
