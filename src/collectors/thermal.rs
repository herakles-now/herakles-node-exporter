//! Thermal sensor collector for reading CPU temperature.
//!
//! This module collects temperature readings from thermal sensors available in:
//! - /sys/class/thermal/thermal_zone*/temp
//! - /sys/class/hwmon/hwmon*/temp*_input

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Temperature reading with sensor name.
#[allow(dead_code)] // Struct defined for internal use, used via collect_temperatures
#[derive(Debug, Clone)]
pub struct ThermalReading {
    pub sensor_name: String,
    pub temperature_celsius: f64,
}

/// Reads temperature from all thermal zones.
/// Returns a HashMap mapping sensor name to temperature in Celsius.
pub fn read_thermal_zones() -> Result<HashMap<String, f64>, String> {
    let mut temperatures = HashMap::new();
    let thermal_base = Path::new("/sys/class/thermal");

    if !thermal_base.exists() {
        return Ok(temperatures); // No thermal zones available
    }

    let entries = fs::read_dir(thermal_base)
        .map_err(|e| format!("Failed to read thermal directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let zone_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        // Only process thermal_zone* directories
        if !zone_name.starts_with("thermal_zone") {
            continue;
        }

        let temp_file = path.join("temp");
        if !temp_file.exists() {
            continue;
        }

        // Read temperature (in millidegrees Celsius)
        match fs::read_to_string(&temp_file) {
            Ok(content) => {
                if let Ok(millidegrees) = content.trim().parse::<i64>() {
                    let celsius = millidegrees as f64 / 1000.0;
                    temperatures.insert(zone_name, celsius);
                }
            }
            Err(_) => continue,
        }
    }

    Ok(temperatures)
}

/// Reads temperature from hardware monitoring devices.
/// Returns a HashMap mapping sensor name to temperature in Celsius.
pub fn read_hwmon_temps() -> Result<HashMap<String, f64>, String> {
    let mut temperatures = HashMap::new();
    let hwmon_base = Path::new("/sys/class/hwmon");

    if !hwmon_base.exists() {
        return Ok(temperatures); // No hwmon devices available
    }

    let entries =
        fs::read_dir(hwmon_base).map_err(|e| format!("Failed to read hwmon directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let hwmon_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        // Only process hwmon* directories
        if !hwmon_name.starts_with("hwmon") {
            continue;
        }

        // Read the name file to get a more descriptive sensor name
        let name_file = path.join("name");
        let device_name = if name_file.exists() {
            fs::read_to_string(&name_file)
                .unwrap_or_else(|_| hwmon_name.clone())
                .trim()
                .to_string()
        } else {
            hwmon_name.clone()
        };

        // Look for temp*_input files
        let dir_entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for temp_entry in dir_entries.flatten() {
            let temp_path = temp_entry.path();
            let temp_filename = match temp_path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // Only process temp*_input files
            if !temp_filename.starts_with("temp") || !temp_filename.ends_with("_input") {
                continue;
            }

            // Read temperature (in millidegrees Celsius)
            match fs::read_to_string(&temp_path) {
                Ok(content) => {
                    if let Ok(millidegrees) = content.trim().parse::<i64>() {
                        let celsius = millidegrees as f64 / 1000.0;
                        let sensor_name = format!("{}_{}", device_name, temp_filename);
                        temperatures.insert(sensor_name, celsius);
                    }
                }
                Err(_) => continue,
            }
        }
    }

    Ok(temperatures)
}

/// Collects all temperature readings from both thermal zones and hwmon.
/// Returns a HashMap mapping sensor name to temperature in Celsius.
pub fn collect_temperatures() -> Result<HashMap<String, f64>, String> {
    let mut all_temps = HashMap::new();

    // Collect from thermal zones
    if let Ok(thermal_temps) = read_thermal_zones() {
        all_temps.extend(thermal_temps);
    }

    // Collect from hwmon devices
    if let Ok(hwmon_temps) = read_hwmon_temps() {
        all_temps.extend(hwmon_temps);
    }

    Ok(all_temps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_temperatures() {
        // This test will pass even if no thermal sensors are available
        let result = collect_temperatures();
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_thermal_zones() {
        let result = read_thermal_zones();
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_hwmon_temps() {
        let result = read_hwmon_temps();
        assert!(result.is_ok());
    }
}
