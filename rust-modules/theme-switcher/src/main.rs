use anyhow::{Context, Result};
use notify_rust::Notification;
use std::env;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use waybar_common::WaybarOutput;

const THEMES: &[&str] = &[
    "tokyo-night",
    "ayu-dark",
    "catppuccin-mocha",
    "gruvbox-dark",
];

struct ThemeConfig {
    waybar_dir: PathBuf,
    sway_dir: PathBuf,
    kitty_dir: PathBuf,
}

impl ThemeConfig {
    fn new() -> Result<Self> {
        let home = env::var("HOME").context("HOME not set")?;
        Ok(Self {
            waybar_dir: PathBuf::from(&home).join(".config/waybar"),
            sway_dir: PathBuf::from(&home).join(".config/sway"),
            kitty_dir: PathBuf::from(&home).join(".config/kitty"),
        })
    }
}

fn get_current_theme(config: &ThemeConfig) -> String {
    let active_link = config.waybar_dir.join("themes/active.css");

    if let Ok(target) = fs::read_link(&active_link) {
        if let Some(name) = target.file_stem() {
            return name.to_string_lossy().to_string();
        }
    }

    "ayu-dark".to_string()
}

fn get_next_theme(current: &str) -> &'static str {
    for (i, theme) in THEMES.iter().enumerate() {
        if *theme == current {
            return THEMES[(i + 1) % THEMES.len()];
        }
    }
    THEMES[0]
}

fn get_theme_display_name(theme: &str) -> String {
    theme
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn get_theme_icon(theme: &str) -> &'static str {
    match theme {
        "tokyo-night" => "󰖔",
        "ayu-dark" => "",
        "catppuccin-mocha" => "",
        "gruvbox-dark" => "󰩴",
        _ => "",
    }
}

fn switch_theme(theme: &str, config: &ThemeConfig) -> Result<()> {
    // Validate theme exists
    let waybar_theme = config.waybar_dir.join(format!("themes/{}.css", theme));
    if !waybar_theme.exists() {
        anyhow::bail!("Theme '{}' not found", theme);
    }

    // Save current wallpaper (for sway)
    let wallpaper_file = config.sway_dir.join("current-wallpaper");
    let current_wallpaper = fs::read_to_string(&wallpaper_file).ok();

    // Update waybar theme symlink
    let active_link = config.waybar_dir.join("themes/active.css");
    let _ = fs::remove_file(&active_link);
    unix_fs::symlink(format!("{}.css", theme), &active_link)?;

    // Update sway theme if it exists
    let sway_theme = config.sway_dir.join(format!("themes/{}", theme));
    if sway_theme.exists() {
        let sway_active = config.sway_dir.join("themes/active");
        let _ = fs::remove_file(&sway_active);
        unix_fs::symlink(theme, &sway_active)?;

        // Reload sway
        let _ = Command::new("swaymsg").arg("reload").output();

        // Restore wallpaper after reload
        if let Some(ref wallpaper) = current_wallpaper {
            let wallpaper = wallpaper.trim();
            if !wallpaper.is_empty() && Path::new(wallpaper).exists() {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = Command::new("swaymsg")
                    .args(["output", "*", "bg", wallpaper, "fill"])
                    .output();
            }
        }
    }

    // Update kitty theme if it exists
    let kitty_theme = config.kitty_dir.join(format!("themes/{}.conf", theme));
    if kitty_theme.exists() {
        let kitty_active = config.kitty_dir.join("themes/active.conf");
        let _ = fs::remove_file(&kitty_active);
        unix_fs::symlink(format!("{}.conf", theme), &kitty_active)?;

        // Reload kitty instances
        let _ = Command::new("killall").args(["-SIGUSR1", "kitty"]).output();
    }

    // Restart waybar
    let _ = Command::new("pkill").arg("waybar").output();
    std::thread::sleep(std::time::Duration::from_millis(200));
    Command::new("waybar")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    // Send notification
    let display_name = get_theme_display_name(theme);
    let icon = get_theme_icon(theme);
    let _ = Notification::new()
        .summary("Theme Switcher")
        .body(&format!("{} Switched to {}", icon, display_name))
        .timeout(2000)
        .show();

    Ok(())
}

fn show_menu(config: &ThemeConfig) -> Result<()> {
    let current = get_current_theme(config);

    // Prepare menu items with icons
    let menu_items: Vec<String> = THEMES
        .iter()
        .map(|theme| {
            let icon = get_theme_icon(theme);
            let name = get_theme_display_name(theme);
            let marker = if *theme == current { "✓ " } else { "  " };
            format!("{}{} {}", marker, icon, name)
        })
        .collect();

    let menu_text = menu_items.join("\n");

    // Try wofi first
    if Command::new("which").arg("wofi").output()?.status.success() {
        let mut child = Command::new("wofi")
            .args([
                "--dmenu",
                "--prompt",
                "Select Theme",
                "--width",
                "350",
                "--height",
                "250",
                "--insensitive",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(menu_text.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout);
            // Extract theme name from selection
            for theme in THEMES {
                let display = get_theme_display_name(theme);
                if selected.contains(&display) {
                    switch_theme(theme, config)?;
                    return Ok(());
                }
            }
        }
    } else if Command::new("which").arg("rofi").output()?.status.success() {
        // Fall back to rofi
        let mut child = Command::new("rofi")
            .args(["-dmenu", "-p", "Theme"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(menu_text.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout);
            for theme in THEMES {
                let display = get_theme_display_name(theme);
                if selected.contains(&display) {
                    switch_theme(theme, config)?;
                    return Ok(());
                }
            }
        }
    } else {
        Notification::new()
            .summary("Theme Switcher")
            .body("Please install wofi or rofi")
            .urgency(notify_rust::Urgency::Critical)
            .show()?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let config = ThemeConfig::new()?;
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match command {
        "status" => {
            let theme = get_current_theme(&config);
            let icon = get_theme_icon(&theme);
            let display_name = get_theme_display_name(&theme);

            WaybarOutput::builder()
                .text(format!("{}", icon))
                .tooltip(format!(
                    "Theme: {}\n\nClick: Menu\nRight-click: Cycle",
                    display_name
                ))
                .class("theme-switcher")
                .build()
                .print();
        }
        "cycle" => {
            let current = get_current_theme(&config);
            let next = get_next_theme(&current);
            switch_theme(next, &config)?;

            // Output new status
            let icon = get_theme_icon(next);
            let display_name = get_theme_display_name(next);
            WaybarOutput::builder()
                .text(format!("{}", icon))
                .tooltip(format!("Theme: {}", display_name))
                .class("theme-switcher")
                .build()
                .print();
        }
        "menu" => {
            show_menu(&config)?;
        }
        theme if THEMES.contains(&theme) => {
            switch_theme(theme, &config)?;
        }
        "list" => {
            println!("Available themes:");
            for theme in THEMES {
                println!(
                    "  {} {} {}",
                    get_theme_icon(theme),
                    theme,
                    get_theme_display_name(theme)
                );
            }
            println!("\nCurrent: {}", get_current_theme(&config));
        }
        _ => {
            eprintln!("Usage: theme-switcher {{status|cycle|menu|list|<theme-name>}}");
            eprintln!("\nAvailable themes:");
            for theme in THEMES {
                eprintln!("  - {}", theme);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}
