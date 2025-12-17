//! System-wide metrics collection from /proc filesystem.
//!
//! This module provides functions to read system-wide metrics such as
//! load average, total RAM, and total SWAP from the /proc filesystem.

use std::collections::HashMap;
use std::fs;
use std::sync::RwLock;

/// System load averages for 1, 5, and 15 minute intervals.
#[derive(Debug, Clone, Copy)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

/// Extended memory information including available memory.
#[derive(Debug, Clone, Copy)]
pub struct ExtendedMemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

/// CPU statistics for calculating usage ratios.
#[derive(Debug, Clone, Copy)]
pub struct CpuStat {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuStat {
    /// Calculate total CPU time (all fields).
    pub fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal
    }

    /// Calculate non-active time (idle + iowait).
    /// This includes both true idle time and time spent waiting for I/O operations.
    pub fn idle_total(&self) -> u64 {
        self.idle + self.iowait
    }
}

/// Reads load average from /proc/loadavg.
///
/// Returns the 1, 5, and 15 minute load averages.
/// Format: "0.00 0.01 0.05 1/234 5678"
pub fn read_load_average() -> Result<LoadAverage, String> {
    let content = fs::read_to_string("/proc/loadavg")
        .map_err(|e| format!("Failed to read /proc/loadavg: {}", e))?;

    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(format!(
            "Invalid /proc/loadavg format: expected at least 3 fields, got {}",
            parts.len()
        ));
    }

    let one_min = parts[0]
        .parse::<f64>()
        .map_err(|e| format!("Failed to parse 1min load average: {}", e))?;
    let five_min = parts[1]
        .parse::<f64>()
        .map_err(|e| format!("Failed to parse 5min load average: {}", e))?;
    let fifteen_min = parts[2]
        .parse::<f64>()
        .map_err(|e| format!("Failed to parse 15min load average: {}", e))?;

    Ok(LoadAverage {
        one_min,
        five_min,
        fifteen_min,
    })
}

/// Reads extended memory information from /proc/meminfo including MemAvailable.
///
/// Returns total and available memory in bytes.
pub fn read_extended_memory_info() -> Result<ExtendedMemoryInfo, String> {
    let content = fs::read_to_string("/proc/meminfo")
        .map_err(|e| format!("Failed to read /proc/meminfo: {}", e))?;

    let mut total_bytes: Option<u64> = None;
    let mut available_bytes: Option<u64> = None;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(kb) = parts[1].parse::<u64>() {
                    total_bytes = Some(kb * 1024);
                }
            }
        } else if line.starts_with("MemAvailable:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(kb) = parts[1].parse::<u64>() {
                    available_bytes = Some(kb * 1024);
                }
            }
        }

        if total_bytes.is_some() && available_bytes.is_some() {
            break;
        }
    }

    match (total_bytes, available_bytes) {
        (Some(total), Some(available)) => Ok(ExtendedMemoryInfo {
            total_bytes: total,
            available_bytes: available,
        }),
        _ => Err("Failed to parse MemTotal or MemAvailable from /proc/meminfo".to_string()),
    }
}

/// Reads CPU statistics from /proc/stat.
///
/// Returns a HashMap with CPU name as key and CpuStat as value.
/// "cpu" represents total across all cores, "cpu0", "cpu1", etc. are individual cores.
pub fn read_cpu_stats() -> Result<HashMap<String, CpuStat>, String> {
    let content = fs::read_to_string("/proc/stat")
        .map_err(|e| format!("Failed to read /proc/stat: {}", e))?;

    let mut stats = HashMap::new();

    for line in content.lines() {
        if line.starts_with("cpu") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                continue;
            }

            let cpu_name = parts[0].to_string();
            
            // Parse CPU time fields
            let user = parts[1].parse::<u64>().unwrap_or(0);
            let nice = parts[2].parse::<u64>().unwrap_or(0);
            let system = parts[3].parse::<u64>().unwrap_or(0);
            let idle = parts[4].parse::<u64>().unwrap_or(0);
            let iowait = parts[5].parse::<u64>().unwrap_or(0);
            let irq = parts[6].parse::<u64>().unwrap_or(0);
            let softirq = parts[7].parse::<u64>().unwrap_or(0);
            let steal = if parts.len() > 8 { parts[8].parse::<u64>().unwrap_or(0) } else { 0 };

            stats.insert(cpu_name, CpuStat {
                user,
                nice,
                system,
                idle,
                iowait,
                irq,
                softirq,
                steal,
            });
        }
    }

    if stats.is_empty() {
        return Err("No CPU statistics found in /proc/stat".to_string());
    }

    Ok(stats)
}

/// CPU statistics cache for calculating deltas.
pub struct CpuStatsCache {
    previous: RwLock<Option<HashMap<String, CpuStat>>>,
}

impl CpuStatsCache {
    pub fn new() -> Self {
        Self {
            previous: RwLock::new(None),
        }
    }

    /// Calculate CPU usage ratios by comparing current and previous stats.
    /// Returns a HashMap with CPU name as key and usage ratio (0.0 to 1.0) as value.
    pub fn calculate_usage_ratios(&self) -> Result<HashMap<String, f64>, String> {
        let current_stats = read_cpu_stats()?;
        
        let mut ratios = HashMap::new();
        
        // Try to get previous stats
        let prev_guard = self.previous.read().map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        if let Some(prev_stats) = prev_guard.as_ref() {
            // Calculate deltas for each CPU
            for (cpu_name, current) in &current_stats {
                if let Some(previous) = prev_stats.get(cpu_name) {
                    let delta_total = current.total().saturating_sub(previous.total());
                    let delta_non_active = current.idle_total().saturating_sub(previous.idle_total());
                    
                    let ratio = if delta_total > 0 {
                        (delta_total - delta_non_active) as f64 / delta_total as f64
                    } else {
                        0.0
                    };
                    
                    ratios.insert(cpu_name.clone(), ratio);
                }
            }
        }
        
        drop(prev_guard);
        
        // Update cache with current stats
        let mut cache_guard = self.previous.write().map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        *cache_guard = Some(current_stats);
        
        Ok(ratios)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_load_average() {
        // Test with valid input
        let result = parse_load_average_line("0.52 0.58 0.59 2/1190 12345");
        assert!(result.is_ok());
        let load = result.unwrap();
        assert!((load.one_min - 0.52).abs() < 0.001);
        assert!((load.five_min - 0.58).abs() < 0.001);
        assert!((load.fifteen_min - 0.59).abs() < 0.001);
    }

    #[test]
    fn test_parse_load_average_invalid() {
        // Test with insufficient fields
        let result = parse_load_average_line("0.52 0.58");
        assert!(result.is_err());

        // Test with non-numeric values
        let result = parse_load_average_line("abc def ghi 1/2 3");
        assert!(result.is_err());
    }

    // Helper functions for testing
    fn parse_load_average_line(line: &str) -> Result<LoadAverage, String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(format!("Invalid format: expected at least 3 fields"));
        }

        let one_min = parts[0]
            .parse::<f64>()
            .map_err(|e| format!("Failed to parse 1min: {}", e))?;
        let five_min = parts[1]
            .parse::<f64>()
            .map_err(|e| format!("Failed to parse 5min: {}", e))?;
        let fifteen_min = parts[2]
            .parse::<f64>()
            .map_err(|e| format!("Failed to parse 15min: {}", e))?;

        Ok(LoadAverage {
            one_min,
            five_min,
            fifteen_min,
        })
    }
}
