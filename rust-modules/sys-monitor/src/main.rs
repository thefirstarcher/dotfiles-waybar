use waybar_common::WaybarOutput;
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    match mode {
        "cpu" => print_cpu(),
        "cpu-detailed" => print_cpu_detailed(),
        "memory" => print_memory(),
        "memory-detailed" => print_memory_detailed(),
        "disk" => print_disk(),
        "disk-all" => print_disk_all(),
        "all" => print_all(),
        "detailed" => print_detailed(),
        _ => {
            eprintln!("Usage: sys-monitor [cpu|cpu-detailed|memory|memory-detailed|disk|disk-all|all|detailed]");
            std::process::exit(1);
        }
    }
}

// CPU monitoring
fn read_cpu_stats() -> Vec<CpuStats> {
    let stat = fs::read_to_string("/proc/stat").unwrap_or_default();
    let mut cores = Vec::new();

    for line in stat.lines() {
        if line.starts_with("cpu") {
            if let Some(stats) = parse_cpu_line(line) {
                cores.push(stats);
            }
        }
    }

    cores
}

#[derive(Clone)]
struct CpuStats {
    name: String,
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
}

impl CpuStats {
    fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq
    }

    fn active(&self) -> u64 {
        self.user + self.nice + self.system + self.iowait + self.irq + self.softirq
    }

    fn usage_percent(&self, other: &CpuStats) -> f32 {
        let active_diff = self.active().saturating_sub(other.active()) as f32;
        let total_diff = self.total().saturating_sub(other.total()) as f32;

        if total_diff > 0.0 {
            (active_diff / total_diff) * 100.0
        } else {
            0.0
        }
    }
}

fn parse_cpu_line(line: &str) -> Option<CpuStats> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 8 {
        return None;
    }

    Some(CpuStats {
        name: parts[0].to_string(),
        user: parts[1].parse().ok()?,
        nice: parts[2].parse().ok()?,
        system: parts[3].parse().ok()?,
        idle: parts[4].parse().ok()?,
        iowait: parts[5].parse().ok()?,
        irq: parts[6].parse().ok()?,
        softirq: parts[7].parse().ok()?,
    })
}

fn read_cpu_usage() -> f32 {
    let stats1 = read_cpu_stats();
    thread::sleep(Duration::from_millis(100));
    let stats2 = read_cpu_stats();

    if let (Some(cpu1), Some(cpu2)) = (stats1.first(), stats2.first()) {
        cpu2.usage_percent(cpu1)
    } else {
        0.0
    }
}

fn read_cpu_usage_per_core() -> Vec<(usize, f32)> {
    let stats1 = read_cpu_stats();
    thread::sleep(Duration::from_millis(100));
    let stats2 = read_cpu_stats();

    stats1.iter()
        .zip(stats2.iter())
        .enumerate()
        .skip(1) // Skip "cpu" (overall)
        .map(|(i, (s1, s2))| (i - 1, s2.usage_percent(s1)))
        .collect()
}

// Memory monitoring
#[derive(Debug)]
struct MemoryInfo {
    total: u64,
    available: u64,
    free: u64,
    buffers: u64,
    cached: u64,
    slab: u64,
    swap_total: u64,
    swap_free: u64,
}

impl MemoryInfo {
    fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    fn swap_used(&self) -> u64 {
        self.swap_total.saturating_sub(self.swap_free)
    }

    fn usage_percent(&self) -> f32 {
        if self.total > 0 {
            (self.used() as f64 / self.total as f64 * 100.0) as f32
        } else {
            0.0
        }
    }
}

fn read_memory_info() -> MemoryInfo {
    let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut info = HashMap::new();

    for line in meminfo.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim().split_whitespace().next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            info.insert(key.to_string(), value);
        }
    }

    MemoryInfo {
        total: *info.get("MemTotal").unwrap_or(&0),
        available: *info.get("MemAvailable").unwrap_or(&0),
        free: *info.get("MemFree").unwrap_or(&0),
        buffers: *info.get("Buffers").unwrap_or(&0),
        cached: *info.get("Cached").unwrap_or(&0),
        slab: *info.get("Slab").unwrap_or(&0),
        swap_total: *info.get("SwapTotal").unwrap_or(&0),
        swap_free: *info.get("SwapFree").unwrap_or(&0),
    }
}

// Disk monitoring
#[derive(Debug)]
struct DiskInfo {
    mount: String,
    total: u64,
    used: u64,
    available: u64,
}

