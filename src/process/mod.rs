//! Process-related modules for memory, CPU, and classification functionality.
//!
//! This module provides:
//! - `memory`: Memory parsing from /proc/<pid>/smaps
//! - `cpu`: CPU time parsing and statistics
//! - `scanner`: Process discovery and filtering
//! - `classifier`: Process grouping and classification

pub mod classifier;
pub mod cpu;
pub mod memory;
pub mod scanner;

// Re-export commonly used types
pub use classifier::{classify_process_raw, classify_process_with_config, SUBGROUPS};
pub use cpu::{get_cpu_stat_for_pid, CpuEntry, CpuStat, CLK_TCK};
pub use memory::{
    parse_memory_for_process, BufferConfig, MAX_IO_BUFFER_BYTES, MAX_SMAPS_BUFFER_BYTES,
    MAX_SMAPS_ROLLUP_BUFFER_BYTES,
};
pub use scanner::{collect_proc_entries, read_process_name, should_include_process};
