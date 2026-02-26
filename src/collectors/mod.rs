//! Collectors module for system metrics.
//!
//! This module contains various collectors for system-level metrics such as
//! disk I/O, filesystem usage, network interface statistics, and thermal sensors.

pub mod diskstats;
pub mod filesystem;
pub mod netdev;
pub mod thermal;
