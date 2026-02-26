//! System-wide installation command for herakles-node-exporter.
//!
//! This module implements the `install` subcommand which sets up:
//! - System user and group (herakles)
//! - Directory structure with proper permissions
//! - Binary installation to /opt/herakles/bin
//! - Default configuration file
//! - systemd service with eBPF capabilities
//! - Automatic service enablement and start

use crate::config::Config;
use nix::unistd::{chown, Gid, Uid};
use serde_yaml;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

/// systemd service unit template for herakles-node-exporter
///
/// The service runs as root to ensure full /proc access and eBPF capabilities.
/// This allows monitoring of all processes (including root-owned) and proper
/// eBPF program loading.
///
/// CRITICAL FIX: SystemCallFilter and SeccompProfile are explicitly disabled
/// to prevent SIGSYS (Signal 31) crashes when loading eBPF programs. The bpf()
/// syscall is essential for eBPF functionality and was previously blocked by
/// implicit seccomp filters activated by ReadWritePaths=.
///
/// Security note: Since eBPF requires elevated privileges, security should be
/// enforced at the deployment level through network isolation, host hardening,
/// and proper access controls rather than systemd restrictions.
const SYSTEMD_UNIT: &str = r#"[Unit]
Description=Herakles Node Exporter
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root

# CRITICAL: Disable SystemCallFilter to allow bpf() syscall for eBPF
# Without these settings, the process crashes with SIGSYS (Signal 31)
SystemCallFilter=
SeccompProfile=
NoNewPrivileges=no

# Verify and re-apply kernel parameters before starting
# The -q flag makes sysctl quiet, but it still sets the parameters and will
# fail (preventing service start) if the parameters cannot be set.
# This ensures parameters are always correct even if they were changed.
ExecStartPre=/usr/sbin/sysctl -q kernel.unprivileged_bpf_disabled=1
ExecStartPre=/usr/sbin/sysctl -q kernel.perf_event_paranoid=2

# Create BPF directories with root ownership
ExecStartPre=/bin/mkdir -p /sys/fs/bpf/herakles/node
ExecStartPre=/bin/chmod 0755 /sys/fs/bpf/herakles
ExecStartPre=/bin/chmod 0755 /sys/fs/bpf/herakles/node

ExecStart=/opt/herakles/bin/herakles-node-exporter

Restart=on-failure
RestartSec=3

[Install]
WantedBy=multi-user.target
"#;

/// Main installation command handler
pub fn command_install(no_service: bool, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Herakles Node Exporter - System Installation");
    println!("===============================================\n");

    // 1. Root-Check
    if !is_root() {
        eprintln!("❌ Installation requires root privileges");
        eprintln!("   Run with: sudo herakles-node-exporter install");
        std::process::exit(1);
    }

    // 2. Check if already installed (when not --force)
    if !force && Path::new("/opt/herakles/bin/herakles-node-exporter").exists() {
        eprintln!("⚠️  Herakles already installed. Use --force to reinstall.");
        std::process::exit(1);
    }

    // 3. Skip user & group creation - not needed for root operation
    println!("👤 Skipping user creation - will run as root for full /proc access");

    // 4. Create directory structure
    println!("📁 Creating directory structure...");
    create_directories()?;

    // 5. Copy binary
    println!("📦 Installing binary...");
    install_binary()?;

    // 6. Generate default config
    println!("⚙️  Generating default configuration...");
    generate_default_config()?;

    // 7. Install systemd service
    if !no_service {
        println!("🔧 Installing systemd service...");
        install_systemd_service()?;

        println!("🔄 Reloading systemd...");
        systemd_daemon_reload()?;

        println!("✅ Enabling service...");
        systemd_enable_service()?;

        println!("🚀 Starting service...");
        systemd_start_service()?;
    }

    // 8. Configure kernel parameters
    configure_kernel_parameters()?;

    println!("\n✅ Installation complete!");
    println!("\nNext steps:");
    println!("  • Check status: systemctl status herakles-node-exporter");
    println!("  • View logs:    journalctl -u herakles-node-exporter -f");
    println!("  • Access:       http://localhost:9215/metrics");

    Ok(())
}

/// Check if the current process is running as root
fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

/// Create required directory structure with proper permissions
fn create_directories() -> Result<(), Box<dyn std::error::Error>> {
    let dirs = [
        "/opt/herakles/bin",
        "/etc/herakles",
        "/var/lib/herakles/ebpf",
        "/var/lib/herakles/state",
        "/run/herakles",
        "/sys/fs/bpf/herakles/node",
    ];

    for dir in &dirs {
        fs::create_dir_all(dir)?;
    }

    // Set ownership: root:root and permissions: 0755
    for dir in [
        "/var/lib/herakles",
        "/run/herakles",
        "/sys/fs/bpf/herakles",
        "/sys/fs/bpf/herakles/node",
    ] {
        chown(dir, Some(Uid::from_raw(0)), Some(Gid::from_raw(0)))?;
        set_permissions(dir, 0o755)?;
    }

    println!("   ✅ Directory structure created with root ownership");
    Ok(())
}

