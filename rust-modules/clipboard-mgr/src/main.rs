use anyhow::Result;
use std::env;
use std::process::Command;
use waybar_common::WaybarOutput;

fn get_clipboard_content() -> Option<String> {
    // Try wl-paste first (Wayland)
    if let Ok(output) = Command::new("wl-paste").args(["-n"]).output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Fall back to xclip (X11)
    if let Ok(output) = Command::new("xclip").args(["-o", "-selection", "clipboard"]).output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    None
}

fn get_clipboard_history() -> Vec<String> {
    // Try cliphist first
    if let Ok(output) = Command::new("cliphist").args(["list"]).output() {
        if output.status.success() {
            let list = String::from_utf8_lossy(&output.stdout);
            return list.lines().take(10).map(|s| s.to_string()).collect();
        }
    }

    Vec::new()
}

fn clear_clipboard() -> Result<()> {
    // Clear current clipboard
    let _ = Command::new("wl-copy").args([""]).output();
    let _ = Command::new("xclip").args(["-selection", "clipboard", "/dev/null"]).output();

    // Clear cliphist history
    let _ = Command::new("cliphist").args(["wipe"]).output();

    Ok(())
}

fn show_clipboard_menu() -> Result<()> {
    // Try to use wofi or rofi with cliphist
    if Command::new("which").arg("wofi").output()?.status.success() {
        let _ = Command::new("sh")
            .args(["-c", "cliphist list | wofi --dmenu --prompt 'Clipboard' | cliphist decode | wl-copy"])
            .spawn()?;
    } else if Command::new("which").arg("rofi").output()?.status.success() {
        let _ = Command::new("sh")
            .args(["-c", "cliphist list | rofi -dmenu -p 'Clipboard' | cliphist decode | wl-copy"])
            .spawn()?;
    } else {
        // Fallback: just show cliphist list in a terminal
        let _ = Command::new("kitty")
            .args(["-e", "sh", "-c", "cliphist list | less"])
            .spawn()?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match command {
        "clear" => {
            clear_clipboard()?;
        }
        "show" => {
            show_clipboard_menu()?;
        }
        "status" | _ => {}
    }

    // Get clipboard history count
    let history = get_clipboard_history();
    let history_count = history.len();

    if let Some(content) = get_clipboard_content() {
        let trimmed = content.trim();

        if trimmed.is_empty() {
            WaybarOutput::builder()
                .text("󰨸")
                .tooltip("Clipboard: Empty\n\nClick: Show history\nRight-click: Clear")
                .class("empty")
                .build()
                .print();
        } else {
            // Show first line or truncated content
            let preview = if trimmed.len() > 50 {
                format!("{}...", &trimmed[..47])
            } else {
                trimmed.to_string()
            };

            let lines: Vec<&str> = trimmed.lines().collect();
            let line_info = if lines.len() > 1 {
                format!(" ({} lines)", lines.len())
            } else {
                String::new()
            };

            let history_info = if history_count > 0 {
                format!("\n\n📋 History: {} items", history_count)
            } else {
                String::new()
            };

            WaybarOutput::builder()
                .text(format!(" {}", history_count))
                .tooltip(format!("Clipboard{}\n{}{}\n\nClick: Show history\nRight-click: Clear",
                    line_info, preview, history_info))
                .class("has-content")
                .build()
                .print();
        }
    } else {
        WaybarOutput::builder()
            .text("󰨸")
            .tooltip("Clipboard: Empty\n\nClick: Show history\nRight-click: Clear")
            .class("empty")
            .build()
            .print();
    }

    Ok(())
}
