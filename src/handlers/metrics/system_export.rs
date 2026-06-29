//! System-level metric population for the Prometheus handler.

use tracing::warn;

use crate::collectors;
use crate::state::SharedState;
use crate::system;

pub(super) fn export_system_metrics(
    state: &SharedState,
    enable_filesystem_collector: bool,
    enable_thermal_collector: bool,
) {
    match state.system_cpu_cache.calculate_usage_ratios() {
        Ok(cpu_ratios) => {
            if let Some(&usage_ratio) = cpu_ratios.usage.get("cpu") {
                state.metrics.system_cpu_usage_ratio.set(usage_ratio);
            }
            if let Some(&idle_ratio) = cpu_ratios.idle.get("cpu") {
                state.metrics.system_cpu_idle_ratio.set(idle_ratio);
            }
            if let Some(&iowait_ratio) = cpu_ratios.iowait.get("cpu") {
                state.metrics.system_cpu_iowait_ratio.set(iowait_ratio);
            }
            if let Some(&steal_ratio) = cpu_ratios.steal.get("cpu") {
                state.metrics.system_cpu_steal_ratio.set(steal_ratio);
            }
        }
        Err(e) => warn!("Failed to calculate CPU ratios: {}", e),
    }

    match system::read_load_average() {
        Ok(load_avg) => {
            state.metrics.system_cpu_load_1.set(load_avg.one_min);
            state.metrics.system_cpu_load_5.set(load_avg.five_min);
            state.metrics.system_cpu_load_15.set(load_avg.fifteen_min);
        }
        Err(e) => warn!("Failed to read load average: {}", e),
    }

    match system::read_extended_memory_info() {
        Ok(mem_info) => {
            state
                .metrics
                .system_memory_total_bytes
                .set(mem_info.total_bytes as f64);
            state
                .metrics
                .system_memory_available_bytes
                .set(mem_info.available_bytes as f64);
            state
                .metrics
                .system_memory_cached_bytes
                .set(mem_info.cached_bytes as f64);
            state
                .metrics
                .system_memory_buffers_bytes
                .set(mem_info.buffers_bytes as f64);

            if mem_info.total_bytes > 0 {
                let mem_used_ratio = (mem_info.total_bytes - mem_info.available_bytes) as f64
                    / mem_info.total_bytes as f64;
                state.metrics.system_memory_used_ratio.set(mem_used_ratio);
            }

            if mem_info.swap_total_bytes > 0 {
                let swap_used_ratio = (mem_info.swap_total_bytes - mem_info.swap_free_bytes) as f64
                    / mem_info.swap_total_bytes as f64;
                state.metrics.system_swap_used_ratio.set(swap_used_ratio);
            } else {
                state.metrics.system_swap_used_ratio.set(0.0);
            }
        }
        Err(e) => warn!("Failed to read memory info: {}", e),
    }

    export_disk_metrics(state);
    export_network_metrics(state);

    if enable_filesystem_collector {
        export_filesystem_metrics(state);
    }

    if enable_thermal_collector {
        export_thermal_metrics(state);
    }

    match system::read_uptime() {
        Ok(uptime) => state.metrics.system_uptime_seconds.set(uptime),
        Err(e) => warn!("Failed to read system uptime: {}", e),
    }

    match system::read_stat_counters() {
        Ok((boot_time, context_switches, forks)) => {
            state.metrics.system_boot_time_seconds.set(boot_time as f64);
            state
                .metrics
                .system_context_switches_total
                .set(context_switches as f64);
            state.metrics.system_forks_total.set(forks as f64);
        }
        Err(e) => warn!("Failed to read stat counters: {}", e),
    }

    match system::read_uname_info() {
        Ok((sysname, release, version, machine)) => {
            state
                .metrics
                .system_uname_info
                .with_label_values(&[&sysname, &release, &version, &machine])
                .set(1.0);
        }
        Err(e) => warn!("Failed to read uname info: {}", e),
    }

    match system::read_system_fd_stats() {
        Ok((open_fds, _unused_fds, max_fds)) => {
            state
                .metrics
                .system_open_fds
                .with_label_values(&["allocated"])
                .set(open_fds as f64);
            state
                .metrics
                .system_open_fds
                .with_label_values(&["max"])
                .set(max_fds as f64);
        }
        Err(e) => warn!("Failed to read system FD stats: {}", e),
    }

    match system::read_entropy() {
        Ok(entropy) => state.metrics.system_entropy_bits.set(entropy as f64),
        Err(e) => warn!("Failed to read entropy: {}", e),
    }

    if let Ok(cpu_psi) = system::read_psi_some_total("/proc/pressure/cpu") {
        state.metrics.system_cpu_psi_wait_seconds_total.set(cpu_psi);
    }
    if let Ok(mem_psi) = system::read_psi_some_total("/proc/pressure/memory") {
        state
            .metrics
            .system_memory_psi_wait_seconds_total
            .set(mem_psi);
    }
    if let Ok(io_psi) = system::read_psi_some_total("/proc/pressure/io") {
        state.metrics.system_disk_psi_wait_seconds_total.set(io_psi);
    }
}

