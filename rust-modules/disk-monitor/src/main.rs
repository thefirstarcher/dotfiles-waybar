use anyhow::{Context, Result};
use std::env;
use std::fs;
use sysinfo::{Disks};
use waybar_common::WaybarOutput;
use waybar_common::output::error_output;

#[derive(Debug)]
struct DiskUsage {
    mount_point: String,
    total_gb: u64,
    used_gb: u64,
    available_gb: u64,
    usage_percent: u8,
}

#[derive(Debug)]
struct DiskIO {
    device: String,
    read_kb_s: f64,
    write_kb_s: f64,
}

fn get_disk_usage() -> Result<Vec<DiskUsage>> {
    let disks = Disks::new_with_refreshed_list();
    let mut usage_list = Vec::new();

    for disk in disks.list() {
        let mount_point = disk.mount_point().to_string_lossy().to_string();
        let total_space = disk.total_space();
        let available_space = disk.available_space();
        let used_space = total_space.saturating_sub(available_space);

        let total_gb = total_space / 1024 / 1024 / 1024;
        let used_gb = used_space / 1024 / 1024 / 1024;
        let available_gb = available_space / 1024 / 1024 / 1024;

        let usage_percent = if total_space > 0 {
            ((used_space as f64 / total_space as f64) * 100.0) as u8
        } else {
            0
        };

        usage_list.push(DiskUsage {
            mount_point,
            total_gb,
            used_gb,
            available_gb,
            usage_percent,
        });
    }

    // Sort by mount point for consistent ordering
    usage_list.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));

    Ok(usage_list)
}

fn get_disk_io() -> Result<Vec<DiskIO>> {
    let diskstats = fs::read_to_string("/proc/diskstats")
        .context("Failed to read /proc/diskstats")?;

    let mut io_list = Vec::new();

    for line in diskstats.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 {
            continue;
        }

        let device = parts[2];

        // Skip loop, ram, and partition devices - only show main disks
        if device.starts_with("loop")
            || device.starts_with("ram")
            || device.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            continue;
        }

        // Read sectors and write sectors (1 sector = 512 bytes)
        let read_sectors = parts[5].parse::<u64>().unwrap_or(0);
        let write_sectors = parts[9].parse::<u64>().unwrap_or(0);

        // For now, just report the cumulative values
        // In a real implementation, we'd store previous values and calculate delta
        let read_kb_s = (read_sectors * 512) as f64 / 1024.0;
        let write_kb_s = (write_sectors * 512) as f64 / 1024.0;

        io_list.push(DiskIO {
            device: device.to_string(),
            read_kb_s,
            write_kb_s,
        });
    }

    Ok(io_list)
}

fn determine_class(usage_percent: u8) -> &'static str {
    match usage_percent {
        0..=70 => "normal",
        71..=85 => "warning",
        _ => "critical",
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("default");

    let disk_usage = get_disk_usage()?;

    if disk_usage.is_empty() {
        error_output("No disks found").print();
        return Ok(());
    }

    match mode {
        "compact" | "default" => {
            // Show only root partition usage
            let root = disk_usage.iter()
                .find(|d| d.mount_point == "/")
                .or_else(|| disk_usage.first())
                .context("No disk found")?;

            let class = determine_class(root.usage_percent);

            WaybarOutput::builder()
                .text(format!(" {}%", root.usage_percent))
                .tooltip(format!("Disk Usage: {}\n{} / {} GB used\n{} GB available\n\nClick for all partitions",
                    root.mount_point,
                    root.used_gb,
                    root.total_gb,
                    root.available_gb))
                .class(class)
                .percentage(root.usage_percent as u32)
                .build()
                .print();
        }
        "detailed" => {
            // Show all partitions
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();
            let mut max_usage = 0u8;

            for disk in &disk_usage {
                // Skip very small mounts (likely special filesystems)
                if disk.total_gb < 1 {
                    continue;
                }

                text_parts.push(format!("{}%", disk.usage_percent));
                tooltip_parts.push(format!("{}: {}% ({}/{}GB)",
                    disk.mount_point,
                    disk.usage_percent,
                    disk.used_gb,
                    disk.total_gb));
                max_usage = max_usage.max(disk.usage_percent);
            }

            let class = determine_class(max_usage);
            let text = if text_parts.len() > 3 {
                format!(" {}", text_parts[..3].join(" "))
            } else {
                format!(" {}", text_parts.join(" "))
            };

            WaybarOutput::builder()
                .text(text)
                .tooltip(tooltip_parts.join("\n"))
                .class(class)
                .percentage(max_usage as u32)
                .build()
                .print();
        }
        "io" => {
            // Show I/O activity
            let io_stats = get_disk_io()?;

            if io_stats.is_empty() {
                error_output("No I/O stats").print();
                return Ok(());
            }

            let mut tooltip_parts = Vec::new();
            tooltip_parts.push("Disk I/O Activity:".to_string());

            for io in &io_stats {
                tooltip_parts.push(format!("{}: R:{:.1}MB W:{:.1}MB (cumulative)",
                    io.device,
                    io.read_kb_s / 1024.0,
                    io.write_kb_s / 1024.0));
            }

            // For display, just show that I/O monitoring is active
            WaybarOutput::builder()
                .text(" I/O")
                .tooltip(tooltip_parts.join("\n"))
                .class("normal")
                .build()
                .print();
        }
        "all" => {
            // Combined: disk usage + I/O hint
            let root = disk_usage.iter()
                .find(|d| d.mount_point == "/")
                .or_else(|| disk_usage.first())
                .context("No disk found")?;

            let mut tooltip_parts = Vec::new();
            tooltip_parts.push("Disk Usage:".to_string());

            for disk in &disk_usage {
                if disk.total_gb < 1 {
                    continue;
                }
                tooltip_parts.push(format!("  {}: {}% ({}/{}GB)",
                    disk.mount_point,
                    disk.usage_percent,
                    disk.used_gb,
                    disk.total_gb));
            }

            let class = determine_class(root.usage_percent);

            WaybarOutput::builder()
                .text(format!(" {}%", root.usage_percent))
                .tooltip(tooltip_parts.join("\n"))
                .class(class)
                .percentage(root.usage_percent as u32)
                .build()
                .print();
        }
        _ => {
            eprintln!("Usage: disk-monitor [compact|detailed|io|all]");
            std::process::exit(1);
        }
    }

    Ok(())
}
