use std::process::Command;
use waybar_common::WaybarOutput;

fn main() {
    let updates = check_updates();

    let (icon, text) = if updates > 0 {
        ("󰓧", format!(" {}", updates))
    } else {
        ("󰓦", String::from(""))
    };

    let tooltip = if updates > 0 {
        format!(
            "{} {} update{} available\n\nClick: Update system",
            icon,
            updates,
            if updates == 1 { "" } else { "s" }
        )
    } else {
        format!("{} System is up to date", icon)
    };

    let class = if updates > 50 {
        "critical"
    } else if updates > 20 {
        "warning"
    } else if updates > 0 {
        "updates-available"
    } else {
        "up-to-date"
    };

    WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .class(class)
        .build()
        .print();
}

fn check_updates() -> usize {
    // Check for pacman updates
    let output = Command::new("checkupdates").output().ok();

    if let Some(output) = output {
        if output.status.success() {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            return count;
        }
    }

    // Fallback: try pacman -Qu
    let output = Command::new("pacman").args(["-Qu"]).output().ok();

    if let Some(output) = output {
        return String::from_utf8_lossy(&output.stdout).lines().count();
    }

    0
}
