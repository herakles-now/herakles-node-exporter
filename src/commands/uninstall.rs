//! System-wide uninstallation command for herakles-node-exporter.
//!
//! This module implements the `uninstall` subcommand which removes:
//! - systemd service (stop, disable, remove unit file)
//! - Installed binary from /opt/herakles/bin
//! - CLI symlink from /usr/local/bin
//! - Configuration file from /etc/herakles
//! - Directory structure with proper safety checks
//!
//! Note: The installer does not create a dedicated system user (the service
//! runs as root for eBPF), so there is no user to remove.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

/// Main uninstallation command handler
pub fn command_uninstall(skip_confirm: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️  Herakles Node Exporter - System Uninstallation");
    println!("=================================================\n");

    // 1. Root-Check
    if !is_root() {
        eprintln!("❌ Uninstallation requires root privileges");
        eprintln!("   Run with: sudo herakles-node-exporter uninstall");
        std::process::exit(1);
    }

    // 2. Check if actually installed
    if !Path::new("/opt/herakles/bin/herakles-node-exporter").exists() {
        eprintln!("⚠️  Herakles does not appear to be installed.");
        eprintln!("   Binary not found at: /opt/herakles/bin/herakles-node-exporter");
        std::process::exit(1);
    }

    // 3. Confirmation prompt (unless --yes)
    if !skip_confirm {
        println!("⚠️  This will remove:");
        println!("   • systemd service (stopped and disabled)");
        println!("   • Binary: /opt/herakles/bin/herakles-node-exporter");
        println!("   • CLI symlink: /usr/local/bin/herakles-node-exporter");
        println!("   • Configuration: /etc/herakles/");
        println!("   • Directories: /opt/herakles/, /var/lib/herakles/, /run/herakles/");
        println!("   • BPF maps: /sys/fs/bpf/herakles/");
        println!("   • Kernel parameter config: /etc/sysctl.d/99-herakles-ebpf.conf");
        println!("\nAre you sure you want to continue? (yes/no): ");

        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "yes" && input != "y" {
            println!("❌ Uninstallation cancelled.");
            std::process::exit(0);
        }
    }

    println!("\n🚀 Starting uninstallation...\n");

    // 4. Stop and disable systemd service
    if service_exists() {
        println!("🛑 Stopping systemd service...");
        stop_systemd_service();

        println!("❌ Disabling systemd service...");
        disable_systemd_service();

        println!("🗑️  Removing systemd service file...");
        remove_systemd_service()?;

        println!("🔄 Reloading systemd...");
        systemd_daemon_reload()?;
    } else {
        println!("ℹ️  systemd service not found, skipping service removal");
    }

    // 5. Remove binary
    println!("🗑️  Removing binary...");
    remove_binary()?;

    // 6. Remove CLI symlink
    println!("🗑️  Removing CLI symlink...");
    remove_cli_symlink()?;

    // 7. Remove configuration
    println!("🗑️  Removing configuration...");
    remove_config()?;

    // 8. Remove directories
    println!("🗑️  Removing directories...");
    remove_directories()?;

    // 9. Remove kernel parameter configuration
    println!("🗑️  Removing kernel parameter configuration...");
    remove_sysctl_config()?;

    println!("\n✅ Uninstallation complete!");
    println!("   System has been returned to pre-installation state.");

    Ok(())
}

/// Check if the current process is running as root
fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

/// Check if the systemd service exists
fn service_exists() -> bool {
    Path::new("/etc/systemd/system/herakles-node-exporter.service").exists()
}

/// Stop the herakles-node-exporter service (ignore errors)
fn stop_systemd_service() {
    let result = Command::new("systemctl")
        .args(["stop", "herakles-node-exporter.service"])
        .status();

    match result {
        Ok(status) if status.success() => {
            println!("   ✅ Service stopped");
        }
        _ => {
            println!("   ⚠️  Failed to stop service (may not be running)");
        }
    }
}

