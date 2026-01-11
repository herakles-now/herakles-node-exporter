// SPDX-License-Identifier: GPL-2.0 OR BSD-3-Clause
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Maximum number of processes to track
#define MAX_ENTRIES 10240

// Process network I/O statistics
struct net_stats {
    u64 rx_bytes;
    u64 tx_bytes;
    u64 rx_packets;
    u64 tx_packets;
    u64 dropped;
};

// Process block I/O statistics
struct blkio_stats {
    u64 read_bytes;
    u64 write_bytes;
    u64 read_ops;
    u64 write_ops;
};

// Syscall pending info for tracking in-flight I/O syscalls
struct io_syscall_info {
    u64 ts;      // Timestamp
    u32 fd;      // File descriptor
    u64 count;   // Requested byte count
    u8 is_write; // 0 = read, 1 = write
};

// BPF maps
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_ENTRIES);
    __type(key, u32); // PID
    __type(value, struct net_stats);
} net_stats_map SEC(".maps");

struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_ENTRIES);
    __type(key, u32); // PID
    __type(value, struct blkio_stats);
} blkio_stats_map SEC(".maps");

// Pending syscalls map to correlate entry/exit
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_ENTRIES);
    __type(key, u64); // pid_tgid
    __type(value, struct io_syscall_info);
} syscall_pending SEC(".maps");

// TCP connection state tracking
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 12); // Number of TCP states
    __type(key, u32); // TCP state
    __type(value, u64); // Count
} tcp_state_map SEC(".maps");

// Event counters for performance monitoring
struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 4);
    __type(key, u32);
    __type(value, u64);
} event_counters SEC(".maps");

// Event counter indices
#define EVENT_NET_RX 0
#define EVENT_NET_TX 1
#define EVENT_BLKIO_READ 2
#define EVENT_BLKIO_WRITE 3

// Helper to get current PID
static __always_inline u32 get_current_pid() {
    return bpf_get_current_pid_tgid() >> 32;
}

// Helper to update network stats for a PID
// Updates the net_stats_map with receive or transmit I/O statistics for a given process.
// If the PID doesn't exist in the map, creates a new entry. Otherwise, atomically
// increments the existing counters.
//
// Parameters:
//   pid: Process ID
//   bytes: Number of bytes received or transmitted
//   is_tx: true for transmit operations, false for receive operations
static __always_inline void update_net_stats(u32 pid, u64 bytes, bool is_tx) {
    struct net_stats *stats = bpf_map_lookup_elem(&net_stats_map, &pid);
    if (!stats) {
        struct net_stats new_stats = {0};
        if (is_tx) {
            new_stats.tx_bytes = bytes;
            new_stats.tx_packets = 1;
        } else {
            new_stats.rx_bytes = bytes;
            new_stats.rx_packets = 1;
        }
        bpf_map_update_elem(&net_stats_map, &pid, &new_stats, BPF_ANY);
        
        // Update event counter for new entry
        u32 idx = is_tx ? EVENT_NET_TX : EVENT_NET_RX;
        u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
        if (counter) {
            __sync_fetch_and_add(counter, 1);
        }
    } else {
        if (is_tx) {
            __sync_fetch_and_add(&stats->tx_bytes, bytes);
            __sync_fetch_and_add(&stats->tx_packets, 1);
            
            u32 idx = EVENT_NET_TX;
            u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
            if (counter) {
                __sync_fetch_and_add(counter, 1);
            }
        } else {
            __sync_fetch_and_add(&stats->rx_bytes, bytes);
            __sync_fetch_and_add(&stats->rx_packets, 1);
            
            u32 idx = EVENT_NET_RX;
            u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
            if (counter) {
                __sync_fetch_and_add(counter, 1);
            }
        }
    }
}

// ========== SYSCALL TRACEPOINT HOOKS FOR NETWORK I/O ==========
// These syscall tracepoints track actual network I/O at the syscall level,
// providing accurate per-process accounting in the correct process context.

