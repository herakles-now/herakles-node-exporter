//! CLI command implementations for herakles-proc-mem-exporter.
//!
//! This module provides implementations for all CLI subcommands:
//! - `check`: System validation
//! - `config`: Configuration file generation
//! - `test`: Metrics collection testing
//! - `subgroups`: Subgroup listing
//! - `generate`: Test data generation

pub mod check;
pub mod config;
pub mod generate;
pub mod subgroups;
pub mod test;

// Re-export command functions
pub use check::command_check;
pub use config::command_config;
pub use generate::command_generate_testdata;
pub use subgroups::command_subgroups;
pub use test::command_test;
