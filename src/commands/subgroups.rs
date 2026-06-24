//! Subgroups command implementation.
//!
//! Lists available process subgroups.

use ahash::AHashMap as HashMap;

use crate::process::SUBGROUPS;

/// Lists available process subgroups (ignores search filters intentionally).
pub fn command_subgroups(
    verbose: bool,
    group: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Herakles Process Memory Exporter - Available Subgroups");
    println!("=========================================================");

    let mut groups_map: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();

    let subgroups_guard = SUBGROUPS.read()?;

    for (process_name, (group, subgroup)) in subgroups_guard.iter() {
        groups_map
            .entry(group)
            .or_default()
            .push((subgroup, process_name));
    }

    for (group_name, subgroups) in &groups_map {
        if let Some(filter) = &group {
            if !group_name.contains(filter) {
                continue;
            }
        }

        println!("\n🏷️  Group: {}", group_name);
        println!("{}", "─".repeat(50));

        let mut subgroup_map: HashMap<&str, Vec<&str>> = HashMap::new();
        for (subgroup, process_name) in subgroups {
            subgroup_map.entry(subgroup).or_default().push(process_name);
        }

        for (subgroup, process_names) in subgroup_map {
            println!("   ├─ 📂 Subgroup: {}", subgroup);

            if verbose {
                for process_name in process_names {
                    println!("   │  ├─ 🔍 Matches: {}", process_name);
                }
            } else {
                let count = process_names.len();
                let examples: Vec<_> = process_names.iter().take(3).cloned().collect();
                println!("   │  ├─ {} matching processes", count);
                if !examples.is_empty() {
                    println!("   │  └─ Examples: {}", examples.join(", "));
                }
            }
        }
    }

    println!(
        "\n📋 Total: {} process patterns in {} groups",
        subgroups_guard.len(),
        groups_map.len()
    );

    Ok(())
}
