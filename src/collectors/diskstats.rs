//! Disk I/O statistics collector.
//!
//! This module provides functionality to read disk I/O statistics from /proc/diskstats
//! and expose them as Prometheus metrics.

use std::collections::HashMap;
use std::fs;

/// Disk statistics for a single device.
#[derive(Debug, Clone)]
pub struct DiskStats {
    #[allow(dead_code)] // Collected for future detailed I/O analysis
    pub reads_completed: u64,
    #[allow(dead_code)] // Collected for future detailed I/O analysis
    pub reads_merged: u64,
    pub sectors_read: u64,
    #[allow(dead_code)] // Collected for future detailed I/O analysis
    pub time_reading_ms: u64,
    #[allow(dead_code)] // Collected for future detailed I/O analysis
    pub writes_completed: u64,
    #[allow(dead_code)] // Collected for future detailed I/O analysis
    pub writes_merged: u64,
    pub sectors_written: u64,
    #[allow(dead_code)] // Collected for future detailed I/O analysis
    pub time_writing_ms: u64,
    pub ios_in_progress: u64,
    pub time_io_ms: u64,
    #[allow(dead_code)] // Collected for future detailed I/O analysis
    pub weighted_time_io_ms: u64,
}

/// Reads disk statistics from /proc/diskstats.
///
/// Returns a HashMap mapping device names to their statistics.
/// Format: major minor name read_ios read_merges read_sectors read_ticks write_ios write_merges write_sectors write_ticks ios_in_progress time_in_queue weighted_time_in_queue
pub fn read_diskstats() -> Result<HashMap<String, DiskStats>, String> {
    let content = fs::read_to_string("/proc/diskstats")
        .map_err(|e| format!("Failed to read /proc/diskstats: {}", e))?;

    let mut stats = HashMap::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 {
            continue; // Skip malformed lines
        }

        let device = parts[2].to_string();

        // Skip loop devices and partitions we don't want to track
        // You can customize this filter as needed
        if device.starts_with("loop") || device.starts_with("ram") {
            continue;
        }

        let disk_stat = DiskStats {
            reads_completed: parts[3].parse().unwrap_or(0),
            reads_merged: parts[4].parse().unwrap_or(0),
            sectors_read: parts[5].parse().unwrap_or(0),
            time_reading_ms: parts[6].parse().unwrap_or(0),
            writes_completed: parts[7].parse().unwrap_or(0),
            writes_merged: parts[8].parse().unwrap_or(0),
            sectors_written: parts[9].parse().unwrap_or(0),
            time_writing_ms: parts[10].parse().unwrap_or(0),
            ios_in_progress: parts[11].parse().unwrap_or(0),
            time_io_ms: parts[12].parse().unwrap_or(0),
            weighted_time_io_ms: parts[13].parse().unwrap_or(0),
        };

        stats.insert(device, disk_stat);
    }

    Ok(stats)
}

/// Reads PSI (Pressure Stall Information) I/O metrics from /proc/pressure/io.
///
/// Returns the "some" total microseconds value converted to seconds.
/// PSI tracks the time processes spend waiting for I/O.
#[allow(dead_code)] // Used via system::read_psi_some_total instead
pub fn read_psi_io() -> Result<f64, String> {
    let content = fs::read_to_string("/proc/pressure/io")
        .map_err(|e| format!("Failed to read /proc/pressure/io: {}", e))?;

    for line in content.lines() {
        if let Some(some_line) = line.strip_prefix("some ") {
            // Parse: "avg10=0.00 avg60=0.00 avg300=0.00 total=12345"
            for part in some_line.split_whitespace() {
                if let Some(total_str) = part.strip_prefix("total=") {
                    let microseconds: u64 = total_str
                        .parse()
                        .map_err(|e| format!("Failed to parse PSI total: {}", e))?;
                    return Ok(microseconds as f64 / 1_000_000.0);
                }
            }
        }
    }

    Err("PSI 'some' line not found in /proc/pressure/io".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_diskstats() {
        let result = read_diskstats();
        assert!(result.is_ok(), "Failed to read diskstats: {:?}", result);

        let stats = result.unwrap();
        // Should have at least one disk
        assert!(!stats.is_empty(), "No disk statistics found");
    }

    #[test]
    fn test_read_psi_io() {
        // PSI might not be available on all systems
        let result = read_psi_io();
        if result.is_ok() {
            let psi_val = result.unwrap();
            assert!(psi_val >= 0.0, "PSI value should be non-negative");
        }
    }
}