// recvfrom syscall entry
SEC("tracepoint/syscalls/sys_enter_recvfrom")
int trace_recvfrom_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // sockfd
    info.count = ctx->args[2];   // len
    info.is_write = 0;           // receive operation
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// recvfrom syscall exit
SEC("tracepoint/syscalls/sys_exit_recvfrom")
int trace_recvfrom_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    // Ignore errors and zero-byte operations
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    update_net_stats(pid, (u64)ret, false);
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// sendto syscall entry
SEC("tracepoint/syscalls/sys_enter_sendto")
int trace_sendto_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // sockfd
    info.count = ctx->args[2];   // len
    info.is_write = 1;           // send operation
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// sendto syscall exit
SEC("tracepoint/syscalls/sys_exit_sendto")
int trace_sendto_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    // Ignore errors and zero-byte operations
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    update_net_stats(pid, (u64)ret, true);
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// recvmsg syscall entry
SEC("tracepoint/syscalls/sys_enter_recvmsg")
int trace_recvmsg_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // sockfd
    info.is_write = 0;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// recvmsg syscall exit
SEC("tracepoint/syscalls/sys_exit_recvmsg")
int trace_recvmsg_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    update_net_stats(pid, (u64)ret, false);
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// sendmsg syscall entry
SEC("tracepoint/syscalls/sys_enter_sendmsg")
int trace_sendmsg_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // sockfd
    info.is_write = 1;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// sendmsg syscall exit
SEC("tracepoint/syscalls/sys_exit_sendmsg")
int trace_sendmsg_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    update_net_stats(pid, (u64)ret, true);
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// recv syscall entry
SEC("tracepoint/syscalls/sys_enter_recv")
int trace_recv_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // sockfd
    info.count = ctx->args[2];   // len
    info.is_write = 0;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// recv syscall exit
SEC("tracepoint/syscalls/sys_exit_recv")
int trace_recv_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    update_net_stats(pid, (u64)ret, false);
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// send syscall entry
SEC("tracepoint/syscalls/sys_enter_send")
int trace_send_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // sockfd
    info.count = ctx->args[2];   // len
    info.is_write = 1;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// send syscall exit
SEC("tracepoint/syscalls/sys_exit_send")
int trace_send_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    update_net_stats(pid, (u64)ret, true);
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// ========== SYSCALL TRACEPOINT HOOKS FOR BLOCK I/O ==========
// Note: struct trace_event_raw_sys_enter and trace_event_raw_sys_exit are defined
// in vmlinux.h and represent the kernel tracepoint contexts for syscall entry/exit.
// They provide access to syscall arguments via ctx->args[] and return value via ctx->ret.

// Helper to update blkio stats for a PID
// Updates the blkio_stats_map with read or write I/O statistics for a given process.
// If the PID doesn't exist in the map, creates a new entry. Otherwise, atomically
// increments the existing counters.
//
// Parameters:
//   pid: Process ID
//   bytes: Number of bytes read or written
//   is_write: true for write operations, false for read operations
static __always_inline void update_blkio_stats(u32 pid, u64 bytes, bool is_write) {
    struct blkio_stats *stats = bpf_map_lookup_elem(&blkio_stats_map, &pid);
    if (!stats) {
        struct blkio_stats new_stats = {0};
        if (is_write) {
            new_stats.write_bytes = bytes;
            new_stats.write_ops = 1;
        } else {
            new_stats.read_bytes = bytes;
            new_stats.read_ops = 1;
        }
        bpf_map_update_elem(&blkio_stats_map, &pid, &new_stats, BPF_ANY);
    } else {
        if (is_write) {
            __sync_fetch_and_add(&stats->write_bytes, bytes);
            __sync_fetch_and_add(&stats->write_ops, 1);
            
            u32 idx = EVENT_BLKIO_WRITE;
            u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
            if (counter) {
                __sync_fetch_and_add(counter, 1);
            }
        } else {
            __sync_fetch_and_add(&stats->read_bytes, bytes);
            __sync_fetch_and_add(&stats->read_ops, 1);
            
            u32 idx = EVENT_BLKIO_READ;
            u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
            if (counter) {
                __sync_fetch_and_add(counter, 1);
            }
        }
    }
}

// Read syscall entry
SEC("tracepoint/syscalls/sys_enter_read")
int trace_read_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // fd
    info.count = ctx->args[2];   // count
    info.is_write = 0;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// Read syscall exit
