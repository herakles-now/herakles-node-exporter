//! Startup requirement validation for herakles-node-exporter.
//!
//! This module validates that the exporter has all necessary permissions
//! and system requirements before starting.

use nix::unistd::geteuid;
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// Validate all runtime requirements
pub fn validate_requirements(enable_ebpf: bool) -> Result<(), ValidationError> {
    info!("🔍 Validating runtime requirements...");

    check_user_privileges()?;
    check_proc_access()?;
    
    if enable_ebpf {
        check_ebpf_requirements()?;
    }

    info!("✅ All runtime requirements validated");
    Ok(())
}

/// Check if running with sufficient privileges
fn check_user_privileges() -> Result<(), ValidationError> {
    if !geteuid().is_root() {
        warn!("⚠️  Not running as root - may not be able to read all processes");
        warn!("   Recommendation: Run as root for full system monitoring");
        // Not an error - continue but warn
    } else {
        info!("✅ Running as root (uid=0)");
    }
    Ok(())
}

/// Check /proc filesystem access for root processes
fn check_proc_access() -> Result<(), ValidationError> {
    // Test reading /proc/1/smaps_rollup (systemd/init)
    let test_file = "/proc/1/smaps_rollup";
    
    // Use metadata to check accessibility without reading the whole file
    match fs::metadata(test_file) {
        Ok(_) => {
            info!("✅ /proc access: Can read all processes");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            error!("❌ Cannot read {} - insufficient permissions", test_file);
            error!("   This means only user-owned processes will be monitored!");
            error!("");
            error!("   Solutions:");
            error!("   1. Run as root:");
            error!("      sudo systemctl edit herakles-node-exporter");
            error!("      Then add these lines in the editor:");
            error!("      [Service]");
            error!("      User=root");
            error!("      Group=root");
            error!("");
            error!("   2. Grant capabilities:");
            error!("      setcap cap_dac_read_search,cap_sys_ptrace+ep /path/to/binary");
            Err(ValidationError::InsufficientPermissions(e.to_string()))
        }
        Err(e) => {
            warn!("⚠️  Could not test /proc access: {}", e);
            Ok(()) // Continue but warn
        }
    }
}

/// Check eBPF requirements
fn check_ebpf_requirements() -> Result<(), ValidationError> {
    debug!("Checking eBPF requirements...");

    // Check BPF filesystem
    if !Path::new("/sys/fs/bpf").exists() {
        error!("❌ /sys/fs/bpf not found - BPF filesystem not mounted");
        error!("   Solution: mount -t bpf bpf /sys/fs/bpf");
        return Err(ValidationError::BpfFsNotMounted);
    }

    // Check if /sys/fs/bpf is writable by checking metadata and permissions
    match fs::metadata("/sys/fs/bpf") {
        Ok(metadata) => {
            use std::os::unix::fs::PermissionsExt;
            let perms = metadata.permissions();
            let mode = perms.mode();
            // Check if writable by owner (we're running as root)
            if mode & 0o200 == 0 {
                error!("❌ /sys/fs/bpf is not writable");
                error!("   eBPF map pinning will fail!");
                return Err(ValidationError::BpfFsNotWritable(
                    "Filesystem is read-only".to_string(),
                ));
            }
            info!("✅ /sys/fs/bpf is accessible and writable");
        }
        Err(e) => {
            error!("❌ Cannot access /sys/fs/bpf: {}", e);
            return Err(ValidationError::BpfFsNotWritable(e.to_string()));
        }
    }

    // Check BTF support
    if !Path::new("/sys/kernel/btf/vmlinux").exists() {
        warn!("⚠️  /sys/kernel/btf/vmlinux not found - BTF support missing");
        warn!("   eBPF may not work. Install linux-headers or enable CONFIG_DEBUG_INFO_BTF");
        // Not fatal - eBPF will try and fail gracefully
    } else {
        info!("✅ BTF support available");
    }

    // Check kernel version
    if let Ok(version) = fs::read_to_string("/proc/version") {
        debug!("Kernel version: {}", version.lines().next().unwrap_or("unknown"));
    }

    info!("✅ eBPF requirements validated");
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
    
    #[error("BPF filesystem not mounted at /sys/fs/bpf")]
    BpfFsNotMounted,
    
    #[error("BPF filesystem not writable: {0}")]
    BpfFsNotWritable(String),
}
