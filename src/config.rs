//! Configuration management for herakles-node-exporter.
//!
//! This module handles loading, merging, and validating configuration from files
//! and CLI arguments. It supports YAML, JSON, and TOML formats.

use crate::cli::{Args, ConfigFormat};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

// Default configuration constants
pub const DEFAULT_BIND_ADDR: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 9215;
pub const DEFAULT_CACHE_TTL: u64 = 30;

/// Ringbuffer configuration for historical metrics tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingbufferConfig {
    /// Maximum memory for all ringbuffers in MB (default: 15)
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: usize,

    /// Sampling interval in seconds (default: 30)
    #[serde(default = "default_interval_seconds")]
    pub interval_seconds: u64,

    /// Minimum entries per subgroup (default: 10)
    #[serde(default = "default_min_entries")]
    pub min_entries_per_subgroup: usize,

    /// Maximum entries per subgroup (default: 120)
    #[serde(default = "default_max_entries")]
    pub max_entries_per_subgroup: usize,
}

fn default_max_memory_mb() -> usize {
    15
}
fn default_interval_seconds() -> u64 {
    30
}
fn default_min_entries() -> usize {
    10
}
fn default_max_entries() -> usize {
    120
}

impl Default for RingbufferConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            interval_seconds: default_interval_seconds(),
            min_entries_per_subgroup: default_min_entries(),
            max_entries_per_subgroup: default_max_entries(),
        }
    }
}

/// Enhanced configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Server configuration
    pub port: Option<u16>,
    pub bind: Option<String>,

    // Metrics collection
    pub min_uss_kb: Option<u64>,
    pub include_names: Option<Vec<String>>,
    pub exclude_names: Option<Vec<String>>,
    pub parallelism: Option<usize>,
    pub max_processes: Option<usize>,

    // Performance tuning
    pub cache_ttl: Option<u64>,
    pub io_buffer_kb: Option<usize>,
    pub smaps_buffer_kb: Option<usize>,
    pub smaps_rollup_buffer_kb: Option<usize>,

    // Feature flags
    pub enable_health: Option<bool>,
    pub enable_telemetry: Option<bool>,
    pub enable_default_collectors: Option<bool>,
    pub enable_pprof: Option<bool>,

    // Logging
    pub log_level: Option<String>,
    pub enable_file_logging: Option<bool>,
    pub log_file: Option<PathBuf>,

    // Classification / search engine
    /// "include" | "exclude" | None
    #[serde(alias = "modify-search-engine")]
    pub search_mode: Option<String>,
    /// List of group names
    #[serde(alias = "groups")]
    pub search_groups: Option<Vec<String>>,
    /// List of subgroup names
    #[serde(alias = "subgroups")]
    pub search_subgroups: Option<Vec<String>>,
    /// If true, completely ignore "other"/"unknown" processes
    #[serde(alias = "disable-others")]
    pub disable_others: Option<bool>,
    /// Top-N processes to export per subgroup (non-"other" groups)
    #[serde(alias = "top-n-subgroup")]
    pub top_n_subgroup: Option<usize>,
    /// Top-N processes to export for "other" group
    #[serde(alias = "top-n-others")]
    pub top_n_others: Option<usize>,
    /// Top-N processes to display in /details endpoint (default: 5)
    #[serde(alias = "details-top-n")]
    pub details_top_n: Option<usize>,

    // Metrics enable flags
    #[serde(alias = "enable-rss")]
    pub enable_rss: Option<bool>,
    #[serde(alias = "enable-pss")]
    pub enable_pss: Option<bool>,
    #[serde(alias = "enable-uss")]
    pub enable_uss: Option<bool>,
    #[serde(alias = "enable-cpu")]
    pub enable_cpu: Option<bool>,

    /// Path to JSON test data file (uses synthetic data instead of /proc)
    #[serde(alias = "test-data-file")]
    pub test_data_file: Option<PathBuf>,

    // TLS/SSL Configuration
    #[serde(alias = "enable-tls")]
    pub enable_tls: Option<bool>,
    #[serde(alias = "tls-cert-path")]
    pub tls_cert_path: Option<String>,
    #[serde(alias = "tls-key-path")]
    pub tls_key_path: Option<String>,

    // eBPF Configuration
    #[serde(alias = "enable-ebpf")]
    pub enable_ebpf: Option<bool>,
    #[serde(alias = "enable-ebpf-network")]
    pub enable_ebpf_network: Option<bool>,
    #[serde(alias = "enable-ebpf-disk")]
    pub enable_ebpf_disk: Option<bool>,
    #[serde(alias = "enable-tcp-tracking")]
    pub enable_tcp_tracking: Option<bool>,

    // Collector enable flags
    #[serde(alias = "enable-filesystem-collector")]
    pub enable_filesystem_collector: Option<bool>,
    #[serde(alias = "enable-thermal-collector")]
    pub enable_thermal_collector: Option<bool>,
    #[serde(alias = "enable-psi-collector")]
    pub enable_psi_collector: Option<bool>,

    // Ringbuffer Configuration
    #[serde(default)]
    pub ringbuffer: RingbufferConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: Some(DEFAULT_BIND_ADDR.to_string()),
            port: Some(DEFAULT_PORT),
            min_uss_kb: Some(0),
            include_names: None,
            exclude_names: None,
            parallelism: None,
            max_processes: None,
            cache_ttl: Some(DEFAULT_CACHE_TTL),
            io_buffer_kb: Some(256),
            smaps_buffer_kb: Some(512),
            smaps_rollup_buffer_kb: Some(256),
            enable_health: Some(true),
            enable_telemetry: Some(true),
            enable_default_collectors: Some(true),
            enable_pprof: Some(false),
            log_level: Some("info".into()),
            enable_file_logging: Some(false),
            log_file: None,
            search_mode: None,
            search_groups: None,
            search_subgroups: None,
            disable_others: Some(false),
            top_n_subgroup: Some(3),
            top_n_others: Some(10),
            details_top_n: Some(5),
            enable_rss: Some(true),
            enable_pss: Some(true),
            enable_uss: Some(true),
            enable_cpu: Some(true),
            test_data_file: None,
            enable_tls: Some(false),
            tls_cert_path: None,
            tls_key_path: None,
            enable_ebpf: Some(true),
            enable_ebpf_network: Some(true),
            enable_ebpf_disk: Some(true),
            enable_tcp_tracking: Some(true),
            enable_filesystem_collector: Some(true),
            enable_thermal_collector: Some(true),
            enable_psi_collector: Some(true),
            ringbuffer: RingbufferConfig::default(),
        }
    }
}

