//! Integration tests for the health module.
//!
//! These tests verify the behavior of `HealthState::get_health()` with
//! various configurations and buffer states.

use herakles_node_exporter::{AppConfig, BufferHealthConfig, HealthState};

/// Helper function to create a default configuration.
fn default_config() -> AppConfig {
    AppConfig::default()
}

/// Helper function to create a config with custom settings for io_buffer.
fn custom_io_config(
    capacity_kb: usize,
    larger_is_better: bool,
    warn_percent: Option<f64>,
    critical_percent: Option<f64>,
) -> AppConfig {
    AppConfig {
        io_buffer: BufferHealthConfig {
            capacity_kb,
            larger_is_better,
            warn_percent,
            critical_percent,
        },
        smaps_buffer: BufferHealthConfig::default(),
        smaps_rollup_buffer: BufferHealthConfig::default(),
    }
}

#[test]
fn test_all_buffers_ok() {
    let state = HealthState::new(default_config());

    // Set all buffers to low values (well below 80% warn threshold)
    state.update_io_buffer_kb(50); // 50/256 = ~19.5%
    state.update_smaps_buffer_kb(100); // 100/512 = ~19.5%
    state.update_smaps_rollup_buffer_kb(50); // 50/256 = ~19.5%

    let response = state.get_health();

    assert_eq!(response.overall_status, "ok");
    assert_eq!(response.buffers.len(), 3);

    for buffer in &response.buffers {
        assert_eq!(buffer.status, "ok", "Buffer {} should be ok", buffer.name);
    }
}

#[test]
fn test_one_buffer_warn() {
    let state = HealthState::new(default_config());

    // IO buffer at 85% (above 80% warn, below 95% critical)
    state.update_io_buffer_kb(218); // 218/256 = ~85%
    state.update_smaps_buffer_kb(100);
    state.update_smaps_rollup_buffer_kb(50);

    let response = state.get_health();

    assert_eq!(response.overall_status, "warn");

    let io_buffer = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .expect("io_buffer_kb not found");
    assert_eq!(io_buffer.status, "warn");
}

#[test]
fn test_one_buffer_critical() {
    let state = HealthState::new(default_config());

    // IO buffer at 98% (above 95% critical)
    state.update_io_buffer_kb(251); // 251/256 = ~98%
    state.update_smaps_buffer_kb(100);
    state.update_smaps_rollup_buffer_kb(50);

    let response = state.get_health();

    assert_eq!(response.overall_status, "critical");

    let io_buffer = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .expect("io_buffer_kb not found");
    assert_eq!(io_buffer.status, "critical");
}

#[test]
fn test_larger_is_better_ok() {
    // When larger_is_better=true, high fill percentage is good
    let config = custom_io_config(100, true, Some(30.0), Some(10.0));
    let state = HealthState::new(config);

    state.update_io_buffer_kb(50); // 50% fill, above 30% warn

    let response = state.get_health();

    let io_buffer = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .expect("io_buffer_kb not found");
    assert_eq!(io_buffer.status, "ok");
    assert!(io_buffer.larger_is_better);
}

#[test]
fn test_larger_is_better_warn() {
    // When larger_is_better=true, low fill percentage triggers warn
    let config = custom_io_config(100, true, Some(30.0), Some(10.0));
    let state = HealthState::new(config);

    state.update_io_buffer_kb(20); // 20% fill, below 30% warn but above 10% critical

    let response = state.get_health();

    let io_buffer = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .expect("io_buffer_kb not found");
    assert_eq!(io_buffer.status, "warn");
}

#[test]
fn test_larger_is_better_critical() {
    // When larger_is_better=true, very low fill percentage triggers critical
    let config = custom_io_config(100, true, Some(30.0), Some(10.0));
    let state = HealthState::new(config);

    state.update_io_buffer_kb(5); // 5% fill, below 10% critical

    let response = state.get_health();

    let io_buffer = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .expect("io_buffer_kb not found");
    assert_eq!(io_buffer.status, "critical");
}

#[test]
fn test_no_thresholds_always_ok() {
    // Without thresholds, status should always be "ok"
    let config = custom_io_config(100, false, None, None);
    let state = HealthState::new(config);

    // Even at 99% fill, should be ok without thresholds
    state.update_io_buffer_kb(99);

    let response = state.get_health();

    let io_buffer = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .expect("io_buffer_kb not found");
    assert_eq!(io_buffer.status, "ok");
}

#[test]
fn test_fill_percent_accuracy() {
    let config = custom_io_config(200, false, None, None);
    let state = HealthState::new(config);

    state.update_io_buffer_kb(100); // 100/200 = 50%

    let response = state.get_health();

    let io_buffer = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .expect("io_buffer_kb not found");

    assert!((io_buffer.fill_percent - 50.0).abs() < 0.01);
    assert_eq!(io_buffer.capacity_kb, 200);
    assert_eq!(io_buffer.current_kb, 100);
}

