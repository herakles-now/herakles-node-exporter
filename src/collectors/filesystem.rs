//! Filesystem statistics collector.
//!
//! This module provides functionality to read filesystem statistics from mounted filesystems
//! and expose them as Prometheus metrics.

use std::fs;

/// Filesystem statistics for a single mount point.
#[derive(Debug, Clone)]
pub struct FilesystemStats {
    pub device: String,
    pub mount_point: String,
    pub fstype: String,
    pub size_bytes: u64,
    pub available_bytes: u64,
    #[allow(dead_code)] // Calculated internally for diagnostics
    pub used_bytes: u64,
    pub files_total: u64,
    pub files_free: u64,
}

/// Reads filesystem statistics from /proc/mounts and uses libc statvfs to get usage data.
///
/// Returns a Vec of filesystem statistics for each mounted filesystem.
pub fn read_filesystem_stats() -> Result<Vec<FilesystemStats>, String> {
    let mounts_content = fs::read_to_string("/proc/mounts")
        .map_err(|e| format!("Failed to read /proc/mounts: {}", e))?;

    let mut stats = Vec::new();

    for line in mounts_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let device = parts[0].to_string();
        let mount_point = parts[1].to_string();
        let fstype = parts[2].to_string();

        // Skip pseudo filesystems and irrelevant mount points
        if should_skip_filesystem(&fstype, &mount_point) {
            continue;
        }

        // Get filesystem stats using statvfs
        match get_statvfs_stats(&mount_point) {
            Ok((size, available, used, files_total, files_free)) => {
                stats.push(FilesystemStats {
                    device,
                    mount_point,
                    fstype,
                    size_bytes: size,
                    available_bytes: available,
                    used_bytes: used,
                    files_total,
                    files_free,
                });
            }
            Err(_) => continue, // Skip filesystems we can't stat
        }
    }

    Ok(stats)
}

/// Checks if a filesystem should be skipped based on type and mount point.
#[allow(dead_code)] // Used internally by read_filesystem_stats
fn should_skip_filesystem(fstype: &str, mount_point: &str) -> bool {
    // Skip pseudo/virtual filesystems
    let skip_types = [
        "proc",
        "sysfs",
        "devpts",
        "devtmpfs",
        "tmpfs",
        "cgroup",
        "cgroup2",
        "pstore",
        "bpf",
        "debugfs",
        "tracefs",
        "fusectl",
        "configfs",
        "securityfs",
        "hugetlbfs",
        "mqueue",
        "autofs",
        "binfmt_misc",
    ];

    if skip_types.contains(&fstype) {
        return true;
    }

    // Skip system mount points
    if mount_point.starts_with("/proc")
        || mount_point.starts_with("/sys")
        || mount_point.starts_with("/dev")
        || mount_point.starts_with("/run")
    {
        return true;
    }

    false
}

/// Gets filesystem statistics using libc statvfs.
#[allow(dead_code)] // Used internally by read_filesystem_stats
fn get_statvfs_stats(path: &str) -> Result<(u64, u64, u64, u64, u64), String> {
    use std::ffi::CString;
    use std::mem;

    let c_path = CString::new(path).map_err(|e| format!("Invalid path: {}", e))?;

    unsafe {
        let mut stat: libc::statvfs = mem::zeroed();
        let result = libc::statvfs(c_path.as_ptr(), &mut stat);

        if result != 0 {
            return Err(format!("statvfs failed for {}", path));
        }

        let block_size = stat.f_frsize as u64;
        let total_blocks = stat.f_blocks;
        let free_blocks = stat.f_bfree;
        let available_blocks = stat.f_bavail;

        let size_bytes = block_size * total_blocks;
        let available_bytes = block_size * available_blocks;
        let used_bytes = size_bytes - (block_size * free_blocks);

        let files_total = stat.f_files;
        let files_free = stat.f_ffree;

        Ok((
            size_bytes,
            available_bytes,
            used_bytes,
            files_total,
            files_free,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_filesystem_stats() {
        let result = read_filesystem_stats();
        assert!(
            result.is_ok(),
            "Failed to read filesystem stats: {:?}",
            result
        );

        let stats = result.unwrap();
        // Should have at least one filesystem (root)
        assert!(!stats.is_empty(), "No filesystem statistics found");

        // Check that root filesystem is present
        let has_root = stats.iter().any(|fs| fs.mount_point == "/");
        assert!(has_root, "Root filesystem not found");
    }

    #[test]
    fn test_should_skip_filesystem() {
        assert!(should_skip_filesystem("proc", "/proc"));
        assert!(should_skip_filesystem("sysfs", "/sys"));
        assert!(should_skip_filesystem("tmpfs", "/dev/shm"));
        assert!(!should_skip_filesystem("ext4", "/"));
        assert!(!should_skip_filesystem("xfs", "/data"));
    }
}
