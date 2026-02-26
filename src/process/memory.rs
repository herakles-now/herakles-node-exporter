//! Memory parsing utilities for reading process memory metrics from /proc.
//!
//! This module provides functions to parse memory information from
//! `/proc/<pid>/smaps` and `/proc/<pid>/smaps_rollup` files.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Static atomics for tracking maximum buffer usage across parse operations.
/// These track the actual bytes read through each buffer type.
pub static MAX_IO_BUFFER_BYTES: AtomicU64 = AtomicU64::new(0);
pub static MAX_SMAPS_BUFFER_BYTES: AtomicU64 = AtomicU64::new(0);
pub static MAX_SMAPS_ROLLUP_BUFFER_BYTES: AtomicU64 = AtomicU64::new(0);

/// Buffer configuration for parsing operations.
#[derive(Clone, Copy)]
pub struct BufferConfig {
    pub io_kb: usize,
    pub smaps_kb: usize,
    pub smaps_rollup_kb: usize,
}

/// Helper to update maximum buffer usage atomically.
pub fn update_max_buffer_usage(current_max: &AtomicU64, new_value: u64) {
    let mut current = current_max.load(Ordering::Relaxed);
    while new_value > current {
        match current_max.compare_exchange_weak(
            current,
            new_value,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(c) => current = c,
        }
    }
}

/// Fast parser for /proc/<pid>/smaps_rollup (Linux >= 4.14).
/// Much faster than reading the full smaps file.
pub fn parse_smaps_rollup(path: &Path, buf_kb: usize) -> Result<(u64, u64, u64), std::io::Error> {
    let file = fs::File::open(path)?;
    let reader = BufReader::with_capacity(buf_kb * 1024, file);

    let mut rss_kb = 0;
    let mut pss_kb = 0;
    let mut private_clean_kb = 0;
    let mut private_dirty_kb = 0;
    let mut bytes_read: u64 = 0;

    for line in reader.lines() {
        let l = line?;
        bytes_read += l.len() as u64 + 1; // +1 for newline
        if let Some(v) = l.strip_prefix("Rss:") {
            rss_kb += parse_kb_value(v).unwrap_or(0);
        } else if let Some(v) = l.strip_prefix("Pss:") {
            pss_kb += parse_kb_value(v).unwrap_or(0);
        } else if let Some(v) = l.strip_prefix("Private_Clean:") {
            private_clean_kb += parse_kb_value(v).unwrap_or(0);
        } else if let Some(v) = l.strip_prefix("Private_Dirty:") {
            private_dirty_kb += parse_kb_value(v).unwrap_or(0);
        }
    }

    // Update maximum buffer usage for smaps_rollup
    update_max_buffer_usage(&MAX_SMAPS_ROLLUP_BUFFER_BYTES, bytes_read);

    Ok((
        rss_kb * 1024,
        pss_kb * 1024,
        (private_clean_kb + private_dirty_kb) * 1024,
    ))
}

/// Parses memory metrics from /proc/pid/smaps file.
pub fn parse_smaps(path: &Path, buf_kb: usize) -> Result<(u64, u64, u64), std::io::Error> {
    let file = fs::File::open(path)?;
    let reader = BufReader::with_capacity(buf_kb * 1024, file);

    let mut rss = 0;
    let mut pss = 0;
    let mut pc = 0;
    let mut pd = 0;
    let mut bytes_read: u64 = 0;

    for line in reader.lines() {
        let l = line?;
        bytes_read += l.len() as u64 + 1; // +1 for newline
        if let Some(kb) = l.strip_prefix("Rss:") {
            rss += parse_kb_value(kb).unwrap_or(0);
        } else if let Some(kb) = l.strip_prefix("Pss:") {
            pss += parse_kb_value(kb).unwrap_or(0);
        } else if let Some(kb) = l.strip_prefix("Private_Clean:") {
            pc += parse_kb_value(kb).unwrap_or(0);
        } else if let Some(kb) = l.strip_prefix("Private_Dirty:") {
            pd += parse_kb_value(kb).unwrap_or(0);
        }
    }

    // Update maximum buffer usage for smaps
    update_max_buffer_usage(&MAX_SMAPS_BUFFER_BYTES, bytes_read);

    Ok((rss * 1024, pss * 1024, (pc + pd) * 1024))
}

