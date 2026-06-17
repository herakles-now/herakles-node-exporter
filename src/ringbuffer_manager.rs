use crate::config::RingbufferConfig;
use crate::ringbuffer::{Ringbuffer, RingbufferEntry, ENTRY_SIZE_BYTES};
use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicI64, Ordering};
use tracing::{info, warn};

/// Statistics about the ringbuffer system.
#[derive(Debug, Clone, Serialize)]
pub struct RingbufferStats {
    pub max_memory_mb: usize,
    pub entry_size_bytes: usize,
    pub interval_seconds: u64,
    pub entries_per_subgroup: usize,
    pub total_subgroups: usize,
    pub estimated_ram_bytes: usize,
    pub history_seconds: u64,
    pub db_enabled: bool,
    pub db_path: Option<String>,
    pub db_entries: usize,
    pub db_size_bytes: u64,
}

/// Manager for multiple ringbuffers, one per subgroup.
pub struct RingbufferManager {
    buffers: DashMap<String, Ringbuffer>,
    entries_per_subgroup: usize,
    interval_seconds: u64,
    config: RingbufferConfig,
    estimated_ram_bytes: usize,
    db: Option<sled::Db>,
    last_prune: AtomicI64,
}

impl RingbufferManager {
    /// Creates a new ringbuffer manager.
    pub fn new(config: RingbufferConfig, initial_subgroup_count: usize) -> Self {
        // Calculate maximum total entries based on memory budget
        let max_bytes = config.max_memory_mb * 1024 * 1024;
        let max_total_entries = max_bytes / ENTRY_SIZE_BYTES;

        // Calculate entries per subgroup
        let subgroup_count = initial_subgroup_count.max(1); // Prevent division by zero
        let calculated_entries = max_total_entries / subgroup_count;

        // Clamp to configured min/max
        let mut entries_per_subgroup = calculated_entries
            .max(config.min_entries_per_subgroup)
            .min(config.max_entries_per_subgroup);

        // If database is enabled, we keep the in-memory capacity at the minimum to save RAM,
        // since the full history is read directly from the sled database on demand.
        if config.enable_database {
            entries_per_subgroup = config.min_entries_per_subgroup;
        }

        // Estimate actual RAM usage
        let estimated_ram_bytes = entries_per_subgroup * ENTRY_SIZE_BYTES * subgroup_count;

        // Open sled database if enabled
        let db = if config.enable_database {
            if let Some(parent) = config.database_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    warn!(
                        "Failed to create database directory {}: {}. Falling back to in-memory only.",
                        parent.display(),
                        e
                    );
                }
            }

