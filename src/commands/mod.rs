//! CLI command implementations for herakles-node-exporter.
//!
//! This module provides implementations for all CLI subcommands:
//! - `check`: System validation
//! - `config`: Configuration file generation
//! - `test`: Metrics collection testing
//! - `subgroups`: Subgroup listing
//! - `generate`: Test data generation
//! - `install`: System-wide installation
//! - `uninstall`: System-wide uninstallation

pub mod check;
pub mod config;
pub mod generate;
pub mod install;
pub mod subgroups;
pub mod test;
pub mod uninstall;

// Re-export command functions
pub use check::command_check;
pub use config::command_config;
pub use generate::command_generate_testdata;
pub use install::command_install;
pub use subgroups::command_subgroups;
pub use test::command_test;
pub use uninstall::command_uninstall;
