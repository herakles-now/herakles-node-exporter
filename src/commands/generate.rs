//! Generate testdata command implementation.
//!
//! Generates synthetic test data JSON files for testing.

use ahash::AHashMap as HashMap;
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::cache::ProcMem;
use crate::config::Config;
use crate::process::{classify_process_with_config, SUBGROUPS};

/// Test process entry for JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProcess {
    pub pid: u32,
    pub name: String,
    pub group: String,
    pub subgroup: String,
    pub rss: u64,
    pub pss: u64,
    pub uss: u64,
    pub cpu_percent: f64,
    pub cpu_time_seconds: f64,
}

/// Root structure for test data JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestData {
    pub version: String,
    pub generated_at: String,
    pub processes: Vec<TestProcess>,
}

/// Converts a TestProcess from JSON test data into ProcMem for metrics.
impl From<TestProcess> for ProcMem {
    fn from(tp: TestProcess) -> Self {
        ProcMem {
            pid: tp.pid,
            name: tp.name,
            rss: tp.rss,
            pss: tp.pss,
            uss: tp.uss,
            cpu_percent: tp.cpu_percent as f32,
            cpu_time_seconds: tp.cpu_time_seconds as f32,
        }
    }
}

/// Load test data from JSON file.
pub fn load_test_data_from_file(path: &Path) -> Result<TestData, String> {
    debug!("Loading test data from: {}", path.display());

    if !path.exists() {
        return Err(format!("Test data file not found: {}", path.display()));
    }

    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read test data file: {}", e))?;
    let test_data: TestData = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse test data JSON: {}", e))?;

    info!(
        "Loaded test data version {} from {}",
        test_data.version, test_data.generated_at
    );

    Ok(test_data)
}

/// Generates synthetic test data JSON file for testing purposes.
pub fn command_generate_testdata(
    output: PathBuf,
    min_per_subgroup: usize,
    others_count: usize,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!(
        "Generating test data: min_per_subgroup={}, others_count={}, output={}",
        min_per_subgroup,
        others_count,
        output.display()
    );

    let mut rng = rand::thread_rng();
    let mut processes: Vec<TestProcess> = Vec::new();
    let mut current_pid: u32 = 1000;

    // Collect unique (group, subgroup) pairs with their associated process name matches
    let mut subgroup_matches: HashMap<(String, String), Vec<String>> = HashMap::new();

    for (process_name, (group, subgroup)) in SUBGROUPS.iter() {
        let key = (group.to_string(), subgroup.to_string());
        subgroup_matches
            .entry(key)
            .or_default()
            .push(process_name.to_string());
    }

    debug!("Found {} unique subgroups", subgroup_matches.len());

    // Generate processes for each subgroup
    for ((group, subgroup), matches) in &subgroup_matches {
        // Skip "other/unknown" - we handle it separately at the end
        if group == "other" && subgroup == "unknown" {
            continue;
        }

        // Apply config filters using classify_process_with_config
        if let Some(sample_name) = matches.first() {
            if classify_process_with_config(sample_name, config).is_none() {
                debug!(
                    "Skipping subgroup {}/{} due to config filters",
                    group, subgroup
                );
                continue;
            }
        }

        // Generate min_per_subgroup processes for this subgroup
        for i in 0..min_per_subgroup {
            let name = if matches.is_empty() {
                format!("{}-{}", subgroup, i + 1)
            } else {
                matches[i % matches.len()].clone()
            };

            let proc = generate_random_process(&mut rng, current_pid, name, group, subgroup);
            processes.push(proc);
            current_pid += 1;
        }

        debug!(
            "Generated {} processes for subgroup {}/{}",
            min_per_subgroup, group, subgroup
        );
    }

    // Generate "other/unknown" processes (unless disabled)
    let disable_others = config.disable_others.unwrap_or(false);
    if !disable_others {
        for i in 0..others_count {
            let name = format!("process-{}", i + 1);
            let proc = generate_random_process(&mut rng, current_pid, name, "other", "other");
            processes.push(proc);
            current_pid += 1;
        }
        debug!("Generated {} 'other' processes", others_count);
    } else {
        debug!("Skipping 'other' processes due to disable_others config");
    }

    // Create the test data structure
    let test_data = TestData {
        version: "1.0".to_string(),
        generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        processes,
    };

    // Write to file as pretty-printed JSON
    let json_content = serde_json::to_string_pretty(&test_data)?;
    fs::write(&output, &json_content)?;

    println!(
        "✅ Generated test data: {} processes in {}",
        test_data.processes.len(),
        output.display()
    );

    Ok(())
}

/// Generates a random test process with realistic memory and CPU values.
fn generate_random_process(
    rng: &mut impl Rng,
    pid: u32,
    name: String,
    group: &str,
    subgroup: &str,
) -> TestProcess {
    // RSS: 10 MB - 2 GB (in bytes)
    let rss = rng.gen_range(10 * 1024 * 1024..2 * 1024 * 1024 * 1024_u64);

    // PSS: 80-95% of RSS
    let pss_ratio: f64 = rng.gen_range(0.80..0.95);
    let pss = (rss as f64 * pss_ratio) as u64;

    // USS: 60-80% of RSS
    let uss_ratio: f64 = rng.gen_range(0.60..0.80);
    let uss = (rss as f64 * uss_ratio) as u64;

    // CPU percent: 0.0 - 100.0
    let cpu_percent: f64 = rng.gen_range(0.0..100.0);

    // CPU time: 0.0 - 10000.0 seconds
    let cpu_time_seconds: f64 = rng.gen_range(0.0..10000.0);

    TestProcess {
        pid,
        name,
        group: group.to_string(),
        subgroup: subgroup.to_string(),
        rss,
        pss,
        uss,
        cpu_percent,
        cpu_time_seconds,
    }
}
