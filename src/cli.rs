//! CLI arguments and subcommands for herakles-proc-mem-exporter.
//!
//! This module defines the command-line interface structure using the clap library,
//! including all flags, options, and subcommands.

use clap::{Parser, Subcommand, ValueEnum};
use std::net::IpAddr;
use std::path::PathBuf;

/// Log level options for CLI parsing
#[derive(Debug, Clone, ValueEnum)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Configuration format options for output
#[derive(Debug, Clone, ValueEnum)]
pub enum ConfigFormat {
    Yaml,
    Json,
    Toml,
}

/// Main CLI arguments structure
#[derive(Parser, Debug)]
#[command(
    name = "herakles-proc-mem-exporter",
    about = "Prometheus exporter for per-process RSS/PSS/USS and CPU metrics",
    long_about = "Prometheus exporter for per-process RSS/PSS/USS and CPU metrics.\n\n\
                  A high-performance Prometheus exporter for per-process memory and CPU metrics \
                  on Linux systems. Provides detailed RSS, PSS, USS memory metrics and CPU usage \
                  with intelligent process classification.",
    author = "Michael Moll <proc-mem@herakles.io> - Herakles IO",
    version = "0.1.0",
    propagate_version = true,
    after_help = "Project: https://github.com/herakles-io/herakles-proc-mem-exporter — More info: https://www.herakles.io — Support: proc-mem@herakles.io"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// HTTP listen port
    #[arg(short = 'p', long)]
    pub port: Option<u16>,

    /// Bind to specific interface/IP
    #[arg(long)]
    pub bind: Option<IpAddr>,

    /// Log level
    #[arg(long, value_enum, default_value = "info")]
    pub log_level: LogLevel,

    /// Config file (YAML/JSON/TOML)
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Disable all config file loading
    #[arg(long)]
    pub no_config: bool,

    /// Print effective merged config and exit
    #[arg(long)]
    pub show_config: bool,

    /// Print only the loaded user config file + full path and exit
    #[arg(long)]
    pub show_user_config: bool,

    /// Output format for --show-config*
    #[arg(long, value_enum, default_value = "yaml")]
    pub config_format: ConfigFormat,

    /// Validate config and exit (return code 1 on error)
    #[arg(long)]
    pub check_config: bool,

    /// Enable /debug/pprof endpoints
    #[arg(long)]
    pub debug: bool,

    /// Cache metrics for N seconds
    #[arg(long)]
    pub cache_ttl: Option<u64>,

    /// Disable /health endpoint + health metrics
    #[arg(long)]
    pub disable_health: bool,

    /// Disable internal exporter_* metrics
    #[arg(long)]
    pub disable_telemetry: bool,

    /// Disable generic collectors
    #[arg(long)]
    pub disable_default_collectors: bool,

    /// Override IO buffer size (KB) for generic /proc readers
    #[arg(long)]
    pub io_buffer_kb: Option<usize>,

    /// Override buffer size (KB) for /proc/<pid>/smaps
    #[arg(long)]
    pub smaps_buffer_kb: Option<usize>,

    /// Override buffer size (KB) for /proc/<pid>/smaps_rollup
    #[arg(long)]
    pub smaps_rollup_buffer_kb: Option<usize>,

    /// Minimum USS in KB to include process
    #[arg(long)]
    pub min_uss_kb: Option<u64>,

    /// Include only processes matching these names (comma-separated)
    #[arg(long)]
    pub include_names: Option<String>,

    /// Exclude processes matching these names (comma-separated)
    #[arg(long)]
    pub exclude_names: Option<String>,

    /// Parallel processing threads (0 = auto)
    #[arg(long)]
    pub parallelism: Option<usize>,

    /// Maximum number of processes to scan
    #[arg(long)]
    pub max_processes: Option<usize>,

    /// Top-N processes to export per subgroup (override config)
    #[arg(long)]
    pub top_n_subgroup: Option<usize>,

    /// Top-N processes to export for "other" group (override config)
    #[arg(long)]
    pub top_n_others: Option<usize>,

    /// Path to JSON test data file (uses synthetic data instead of /proc)
    #[arg(short = 't', long)]
    pub test_data_file: Option<PathBuf>,

    /// Enable TLS/SSL for HTTPS
    #[arg(long)]
    pub enable_tls: bool,

    /// Path to TLS certificate file (PEM format)
    #[arg(long)]
    pub tls_cert: Option<PathBuf>,

    /// Path to TLS private key file (PEM format)
    #[arg(long)]
    pub tls_key: Option<PathBuf>,
}

/// Subcommands for additional functionality
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Validate configuration and system requirements
    Check {
        /// Check memory accessibility
        #[arg(long)]
        memory: bool,

        /// Check /proc filesystem
        #[arg(long)]
        proc: bool,

        /// Check all system requirements
        #[arg(long)]
        all: bool,
    },

    /// Generate configuration files
    Config {
        /// Output file path
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(long, value_enum, default_value = "yaml")]
        format: ConfigFormat,

        /// Include comments and examples
        #[arg(long)]
        commented: bool,
    },

    /// Test metrics collection
    Test {
        /// Number of test iterations
        #[arg(short = 'n', long, default_value_t = 1)]
        iterations: usize,

        /// Show detailed process information
        #[arg(long)]
        verbose: bool,

        /// Output format
        #[arg(long, value_enum, default_value = "yaml")]
        format: ConfigFormat,
    },

    /// List available process subgroups
    Subgroups {
        /// Show detailed matching rules
        #[arg(long)]
        verbose: bool,

        /// Filter by group name
        #[arg(short = 'g', long)]
        group: Option<String>,
    },

    /// Generate synthetic test data JSON file
    GenerateTestdata {
        /// Output file path
        #[arg(short = 'o', long, default_value = "testdata.json")]
        output: PathBuf,

        /// Minimum number of processes per subgroup
        #[arg(long, default_value_t = 6)]
        min_per_subgroup: usize,

        /// Number of "other" processes to generate
        #[arg(long, default_value_t = 12)]
        others_count: usize,
    },
}