            match sled::open(&config.database_path) {
                Ok(database) => {
                    info!(
                        "Persistent sled database opened at: {}",
                        config.database_path.display()
                    );
                    Some(database)
                }
                Err(e) => {
                    warn!(
                        "Failed to open database at {}: {}. Falling back to in-memory only.",
                        config.database_path.display(),
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        Self {
            buffers: DashMap::new(),
            entries_per_subgroup,
            interval_seconds: config.interval_seconds,
            config,
            estimated_ram_bytes,
            db,
            last_prune: AtomicI64::new(0),
        }
    }

    /// Records a metric entry for a specific subgroup.
    pub fn record(&self, subgroup: &str, entry: RingbufferEntry) {
        // 1. Push to in-memory ringbuffer (keeps a small recent cache)
        self.buffers
            .entry(subgroup.to_string())
            .or_insert_with(|| Ringbuffer::new(self.entries_per_subgroup))
            .push(entry);

        // 2. Persist to database if enabled
        if let Some(ref db) = self.db {
            let mut key = Vec::with_capacity(subgroup.len() + 1 + 8);
            key.extend_from_slice(subgroup.as_bytes());
            key.push(b':');
            key.extend_from_slice(&entry.timestamp.to_be_bytes());

            if let Ok(val_bytes) = serde_json::to_vec(&entry) {
                if let Err(e) = db.insert(&key, val_bytes) {
                    warn!("Failed to insert entry into sled database: {}", e);
                }
            }
        }
    }

    /// Prunes database entries exceeding the retention limit.
    pub fn prune_database(&self, force: bool) -> Result<(), Box<dyn std::error::Error>> {
        let db = match &self.db {
            Some(db) => db,
            None => return Ok(()),
        };

        let now = chrono::Utc::now().timestamp();
        if !force {
            let last = self.last_prune.load(Ordering::Relaxed);
            if now - last < 300 {
                return Ok(());
            }
        }
        self.last_prune.store(now, Ordering::Relaxed);

        let limit = crate::config::parse_retention(&self.config.retention)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let current_len = db.len();

        match limit {
            crate::config::RetentionLimit::Duration(duration) => {
                let threshold = chrono::Utc::now().timestamp() - duration.as_secs() as i64;
                let mut to_remove = Vec::new();

                for item in db.iter() {
                    if let Ok((key, _)) = item {
                        // Key format: subgroup + ':' + timestamp (8 bytes)
                        if key.len() >= 9 {
                            let len = key.len();
                            let mut ts_bytes = [0u8; 8];
                            ts_bytes.copy_from_slice(&key[len - 8..]);
                            let timestamp = i64::from_be_bytes(ts_bytes);
                            if timestamp < threshold {
                                to_remove.push(key);
                            }
                        }
                    } else {
                        break;
                    }
                }

                for key in to_remove {
                    let _ = db.remove(&key);
                }
            }
            crate::config::RetentionLimit::Size(size_bytes) => {
                let max_entries = (size_bytes / 512) as usize;
                if current_len > max_entries {
                    let to_remove_count = current_len - max_entries;
                    let mut keys_with_ts = Vec::with_capacity(current_len);

                    for item in db.iter() {
                        if let Ok((key, _)) = item {
                            if key.len() >= 9 {
                                let len = key.len();
                                let mut ts_bytes = [0u8; 8];
                                ts_bytes.copy_from_slice(&key[len - 8..]);
                                let timestamp = i64::from_be_bytes(ts_bytes);
                                keys_with_ts.push((timestamp, key));
                            }
                        } else {
                            break;
                        }
                    }

                    // Sort oldest first
                    keys_with_ts.sort_by_key(|(ts, _)| *ts);

                    for (_, key) in keys_with_ts
                        .iter()
                        .take(to_remove_count.min(keys_with_ts.len()))
                    {
                        let _ = db.remove(key);
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns statistics about the ringbuffer system.
    pub fn get_stats(&self) -> RingbufferStats {
        let total_subgroups = self.get_all_subgroups().len();
        let history_seconds = self.entries_per_subgroup as u64 * self.interval_seconds;

        let db_entries = self.db.as_ref().map(|db| db.len()).unwrap_or(0);
        let db_size_bytes = if self.config.enable_database {
            get_dir_size(&self.config.database_path).unwrap_or(0)
        } else {
            0
        };

        RingbufferStats {
            max_memory_mb: self.config.max_memory_mb,
            entry_size_bytes: ENTRY_SIZE_BYTES,
            interval_seconds: self.interval_seconds,
            entries_per_subgroup: self.entries_per_subgroup,
            total_subgroups,
            estimated_ram_bytes: self.estimated_ram_bytes,
            history_seconds,
            db_enabled: self.config.enable_database,
            db_path: if self.config.enable_database {
                Some(self.config.database_path.to_string_lossy().into_owned())
            } else {
                None
            },
            db_entries,
            db_size_bytes,
        }
    }

    /// Returns the historical entries for a specific subgroup.
    ///
    /// If the persistent database is enabled, it reads directly from `sled` using prefix scan.
    /// Otherwise, it falls back to the in-memory ringbuffer.
    pub fn get_subgroup_history(&self, subgroup: &str) -> Option<Vec<RingbufferEntry>> {
        if let Some(ref db) = self.db {
            let mut prefix = subgroup.to_string();
            prefix.push(':');

            let mut entries = Vec::new();
            for (_, value) in db.scan_prefix(prefix.as_bytes()).flatten() {
                if let Ok(entry) = serde_json::from_slice::<RingbufferEntry>(&value) {
                    entries.push(entry);
                }
            }

            if entries.is_empty() {
                if self.buffers.contains_key(subgroup) {
                    Some(Vec::new())
                } else {
                    None
                }
            } else {
                Some(entries)
            }
        } else {
            self.buffers.get(subgroup).map(|rb| rb.get_history())
        }
    }

    /// Returns a list of all known subgroup names (from memory or DB).
    pub fn get_all_subgroups(&self) -> Vec<String> {
        let mut subgroups = std::collections::HashSet::new();

        // 1. Memory subgroups
        for entry in self.buffers.iter() {
            subgroups.insert(entry.key().clone());
        }

        // 2. Database subgroups
        if let Some(ref db) = self.db {
            for (key, _) in db.iter().flatten() {
                if let Some(pos) = key.iter().position(|&b| b == b':') {
                    if let Ok(subgroup) = std::str::from_utf8(&key[..pos]) {
                        subgroups.insert(subgroup.to_string());
                    }
                }
            }
        }

        subgroups.into_iter().collect()
    }

    /// Flushes all pending writes in the database to disk.
    pub fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref db) = self.db {
            db.flush()?;
            info!("Persistent sled database flushed successfully.");
        }
        Ok(())
    }
}

fn get_dir_size<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<u64> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(0);
    }
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    let mut size = 0;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            size += meta.len();
        } else if meta.is_dir() {
            size += get_dir_size(entry.path())?;
        }
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ringbuffer::TopProcessInfo;
    use std::path::PathBuf;

    fn default_config() -> RingbufferConfig {
        RingbufferConfig {
            max_memory_mb: 15,
            interval_seconds: 30,
            min_entries_per_subgroup: 10,
            max_entries_per_subgroup: 120,
            enable_database: false,
            database_path: PathBuf::from("/var/lib/herakles/metrics.db"),
            retention: "24h".to_string(),
        }
    }

    #[test]
    fn test_manager_initialization_small_subgroup_count() {
        // With 10 subgroups, should get max entries (120)
        let manager = RingbufferManager::new(default_config(), 10);
        let stats = manager.get_stats();

        assert_eq!(stats.max_memory_mb, 15);
        assert_eq!(stats.entry_size_bytes, 256); // Updated for new structure
        assert_eq!(stats.entries_per_subgroup, 120); // Capped at max
    }

    #[test]
    fn test_manager_initialization_large_subgroup_count() {
        // With 40000 subgroups: 15*1024*1024 / 256 / 40000 ≈ 1.5 entries
        // Should be clamped to min (10)
        let manager = RingbufferManager::new(default_config(), 40000);
        let stats = manager.get_stats();

        assert_eq!(stats.entries_per_subgroup, 10); // Capped at min
    }

    #[test]
    fn test_manager_initialization_medium_subgroup_count() {
        // With 5000 subgroups: 15*1024*1024 / 256 / 5000 ≈ 12 entries
        let manager = RingbufferManager::new(default_config(), 5000);
        let stats = manager.get_stats();

        // Should be between min and max, closer to min now due to larger entry size
        assert!(stats.entries_per_subgroup >= 10);
        assert!(stats.entries_per_subgroup <= 120);
        // With 256-byte entries, we get fewer entries per subgroup
        assert!(stats.entries_per_subgroup >= 10);
        assert!(stats.entries_per_subgroup < 15);
    }

    #[test]
    fn test_record_and_retrieve() {
        let manager = RingbufferManager::new(default_config(), 10);

        // Record an entry
        let entry = RingbufferEntry {
            timestamp: 1000,
            rss_kb: 100,
            pss_kb: 90,
            uss_kb: 80,
            cpu_percent: 5.0,
            cpu_time_seconds: 1.0,
            top_cpu: [TopProcessInfo::default(); 3],
            top_rss: [TopProcessInfo::default(); 3],
            top_pss: [TopProcessInfo::default(); 3],
            _padding: [],
        };

        manager.record("test_subgroup", entry);

        // Retrieve it
        let history = manager.get_subgroup_history("test_subgroup");
        assert!(history.is_some());

        let history = history.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].timestamp, 1000);
    }

