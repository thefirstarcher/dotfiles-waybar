use std::fs;
use std::path::Path;
use waybar_common::WaybarOutput;

#[derive(Debug, Default)]
struct PrivacyStatus {
    mic_active: bool,
    camera_active: bool,
    screen_sharing: bool,
}

fn check_microphone() -> bool {
    // Check ALSA capture devices
    if let Ok(entries) = fs::read_dir("/proc/asound") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().unwrap().to_string_lossy().starts_with("card") {
                let pcm_path = path.join("pcm0c");
                if pcm_path.exists() {
                    if let Ok(sub_entries) = fs::read_dir(&pcm_path) {
                        for sub in sub_entries.flatten() {
                            let status_file = sub.path().join("status");
                            if let Ok(content) = fs::read_to_string(&status_file) {
                                if content.contains("state: RUNNING") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Check PulseAudio/PipeWire sources
    if let Ok(output) = std::process::Command::new("pactl")
        .args(["list", "source-outputs"])
        .output()
    {
        let out = String::from_utf8_lossy(&output.stdout);
        if out.contains("Source Output") {
            return true;
        }
    }
    
    false
}

fn check_camera() -> bool {
    // Check /sys/class/video4linux devices
    if let Ok(entries) = fs::read_dir("/sys/class/video4linux") {
        for entry in entries.flatten() {
            let dev_path = Path::new("/dev").join(entry.file_name());
            
            // Check if device is open by checking fuser or lsof
            if let Ok(output) = std::process::Command::new("fuser")
                .arg(&dev_path)
                .output()
            {
                if !output.stdout.is_empty() {
                    return true;
                }
            }
        }
    }
    
    false
}

fn check_screen_sharing() -> bool {
    // Check for common screen recording/sharing processes
    let screen_share_procs = [
        "wf-recorder",
        "obs",
        "ffmpeg",
        "grim",
        "slurp",
        "xdg-desktop-portal",
    ];
    
    if let Ok(output) = std::process::Command::new("pgrep")
        .arg("-x")
        .arg(screen_share_procs.join("|"))
        .output()
    {
        return !output.stdout.is_empty();
    }
    
    // Check for active Wayland screen capture
    if let Ok(output) = std::process::Command::new("ps")
        .args(["aux"])
        .output()
    {
        let out = String::from_utf8_lossy(&output.stdout);
        for proc in &screen_share_procs {
            if out.contains(proc) {
                return true;
            }
        }
    }
    
    false
}

fn main() {
    let status = PrivacyStatus {
        mic_active: check_microphone(),
        camera_active: check_camera(),
        screen_sharing: check_screen_sharing(),
    };
    
    let mut icons = Vec::new();
    let mut tooltip_lines = Vec::new();
    let mut classes: Vec<String> = Vec::new();

    if status.mic_active {
        icons.push("");
        tooltip_lines.push("Microphone: Active");
        classes.push("mic-active".to_string());
    }

    if status.camera_active {
        icons.push("");
        tooltip_lines.push("Camera: Active");
        classes.push("camera-active".to_string());
    }

    if status.screen_sharing {
        icons.push("");
        tooltip_lines.push("Screen Sharing: Active");
        classes.push("screen-active".to_string());
    }
    
    let (text, tooltip, class) = if icons.is_empty() {
        (
            "".to_string(),
            "Privacy: No active recordings".to_string(),
            "idle",
        )
    } else {
        (
            icons.join(" "),
            tooltip_lines.join("\n"),
            "active",
        )
    };
    
    WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .class(class)
        .classes(classes)
        .build()
        .print();
}
