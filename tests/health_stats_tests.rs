//! Integration tests for health stats module.
//!
//! These tests verify that the HealthStats structure correctly tracks
//! and reports all the new metrics including eBPF performance, error tracking,
//! timing breakdown, and resource limits.

use herakles_node_exporter::health_stats::HealthStats;
use std::sync::Arc;

#[test]
fn test_health_stats_new_fields_initialize() {
    let stats = HealthStats::new();

    // Test eBPF Performance fields
    let (ep_cur, ep_avg, ep_max, ep_min, ep_count) = stats.ebpf_events_per_sec.snapshot();
    assert_eq!(ep_count, 0);
    assert_eq!(ep_cur, 0.0);
    assert_eq!(ep_avg, 0.0);

    let ebpf_lost = stats
        .ebpf_lost_events
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(ebpf_lost, 0);

    // Test Error Tracking fields
    let proc_errors = stats
        .proc_read_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(proc_errors, 0);

    let parsing_errors = stats
        .parsing_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(parsing_errors, 0);

    let perm_denied = stats
        .permission_denied_count
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(perm_denied, 0);

    let ebpf_fails = stats
        .ebpf_init_failures
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(ebpf_fails, 0);

    // Test Timing Breakdown fields
    let (pd_cur, _, _, _, pd_count) = stats.parsing_duration_ms.snapshot();
    assert_eq!(pd_count, 0);
    assert_eq!(pd_cur, 0.0);

    let (sd_cur, _, _, _, sd_count) = stats.serialization_duration_ms.snapshot();
    assert_eq!(sd_count, 0);
    assert_eq!(sd_cur, 0.0);

    let (lw_cur, _, _, _, lw_count) = stats.lock_wait_duration_ms.snapshot();
    assert_eq!(lw_count, 0);
    assert_eq!(lw_cur, 0.0);

    // Test Resource Limits fields
    let open_fds = stats.open_fds.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(open_fds, 0);

    let max_fds = stats.max_fds.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(max_fds, 0);

    let (mrs_cur, _, _, _, mrs_count) = stats.metrics_response_size_kb.snapshot();
    assert_eq!(mrs_count, 0);
    assert_eq!(mrs_cur, 0.0);

    let (tts_cur, _, _, _, tts_count) = stats.total_time_series.snapshot();
    assert_eq!(tts_count, 0);
    assert_eq!(tts_cur, 0.0);
}

#[test]
fn test_health_stats_recording_methods() {
    let stats = HealthStats::new();

    // Test eBPF recording
    stats.record_ebpf_events_per_sec(1234.5);
    let (ep_cur, _, _, _, _) = stats.ebpf_events_per_sec.snapshot();
    assert_eq!(ep_cur, 1234.5);

    stats.ebpf_map_usage_percent.add_sample(45.2);
    let (mu_cur, _, _, _, _) = stats.ebpf_map_usage_percent.snapshot();
    assert_eq!(mu_cur, 45.2);

    stats.ebpf_overhead_cpu_percent.add_sample(0.8);
    let (eo_cur, _, _, _, _) = stats.ebpf_overhead_cpu_percent.snapshot();
    assert_eq!(eo_cur, 0.8);

    // Test Error Tracking recording
    stats.record_proc_read_error();
    stats.record_proc_read_error();
    let proc_errors = stats
        .proc_read_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(proc_errors, 2);

    stats.record_parsing_error();
    let parsing_errors = stats
        .parsing_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(parsing_errors, 1);

    stats.record_permission_denied();
    stats.record_permission_denied();
    stats.record_permission_denied();
    let perm_denied = stats
        .permission_denied_count
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(perm_denied, 3);

    // Test Timing Breakdown recording
    stats.record_serialization_duration_ms(3.2);
    let (sd_cur, _, _, _, _) = stats.serialization_duration_ms.snapshot();
    assert_eq!(sd_cur, 3.2);

    stats.record_lock_wait_duration_ms(0.1);
    let (lw_cur, _, _, _, _) = stats.lock_wait_duration_ms.snapshot();
    assert_eq!(lw_cur, 0.1);

    // Test Resource Limits recording
    stats.update_fd_usage(128, 1024);
    let open_fds = stats.open_fds.load(std::sync::atomic::Ordering::Relaxed);
    let max_fds = stats.max_fds.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(open_fds, 128);
    assert_eq!(max_fds, 1024);

    stats.record_metrics_response_size_kb(245.3);
    let (mrs_cur, _, _, _, _) = stats.metrics_response_size_kb.snapshot();
    assert_eq!(mrs_cur, 245.3);

    stats.record_total_time_series(1840);
    let (tts_cur, _, _, _, _) = stats.total_time_series.snapshot();
    assert_eq!(tts_cur, 1840.0);
}