/// Validate effective config (used by --check-config and at startup)
pub fn validate_effective_config(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Metrics flags: at least one must be true
    let enable_rss = cfg.enable_rss.unwrap_or(true);
    let enable_pss = cfg.enable_pss.unwrap_or(true);
    let enable_uss = cfg.enable_uss.unwrap_or(true);
    let enable_cpu = cfg.enable_cpu.unwrap_or(true);

    if !(enable_rss || enable_pss || enable_uss || enable_cpu) {
        return Err(
            "At least one of enable_rss/enable_pss/enable_uss/enable_cpu must be true".into(),
        );
    }

    // Search mode validation
    if let Some(mode) = cfg.search_mode.as_deref() {
        let has_groups = cfg.search_groups.as_ref().is_some_and(|v| !v.is_empty());
        let has_subgroups = cfg.search_subgroups.as_ref().is_some_and(|v| !v.is_empty());

        match mode {
            "include" | "exclude" => {
                if !(has_groups || has_subgroups) {
                    return Err("search_mode is set to include/exclude, \
                        but no search_groups or search_subgroups defined"
                        .into());
                }
            }
            other => {
                return Err(format!(
                    "Invalid search_mode '{}', expected 'include' or 'exclude'",
                    other
                )
                .into());
            }
        }
    }

    // TLS validation
    if cfg.enable_tls.unwrap_or(false) {
        let cert_path = cfg.tls_cert_path.as_deref();
        let key_path = cfg.tls_key_path.as_deref();

        match (cert_path, key_path) {
            (None, None) => {
                return Err(
                    "TLS is enabled but neither tls_cert_path nor tls_key_path are set".into(),
                );
            }
            (Some(_), None) => {
                return Err("TLS is enabled but tls_key_path is not set".into());
            }
            (None, Some(_)) => {
                return Err("TLS is enabled but tls_cert_path is not set".into());
            }
            (Some(cert), Some(key)) => {
                // Check if files exist
                let cert_path = std::path::Path::new(cert);
                let key_path = std::path::Path::new(key);

                if !cert_path.exists() {
                    return Err(format!("TLS certificate file not found: {}", cert).into());
                }
                if !key_path.exists() {
                    return Err(format!("TLS private key file not found: {}", key).into());
                }

                // Check if files are readable and not empty
                match std::fs::metadata(cert_path) {
                    Ok(meta) if meta.len() == 0 => {
                        return Err(format!("TLS certificate file is empty: {}", cert).into());
                    }
                    Err(e) => {
                        return Err(format!(
                            "TLS certificate file is not readable: {} ({})",
                            cert, e
                        )
                        .into());
                    }
                    Ok(_) => {}
                }

                match std::fs::metadata(key_path) {
                    Ok(meta) if meta.len() == 0 => {
                        return Err(format!("TLS private key file is empty: {}", key).into());
                    }
                    Err(e) => {
                        return Err(format!(
                            "TLS private key file is not readable: {} ({})",
                            key, e
                        )
                        .into());
                    }
                    Ok(_) => {}
                }
            }
        }
    }

    Ok(())
}

