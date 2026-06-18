//! Check command implementation.
//!
//! Validates system requirements and configuration.

use std::path::Path;

use crate::config::{validate_effective_config, Config};
use crate::process::{collect_proc_entries, parse_memory_for_process, BufferConfig, SUBGROUPS};

/// Validates system requirements and configuration.
pub fn command_check(
    memory: bool,
    proc: bool,
    all: bool,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Herakles Process Memory Exporter - System Check");
    println!("===================================================");

    let mut all_ok = true;

    // Check /proc filesystem
    if proc || all {
        println!("\n📁 Checking /proc filesystem...");
        if Path::new("/proc").exists() {
            println!("   ✅ /proc filesystem accessible");

            // Check if we can read process directories
            let proc_entries = collect_proc_entries("/proc", Some(5));
            if proc_entries.is_empty() {
                println!("   ❌ Cannot read any process entries from /proc");
                all_ok = false;
            } else {
                println!("   ✅ Can read {} process entries", proc_entries.len());
            }
        } else {
            println!("   ❌ /proc filesystem not found");
            all_ok = false;
        }
    }

    // Check memory metrics accessibility
    if memory || all {
        println!("\n💾 Checking memory metrics accessibility...");
        let test_pid = std::process::id();
        let test_path = Path::new("/proc").join(test_pid.to_string());

        if test_path.join("smaps_rollup").exists() {
            println!("   ✅ smaps_rollup available (fast path)");
        } else if test_path.join("smaps").exists() {
            println!("   ✅ smaps available (slow path)");
        } else {
            println!("   ❌ No memory maps accessible");
            all_ok = false;
        }

        // Test actual parsing
        let buffer_config = BufferConfig {
            io_kb: config.io_buffer_kb.unwrap_or(256),
            smaps_kb: config.smaps_buffer_kb.unwrap_or(512),
            smaps_rollup_kb: config.smaps_rollup_buffer_kb.unwrap_or(256),
        };

        match parse_memory_for_process(&test_path, &buffer_config) {
            Ok((rss, pss, uss)) => {
                println!(
                    "   ✅ Memory parsing successful: RSS={}MB, PSS={}MB, USS={}MB",
                    rss / 1024 / 1024,
                    pss / 1024 / 1024,
                    uss / 1024 / 1024
                );
            }
            Err(e) => {
                println!("   ❌ Memory parsing failed: {}", e);
                all_ok = false;
            }
        }
    }

    // Check configuration
    println!("\n⚙️  Checking configuration...");
    match validate_effective_config(config) {
        Ok(_) => {
            println!("   ✅ Configuration is valid");
        }
        Err(e) => {
            println!("   ❌ Configuration invalid: {}", e);
            all_ok = false;
        }
    }

    // Check database configuration and access
    if all || config.ringbuffer.enable_database {
        println!("\n🗄️  Checking sled database persistence...");
        if config.ringbuffer.enable_database {
            println!(
                "   ⚙️  Database path: {}",
                config.ringbuffer.database_path.display()
            );
            println!("   ⚙️  Retention limit: {}", config.ringbuffer.retention);

            // 1. Try to create the parent directory if it does not exist
            if let Some(parent) = config.ringbuffer.database_path.parent() {
                match std::fs::create_dir_all(parent) {
                    Ok(_) => {
                        println!(
                            "   ✅ Database directory is accessible/created: {}",
                            parent.display()
                        );
                    }
                    Err(e) => {
                        println!(
                            "   ❌ Failed to create/access database directory {}: {}",
                            parent.display(),
                            e
                        );
                        all_ok = false;
                    }
                }
            }

            // 2. Try to open the database to check if it's writeable and not locked by another process
            match sled::open(&config.ringbuffer.database_path) {
                Ok(_db) => {
                    println!("   ✅ Sled database opened successfully (write check passed, no lock contention)");
                }
                Err(e) => {
                    println!("   ❌ Failed to open sled database: {}", e);
                    println!("      ⚠️  Note: Sled requires exclusive file locks. If another instance of herakles-node-exporter is running, it will fail to open.");
                    all_ok = false;
                }
            }
        } else {
            println!("   ℹ️  Database persistence is disabled in configuration.");
        }
    }

    // Check subgroups configuration
    println!("\n📊 Checking subgroups configuration...");
    let subgroups_guard = SUBGROUPS.read().unwrap();
    if subgroups_guard.is_empty() {
        println!("   ⚠️  No subgroups configured");
    } else {
        println!("   ✅ {} subgroups loaded", subgroups_guard.len());
    }

    println!("\n📋 Summary:");
    if all_ok {
        println!("   ✅ All checks passed - system is ready");
        Ok(())
    } else {
        println!("   ❌ Some checks failed - please review warnings");
        std::process::exit(1);
    }
}
