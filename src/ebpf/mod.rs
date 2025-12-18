//! eBPF manager module for process I/O tracking.
//!
//! This module provides eBPF-based tracking of per-process network and block I/O.
//! When eBPF is not available (old kernel, missing permissions, or feature disabled),
//! it gracefully degrades and returns empty results.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::debug;

#[cfg(feature = "ebpf")]
use tracing::{info, warn};

/// Process network I/O statistics from eBPF.
#[derive(Debug, Clone, Default)]
pub struct ProcessNetStats {
    pub pid: u32,
    pub comm: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub dropped: u64,
}

/// Process block I/O statistics from eBPF.
#[derive(Debug, Clone, Default)]
pub struct ProcessBlkioStats {
    pub pid: u32,
    pub comm: String,
    pub device: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

/// TCP connection statistics from eBPF.
#[derive(Debug, Clone, Default)]
pub struct TcpStats {
    pub established: u64,
    pub syn_sent: u64,
    pub syn_recv: u64,
    pub fin_wait1: u64,
    pub fin_wait2: u64,
    pub time_wait: u64,
    pub close: u64,
    pub close_wait: u64,
    pub last_ack: u64,
    pub listen: u64,
    pub closing: u64,
}

/// Performance statistics for eBPF programs.
#[derive(Debug, Clone, Copy)]
pub struct EbpfPerfStats {
    pub enabled: bool,
    pub programs_loaded: usize,
    pub events_per_sec: f64,
    pub lost_events_total: u64,
    pub map_usage_percent: f64,
    pub cpu_overhead_percent: f64,
}

/// eBPF manager for loading and managing eBPF programs.
pub struct EbpfManager {
    enabled: bool,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))]
    inner: Arc<Mutex<Option<EbpfInner>>>,
}

struct EbpfInner {
    // Placeholder for actual eBPF structures
    // In a real implementation, this would hold libbpf-rs objects
    #[allow(dead_code)]
    loaded: bool,
}

impl EbpfManager {
    /// Creates a new eBPF manager.
    /// 
    /// Returns an error if eBPF cannot be initialized. The caller should
    /// handle this gracefully and continue without eBPF metrics.
    pub fn new() -> Result<Self, anyhow::Error> {
        #[cfg(feature = "ebpf")]
        {
            match Self::try_init_ebpf() {
                Ok(inner) => {
                    info!("eBPF initialized successfully");
                    Ok(Self {
                        enabled: true,
                        inner: Arc::new(Mutex::new(Some(inner))),
                    })
                }
                Err(e) => {
                    warn!("Failed to initialize eBPF (will run without eBPF metrics): {}", e);
                    Ok(Self {
                        enabled: false,
                        inner: Arc::new(Mutex::new(None)),
                    })
                }
            }
        }
        
        #[cfg(not(feature = "ebpf"))]
        {
            debug!("eBPF feature not enabled at compile time");
            Ok(Self {
                enabled: false,
                inner: Arc::new(Mutex::new(None)),
            })
        }
    }

    #[cfg(feature = "ebpf")]
    fn try_init_ebpf() -> Result<EbpfInner, anyhow::Error> {
        // TODO: Actual eBPF initialization would go here
        // This would involve:
        // 1. Loading eBPF object file (embedded via include_bytes!)
        // 2. Attaching to kernel tracepoints/kprobes
        // 3. Setting up BPF maps for communication
        
        // For now, return an error to indicate eBPF is not yet implemented
        Err(anyhow::anyhow!("eBPF implementation pending - requires eBPF C code from ultimate-exporter"))
    }

    /// Returns true if eBPF is enabled and functional.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Reads process network I/O statistics from eBPF maps.
    pub fn read_process_net_stats(&self) -> Result<Vec<ProcessNetStats>, anyhow::Error> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        // TODO: Read from eBPF maps
        // In a real implementation, this would:
        // 1. Iterate over BPF_MAP_TYPE_HASH containing per-PID stats
        // 2. Parse the binary data into ProcessNetStats
        // 3. Resolve process names from /proc if needed
        