/// Resolves configuration from CLI args, config file, and defaults.
/// This enforces precedence: CLI (if provided) > config file > default.
pub fn resolve_config(args: &Args) -> Result<Config, Box<dyn std::error::Error>> {
    let mut config = if args.no_config {
        Config::default()
    } else {
        load_config(args.config.as_deref().and_then(|p| p.to_str()))?
    };

    // Override with CLI args
    if let Some(bind_ip) = args.bind {
        config.bind = Some(bind_ip.to_string());
    }

    // Only override port if the user supplied it on the CLI.
    if let Some(cli_port) = args.port {
        config.port = Some(cli_port);
    }

    if args.min_uss_kb.is_some() {
        config.min_uss_kb = args.min_uss_kb;
    }

    // Parse comma-separated include/exclude names
    if let Some(include_str) = &args.include_names {
        config.include_names = Some(
            include_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        );
    }

    if let Some(exclude_str) = &args.exclude_names {
        config.exclude_names = Some(
            exclude_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        );
    }

    // Performance settings
    if let Some(io_buffer_kb) = args.io_buffer_kb {
        config.io_buffer_kb = Some(io_buffer_kb);
    }
    if let Some(smaps_buffer_kb) = args.smaps_buffer_kb {
        config.smaps_buffer_kb = Some(smaps_buffer_kb);
    }
    if let Some(smaps_rollup_buffer_kb) = args.smaps_rollup_buffer_kb {
        config.smaps_rollup_buffer_kb = Some(smaps_rollup_buffer_kb);
    }
    if let Some(cache_ttl) = args.cache_ttl {
        config.cache_ttl = Some(cache_ttl);
    }

    // Top-N overrides: CLI wins if provided
    if let Some(n) = args.top_n_subgroup {
        config.top_n_subgroup = Some(n);
    }
    if let Some(n) = args.top_n_others {
        config.top_n_others = Some(n);
    }

    // Feature flags
    if args.disable_health {
        config.enable_health = Some(false);
    }
    if args.disable_telemetry {
        config.enable_telemetry = Some(false);
    }
    if args.disable_default_collectors {
        config.enable_default_collectors = Some(false);
    }
    if args.debug {
        config.enable_pprof = Some(true);
    }

    // Test data file: CLI wins if provided
    if let Some(test_file) = &args.test_data_file {
        config.test_data_file = Some(test_file.clone());
    }

    // TLS configuration: CLI wins if provided
    if args.enable_tls {
        config.enable_tls = Some(true);
    }
    if let Some(cert_path) = &args.tls_cert {
        config.tls_cert_path = Some(cert_path.to_string_lossy().to_string());
    }
    if let Some(key_path) = &args.tls_key {
        config.tls_key_path = Some(key_path.to_string_lossy().to_string());
    }

    // eBPF configuration: CLI wins if provided
    if args.enable_ebpf {
        config.enable_ebpf = Some(true);
    }
    if args.enable_ebpf_network {
        config.enable_ebpf_network = Some(true);
    }
    if args.disable_ebpf_network {
        config.enable_ebpf_network = Some(false);
    }
    if args.enable_ebpf_disk {
        config.enable_ebpf_disk = Some(true);
    }
    if args.disable_ebpf_disk {
        config.enable_ebpf_disk = Some(false);
    }
    if args.enable_tcp_tracking {
        config.enable_tcp_tracking = Some(true);
    }
    if args.disable_tcp_tracking {
        config.enable_tcp_tracking = Some(false);
    }

    Ok(config)
}

/// Enhanced configuration loading with multiple format support
pub fn load_config(path: Option<&str>) -> Result<Config, Box<dyn std::error::Error>> {
    let path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        // Try default locations
        let defaults = [
            "/etc/herakles/node-exporter.yaml",
            "/etc/herakles/node-exporter.yml",
            "/etc/herakles/node-exporter.json",
            "./herakles-node-exporter.yaml",
            "./herakles-node-exporter.yml",
            "./herakles-node-exporter.json",
        ];

        defaults
            .iter()
            .find(|p| Path::new(p).exists())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(""))
    };

    if !path.exists() || path.to_string_lossy().is_empty() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&path)?;

    match path.extension().and_then(|s| s.to_str()) {
        Some("json") => {
            let config: Config = serde_json::from_str(&content)?;
            info!("Loaded JSON configuration from: {}", path.display());
            Ok(config)
        }
        Some("toml") => {
            let config: Config = toml::from_str(&content)?;
            info!("Loaded TOML configuration from: {}", path.display());
            Ok(config)
        }
        _ => {
            // Default to YAML
            let config: Config = serde_yaml::from_str(&content)?;
            info!("Loaded YAML configuration from: {}", path.display());
            Ok(config)
        }
    }
}

/// Shows configuration in requested format
pub fn show_config(
    config: &Config,
    format: ConfigFormat,
    user_config: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = match format {
        ConfigFormat::Json => serde_json::to_string_pretty(config)?,
        ConfigFormat::Toml => toml::to_string_pretty(config)?,
        ConfigFormat::Yaml => serde_yaml::to_string(config)?,
    };

    if user_config {
        println!("User configuration (effective values):");
    }
    println!("{output}");
    Ok(())
}
