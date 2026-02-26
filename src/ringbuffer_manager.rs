//! Ringbuffer manager for managing multiple ringbuffers across subgroups.
//!
//! This module provides the `RingbufferManager` which maintains a collection
//! of ringbuffers, one per subgroup, with deterministic memory allocation.

use crate::config::RingbufferConfig;
use crate::ringbuffer::{Ringbuffer, RingbufferEntry, ENTRY_SIZE_BYTES};
#[cfg(test)]
use crate::ringbuffer::TopProcessInfo;
use dashmap::DashMap;
use serde::Serialize;

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
}

/// Manager for multiple ringbuffers, one per subgroup.
pub struct RingbufferManager {
    buffers: DashMap<String, Ringbuffer>,
    entries_per_subgroup: usize,
    interval_seconds: u64,
    config: RingbufferConfig,
    estimated_ram_bytes: usize,
}

impl RingbufferManager {
    /// Creates a new ringbuffer manager.
    ///
    /// # Arguments
    /// * `config` - Ringbuffer configuration
    /// * `initial_subgroup_count` - Number of subgroups expected at startup
    ///
    /// The manager calculates entries_per_subgroup at initialization based on:
    /// - max_memory_mb / ENTRY_SIZE_BYTES / initial_subgroup_count
    /// - Clamped between min_entries_per_subgroup and max_entries_per_subgroup
    pub fn new(config: RingbufferConfig, initial_subgroup_count: usize) -> Self {
        // Calculate maximum total entries based on memory budget
        let max_bytes = config.max_memory_mb * 1024 * 1024;
        let max_total_entries = max_bytes / ENTRY_SIZE_BYTES;

        // Calculate entries per subgroup
        let subgroup_count = initial_subgroup_count.max(1); // Prevent division by zero
        let calculated_entries = max_total_entries / subgroup_count;

        // Clamp to configured min/max
        let entries_per_subgroup = calculated_entries
            .max(config.min_entries_per_subgroup)
            .min(config.max_entries_per_subgroup);

        // Estimate actual RAM usage
        let estimated_ram_bytes = entries_per_subgroup * ENTRY_SIZE_BYTES * subgroup_count;

        Self {
            buffers: DashMap::new(),
            entries_per_subgroup,
            interval_seconds: config.interval_seconds,
            config,
            estimated_ram_bytes,
        }
    }

    /// Records a metric entry for a specific subgroup.
    ///
    /// If the subgroup doesn't have a ringbuffer yet, one is created
    /// with the pre-calculated capacity.
    pub fn record(&self, subgroup: &str, entry: RingbufferEntry) {
        self.buffers
            .entry(subgroup.to_string())
            .or_insert_with(|| Ringbuffer::new(self.entries_per_subgroup))
            .push(entry);
    }

    /// Returns statistics about the ringbuffer system.
    pub fn get_stats(&self) -> RingbufferStats {
        let total_subgroups = self.buffers.len();
        let history_seconds = self.entries_per_subgroup as u64 * self.interval_seconds;

        RingbufferStats {
            max_memory_mb: self.config.max_memory_mb,
            entry_size_bytes: ENTRY_SIZE_BYTES,
            interval_seconds: self.interval_seconds,
            entries_per_subgroup: self.entries_per_subgroup,
            total_subgroups,
            estimated_ram_bytes: self.estimated_ram_bytes,
            history_seconds,
        }
    }

    /// Returns the historical entries for a specific subgroup.
    ///
    /// Returns None if the subgroup doesn't exist.
    pub fn get_subgroup_history(&self, subgroup: &str) -> Option<Vec<RingbufferEntry>> {
        self.buffers.get(subgroup).map(|rb| rb.get_history())
    }

    /// Returns a reference to the ringbuffer for a specific subgroup.
    ///
    /// Returns None if the subgroup doesn't exist.
    /// This allows access to ringbuffer methods like len() and capacity().
    pub fn get_subgroup_buffer(&self, subgroup: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Ringbuffer>> {
        self.buffers.get(subgroup)
    }

    /// Returns a list of all known subgroup names.
    pub fn get_all_subgroups(&self) -> Vec<String> {
        self.buffers
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> RingbufferConfig {
        RingbufferConfig {
            max_memory_mb: 15,
            interval_seconds: 30,
            min_entries_per_subgroup: 10,
            max_entries_per_subgroup: 120,
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
}
