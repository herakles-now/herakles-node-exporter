# Subgroups System

The Herakles Process Memory Exporter uses a classification system to organize processes into groups and subgroups. This enables better analysis and filtering of process metrics.

## How Process Classification Works

1. **Process Name Extraction**: The exporter reads the process name from `/proc/<pid>/comm`
2. **Pattern Matching**: The name is matched against patterns defined in subgroup configurations
3. **Cmdline Matching**: If no match, the full command line can also be matched
4. **Default Classification**: Unmatched processes are classified as `group="other"`, `subgroup="other"`

### Classification Flow

```
                    ┌─────────────────────┐
                    │  Read /proc/<pid>   │
                    │     /comm           │
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │  Match against      │
                    │  subgroups.toml     │
                    │  patterns           │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
              ▼                ▼                ▼
        ┌───────────┐   ┌───────────┐   ┌───────────┐
        │  Match    │   │  No Match │   │  Cmdline  │
        │  Found    │   │  Try      │   │  Match    │
        └─────┬─────┘   │  Cmdline  │   │  Found    │
              │         └─────┬─────┘   └─────┬─────┘
              │               │               │
              ▼               ▼               ▼
        ┌───────────┐   ┌───────────┐   ┌───────────┐
        │ group/    │   │ other/    │   │ group/    │
        │ subgroup  │   │ other     │   │ subgroup  │
        └───────────┘   └───────────┘   └───────────┘
```

## Built-in Subgroups Overview

The exporter includes 140+ predefined subgroups organized into categories:

### Backup Solutions

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| backup | bacula | bacula-dir, bacula-sd, bacula-fd |
| backup | commvault | commvault, cvd, cvlaunch, cvfwd, cvmountd, cvrds, cvnet |
| backup | cohesity | cohesity-agent, cohesity-service, iris |
| backup | netbackup | nbjm, nbpem, nbemm, bprd, bpdbm, vnetd, bpjava, nbdisco, nbrb |
| backup | networker | nsrexecd, nsrd, savegrp, nsrindexd, nsrmmd |
| backup | rubrik | rubrik-agent, rubrik-gpsvc, rubrik-cdm |
| backup | spectrum_protect | dsmcad, dsmsched |
| backup | tsm | dsmc, dsmcad, dsmsched, dsmagent, dsmcsvc |
| backup | veeam | veeamagent, veeamtransport, veeam.guest, Veeam.Backup, VeeamAgent |

### Cache Systems

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| cache | memcached | memcached |
| cache | redis | redis-server, redis-sentinel |
| cache | varnish | varnishd |

### CI/CD & Automation

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| cicd | ansible | ansible |
| cicd | chef | chef-client, chef-server |
| cicd | gitlab | gitlab, sidekiq, puma |
| cicd | jenkins | jenkins |
| cicd | puppet | puppetagent, puppetmaster |
| cicd | saltstack | salt-master, salt-minion |
| cicd | terraform | terraform |

### Container & Orchestration

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| container | containerd | containerd |
| container | crio | crio |
| container | docker | dockerd, docker |
| container | kubelet | kubelet |
| container | podman | podman |

### Database Systems

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| db | cassandra | cassandra, cassandra-jvm |
| db | clickhouse | clickhouse-server, clickhouse-serv |
| db | cockroachdb | cockroach |
| db | couchbase | beam.smp, couchbase-server, memcached, cbq-engine |
| db | couchdb | couchdb |
| db | db2 | db2sysc, db2agent, db2tcpcm, db2wdog, db2vend, db2acd, db2resyn, db2ipccm |
| db | influxdb | influxd |
| db | mongodb | mongod, mongos |
| db | mssql | sqlservr |
| db | mysql | mysqld, mariadbd |
| db | oracle | oracle, tnslsnr, pmon, smon, dbwr, lgwr, ckpt, arch, reco, mmon, mmnl, dbr, arc0-3 |
| db | percona | percona-server, percona-xtradb-cluster |
| db | postgres | postgres, postmaster, autovacuum, walwriter, bgwriter, checkpointer |
| db | rethinkdb | rethinkdb |
| db | timescaledb | timescaledb |
| db | yugabyte | yb-* |

