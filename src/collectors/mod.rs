//! Collectors module for system metrics.
//!
//! This module contains various collectors for system-level metrics such as
//! disk I/O, filesystem usage, and network interface statistics.

pub mod diskstats;
pub mod filesystem;
pub mod netdev;