#[test]
fn test_health_stats_render_table_contains_new_sections() {
    let stats = Arc::new(HealthStats::new());

    // Record some sample data
    stats.record_ebpf_events_per_sec(12450.0);
    stats.ebpf_map_usage_percent.add_sample(45.2);
    stats.ebpf_overhead_cpu_percent.add_sample(0.8);

    stats.record_proc_read_error();
    stats.record_parsing_error();
    stats.record_permission_denied();

    stats.record_serialization_duration_ms(3.2);
    stats.record_lock_wait_duration_ms(0.1);

    stats.update_fd_usage(128, 1024);
    stats.record_metrics_response_size_kb(245.3);
    stats.record_total_time_series(1840);

    // Render the table
    let output = stats.render_table();

    // Check for new section headers
    assert!(
        output.contains("EBPF PERFORMANCE"),
        "Should contain EBPF PERFORMANCE section"
    );
    assert!(
        output.contains("ERROR TRACKING"),
        "Should contain ERROR TRACKING section"
    );
    assert!(
        output.contains("TIMING BREAKDOWN (ms)"),
        "Should contain TIMING BREAKDOWN section"
    );
    assert!(
        output.contains("RESOURCE LIMITS"),
        "Should contain RESOURCE LIMITS section"
    );

    // Check for specific metric names
    assert!(
        output.contains("ebpf_events_per_sec"),
        "Should contain ebpf_events_per_sec metric"
    );
    assert!(
        output.contains("ebpf_lost_events_total"),
        "Should contain ebpf_lost_events_total metric"
    );
    assert!(
        output.contains("ebpf_map_usage (%)"),
        "Should contain ebpf_map_usage metric"
    );
    assert!(
        output.contains("ebpf_overhead_cpu (%)"),
        "Should contain ebpf_overhead_cpu metric"
    );

    assert!(
        output.contains("proc_read_errors"),
        "Should contain proc_read_errors metric"
    );
    assert!(
        output.contains("parsing_errors"),
        "Should contain parsing_errors metric"
    );
    assert!(
        output.contains("permission_denied_count"),
        "Should contain permission_denied_count metric"
    );
    assert!(
        output.contains("ebpf_init_failures"),
        "Should contain ebpf_init_failures metric"
    );

    assert!(
        output.contains("parsing_duration"),
        "Should contain parsing_duration metric"
    );
    assert!(
        output.contains("serialization_duration"),
        "Should contain serialization_duration metric"
    );
    assert!(
        output.contains("lock_wait_duration"),
        "Should contain lock_wait_duration metric"
    );

    assert!(
        output.contains("open_file_descriptors"),
        "Should contain open_file_descriptors metric"
    );
    assert!(
        output.contains("max_file_descriptors"),
        "Should contain max_file_descriptors metric"
    );
    assert!(
        output.contains("fd_usage (%)"),
        "Should contain fd_usage metric"
    );
    assert!(
        output.contains("metrics_response_size (KB)"),
        "Should contain metrics_response_size metric"
    );
    assert!(
        output.contains("total_time_series"),
        "Should contain total_time_series metric"
    );

    // Check that recorded values are present
    let ebpf_section = output.split("EBPF PERFORMANCE").nth(1).unwrap();
    assert!(
        ebpf_section.contains("12450"),
        "Should contain recorded ebpf_events_per_sec value"
    );

    let ebpf_map_section = output.split("ebpf_map_usage (%)").nth(1).unwrap();
    assert!(
        ebpf_map_section.contains("45.2"),
        "Should contain recorded ebpf_map_usage value"
    );

    let fd_section = output.split("RESOURCE LIMITS").nth(1).unwrap();
    assert!(
        fd_section.contains("128"),
        "Should contain recorded open_file_descriptors value"
    );
    assert!(
        fd_section.contains("1024"),
        "Should contain recorded max_file_descriptors value"
    );

    let response_size_section = output.split("metrics_response_size (KB)").nth(1).unwrap();
    assert!(
        response_size_section.contains("245.3"),
        "Should contain recorded metrics_response_size value"
    );

    let time_series_section = output.split("total_time_series").nth(1).unwrap();
    assert!(
        time_series_section.contains("1840"),
        "Should contain recorded total_time_series value"
    );
}

#[test]
fn test_fd_usage_percentage_calculation() {
    let stats = HealthStats::new();

    // Test with valid values
    stats.update_fd_usage(128, 1024);
    let output = stats.render_table();
    assert!(output.contains("12.5"), "FD usage should be 12.5%");

    // Test with zero max (edge case)
    stats.update_fd_usage(50, 0);
    let output = stats.render_table();
    // Should handle division by zero gracefully
    assert!(output.contains("fd_usage (%)"));
}

#[test]
fn test_thread_safety_of_new_fields() {
    use std::thread;

    let stats = Arc::new(HealthStats::new());
    let mut handles = vec![];

    // Spawn multiple threads to update stats concurrently
    for i in 0..10 {
        let stats_clone = Arc::clone(&stats);
        let handle = thread::spawn(move || {
            stats_clone.record_ebpf_events_per_sec(100.0 * i as f64);
            stats_clone.record_proc_read_error();
            stats_clone.record_parsing_error();
            stats_clone.record_permission_denied();
            stats_clone.record_serialization_duration_ms(1.0 * i as f64);
            stats_clone.record_lock_wait_duration_ms(0.1 * i as f64);
            stats_clone.update_fd_usage(10 + i as u64, 1000);
            stats_clone.record_metrics_response_size_kb(50.0 * i as f64);
            stats_clone.record_total_time_series(100 * i as u64);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify that all updates were recorded
    let proc_errors = stats
        .proc_read_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(proc_errors, 10, "Should have recorded 10 proc read errors");

    let parsing_errors = stats
        .parsing_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(parsing_errors, 10, "Should have recorded 10 parsing errors");

    let perm_denied = stats
        .permission_denied_count
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(perm_denied, 10, "Should have recorded 10 permission denied");

    // FD usage should be the last value written
    let open_fds = stats.open_fds.load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        open_fds >= 10 && open_fds <= 19,
        "Open FDs should be in range"
    );

    let max_fds = stats.max_fds.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(max_fds, 1000, "Max FDs should be 1000");
}
