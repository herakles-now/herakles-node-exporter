//! Prometheus metrics definitions for herakles-proc-mem-exporter.
//!
//! This module defines all the Prometheus metrics used to export process
//! memory and CPU usage information.

use crate::config::Config;
use prometheus::{Gauge, GaugeVec, Opts, Registry};

/// Collection of Prometheus metrics for memory and CPU monitoring.
#[derive(Clone)]
pub struct MemoryMetrics {
    pub rss: GaugeVec,
    pub pss: GaugeVec,
    pub uss: GaugeVec,
    pub cpu_usage: GaugeVec,
    pub cpu_time: GaugeVec,

    // Aggregated per-subgroup sums
    pub agg_rss_sum: GaugeVec,
    pub agg_pss_sum: GaugeVec,
    pub agg_uss_sum: GaugeVec,
    pub agg_cpu_percent_sum: GaugeVec,
    pub agg_cpu_time_sum: GaugeVec,

    // Top-N metrics per subgroup
    pub top_rss: GaugeVec,
    pub top_pss: GaugeVec,
    pub top_uss: GaugeVec,
    pub top_cpu_percent: GaugeVec,
    pub top_cpu_time: GaugeVec,

    // Percentage-of-subgroup metrics for Top-N
    pub top_cpu_percent_of_subgroup: GaugeVec,
    pub top_rss_percent_of_subgroup: GaugeVec,
    pub top_pss_percent_of_subgroup: GaugeVec,
    pub top_uss_percent_of_subgroup: GaugeVec,

    // System-wide metrics
    pub system_memory_total_bytes: Gauge,
    pub system_memory_available_bytes: Gauge,
    pub system_memory_used_ratio: Gauge,
    pub system_memory_cached_bytes: Gauge,
    pub system_memory_buffers_bytes: Gauge,
    pub system_memory_swap_used_ratio: Gauge,
    pub system_cpu_usage_ratio: GaugeVec,
    pub system_cpu_idle_ratio: GaugeVec,
    pub system_cpu_iowait_ratio: GaugeVec,
    pub system_cpu_steal_ratio: GaugeVec,
    pub system_load1: Gauge,
    pub system_load5: Gauge,
    pub system_load15: Gauge,
    pub system_cpu_psi_wait_seconds_total: Gauge,
    pub system_memory_psi_wait_seconds_total: Gauge,
    
    // CPU group metrics
    pub cpu_group_usage_ratio: GaugeVec,
    pub cpu_group_seconds_total: GaugeVec,
    
    // CPU top process metrics
    pub cpu_top_process_usage_ratio: GaugeVec,
    pub cpu_top_process_seconds_total: GaugeVec,
    
    // Memory group swap metrics
    pub mem_group_swap_bytes: GaugeVec,
}

