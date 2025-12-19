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

    // Top-3 RSS Memory metrics (9 metrics) - Labels: group, subgroup (and comm for _comm metrics)
    pub mem_rss_subgroup_top1_bytes: GaugeVec,
    pub mem_rss_subgroup_top2_bytes: GaugeVec,
    pub mem_rss_subgroup_top3_bytes: GaugeVec,
    pub mem_rss_subgroup_top1_pid: GaugeVec,
    pub mem_rss_subgroup_top2_pid: GaugeVec,
    pub mem_rss_subgroup_top3_pid: GaugeVec,
    pub mem_rss_subgroup_top1_comm: GaugeVec,  // Labels: group, subgroup, comm
    pub mem_rss_subgroup_top2_comm: GaugeVec,  // Labels: group, subgroup, comm
    pub mem_rss_subgroup_top3_comm: GaugeVec,  // Labels: group, subgroup, comm

    // Top-3 CPU Usage metrics (9 metrics) - Labels: group, subgroup (and comm for _comm metrics)
    pub cpu_usage_subgroup_top1_percent: GaugeVec,
    pub cpu_usage_subgroup_top2_percent: GaugeVec,
    pub cpu_usage_subgroup_top3_percent: GaugeVec,
    pub cpu_usage_subgroup_top1_pid: GaugeVec,
    pub cpu_usage_subgroup_top2_pid: GaugeVec,
    pub cpu_usage_subgroup_top3_pid: GaugeVec,
    pub cpu_usage_subgroup_top1_comm: GaugeVec,  // Labels: group, subgroup, comm
    pub cpu_usage_subgroup_top2_comm: GaugeVec,  // Labels: group, subgroup, comm
    pub cpu_usage_subgroup_top3_comm: GaugeVec,  // Labels: group, subgroup, comm
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
        let node_load1 = Gauge::new(
            "herakles_node_load1",
            "System load average over 1 minute",
        )?;
        let node_load5 = Gauge::new(
            "herakles_node_load5",
            "System load average over 5 minutes",
        )?;
        let node_load15 = Gauge::new(
            "herakles_node_load15",
            "System load average over 15 minutes",
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
        let mem_rss_subgroup_top1_pid = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top1_pid",
                "Top 1 RSS process PID per subgroup",
            ),
            &["subgroup"],
        )?;
        let mem_rss_subgroup_top2_pid = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top2_pid",
                "Top 2 RSS process PID per subgroup",
            ),
            &["subgroup"],
        )?;
        let mem_rss_subgroup_top3_pid = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top3_pid",
                "Top 3 RSS process PID per subgroup",
            ),
            &["subgroup"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let mem_rss_subgroup_top1_comm = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top1_comm",
                "Top 1 RSS process name per subgroup (info metric)",
            ),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let mem_rss_subgroup_top2_comm = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top2_comm",
                "Top 2 RSS process name per subgroup (info metric)",
            ),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let mem_rss_subgroup_top3_comm = GaugeVec::new(
            Opts::new(
                "herakles_mem_rss_subgroup_top3_comm",
                "Top 3 RSS process name per subgroup (info metric)",
            ),
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
        let cpu_usage_subgroup_top1_pid = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top1_pid",
                "Top 1 CPU usage process PID per subgroup",
            ),
            &["subgroup"],
        )?;
        let cpu_usage_subgroup_top2_pid = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top2_pid",
                "Top 2 CPU usage process PID per subgroup",
            ),
            &["subgroup"],
        )?;
        let cpu_usage_subgroup_top3_pid = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top3_pid",
                "Top 3 CPU usage process PID per subgroup",
            ),
            &["subgroup"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let cpu_usage_subgroup_top1_comm = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top1_comm",
                "Top 1 CPU usage process name per subgroup (info metric)",
            ),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let cpu_usage_subgroup_top2_comm = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top2_comm",
                "Top 2 CPU usage process name per subgroup (info metric)",
            ),
            &["subgroup", "comm"],
        )?;
        // Info-style metric: value is always 1.0, actual data is in the 'comm' label
        let cpu_usage_subgroup_top3_comm = GaugeVec::new(
            Opts::new(
                "herakles_cpu_usage_subgroup_top3_comm",
                "Top 3 CPU usage process name per subgroup (info metric)",
            ),
            &["subgroup", "comm"],
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
        registry.register(Box::new(mem_rss_subgroup_top1_pid.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top2_pid.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top3_pid.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top1_comm.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top2_comm.clone()))?;
        registry.register(Box::new(mem_rss_subgroup_top3_comm.clone()))?;

        // Register Top-3 CPU Usage metrics
        registry.register(Box::new(cpu_usage_subgroup_top1_percent.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top2_percent.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top3_percent.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top1_pid.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top2_pid.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top3_pid.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top1_comm.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top2_comm.clone()))?;
        registry.register(Box::new(cpu_usage_subgroup_top3_comm.clone()))?;

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
            mem_rss_subgroup_top1_pid,
            mem_rss_subgroup_top2_pid,
            mem_rss_subgroup_top3_pid,
            mem_rss_subgroup_top1_comm,
            mem_rss_subgroup_top2_comm,
            mem_rss_subgroup_top3_comm,
            cpu_usage_subgroup_top1_percent,
            cpu_usage_subgroup_top2_percent,
            cpu_usage_subgroup_top3_percent,
            cpu_usage_subgroup_top1_pid,
            cpu_usage_subgroup_top2_pid,
            cpu_usage_subgroup_top3_pid,
            cpu_usage_subgroup_top1_comm,
            cpu_usage_subgroup_top2_comm,
            cpu_usage_subgroup_top3_comm,
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
        self.mem_rss_subgroup_top1_pid.reset();
        self.mem_rss_subgroup_top2_pid.reset();
        self.mem_rss_subgroup_top3_pid.reset();
        self.mem_rss_subgroup_top1_comm.reset();
        self.mem_rss_subgroup_top2_comm.reset();
        self.mem_rss_subgroup_top3_comm.reset();

        // Reset Top-3 CPU Usage metrics
        self.cpu_usage_subgroup_top1_percent.reset();
        self.cpu_usage_subgroup_top2_percent.reset();
        self.cpu_usage_subgroup_top3_percent.reset();
        self.cpu_usage_subgroup_top1_pid.reset();
        self.cpu_usage_subgroup_top2_pid.reset();
        self.cpu_usage_subgroup_top3_pid.reset();
        self.cpu_usage_subgroup_top1_comm.reset();
        self.cpu_usage_subgroup_top2_comm.reset();
        self.cpu_usage_subgroup_top3_comm.reset();
    }
}