### ERP Systems

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| erp | peoplesoft | psadmin, psserver, psappsrv, psprcsrv, psqcksrv |
| erp | sap | sapstart, saposcol, disp+work, gwrd, icman, enqrep, enqwork, msg_server, jstart, jlaunch, jcontrol, jexec |

### Logging & SIEM

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| logging | elasticsearch | elasticsearch |
| logging | filebeat | filebeat |
| logging | fluentd | fluentd |
| logging | graylog | graylog-server |
| logging | kibana | kibana |
| logging | log_collectors | vector, nxlog |
| logging | logstash | logstash |
| logging | splunk | splunkd |

### Messaging & Queueing

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| messaging | activemq | (cmdline: org.apache.activemq) |
| messaging | kafka | kafka |
| messaging | nats | nats-server |
| messaging | nsq | nsqd, nsqlookupd |
| messaging | pulsar | pulsar |
| messaging | rabbitmq | beam.smp, epmd, rabbitmq-server |
| messaging | zeromq | zeromq |

### Monitoring & Observability

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| monitoring | alertmanager | alertmanager |
| monitoring | blackbox | blackbox_exporter |
| monitoring | grafana | grafana, grafana-server |
| monitoring | icinga_nagios | nagios, icinga2, nrpe |
| monitoring | node_exporter | node_exporter |
| monitoring | prometheus | prometheus |
| monitoring | telegraf | telegraf |
| monitoring | thanos | thanos |
| monitoring | victoriametrics | victoria-metrics, victoriametrics, vmstorage, vminsert, vmselect |
| monitoring | zabbix | zabbix_agentd, zabbix_server, zabbix_proxy |

### Network Services

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| network | bind | named |
| network | dhcp | dhcpd |
| network | haproxy | haproxy |
| network | keepalived | keepalived |
| network | ntp | ntpd, chronyd |
| network | proxy_squid | squid |
| network | vpn | openvpn, strongswan |

### Web & Application Servers

| Group | Subgroup | Process Matches |
|-------|----------|-----------------|
| web | apache | httpd, apache2 |
| web | caddy | caddy |
| web | nginx | nginx |
| web | tomcat | tomcat, (cmdline: org.apache.catalina.startup.Bootstrap) |
| web | weblogic | (cmdline: weblogic.Server, weblogic.nodemanager.NMMain) |
| web | websphere | (cmdline: com.ibm.ws.runtime.WSStart, wasadmin) |

## Creating Custom Subgroups

### TOML File Format

Create a `subgroups.toml` file with your custom definitions:

```toml
subgroups = [
  # Basic match by process name
  { group = "myapp", subgroup = "api", matches = [
    "myapp-api",
    "api-server",
    "myapp-gateway",
  ] },
  
  # Match by command line (for Java apps, scripts, etc.)
  { group = "myapp", subgroup = "worker", cmdline_matches = [
    "java.*com.myapp.Worker",
    "python.*worker.py",
  ] },
  
  # Combined name and cmdline matching
  { group = "myapp", subgroup = "scheduler", 
    matches = ["myapp-scheduler"],
    cmdline_matches = ["cron.*myapp"]
  },
]
```

### File Locations and Precedence

Custom subgroups are loaded from multiple locations (later files override earlier):

1. **Built-in subgroups** - Compiled into the binary from `data/subgroups.toml`
2. **System-wide** - `/etc/herakles/subgroups.toml`
3. **Current directory** - `./subgroups.toml`

### Example: Custom Application Monitoring

