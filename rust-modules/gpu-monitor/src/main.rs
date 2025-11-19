use std::fs;
use waybar_common::WaybarOutput;

#[derive(Debug)]
struct GpuInfo {
    name: String,
    cur_freq: u32,
    max_freq: u32,
    min_freq: u32,
}

fn read_freq(path: &str) -> Option<u32> {
    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn get_gpu_name() -> String {
    // Read PCI device info
    if let Ok(content) = fs::read_to_string("/sys/class/drm/card1/device/uevent") {
        for line in content.lines() {
            if line.starts_with("PCI_ID=") {
                let pci_id = line.strip_prefix("PCI_ID=").unwrap_or("");
                // Common Intel GPU names based on PCI ID
                return match pci_id {
                    "8086:46A8" => "Intel Iris Xe".to_string(),
                    _ if pci_id.starts_with("8086:") => "Intel GPU".to_string(),
                    _ if pci_id.starts_with("10DE:") => "NVIDIA GPU".to_string(),
                    _ if pci_id.starts_with("1002:") => "AMD GPU".to_string(),
                    _ => format!("GPU {}", pci_id),
                };
            }
        }
    }
    "GPU".to_string()
}

fn get_gpu_info() -> Option<GpuInfo> {
    let base_path = "/sys/class/drm/card1/gt/gt0";
    
    Some(GpuInfo {
        name: get_gpu_name(),
        cur_freq: read_freq(&format!("{}/rps_cur_freq_mhz", base_path))?,
        max_freq: read_freq(&format!("{}/rps_max_freq_mhz", base_path))?,
        min_freq: read_freq(&format!("{}/rps_min_freq_mhz", base_path))?,
    })
}

fn main() {
    match get_gpu_info() {
        Some(info) => {
            // Calculate utilization as % of frequency range
            let freq_range = info.max_freq - info.min_freq;
            let freq_used = info.cur_freq.saturating_sub(info.min_freq);
            let utilization = if freq_range > 0 {
                ((freq_used as f32 / freq_range as f32) * 100.0) as u32
            } else {
                0
            };
            
            let class = match utilization {
                0..=30 => "idle",
                31..=70 => "active",
                _ => "high",
            };
            
            WaybarOutput::builder()
                .text(format!(" {:>3}%", utilization))
                .tooltip(format!(
                    "{}\nFrequency: {} MHz / {} MHz\nUtilization: ~{}%",
                    info.name, info.cur_freq, info.max_freq, utilization
                ))
                .class(class)
                .percentage(utilization)
                .build()
                .print();
        }
        None => {
            WaybarOutput::builder()
                .text(" N/A")
                .tooltip("GPU information unavailable")
                .class("unavailable")
                .build()
                .print();
        }
    }
}