    #[test]
    fn test_multiple_subgroups() {
        let manager = RingbufferManager::new(default_config(), 10);

        // Record entries for different subgroups
        for i in 0..3 {
            let entry = RingbufferEntry {
                timestamp: 1000 + i,
                rss_kb: 100,
                pss_kb: 90,
                uss_kb: 80,
                cpu_percent: 5.0,
                cpu_time_seconds: 1.0,
                top_cpu: [TopProcessInfo::default(); 3],
                top_rss: [TopProcessInfo::default(); 3],
                top_pss: [TopProcessInfo::default(); 3],
                _padding: [],
            };
            manager.record(&format!("subgroup_{}", i), entry);
        }

        let stats = manager.get_stats();
        assert_eq!(stats.total_subgroups, 3);

        let subgroups = manager.get_all_subgroups();
        assert_eq!(subgroups.len(), 3);
    }

    #[test]
    fn test_nonexistent_subgroup() {
        let manager = RingbufferManager::new(default_config(), 10);
        let history = manager.get_subgroup_history("nonexistent");
        assert!(history.is_none());
    }

    #[test]
    fn test_history_seconds_calculation() {
        let manager = RingbufferManager::new(default_config(), 10);
        let stats = manager.get_stats();

        // history_seconds = entries_per_subgroup * interval_seconds
        assert_eq!(
            stats.history_seconds,
            stats.entries_per_subgroup as u64 * 30
        );
    }

