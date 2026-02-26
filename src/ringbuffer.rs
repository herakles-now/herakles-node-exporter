//! Ringbuffer module for tracking historical metrics.
//!
//! This module provides a fixed-size ringbuffer for storing historical
//! metrics entries with predictable memory usage.

/// Size of a single ringbuffer entry in bytes (256 bytes with extended top-N data).
pub const ENTRY_SIZE_BYTES: usize = 256;

/// Top process information stored in ringbuffer (24 bytes per entry).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TopProcessInfo {
    pub pid: u32,       // 4 bytes - Process ID
    pub value: u32,     // 4 bytes - Value in KB (for memory) or scaled (for CPU)
    pub name: [u8; 16], // 16 bytes - Null-terminated process name
}

impl Default for TopProcessInfo {
    fn default() -> Self {
        Self {
            pid: 0,
            value: 0,
            name: [0; 16],
        }
    }
}

impl TopProcessInfo {
    /// Create a new TopProcessInfo with the given PID, value, and name.
    pub fn new(pid: u32, value: u32, name: &str) -> Self {
        let mut name_bytes = [0u8; 16];
        let bytes = name.as_bytes();
        // Truncate to 15 bytes max, ensuring the 16th byte remains 0 for null termination
        let len = bytes.len().min(15);
        name_bytes[..len].copy_from_slice(&bytes[..len]);
        Self {
            pid,
            value,
            name: name_bytes,
        }
    }

    /// Get the process name as a string.
    pub fn name_str(&self) -> String {
        // Find the null terminator, or use full length if none found
        let len = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name.len());
        String::from_utf8_lossy(&self.name[..len]).to_string()
    }
}

/// Fixed-size entry for ringbuffer storage (256 bytes with extended data).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RingbufferEntry {
    // Existing aggregated metrics (40 bytes)
    pub timestamp: i64,        // 8 bytes - Unix timestamp
    pub rss_kb: u64,           // 8 bytes
    pub pss_kb: u64,           // 8 bytes
    pub uss_kb: u64,           // 8 bytes
    pub cpu_percent: f32,      // 4 bytes
    pub cpu_time_seconds: f32, // 4 bytes

    // Top-3 processes by each metric
    // 3 entries per metric × 3 metrics = 9 entries × 24 bytes = 216 bytes
    pub top_cpu: [TopProcessInfo; 3], // 72 bytes - Top 3 by CPU
    pub top_rss: [TopProcessInfo; 3], // 72 bytes - Top 3 by RSS
    pub top_pss: [TopProcessInfo; 3], // 72 bytes - Top 3 by PSS

    // Total: 40 + 216 = 256 bytes exactly
    pub _padding: [u8; 0], // No padding needed - exactly 256 bytes
}

/// A circular buffer for storing metric entries with fixed capacity.
pub struct Ringbuffer {
    entries: Vec<RingbufferEntry>,
    capacity: usize,
    write_index: usize,
    count: usize,
}

impl Ringbuffer {
    /// Creates a new ringbuffer with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(capacity);
        entries.resize(capacity, RingbufferEntry::default());

        Self {
            entries,
            capacity,
            write_index: 0,
            count: 0,
        }
    }

    /// Pushes a new entry into the ringbuffer.
    ///
    /// If the buffer is full, the oldest entry will be overwritten.
    pub fn push(&mut self, entry: RingbufferEntry) {
        self.entries[self.write_index] = entry;
        self.write_index = (self.write_index + 1) % self.capacity;

        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Returns all entries in chronological order (oldest to newest).
    pub fn get_history(&self) -> Vec<RingbufferEntry> {
        if self.count == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(self.count);

        if self.count < self.capacity {
            // Buffer not yet full, entries are in order from 0 to count-1
            result.extend_from_slice(&self.entries[0..self.count]);
        } else {
            // Buffer is full, need to arrange from write_index (oldest) to end, then from 0
            result.extend_from_slice(&self.entries[self.write_index..]);
            result.extend_from_slice(&self.entries[0..self.write_index]);
        }

        result
    }

    /// Returns the current number of entries in the buffer.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns the maximum capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns true if the buffer is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_size() {
        // Verify the entry is exactly 256 bytes
        assert_eq!(std::mem::size_of::<RingbufferEntry>(), ENTRY_SIZE_BYTES);
    }

    #[test]
    fn test_ringbuffer_push_and_read() {
        let mut rb = Ringbuffer::new(3);

        assert_eq!(rb.len(), 0);
        assert_eq!(rb.capacity(), 3);

        // Push first entry
        rb.push(RingbufferEntry {
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
        });

        assert_eq!(rb.len(), 1);
        let history = rb.get_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].timestamp, 1000);
    }

    #[test]
    fn test_ringbuffer_chronological_order() {
        let mut rb = Ringbuffer::new(3);

        // Push three entries
        for i in 0..3 {
            rb.push(RingbufferEntry {
                timestamp: 1000 + i * 100,
                rss_kb: 100 + i as u64,
                pss_kb: 90,
                uss_kb: 80,
                cpu_percent: 5.0,
                cpu_time_seconds: 1.0,
                top_cpu: [TopProcessInfo::default(); 3],
                top_rss: [TopProcessInfo::default(); 3],
                top_pss: [TopProcessInfo::default(); 3],
                _padding: [],
            });
        }

        let history = rb.get_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].timestamp, 1000);
        assert_eq!(history[1].timestamp, 1100);
        assert_eq!(history[2].timestamp, 1200);
    }

    #[test]
    fn test_ringbuffer_wraparound() {
        let mut rb = Ringbuffer::new(3);

        // Push 5 entries (will wrap around)
        for i in 0..5 {
            rb.push(RingbufferEntry {
                timestamp: 1000 + i * 100,
                rss_kb: 100 + i as u64,
                pss_kb: 90,
                uss_kb: 80,
                cpu_percent: 5.0,
                cpu_time_seconds: 1.0,
                top_cpu: [TopProcessInfo::default(); 3],
                top_rss: [TopProcessInfo::default(); 3],
                top_pss: [TopProcessInfo::default(); 3],
                _padding: [],
            });
        }

        // Should only have the last 3 entries in chronological order
        let history = rb.get_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].timestamp, 1200); // Entry 2 (oldest remaining)
        assert_eq!(history[1].timestamp, 1300); // Entry 3
        assert_eq!(history[2].timestamp, 1400); // Entry 4 (newest)
    }

    #[test]
    fn test_ringbuffer_empty() {
        let rb = Ringbuffer::new(10);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());

        let history = rb.get_history();
        assert_eq!(history.len(), 0);
    }
}