/// Install the binary to /opt/herakles/bin
fn install_binary() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;
    let target = "/opt/herakles/bin/herakles-node-exporter";

    fs::copy(&current_exe, target)?;
    set_permissions(target, 0o755)?;

    println!("   ✅ Binary installed to {}", target);
    Ok(())
}

/// Generate default configuration file
fn generate_default_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let yaml = serde_yaml::to_string(&config)?;

    fs::write("/etc/herakles/herakles-node-exporter.yaml", yaml)?;
    set_permissions("/etc/herakles/herakles-node-exporter.yaml", 0o644)?;

    println!("   ✅ Config written to /etc/herakles/herakles-node-exporter.yaml");
    Ok(())
}

/// Install systemd service unit file
fn install_systemd_service() -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        "/etc/systemd/system/herakles-node-exporter.service",
        SYSTEMD_UNIT,
    )?;
    println!("   ✅ systemd unit installed");
    Ok(())
}

/// Reload systemd daemon to pick up new service file
fn systemd_daemon_reload() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("systemctl").arg("daemon-reload").status()?;
    Ok(())
}

/// Enable the herakles-node-exporter service
fn systemd_enable_service() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("systemctl")
        .args(["enable", "herakles-node-exporter.service"])
        .status()?;
    Ok(())
}

/// Start the herakles-node-exporter service
fn systemd_start_service() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("systemctl")
        .args(["start", "herakles-node-exporter.service"])
        .status()?;
    Ok(())
}

/// Configure kernel parameters for eBPF and persist them
fn configure_kernel_parameters() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 Configuring kernel parameters for eBPF...");

    let params = [
        ("kernel.unprivileged_bpf_disabled", "1"),
        ("kernel.perf_event_paranoid", "2"),
    ];

    let mut all_ok = true;

    // Set runtime parameters
    for (key, value) in &params {
        match set_sysctl(key, value) {
            Ok(_) => println!("   ✅ {} = {}", key, value),
            Err(e) => {
                eprintln!("   ❌ Failed to set {}: {}", key, e);
                all_ok = false;
            }
        }
    }

    // Persist to /etc/sysctl.d/99-herakles-ebpf.conf
    let sysctl_config = format!(
        "# Kernel parameters for Herakles Node Exporter eBPF\n\
         # Generated by herakles-node-exporter installer\n\
         \n\
         kernel.unprivileged_bpf_disabled = 1\n\
         kernel.perf_event_paranoid = 2\n"
    );

    match fs::write("/etc/sysctl.d/99-herakles-ebpf.conf", sysctl_config) {
        Ok(_) => println!("   ✅ Persistent configuration written to /etc/sysctl.d/99-herakles-ebpf.conf"),
        Err(e) => {
            eprintln!("   ❌ Failed to write persistent config: {}", e);
            all_ok = false;
        }
    }

    if !all_ok {
        eprintln!("\n⚠️  Some kernel parameters could not be configured.");
        eprintln!("   eBPF may not work correctly. Manual configuration required:");
        eprintln!("   • sudo sysctl -w kernel.unprivileged_bpf_disabled=1");
        eprintln!("   • sudo sysctl -w kernel.perf_event_paranoid=2");
    }

    Ok(())
}

/// Set a sysctl parameter at runtime
fn set_sysctl(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("sysctl")
        .args(["-w", &format!("{}={}", key, value)])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "sysctl command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

/// Set file permissions using Unix mode
fn set_permissions(path: &str, mode: u32) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_root() {
        // Just testing that the function is callable
        // Result depends on whether test is run as root
        let _ = is_root();
    }

    #[test]
    fn test_systemd_unit_format() {
        // Verify systemd unit has expected sections
        assert!(SYSTEMD_UNIT.contains("[Unit]"));
        assert!(SYSTEMD_UNIT.contains("[Service]"));
        assert!(SYSTEMD_UNIT.contains("[Install]"));
        assert!(SYSTEMD_UNIT.contains("User=root"));
        assert!(SYSTEMD_UNIT.contains("Group=root"));
        assert!(SYSTEMD_UNIT.contains("/opt/herakles/bin/herakles-node-exporter"));
        assert!(SYSTEMD_UNIT.contains("/sys/fs/bpf/herakles"));
        
        // CRITICAL: Ensure SystemCallFilter is explicitly disabled for eBPF
        // Verify that these directives are set to empty values (which disables them)
        assert!(SYSTEMD_UNIT.contains("SystemCallFilter=\n"));
        assert!(SYSTEMD_UNIT.contains("SeccompProfile=\n"));
        assert!(SYSTEMD_UNIT.contains("NoNewPrivileges=no"));
        
        // Ensure ReadWritePaths is removed (it triggers implicit seccomp filters)
        assert!(!SYSTEMD_UNIT.contains("ReadWritePaths"));
        
        // Ensure kernel parameter verification is present
        assert!(SYSTEMD_UNIT.contains("kernel.unprivileged_bpf_disabled=1"));
        assert!(SYSTEMD_UNIT.contains("kernel.perf_event_paranoid=2"));
    }
}