impl DiskInfo {
    fn usage_percent(&self) -> u32 {
        if self.total > 0 {
            ((self.used as f64 / self.total as f64) * 100.0) as u32
        } else {
            0
        }
    }

    fn total_gb(&self) -> f64 {
        self.total as f64 / 1024.0 / 1024.0
    }

    fn used_gb(&self) -> f64 {
        self.used as f64 / 1024.0 / 1024.0
    }
}

fn read_disk_info(path: &str) -> Option<DiskInfo> {
    let stat = nix::sys::statvfs::statvfs(path).ok()?;

    let block_size = stat.fragment_size();
    let total = stat.blocks() * block_size;
    let available = stat.blocks_available() * block_size;
    let used = total - available;

    Some(DiskInfo {
        mount: path.to_string(),
        total,
        used,
        available,
    })
}

fn read_all_disks() -> Vec<DiskInfo> {
    let mounts = fs::read_to_string("/proc/mounts").unwrap_or_default();
    let mut disks = Vec::new();

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let mount = parts[1];
            let fstype = parts.get(2).unwrap_or(&"");

            // Filter for real filesystems
            if mount.starts_with("/") &&
               !mount.starts_with("/proc") &&
               !mount.starts_with("/sys") &&
               !mount.starts_with("/dev") &&
               !mount.starts_with("/run") &&
               (*fstype == "ext4" || *fstype == "ext3" || *fstype == "btrfs" ||
                *fstype == "xfs" || *fstype == "f2fs" || *fstype == "ntfs")
            {
                if let Some(disk) = read_disk_info(mount) {
                    disks.push(disk);
                }
            }
        }
    }

    disks
}

// Print functions
fn print_cpu() {
    let cpu_usage = read_cpu_usage();

    let output = WaybarOutput::builder()
        .text(format!(" {:.0}%", cpu_usage))
        .tooltip(format!("CPU Usage: {:.1}%", cpu_usage))
        .percentage(cpu_usage as u32)
        .class(if cpu_usage > 80.0 { "critical" } else if cpu_usage > 60.0 { "warning" } else { "normal" })
        .build();

    output.print();
}

fn print_cpu_detailed() {
    let cores = read_cpu_usage_per_core();
    let overall = cores.iter().map(|(_, usage)| usage).sum::<f32>() / cores.len() as f32;

    let core_bars: Vec<String> = cores.iter()
        .map(|(i, usage)| {
            let filled = (usage / 10.0) as usize;
            let bar = "▁▂▃▄▅▆▇█";
            let char_idx = filled.min(7);
            format!("C{}: {}", i, bar.chars().nth(char_idx).unwrap_or('▁'))
        })
        .collect();

    let text = format!(" {:.0}%", overall);
    let tooltip = format!("CPU: {:.1}%\n\n{}", overall, core_bars.join("\n"));

    let output = WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .percentage(overall as u32)
        .class(if overall > 80.0 { "critical" } else if overall > 60.0 { "warning" } else { "normal" })
        .build();

    output.print();
}

fn print_memory() {
    let mem = read_memory_info();
    let used_gb = mem.used() as f64 / 1024.0 / 1024.0;
    let total_gb = mem.total as f64 / 1024.0 / 1024.0;
    let percent = mem.usage_percent() as u32;

    let output = WaybarOutput::builder()
        .text(format!(" {:.1}GB", used_gb))
        .tooltip(format!("Memory: {:.1}GB / {:.1}GB ({:.0}%)", used_gb, total_gb, percent))
        .percentage(percent)
        .class(if percent > 90 { "critical" } else if percent > 75 { "warning" } else { "normal" })
        .build();

    output.print();
}

fn print_memory_detailed() {
    let mem = read_memory_info();
    let to_gb = |kb: u64| kb as f64 / 1024.0 / 1024.0;

    let used = to_gb(mem.used());
    let total = to_gb(mem.total);
    let buffers = to_gb(mem.buffers);
    let cached = to_gb(mem.cached);
    let available = to_gb(mem.available);
    let swap_used = to_gb(mem.swap_used());
    let swap_total = to_gb(mem.swap_total);

    let percent = mem.usage_percent() as u32;

    let tooltip = format!(
        "Memory: {:.1}GB / {:.1}GB ({:.0}%)\n\
         Used: {:.1}GB\n\
         Available: {:.1}GB\n\
         Buffers: {:.1}GB\n\
         Cached: {:.1}GB\n\
         Swap: {:.1}GB / {:.1}GB",
        used, total, percent,
        used, available, buffers, cached,
        swap_used, swap_total
    );

    let output = WaybarOutput::builder()
        .text(format!(" {:.1}GB", used))
        .tooltip(tooltip)
        .percentage(percent)
        .class(if percent > 90 { "critical" } else if percent > 75 { "warning" } else { "normal" })
        .build();

    output.print();
}

