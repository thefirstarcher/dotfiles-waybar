use anyhow::Result;
use std::env;
use std::fs;
use std::process::Command;
use waybar_common::WaybarOutput;
use waybar_common::output::error_output;

#[derive(Debug)]
struct NetworkQuality {
    signal_strength: Option<i32>,
    latency_ms: Option<f32>,
    active_connections: usize,
    is_wireless: bool,
}

fn get_wifi_signal_strength() -> Option<i32> {
    let wireless = fs::read_to_string("/proc/net/wireless").ok()?;
    for line in wireless.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(link) = parts[2].trim_end_matches('.').parse::<i32>() {
                return Some(link);
            }
        }
    }
    None
}

fn get_latency() -> Option<f32> {
    // Ping 1.1.1.1 (Cloudflare DNS) once for quick latency check
    let output = Command::new("ping")
        .args(&["-c", "1", "-W", "1", "1.1.1.1"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Parse: time=X.XX ms
    for line in output_str.lines() {
        if line.contains("time=") {
            if let Some(time_part) = line.split("time=").nth(1) {
                if let Some(time_str) = time_part.split_whitespace().next() {
                    return time_str.parse::<f32>().ok();
                }
            }
        }
    }
    None
}

fn get_active_connections() -> usize {
    // Count active TCP connections
    if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
        content.lines().skip(1).count() // Skip header
    } else {
        0
    }
}

fn collect_network_quality() -> Result<NetworkQuality> {
    let signal_strength = get_wifi_signal_strength();
    let is_wireless = signal_strength.is_some();
    let latency_ms = get_latency();
    let active_connections = get_active_connections();

    Ok(NetworkQuality {
        signal_strength,
        latency_ms,
        active_connections,
        is_wireless,
    })
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("default");

    let quality = collect_network_quality()?;

    match mode {
        "compact" | "default" => {
            // Show signal or connection type + latency if available
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();

            if let Some(signal) = quality.signal_strength {
                let (icon, class, desc) = match signal {
                    90..=100 => ("", "excellent", "Excellent"),
                    70..=89 => ("", "good", "Good"),
                    50..=69 => ("", "fair", "Fair"),
                    30..=49 => ("", "poor", "Poor"),
                    _ => ("", "bad", "Weak"),
                };

                text_parts.push(format!("{} {}%", icon, signal));
                tooltip_parts.push(format!("WiFi Signal: {} ({}%)", desc, signal));

                WaybarOutput::builder()
                    .text(text_parts.join(" "))
                    .tooltip(tooltip_parts.join("\n"))
                    .class(class)
                    .percentage(signal as u32)
                    .build()
                    .print();
            } else {
                // Wired connection
                WaybarOutput::builder()
                    .text(" ETH")
                    .tooltip("Wired Ethernet")
                    .class("good")
                    .build()
                    .print();
            }
        }
        "detailed" => {
            // Show comprehensive network info
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();
            let mut class = "good";

            // Signal/Connection type
            if let Some(signal) = quality.signal_strength {
                let (icon, sig_class, desc) = match signal {
                    90..=100 => ("", "excellent", "Excellent"),
                    70..=89 => ("", "good", "Good"),
                    50..=69 => ("", "fair", "Fair"),
                    30..=49 => ("", "poor", "Poor"),
                    _ => ("", "bad", "Weak"),
                };

                text_parts.push(format!("{} {}%", icon, signal));
                tooltip_parts.push(format!("WiFi Signal: {} ({}%)", desc, signal));
                class = sig_class;
            } else {
                text_parts.push(" ETH".to_string());
                tooltip_parts.push("Connection: Wired Ethernet".to_string());
            }

            // Latency
            if let Some(latency) = quality.latency_ms {
                text_parts.push(format!("{}ms", latency.round() as u32));
                tooltip_parts.push(format!("Latency: {:.1}ms", latency));

                // Adjust class based on latency
                if latency > 100.0 && class == "good" {
                    class = "fair";
                } else if latency > 200.0 {
                    class = "poor";
                }
            }

            // Active connections
            if quality.active_connections > 0 {
                tooltip_parts.push(format!("Active Connections: {}", quality.active_connections));
            }

            tooltip_parts.push("\nClick for network settings".to_string());

            WaybarOutput::builder()
                .text(text_parts.join(" "))
                .tooltip(tooltip_parts.join("\n"))
                .class(class)
                .build()
                .print();
        }
        "latency" => {
            // Show only latency
            if let Some(latency) = quality.latency_ms {
                let class = match latency as u32 {
                    0..=50 => "excellent",
                    51..=100 => "good",
                    101..=200 => "fair",
                    _ => "poor",
                };

                WaybarOutput::builder()
                    .text(format!(" {}ms", latency.round() as u32))
                    .tooltip(format!("Network Latency: {:.1}ms", latency))
                    .class(class)
                    .build()
                    .print();
            } else {
                error_output("No connectivity").print();
            }
        }
        _ => {
            eprintln!("Usage: net-quality [compact|detailed|latency]");
            std::process::exit(1);
        }
    }

    Ok(())
}
