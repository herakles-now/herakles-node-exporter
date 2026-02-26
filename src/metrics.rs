//! Prometheus metrics definitions for herakles-node-exporter.
//!
//! This module defines all the Prometheus metrics according to the system specification.
//! Only system-level and group-level metrics are exposed. No per-process or Top-N metrics.

use prometheus::{Counter, CounterVec, Gauge, GaugeVec, Opts, Registry};

/// Collection of Prometheus metrics according to system specification.
#[derive(Clone)]
pub struct MemoryMetrics {
    // ========== CPU System Metrics ==========
    pub system_cpu_usage_ratio: Gauge,
    pub system_cpu_idle_ratio: Gauge,
    pub system_cpu_iowait_ratio: Gauge,
    pub system_cpu_steal_ratio: Gauge,
    pub system_cpu_load_1: Gauge,
    pub system_cpu_load_5: Gauge,
    pub system_cpu_load_15: Gauge,
    pub system_cpu_psi_wait_seconds_total: Counter,

    // ========== Memory System Metrics ==========
    pub system_memory_total_bytes: Gauge,
    pub system_memory_available_bytes: Gauge,
    pub system_memory_used_ratio: Gauge,
    pub system_memory_cached_bytes: Gauge,
    pub system_memory_buffers_bytes: Gauge,
    pub system_swap_used_ratio: Gauge,
    pub system_memory_psi_wait_seconds_total: Counter,

    // ========== Disk System Metrics ==========
    pub system_disk_read_bytes_total: CounterVec, // labels: device
    pub system_disk_write_bytes_total: CounterVec, // labels: device
    pub system_disk_io_time_seconds_total: CounterVec, // labels: device
    pub system_disk_queue_depth: GaugeVec,      // labels: device
    pub system_disk_psi_wait_seconds_total: Counter,

    // ========== Network System Metrics ==========
    pub system_net_rx_bytes_total: CounterVec,  // labels: iface
    pub system_net_tx_bytes_total: CounterVec,  // labels: iface
    pub system_net_rx_errors_total: CounterVec, // labels: iface
    pub system_net_tx_errors_total: CounterVec, // labels: iface
    pub system_net_drops_total: CounterVec,     // labels: iface, direction

    // ========== Filesystem System Metrics ==========
    pub system_filesystem_avail_bytes: GaugeVec,  // labels: device, mountpoint, fstype
    pub system_filesystem_size_bytes: GaugeVec,   // labels: device, mountpoint, fstype
    pub system_filesystem_files: GaugeVec,        // labels: device, mountpoint, fstype
    pub system_filesystem_files_free: GaugeVec,   // labels: device, mountpoint, fstype

    // ========== TCP Connection Metrics (eBPF) ==========
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_established: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_syn_sent: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_syn_recv: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_fin_wait1: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_fin_wait2: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_time_wait: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_close: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_close_wait: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_last_ack: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_listen: Gauge,
    #[cfg_attr(not(feature = "ebpf"), allow(dead_code))] // Used when eBPF feature is enabled
    pub system_tcp_connections_closing: Gauge,

    // ========== Hardware/Host Metrics ==========
    pub system_cpu_temp_celsius: GaugeVec, // labels: sensor
    pub system_uptime_seconds: Gauge,
    pub system_boot_time_seconds: Gauge,
    pub system_uname_info: GaugeVec, // labels: sysname, release, version, machine

    // ========== Kernel/Runtime Metrics ==========
    pub system_context_switches_total: Counter,
    pub system_forks_total: Counter,
    pub system_open_fds: GaugeVec, // labels: state (allocated/max)
    pub system_entropy_bits: Gauge,

    // ========== CPU Group Metrics ==========
    pub group_cpu_usage_ratio: GaugeVec, // labels: group, subgroup
    pub group_cpu_seconds_total: CounterVec, // labels: group, subgroup, mode

    // ========== Memory Group Metrics ==========
    pub group_memory_rss_bytes: GaugeVec, // labels: group, subgroup
    pub group_memory_pss_bytes: GaugeVec, // labels: group, subgroup
    pub group_memory_swap_bytes: GaugeVec, // labels: group, subgroup

