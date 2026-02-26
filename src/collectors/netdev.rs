//! Network interface statistics collector.
//!
//! This module provides functionality to read network interface statistics from /proc/net/dev
//! and expose them as Prometheus metrics.

use std::collections::HashMap;
use std::fs;

/// Network interface statistics.
#[derive(Debug, Clone)]
pub struct NetDevStats {
    pub receive_bytes: u64,
    #[allow(dead_code)] // Collected but not yet exposed as metric
    pub receive_packets: u64,
    pub receive_errs: u64,
    pub receive_drop: u64,
    pub transmit_bytes: u64,
    #[allow(dead_code)] // Collected but not yet exposed as metric
    pub transmit_packets: u64,
    pub transmit_errs: u64,
    pub transmit_drop: u64,
}

/// Reads network interface statistics from /proc/net/dev.
///
/// Returns a HashMap mapping interface names to their statistics.
pub fn read_netdev_stats() -> Result<HashMap<String, NetDevStats>, String> {
    let content = fs::read_to_string("/proc/net/dev")
        .map_err(|e| format!("Failed to read /proc/net/dev: {}", e))?;

    let mut stats = HashMap::new();

    for (idx, line) in content.lines().enumerate() {
        // Skip the first two header lines
        if idx < 2 {
            continue;
        }

        // Split by ':' to separate interface name from stats
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() != 2 {
            continue;
        }

        let interface = parts[0].trim().to_string();
        let stats_str = parts[1].trim();

        let values: Vec<&str> = stats_str.split_whitespace().collect();
        if values.len() < 16 {
            continue; // Skip malformed lines
        }

        let net_stat = NetDevStats {
            receive_bytes: values[0].parse().unwrap_or(0),
            receive_packets: values[1].parse().unwrap_or(0),
            receive_errs: values[2].parse().unwrap_or(0),
            receive_drop: values[3].parse().unwrap_or(0),
            transmit_bytes: values[8].parse().unwrap_or(0),
            transmit_packets: values[9].parse().unwrap_or(0),
            transmit_errs: values[10].parse().unwrap_or(0),
            transmit_drop: values[11].parse().unwrap_or(0),
        };

        stats.insert(interface, net_stat);
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_netdev_stats() {
        let result = read_netdev_stats();
        assert!(result.is_ok(), "Failed to read netdev stats: {:?}", result);

        let stats = result.unwrap();
        // Should have at least one interface (lo)
        assert!(!stats.is_empty(), "No network interface statistics found");

        // Check that loopback interface is present
        let has_lo = stats.contains_key("lo");
        assert!(has_lo, "Loopback interface not found");
    }
}
