use anyhow::Result;
use std::fs;
use waybar_common::WaybarOutput;

fn get_wifi_signal_strength() -> Option<i32> {
    // Read signal strength from /proc/net/wireless
    let wireless = fs::read_to_string("/proc/net/wireless").ok()?;

    for line in wireless.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            // Signal quality is usually in the 3rd column
            if let Ok(link) = parts[2].trim_end_matches('.').parse::<i32>() {
                return Some(link);
            }
        }
    }
    None
}

fn main() -> Result<()> {
    if let Some(quality) = get_wifi_signal_strength() {
        let (icon, class, desc) = match quality {
            90..=100 => ("", "excellent", "Excellent"),
            70..=89 => ("", "good", "Good"),
            50..=69 => ("", "fair", "Fair"),
            30..=49 => ("", "poor", "Poor"),
            _ => ("", "bad", "Weak"),
        };

        WaybarOutput::builder()
            .text(format!("{} {}%", icon, quality))
            .tooltip(format!("Signal Quality: {}\n{}%", desc, quality))
            .class(class)
            .percentage(quality as u32)
            .build()
            .print();
    } else {
        // No wireless connection or ethernet
        WaybarOutput::builder()
            .text(" ETH")
            .tooltip("Wired connection")
            .class("good")
            .build()
            .print();
    }

    Ok(())
}
