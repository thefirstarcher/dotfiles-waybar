use anyhow::Result;
use std::env;
use std::fs;
use std::path::Path;
use waybar_common::WaybarOutput;
use waybar_common::output::error_output;

#[derive(Debug)]
struct PowerInfo {
    on_battery: bool,
    battery_percent: Option<u8>,
    battery_status: Option<String>,
    power_profile: Option<String>,
    brightness: Option<u8>,
    power_draw_watts: Option<f32>,
}

fn read_file_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_battery_info() -> Result<PowerInfo> {
    let power_supply_path = Path::new("/sys/class/power_supply");

    let mut on_battery = false;
    let mut battery_percent = None;
    let mut battery_status = None;
    let mut power_draw_watts = None;

    // Check for battery
    if let Ok(entries) = fs::read_dir(power_supply_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Check if this is a battery
            if name.starts_with("BAT") {
                // Read capacity
                if let Some(capacity_str) = read_file_trim(&path.join("capacity")) {
                    battery_percent = capacity_str.parse::<u8>().ok();
                }

                // Read status
                battery_status = read_file_trim(&path.join("status"));

                // Read power draw (if available)
                if let Some(power_str) = read_file_trim(&path.join("power_now")) {
                    if let Ok(power_uw) = power_str.parse::<u64>() {
                        power_draw_watts = Some(power_uw as f32 / 1_000_000.0);
                    }
                }
            }

            // Check AC adapter
            if name.starts_with("AC") || name.starts_with("ADP") {
                if let Some(online) = read_file_trim(&path.join("online")) {
                    on_battery = online == "0";
                }
            }
        }
    }

    // If we have battery info but no AC info, infer from battery status
    if battery_status.is_some() && !on_battery {
        if let Some(ref status) = battery_status {
            on_battery = status == "Discharging";
        }
    }

    // Read power profile (if available)
    let power_profile = read_file_trim(Path::new("/sys/firmware/acpi/platform_profile"))
        .or_else(|| {
            // Try powerprofilesctl
            std::process::Command::new("powerprofilesctl")
                .arg("get")
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| s.trim().to_string())
        });

    // Read brightness
    let brightness = if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
        entries
            .filter_map(|e| e.ok())
            .find_map(|entry| {
                let path = entry.path();
                let max = read_file_trim(&path.join("max_brightness"))?
                    .parse::<u32>().ok()?;
                let current = read_file_trim(&path.join("brightness"))?
                    .parse::<u32>().ok()?;

                if max > 0 {
                    Some(((current as f32 / max as f32) * 100.0) as u8)
                } else {
                    None
                }
            })
    } else {
        None
    };

    Ok(PowerInfo {
        on_battery,
        battery_percent,
        battery_status,
        power_profile,
        brightness,
        power_draw_watts,
    })
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("default");

    let power_info = read_battery_info()?;

    match mode {
        "compact" | "default" => {
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();

            // Power mode indicator
            let power_icon = if power_info.on_battery { "" } else { "" };
            text_parts.push(power_icon.to_string());

            // Battery percent (if available)
            if let Some(percent) = power_info.battery_percent {
                text_parts.push(format!("{}%", percent));
                tooltip_parts.push(format!("Battery: {}%", percent));

                if let Some(ref status) = power_info.battery_status {
                    tooltip_parts.push(format!("Status: {}", status));
                }
            }

            // Brightness (if available)
            if let Some(brightness) = power_info.brightness {
                tooltip_parts.push(format!("Brightness: {}%", brightness));
            }

            // Power profile
            if let Some(ref profile) = power_info.power_profile {
                tooltip_parts.push(format!("Profile: {}", profile));
            }

            tooltip_parts.push("\nClick for power settings".to_string());

            let class = if power_info.on_battery {
                if let Some(percent) = power_info.battery_percent {
                    if percent <= 15 {
                        "critical"
                    } else if percent <= 30 {
                        "warning"
                    } else {
                        "battery"
                    }
                } else {
                    "battery"
                }
            } else {
                "charging"
            };

            WaybarOutput::builder()
                .text(text_parts.join(" "))
                .tooltip(tooltip_parts.join("\n"))
                .class(class)
                .build()
                .print();
        }
        "detailed" => {
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();

            // Power status
            let power_icon = if power_info.on_battery { "" } else { "" };
            text_parts.push(power_icon.to_string());

            tooltip_parts.push(format!("Power: {}",
                if power_info.on_battery { "On Battery" } else { "AC Power" }));

            // Battery
            if let Some(percent) = power_info.battery_percent {
                text_parts.push(format!("{}%", percent));
                tooltip_parts.push(format!("\nBattery: {}%", percent));

                if let Some(ref status) = power_info.battery_status {
                    tooltip_parts.push(format!("Status: {}", status));
                }

                // Power draw
                if let Some(watts) = power_info.power_draw_watts {
                    tooltip_parts.push(format!("Power Draw: {:.1}W", watts));
                    text_parts.push(format!("{:.1}W", watts));
                }
            }

            // Brightness
            if let Some(brightness) = power_info.brightness {
                text_parts.push(format!(" {}%", brightness));
                tooltip_parts.push(format!("\nBrightness: {}%", brightness));
            }

            // Power profile
            if let Some(ref profile) = power_info.power_profile {
                tooltip_parts.push(format!("\nPower Profile: {}", profile));

                let profile_short = match profile.as_str() {
                    "performance" => "⚡",
                    "balanced" => "⚖",
                    "power-saver" => "",
                    _ => "",
                };
                if !profile_short.is_empty() {
                    text_parts.push(profile_short.to_string());
                }
            }

            let class = if power_info.on_battery {
                if let Some(percent) = power_info.battery_percent {
                    if percent <= 15 {
                        "critical"
                    } else if percent <= 30 {
                        "warning"
                    } else {
                        "battery"
                    }
                } else {
                    "battery"
                }
            } else {
                "charging"
            };

            WaybarOutput::builder()
                .text(text_parts.join(" "))
                .tooltip(tooltip_parts.join("\n"))
                .class(class)
                .build()
                .print();
        }
        "battery-only" => {
            // Show only battery info
            if let Some(percent) = power_info.battery_percent {
                let icon = if power_info.on_battery { "" } else { "" };
                let class = if power_info.on_battery {
                    if percent <= 15 { "critical" }
                    else if percent <= 30 { "warning" }
                    else { "battery" }
                } else {
                    "charging"
                };

                WaybarOutput::builder()
                    .text(format!("{} {}%", icon, percent))
                    .tooltip(format!("Battery: {}%\nStatus: {}",
                        percent,
                        power_info.battery_status.as_deref().unwrap_or("Unknown")))
                    .class(class)
                    .percentage(percent as u32)
                    .build()
                    .print();
            } else {
                error_output("No battery found").print();
            }
        }
        _ => {
            eprintln!("Usage: power-manager [compact|detailed|battery-only]");
            std::process::exit(1);
        }
    }

    Ok(())
}
