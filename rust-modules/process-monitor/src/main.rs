use anyhow::{Context, Result};
use std::env;
use std::process::Command;
use sysinfo::System;
use waybar_common::WaybarOutput;
use waybar_common::output::error_output;

#[derive(Debug)]
struct ProcessInfo {
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
}

#[derive(Debug)]
struct SystemStats {
    top_cpu: Vec<ProcessInfo>,
    top_memory: Vec<ProcessInfo>,
    total_processes: usize,
    docker_containers: Option<DockerStats>,
    systemd_failed: Option<Vec<String>>,
}

#[derive(Debug)]
struct DockerStats {
    running: usize,
    total: usize,
}

fn get_top_processes(sys: &System, count: usize) -> (Vec<ProcessInfo>, Vec<ProcessInfo>) {
    let mut processes: Vec<_> = sys.processes().iter().map(|(_, proc)| proc).collect();

    // Top CPU
    processes.sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap());
    let top_cpu: Vec<ProcessInfo> = processes
        .iter()
        .take(count)
        .filter(|p| p.cpu_usage() > 0.1)
        .map(|p| ProcessInfo {
            name: p.name().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_mb: p.memory() / 1024 / 1024,
        })
        .collect();

    // Top Memory
    processes.sort_by(|a, b| b.memory().cmp(&a.memory()));
    let top_memory: Vec<ProcessInfo> = processes
        .iter()
        .take(count)
        .filter(|p| p.memory() > 10 * 1024 * 1024) // > 10MB
        .map(|p| ProcessInfo {
            name: p.name().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_mb: p.memory() / 1024 / 1024,
        })
        .collect();

    (top_cpu, top_memory)
}

fn get_docker_stats() -> Option<DockerStats> {
    // Check if docker is available
    let output = Command::new("docker")
        .args(&["ps", "-a", "--format", "{{.State}}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let states = String::from_utf8_lossy(&output.stdout);
    let total = states.lines().count();
    let running = states.lines().filter(|s| s.trim() == "running").count();

    Some(DockerStats { running, total })
}

fn get_systemd_failed() -> Option<Vec<String>> {
    let output = Command::new("systemctl")
        .args(&["--failed", "--no-pager", "--no-legend", "--plain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let failed: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split_whitespace()
                .next()
                .unwrap_or(line)
                .to_string()
        })
        .collect();

    if failed.is_empty() {
        None
    } else {
        Some(failed)
    }
}

fn collect_system_stats() -> Result<SystemStats> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let (top_cpu, top_memory) = get_top_processes(&sys, 5);
    let total_processes = sys.processes().len();
    let docker_containers = get_docker_stats();
    let systemd_failed = get_systemd_failed();

    Ok(SystemStats {
        top_cpu,
        top_memory,
        total_processes,
        docker_containers,
        systemd_failed,
    })
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("default");

    let stats = collect_system_stats().context("Failed to collect system stats")?;

    match mode {
        "compact" | "default" => {
            // Show only process count and alerts
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();
            let mut alerts = Vec::new();

            // Process count
            text_parts.push(format!(" {}", stats.total_processes));
            tooltip_parts.push(format!("Running Processes: {}", stats.total_processes));

            // Docker status
            if let Some(docker) = &stats.docker_containers {
                if docker.running > 0 {
                    text_parts.push(format!(" {}/{}", docker.running, docker.total));
                }
                tooltip_parts.push(format!("\nDocker: {} running / {} total", docker.running, docker.total));
            }

            // Systemd failures
            if let Some(failed) = &stats.systemd_failed {
                text_parts.push(format!("⚠ {}", failed.len()));
                tooltip_parts.push(format!("\nSystemd: {} failed units", failed.len()));
                alerts.push(format!("{} failed services", failed.len()));
            }

            // Add hint for more details
            tooltip_parts.push("\n\nClick for detailed view".to_string());

            let class = if stats.systemd_failed.is_some() {
                "warning"
            } else {
                "normal"
            };

            WaybarOutput::builder()
                .text(text_parts.join("  "))
                .tooltip(tooltip_parts.join(""))
                .class(class)
                .build()
                .print();
        }
        "detailed" => {
            // Show top processes
            let mut text_parts = Vec::new();
            let mut tooltip_parts = Vec::new();

            text_parts.push(format!(" {}", stats.total_processes));

            // Top CPU processes
            if !stats.top_cpu.is_empty() {
                tooltip_parts.push("Top CPU:".to_string());
                for proc in stats.top_cpu.iter().take(3) {
                    tooltip_parts.push(format!("  {} - {:.1}%", proc.name, proc.cpu_usage));
                }
            }

            // Top Memory processes
            if !stats.top_memory.is_empty() {
                tooltip_parts.push("\nTop Memory:".to_string());
                for proc in stats.top_memory.iter().take(3) {
                    tooltip_parts.push(format!("  {} - {}MB", proc.name, proc.memory_mb));
                }
            }

            // Docker
            if let Some(docker) = &stats.docker_containers {
                tooltip_parts.push(format!("\nDocker: {} running / {} total", docker.running, docker.total));
                if docker.running > 0 {
                    text_parts.push(format!(" {}/{}", docker.running, docker.total));
                }
            }

            // Systemd failures
            if let Some(failed) = &stats.systemd_failed {
                tooltip_parts.push(format!("\nFailed Services ({}):", failed.len()));
                for service in failed.iter().take(5) {
                    tooltip_parts.push(format!("  ⚠ {}", service));
                }
                if failed.len() > 5 {
                    tooltip_parts.push(format!("  ... and {} more", failed.len() - 5));
                }
                text_parts.push(format!("⚠ {}", failed.len()));
            }

            let class = if stats.systemd_failed.is_some() {
                "critical"
            } else {
                "normal"
            };

            WaybarOutput::builder()
                .text(text_parts.join("  "))
                .tooltip(tooltip_parts.join("\n"))
                .class(class)
                .build()
                .print();
        }
        "top" => {
            // Show only top 3 CPU hogs
            if stats.top_cpu.is_empty() {
                error_output("No high CPU processes").print();
                return Ok(());
            }

            let text = format!(" {:.0}%", stats.top_cpu[0].cpu_usage);
            let mut tooltip_parts = Vec::new();

            for (i, proc) in stats.top_cpu.iter().take(3).enumerate() {
                tooltip_parts.push(format!("{}. {} - {:.1}% CPU, {}MB RAM",
                    i + 1, proc.name, proc.cpu_usage, proc.memory_mb));
            }

            WaybarOutput::builder()
                .text(text)
                .tooltip(tooltip_parts.join("\n"))
                .class("normal")
                .build()
                .print();
        }
        _ => {
            eprintln!("Usage: process-monitor [compact|detailed|top]");
            std::process::exit(1);
        }
    }

    Ok(())
}
