//! eBPF manager module for process I/O tracking.
//!
//! This module provides eBPF-based tracking of per-process network and block I/O.
//! When eBPF is not available (old kernel, missing permissions, or feature disabled),
//! it gracefully degrades and returns empty results.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(feature = "ebpf")]
use std::time::Instant;

#[cfg(feature = "ebpf")]
use tracing::{info, warn};

#[cfg(feature = "ebpf")]
use libbpf_rs::{MapCore, MapFlags, Object, ObjectBuilder};

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
    #[allow(dead_code)] // Part of public API, used when eBPF feature is enabled
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
    #[cfg(feature = "ebpf")]
    object: Object,
    #[cfg(feature = "ebpf")]
    #[allow(dead_code)]  // Used for performance metrics calculation
    start_time: Instant,
    #[cfg(feature = "ebpf")]
    last_event_count: u64,
    #[cfg(feature = "ebpf")]
    last_check: Instant,
    #[cfg(feature = "ebpf")]
    #[allow(dead_code)]  // CRITICAL: Must be kept alive to prevent eBPF detachment
    links: Vec<libbpf_rs::Link>,
    #[cfg(not(feature = "ebpf"))]
    #[allow(dead_code)]
    loaded: bool,
}

// SAFETY: EbpfInner is only accessed through a Mutex, ensuring exclusive access.
// The Object and Link types from libbpf-rs are safe to send between threads when
// properly synchronized, which the Mutex provides.
#[cfg(feature = "ebpf")]
unsafe impl Send for EbpfInner {}

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
        // Load eBPF object file (embedded at compile time)
        let obj_path = env!("EBPF_OBJECT_PATH");
        
        let mut builder = ObjectBuilder::default();
        builder.debug(cfg!(debug_assertions));
        
        let open_obj = builder.open_file(obj_path)?;
        let obj = open_obj.load()?;
        
        // Attach all programs and store links to keep them alive
        let mut links = Vec::new();
        for prog in obj.progs_mut() {
            match prog.attach() {
                Ok(link) => {
                    info!("✅ Attached eBPF program: {}", prog.name().to_string_lossy());
                    links.push(link);
                }
                Err(e) => {
                    warn!("⚠️  Failed to attach eBPF program {}: {}", prog.name().to_string_lossy(), e);
                    // Continue with other programs
                }
            }
        }
        
        if links.is_empty() {
            return Err(anyhow::anyhow!("No eBPF programs could be attached"));
        }
        
        info!("✅ eBPF initialized: {} programs attached", links.len());
        info!("   - Network RX/TX tracking enabled");
        info!("   - Block I/O tracking enabled");
        info!("   - TCP state tracking enabled");
        
        let now = Instant::now();
        Ok(EbpfInner {
            object: obj,
            start_time: now,
            last_event_count: 0,
            last_check: now,
            links,
        })
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

        #[cfg(feature = "ebpf")]
        {
            let inner = self.inner.lock().unwrap();
            if let Some(ref inner) = *inner {
                let map = Self::find_map(&inner.object, "net_stats_map")
                    .ok_or_else(|| anyhow::anyhow!("net_stats_map not found"))?;
                let mut stats = Vec::new();
                
                for key in map.keys() {
                    if let Some(value) = map.lookup(&key, MapFlags::ANY)? {
                        // Convert key Vec<u8> to u32
                        if key.len() < 4 {
                            continue;
                        }
                        let pid = u32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
                        
                        // Parse the net_stats struct: 5 u64 fields (40 bytes)
                        if value.len() >= 40 {
                            let mut data = [0u64; 5];
                            for (i, chunk) in value.chunks_exact(8).take(5).enumerate() {
                                data[i] = u64::from_ne_bytes(chunk.try_into().unwrap());
                            }
                            
                            let comm = Self::read_process_name(pid)
                                .unwrap_or_else(|| format!("pid_{}", pid));
                            
                            stats.push(ProcessNetStats {
                                pid,
                                comm,
                                rx_bytes: data[0],
                                tx_bytes: data[1],
                                rx_packets: data[2],
                                tx_packets: data[3],
                                dropped: data[4],
                            });
                        }
                    }
                }
                
                return Ok(stats);
            }
        }
        
        Ok(Vec::new())
    }

    /// Reads process block I/O statistics from eBPF maps.
    pub fn read_process_blkio_stats(&self) -> Result<Vec<ProcessBlkioStats>, anyhow::Error> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        #[cfg(feature = "ebpf")]
        {
            let inner = self.inner.lock().unwrap();
            if let Some(ref inner) = *inner {
                let map = Self::find_map(&inner.object, "blkio_stats_map")
                    .ok_or_else(|| anyhow::anyhow!("blkio_stats_map not found"))?;
                let mut stats = Vec::new();
                
                for key in map.keys() {
                    if let Some(value) = map.lookup(&key, MapFlags::ANY)? {
                        // Parse io_key struct: pid (u32) + dev (u32) = 8 bytes
                        if key.len() >= 8 {
                            let mut key_data = [0u32; 2];
                            for (i, chunk) in key.chunks_exact(4).take(2).enumerate() {
                                key_data[i] = u32::from_ne_bytes(chunk.try_into().unwrap());
                            }
                            let pid = key_data[0];
                            let dev = key_data[1];
                            
                            // Parse blkio_stats struct: 4 u64 fields (32 bytes)
                            if value.len() >= 32 {
                                let mut data = [0u64; 4];
                                for (i, chunk) in value.chunks_exact(8).take(4).enumerate() {
                                    data[i] = u64::from_ne_bytes(chunk.try_into().unwrap());
                                }
                                
                                let comm = Self::read_process_name(pid)
                                    .unwrap_or_else(|| format!("pid_{}", pid));
                                let device = Self::resolve_device_name(dev >> 20, dev & 0xfffff);
                                
                                stats.push(ProcessBlkioStats {
                                    pid,
                                    comm,
                                    device,
                                    read_bytes: data[0],
                                    write_bytes: data[1],
                                    read_ops: data[2],
                                    write_ops: data[3],
                                });
                            }
                        }
                    }
                }
                
                return Ok(stats);
            }
        }

        Ok(Vec::new())
    }

    /// Reads TCP connection statistics from eBPF maps.
    pub fn read_tcp_stats(&self) -> Result<TcpStats, anyhow::Error> {
        if !self.enabled {
            return Ok(TcpStats::default());
        }

        #[cfg(feature = "ebpf")]
        {
            let inner = self.inner.lock().unwrap();
            if let Some(ref inner) = *inner {
                let map = Self::find_map(&inner.object, "tcp_state_map")
                    .ok_or_else(|| anyhow::anyhow!("tcp_state_map not found"))?;
                
                // TCP states from include/net/tcp_states.h
                const TCP_ESTABLISHED: u32 = 1;
                const TCP_SYN_SENT: u32 = 2;
                const TCP_SYN_RECV: u32 = 3;
                const TCP_FIN_WAIT1: u32 = 4;
                const TCP_FIN_WAIT2: u32 = 5;
                const TCP_TIME_WAIT: u32 = 6;
                const TCP_CLOSE: u32 = 7;
                const TCP_CLOSE_WAIT: u32 = 8;
                const TCP_LAST_ACK: u32 = 9;
                const TCP_LISTEN: u32 = 10;
                const TCP_CLOSING: u32 = 11;
                
                let mut tcp_stats = TcpStats::default();
                
                for state in [TCP_ESTABLISHED, TCP_SYN_SENT, TCP_SYN_RECV, TCP_FIN_WAIT1, 
                              TCP_FIN_WAIT2, TCP_TIME_WAIT, TCP_CLOSE, TCP_CLOSE_WAIT,
                              TCP_LAST_ACK, TCP_LISTEN, TCP_CLOSING] {
                    let key = state.to_ne_bytes();
                    if let Some(value) = map.lookup(&key, MapFlags::ANY).ok().flatten() {
                        if value.len() >= 8 {
                            let count = u64::from_ne_bytes(value[0..8].try_into().unwrap());
                            match state {
                                TCP_ESTABLISHED => tcp_stats.established = count,
                                TCP_SYN_SENT => tcp_stats.syn_sent = count,
                                TCP_SYN_RECV => tcp_stats.syn_recv = count,
                                TCP_FIN_WAIT1 => tcp_stats.fin_wait1 = count,
                                TCP_FIN_WAIT2 => tcp_stats.fin_wait2 = count,
                                TCP_TIME_WAIT => tcp_stats.time_wait = count,
                                TCP_CLOSE => tcp_stats.close = count,
                                TCP_CLOSE_WAIT => tcp_stats.close_wait = count,
                                TCP_LAST_ACK => tcp_stats.last_ack = count,
                                TCP_LISTEN => tcp_stats.listen = count,
                                TCP_CLOSING => tcp_stats.closing = count,
                                _ => {}
                            }
                        }
                    }
                }
                
                return Ok(tcp_stats);
            }
        }

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

    /// Helper function to find a map by name in an Object.
    #[cfg(feature = "ebpf")]
    fn find_map<'a>(object: &'a Object, name: &str) -> Option<libbpf_rs::Map<'a>> {
        object.maps().find(|m| {
            m.name().to_str() == Some(name)
        })
    }

    /// Reads process information cache for name resolution.
    #[allow(dead_code)]
    fn read_process_name(pid: u32) -> Option<String> {
        std::fs::read_to_string(format!("/proc/{}/comm", pid))
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Returns performance statistics for eBPF programs.
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
        
        #[cfg(feature = "ebpf")]
        {
            let mut inner_guard = self.inner.lock().unwrap();
            if let Some(ref mut inner) = *inner_guard {
                // Calculate events per second from event_counters map
                let events_per_sec = Self::calculate_event_rate(&inner.object, &mut inner.last_check, &mut inner.last_event_count);
                let map_usage = Self::calculate_map_usage(&inner.object);
                
                return EbpfPerfStats {
                    enabled: true,
                    programs_loaded: 4, // netif_receive_skb, dev_queue_xmit, block_rq_issue, inet_sock_set_state
                    events_per_sec,
                    lost_events_total: 0, // TODO: Implement from perf buffer if needed
                    map_usage_percent: map_usage,
                    cpu_overhead_percent: 0.0, // Difficult to measure accurately
                };
            }
        }
        
        EbpfPerfStats {
            enabled: true,
            programs_loaded: 0,
            events_per_sec: 0.0,
            lost_events_total: 0,
            map_usage_percent: 0.0,
            cpu_overhead_percent: 0.0,
        }
    }
    
    #[cfg(feature = "ebpf")]
    fn calculate_event_rate(object: &Object, last_check: &mut Instant, last_count: &mut u64) -> f64 {
        if let Some(map) = Self::find_map(object, "event_counters") {
            let mut total_events = 0u64;
            
            // Sum all event counters (indices 0-3)
            for idx in 0..4u32 {
                let key = idx.to_ne_bytes();
                if let Ok(Some(value)) = map.lookup(&key, MapFlags::ANY) {
                    if value.len() >= 8 {
                        let count = u64::from_ne_bytes(value[0..8].try_into().unwrap());
                        total_events += count;
                    }
                }
            }
            
            // Calculate rate since last check
            let now = Instant::now();
            let elapsed = now.duration_since(*last_check).as_secs_f64();
            if elapsed > 0.0 {
                let events_since_last = total_events.saturating_sub(*last_count);
                *last_count = total_events;
                *last_check = now;
                return events_since_last as f64 / elapsed;
            }
        }
        0.0
    }
    
    #[cfg(feature = "ebpf")]
    fn calculate_map_usage(object: &Object) -> f64 {
        // Calculate usage for the main maps
        let mut total_usage = 0.0;
        let mut map_count = 0;
        
        for map_name in ["net_stats_map", "blkio_stats_map", "tcp_state_map"] {
            if let Some(map) = Self::find_map(object, map_name) {
                // Count entries in the map
                let entry_count = map.keys().count();
                let max_entries = match map_name {
                    "net_stats_map" | "blkio_stats_map" => 10240,
                    "tcp_state_map" => 12,
                    _ => 1,
                };
                
                if max_entries > 0 {
                    total_usage += (entry_count as f64 / max_entries as f64) * 100.0;
                    map_count += 1;
                }
            }
        }
        
        if map_count > 0 {
            total_usage / map_count as f64
        } else {
            0.0
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