fn export_disk_metrics(state: &SharedState) {
    match collectors::diskstats::read_diskstats() {
        Ok(diskstats) => {
            for (device, stats) in diskstats {
                state
                    .metrics
                    .system_disk_read_bytes_total
                    .with_label_values(&[&device])
                    .set(stats.sectors_read as f64 * 512.0);
                state
                    .metrics
                    .system_disk_write_bytes_total
                    .with_label_values(&[&device])
                    .set(stats.sectors_written as f64 * 512.0);
                state
                    .metrics
                    .system_disk_io_time_seconds_total
                    .with_label_values(&[&device])
                    .set(stats.time_io_ms as f64 / 1000.0);
                state
                    .metrics
                    .system_disk_queue_depth
                    .with_label_values(&[&device])
                    .set(stats.ios_in_progress as f64);
            }
        }
        Err(e) => warn!("Failed to read disk statistics: {}", e),
    }
}

fn export_network_metrics(state: &SharedState) {
    match collectors::netdev::read_netdev_stats() {
        Ok(netdevs) => {
            for (device, stats) in netdevs {
                state
                    .metrics
                    .system_net_rx_bytes_total
                    .with_label_values(&[&device])
                    .set(stats.receive_bytes as f64);
                state
                    .metrics
                    .system_net_tx_bytes_total
                    .with_label_values(&[&device])
                    .set(stats.transmit_bytes as f64);
                state
                    .metrics
                    .system_net_rx_errors_total
                    .with_label_values(&[&device])
                    .set(stats.receive_errs as f64);
                state
                    .metrics
                    .system_net_tx_errors_total
                    .with_label_values(&[&device])
                    .set(stats.transmit_errs as f64);
                state
                    .metrics
                    .system_net_drops_total
                    .with_label_values(&[device.as_str(), "rx"])
                    .set(stats.receive_drop as f64);
                state
                    .metrics
                    .system_net_drops_total
                    .with_label_values(&[device.as_str(), "tx"])
                    .set(stats.transmit_drop as f64);
            }
        }
        Err(e) => warn!("Failed to read network device statistics: {}", e),
    }
}

fn export_filesystem_metrics(state: &SharedState) {
    match collectors::filesystem::read_filesystem_stats() {
        Ok(filesystems) => {
            for fs in filesystems {
                state
                    .metrics
                    .system_filesystem_avail_bytes
                    .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                    .set(fs.available_bytes as f64);
                state
                    .metrics
                    .system_filesystem_size_bytes
                    .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                    .set(fs.size_bytes as f64);
                state
                    .metrics
                    .system_filesystem_files
                    .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                    .set(fs.files_total as f64);
                state
                    .metrics
                    .system_filesystem_files_free
                    .with_label_values(&[&fs.device, &fs.mount_point, &fs.fstype])
                    .set(fs.files_free as f64);
            }
        }
        Err(e) => warn!("Failed to read filesystem statistics: {}", e),
    }
}

fn export_thermal_metrics(state: &SharedState) {
    match collectors::thermal::collect_temperatures() {
        Ok(temperatures) => {
            for (sensor, temp) in temperatures {
                state
                    .metrics
                    .system_cpu_temp_celsius
                    .with_label_values(&[&sensor])
                    .set(temp);
            }
        }
        Err(e) => warn!("Failed to read thermal sensors: {}", e),
    }
}
