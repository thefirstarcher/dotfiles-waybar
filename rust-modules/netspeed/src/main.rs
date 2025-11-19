use waybar_common::WaybarOutput;
use std::fs;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

fn main() {
    let (down_bps, up_bps) = measure_speed();

    let down_mbps = down_bps / 1_000_000.0;
    let up_mbps = up_bps / 1_000_000.0;

    let text = if down_mbps >= 1.0 || up_mbps >= 1.0 {
        format!(" {:>4.1}↓ {:>4.1}↑", down_mbps, up_mbps)
    } else {
        let down_kbps = down_bps / 1_000.0;
        let up_kbps = up_bps / 1_000.0;
        format!(" {:>4.0}K↓ {:>4.0}K↑", down_kbps, up_kbps)
    };

    let tooltip = format!(
        "Download: {:.2} Mbps ({:.0} KB/s)\nUpload: {:.2} Mbps ({:.0} KB/s)",
        down_mbps, down_bps / 1_000.0,
        up_mbps, up_bps / 1_000.0
    );

    let max_mbps = down_mbps.max(up_mbps);
    let class = if max_mbps > 100.0 { "high" }
                else if max_mbps > 10.0 { "medium" }
                else { "low" };

    WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .class(class)
        .build()
        .print();
}

#[derive(Debug, Default)]
struct NetStats {
    rx_bytes: u64,
    tx_bytes: u64,
}

fn read_net_stats() -> HashMap<String, NetStats> {
    let content = fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let mut stats = HashMap::new();

    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        let interface = parts[0].trim_end_matches(':');

        // Skip loopback and virtual interfaces
        if interface == "lo" || interface.starts_with("docker") ||
           interface.starts_with("veth") || interface.starts_with("br-") {
            continue;
        }

        if let (Ok(rx), Ok(tx)) = (parts[1].parse::<u64>(), parts[9].parse::<u64>()) {
            stats.insert(interface.to_string(), NetStats {
                rx_bytes: rx,
                tx_bytes: tx,
            });
        }
    }

    stats
}

fn measure_speed() -> (f64, f64) {
    let stats1 = read_net_stats();
    thread::sleep(Duration::from_secs(1));
    let stats2 = read_net_stats();

    let mut total_rx_diff = 0u64;
    let mut total_tx_diff = 0u64;

    for (iface, s2) in stats2.iter() {
        if let Some(s1) = stats1.get(iface) {
            total_rx_diff += s2.rx_bytes.saturating_sub(s1.rx_bytes);
            total_tx_diff += s2.tx_bytes.saturating_sub(s1.tx_bytes);
        }
    }

    (total_rx_diff as f64, total_tx_diff as f64)
}
