//! Process classification for grouping processes into categories.
//!
//! This module provides functions to classify processes into groups and subgroups
//! based on their names, using a configurable mapping loaded from TOML files.

use crate::config::Config;
use ahash::AHashMap as HashMap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Type alias for the subgroups map.
pub type SubgroupsMap = HashMap<Arc<str>, (Arc<str>, Arc<str>)>;

/// Data structure for subgroup configuration from TOML.
#[derive(Deserialize)]
struct Subgroup {
    group: String,
    subgroup: String,
    matches: Option<Vec<String>>,
    cmdline_matches: Option<Vec<String>>,
}

/// Root structure for subgroups configuration.
#[derive(Deserialize)]
struct SubgroupsConfig {
    subgroups: Vec<Subgroup>,
}

/// Helper: load subgroups from TOML string into map.
fn load_subgroups_from_str(content: &str, map: &mut SubgroupsMap) {
    let parsed: SubgroupsConfig = match toml::from_str(content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to parse subgroups TOML: {}", e);
            return;
        }
    };

    for sg in parsed.subgroups {
        let group_arc: Arc<str> = Arc::from(sg.group.as_str());
        let subgroup_arc: Arc<str> = Arc::from(sg.subgroup.as_str());

        if let Some(matches) = sg.matches {
            for m in matches {
                let key_arc: Arc<str> = Arc::from(m.as_str());
                map.insert(key_arc, (Arc::clone(&group_arc), Arc::clone(&subgroup_arc)));
            }
        }
        if let Some(cmdlines) = sg.cmdline_matches {
            for cmd in cmdlines {
                let key_arc: Arc<str> = Arc::from(cmd.as_str());
                map.insert(key_arc, (Arc::clone(&group_arc), Arc::clone(&subgroup_arc)));
            }
        }
    }
}

/// Helper: load subgroups from TOML file path (if exists).
fn load_subgroups_from_file(path: &str, map: &mut SubgroupsMap) {
    let p = Path::new(path);
    if !p.exists() {
        return;
    }
    match fs::read_to_string(p) {
        Ok(content) => {
            load_subgroups_from_str(&content, map);
            eprintln!("Loaded additional subgroups from {}", path);
        }
        Err(e) => {
            eprintln!("Failed to read subgroups file {}: {}", path, e);
        }
    }
}

/// Static configuration for process subgroups loaded from TOML file(s).
pub static SUBGROUPS: Lazy<SubgroupsMap> = Lazy::new(|| {
    let mut map = HashMap::new();

    // 1) built-in subgroups from embedded file
    let content = include_str!("../../data/subgroups.toml");
    load_subgroups_from_str(content, &mut map);

    // 2) optional system-wide subgroups
    load_subgroups_from_file("/etc/herakles/subgroups.toml", &mut map);

    // 3) optional subgroups in current working directory
    load_subgroups_from_file("./subgroups.toml", &mut map);

    map
});

// Static Arc<str> for default classification values to avoid repeated allocations
static OTHER_STR: Lazy<Arc<str>> = Lazy::new(|| Arc::from("other"));
static UNKNOWN_STR: Lazy<Arc<str>> = Lazy::new(|| Arc::from("unknown"));

/// Classifies a process into group and subgroup based on process name (raw).
pub fn classify_process_raw(process_name: &str) -> (Arc<str>, Arc<str>) {
    SUBGROUPS
        .get(process_name)
        .map(|(g, sg)| (Arc::clone(g), Arc::clone(sg)))
        .unwrap_or_else(|| (Arc::clone(&OTHER_STR), Arc::clone(&UNKNOWN_STR)))
}

/// Classification including config rules (include/exclude, disable_others).
pub fn classify_process_with_config(
    process_name: &str,
    cfg: &Config,
) -> Option<(Arc<str>, Arc<str>)> {
    let (group, subgroup) = classify_process_raw(process_name);

    // If user explicitly disabled "other" bucket, drop these processes
    let disable_others = cfg.disable_others.unwrap_or(false);
    if disable_others && group.as_ref() == "other" {
        return None;
    }

    // Apply include/exclude/search-mode logic
    let mode = cfg.search_mode.as_deref().unwrap_or("none");

    let group_match = cfg
        .search_groups
        .as_ref()
        .is_some_and(|v| v.iter().any(|g| g == group.as_ref()));
    let subgroup_match = cfg
        .search_subgroups
        .as_ref()
        .is_some_and(|v| v.iter().any(|sg| sg == subgroup.as_ref()));

    let allowed = match mode {
        "include" => {
            // Only these groups/subgroups
            group_match || subgroup_match
        }
        "exclude" => {
            // Everything except these groups/subgroups
            !(group_match || subgroup_match)
        }
        _ => true, // no filter
    };

    if !allowed {
        return None;
    }

    // Normalize: treat all "unknown" subgroups in the "other" group as "other"
    // so that subgroup "unknown" does not appear in exports.
    if group.as_ref().eq_ignore_ascii_case("other") {
        Some((Arc::clone(&OTHER_STR), Arc::clone(&OTHER_STR)))
    } else {
        Some((group, subgroup))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Tests for classify_process (subgroup classification)
    // -------------------------------------------------------------------------

    #[test]
    fn test_classify_process_raw_unknown() {
        // Unknown process should fall into "other"/"unknown"
        let (group, subgroup) = classify_process_raw("totally_unknown_process_xyz123");
        assert_eq!(group.as_ref(), "other");
        assert_eq!(subgroup.as_ref(), "unknown");
    }
}