    #[test]
    fn test_retention_parsing() {
        use crate::config::{parse_retention, RetentionLimit};
        use std::time::Duration;

        assert_eq!(
            parse_retention("24h").unwrap(),
            RetentionLimit::Duration(Duration::from_secs(24 * 3600))
        );
        assert_eq!(
            parse_retention("7d").unwrap(),
            RetentionLimit::Duration(Duration::from_secs(7 * 24 * 3600))
        );
        assert_eq!(
            parse_retention("100MB").unwrap(),
            RetentionLimit::Size(100 * 1024 * 1024)
        );
        assert_eq!(
            parse_retention("100mg").unwrap(),
            RetentionLimit::Size(100 * 1024 * 1024)
        );
        assert_eq!(
            parse_retention("1G").unwrap(),
            RetentionLimit::Size(1024 * 1024 * 1024)
        );
        assert_eq!(
            parse_retention("60s").unwrap(),
            RetentionLimit::Duration(Duration::from_secs(60))
        );
    }

    #[test]
    fn test_sled_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_metrics.db");

        let mut config = default_config();
        config.enable_database = true;
        config.database_path = db_path.clone();
        config.retention = "24h".to_string();

        let now_ts = chrono::Utc::now().timestamp();

        // Scope 1: Write data to db
        {
            let manager = RingbufferManager::new(config.clone(), 10);
            assert!(manager.db.is_some(), "Database failed to open");
            let entry = RingbufferEntry {
                timestamp: now_ts,
                rss_kb: 500,
                pss_kb: 400,
                uss_kb: 300,
                cpu_percent: 12.5,
                cpu_time_seconds: 5.0,
                top_cpu: [TopProcessInfo::default(); 3],
                top_rss: [TopProcessInfo::default(); 3],
                top_pss: [TopProcessInfo::default(); 3],
                _padding: [],
            };
            manager.record("subgroup_a", entry);

            if let Some(ref db) = manager.db {
                let _ = db.flush();
            }
        }

        // Scope 2: Read it back from the same database
        {
            let manager = RingbufferManager::new(config, 10);
            assert!(manager.db.is_some(), "Database failed to open in Scope 2");

            let history = manager.get_subgroup_history("subgroup_a");
            assert!(history.is_some());
            let history = history.unwrap();
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].timestamp, now_ts);
            assert_eq!(history[0].rss_kb, 500);
        }
    }

    #[test]
    fn test_sled_pruning() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_metrics.db");

        let mut config = default_config();
        config.enable_database = true;
        config.database_path = db_path.clone();
        config.retention = "10s".to_string();

        let manager = RingbufferManager::new(config.clone(), 10);
        let now_ts = chrono::Utc::now().timestamp();

        let make_entry = |ts| RingbufferEntry {
            timestamp: ts,
            rss_kb: 500,
            pss_kb: 400,
            uss_kb: 300,
            cpu_percent: 12.5,
            cpu_time_seconds: 5.0,
            top_cpu: [TopProcessInfo::default(); 3],
            top_rss: [TopProcessInfo::default(); 3],
            top_pss: [TopProcessInfo::default(); 3],
            _padding: [],
        };

        // Record one old entry (20 seconds ago) and one recent entry (now)
        manager.record("subgroup_a", make_entry(now_ts - 20));
        manager.record("subgroup_a", make_entry(now_ts));

        // Flush and prune
        if let Some(ref db) = manager.db {
            let _ = db.flush();
        }
        manager.prune_database(true).unwrap();

        // Historical data should now only contain the recent entry
        let history = manager.get_subgroup_history("subgroup_a").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].timestamp, now_ts);
    }
}