SEC("tracepoint/syscalls/sys_exit_read")
int trace_read_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    // Ignore errors and zero-byte operations
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    struct io_syscall_info *info = bpf_map_lookup_elem(&syscall_pending, &pid_tgid);
    if (info && !info->is_write) {
        update_blkio_stats(pid, (u64)ret, false);
    }
    
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// Write syscall entry
SEC("tracepoint/syscalls/sys_enter_write")
int trace_write_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // fd
    info.count = ctx->args[2];   // count
    info.is_write = 1;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// Write syscall exit
SEC("tracepoint/syscalls/sys_exit_write")
int trace_write_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    // Ignore errors and zero-byte operations
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    struct io_syscall_info *info = bpf_map_lookup_elem(&syscall_pending, &pid_tgid);
    if (info && info->is_write) {
        update_blkio_stats(pid, (u64)ret, true);
    }
    
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// pread64 syscall entry
SEC("tracepoint/syscalls/sys_enter_pread64")
int trace_pread64_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // fd
    info.count = ctx->args[2];   // count
    info.is_write = 0;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// pread64 syscall exit
SEC("tracepoint/syscalls/sys_exit_pread64")
int trace_pread64_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    struct io_syscall_info *info = bpf_map_lookup_elem(&syscall_pending, &pid_tgid);
    if (info && !info->is_write) {
        update_blkio_stats(pid, (u64)ret, false);
    }
    
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// pwrite64 syscall entry
SEC("tracepoint/syscalls/sys_enter_pwrite64")
int trace_pwrite64_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // fd
    info.count = ctx->args[2];   // count
    info.is_write = 1;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// pwrite64 syscall exit
SEC("tracepoint/syscalls/sys_exit_pwrite64")
int trace_pwrite64_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    struct io_syscall_info *info = bpf_map_lookup_elem(&syscall_pending, &pid_tgid);
    if (info && info->is_write) {
        update_blkio_stats(pid, (u64)ret, true);
    }
    
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// readv syscall entry
SEC("tracepoint/syscalls/sys_enter_readv")
int trace_readv_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // fd
    // Note: count not set for readv as total size is unknown until syscall returns
    info.is_write = 0;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// readv syscall exit
SEC("tracepoint/syscalls/sys_exit_readv")
int trace_readv_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    struct io_syscall_info *info = bpf_map_lookup_elem(&syscall_pending, &pid_tgid);
    if (info && !info->is_write) {
        update_blkio_stats(pid, (u64)ret, false);
    }
    
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// writev syscall entry
SEC("tracepoint/syscalls/sys_enter_writev")
int trace_writev_enter(struct trace_event_raw_sys_enter *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    
    struct io_syscall_info info = {0};
    info.ts = bpf_ktime_get_ns();
    info.fd = ctx->args[0];      // fd
    // Note: count not set for writev as total size is unknown until syscall returns
    info.is_write = 1;
    
    bpf_map_update_elem(&syscall_pending, &pid_tgid, &info, BPF_ANY);
    return 0;
}

// writev syscall exit
SEC("tracepoint/syscalls/sys_exit_writev")
int trace_writev_exit(struct trace_event_raw_sys_exit *ctx) {
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    long ret = ctx->ret;
    
    if (ret <= 0) {
        bpf_map_delete_elem(&syscall_pending, &pid_tgid);
        return 0;
    }
    
    struct io_syscall_info *info = bpf_map_lookup_elem(&syscall_pending, &pid_tgid);
    if (info && info->is_write) {
        update_blkio_stats(pid, (u64)ret, true);
    }
    
    bpf_map_delete_elem(&syscall_pending, &pid_tgid);
    return 0;
}

// TCP state change tracepoint
SEC("tracepoint/sock/inet_sock_set_state")
int trace_inet_sock_set_state(struct trace_event_raw_inet_sock_set_state *ctx) {
    u32 newstate = ctx->newstate;
    
    u64 *count = bpf_map_lookup_elem(&tcp_state_map, &newstate);
    if (!count) {
        u64 initial = 1;
        bpf_map_update_elem(&tcp_state_map, &newstate, &initial, BPF_ANY);
    } else {
        __sync_fetch_and_add(count, 1);
    }
    
    return 0;
}

char LICENSE[] SEC("license") = "GPL";