        Ok(Vec::new())
    }

    /// Reads process block I/O statistics from eBPF maps.
    pub fn read_process_blkio_stats(&self) -> Result<Vec<ProcessBlkioStats>, anyhow::Error> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        // TODO: Read from eBPF maps
        Ok(Vec::new())
    }

    /// Reads TCP connection statistics from eBPF maps.
    pub fn read_tcp_stats(&self) -> Result<TcpStats, anyhow::Error> {
        if !self.enabled {
            return Ok(TcpStats::default());
        }

        // TODO: Read from eBPF maps
        Ok(TcpStats::default())
    }

    /// Resolves device name from major:minor numbers.
    /// 
    /// This is used to convert kernel device numbers to names like "sda", "nvme0n1", etc.
    #[allow(dead_code)]
    fn resolve_device_name(major: u32, minor: u32) -> String {
        // Try to read from /proc/diskstats or /sys/dev/block
        let path = format!("/sys/dev/block/{}:{}/uevent", major, minor);
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(name) = line.strip_prefix("DEVNAME=") {
                    return name.to_string();
                }
            }
        }
        
        // Fallback to major:minor notation
        format!("{}:{}", major, minor)
    }

    /// Reads process information cache for name resolution.
    #[allow(dead_code)]
    fn read_process_name(pid: u32) -> Option<String> {
        std::fs::read_to_string(format!("/proc/{}/comm", pid))
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Returns performance statistics for eBPF programs.
    /// 
    /// # Note
    /// This is a placeholder implementation that returns zero/default values.
    /// Actual eBPF performance tracking will be implemented once the eBPF programs
    /// are integrated from the ultimate-exporter C code. When implemented, this will:
    /// - Track real-time event rates from eBPF ring buffers
    /// - Monitor lost events due to buffer overruns
    /// - Calculate BPF map memory usage
    /// - Estimate CPU overhead from eBPF program execution
    pub fn get_performance_stats(&self) -> EbpfPerfStats {
        if !self.enabled {
            return EbpfPerfStats {
                enabled: false,
                programs_loaded: 0,
                events_per_sec: 0.0,
                lost_events_total: 0,
                map_usage_percent: 0.0,
                cpu_overhead_percent: 0.0,
            };
        }
        
        // Placeholder implementation - returns zero values until eBPF programs are integrated
        EbpfPerfStats {
            enabled: true,
            programs_loaded: 0, // Will be: network, blkio, tcp when implemented
            events_per_sec: 0.0,
            lost_events_total: 0,
            map_usage_percent: 0.0,
            cpu_overhead_percent: 0.0,
        }
    }
}

/// Helper function to aggregate I/O stats by group/subgroup.
pub fn aggregate_io_by_subgroup(
    net_stats: &[ProcessNetStats],
    blkio_stats: &[ProcessBlkioStats],
) -> (
    HashMap<(String, String), (u64, u64)>, // (group, subgroup) -> (rx_bytes, tx_bytes)
    HashMap<(String, String), (u64, u64)>, // (group, subgroup) -> (read_bytes, write_bytes)
) {
    use crate::process::classify_process_raw;
    
    let mut net_agg = HashMap::new();
    let mut blkio_agg = HashMap::new();
    
    // Aggregate network stats
    for stat in net_stats {
        let (group, subgroup) = classify_process_raw(&stat.comm);
        let key = (group.to_string(), subgroup.to_string());
        let entry = net_agg.entry(key).or_insert((0u64, 0u64));
        entry.0 += stat.rx_bytes;
        entry.1 += stat.tx_bytes;
    }
    
    // Aggregate block I/O stats
    for stat in blkio_stats {
        let (group, subgroup) = classify_process_raw(&stat.comm);
        let key = (group.to_string(), subgroup.to_string());
        let entry = blkio_agg.entry(key).or_insert((0u64, 0u64));
        entry.0 += stat.read_bytes;
        entry.1 += stat.write_bytes;
    }
    
    (net_agg, blkio_agg)
}

/// Calculate top-N processes by I/O.
pub fn calculate_top_io_processes(
    net_stats: &[ProcessNetStats],
    blkio_stats: &[ProcessBlkioStats],
    n: usize,
) -> (
    Vec<ProcessNetStats>,  // Top-N by network I/O
    Vec<ProcessBlkioStats>, // Top-N by block I/O
) {
    use crate::process::classify_process_raw;
    
    // Group by subgroup
    let mut net_by_subgroup: HashMap<(String, String), Vec<ProcessNetStats>> = HashMap::new();
    let mut blkio_by_subgroup: HashMap<(String, String), Vec<ProcessBlkioStats>> = HashMap::new();
    
    for stat in net_stats {
        let (group, subgroup) = classify_process_raw(&stat.comm);
        let key = (group.to_string(), subgroup.to_string());
        net_by_subgroup.entry(key).or_default().push(stat.clone());
    }
    
    for stat in blkio_stats {
        let (group, subgroup) = classify_process_raw(&stat.comm);
        let key = (group.to_string(), subgroup.to_string());
        blkio_by_subgroup.entry(key).or_default().push(stat.clone());
    }
    
    // Get top-N from each subgroup
    let mut top_net = Vec::new();
    for (_, mut stats) in net_by_subgroup {
        stats.sort_by_key(|s| std::cmp::Reverse(s.rx_bytes + s.tx_bytes));
        top_net.extend(stats.into_iter().take(n));
    }
    
    let mut top_blkio = Vec::new();
    for (_, mut stats) in blkio_by_subgroup {
        stats.sort_by_key(|s| std::cmp::Reverse(s.read_bytes + s.write_bytes));
        top_blkio.extend(stats.into_iter().take(n));
    }
    
    (top_net, top_blkio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_manager_creation() {
        // Should succeed even without eBPF available
        let manager = EbpfManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_disabled_ebpf_returns_empty() {
        let manager = EbpfManager::new().unwrap();
        let net_stats = manager.read_process_net_stats().unwrap();
        let blkio_stats = manager.read_process_blkio_stats().unwrap();
        let tcp_stats = manager.read_tcp_stats().unwrap();
        
        assert!(net_stats.is_empty());
        assert!(blkio_stats.is_empty());
        assert_eq!(tcp_stats.established, 0);
    }

    #[test]
    fn test_device_name_resolution() {
        // Test fallback behavior
        let name = EbpfManager::resolve_device_name(8, 0);
        assert!(!name.is_empty());
    }
}
