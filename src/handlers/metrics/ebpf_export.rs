//! eBPF-backed metric population for the Prometheus handler.

use crate::state::SharedState;

#[cfg(feature = "ebpf")]
use ahash::AHashMap as HashMap;
#[cfg(feature = "ebpf")]
use tracing::warn;

#[cfg(feature = "ebpf")]
pub(super) fn export_blkio_metrics(state: &SharedState) {
    if let Some(ebpf) = &state.ebpf {
        match ebpf.read_process_blkio_stats() {
            Ok(blkio_stats) => {
                let mut blkio_groups: HashMap<(String, String), (u64, u64, u64, u64)> =
                    HashMap::new();

                for stat in blkio_stats {
                    let (group, subgroup) = crate::process::classify_process_raw(&stat.comm);
                    let entry = blkio_groups
                        .entry((group.to_string(), subgroup.to_string()))
                        .or_insert((0, 0, 0, 0));

                    entry.0 += stat.read_bytes;
                    entry.1 += stat.write_bytes;
                    entry.2 += stat.read_ops;
                    entry.3 += stat.write_ops;
                }

                for ((group, subgroup), (read_bytes, write_bytes, read_ops, write_ops)) in
                    blkio_groups
                {
                    state
                        .metrics
                        .group_blkio_read_bytes_total
                        .with_label_values(&[&group, &subgroup])
                        .set(read_bytes as f64);
                    state
                        .metrics
                        .group_blkio_write_bytes_total
                        .with_label_values(&[&group, &subgroup])
                        .set(write_bytes as f64);
                    state
                        .metrics
                        .group_blkio_read_syscalls_total
                        .with_label_values(&[&group, &subgroup])
                        .set(read_ops as f64);
                    state
                        .metrics
                        .group_blkio_write_syscalls_total
                        .with_label_values(&[&group, &subgroup])
                        .set(write_ops as f64);
                }
            }
            Err(e) => warn!("Failed to read eBPF block I/O statistics: {}", e),
        }
    }
}

#[cfg(not(feature = "ebpf"))]
pub(super) fn export_blkio_metrics(_state: &SharedState) {}

#[cfg(feature = "ebpf")]
pub(super) fn export_network_metrics(state: &SharedState) {
    if let Some(ebpf) = &state.ebpf {
        match ebpf.read_process_net_stats() {
            Ok(net_stats) => {
                let mut net_groups: HashMap<(String, String), (u64, u64)> = HashMap::new();

                for stat in net_stats {
                    let (group, subgroup) = crate::process::classify_process_raw(&stat.comm);
                    let entry = net_groups
                        .entry((group.to_string(), subgroup.to_string()))
                        .or_insert((0, 0));

                    entry.0 += stat.rx_bytes;
                    entry.1 += stat.tx_bytes;
                }

                for ((group, subgroup), (rx, tx)) in net_groups {
                    state
                        .metrics
                        .group_net_rx_bytes_total
                        .with_label_values(&[&group, &subgroup])
                        .set(rx as f64);

                    state
                        .metrics
                        .group_net_tx_bytes_total
                        .with_label_values(&[&group, &subgroup])
                        .set(tx as f64);
                }
            }
            Err(e) => warn!("Failed to read eBPF network statistics: {}", e),
        }
    }
}

#[cfg(not(feature = "ebpf"))]
pub(super) fn export_network_metrics(_state: &SharedState) {}

#[cfg(feature = "ebpf")]
pub(super) fn export_tcp_metrics(state: &SharedState, enable_tcp_tracking: bool) {
    if !enable_tcp_tracking {
        return;
    }

    if let Some(ebpf) = &state.ebpf {
        match ebpf.read_tcp_stats() {
            Ok(tcp_stats) => {
                state
                    .metrics
                    .system_tcp_connections_established
                    .set(tcp_stats.established as f64);
                state
                    .metrics
                    .system_tcp_connections_syn_sent
                    .set(tcp_stats.syn_sent as f64);
                state
                    .metrics
                    .system_tcp_connections_syn_recv
                    .set(tcp_stats.syn_recv as f64);
                state
                    .metrics
                    .system_tcp_connections_fin_wait1
                    .set(tcp_stats.fin_wait1 as f64);
                state
                    .metrics
                    .system_tcp_connections_fin_wait2
                    .set(tcp_stats.fin_wait2 as f64);
                state
                    .metrics
                    .system_tcp_connections_time_wait
                    .set(tcp_stats.time_wait as f64);
                state
                    .metrics
                    .system_tcp_connections_close
                    .set(tcp_stats.close as f64);
                state
                    .metrics
                    .system_tcp_connections_close_wait
                    .set(tcp_stats.close_wait as f64);
                state
                    .metrics
                    .system_tcp_connections_last_ack
                    .set(tcp_stats.last_ack as f64);
                state
                    .metrics
                    .system_tcp_connections_listen
                    .set(tcp_stats.listen as f64);
                state
                    .metrics
                    .system_tcp_connections_closing
                    .set(tcp_stats.closing as f64);
            }
            Err(e) => warn!("Failed to read TCP connection statistics: {}", e),
        }
    }
}

#[cfg(not(feature = "ebpf"))]
pub(super) fn export_tcp_metrics(_state: &SharedState, _enable_tcp_tracking: bool) {}