#[test]
fn test_overall_status_is_worst() {
    let state = HealthState::new(default_config());

    // Set buffers to different status levels
    state.update_io_buffer_kb(251); // critical (~98%)
    state.update_smaps_buffer_kb(420); // warn (~82%)
    state.update_smaps_rollup_buffer_kb(50); // ok (~19.5%)

    let response = state.get_health();

    // Overall should be the worst (critical)
    assert_eq!(response.overall_status, "critical");

    // Verify individual statuses
    let io = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .unwrap();
    let smaps = response
        .buffers
        .iter()
        .find(|b| b.name == "smaps_buffer_kb")
        .unwrap();
    let rollup = response
        .buffers
        .iter()
        .find(|b| b.name == "smaps_rollup_buffer_kb")
        .unwrap();

    assert_eq!(io.status, "critical");
    assert_eq!(smaps.status, "warn");
    assert_eq!(rollup.status, "ok");
}

#[test]
fn test_json_serialization() {
    let state = HealthState::new(default_config());
    state.update_io_buffer_kb(100);
    state.update_smaps_buffer_kb(200);
    state.update_smaps_rollup_buffer_kb(50);

    let response = state.get_health();
    let json = serde_json::to_string(&response).expect("Failed to serialize");

    // Verify the JSON contains expected fields
    assert!(json.contains("\"buffers\""));
    assert!(json.contains("\"overall_status\""));
    assert!(json.contains("\"io_buffer_kb\""));
    assert!(json.contains("\"smaps_buffer_kb\""));
    assert!(json.contains("\"smaps_rollup_buffer_kb\""));
    assert!(json.contains("\"fill_percent\""));
    assert!(json.contains("\"larger_is_better\""));
}

#[test]
fn test_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let state = Arc::new(HealthState::new(default_config()));
    let mut handles = vec![];

    // Spawn multiple threads to update and read health state
    for i in 0..10 {
        let state_clone = Arc::clone(&state);
        let handle = thread::spawn(move || {
            state_clone.update_io_buffer_kb(i * 10);
            state_clone.update_smaps_buffer_kb(i * 20);
            state_clone.update_smaps_rollup_buffer_kb(i * 5);

            // Read health state
            let response = state_clone.get_health();
            assert_eq!(response.buffers.len(), 3);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Final state should be valid
    let final_response = state.get_health();
    assert_eq!(final_response.buffers.len(), 3);
}

#[test]
fn test_boundary_conditions() {
    // Test exact threshold values
    let config = custom_io_config(100, false, Some(80.0), Some(95.0));
    let state = HealthState::new(config);

    // Exactly at warn threshold (80%)
    state.update_io_buffer_kb(80);
    let response = state.get_health();
    let io = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .unwrap();
    // At exactly 80%, should still be ok (threshold is >)
    assert_eq!(io.status, "ok");

    // Just above warn threshold
    state.update_io_buffer_kb(81);
    let response = state.get_health();
    let io = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .unwrap();
    assert_eq!(io.status, "warn");
}

#[test]
fn test_larger_is_better_boundary() {
    // Test boundary for larger_is_better mode
    let config = custom_io_config(100, true, Some(30.0), Some(10.0));
    let state = HealthState::new(config);

    // Exactly at warn threshold (30%)
    state.update_io_buffer_kb(30);
    let response = state.get_health();
    let io = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .unwrap();
    // At exactly 30%, should still be ok (threshold is <)
    assert_eq!(io.status, "ok");

    // Just below warn threshold
    state.update_io_buffer_kb(29);
    let response = state.get_health();
    let io = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .unwrap();
    assert_eq!(io.status, "warn");
}

#[test]
fn test_mixed_larger_is_better() {
    // Test configuration where different buffers have different larger_is_better settings
    let config = AppConfig {
        io_buffer: BufferHealthConfig {
            capacity_kb: 100,
            larger_is_better: true, // Cache-like buffer
            warn_percent: Some(30.0),
            critical_percent: Some(10.0),
        },
        smaps_buffer: BufferHealthConfig {
            capacity_kb: 100,
            larger_is_better: false, // Overflow-like buffer
            warn_percent: Some(80.0),
            critical_percent: Some(95.0),
        },
        smaps_rollup_buffer: BufferHealthConfig::default(),
    };

    let state = HealthState::new(config);

    // io_buffer: 50% is ok (larger_is_better=true, above 30%)
    // smaps_buffer: 50% is ok (larger_is_better=false, below 80%)
    state.update_io_buffer_kb(50);
    state.update_smaps_buffer_kb(50);
    state.update_smaps_rollup_buffer_kb(50);

    let response = state.get_health();

    let io = response
        .buffers
        .iter()
        .find(|b| b.name == "io_buffer_kb")
        .unwrap();
    let smaps = response
        .buffers
        .iter()
        .find(|b| b.name == "smaps_buffer_kb")
        .unwrap();

    assert_eq!(io.status, "ok");
    assert!(io.larger_is_better);
    assert_eq!(smaps.status, "ok");
    assert!(!smaps.larger_is_better);
}