/// Disable the herakles-node-exporter service (ignore errors)
fn disable_systemd_service() {
    let result = Command::new("systemctl")
        .args(["disable", "herakles-node-exporter.service"])
        .status();

    match result {
        Ok(status) if status.success() => {
            println!("   ✅ Service disabled");
        }
        _ => {
            println!("   ⚠️  Failed to disable service (may not be enabled)");
        }
    }
}

/// Remove the systemd service unit file
fn remove_systemd_service() -> Result<(), Box<dyn std::error::Error>> {
    let service_path = "/etc/systemd/system/herakles-node-exporter.service";

    if Path::new(service_path).exists() {
        fs::remove_file(service_path)?;
        println!("   ✅ Service file removed");
    } else {
        println!("   ℹ️  Service file not found, skipping");
    }

    Ok(())
}

/// Reload systemd daemon
fn systemd_daemon_reload() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("systemctl").arg("daemon-reload").status()?;
    println!("   ✅ systemd reloaded");
    Ok(())
}

/// Remove the binary from /opt/herakles/bin
fn remove_binary() -> Result<(), Box<dyn std::error::Error>> {
    let binary_path = "/opt/herakles/bin/herakles-node-exporter";

    if Path::new(binary_path).exists() {
        fs::remove_file(binary_path)?;
        println!("   ✅ Binary removed: {}", binary_path);
    } else {
        println!("   ⚠️  Binary not found, skipping");
    }

    Ok(())
}

/// Remove the CLI symlink from /usr/local/bin
fn remove_cli_symlink() -> Result<(), Box<dyn std::error::Error>> {
    let symlink_path = "/usr/local/bin/herakles-node-exporter";

    if Path::new(symlink_path).exists() {
        fs::remove_file(symlink_path)?;
        println!("   ✅ CLI symlink removed: {}", symlink_path);
    } else {
        println!("   ℹ️  CLI symlink not found, skipping");
    }

    Ok(())
}

/// Remove configuration directory and files
fn remove_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = "/etc/herakles";

    if Path::new(config_dir).exists() {
        fs::remove_dir_all(config_dir)?;
        println!("   ✅ Configuration removed: {}", config_dir);
    } else {
        println!("   ℹ️  Configuration directory not found, skipping");
    }

    Ok(())
}

/// Remove all installation directories
fn remove_directories() -> Result<(), Box<dyn std::error::Error>> {
    // Note: These are parent directories that will recursively remove all contents
    // e.g., /sys/fs/bpf/herakles will remove /sys/fs/bpf/herakles/node as well
    let dirs = [
        "/opt/herakles",
        "/var/lib/herakles",
        "/run/herakles",
        "/sys/fs/bpf/herakles",
    ];

    for dir in &dirs {
        if Path::new(dir).exists() {
            match fs::remove_dir_all(dir) {
                Ok(_) => println!("   ✅ Removed: {}", dir),
                Err(e) => {
                    println!("   ⚠️  Failed to remove {}: {} (continuing anyway)", dir, e);
                }
            }
        } else {
            println!("   ℹ️  Directory not found: {} (skipping)", dir);
        }
    }

    Ok(())
}

/// Remove the persistent sysctl configuration
fn remove_sysctl_config() -> Result<(), Box<dyn std::error::Error>> {
    let sysctl_path = "/etc/sysctl.d/99-herakles-ebpf.conf";

    if Path::new(sysctl_path).exists() {
        fs::remove_file(sysctl_path)?;
        println!("   ✅ Sysctl configuration removed: {}", sysctl_path);
        println!("   ℹ️  Note: Kernel parameters remain active until reboot");
        println!("   To reset to system defaults immediately, run:");
        // Note: These are typical Linux kernel defaults:
        // - unprivileged_bpf_disabled=2 (more restrictive, unprivileged access disabled)
        // - perf_event_paranoid=4 (paranoid mode, restricts performance monitoring)
        println!("      • sudo sysctl -w kernel.unprivileged_bpf_disabled=2");
        println!("      • sudo sysctl -w kernel.perf_event_paranoid=4");
    } else {
        println!("   ℹ️  Sysctl configuration not found, skipping");
    }

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
    fn test_service_exists() {
        // Test that the function is callable (result depends on system state)
        let _ = service_exists();
    }
}
