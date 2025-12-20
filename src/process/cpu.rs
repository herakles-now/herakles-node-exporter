//! CPU statistics parsing for process metrics.
//!
//! This module provides functions to parse CPU time information from
//! `/proc/<pid>/stat` and manage CPU usage caching for delta calculations.

use ahash::AHashMap as HashMap;
use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;
use std::sync::RwLock as StdRwLock;
use std::time::Instant;
use tracing::debug;

/// Get system clock ticks per second (usually 100, but can vary).
fn get_clk_tck() -> f64 {
    #[cfg(unix)]
    {
        // SAFETY: sysconf is safe to call with _SC_CLK_TCK
        // Returns -1 on error, 0 if undefined - both are handled by the > 0 check
        unsafe {
            let tck = libc::sysconf(libc::_SC_CLK_TCK);
            if tck > 0 {
                return tck as f64;
            }
        }
    }
    // Fallback to common default for error cases or non-Unix platforms
    100.0
}

/// System clock ticks per second (for CPU time calculation).
pub static CLK_TCK: Lazy<f64> = Lazy::new(get_clk_tck);

/// Cached CPU statistics for a single process (monotonic CPU time + last computed percent).
#[derive(Clone, Copy)]
pub struct CpuStat {
    pub cpu_percent: f64,
    pub cpu_time_seconds: f64,
}

/// Cache entry with timestamp for delta-based CPU calculation.
pub struct CpuEntry {
    pub stat: CpuStat,
    pub last_updated: Instant,
}

/// Parse total CPU time (user+system) in seconds from /proc/<pid>/stat.
pub fn parse_cpu_time_seconds(proc_path: &Path) -> Result<f64, std::io::Error> {
    let stat_path = proc_path.join("stat");
    let content = fs::read_to_string(stat_path)?;

    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() <= 14 {
        return Err(std::io::Error::other("Invalid stat format"));
    }

    let utime: f64 = parts[13].parse().unwrap_or(0.0);
    let stime: f64 = parts[14].parse().unwrap_or(0.0);

    // Use system-detected clock ticks per second
    Ok((utime + stime) / *CLK_TCK)
}

/// Parse process start time from /proc/<pid>/stat (field 22 - starttime in jiffies).
/// Returns start time in seconds since system boot.
pub fn parse_start_time_seconds(proc_path: &Path) -> Result<f64, std::io::Error> {
    let stat_path = proc_path.join("stat");
    let content = fs::read_to_string(stat_path)?;

    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() <= 21 {
        return Err(std::io::Error::other("Invalid stat format"));
    }

    // Field 22 is at index 21 (0-based)
    let starttime_jiffies: u64 = parts[21]
        .parse()
        .map_err(|_| std::io::Error::other("Failed to parse starttime field"))?;

    // Get system uptime
    let system_uptime = crate::system::read_uptime().unwrap_or(0.0);

    // Calculate process start time: system_uptime - (starttime_jiffies / HZ)
    let start_time_seconds = system_uptime - (starttime_jiffies as f64 / *CLK_TCK);

    Ok(start_time_seconds)
}

/// Returns CPU stats for a PID using delta between samples.
pub fn get_cpu_stat_for_pid(
    pid: u32,
    proc_path: &Path,
    cache: &StdRwLock<HashMap<u32, CpuEntry>>,
) -> CpuStat {
    let now = Instant::now();
    let cpu_time_seconds = match parse_cpu_time_seconds(proc_path) {
        Ok(v) => v,
        Err(e) => {
            debug!("Failed to read CPU time for pid {}: {}", pid, e);
            0.0
        }
    };

    let mut cpu_percent = 0.0;

    // Use delta between last and current CPU time to compute percent
    {
        let cache_read = cache.read().expect("cpu_cache read lock poisoned");
        if let Some(entry) = cache_read.get(&pid) {
            let dt = now.duration_since(entry.last_updated).as_secs_f64();
            if dt > 0.0 {
                let delta_cpu = cpu_time_seconds - entry.stat.cpu_time_seconds;
                if delta_cpu > 0.0 {
                    cpu_percent = (delta_cpu / dt) * 100.0;
                }
            }
        }
    }

    let stat = CpuStat {
        cpu_percent,
        cpu_time_seconds,
    };

    // Store updated value in cache
    {
        let mut cache_write = cache.write().expect("cpu_cache write lock poisoned");
        cache_write.insert(
            pid,
            CpuEntry {
                stat,
                last_updated: now,
            },
        );
    }

    stat
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // -------------------------------------------------------------------------
    // Tests for parse_cpu_time_seconds
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_cpu_time_seconds() {
        // Create a temporary directory to simulate /proc/<pid>/stat
        let dir = tempdir().expect("Failed to create temp dir");
        let stat_path = dir.path().join("stat");

        // Typical /proc/<pid>/stat format:
        // pid (comm) state ppid pgrp session tty_nr tpgid flags minflt cminflt majflt cmajflt utime stime ...
        // Fields 14 and 15 (0-indexed: 13 and 14) are utime and stime in clock ticks
        // With CLK_TCK typically 100, ticks / 100 = seconds

        // Example: utime=1000, stime=500 -> total = 1500 ticks
        // If CLK_TCK is 100, then CPU time = 15.0 seconds
        let stat_content = "1234 (test_process) S 1 1234 1234 0 -1 4194304 100 0 0 0 1000 500 0 0 20 0 1 0 12345 12345678 1234 18446744073709551615 4194304 4238788 140736466511168 0 0 0 0 0 0 0 0 0 17 1 0 0 0 0 0";
        std::fs::write(&stat_path, stat_content).expect("Failed to write stat file");

        let result = parse_cpu_time_seconds(dir.path());
        assert!(result.is_ok());

        // Calculate expected value: (1000 + 500) / CLK_TCK
        let expected = 1500.0 / *CLK_TCK;
        let actual = result.unwrap();
        assert!(
            (actual - expected).abs() < 0.001,
            "Expected ~{:.3}, got {:.3}",
            expected,
            actual
        );
    }

    #[test]
    fn test_parse_cpu_time_seconds_invalid_stat() {
        let dir = tempdir().expect("Failed to create temp dir");
        let stat_path = dir.path().join("stat");

        // Invalid stat file with not enough fields
        std::fs::write(&stat_path, "1234 (test) S 1 2 3").expect("Failed to write stat file");

        let result = parse_cpu_time_seconds(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cpu_time_seconds_missing_file() {
        let dir = tempdir().expect("Failed to create temp dir");

        // No stat file exists
        let result = parse_cpu_time_seconds(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cpu_time_seconds_zero_values() {
        let dir = tempdir().expect("Failed to create temp dir");
        let stat_path = dir.path().join("stat");

        // utime=0, stime=0
        let stat_content = "1234 (idle_process) S 1 1234 1234 0 -1 4194304 0 0 0 0 0 0 0 0 20 0 1 0 12345 12345678 1234 18446744073709551615 4194304 4238788 140736466511168 0 0 0 0 0 0 0 0 0 17 1 0 0 0 0 0";
        std::fs::write(&stat_path, stat_content).expect("Failed to write stat file");

        let result = parse_cpu_time_seconds(dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.0);
    }
}
