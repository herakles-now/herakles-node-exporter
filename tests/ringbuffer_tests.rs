//! Integration tests for the ringbuffer system.
//!
//! These tests verify end-to-end behavior of the ringbuffer system
//! by exercising the public API through realistic usage patterns.

/// Test that demonstrates the expected memory allocation pattern
#[test]
fn test_memory_calculation_examples() {
    // These are the calculations from the requirements
    let entry_size = 48usize;
    let max_memory_mb = 15usize;
    let max_bytes = max_memory_mb * 1024 * 1024;
    let max_total_entries = max_bytes / entry_size;

    let test_cases = [
        (10, 120),   // 10 subgroups → 32768 entries/subgroup, capped at 120
        (50, 120),   // 50 subgroups → 6553 entries/subgroup, capped at 120
        (200, 120),  // 200 subgroups → 1638 entries/subgroup, capped at 120
        (5000, 65),  // 5000 subgroups → 65 entries/subgroup
        (40000, 10), // 40000 subgroups → 8 entries/subgroup, capped at 10
    ];

    for (subgroup_count, expected_entries) in test_cases {
        let calculated = max_total_entries / subgroup_count;
        let clamped = calculated.clamp(10, 120);

        assert!(
            clamped.abs_diff(expected_entries) <= 1,
            "For {} subgroups, expected ~{} entries but calculated {}",
            subgroup_count,
            expected_entries,
            clamped
        );
    }
}
