//! Integration tests for TLS configuration validation.
//!
//! These tests verify the behavior of TLS configuration validation.

use std::io::Write;
use tempfile::NamedTempFile;

/// Helper to get the binary path
fn binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_herakles-node-exporter"))
}

#[test]
fn test_tls_enabled_without_paths() {
    let output = std::process::Command::new(binary_path())
        .args(["--enable-tls", "--check-config"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(
        stdout.contains("TLS is enabled but neither tls_cert_path nor tls_key_path are set")
            || stderr.contains("TLS is enabled but neither tls_cert_path nor tls_key_path are set"),
        "Expected error about missing TLS paths, got stdout: '{}', stderr: '{}'",
        stdout,
        stderr
    );
}

#[test]
fn test_tls_enabled_with_cert_only() {
    let output = std::process::Command::new(binary_path())
        .args([
            "--enable-tls",
            "--tls-cert",
            "/some/path.pem",
            "--check-config",
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(
        stdout.contains("TLS is enabled but tls_key_path is not set")
            || stderr.contains("TLS is enabled but tls_key_path is not set"),
        "Expected error about missing key path, got stdout: '{}', stderr: '{}'",
        stdout,
        stderr
    );
}

#[test]
fn test_tls_enabled_with_key_only() {
    let output = std::process::Command::new(binary_path())
        .args([
            "--enable-tls",
            "--tls-key",
            "/some/path.pem",
            "--check-config",
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(
        stdout.contains("TLS is enabled but tls_cert_path is not set")
            || stderr.contains("TLS is enabled but tls_cert_path is not set"),
        "Expected error about missing cert path, got stdout: '{}', stderr: '{}'",
        stdout,
        stderr
    );
}

#[test]
fn test_tls_enabled_with_nonexistent_files() {
    let output = std::process::Command::new(binary_path())
        .args([
            "--enable-tls",
            "--tls-cert",
            "/nonexistent/cert.pem",
            "--tls-key",
            "/nonexistent/key.pem",
            "--check-config",
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(
        stdout.contains("TLS certificate file not found")
            || stderr.contains("TLS certificate file not found"),
        "Expected error about missing cert file, got stdout: '{}', stderr: '{}'",
        stdout,
        stderr
    );
}

#[test]
fn test_tls_enabled_with_valid_files() {
    // Create temporary certificate and key files
    let mut cert_file = NamedTempFile::new().expect("Failed to create temp cert file");
    let mut key_file = NamedTempFile::new().expect("Failed to create temp key file");

    // Write some dummy content (doesn't need to be valid for check-config)
    writeln!(
        cert_file,
        "-----BEGIN CERTIFICATE-----\nDUMMY\n-----END CERTIFICATE-----"
    )
    .expect("Failed to write cert");
    cert_file.flush().expect("Failed to flush cert file");

    writeln!(
        key_file,
        "-----BEGIN PRIVATE KEY-----\nDUMMY\n-----END PRIVATE KEY-----"
    )
    .expect("Failed to write key");
    key_file.flush().expect("Failed to flush key file");

    let cert_path = cert_file.path().to_str().unwrap();
    let key_path = key_file.path().to_str().unwrap();

    let output = std::process::Command::new(binary_path())
        .args([
            "--enable-tls",
            "--tls-cert",
            cert_path,
            "--tls-key",
            key_path,
            "--check-config",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print diagnostic info if the command fails
    if !output.status.success() {
        eprintln!("Command failed with status: {:?}", output.status);
        eprintln!("stdout:\n{}", stdout);
        eprintln!("stderr:\n{}", stderr);
    }

    assert!(
        output.status.success(),
        "Expected config validation to pass with valid files\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    assert!(
        stdout.contains("Configuration is valid"),
        "Expected success message\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_tls_disabled_by_default() {
    let output = std::process::Command::new(binary_path())
        .args(["--show-config"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains("enable_tls: false"),
        "Expected TLS to be disabled by default, got: '{}'",
        stdout
    );
}

#[test]
fn test_tls_config_in_show_config() {
    let output = std::process::Command::new(binary_path())
        .args(["--show-config"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains("enable_tls:"),
        "Expected enable_tls in config output"
    );
    assert!(
        stdout.contains("tls_cert_path:"),
        "Expected tls_cert_path in config output"
    );
    assert!(
        stdout.contains("tls_key_path:"),
        "Expected tls_key_path in config output"
    );
}
