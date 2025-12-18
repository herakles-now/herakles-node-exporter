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

// Key structure for BPF maps (PID + device for blkio)
struct io_key {
    u32 pid;
    u32 dev; // For block I/O: major:minor device number
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
    __type(key, struct io_key);
    __type(value, struct blkio_stats);
} blkio_stats_map SEC(".maps");

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

// Network receive tracepoint
SEC("tracepoint/net/netif_receive_skb")
int trace_netif_receive_skb(struct trace_event_raw_net_dev_template *ctx) {
    u32 pid = get_current_pid();
    u32 len = ctx->len;
    
    struct net_stats *stats = bpf_map_lookup_elem(&net_stats_map, &pid);
    if (!stats) {
        struct net_stats new_stats = {0};
        new_stats.rx_bytes = len;
        new_stats.rx_packets = 1;
        bpf_map_update_elem(&net_stats_map, &pid, &new_stats, BPF_ANY);
    } else {
        __sync_fetch_and_add(&stats->rx_bytes, len);
        __sync_fetch_and_add(&stats->rx_packets, 1);
    }
    
    // Increment event counter
    u32 idx = EVENT_NET_RX;
    u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
    if (counter) {
        __sync_fetch_and_add(counter, 1);
    }
    
    return 0;
}

// Network transmit kprobe
SEC("kprobe/dev_queue_xmit")
int BPF_KPROBE(trace_dev_queue_xmit, struct sk_buff *skb) {
    u32 pid = get_current_pid();
    u32 len = BPF_CORE_READ(skb, len);
    
    struct net_stats *stats = bpf_map_lookup_elem(&net_stats_map, &pid);
    if (!stats) {
        struct net_stats new_stats = {0};
        new_stats.tx_bytes = len;
        new_stats.tx_packets = 1;
        bpf_map_update_elem(&net_stats_map, &pid, &new_stats, BPF_ANY);
    } else {
        __sync_fetch_and_add(&stats->tx_bytes, len);
        __sync_fetch_and_add(&stats->tx_packets, 1);
    }
    
    // Increment event counter
    u32 idx = EVENT_NET_TX;
    u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
    if (counter) {
        __sync_fetch_and_add(counter, 1);
    }
    
    return 0;
}

// Block I/O request raw tracepoint (kernel-agnostic)
SEC("raw_tracepoint/block_rq_issue")
int raw_trace_block_rq_issue(struct bpf_raw_tracepoint_args *ctx) {
    u32 pid = get_current_pid();
    
    // Raw tracepoint args: (struct request *rq)
    // Read request structure using BPF_CORE_READ
    struct request *rq = (struct request *)ctx->args[0];
    
    // Read device number using CO-RE
    dev_t dev = BPF_CORE_READ(rq, rq_disk, major) << 20 | 
                BPF_CORE_READ(rq, rq_disk, first_minor);
    
    // Read operation size (in bytes)
    unsigned int data_len = BPF_CORE_READ(rq, __data_len);
    
    // Determine if read or write operation
    unsigned int cmd_flags = BPF_CORE_READ(rq, cmd_flags);
    bool is_write = (cmd_flags & 1);  // REQ_OP_WRITE = 1
    
    struct io_key key = {
        .pid = pid,
        .dev = dev,
    };
    
    struct blkio_stats *stats = bpf_map_lookup_elem(&blkio_stats_map, &key);
    if (!stats) {
        struct blkio_stats new_stats = {0};
        if (is_write) {
            new_stats.write_bytes = data_len;
            new_stats.write_ops = 1;
        } else {
            new_stats.read_bytes = data_len;
            new_stats.read_ops = 1;
        }
        bpf_map_update_elem(&blkio_stats_map, &key, &new_stats, BPF_ANY);
    } else {
        if (is_write) {
            __sync_fetch_and_add(&stats->write_bytes, data_len);
            __sync_fetch_and_add(&stats->write_ops, 1);
            
            u32 idx = EVENT_BLKIO_WRITE;
            u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
            if (counter) {
                __sync_fetch_and_add(counter, 1);
            }
        } else {
            __sync_fetch_and_add(&stats->read_bytes, data_len);
            __sync_fetch_and_add(&stats->read_ops, 1);
            
            u32 idx = EVENT_BLKIO_READ;
            u64 *counter = bpf_map_lookup_elem(&event_counters, &idx);
            if (counter) {
                __sync_fetch_and_add(counter, 1);
            }
        }
    }
    
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
