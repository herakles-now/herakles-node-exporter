//! Test command implementation.
//!
//! Tests metrics collection and displays results.

use std::time::Instant;

use crate::cli::ConfigFormat;
use crate::config::Config;
use crate::process::{
    classify_process_raw, collect_proc_entries, parse_memory_for_process, read_process_name,
    BufferConfig, CpuStat,
};

/// Process memory metrics for test output.
struct TestProcMem {
    _pid: u32,
    _name: String,
    rss: u64,
    pss: u64,
    uss: u64,
    _cpu_percent: f32,
    _cpu_time_seconds: f32,
}

/// Tests metrics collection.
pub fn command_test(
    iterations: usize,
    verbose: bool,
    _format: ConfigFormat,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Herakles Process Memory Exporter - Test Mode");
    println!("================================================");

    let buffer_config = BufferConfig {
        io_kb: config.io_buffer_kb.unwrap_or(256),
        smaps_kb: config.smaps_buffer_kb.unwrap_or(512),
        smaps_rollup_kb: config.smaps_rollup_buffer_kb.unwrap_or(256),
    };

    for iteration in 1..=iterations {
        println!("\n🔄 Iteration {}/{}:", iteration, iterations);

        let start = Instant::now();
        let entries = collect_proc_entries("/proc", config.max_processes);
        println!("   📁 Found {} process entries", entries.len());

        let mut results = Vec::new();
        let mut error_count = 0;

        for entry in entries.iter().take(10) {
            match read_process_name(&entry.proc_path) {
                Some(name) => match parse_memory_for_process(&entry.proc_path, &buffer_config) {
                    Ok((rss, pss, uss)) => {
                        let cpu = CpuStat {
                            cpu_percent: 0.0,
                            cpu_time_seconds: 0.0,
                        };

                        results.push(TestProcMem {
                            _pid: entry.pid,
                            _name: name.clone(),
                            rss,
                            pss,
                            uss,
                            _cpu_percent: cpu.cpu_percent as f32,
                            _cpu_time_seconds: cpu.cpu_time_seconds as f32,
                        });

                        if verbose {
                            let base = classify_process_raw(&name);
                            println!("   ├─ {} (PID: {})", name, entry.pid);
                            println!("   │  ├─ Group: {}/{}", base.0, base.1);
                            println!("   │  ├─ RSS: {} MB", rss / 1024 / 1024);
                            println!("   │  ├─ PSS: {} MB", pss / 1024 / 1024);
                            println!("   │  └─ USS: {} MB", uss / 1024 / 1024);
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        if verbose {
                            println!("   ├─ ❌ PID {}: {}", entry.pid, e);
                        }
                    }
                },
                None => {
                    error_count += 1;
                }
            }
        }

        let duration = start.elapsed();
        println!(
            "   ⏱️  Scan duration: {:.2}ms",
            duration.as_secs_f64() * 1000.0
        );
        println!("   📊 Successfully scanned: {} processes", results.len());
        println!("   ❌ Errors: {}", error_count);

        if !results.is_empty() {
            let total_rss: u64 = results.iter().map(|p| p.rss).sum();
            let total_pss: u64 = results.iter().map(|p| p.pss).sum();
            let total_uss: u64 = results.iter().map(|p| p.uss).sum();

            println!("   📈 Memory totals:");
            println!("      ├─ RSS: {} MB", total_rss / 1024 / 1024);
            println!("      ├─ PSS: {} MB", total_pss / 1024 / 1024);
            println!("      └─ USS: {} MB", total_uss / 1024 / 1024);
        }
    }

    println!("\n✅ Test completed successfully");
    Ok(())
}
