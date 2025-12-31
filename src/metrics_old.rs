//! Prometheus metrics definitions for herakles-node-exporter.
//!
//! This module defines all the Prometheus metrics used to export process
//! memory and CPU usage information.

use crate::config::Config;
use prometheus::{Gauge, GaugeVec, Opts, Registry};

/// Collection of Prometheus metrics for memory and CPU monitoring.
#[derive(Clone)]
pub struct MemoryMetrics {
    // Node-Level Metrics (28 metrics)
    pub node_uptime_seconds: Gauge,
    pub node_cpu_usage_percent: Gauge,
    pub node_cpu_iowait_percent: Gauge,
    pub node_cpu_steal_percent: Gauge,
    pub node_mem_total_bytes: Gauge,
    pub node_mem_used_bytes: Gauge,
    pub node_mem_available_bytes: Gauge,
    pub node_mem_cached_bytes: Gauge,
    pub node_mem_buffers_bytes: Gauge,
    pub node_mem_swap_used_bytes: Gauge,
    pub node_mem_swap_total_bytes: Gauge,
    pub node_io_read_bytes_per_second: Gauge,
    pub node_io_write_bytes_per_second: Gauge,
    pub node_io_read_iops_per_second: Gauge,
    pub node_io_write_iops_per_second: Gauge,
    pub node_net_rx_bytes_per_second: Gauge,
    pub node_net_tx_bytes_per_second: Gauge,
    pub node_net_rx_dropped_packets_per_second: Gauge,
    pub node_net_tx_dropped_packets_per_second: Gauge,
    pub node_net_rx_error_packets_per_second: Gauge,
    pub node_net_tx_error_packets_per_second: Gauge,
    pub node_fd_open: Gauge,
    pub node_fd_kernel_max: Gauge,
    pub node_fd_used_ratio: Gauge,
    pub node_load1: Gauge,
    pub node_load5: Gauge,
    pub node_load15: Gauge,

    // New system-level metrics
    pub system_uptime_seconds: Gauge,
    pub system_boot_time_seconds: Gauge,
    pub system_context_switches_total: Gauge,
    pub system_forks_total: Gauge,
    pub system_open_fds: Gauge,
    pub system_entropy_bits: Gauge,
    pub system_cpu_psi_wait_seconds_total: Gauge,
    pub system_memory_psi_wait_seconds_total: Gauge,
    pub system_disk_psi_wait_seconds_total: Gauge,

    // Subgroup-Level Metrics (13 metrics) - Labels: group, subgroup
    pub subgroup_info: GaugeVec,
    pub mem_rss_subgroup_bytes: GaugeVec,
    pub mem_pss_subgroup_bytes: GaugeVec,
    pub mem_uss_subgroup_bytes: GaugeVec,
    pub mem_swap_subgroup_bytes: GaugeVec,
    pub cpu_usage_subgroup_percent: GaugeVec,
    pub cpu_iowait_subgroup_percent: GaugeVec,
    pub io_read_subgroup_bytes_per_second: GaugeVec,
    pub io_write_subgroup_bytes_per_second: GaugeVec,
    pub net_rx_subgroup_bytes_per_second: GaugeVec,
    pub net_tx_subgroup_bytes_per_second: GaugeVec,
    pub subgroup_oldest_uptime_seconds: GaugeVec,
    pub subgroup_alert_armed: GaugeVec,

    // Top-3 RSS Memory metrics (6 metrics) - Labels: group, subgroup (and comm for _comm metrics)
    // NOTE: PID metrics removed - PIDs are kept internal only
    pub mem_rss_subgroup_top1_bytes: GaugeVec,
    pub mem_rss_subgroup_top2_bytes: GaugeVec,
    pub mem_rss_subgroup_top3_bytes: GaugeVec,
    pub mem_rss_subgroup_top1_comm: GaugeVec, // Labels: group, subgroup, comm
    pub mem_rss_subgroup_top2_comm: GaugeVec, // Labels: group, subgroup, comm
    pub mem_rss_subgroup_top3_comm: GaugeVec, // Labels: group, subgroup, comm

    // Top-3 CPU Usage metrics (6 metrics) - Labels: group, subgroup (and comm for _comm metrics)
    // NOTE: PID metrics removed - PIDs are kept internal only
    pub cpu_usage_subgroup_top1_percent: GaugeVec,
    pub cpu_usage_subgroup_top2_percent: GaugeVec,
    pub cpu_usage_subgroup_top3_percent: GaugeVec,
    pub cpu_usage_subgroup_top1_comm: GaugeVec, // Labels: group, subgroup, comm
    pub cpu_usage_subgroup_top2_comm: GaugeVec, // Labels: group, subgroup, comm
    pub cpu_usage_subgroup_top3_comm: GaugeVec, // Labels: group, subgroup, comm

