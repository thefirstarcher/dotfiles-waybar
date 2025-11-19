use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;
use waybar_common::WaybarOutput;
use waybar_common::output::error_output;

#[derive(Debug)]
struct ThermalData {
    cpu_package: Option<f32>,
    cpu_cores: Vec<(usize, f32)>,
    nvme_temps: Vec<(String, f32)>,
    fan_speeds: Vec<(String, u32)>,
}

fn _read_temp_from_hwmon(hwmon_path: &Path, _label: &str) -> Option<f32> {
    let temp_input = hwmon_path.join("temp1_input");
    if let Ok(content) = fs::read_to_string(&temp_input) {
        if let Ok(millidegrees) = content.trim().parse::<i32>() {
            return Some(millidegrees as f32 / 1000.0);
        }
    }
    None
}

fn read_cpu_temps() -> Result<(Option<f32>, Vec<(usize, f32)>)> {
    let mut package_temp = None;
    let mut core_temps = Vec::new();

    // Try reading from /sys/class/hwmon
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Check if this is a CPU thermal sensor
            if let Ok(name) = fs::read_to_string(path.join("name")) {
                let name = name.trim();

                if name == "coretemp" || name == "k10temp" || name == "zenpower" {
                    // Try to find package temp (temp1) and core temps
                    for i in 1..=20 {
                        let temp_label_path = path.join(format!("temp{}_label", i));
                        let temp_input_path = path.join(format!("temp{}_input", i));

                        if temp_input_path.exists() {
                            if let Ok(temp_str) = fs::read_to_string(&temp_input_path) {
                                if let Ok(millidegrees) = temp_str.trim().parse::<i32>() {
                                    let temp = millidegrees as f32 / 1000.0;

                                    // Try to determine if this is package or core temp
                                    let label = fs::read_to_string(&temp_label_path)
                                        .unwrap_or_default()
                                        .trim()
                                        .to_string();

                                    if label.contains("Package") || label.contains("Tctl") || i == 1 {
                                        package_temp = Some(temp);
                                    } else if label.contains("Core") {
                                        if let Some(core_num) = label.split_whitespace()
                                            .last()
                                            .and_then(|s| s.parse::<usize>().ok()) {
                                            core_temps.push((core_num, temp));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort core temps by core number
    core_temps.sort_by_key(|(num, _)| *num);

    Ok((package_temp, core_temps))
}

fn read_nvme_temps() -> Result<Vec<(String, f32)>> {
    let mut nvme_temps = Vec::new();

    // Read from /sys/class/hwmon for NVMe drives
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if let Ok(name) = fs::read_to_string(path.join("name")) {
                let name = name.trim();

                if name.starts_with("nvme") {
                    // Try to read temperature
                    let temp_input = path.join("temp1_input");
                    if let Ok(temp_str) = fs::read_to_string(&temp_input) {
                        if let Ok(millidegrees) = temp_str.trim().parse::<i32>() {
                            let temp = millidegrees as f32 / 1000.0;
                            nvme_temps.push((name.to_string(), temp));
                        }
                    }
                }
            }
        }
    }

    Ok(nvme_temps)
}

fn read_fan_speeds() -> Result<Vec<(String, u32)>> {
    let mut fan_speeds = Vec::new();

    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Check all fan inputs
            for i in 1..=10 {
                let fan_input_path = path.join(format!("fan{}_input", i));
                let fan_label_path = path.join(format!("fan{}_label", i));

                if fan_input_path.exists() {
                    if let Ok(rpm_str) = fs::read_to_string(&fan_input_path) {
                        if let Ok(rpm) = rpm_str.trim().parse::<u32>() {
                            if rpm > 0 {
                                let label = fs::read_to_string(&fan_label_path)
                                    .unwrap_or_else(|_| format!("Fan {}", i))
                                    .trim()
                                    .to_string();
                                fan_speeds.push((label, rpm));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(fan_speeds)
}

fn collect_thermal_data() -> Result<ThermalData> {
    let (cpu_package, cpu_cores) = read_cpu_temps()?;
    let nvme_temps = read_nvme_temps()?;
    let fan_speeds = read_fan_speeds()?;

    Ok(ThermalData {
        cpu_package,
        cpu_cores,
        nvme_temps,
        fan_speeds,
    })
}

fn determine_class(temp: f32) -> &'static str {
    match temp as i32 {
        0..=60 => "normal",
        61..=75 => "warning",
        _ => "critical",
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("default");

    let data = collect_thermal_data().context("Failed to collect thermal data")?;

    match mode {
        "compact" | "default" => {
            // Show only CPU package temp
            if let Some(cpu_temp) = data.cpu_package {
                let class = determine_class(cpu_temp);
                WaybarOutput::builder()
                    .text(format!(" {:.0}°C", cpu_temp))
                    .tooltip(format!("CPU: {:.1}°C\n\nHover for more details\nClick for full view", cpu_temp))
                    .class(class)
                    .percentage(cpu_temp as u32)
                    .build()
                    .print();
            } else {
                error_output("No thermal data").print();
            }
        }
        "detailed" => {
            // Show comprehensive thermal info
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();
            let mut max_temp = 0.0f32;

            // CPU Package
            if let Some(cpu_temp) = data.cpu_package {
                text_parts.push(format!(" {:.0}°C", cpu_temp));
                tooltip_parts.push(format!("CPU Package: {:.1}°C", cpu_temp));
                max_temp = max_temp.max(cpu_temp);
            }

            // Core temps (in tooltip)
            if !data.cpu_cores.is_empty() {
                tooltip_parts.push("\nPer-Core Temperatures:".to_string());
                for (core_num, temp) in &data.cpu_cores {
                    tooltip_parts.push(format!("Core {}: {:.1}°C", core_num, temp));
                    max_temp = max_temp.max(*temp);
                }
            }

            // NVMe temps
            if !data.nvme_temps.is_empty() {
                tooltip_parts.push("\nNVMe Drives:".to_string());
                for (name, temp) in &data.nvme_temps {
                    text_parts.push(format!(" {:.0}°C", temp));
                    tooltip_parts.push(format!("{}: {:.1}°C", name, temp));
                    max_temp = max_temp.max(*temp);
                }
            }

            // Fan speeds
            if !data.fan_speeds.is_empty() {
                tooltip_parts.push("\nFan Speeds:".to_string());
                for (label, rpm) in &data.fan_speeds {
                    tooltip_parts.push(format!("{}: {} RPM", label, rpm));
                }
            }

            let text = text_parts.join("  ");
            let tooltip = tooltip_parts.join("\n");
            let class = determine_class(max_temp);

            WaybarOutput::builder()
                .text(text)
                .tooltip(tooltip)
                .class(class)
                .percentage(max_temp as u32)
                .build()
                .print();
        }
        "cores" => {
            // Show per-core temps
            if data.cpu_cores.is_empty() {
                error_output("No core data").print();
                return Ok(());
            }

            let temps: Vec<String> = data.cpu_cores.iter()
                .map(|(_, temp)| format!("{:.0}°", temp))
                .collect();

            let text = format!(" {}", temps.join(" "));

            let mut tooltip_parts = Vec::new();
            let mut max_temp = 0.0f32;

            for (core_num, temp) in &data.cpu_cores {
                tooltip_parts.push(format!("Core {}: {:.1}°C", core_num, temp));
                max_temp = max_temp.max(*temp);
            }

            let class = determine_class(max_temp);

            WaybarOutput::builder()
                .text(text)
                .tooltip(tooltip_parts.join("\n"))
                .class(class)
                .percentage(max_temp as u32)
                .build()
                .print();
        }
        _ => {
            eprintln!("Usage: thermal-monitor [compact|detailed|cores]");
            std::process::exit(1);
        }
    }

    Ok(())
}
