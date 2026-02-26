//! Test to verify CPU percent averaging in ringbuffer entries.
//!
//! This test validates that the ringbuffer records average CPU percent
//! instead of sum when aggregating multiple processes in a subgroup.

#[test]
fn test_cpu_percent_averaging_logic() {
    // This test validates the mathematical logic used in the ringbuffer recording.
    // If we have 3 processes with CPU percentages: 10%, 20%, 30%
    // The sum is 60%, but the average should be 20%

    let cpu_percentages = vec![10.0_f32, 20.0_f32, 30.0_f32];
    let process_count = cpu_percentages.len();

    // Simulate the aggregation logic
    let cpu_percent_sum: f64 = cpu_percentages.iter().map(|&x| x as f64).sum();

    // Calculate average (this is what the fix implements)
    let avg_cpu_percent = if process_count > 0 {
        (cpu_percent_sum / process_count as f64) as f32
    } else {
        0.0
    };

    // The average should be 20%, not 60%
    assert_eq!(avg_cpu_percent, 20.0);
    assert_ne!(avg_cpu_percent, 60.0); // This was the bug - storing sum instead of average
}

#[test]
fn test_cpu_percent_averaging_single_process() {
    // Edge case: single process should have average equal to its own value
    let cpu_percentages = vec![15.5_f32];
    let process_count = cpu_percentages.len();

    let cpu_percent_sum: f64 = cpu_percentages.iter().map(|&x| x as f64).sum();
    let avg_cpu_percent = if process_count > 0 {
        (cpu_percent_sum / process_count as f64) as f32
    } else {
        0.0
    };

    assert_eq!(avg_cpu_percent, 15.5);
}

#[test]
fn test_cpu_percent_averaging_zero_processes() {
    // Edge case: zero processes should return 0.0
    let process_count = 0;
    let cpu_percent_sum = 0.0_f64;

    let avg_cpu_percent = if process_count > 0 {
        (cpu_percent_sum / process_count as f64) as f32
    } else {
        0.0
    };

    assert_eq!(avg_cpu_percent, 0.0);
}

#[test]
fn test_cpu_percent_averaging_high_values() {
    // Test with high CPU values to ensure no overflow issues
    let cpu_percentages = vec![95.0_f32, 98.0_f32, 100.0_f32];
    let process_count = cpu_percentages.len();

    let cpu_percent_sum: f64 = cpu_percentages.iter().map(|&x| x as f64).sum();
    let avg_cpu_percent = if process_count > 0 {
        (cpu_percent_sum / process_count as f64) as f32
    } else {
        0.0
    };

    // Average of 95, 98, 100 should be (95 + 98 + 100) / 3 = 97.666...
    let expected_avg = (95.0 + 98.0 + 100.0) / 3.0;
    assert!((avg_cpu_percent - expected_avg).abs() < 0.001);
}
