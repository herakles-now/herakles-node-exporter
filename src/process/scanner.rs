//! Process scanning utilities for discovering and reading process entries from /proc.
//!
//! This module provides functions to scan the /proc filesystem for process entries
//! and read process information like names and memory data.

use crate::config::Config;
use crate::process::memory::{update_max_buffer_usage, MAX_IO_BUFFER_BYTES};
use std::fs;
use std::path::{Path, PathBuf};

/// Process entry representing a directory in /proc filesystem.
#[derive(Debug, Clone)]
pub struct ProcEntry {
    pub pid: u32,
    pub proc_path: PathBuf,
}

/// Scans /proc directory for process entries with numeric PIDs.
pub fn collect_proc_entries(root: &str, max: Option<usize>) -> Vec<ProcEntry> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = match p.file_name().and_then(|s| s.to_str()) {
                Some(v) => v,
                None => continue,
            };
            if !name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            if !p.join("smaps").exists() && !p.join("smaps_rollup").exists() {
                continue;
            }
            let pid: u32 = match name.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            out.push(ProcEntry { pid, proc_path: p });
            if let Some(maxp) = max {
                if out.len() >= maxp {
                    break;
                }
            }
        }
    }
    out
}

/// Reads process name from comm file or extracts from cmdline.
pub fn read_process_name(proc_path: &Path) -> Option<String> {
    let comm = proc_path.join("comm");
    if let Ok(s) = fs::read_to_string(&comm) {
        let t = s.trim();
        if !t.is_empty() {
            // Track io_buffer usage for comm file
            update_max_buffer_usage(&MAX_IO_BUFFER_BYTES, s.len() as u64);
            return Some(t.into());
        }
    }

    let cmd = proc_path.join("cmdline");
    if let Ok(content) = fs::read(&cmd) {
        if !content.is_empty() {
            // Track io_buffer usage for cmdline file
            update_max_buffer_usage(&MAX_IO_BUFFER_BYTES, content.len() as u64);
            let parts: Vec<&str> = content
                .split(|&b| b == 0u8)
                .filter_map(|s| std::str::from_utf8(s).ok())
                .collect();
            if !parts.is_empty() {
                if let Some(name) = Path::new(parts[0]).file_name() {
                    return name.to_str().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

/// Determines if a process should be included based on configuration filters.
pub fn should_include_process(name: &str, cfg: &Config) -> bool {
    if let Some(ex) = &cfg.exclude_names {
        if ex.iter().any(|s| name.contains(s)) {
            return false;
        }
    }
    if let Some(inc) = &cfg.include_names {
        if !inc.is_empty() {
            return inc.iter().any(|s| name.contains(s));
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Tests for should_include_process
    // -------------------------------------------------------------------------

    #[test]
    fn test_should_include_process_no_filters() {
        let cfg = Config::default();
        assert!(should_include_process("nginx", &cfg));
        assert!(should_include_process("postgres", &cfg));
        assert!(should_include_process("any_process", &cfg));
    }

    #[test]
    fn test_should_include_process_with_exclude() {
        let mut cfg = Config::default();
        cfg.exclude_names = Some(vec!["test".to_string(), "debug".to_string()]);

        assert!(!should_include_process("test_app", &cfg));
        assert!(!should_include_process("debug_server", &cfg));
        assert!(should_include_process("nginx", &cfg));
        assert!(should_include_process("production_app", &cfg));
    }

    #[test]
    fn test_should_include_process_with_include() {
        let mut cfg = Config::default();
        cfg.include_names = Some(vec!["nginx".to_string(), "postgres".to_string()]);

        assert!(should_include_process("nginx", &cfg));
        assert!(should_include_process("nginx-worker", &cfg));
        assert!(should_include_process("postgres", &cfg));
        assert!(!should_include_process("mysql", &cfg));
        assert!(!should_include_process("redis", &cfg));
    }

    #[test]
    fn test_should_include_process_exclude_takes_priority() {
        let mut cfg = Config::default();
        cfg.include_names = Some(vec!["app".to_string()]);
        cfg.exclude_names = Some(vec!["test".to_string()]);

        // "test_app" matches both include ("app") and exclude ("test")
        // Exclude should take priority
        assert!(!should_include_process("test_app", &cfg));
        assert!(should_include_process("prod_app", &cfg));
    }
}