/// Parses kilobyte values from smaps file lines.
pub fn parse_kb_value(v: &str) -> Option<u64> {
    v.split_whitespace().next()?.parse().ok()
}

/// Wrapper that selects the fastest available memory parser.
/// Uses smaps_rollup when available, otherwise falls back to full smaps.
pub fn parse_memory_for_process(
    proc_path: &Path,
    buffers: &BufferConfig,
) -> Result<(u64, u64, u64), std::io::Error> {
    let rollup = proc_path.join("smaps_rollup");
    if rollup.exists() {
        return parse_smaps_rollup(&rollup, buffers.smaps_rollup_kb);
    }

    let smaps = proc_path.join("smaps");
    parse_smaps(&smaps, buffers.smaps_kb)
}

/// Reads VmSwap from /proc/[pid]/status.
/// Returns swap usage in bytes.
pub fn read_vmswap(proc_path: &Path) -> Result<u64, std::io::Error> {
    let status_path = proc_path.join("status");
    let content = fs::read_to_string(status_path)?;

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("VmSwap:") {
            if let Some(kb) = parse_kb_value(v) {
                return Ok(kb * 1024);
            }
        }
    }

    // If VmSwap is not present in status (kernel < 2.6.34 or no swap), return 0
    Ok(0)
}

/// Reads Block I/O statistics from /proc/[pid]/io.
/// Returns (read_bytes, write_bytes) from storage devices.
/// Note: Requires appropriate permissions (usually root or CAP_SYS_PTRACE).
pub fn read_block_io(proc_path: &Path) -> Result<(u64, u64), std::io::Error> {
    let io_path = proc_path.join("io");
    let content = fs::read_to_string(io_path)?;

    let mut read_bytes = 0u64;
    let mut write_bytes = 0u64;
    let mut found_read = false;
    let mut found_write = false;

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("read_bytes:") {
            read_bytes = v.trim().parse().unwrap_or(0);
            found_read = true;
        } else if let Some(v) = line.strip_prefix("write_bytes:") {
            write_bytes = v.trim().parse().unwrap_or(0);
            found_write = true;
        }

        // Early exit if we've found both values
        if found_read && found_write {
            break;
        }
    }

    Ok((read_bytes, write_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Tests for parse_kb_value
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_kb_value() {
        // Standard smaps format with trailing "kB"
        assert_eq!(parse_kb_value("       1234 kB"), Some(1234));
        assert_eq!(parse_kb_value("1234 kB"), Some(1234));
        assert_eq!(parse_kb_value("0 kB"), Some(0));
        assert_eq!(parse_kb_value("999999999 kB"), Some(999999999));

        // Just numeric values (whitespace trimmed)
        assert_eq!(parse_kb_value("  42  "), Some(42));
        assert_eq!(parse_kb_value("100"), Some(100));

        // Large values
        assert_eq!(parse_kb_value("18446744073709551615"), Some(u64::MAX));
    }

    #[test]
    fn test_parse_kb_value_invalid() {
        // Empty input
        assert_eq!(parse_kb_value(""), None);

        // Only whitespace
        assert_eq!(parse_kb_value("   "), None);

        // Non-numeric input
        assert_eq!(parse_kb_value("abc"), None);
        assert_eq!(parse_kb_value("kB"), None);

        // Negative values (can't parse as u64)
        assert_eq!(parse_kb_value("-1 kB"), None);

        // Floating point values
        assert_eq!(parse_kb_value("1.5 kB"), None);

        // Mixed invalid formats
        assert_eq!(parse_kb_value("12abc34 kB"), None);
    }
}