    // Group Core Metrics (6 new metrics) - Labels: group, subgroup
    pub group_memory_rss_bytes_sum: GaugeVec,
    pub group_memory_pss_bytes_sum: GaugeVec,
    pub group_memory_uss_bytes_sum: GaugeVec,
    pub group_cpu_usage_percent_sum: GaugeVec,
    pub group_cpu_time_seconds_sum: GaugeVec,
    pub group_cpu_uptime_oldest_process_seconds: GaugeVec,

    // System Ratios (6 new metrics) - No labels
    pub system_cpu_usage_ratio: Gauge,
    pub system_cpu_idle_ratio: Gauge,
    pub system_cpu_iowait_ratio: Gauge,
    pub system_cpu_steal_ratio: Gauge,
    pub system_memory_used_ratio: Gauge,
    pub system_memory_swap_used_ratio: Gauge,

    // Disk Device-Level (5 new metrics) - Labels: device
    pub system_disk_reads_completed_total: GaugeVec,
    pub system_disk_read_bytes_total: GaugeVec,
    pub system_disk_writes_completed_total: GaugeVec,
    pub system_disk_write_bytes_total: GaugeVec,
    pub system_disk_io_now: GaugeVec,

    // Thermal Metrics - Labels: sensor
    pub system_cpu_temp_celsius: GaugeVec,

    // Filesystem (4 new metrics) - Labels: device, mountpoint, fstype
    pub filesystem_avail_bytes: GaugeVec,
    pub filesystem_size_bytes: GaugeVec,
    pub filesystem_files: GaugeVec,
    pub filesystem_files_free: GaugeVec,

    // Network Device-Level (5 new metrics) - Labels: device
    pub system_net_receive_bytes_total: GaugeVec,
    pub system_net_transmit_bytes_total: GaugeVec,
    pub system_net_receive_packets_total: GaugeVec,
    pub system_net_receive_errs_total: GaugeVec,
    pub system_net_receive_drop_total: GaugeVec,

    // eBPF Group Network (4 new metrics) - Labels: group, subgroup
    pub group_net_rx_bytes_total: GaugeVec,
    pub group_net_tx_bytes_total: GaugeVec,
    pub group_net_packets_total: GaugeVec,
    pub group_net_dropped_total: GaugeVec,

    // eBPF Group I/O (2 new metrics) - Labels: group, subgroup
    pub group_blkio_read_bytes_total: GaugeVec,
    pub group_blkio_write_bytes_total: GaugeVec,
}