impl MemoryMetrics {
    /// Creates and registers all Prometheus metrics with the registry.
    pub fn new(registry: &Registry) -> Result<Self, Box<dyn std::error::Error>> {
        let labels = &["pid", "name", "group", "subgroup", "uptime_in_seconds"];

        let rss = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_rss_bytes",
                "Resident Set Size per process in bytes",
            ),
            labels,
        )?;
        let pss = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_pss_bytes",
                "Proportional Set Size per process in bytes",
            ),
            labels,
        )?;
        let uss = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_uss_bytes",
                "Unique Set Size per process in bytes",
            ),
            labels,
        )?;
        let cpu_usage = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_cpu_percent",
                "CPU usage per process in percent (delta over last scan)",
            ),
            labels,
        )?;
        let cpu_time = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_cpu_time_seconds",
                "Total CPU time used per process",
            ),
            labels,
        )?;

        // Aggregated sums per subgroup (renamed metrics)
        let agg_rss_sum = GaugeVec::new(
            Opts::new(
                "herakles_mem_group_rss_bytes",
                "Sum of RSS bytes per subgroup",
            ),
            &["group", "subgroup", "uptime_in_seconds"],
        )?;
        let agg_pss_sum = GaugeVec::new(
            Opts::new(
                "herakles_mem_group_pss_bytes",
                "Sum of PSS bytes per subgroup",
            ),
            &["group", "subgroup", "uptime_in_seconds"],
        )?;
        let agg_uss_sum = GaugeVec::new(
            Opts::new(
                "herakles_mem_group_uss_bytes",
                "Sum of USS bytes per subgroup",
            ),
            &["group", "subgroup", "uptime_in_seconds"],
        )?;
        let agg_cpu_percent_sum = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_group_cpu_percent_sum",
                "Sum of CPU percent per subgroup",
            ),
            &["group", "subgroup", "uptime_in_seconds"],
        )?;
        let agg_cpu_time_sum = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_group_cpu_time_seconds_sum",
                "Sum of CPU time seconds per subgroup",
            ),
            &["group", "subgroup", "uptime_in_seconds"],
        )?;

        // Top-N metrics per subgroup (renamed with "comm" label instead of "name")
        let top_rss = GaugeVec::new(
            Opts::new("herakles_mem_top_process_rss_bytes", "Top-N RSS per subgroup"),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;
        let top_pss = GaugeVec::new(
            Opts::new("herakles_mem_top_process_pss_bytes", "Top-N PSS per subgroup"),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;
        let top_uss = GaugeVec::new(
            Opts::new("herakles_mem_top_process_uss_bytes", "Top-N USS per subgroup"),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;
        let top_cpu_percent = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_top_cpu_percent",
                "Top-N CPU percent per subgroup",
            ),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;
        let top_cpu_time = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_top_cpu_time_seconds",
                "Top-N CPU time seconds per subgroup",
            ),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;

        // Percentage-of-subgroup metrics (use "comm" instead of "name")
        let top_cpu_percent_of_subgroup = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_top_cpu_percent_of_subgroup",
                "Top-N CPU time as percentage of subgroup total CPU time",
            ),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;
        let top_rss_percent_of_subgroup = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_top_rss_percent_of_subgroup",
                "Top-N RSS as percentage of subgroup total RSS",
            ),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;
        let top_pss_percent_of_subgroup = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_top_pss_percent_of_subgroup",
                "Top-N PSS as percentage of subgroup total PSS",
            ),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;
        let top_uss_percent_of_subgroup = GaugeVec::new(
            Opts::new(
                "herakles_proc_mem_top_uss_percent_of_subgroup",
                "Top-N USS as percentage of subgroup total USS",
            ),
            &[
                "group",
                "subgroup",
                "rank",
                "pid",
                "comm",
                "uptime_in_seconds",
            ],
        )?;

        // System-wide memory metrics (renamed)
        let system_memory_total_bytes = Gauge::new(
            "herakles_mem_system_total_bytes",
            "Total system memory in bytes (MemTotal from /proc/meminfo)",
        )?;
        let system_memory_available_bytes = Gauge::new(
            "herakles_mem_system_available_bytes",
            "Available system memory in bytes (MemAvailable from /proc/meminfo)",
        )?;
        let system_memory_used_ratio = Gauge::new(
            "herakles_mem_system_used_ratio",
            "Memory used ratio: 1 - (available_bytes / total_bytes), value between 0.0 and 1.0",
        )?;
        let system_memory_cached_bytes = Gauge::new(
            "herakles_mem_system_cached_bytes",
            "Page cache memory in bytes (Cached from /proc/meminfo)",
        )?;
        let system_memory_buffers_bytes = Gauge::new(
            "herakles_mem_system_buffers_bytes",
            "Buffer cache memory in bytes (Buffers from /proc/meminfo)",
        )?;
        let system_memory_swap_used_ratio = Gauge::new(
            "herakles_mem_system_swap_used_ratio",
            "Swap used ratio: (SwapTotal - SwapFree) / SwapTotal, value between 0.0 and 1.0",
        )?;
        let system_memory_psi_wait_seconds_total = Gauge::new(
            "herakles_mem_system_psi_wait_seconds_total",
            "Memory Pressure Stall Information (PSI) - some total seconds from /proc/pressure/memory",
        )?;
        
        // System-wide CPU metrics (renamed)
        let system_cpu_usage_ratio = GaugeVec::new(
            Opts::new(
                "herakles_cpu_system_usage_ratio",
                "CPU usage ratio per core and total, calculated from /proc/stat deltas",
            ),
            &["cpu"],
        )?;
        let system_cpu_idle_ratio = GaugeVec::new(
            Opts::new(
                "herakles_cpu_system_idle_ratio",
                "CPU idle ratio per core and total (0.0 to 1.0) from /proc/stat",
            ),
            &["cpu"],
        )?;
        let system_cpu_iowait_ratio = GaugeVec::new(
            Opts::new(
                "herakles_cpu_system_iowait_ratio",
                "CPU IO-wait ratio per core and total (0.0 to 1.0) from /proc/stat",
            ),
            &["cpu"],
        )?;
        let system_cpu_steal_ratio = GaugeVec::new(
            Opts::new(
                "herakles_cpu_system_steal_ratio",
                "CPU steal time ratio per core and total (0.0 to 1.0) from /proc/stat",
            ),
            &["cpu"],
        )?;
        let system_load1 = Gauge::new(
            "herakles_cpu_system_load_1",
            "System load average over 1 minute",
        )?;
        let system_load5 = Gauge::new(
            "herakles_cpu_system_load_5",
            "System load average over 5 minutes",
        )?;
        let system_load15 = Gauge::new(
            "herakles_cpu_system_load_15",
            "System load average over 15 minutes",
        )?;
        let system_cpu_psi_wait_seconds_total = Gauge::new(
            "herakles_cpu_system_psi_wait_seconds_total",
            "CPU Pressure Stall Information (PSI) - some total seconds from /proc/pressure/cpu",
        )?;
        
        // CPU group metrics
        let cpu_group_usage_ratio = GaugeVec::new(
            Opts::new(
                "herakles_cpu_group_usage_ratio",
                "CPU usage ratio per subgroup (0.0 to 1.0)",
            ),
            &["group", "subgroup"],
        )?;
        let cpu_group_seconds_total = GaugeVec::new(
            Opts::new(
                "herakles_cpu_group_seconds_total",
                "Total CPU time seconds per subgroup",
            ),
            &["group", "subgroup", "mode"],
        )?;
        
        // CPU top process metrics (with comm label)
        let cpu_top_process_usage_ratio = GaugeVec::new(
            Opts::new(
                "herakles_cpu_top_process_usage_ratio",
                "Top-3 CPU usage ratio per subgroup (0.0 to 1.0)",
            ),
            &["group", "subgroup", "rank", "pid", "comm"],
        )?;
        let cpu_top_process_seconds_total = GaugeVec::new(
            Opts::new(
                "herakles_cpu_top_process_seconds_total",
                "Top-3 CPU time seconds per subgroup",
            ),
            &["group", "subgroup", "rank", "pid", "comm", "mode"],
        )?;
        
        // Memory group swap metrics
        let mem_group_swap_bytes = GaugeVec::new(
            Opts::new(
                "herakles_mem_group_swap_bytes",
                "Swap usage in bytes per subgroup from /proc/[pid]/status VmSwap",
            ),
            &["group", "subgroup"],
        )?;

        registry.register(Box::new(rss.clone()))?;
        registry.register(Box::new(pss.clone()))?;
        registry.register(Box::new(uss.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;
        registry.register(Box::new(cpu_time.clone()))?;

        registry.register(Box::new(agg_rss_sum.clone()))?;
        registry.register(Box::new(agg_pss_sum.clone()))?;
        registry.register(Box::new(agg_uss_sum.clone()))?;
        registry.register(Box::new(agg_cpu_percent_sum.clone()))?;
        registry.register(Box::new(agg_cpu_time_sum.clone()))?;

        registry.register(Box::new(top_rss.clone()))?;
        registry.register(Box::new(top_pss.clone()))?;
        registry.register(Box::new(top_uss.clone()))?;
        registry.register(Box::new(top_cpu_percent.clone()))?;
        registry.register(Box::new(top_cpu_time.clone()))?;

        registry.register(Box::new(top_cpu_percent_of_subgroup.clone()))?;
        registry.register(Box::new(top_rss_percent_of_subgroup.clone()))?;
        registry.register(Box::new(top_pss_percent_of_subgroup.clone()))?;
        registry.register(Box::new(top_uss_percent_of_subgroup.clone()))?;

        registry.register(Box::new(system_memory_total_bytes.clone()))?;
        registry.register(Box::new(system_memory_available_bytes.clone()))?;
        registry.register(Box::new(system_memory_used_ratio.clone()))?;
        registry.register(Box::new(system_memory_cached_bytes.clone()))?;
        registry.register(Box::new(system_memory_buffers_bytes.clone()))?;
        registry.register(Box::new(system_memory_swap_used_ratio.clone()))?;
        registry.register(Box::new(system_memory_psi_wait_seconds_total.clone()))?;
        registry.register(Box::new(system_cpu_usage_ratio.clone()))?;
        registry.register(Box::new(system_cpu_idle_ratio.clone()))?;
        registry.register(Box::new(system_cpu_iowait_ratio.clone()))?;
        registry.register(Box::new(system_cpu_steal_ratio.clone()))?;
        registry.register(Box::new(system_load1.clone()))?;
        registry.register(Box::new(system_load5.clone()))?;
        registry.register(Box::new(system_load15.clone()))?;
        registry.register(Box::new(system_cpu_psi_wait_seconds_total.clone()))?;
        registry.register(Box::new(cpu_group_usage_ratio.clone()))?;
        registry.register(Box::new(cpu_group_seconds_total.clone()))?;
        registry.register(Box::new(cpu_top_process_usage_ratio.clone()))?;
        registry.register(Box::new(cpu_top_process_seconds_total.clone()))?;
        registry.register(Box::new(mem_group_swap_bytes.clone()))?;

        Ok(Self {
            rss,
            pss,
            uss,
            cpu_usage,
            cpu_time,
            agg_rss_sum,
            agg_pss_sum,
            agg_uss_sum,
            agg_cpu_percent_sum,
            agg_cpu_time_sum,
            top_rss,
            top_pss,
            top_uss,
            top_cpu_percent,
            top_cpu_time,
            top_cpu_percent_of_subgroup,
            top_rss_percent_of_subgroup,
            top_pss_percent_of_subgroup,
            top_uss_percent_of_subgroup,
            system_memory_total_bytes,
            system_memory_available_bytes,
            system_memory_used_ratio,
            system_memory_cached_bytes,
            system_memory_buffers_bytes,
            system_memory_swap_used_ratio,
            system_memory_psi_wait_seconds_total,
            system_cpu_usage_ratio,
            system_cpu_idle_ratio,
            system_cpu_iowait_ratio,
            system_cpu_steal_ratio,
            system_load1,
            system_load5,
            system_load15,
            system_cpu_psi_wait_seconds_total,
            cpu_group_usage_ratio,
            cpu_group_seconds_total,
            cpu_top_process_usage_ratio,
            cpu_top_process_seconds_total,
            mem_group_swap_bytes,
        })
    }

    /// Resets all metrics to zero (used before updating with fresh data).
    pub fn reset(&self) {
        self.rss.reset();
        self.pss.reset();
        self.uss.reset();
        self.cpu_usage.reset();
        self.cpu_time.reset();

        self.agg_rss_sum.reset();
        self.agg_pss_sum.reset();
        self.agg_uss_sum.reset();
        self.agg_cpu_percent_sum.reset();
        self.agg_cpu_time_sum.reset();

        self.top_rss.reset();
        self.top_pss.reset();
        self.top_uss.reset();
        self.top_cpu_percent.reset();
        self.top_cpu_time.reset();

        self.top_cpu_percent_of_subgroup.reset();
        self.top_rss_percent_of_subgroup.reset();
        self.top_pss_percent_of_subgroup.reset();
        self.top_uss_percent_of_subgroup.reset();

        // Reset system metrics
        self.system_cpu_usage_ratio.reset();
        self.system_cpu_idle_ratio.reset();
        self.system_cpu_iowait_ratio.reset();
        self.system_cpu_steal_ratio.reset();
        
        // Reset group and top process metrics
        self.cpu_group_usage_ratio.reset();
        self.cpu_group_seconds_total.reset();
        self.cpu_top_process_usage_ratio.reset();
        self.cpu_top_process_seconds_total.reset();
        self.mem_group_swap_bytes.reset();
    }

    /// Sets system memory metrics (total, available, used ratio, cached, buffers, swap).
    pub fn set_system_memory_metrics(&self, total_bytes: u64, available_bytes: u64, cached_bytes: u64, buffers_bytes: u64, swap_total_bytes: u64, swap_free_bytes: u64) {
        self.system_memory_total_bytes.set(total_bytes as f64);
        self.system_memory_available_bytes.set(available_bytes as f64);
        self.system_memory_cached_bytes.set(cached_bytes as f64);
        self.system_memory_buffers_bytes.set(buffers_bytes as f64);
        
        // Calculate used ratio: 1 - (available / total)
        if total_bytes > 0 {
            let used_ratio = 1.0 - (available_bytes as f64 / total_bytes as f64);
            self.system_memory_used_ratio.set(used_ratio);
        } else {
            self.system_memory_used_ratio.set(0.0);
        }
        
        // Calculate swap used ratio: (total - free) / total
        if swap_total_bytes > 0 {
            let swap_used_ratio = (swap_total_bytes - swap_free_bytes) as f64 / swap_total_bytes as f64;
            self.system_memory_swap_used_ratio.set(swap_used_ratio);
        } else {
            self.system_memory_swap_used_ratio.set(0.0);
        }
    }

    /// Sets CPU usage ratio metrics for each CPU core and total.
    pub fn set_system_cpu_usage_ratios(&self, cpu_ratios: &crate::system::CpuRatios) {
        for (cpu_name, ratio) in &cpu_ratios.usage {
            self.system_cpu_usage_ratio
                .with_label_values(&[cpu_name])
                .set(*ratio);
        }
        for (cpu_name, ratio) in &cpu_ratios.idle {
            self.system_cpu_idle_ratio
                .with_label_values(&[cpu_name])
                .set(*ratio);
        }
        for (cpu_name, ratio) in &cpu_ratios.iowait {
            self.system_cpu_iowait_ratio
                .with_label_values(&[cpu_name])
                .set(*ratio);
        }
        for (cpu_name, ratio) in &cpu_ratios.steal {
            self.system_cpu_steal_ratio
                .with_label_values(&[cpu_name])
                .set(*ratio);
        }
    }

    /// Sets load average metrics with the new metric names.
    pub fn set_system_load_metrics(&self, load_1min: f64, load_5min: f64, load_15min: f64) {
        self.system_load1.set(load_1min);
        self.system_load5.set(load_5min);
        self.system_load15.set(load_15min);
    }

    /// Sets PSI metrics (Pressure Stall Information).
    pub fn set_psi_metrics(&self, cpu_psi_total: f64, memory_psi_total: f64) {
        self.system_cpu_psi_wait_seconds_total.set(cpu_psi_total);
        self.system_memory_psi_wait_seconds_total.set(memory_psi_total);
    }

    /// Sets metric values for a specific process with classification.
    #[allow(clippy::too_many_arguments)]
    pub fn set_for_process(
        &self,
        pid: &str,
        name: &str,
        group: &str,
        subgroup: &str,
        rss: u64,
        pss: u64,
        uss: u64,
        cpu_percent: f64,
        cpu_time_seconds: f64,
        cfg: &Config,
        uptime_in_seconds: &str,
    ) {
        let labels = &[pid, name, group, subgroup, uptime_in_seconds];

        let enable_rss = cfg.enable_rss.unwrap_or(true);
        let enable_pss = cfg.enable_pss.unwrap_or(true);
        let enable_uss = cfg.enable_uss.unwrap_or(true);
        let enable_cpu = cfg.enable_cpu.unwrap_or(true);

        if enable_rss {
            self.rss.with_label_values(labels).set(rss as f64);
        }
        if enable_pss {
            self.pss.with_label_values(labels).set(pss as f64);
        }
        if enable_uss {
            self.uss.with_label_values(labels).set(uss as f64);
        }
        if enable_cpu {
            self.cpu_usage.with_label_values(labels).set(cpu_percent);
            self.cpu_time
                .with_label_values(labels)
                .set(cpu_time_seconds);
        }
    }
}