```toml
# /etc/herakles/subgroups.toml - Custom subgroups for our stack

subgroups = [
  # Our custom microservices
  { group = "acme", subgroup = "user-service", matches = ["user-service", "user-api"] },
  { group = "acme", subgroup = "order-service", matches = ["order-service", "order-api"] },
  { group = "acme", subgroup = "payment-service", matches = ["payment-service"] },
  { group = "acme", subgroup = "inventory-service", matches = ["inventory-api"] },
  
  # Our batch jobs
  { group = "acme-batch", subgroup = "data-sync", matches = ["data-sync-job"] },
  { group = "acme-batch", subgroup = "report-gen", cmdline_matches = ["python.*generate_reports.py"] },
  
  # Third-party services we use
  { group = "vendor", subgroup = "datadog", matches = ["dd-agent", "datadog-agent"] },
  { group = "vendor", subgroup = "newrelic", matches = ["newrelic-infra"] },
]
```

## Search Mode Filters

Use configuration options to filter which groups/subgroups are monitored.

### Include Mode

Only monitor specified groups/subgroups:

```yaml
search_mode: "include"
search_groups:
  - db
  - web
  - acme
search_subgroups:
  - prometheus
  - grafana
```

### Exclude Mode

Monitor everything except specified groups/subgroups:

```yaml
search_mode: "exclude"
search_groups:
  - system
  - kernel
search_subgroups:
  - unknown
```

### Disable "Other" Processes

Skip all unclassified processes:

```yaml
disable_others: true
```

## Top-N Configuration per Subgroup Type

Control how many processes are exported per subgroup:

```yaml
# For defined subgroups (db, web, etc.)
top_n_subgroup: 5

# For "other" group (unclassified processes)
top_n_others: 10
```

## CLI Commands for Subgroups

### List All Subgroups

```bash
herakles-proc-mem-exporter subgroups
```

### List with Detailed Matching Rules

```bash
herakles-proc-mem-exporter subgroups --verbose
```

### Filter by Group

```bash
herakles-proc-mem-exporter subgroups --group db
```

### View Subgroups via HTTP

```bash
curl http://localhost:9215/subgroups
```

## Examples for Different Use Cases

### Database-Focused Monitoring

```yaml
search_mode: "include"
search_groups:
  - db
  - cache
search_subgroups:
  - redis
  - memcached
disable_others: true
top_n_subgroup: 10
```

Custom subgroups for specific databases:

```toml
subgroups = [
  { group = "db", subgroup = "mysql_cluster", matches = ["mysql-router", "mysqld-cluster"] },
  { group = "db", subgroup = "patroni", matches = ["patroni", "patroni-api"] },
]
```

### Kubernetes/Container Monitoring

```yaml
search_mode: "include"
search_groups:
  - container
  - monitoring
search_subgroups:
  - kubelet
  - containerd
  - prometheus
top_n_subgroup: 20
```

### Java Application Monitoring

```toml
subgroups = [
  { group = "java", subgroup = "spring-boot", cmdline_matches = [
    "org.springframework.boot.loader.JarLauncher",
    "org.springframework.boot.loader.WarLauncher",
  ] },
  { group = "java", subgroup = "jvm-services", matches = ["java"] },
  { group = "java", subgroup = "kafka-streams", cmdline_matches = ["org.apache.kafka.streams"] },
]
```

### Multi-Tenant Environment

```toml
subgroups = [
  # Tenant A
  { group = "tenant-a", subgroup = "api", matches = ["tenant-a-api"] },
  { group = "tenant-a", subgroup = "worker", matches = ["tenant-a-worker"] },
  
  # Tenant B
  { group = "tenant-b", subgroup = "api", matches = ["tenant-b-api"] },
  { group = "tenant-b", subgroup = "worker", matches = ["tenant-b-worker"] },
]
```

```yaml
# Config to monitor specific tenant
search_mode: "include"
search_groups:
  - tenant-a
```

## Next Steps

- [Configure Prometheus integration](Prometheus-Integration.md)
- [Set up alerting based on subgroups](Alerting-Examples.md)
- [Performance tuning for large subgroup sets](Performance-Tuning.md)

## 🔗 Project & Support

Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io