impl MemoryMetrics {
    /// Creates and registers all Prometheus metrics with the registry.
    pub fn new(registry: &Registry) -> Result<Self, Box<dyn std::error::Error>> {
        // Node-Level Metrics (28 metrics)
        let node_uptime_seconds = Gauge::new(
            "herakles_node_uptime_seconds",
            "System uptime in seconds from /proc/uptime",
        )?;
        let node_cpu_usage_percent = Gauge::new(
            "herakles_node_cpu_usage_percent",
            "Total CPU usage percentage across all cores",
        )?;
        let node_cpu_iowait_percent = Gauge::new(
            "herakles_node_cpu_iowait_percent",
            "Total CPU iowait percentage across all cores",
        )?;
        let node_cpu_steal_percent = Gauge::new(
            "herakles_node_cpu_steal_percent",
            "Total CPU steal percentage across all cores",
        )?;
        let node_mem_total_bytes = Gauge::new(
            "herakles_node_mem_total_bytes",
            "Total system memory in bytes",
        )?;
        let node_mem_used_bytes = Gauge::new(
            "herakles_node_mem_used_bytes",
            "Used system memory in bytes (total - available)",
        )?;
        let node_mem_available_bytes = Gauge::new(
            "herakles_node_mem_available_bytes",
            "Available system memory in bytes",
        )?;
        let node_mem_cached_bytes = Gauge::new(
            "herakles_node_mem_cached_bytes",
            "Page cache memory in bytes",
        )?;
        let node_mem_buffers_bytes = Gauge::new(
            "herakles_node_mem_buffers_bytes",
            "Buffer cache memory in bytes",
        )?;
        let node_mem_swap_used_bytes = Gauge::new(
            "herakles_node_mem_swap_used_bytes",
            "Used swap space in bytes",
        )?;
        let node_mem_swap_total_bytes = Gauge::new(
            "herakles_node_mem_swap_total_bytes",
            "Total swap space in bytes",
        )?;
        let node_io_read_bytes_per_second = Gauge::new(
            "herakles_node_io_read_bytes_per_second",
            "Total I/O read throughput in bytes per second",
        )?;
        let node_io_write_bytes_per_second = Gauge::new(
            "herakles_node_io_write_bytes_per_second",
            "Total I/O write throughput in bytes per second",
        )?;
        let node_io_read_iops_per_second = Gauge::new(
            "herakles_node_io_read_iops_per_second",
            "Total I/O read operations per second",
        )?;
        let node_io_write_iops_per_second = Gauge::new(
            "herakles_node_io_write_iops_per_second",
            "Total I/O write operations per second",
        )?;
        let node_net_rx_bytes_per_second = Gauge::new(
            "herakles_node_net_rx_bytes_per_second",
            "Total network receive throughput in bytes per second",
        )?;
        let node_net_tx_bytes_per_second = Gauge::new(
            "herakles_node_net_tx_bytes_per_second",
            "Total network transmit throughput in bytes per second",
        )?;
        let node_net_rx_dropped_packets_per_second = Gauge::new(
            "herakles_node_net_rx_dropped_packets_per_second",
            "Total network receive dropped packets per second",
        )?;
        let node_net_tx_dropped_packets_per_second = Gauge::new(
            "herakles_node_net_tx_dropped_packets_per_second",
            "Total network transmit dropped packets per second",
        )?;
        let node_net_rx_error_packets_per_second = Gauge::new(
            "herakles_node_net_rx_error_packets_per_second",
            "Total network receive error packets per second",
        )?;
        let node_net_tx_error_packets_per_second = Gauge::new(
            "herakles_node_net_tx_error_packets_per_second",
            "Total network transmit error packets per second",
        )?;
        let node_fd_open = Gauge::new(
            "herakles_node_fd_open",
            "Number of open file descriptors system-wide from /proc/sys/fs/file-nr",
        )?;
        let node_fd_kernel_max = Gauge::new(
            "herakles_node_fd_kernel_max",
            "Maximum number of file descriptors system-wide from /proc/sys/fs/file-nr",
        )?;
        let node_fd_used_ratio = Gauge::new(
            "herakles_node_fd_used_ratio",
            "Ratio of used file descriptors (open / max)",
        )?;
        let node_load1 = Gauge::new("herakles_node_load1", "System load average over 1 minute")?;
        let node_load5 = Gauge::new("herakles_node_load5", "System load average over 5 minutes")?;
        let node_load15 = Gauge::new(
            "herakles_node_load15",
            "System load average over 15 minutes",
        )?;

        // New system-level metrics
        let system_uptime_seconds = Gauge::new(
            "herakles_system_uptime_seconds",
            "System uptime in seconds",
        )?;
        let system_boot_time_seconds = Gauge::new(
            "herakles_system_boot_time_seconds",
            "System boot time as Unix timestamp",
        )?;
        let system_context_switches_total = Gauge::new(
            "herakles_system_context_switches_total",
            "Total number of context switches",
        )?;
        let system_forks_total = Gauge::new(
            "herakles_system_forks_total",
            "Total number of forks since boot",
        )?;
        let system_open_fds = Gauge::new(
            "herakles_system_open_fds",
            "Number of open file descriptors system-wide",
        )?;
        let system_entropy_bits = Gauge::new(
            "herakles_system_entropy_bits",
            "Available entropy in bits",
        )?;
        let system_cpu_psi_wait_seconds_total = Gauge::new(
            "herakles_system_cpu_psi_wait_seconds_total",
            "Total CPU pressure stall time in seconds",
        )?;
        let system_memory_psi_wait_seconds_total = Gauge::new(
            "herakles_system_memory_psi_wait_seconds_total",
            "Total memory pressure stall time in seconds",
        )?;
        let system_disk_psi_wait_seconds_total = Gauge::new(
            "herakles_system_disk_psi_wait_seconds_total",
            "Total I/O pressure stall time in seconds",
        )?;

        // Subgroup metadata metrics
        let subgroup_info = GaugeVec::new(
            Opts::new(
                "herakles_subgroup_info",
                "Subgroup information (always 1.0)",
            ),
            &["group", "subgroup"],
        )?;
        let subgroup_oldest_uptime_seconds = GaugeVec::new(
            Opts::new(
                "herakles_subgroup_oldest_uptime_seconds",
                "Oldest process uptime in seconds per subgroup",
            ),
            &["subgroup"],
        )?;
        let subgroup_alert_armed = GaugeVec::new(
            Opts::new(
                "herakles_subgroup_alert_armed",
                "Alert armed status per subgroup (1.0 = armed, 0.0 = not armed)",
            ),
            &["subgroup"],
        )?;

        // Subgroup-level aggregated metrics (without uptime label)
        let mem_rss_subgroup_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_bytes",
                "Sum of RSS bytes per subgroup",
            ),
            &["subgroup"],
        )?;
        let mem_pss_subgroup_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_pss_subgroup_bytes",
                "Sum of PSS bytes per subgroup",
            ),
            &["subgroup"],
        )?;
        let mem_uss_subgroup_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_uss_subgroup_bytes",
                "Sum of USS bytes per subgroup",
            ),
            &["subgroup"],
        )?;
        let mem_swap_subgroup_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_swap_subgroup_bytes",
                "Sum of swap bytes per subgroup",
            ),
            &["subgroup"],
        )?;
        let cpu_usage_subgroup_percent = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_percent",
                "CPU usage percentage per subgroup",
            ),
            &["subgroup"],
        )?;
        let cpu_iowait_subgroup_percent = GaugeVec::new(
            Opts::new(
                "herakles_cpu_iowait_subgroup_percent",
                "CPU iowait percentage per subgroup",
            ),
            &["subgroup"],
        )?;
        let io_read_subgroup_bytes_per_second = GaugeVec::new(
            Opts::new(
                "herakles_io_read_subgroup_bytes_per_second",
                "I/O read throughput in bytes per second per subgroup",
            ),
            &["subgroup"],
        )?;
        let io_write_subgroup_bytes_per_second = GaugeVec::new(
            Opts::new(
                "herakles_io_write_subgroup_bytes_per_second",
                "I/O write throughput in bytes per second per subgroup",
            ),
            &["subgroup"],
        )?;
        let net_rx_subgroup_bytes_per_second = GaugeVec::new(
            Opts::new(
                "herakles_net_rx_subgroup_bytes_per_second",
                "Network receive throughput in bytes per second per subgroup",
            ),
            &["subgroup"],
        )?;
        let net_tx_subgroup_bytes_per_second = GaugeVec::new(
            Opts::new(
                "herakles_net_tx_subgroup_bytes_per_second",
                "Network transmit throughput in bytes per second per subgroup",
            ),
            &["subgroup"],
        )?;

        // Top-3 RSS Memory metrics (separate for top1, top2, top3)
        let mem_rss_subgroup_top1_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top1_bytes",
                "Top 1 RSS bytes per subgroup",
            ),
            &["subgroup"],
        )?;
        let mem_rss_subgroup_top2_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top2_bytes",
                "Top 2 RSS bytes per subgroup",
            ),
            &["subgroup"],
        )?;
        let mem_rss_subgroup_top3_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top3_bytes",
                "Top 3 RSS bytes per subgroup",
            ),
            &["subgroup"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let mem_rss_subgroup_top1_comm = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top1_comm",
                "Top 1 RSS process name per subgroup",
            )
            .const_label("_type", "info"),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let mem_rss_subgroup_top2_comm = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top2_comm",
                "Top 2 RSS process name per subgroup",
            )
            .const_label("_type", "info"),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let mem_rss_subgroup_top3_comm = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top3_comm",
                "Top 3 RSS process name per subgroup",
            )
            .const_label("_type", "info"),
            &["subgroup", "comm"],
        )?;

        // Top-3 CPU Usage metrics (separate for top1, top2, top3)
        let cpu_usage_subgroup_top1_percent = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top1_percent",
                "Top 1 CPU usage percentage per subgroup",
            ),
            &["subgroup"],
        )?;
        let cpu_usage_subgroup_top2_percent = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top2_percent",
                "Top 2 CPU usage percentage per subgroup",
            ),
            &["subgroup"],
        )?;
        let cpu_usage_subgroup_top3_percent = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top3_percent",
                "Top 3 CPU usage percentage per subgroup",
            ),
            &["subgroup"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let cpu_usage_subgroup_top1_comm = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top1_comm",
                "Top 1 CPU usage process name per subgroup",
            )
            .const_label("_type", "info"),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let cpu_usage_subgroup_top2_comm = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top2_comm",
                "Top 2 CPU usage process name per subgroup",
            )
            .const_label("_type", "info"),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let cpu_usage_subgroup_top3_comm = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top3_comm",
                "Top 3 CPU usage process name per subgroup",
            )
            .const_label("_type", "info"),
            &["subgroup", "comm"],
        )?;

        // Group Core Metrics (6 new metrics)
        let mem_group_rss_bytes_sum = GaugeVec::new(
            Opts::new(
                "herakles_group_memory_rss_bytes",
                "Sum of RSS bytes per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let mem_group_pss_bytes_sum = GaugeVec::new(
            Opts::new(
                "herakles_group_memory_pss_bytes",
                "Sum of PSS bytes per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let mem_group_uss_bytes_sum = GaugeVec::new(
            Opts::new(
                "herakles_group_memory_uss_bytes",
                "Sum of USS bytes per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let cpu_group_usage_percent_sum = GaugeVec::new(
            Opts::new(
                "herakles_group_cpu_usage_percent",
                "Sum of CPU usage percentage per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let cpu_group_time_seconds_sum = GaugeVec::new(
            Opts::new(
                "herakles_group_cpu_time_seconds",
                "Sum of CPU time in seconds per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;
        let cpu_group_uptime_oldest_process_seconds = GaugeVec::new(
            Opts::new(
                "herakles_group_cpu_uptime_oldest_process_seconds",
                "Uptime in seconds of oldest process per group and subgroup",
            ),
            &["group", "subgroup"],
        )?;

        // System Ratios (6 new metrics)
        let cpu_system_usage_ratio = Gauge::new(
            "herakles_system_cpu_usage_ratio",
            "System CPU usage ratio (0.0-1.0)",
        )?;
        let cpu_system_idle_ratio = Gauge::new(
            "herakles_system_cpu_idle_ratio",
            "System CPU idle ratio (0.0-1.0)",
        )?;
        let cpu_system_iowait_ratio = Gauge::new(
            "herakles_system_cpu_iowait_ratio",
            "System CPU iowait ratio (0.0-1.0)",
        )?;
        let cpu_system_steal_ratio = Gauge::new(
            "herakles_system_cpu_steal_ratio",
            "System CPU steal ratio (0.0-1.0)",
        )?;
        let mem_system_used_ratio = Gauge::new(
            "herakles_system_memory_used_ratio",
            "System memory used ratio (0.0-1.0)",
        )?;
        let mem_system_swap_used_ratio = Gauge::new(
            "herakles_system_memory_swap_used_ratio",
            "System swap memory used ratio (0.0-1.0)",
        )?;

        // Disk Device-Level Metrics (5 new metrics)
        let disk_reads_completed_total = GaugeVec::new(
            Opts::new(
                "herakles_system_disk_reads_completed_total",
                "Total number of read operations completed per disk device",
            ),
            &["device"],
        )?;
        let disk_read_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_system_disk_read_bytes_total",
                "Total bytes read from disk device",
            ),
            &["device"],
        )?;
        let disk_writes_completed_total = GaugeVec::new(
            Opts::new(
                "herakles_system_disk_writes_completed_total",
                "Total number of write operations completed per disk device",
            ),
            &["device"],
        )?;
        let disk_write_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_system_disk_write_bytes_total",
                "Total bytes written to disk device",
            ),
            &["device"],
        )?;
        let disk_io_now = GaugeVec::new(
            Opts::new(
                "herakles_system_disk_io_now",
                "Number of I/O operations currently in progress for disk device",
            ),
            &["device"],
        )?;

        // Thermal Metrics
        let system_cpu_temp_celsius = GaugeVec::new(
            Opts::new(
                "herakles_system_cpu_temp_celsius",
                "CPU/sensor temperature in Celsius",
            ),
            &["sensor"],
        )?;

        // Filesystem Metrics (4 new metrics)
        let filesystem_avail_bytes = GaugeVec::new(
            Opts::new(
                "herakles_filesystem_avail_bytes",
                "Available bytes in filesystem",
            ),
            &["device", "mountpoint", "fstype"],
        )?;
        let filesystem_size_bytes = GaugeVec::new(
            Opts::new(
                "herakles_filesystem_size_bytes",
                "Total size of filesystem in bytes",
            ),
            &["device", "mountpoint", "fstype"],
        )?;
        let filesystem_files = GaugeVec::new(
            Opts::new(
                "herakles_filesystem_files",
                "Total number of inodes in filesystem",
            ),
            &["device", "mountpoint", "fstype"],
        )?;
        let filesystem_files_free = GaugeVec::new(
            Opts::new(
                "herakles_filesystem_files_free",
                "Number of free inodes in filesystem",
            ),
            &["device", "mountpoint", "fstype"],
        )?;

        // Network Device-Level Metrics (5 new metrics)
        let network_receive_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_system_net_rx_bytes_total",
                "Total bytes received per network device",
            ),
            &["device"],
        )?;
        let network_transmit_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_system_net_tx_bytes_total",
                "Total bytes transmitted per network device",
            ),
            &["device"],
        )?;
        let network_receive_packets_total = GaugeVec::new(
            Opts::new(
                "herakles_system_net_rx_packets_total",
                "Total packets received per network device",
            ),
            &["device"],
        )?;
        let network_receive_errs_total = GaugeVec::new(
            Opts::new(
                "herakles_system_net_rx_errors_total",
                "Total receive errors per network device",
            ),
            &["device"],
        )?;
        let network_receive_drop_total = GaugeVec::new(
            Opts::new(
                "herakles_system_net_rx_drop_total",
                "Total receive drops per network device",
            ),
            &["device"],
        )?;

        // eBPF Group Network Metrics (4 new metrics)
        let net_group_rx_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_group_net_rx_bytes_total",
                "Total bytes received per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;
        let net_group_tx_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_group_net_tx_bytes_total",
                "Total bytes transmitted per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;
        let net_group_packets_total = GaugeVec::new(
            Opts::new(
                "herakles_group_net_packets_total",
                "Total packets per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;
        let net_group_dropped_total = GaugeVec::new(
            Opts::new(
                "herakles_group_net_dropped_total",
                "Total dropped packets per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;

        // eBPF Group I/O Metrics (2 new metrics)
        let io_group_read_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_group_blkio_read_bytes_total",
                "Total bytes read per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;
        let io_group_write_bytes_total = GaugeVec::new(
            Opts::new(
                "herakles_group_blkio_write_bytes_total",
                "Total bytes written per group and subgroup (eBPF)",
            ),
            &["group", "subgroup"],
        )?;

        // Register all node-level metrics
        registry.register(Box::new(node_uptime_seconds.clone()))?;
        registry.register(Box::new(node_cpu_usage_percent.clone()))?;
        registry.register(Box::new(node_cpu_iowait_percent.clone()))?;
        registry.register(Box::new(node_cpu_steal_percent.clone()))?;
        registry.register(Box::new(node_mem_total_bytes.clone()))?;
        registry.register(Box::new(node_mem_used_bytes.clone()))?;
        registry.register(Box::new(node_mem_available_bytes.clone()))?;
        registry.register(Box::new(node_mem_cached_bytes.clone()))?;
        registry.register(Box::new(node_mem_buffers_bytes.clone()))?;
        registry.register(Box::new(node_mem_swap_used_bytes.clone()))?;
        registry.register(Box::new(node_mem_swap_total_bytes.clone()))?;
        registry.register(Box::new(node_io_read_bytes_per_second.clone()))?;
        registry.register(Box::new(node_io_write_bytes_per_second.clone()))?;
        registry.register(Box::new(node_io_read_iops_per_second.clone()))?;
        registry.register(Box::new(node_io_write_iops_per_second.clone()))?;
        registry.register(Box::new(node_net_rx_bytes_per_second.clone()))?;
        registry.register(Box::new(node_net_tx_bytes_per_second.clone()))?;
        registry.register(Box::new(node_net_rx_dropped_packets_per_second.clone()))?;
        registry.register(Box::new(node_net_tx_dropped_packets_per_second.clone()))?;
        registry.register(Box::new(node_net_rx_error_packets_per_second.clone()))?;
        registry.register(Box::new(node_net_tx_error_packets_per_second.clone()))?;
        registry.register(Box::new(node_fd_open.clone()))?;
        registry.register(Box::new(node_fd_kernel_max.clone()))?;
        registry.register(Box::new(node_fd_used_ratio.clone()))?;
        registry.register(Box::new(node_load1.clone()))?;
        registry.register(Box::new(node_load5.clone()))?;
        registry.register(Box::new(node_load15.clone()))?;

        // Register new system-level metrics
        registry.register(Box::new(system_uptime_seconds.clone()))?;
        registry.register(Box::new(system_boot_time_seconds.clone()))?;
        registry.register(Box::new(system_context_switches_total.clone()))?;
        registry.register(Box::new(system_forks_total.clone()))?;
        registry.register(Box::new(system_open_fds.clone()))?;
        registry.register(Box::new(system_entropy_bits.clone()))?;
        registry.register(Box::new(system_cpu_psi_wait_seconds_total.clone()))?;
        registry.register(Box::new(system_memory_psi_wait_seconds_total.clone()))?;
        registry.register(Box::new(system_disk_psi_wait_seconds_total.clone()))?;

        // Register subgroup metadata metrics
        registry.register(Box::new(subgroup_info.clone()))?;
        registry.register(Box::new(subgroup_oldest_uptime_seconds.clone()))?;
        registry.register(Box::new(subgroup_alert_armed.clone()))?;

        // Register subgroup-level aggregated metrics
        registry.register(Box::new(mem_rss_subgroup_bytes.clone()))?;
        registry.register(Box::new(mem_pss_subgroup_bytes.clone()))?;
        registry.register(Box::new(mem_uss_subgroup_bytes.clone()))?;
        registry.register(Box::new(mem_swap_subgroup_bytes.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_percent.clone()))?;
        registry.register(Box::new(cpu_iowait_subgroup_percent.clone()))?;
        registry.register(Box::new(io_read_subgroup_bytes_per_second.clone()))?;
        registry.register(Box::new(io_write_subgroup_bytes_per_second.clone()))?;
        registry.register(Box::new(net_rx_subgroup_bytes_per_second.clone()))?;
        registry.register(Box::new(net_tx_subgroup_bytes_per_second.clone()))?;

        // Register Top-3 RSS Memory metrics
        registry.register(Box::new(mem_rss_subgroup_top1_bytes.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top2_bytes.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top3_bytes.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top1_comm.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top2_comm.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top3_comm.clone()))?;

        // Register Top-3 CPU Usage metrics
        registry.register(Box::new(cpu_usage_subgroup_top1_percent.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top2_percent.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top3_percent.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top1_comm.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top2_comm.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top3_comm.clone()))?;

        // Register Group Core metrics
        registry.register(Box::new(mem_group_rss_bytes_sum.clone()))?;
        registry.register(Box::new(mem_group_pss_bytes_sum.clone()))?;
        registry.register(Box::new(mem_group_uss_bytes_sum.clone()))?;
        registry.register(Box::new(cpu_group_usage_percent_sum.clone()))?;
        registry.register(Box::new(cpu_group_time_seconds_sum.clone()))?;
        registry.register(Box::new(cpu_group_uptime_oldest_process_seconds.clone()))?;

        // Register System Ratios
        registry.register(Box::new(cpu_system_usage_ratio.clone()))?;
        registry.register(Box::new(cpu_system_idle_ratio.clone()))?;
        registry.register(Box::new(cpu_system_iowait_ratio.clone()))?;
        registry.register(Box::new(cpu_system_steal_ratio.clone()))?;
        registry.register(Box::new(mem_system_used_ratio.clone()))?;
        registry.register(Box::new(mem_system_swap_used_ratio.clone()))?;

        // Register Disk Device-Level metrics
        registry.register(Box::new(disk_reads_completed_total.clone()))?;
        registry.register(Box::new(disk_read_bytes_total.clone()))?;
        registry.register(Box::new(disk_writes_completed_total.clone()))?;
        registry.register(Box::new(disk_write_bytes_total.clone()))?;
        registry.register(Box::new(disk_io_now.clone()))?;

        // Register Thermal metrics
        registry.register(Box::new(system_cpu_temp_celsius.clone()))?;

        // Register Filesystem metrics
        registry.register(Box::new(filesystem_avail_bytes.clone()))?;
        registry.register(Box::new(filesystem_size_bytes.clone()))?;
        registry.register(Box::new(filesystem_files.clone()))?;
        registry.register(Box::new(filesystem_files_free.clone()))?;

        // Register Network Device-Level metrics
        registry.register(Box::new(network_receive_bytes_total.clone()))?;
        registry.register(Box::new(network_transmit_bytes_total.clone()))?;
        registry.register(Box::new(network_receive_packets_total.clone()))?;
        registry.register(Box::new(network_receive_errs_total.clone()))?;
        registry.register(Box::new(network_receive_drop_total.clone()))?;

        // Register eBPF Group Network metrics
        registry.register(Box::new(net_group_rx_bytes_total.clone()))?;
        registry.register(Box::new(net_group_tx_bytes_total.clone()))?;
        registry.register(Box::new(net_group_packets_total.clone()))?;
        registry.register(Box::new(net_group_dropped_total.clone()))?;

        // Register eBPF Group I/O metrics
        registry.register(Box::new(io_group_read_bytes_total.clone()))?;
        registry.register(Box::new(io_group_write_bytes_total.clone()))?;

        Ok(Self {
            node_uptime_seconds,
            node_cpu_usage_percent,
            node_cpu_iowait_percent,
            node_cpu_steal_percent,
            node_mem_total_bytes,
            node_mem_used_bytes,
            node_mem_available_bytes,
            node_mem_cached_bytes,
            node_mem_buffers_bytes,
            node_mem_swap_used_bytes,
            node_mem_swap_total_bytes,
            node_io_read_bytes_per_second,
            node_io_write_bytes_per_second,
            node_io_read_iops_per_second,
            node_io_write_iops_per_second,
            node_net_rx_bytes_per_second,
            node_net_tx_bytes_per_second,
            node_net_rx_dropped_packets_per_second,
            node_net_tx_dropped_packets_per_second,
            node_net_rx_error_packets_per_second,
            node_net_tx_error_packets_per_second,
            node_fd_open,
            node_fd_kernel_max,
            node_fd_used_ratio,
            node_load1,
            node_load5,
            node_load15,
            system_uptime_seconds,
            system_boot_time_seconds,
            system_context_switches_total,
            system_forks_total,
            system_open_fds,
            system_entropy_bits,
            system_cpu_psi_wait_seconds_total,
            system_memory_psi_wait_seconds_total,
            system_disk_psi_wait_seconds_total,
            subgroup_info,
            subgroup_oldest_uptime_seconds,
            subgroup_alert_armed,
            mem_rss_subgroup_bytes,
            mem_pss_subgroup_bytes,
            mem_uss_subgroup_bytes,
            mem_swap_subgroup_bytes,
            cpu_usage_subgroup_percent,
            cpu_iowait_subgroup_percent,
            io_read_subgroup_bytes_per_second,
            io_write_subgroup_bytes_per_second,
            net_rx_subgroup_bytes_per_second,
            net_tx_subgroup_bytes_per_second,
            mem_rss_subgroup_top1_bytes,
            mem_rss_subgroup_top2_bytes,
            mem_rss_subgroup_top3_bytes,
            mem_rss_subgroup_top1_comm,
            mem_rss_subgroup_top2_comm,
            mem_rss_subgroup_top3_comm,
            cpu_usage_subgroup_top1_percent,
            cpu_usage_subgroup_top2_percent,
            cpu_usage_subgroup_top3_percent,
            cpu_usage_subgroup_top1_comm,
            cpu_usage_subgroup_top2_comm,
            cpu_usage_subgroup_top3_comm,
            group_memory_rss_bytes_sum: mem_group_rss_bytes_sum,
            group_memory_pss_bytes_sum: mem_group_pss_bytes_sum,
            group_memory_uss_bytes_sum: mem_group_uss_bytes_sum,
            group_cpu_usage_percent_sum: cpu_group_usage_percent_sum,
            group_cpu_time_seconds_sum: cpu_group_time_seconds_sum,
            group_cpu_uptime_oldest_process_seconds: cpu_group_uptime_oldest_process_seconds,
            system_cpu_usage_ratio: cpu_system_usage_ratio,
            system_cpu_idle_ratio: cpu_system_idle_ratio,
            system_cpu_iowait_ratio: cpu_system_iowait_ratio,
            system_cpu_steal_ratio: cpu_system_steal_ratio,
            system_memory_used_ratio: mem_system_used_ratio,
            system_memory_swap_used_ratio: mem_system_swap_used_ratio,
            system_disk_reads_completed_total: disk_reads_completed_total,
            system_disk_read_bytes_total: disk_read_bytes_total,
            system_disk_writes_completed_total: disk_writes_completed_total,
            system_disk_write_bytes_total: disk_write_bytes_total,
            system_disk_io_now: disk_io_now,
            system_cpu_temp_celsius,
            filesystem_avail_bytes,
            filesystem_size_bytes,
            filesystem_files,
            filesystem_files_free,
            system_net_receive_bytes_total: network_receive_bytes_total,
            system_net_transmit_bytes_total: network_transmit_bytes_total,
            system_net_receive_packets_total: network_receive_packets_total,
            system_net_receive_errs_total: network_receive_errs_total,
            system_net_receive_drop_total: network_receive_drop_total,
            group_net_rx_bytes_total: net_group_rx_bytes_total,
            group_net_tx_bytes_total: net_group_tx_bytes_total,
            group_net_packets_total: net_group_packets_total,
            group_net_dropped_total: net_group_dropped_total,
            group_blkio_read_bytes_total: io_group_read_bytes_total,
            group_blkio_write_bytes_total: io_group_write_bytes_total,
        })
    }

    /// Resets all metrics to zero (used before updating with fresh data).
    pub fn reset(&self) {
        // Reset subgroup metadata metrics
        self.subgroup_info.reset();
        self.subgroup_oldest_uptime_seconds.reset();
        self.subgroup_alert_armed.reset();

        // Reset subgroup-level aggregated metrics
        self.mem_rss_subgroup_bytes.reset();
        self.mem_pss_subgroup_bytes.reset();
        self.mem_uss_subgroup_bytes.reset();
        self.mem_swap_subgroup_bytes.reset();
        self.cpu_usage_subgroup_percent.reset();
        self.cpu_iowait_subgroup_percent.reset();
        self.io_read_subgroup_bytes_per_second.reset();
        self.io_write_subgroup_bytes_per_second.reset();
        self.net_rx_subgroup_bytes_per_second.reset();
        self.net_tx_subgroup_bytes_per_second.reset();

        // Reset Top-3 RSS Memory metrics
        self.mem_rss_subgroup_top1_bytes.reset();
        self.mem_rss_subgroup_top2_bytes.reset();
        self.mem_rss_subgroup_top3_bytes.reset();
        self.mem_rss_subgroup_top1_comm.reset();
        self.mem_rss_subgroup_top2_comm.reset();
        self.mem_rss_subgroup_top3_comm.reset();

        // Reset Top-3 CPU Usage metrics
        self.cpu_usage_subgroup_top1_percent.reset();
        self.cpu_usage_subgroup_top2_percent.reset();
        self.cpu_usage_subgroup_top3_percent.reset();
        self.cpu_usage_subgroup_top1_comm.reset();
        self.cpu_usage_subgroup_top2_comm.reset();
        self.cpu_usage_subgroup_top3_comm.reset();

        // Reset Group Core metrics
        self.group_memory_rss_bytes_sum.reset();
        self.group_memory_pss_bytes_sum.reset();
        self.group_memory_uss_bytes_sum.reset();
        self.group_cpu_usage_percent_sum.reset();
        self.group_cpu_time_seconds_sum.reset();
        self.group_cpu_uptime_oldest_process_seconds.reset();

        // Note: System Ratio metrics (system_cpu_*, system_memory_*) are single Gauges
        // (not GaugeVec) and are set fresh on every scrape, so no reset needed

        // Reset Disk Device-Level metrics
        self.system_disk_reads_completed_total.reset();
        self.system_disk_read_bytes_total.reset();
        self.system_disk_writes_completed_total.reset();
        self.system_disk_write_bytes_total.reset();
        self.system_disk_io_now.reset();

        // Reset Thermal metrics
        self.system_cpu_temp_celsius.reset();

        // Reset Filesystem metrics
        self.filesystem_avail_bytes.reset();
        self.filesystem_size_bytes.reset();
        self.filesystem_files.reset();
        self.filesystem_files_free.reset();

        // Reset Network Device-Level metrics
        self.system_net_receive_bytes_total.reset();
        self.system_net_transmit_bytes_total.reset();
        self.system_net_receive_packets_total.reset();
        self.system_net_receive_errs_total.reset();
        self.system_net_receive_drop_total.reset();

        // Reset eBPF Group Network metrics
        self.group_net_rx_bytes_total.reset();
        self.group_net_tx_bytes_total.reset();
        self.group_net_packets_total.reset();
        self.group_net_dropped_total.reset();

        // Reset eBPF Group I/O metrics
        self.group_blkio_read_bytes_total.reset();
        self.group_blkio_write_bytes_total.reset();
    }
}