fn print_disk() {
    if let Some(disk) = read_disk_info("/") {
        let percent = disk.usage_percent();

        let output = WaybarOutput::builder()
            .text(format!(" {:.0}%", percent))
            .tooltip(format!("Disk /: {:.1}GB / {:.1}GB ({:.0}% used)",
                disk.used_gb(), disk.total_gb(), percent))
            .percentage(percent)
            .class(if percent > 90 { "critical" } else if percent > 80 { "warning" } else { "normal" })
            .build();

        output.print();
    } else {
        WaybarOutput::new(" N/A").print();
    }
}

fn print_disk_all() {
    let disks = read_all_disks();

    if disks.is_empty() {
        WaybarOutput::new(" N/A").print();
        return;
    }

    let max_percent = disks.iter().map(|d| d.usage_percent()).max().unwrap_or(0);
    let total_used: f64 = disks.iter().map(|d| d.used_gb()).sum();

    let tooltip = disks.iter()
        .map(|d| format!("{}: {:.1}GB / {:.1}GB ({:.0}%)",
            d.mount, d.used_gb(), d.total_gb(), d.usage_percent()))
        .collect::<Vec<_>>()
        .join("\n");

    let output = WaybarOutput::builder()
        .text(format!(" {:.0}%", max_percent))
        .tooltip(format!("Disks:\n{}", tooltip))
        .percentage(max_percent)
        .class(if max_percent > 90 { "critical" } else if max_percent > 80 { "warning" } else { "normal" })
        .build();

    output.print();
}

fn print_all() {
    let cpu_usage = read_cpu_usage();
    let mem = read_memory_info();
    let used_mem = mem.used() as f64 / 1024.0 / 1024.0;
    let total_mem = mem.total as f64 / 1024.0 / 1024.0;
    let mem_percent = mem.usage_percent();

    let text = format!(" {:.0}%  {:.1}GB", cpu_usage, used_mem);
    let tooltip = format!(
        "CPU: {:.1}%\nMemory: {:.1}GB / {:.1}GB ({:.0}%)",
        cpu_usage, used_mem, total_mem, mem_percent
    );

    let max_percent = cpu_usage.max(mem_percent) as u32;

    let output = WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .percentage(max_percent)
        .class(if max_percent > 90 { "critical" } else if max_percent > 75 { "warning" } else { "normal" })
        .build();

    output.print();
}

fn print_detailed() {
    let cores = read_cpu_usage_per_core();
    let cpu_overall = cores.iter().map(|(_, usage)| usage).sum::<f32>() / cores.len() as f32;

    let mem = read_memory_info();
    let used_mem = mem.used() as f64 / 1024.0 / 1024.0;
    let total_mem = mem.total as f64 / 1024.0 / 1024.0;
    let mem_percent = mem.usage_percent();

    let disks = read_all_disks();
    let disk_max = disks.iter().map(|d| d.usage_percent()).max().unwrap_or(0);

    let text = format!(" {:.0}%  {:.1}GB  {:.0}%", cpu_overall, used_mem, disk_max);

    let cpu_bars: Vec<String> = cores.iter().take(8) // First 8 cores
        .map(|(i, usage)| format!("C{}: {:.0}%", i, usage))
        .collect();

    let disk_info = disks.iter()
        .map(|d| format!("{}: {:.0}%", d.mount, d.usage_percent()))
        .collect::<Vec<_>>()
        .join("\n");

    let tooltip = format!(
        "CPU: {:.1}%\n{}\n\n\
         Memory: {:.1}GB / {:.1}GB ({:.0}%)\n\n\
         Disks:\n{}",
        cpu_overall,
        cpu_bars.join(" | "),
        used_mem, total_mem, mem_percent,
        disk_info
    );

    let max_percent = cpu_overall.max(mem_percent).max(disk_max as f32) as u32;

    let output = WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .percentage(max_percent)
        .class(if max_percent > 90 { "critical" } else if max_percent > 75 { "warning" } else { "normal" })
        .build();

    output.print();
}
