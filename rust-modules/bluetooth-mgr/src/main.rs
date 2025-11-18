use anyhow::Result;
use std::env;
use std::process::Command;
use waybar_common::WaybarOutput;

fn is_bluetooth_enabled() -> bool {
    Command::new("bluetoothctl")
        .args(["show"])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout).contains("Powered: yes")
        })
        .unwrap_or(false)
}

fn get_connected_devices() -> Vec<String> {
    Command::new("bluetoothctl")
        .args(["devices", "Connected"])
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Some(
                stdout
                    .lines()
                    .filter(|line| line.contains("Device"))
                    .map(|line| {
                        line.split_whitespace()
                            .skip(2)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect()
            )
        })
        .unwrap_or_default()
}

fn toggle_bluetooth() -> Result<()> {
    let enabled = is_bluetooth_enabled();
    let cmd = if enabled { "off" } else { "on" };

    Command::new("bluetoothctl")
        .args(["power", cmd])
        .output()?;

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match command {
        "toggle" => {
            toggle_bluetooth()?;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        "status" | _ => {}
    }

    let enabled = is_bluetooth_enabled();
    let devices = get_connected_devices();

    let (icon, text, class, tooltip) = if !enabled {
        ("".to_string(), "".to_string(), "disabled", "Bluetooth: Disabled".to_string())
    } else if devices.is_empty() {
        ("".to_string(), "".to_string(), "disconnected", "Bluetooth: No devices connected".to_string())
    } else {
        let device_list = devices.join("\n");
        let count = devices.len();
        (
            "".to_string(),
            format!(" {}", count),
            "connected",
            format!("Bluetooth: {} device(s)\n{}", count, device_list),
        )
    };

    WaybarOutput::builder()
        .text(if text.is_empty() {
            icon
        } else {
            text
        })
        .tooltip(&tooltip)
        .class(class)
        .build()
        .print();

    Ok(())
}
