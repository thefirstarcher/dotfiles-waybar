use anyhow::Result;
use std::env;
use std::process::Command;
use waybar_common::WaybarOutput;

fn get_vpn_status() -> (bool, Option<String>) {
    // Check if any tun/tap interfaces are up (common VPN indicator)
    if let Ok(output) = Command::new("ip").args(["link", "show"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if (line.contains("tun") || line.contains("tap") || line.contains("wg"))
                && line.contains("state UP") {
                // Extract interface name
                if let Some(name) = line.split(':').nth(1) {
                    return (true, Some(name.trim().to_string()));
                }
                return (true, Some("VPN".to_string()));
            }
        }
    }

    // Check NetworkManager VPN connections
    if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "NAME,TYPE,STATE", "connection", "show", "--active"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 && parts[1].contains("vpn") && parts[2] == "activated" {
                return (true, Some(parts[0].to_string()));
            }
        }
    }

    (false, None)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    if command == "toggle" {
        // This would need more complex logic to know which VPN to connect/disconnect
        // For now, just show status
    }

    let (connected, vpn_name) = get_vpn_status();

    if connected {
        let name = vpn_name.unwrap_or_else(|| "VPN".to_string());
        WaybarOutput::builder()
            .text(format!(" {}", name))
            .tooltip(format!("VPN Connected: {}", name))
            .class("connected")
            .build()
            .print();
    } else {
        WaybarOutput::builder()
            .text("")
            .tooltip("VPN: Disconnected")
            .class("disconnected")
            .build()
            .print();
    }

    Ok(())
}
