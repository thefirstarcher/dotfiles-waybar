use waybar_common::WaybarOutput;
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

#[derive(Debug, Clone)]
struct SinkInput {
    app_name: String,
    volume_percent: u32,
    muted: bool,
}

#[derive(Debug, Clone, Copy)]
enum DisplayMode {
    Active,    // Show all active apps with volumes
    Count,     // Just show count of playing apps
    Focused,   // Show only the focused/latest app
    List,      // List all apps (one per line in tooltip)
}

impl DisplayMode {
    fn from_str(s: &str) -> Self {
        match s {
            "active" => DisplayMode::Active,
            "count" => DisplayMode::Count,
            "focused" => DisplayMode::Focused,
            "list" => DisplayMode::List,
            _ => DisplayMode::Active,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("active");
    let display_mode = DisplayMode::from_str(mode);

    match run_mixer(display_mode) {
        Ok(output) => output.print(),
        Err(e) => {
            // Fallback when PulseAudio unavailable
            WaybarOutput::builder()
                .text("🔊 --")
                .tooltip(format!("Volume Mixer\n\nError: {}", e))
                .class("normal")
                .build()
                .print();
        }
    }
}

fn run_mixer(mode: DisplayMode) -> Result<WaybarOutput> {
    let sink_inputs = get_sink_inputs()?;

    if sink_inputs.is_empty() {
        return Ok(WaybarOutput::builder()
            .text("🔊 --")
            .tooltip("Volume Mixer\n\nNo apps playing audio")
            .class("normal")
            .build());
    }

    Ok(format_output(mode, &sink_inputs))
}

fn get_sink_inputs() -> Result<Vec<SinkInput>> {
    let output = Command::new("pactl")
        .arg("list")
        .arg("sink-inputs")
        .output()
        .context("Failed to run pactl command")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("pactl command failed"));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_pactl_output(&text)
}

fn parse_pactl_output(text: &str) -> Result<Vec<SinkInput>> {
    let mut inputs = Vec::new();
    let mut current_app: Option<String> = None;
    let mut current_volume: Option<u32> = None;
    let mut current_muted = false;

    for line in text.lines() {
        let line = line.trim();

        // Extract application name
        if line.starts_with("application.name = ") {
            current_app = Some(line
                .trim_start_matches("application.name = ")
                .trim_matches('"')
                .to_string());
        }

        // Extract volume (look for percentage)
        if line.starts_with("Volume:") {
            // Parse "Volume: front-left: 65536 / 100% / 0.00 dB, ..."
            if let Some(percent_pos) = line.find('%') {
                // Look backwards for the number
                let before_percent = &line[..percent_pos];
                if let Some(last_space) = before_percent.rfind(|c: char| c.is_whitespace() || c == '/') {
                    let number_str = before_percent[last_space + 1..].trim();
                    if let Ok(vol) = number_str.parse::<u32>() {
                        current_volume = Some(vol);
                    }
                }
            }
        }

        // Extract mute status
        if line.starts_with("Mute:") {
            current_muted = line.contains("yes");
        }

        // When we hit a new sink input or end of an entry, save the current one
        if (line.starts_with("Sink Input #") || line.is_empty()) && current_app.is_some() {
            if let (Some(app), Some(vol)) = (current_app.take(), current_volume.take()) {
                inputs.push(SinkInput {
                    app_name: app,
                    volume_percent: vol,
                    muted: current_muted,
                });
            }
            current_app = None;
            current_volume = None;
            current_muted = false;
        }
    }

    // Handle last entry
    if let (Some(app), Some(vol)) = (current_app, current_volume) {
        inputs.push(SinkInput {
            app_name: app,
            volume_percent: vol,
            muted: current_muted,
        });
    }

    Ok(inputs)
}

fn format_output(mode: DisplayMode, inputs: &[SinkInput]) -> WaybarOutput {
    let (text, class) = match mode {
        DisplayMode::Count => {
            (format!("🔊 {} app{}", inputs.len(), if inputs.len() == 1 { "" } else { "s" }), "normal")
        }
        DisplayMode::Focused => {
            if let Some(app) = inputs.last() {
                let icon = if app.muted { "🔇" } else { "🔊" };
                let text = format!("{} {} {}%", icon, truncate(&app.app_name, 15), app.volume_percent);
                let class = classify_volume(app.volume_percent);
                (text, class)
            } else {
                ("🔊 --".to_string(), "normal")
            }
        }
        DisplayMode::Active => {
            let apps_str: Vec<String> = inputs.iter().take(3).map(|app| {
                format!("{} {}%", truncate(&app.app_name, 10), app.volume_percent)
            }).collect();

            let text = if apps_str.is_empty() {
                "🔊 --".to_string()
            } else {
                format!("🔊 {}", apps_str.join(" | "))
            };

            (text, "normal")
        }
        DisplayMode::List => {
            (format!("🔊 {} apps", inputs.len()), "normal")
        }
    };

    let tooltip = generate_tooltip(inputs);

    WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .class(class)
        .build()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

fn classify_volume(volume: u32) -> &'static str {
    if volume > 100 {
        "critical"
    } else if volume > 80 {
        "warning"
    } else {
        "normal"
    }
}

fn generate_tooltip(inputs: &[SinkInput]) -> String {
    let mut tooltip = String::from("Volume Mixer\n\n");

    if inputs.is_empty() {
        tooltip.push_str("No apps playing audio");
        return tooltip;
    }

    tooltip.push_str(&format!("Active Applications ({})\n\n", inputs.len()));

    for app in inputs {
        let icon = if app.muted { "🔇" } else { "🔊" };
        let bar = volume_bar(app.volume_percent);

        tooltip.push_str(&format!(
            "{} {:20} {} {}%{}\n",
            icon,
            truncate(&app.app_name, 20),
            bar,
            app.volume_percent,
            if app.volume_percent > 100 { " ⚠" } else { "" }
        ));
    }

    tooltip.push_str("\nActions:\n");
    tooltip.push_str("• Click: Open volume control (pavucontrol)\n");
    tooltip.push_str("• Right-click: Adjust volume\n");
    tooltip.push_str("\nModes:\n");
    tooltip.push_str("• active - Show all apps with volumes\n");
    tooltip.push_str("• count - Show app count only\n");
    tooltip.push_str("• focused - Show latest app\n");
    tooltip.push_str("• list - Detailed list in tooltip");

    tooltip
}

fn volume_bar(volume: u32) -> String {
    let bars = (volume as f32 / 10.0).round() as usize;
    let filled = bars.min(10);
    let empty = 10 - filled;

    format!("[{}{}]", "━".repeat(filled), "─".repeat(empty))
}
