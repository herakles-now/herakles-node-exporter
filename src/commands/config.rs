//! Config command implementation.
//!
//! Generates configuration files in various formats.

use std::fs;
use std::path::PathBuf;

use crate::cli::ConfigFormat;
use crate::config::Config;

/// Generates configuration files.
pub fn command_config(
    output: Option<PathBuf>,
    format: ConfigFormat,
    commented: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let output = match output {
        Some(path) => path,
        None => PathBuf::from("herakles-node-exporter.yaml"),
    };

    let content = match format {
        ConfigFormat::Json => serde_json::to_string_pretty(&config)?,
        ConfigFormat::Toml => toml::to_string_pretty(&config)?,
        ConfigFormat::Yaml => {
            let mut content = serde_yaml::to_string(&config)?;
            if commented {
                content = add_config_comments(content);
            }
            content
        }
    };

    if output.to_string_lossy() == "-" {
        print!("{}", content);
    } else {
        fs::write(&output, content)?;
        println!("✅ Configuration written to: {}", output.display());
    }

    Ok(())
}

/// Adds comments to YAML configuration.
fn add_config_comments(yaml: String) -> String {
    let comments = r#"# Herakles Process Memory Exporter Configuration
# =================================================
#
# Server Configuration
# --------------------
# bind: "0.0.0.0"              # Bind IP (0.0.0.0 = all interfaces)
# port: 9215                   # HTTP port
#
# Metrics Collection
# ------------------
# min_uss_kb: 0                # Minimum USS in KB to include process
# include_names: null          # Include only processes matching these names
# exclude_names: null          # Exclude processes matching these names
# parallelism: null            # Parallel threads (null = auto)
# max_processes: null          # Maximum processes to scan
#
# Performance Tuning
# ------------------
# cache_ttl: 30                # Cache metrics for N seconds
# io_buffer_kb: 256            # Buffer size for generic /proc readers
# smaps_buffer_kb: 512         # Buffer size for smaps parsing
# smaps_rollup_buffer_kb: 256  # Buffer size for smaps_rollup parsing
#
# Feature Flags
# -------------
# enable_health: true          # Enable /health endpoint
# enable_telemetry: true       # Enable internal metrics
# enable_default_collectors: true # Enable generic collectors
# enable_pprof: false          # Enable /debug/pprof endpoints
#
# Logging
# -------
# log_level: "info"            # off, error, warn, info, debug, trace
# enable_file_logging: false   # Enable file logging
# log_file: null               # Log file path (null = stderr)
#
# Classification / Search Engine
# ------------------------------
# search_mode: null            # "include" or "exclude" or null for disabled
# search_groups: null          # List of group names (e.g. ["db", "system"])
# search_subgroups: null       # List of subgroup names (e.g. ["postgres", "nginx"])
# disable_others: false        # Skip 'other/unknown' processes completely
# top_n_subgroup: 3          # Top-N processes per subgroup (non-"other" groups)
# top_n_others: 10           # Top-N processes for "other" group
#
# Metrics Enable Flags
# --------------------
# enable_rss: true             # Export RSS metrics
# enable_pss: true             # Export PSS metrics
# enable_uss: true             # Export USS metrics
# enable_cpu: true             # Export CPU metrics
#
# Collector Enable Flags
# ----------------------
# enable_filesystem_collector: true  # Enable filesystem metrics collection
# enable_thermal_collector: true     # Enable CPU/thermal sensors
# enable_psi_collector: true         # Enable PSI (Pressure Stall Information)
#
# TLS/SSL Configuration
# ---------------------
# enable_tls: false            # Enable HTTPS (default: false)
# tls_cert_path: null          # Path to TLS certificate (PEM format)
# tls_key_path: null           # Path to TLS private key (PEM format)
"#;

    format!("{comments}\n{yaml}")
}