    // ========== Block I/O Group Metrics ==========
    pub group_blkio_read_bytes_total: CounterVec, // labels: group, subgroup
    pub group_blkio_write_bytes_total: CounterVec, // labels: group, subgroup
    pub group_blkio_read_syscalls_total: CounterVec, // labels: group, subgroup
    pub group_blkio_write_syscalls_total: CounterVec, // labels: group, subgroup

    // ========== Network Group Metrics ==========
    pub group_net_rx_bytes_total: CounterVec, // labels: group, subgroup
    pub group_net_tx_bytes_total: CounterVec, // labels: group, subgroup
    pub group_net_connections_total: GaugeVec, // labels: group, subgroup, proto

    // ========== eBPF Performance Metrics ==========
    pub ebpf_events_processed_total: Counter,
    pub ebpf_events_dropped_total: Counter,
    pub ebpf_maps_count: Gauge,
    pub ebpf_cpu_seconds_total: Counter,
}

impl MemoryMetrics {
    /// Creates and registers all Prometheus metrics with the registry.
    pub fn new(registry: &Registry) -> Result<Self, Box<dyn std::error::Error>> {
        // ========== CPU System Metrics ==========
        let system_cpu_usage_ratio = Gauge::new(
            "herakles_system_cpu_usage_ratio",
            "System CPU usage ratio (0.0-1.0)",
        )?;
        let system_cpu_idle_ratio = Gauge::new(
            "herakles_system_cpu_idle_ratio",
            "System CPU idle ratio (0.0-1.0)",
        )?;
        let system_cpu_iowait_ratio = Gauge::new(
            "herakles_system_cpu_iowait_ratio",
            "System CPU iowait ratio (0.0-1.0)",
        )?;
        let system_cpu_steal_ratio = Gauge::new(
            "herakles_system_cpu_steal_ratio",
            "System CPU steal ratio (0.0-1.0)",
        )?;
        let system_cpu_load_1 = Gauge::new(
            "herakles_system_cpu_load_1",
            "System load average over 1 minute",
        )?;
        let system_cpu_load_5 = Gauge::new(
            "herakles_system_cpu_load_5",
            "System load average over 5 minutes",
        )?;
        let system_cpu_load_15 = Gauge::new(
            "herakles_system_cpu_load_15",
            "System load average over 15 minutes",
        )?;
        let system_cpu_psi_wait_seconds_total = Counter::new(
            "herakles_system_cpu_psi_wait_seconds_total",
            "Total CPU pressure stall time in seconds",
        )?;

        // ========== Memory System Metrics ==========
        let system_memory_total_bytes = Gauge::new(
            "herakles_system_memory_total_bytes",
            "Total system memory in bytes",
        )?;
        let system_memory_available_bytes = Gauge::new(
            "herakles_system_memory_available_bytes",
            "Available system memory in bytes",
        )?;
        let system_memory_used_ratio = Gauge::new(
            "herakles_system_memory_used_ratio",
            "System memory used ratio (0.0-1.0)",
        )?;
        let system_memory_cached_bytes = Gauge::new(
            "herakles_system_memory_cached_bytes",
            "Page cache memory in bytes",
        )?;
        let system_memory_buffers_bytes = Gauge::new(
            "herakles_system_memory_buffers_bytes",
            "Buffer cache memory in bytes",
        )?;
        let system_swap_used_ratio = Gauge::new(
            "herakles_system_swap_used_ratio",
            "System swap memory used ratio (0.0-1.0)",
        )?;
        let system_memory_psi_wait_seconds_total = Counter::new(
            "herakles_system_memory_psi_wait_seconds_total",
            "Total memory pressure stall time in seconds",
        )?;

        // ========== Disk System Metrics ==========
        let system_disk_read_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_system_disk_read_bytes_total",
                "Total bytes read from disk device",
            ),
            &["device"],
        )?;
        let system_disk_write_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_system_disk_write_bytes_total",
                "Total bytes written to disk device",
            ),
            &["device"],
        )?;
        let system_disk_io_time_seconds_total = CounterVec::new(
            Opts::new(
                "herakles_system_disk_io_time_seconds_total",
                "Total time spent doing I/Os in seconds",
            ),
            &["device"],
        )?;
        let system_disk_queue_depth = GaugeVec::new(
            Opts::new(
                "herakles_system_disk_queue_depth",
                "Number of I/O operations currently in progress for disk device",
            ),
            &["device"],
        )?;
        let system_disk_psi_wait_seconds_total = Counter::new(
            "herakles_system_disk_psi_wait_seconds_total",
            "Total I/O pressure stall time in seconds",
        )?;

        // ========== Network System Metrics ==========
        let system_net_rx_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_system_net_rx_bytes_total",
                "Total bytes received per network interface",
            ),
            &["iface"],
        )?;
        let system_net_tx_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_system_net_tx_bytes_total",
                "Total bytes transmitted per network interface",
            ),
            &["iface"],
        )?;
        let system_net_rx_errors_total = CounterVec::new(
            Opts::new(
                "herakles_system_net_rx_errors_total",
                "Total receive errors per network interface",
            ),
            &["iface"],
        )?;
        let system_net_tx_errors_total = CounterVec::new(
            Opts::new(
                "herakles_system_net_tx_errors_total",
                "Total transmit errors per network interface",
            ),
            &["iface"],
        )?;
        let system_net_drops_total = CounterVec::new(
            Opts::new(
                "herakles_system_net_drops_total",
                "Total dropped packets per network interface and direction",
            ),
            &["iface", "direction"],
        )?;

        // ========== Filesystem System Metrics ==========
        let system_filesystem_avail_bytes = GaugeVec::new(
            Opts::new(
                "herakles_system_filesystem_avail_bytes",
                "Filesystem space available to non-root users in bytes",
            ),
            &["device", "mountpoint", "fstype"],
        )?;
        let system_filesystem_size_bytes = GaugeVec::new(
            Opts::new(
                "herakles_system_filesystem_size_bytes",
                "Filesystem total size in bytes",
            ),
            &["device", "mountpoint", "fstype"],
        )?;
        let system_filesystem_files = GaugeVec::new(
            Opts::new(
                "herakles_system_filesystem_files",
                "Filesystem total file nodes",
            ),
            &["device", "mountpoint", "fstype"],
        )?;
        let system_filesystem_files_free = GaugeVec::new(
            Opts::new(
                "herakles_system_filesystem_files_free",
                "Filesystem free file nodes",
            ),
            &["device", "mountpoint", "fstype"],
        )?;

        // ========== TCP Connection Metrics (eBPF) ==========
        let system_tcp_connections_established = Gauge::new(
            "herakles_system_tcp_connections_established",
            "Number of TCP connections in ESTABLISHED state",
        )?;
        let system_tcp_connections_syn_sent = Gauge::new(
            "herakles_system_tcp_connections_syn_sent",
            "Number of TCP connections in SYN_SENT state",
        )?;
        let system_tcp_connections_syn_recv = Gauge::new(
            "herakles_system_tcp_connections_syn_recv",
            "Number of TCP connections in SYN_RECV state",
        )?;
        let system_tcp_connections_fin_wait1 = Gauge::new(
            "herakles_system_tcp_connections_fin_wait1",
            "Number of TCP connections in FIN_WAIT1 state",
        )?;
        let system_tcp_connections_fin_wait2 = Gauge::new(
            "herakles_system_tcp_connections_fin_wait2",
            "Number of TCP connections in FIN_WAIT2 state",
        )?;
        let system_tcp_connections_time_wait = Gauge::new(
            "herakles_system_tcp_connections_time_wait",
            "Number of TCP connections in TIME_WAIT state",
        )?;
        let system_tcp_connections_close = Gauge::new(
            "herakles_system_tcp_connections_close",
            "Number of TCP connections in CLOSE state",
        )?;
        let system_tcp_connections_close_wait = Gauge::new(
            "herakles_system_tcp_connections_close_wait",
            "Number of TCP connections in CLOSE_WAIT state",
        )?;
        let system_tcp_connections_last_ack = Gauge::new(
            "herakles_system_tcp_connections_last_ack",
            "Number of TCP connections in LAST_ACK state",
        )?;
        let system_tcp_connections_listen = Gauge::new(
            "herakles_system_tcp_connections_listen",
            "Number of TCP connections in LISTEN state",
        )?;
        let system_tcp_connections_closing = Gauge::new(
            "herakles_system_tcp_connections_closing",
            "Number of TCP connections in CLOSING state",
        )?;

        // ========== Hardware/Host Metrics ==========
        let system_cpu_temp_celsius = GaugeVec::new(
            Opts::new(
                "herakles_system_cpu_temp_celsius",
                "CPU/sensor temperature in Celsius",
            ),
            &["sensor"],
        )?;
        let system_uptime_seconds =
            Gauge::new("herakles_system_uptime_seconds", "System uptime in seconds")?;
        let system_boot_time_seconds = Gauge::new(
            "herakles_system_boot_time_seconds",
            "System boot time as Unix timestamp",
        )?;
        let system_uname_info = GaugeVec::new(
            Opts::new(
                "herakles_system_uname_info",
                "System information from uname",
            ),
            &["sysname", "release", "version", "machine"],
        )?;

        // ========== Kernel/Runtime Metrics ==========
        let system_context_switches_total = Counter::new(
            "herakles_system_context_switches_total",
            "Total number of context switches",
        )?;
        let system_forks_total = Counter::new(
            "herakles_system_forks_total",
            "Total number of forks since boot",
        )?;
        let system_open_fds = GaugeVec::new(
            Opts::new(
                "herakles_system_open_fds",
                "Number of file descriptors system-wide",
            ),
            &["state"],
        )?;
        let system_entropy_bits =
            Gauge::new("herakles_system_entropy_bits", "Available entropy in bits")?;

        // ========== CPU Group Metrics ==========
        let group_cpu_usage_ratio = GaugeVec::new(
            Opts::new(
                "herakles_group_cpu_usage_ratio",
                "CPU usage ratio per group and subgroup (0.0-1.0)",
            ),
            &["group", "subgroup"],
        )?;
        let group_cpu_seconds_total = CounterVec::new(
            Opts::new(
                "herakles_group_cpu_seconds_total",
                "Total CPU time in seconds per group, subgroup, and mode",
            ),
            &["group", "subgroup", "mode"],
        )?;

        // ========== Memory Group Metrics ==========
        let group_memory_rss_bytes = GaugeVec::new(
            Opts::new(
                "herakles_group_memory_rss_bytes",
                "Sum of RSS bytes per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let group_memory_pss_bytes = GaugeVec::new(
            Opts::new(
                "herakles_group_memory_pss_bytes",
                "Sum of PSS bytes per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let group_memory_swap_bytes = GaugeVec::new(
            Opts::new(
                "herakles_group_memory_swap_bytes",
                "Sum of swap bytes per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;

        // ========== Block I/O Group Metrics ==========
        let group_blkio_read_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_group_blkio_read_bytes_total",
                "Total bytes read per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let group_blkio_write_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_group_blkio_write_bytes_total",
                "Total bytes written per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let group_blkio_read_syscalls_total = CounterVec::new(
            Opts::new(
                "herakles_group_blkio_read_syscalls_total",
                "Total read syscalls per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let group_blkio_write_syscalls_total = CounterVec::new(
            Opts::new(
                "herakles_group_blkio_write_syscalls_total",
                "Total write syscalls per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;

        // ========== Network Group Metrics ==========
        let group_net_rx_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_group_net_rx_bytes_total",
                "Total bytes received per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;
        let group_net_tx_bytes_total = CounterVec::new(
            Opts::new(
                "herakles_group_net_tx_bytes_total",
                "Total bytes transmitted per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;
        let group_net_connections_total = GaugeVec::new(
            Opts::new(
                "herakles_group_net_connections_total",
                "Total network connections per group, subgroup, and protocol",
            ),
            &["group", "subgroup", "proto"],
        )?;

        // ========== eBPF Performance Metrics ==========
        let ebpf_events_processed_total = Counter::new(
            "herakles_ebpf_events_processed_total",
            "Total number of eBPF events processed",
        )?;
        let ebpf_events_dropped_total = Counter::new(
            "herakles_ebpf_events_dropped_total",
            "Total number of eBPF events dropped",
        )?;
        let ebpf_maps_count = Gauge::new(
            "herakles_ebpf_maps_count",
            "Number of eBPF programs currently loaded",
        )?;
        let ebpf_cpu_seconds_total = Counter::new(
            "herakles_ebpf_cpu_seconds_total",
            "Total CPU time used by eBPF programs in seconds",
        )?;

        // ========== Register All Metrics ==========
        // CPU System
        registry.register(Box::new(system_cpu_usage_ratio.clone()))?;
        registry.register(Box::new(system_cpu_idle_ratio.clone()))?;
        registry.register(Box::new(system_cpu_iowait_ratio.clone()))?;
        registry.register(Box::new(system_cpu_steal_ratio.clone()))?;
        registry.register(Box::new(system_cpu_load_1.clone()))?;
        registry.register(Box::new(system_cpu_load_5.clone()))?;
        registry.register(Box::new(system_cpu_load_15.clone()))?;
        registry.register(Box::new(system_cpu_psi_wait_seconds_total.clone()))?;

        // Memory System
        registry.register(Box::new(system_memory_total_bytes.clone()))?;
        registry.register(Box::new(system_memory_available_bytes.clone()))?;
        registry.register(Box::new(system_memory_used_ratio.clone()))?;
        registry.register(Box::new(system_memory_cached_bytes.clone()))?;
        registry.register(Box::new(system_memory_buffers_bytes.clone()))?;
        registry.register(Box::new(system_swap_used_ratio.clone()))?;
        registry.register(Box::new(system_memory_psi_wait_seconds_total.clone()))?;

        // Disk System
        registry.register(Box::new(system_disk_read_bytes_total.clone()))?;
        registry.register(Box::new(system_disk_write_bytes_total.clone()))?;
        registry.register(Box::new(system_disk_io_time_seconds_total.clone()))?;
        registry.register(Box::new(system_disk_queue_depth.clone()))?;
        registry.register(Box::new(system_disk_psi_wait_seconds_total.clone()))?;

        // Network System
        registry.register(Box::new(system_net_rx_bytes_total.clone()))?;
        registry.register(Box::new(system_net_tx_bytes_total.clone()))?;
        registry.register(Box::new(system_net_rx_errors_total.clone()))?;
        registry.register(Box::new(system_net_tx_errors_total.clone()))?;
        registry.register(Box::new(system_net_drops_total.clone()))?;

        // Filesystem System
        registry.register(Box::new(system_filesystem_avail_bytes.clone()))?;
        registry.register(Box::new(system_filesystem_size_bytes.clone()))?;
        registry.register(Box::new(system_filesystem_files.clone()))?;
        registry.register(Box::new(system_filesystem_files_free.clone()))?;

        // TCP Connections
        registry.register(Box::new(system_tcp_connections_established.clone()))?;
        registry.register(Box::new(system_tcp_connections_syn_sent.clone()))?;
        registry.register(Box::new(system_tcp_connections_syn_recv.clone()))?;
        registry.register(Box::new(system_tcp_connections_fin_wait1.clone()))?;
        registry.register(Box::new(system_tcp_connections_fin_wait2.clone()))?;
        registry.register(Box::new(system_tcp_connections_time_wait.clone()))?;
        registry.register(Box::new(system_tcp_connections_close.clone()))?;
        registry.register(Box::new(system_tcp_connections_close_wait.clone()))?;
        registry.register(Box::new(system_tcp_connections_last_ack.clone()))?;
        registry.register(Box::new(system_tcp_connections_listen.clone()))?;
        registry.register(Box::new(system_tcp_connections_closing.clone()))?;

        // Hardware/Host
        registry.register(Box::new(system_cpu_temp_celsius.clone()))?;
        registry.register(Box::new(system_uptime_seconds.clone()))?;
        registry.register(Box::new(system_boot_time_seconds.clone()))?;
        registry.register(Box::new(system_uname_info.clone()))?;

        // Kernel/Runtime
        registry.register(Box::new(system_context_switches_total.clone()))?;
        registry.register(Box::new(system_forks_total.clone()))?;
        registry.register(Box::new(system_open_fds.clone()))?;
        registry.register(Box::new(system_entropy_bits.clone()))?;

        // CPU Group
        registry.register(Box::new(group_cpu_usage_ratio.clone()))?;
        registry.register(Box::new(group_cpu_seconds_total.clone()))?;

        // Memory Group
        registry.register(Box::new(group_memory_rss_bytes.clone()))?;
        registry.register(Box::new(group_memory_pss_bytes.clone()))?;
        registry.register(Box::new(group_memory_swap_bytes.clone()))?;

        // Block I/O Group
        registry.register(Box::new(group_blkio_read_bytes_total.clone()))?;
        registry.register(Box::new(group_blkio_write_bytes_total.clone()))?;
        registry.register(Box::new(group_blkio_read_syscalls_total.clone()))?;
        registry.register(Box::new(group_blkio_write_syscalls_total.clone()))?;

        // Network Group
        registry.register(Box::new(group_net_rx_bytes_total.clone()))?;
        registry.register(Box::new(group_net_tx_bytes_total.clone()))?;
        registry.register(Box::new(group_net_connections_total.clone()))?;

        // eBPF Performance Metrics
        registry.register(Box::new(ebpf_events_processed_total.clone()))?;
        registry.register(Box::new(ebpf_events_dropped_total.clone()))?;
        registry.register(Box::new(ebpf_maps_count.clone()))?;
        registry.register(Box::new(ebpf_cpu_seconds_total.clone()))?;

        Ok(Self {
            system_cpu_usage_ratio,
            system_cpu_idle_ratio,
            system_cpu_iowait_ratio,
            system_cpu_steal_ratio,
            system_cpu_load_1,
            system_cpu_load_5,
            system_cpu_load_15,
            system_cpu_psi_wait_seconds_total,
            system_memory_total_bytes,
            system_memory_available_bytes,
            system_memory_used_ratio,
            system_memory_cached_bytes,
            system_memory_buffers_bytes,
            system_swap_used_ratio,
            system_memory_psi_wait_seconds_total,
            system_disk_read_bytes_total,
            system_disk_write_bytes_total,
            system_disk_io_time_seconds_total,
            system_disk_queue_depth,
            system_disk_psi_wait_seconds_total,
            system_net_rx_bytes_total,
            system_net_tx_bytes_total,
            system_net_rx_errors_total,
            system_net_tx_errors_total,
            system_net_drops_total,
            system_filesystem_avail_bytes,
            system_filesystem_size_bytes,
            system_filesystem_files,
            system_filesystem_files_free,
            system_tcp_connections_established,
            system_tcp_connections_syn_sent,
            system_tcp_connections_syn_recv,
            system_tcp_connections_fin_wait1,
            system_tcp_connections_fin_wait2,
            system_tcp_connections_time_wait,
            system_tcp_connections_close,
            system_tcp_connections_close_wait,
            system_tcp_connections_last_ack,
            system_tcp_connections_listen,
            system_tcp_connections_closing,
            system_cpu_temp_celsius,
            system_uptime_seconds,
            system_boot_time_seconds,
            system_uname_info,
            system_context_switches_total,
            system_forks_total,
            system_open_fds,
            system_entropy_bits,
            group_cpu_usage_ratio,
            group_cpu_seconds_total,
            group_memory_rss_bytes,
            group_memory_pss_bytes,
            group_memory_swap_bytes,
            group_blkio_read_bytes_total,
            group_blkio_write_bytes_total,
            group_blkio_read_syscalls_total,
            group_blkio_write_syscalls_total,
            group_net_rx_bytes_total,
            group_net_tx_bytes_total,
            group_net_connections_total,
            ebpf_events_processed_total,
            ebpf_events_dropped_total,
            ebpf_maps_count,
            ebpf_cpu_seconds_total,
        })
    }

    /// Resets only group-level metrics (not system-level metrics).
    /// 
    /// This is more efficient than `reset()` because it only resets metrics that
    /// change frequently based on process aggregation. System-level metrics like
    /// disk stats, network stats, and hardware info are queried fresh on every
    /// scrape and don't need to be reset.
    /// 
    /// Use this method in the metrics handler to reduce unnecessary work and
    /// improve scrape performance.
    /// 
    /// Note: Counter metrics are never reset as they are monotonically increasing.
    pub fn reset_group_metrics(&self) {
        // CPU Group - only reset usage ratio (gauge), not cpu_seconds_total (counter)
        self.group_cpu_usage_ratio.reset();

        // Memory Group
        self.group_memory_rss_bytes.reset();
        self.group_memory_pss_bytes.reset();
        self.group_memory_swap_bytes.reset();

        // Network Group - only reset connections (gauge)
        self.group_net_connections_total.reset();
    }
}
