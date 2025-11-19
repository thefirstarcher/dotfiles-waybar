use waybar_common::WaybarOutput;
use anyhow::{Context, Result};
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use rustfft::{FftPlanner, num_complex::Complex};
use std::env;

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u8 = 2;
const BUFFER_SIZE: usize = 2048;
const NUM_BARS: usize = 8;

#[derive(Debug, Clone, Copy)]
enum VizMode {
    Spectrum,
    Bars,
    Waveform,
    Peak,
    Minimal,
}

impl VizMode {
    fn from_str(s: &str) -> Self {
        match s {
            "spectrum" => VizMode::Spectrum,
            "bars" => VizMode::Bars,
            "waveform" => VizMode::Waveform,
            "peak" => VizMode::Peak,
            "minimal" => VizMode::Minimal,
            _ => VizMode::Bars,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("bars");
    let viz_mode = VizMode::from_str(mode);

    match run_visualization(viz_mode) {
        Ok(output) => output.print(),
        Err(e) => {
            // Fallback output when no audio or error
            WaybarOutput::builder()
                .text("♪ --")
                .tooltip(format!("Audio Visualizer\n\nError: {}", e))
                .class("normal")
                .build()
                .print();
        }
    }
}

fn run_visualization(mode: VizMode) -> Result<WaybarOutput> {
    // Try to capture audio, but don't fail if no audio is playing
    match capture_audio_snapshot() {
        Ok(samples) => {
            let (levels, peak) = analyze_audio(&samples)?;
            Ok(format_output(mode, &levels, peak))
        }
        Err(_) => {
            // No audio playing - show idle state
            Ok(WaybarOutput::builder()
                .text("♪ --")
                .tooltip("Audio Visualizer\n\nNo audio playing")
                .class("normal")
                .build())
        }
    }
}

fn capture_audio_snapshot() -> Result<Vec<f32>> {
    let spec = Spec {
        format: Format::FLOAT32NE,
        channels: CHANNELS,
        rate: SAMPLE_RATE,
    };

    // Connect to PulseAudio monitor (capture sink output)
    let simple = Simple::new(
        None,                    // Use default server
        "waybar-audio-viz",      // Application name
        Direction::Record,       // We're recording
        None,                    // Use default device
        "Audio Visualizer",      // Stream name
        &spec,                   // Sample spec
        None,                    // Use default channel map
        None,                    // Use default buffering attributes
    ).context("Failed to connect to PulseAudio. Is audio playing?")?;

    // Capture audio buffer
    let mut buffer = vec![0u8; BUFFER_SIZE * 4 * CHANNELS as usize];
    simple.read(&mut buffer).context("Failed to read audio data")?;

    // Convert bytes to f32 samples (mono)
    let samples: Vec<f32> = buffer
        .chunks_exact(4)
        .step_by(CHANNELS as usize) // Take only left channel for mono
        .map(|chunk| {
            let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            f32::from_ne_bytes(bytes)
        })
        .collect();

    Ok(samples)
}

fn analyze_audio(samples: &[f32]) -> Result<(Vec<f32>, f32)> {
    // Calculate peak level
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    // Prepare FFT
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(BUFFER_SIZE);

    // Convert samples to complex numbers
    let mut complex_samples: Vec<Complex<f32>> = samples
        .iter()
        .take(BUFFER_SIZE)
        .map(|&s| Complex::new(s, 0.0))
        .collect();

    // Pad if needed
    complex_samples.resize(BUFFER_SIZE, Complex::new(0.0, 0.0));

    // Perform FFT
    fft.process(&mut complex_samples);

    // Extract magnitude spectrum (only first half, second half is mirror)
    let magnitudes: Vec<f32> = complex_samples
        .iter()
        .take(BUFFER_SIZE / 2)
        .map(|c| c.norm())
        .collect();

    // Group into frequency bands (logarithmic spacing for better visualization)
    let bands = group_into_bands(&magnitudes, NUM_BARS);

    Ok((bands, peak))
}

fn group_into_bands(magnitudes: &[f32], num_bands: usize) -> Vec<f32> {
    let total_bins = magnitudes.len();
    let mut bands = vec![0.0; num_bands];

    for (i, band) in bands.iter_mut().enumerate() {
        // Logarithmic distribution of frequency bins
        let start = (total_bins as f32 * (i as f32 / num_bands as f32).powi(2)) as usize;
        let end = (total_bins as f32 * ((i + 1) as f32 / num_bands as f32).powi(2)) as usize;
        let end = end.min(total_bins);

        if end > start {
            let sum: f32 = magnitudes[start..end].iter().sum();
            *band = sum / (end - start) as f32;
        }
    }

    // Normalize to 0-1 range
    let max_val = bands.iter().cloned().fold(0.0f32, f32::max);
    if max_val > 0.0 {
        for band in &mut bands {
            *band /= max_val;
        }
    }

    bands
}

fn format_output(mode: VizMode, levels: &[f32], peak: f32) -> WaybarOutput {
    let (text, class) = match mode {
        VizMode::Spectrum => {
            let bars = levels
                .iter()
                .map(|&level| level_to_bar(level))
                .collect::<String>();
            (format!("♪ {}", bars), classify_level(peak))
        }
        VizMode::Bars => {
            let bars = levels
                .iter()
                .map(|&level| if level > 0.1 { '|' } else { '·' })
                .collect::<String>();
            (format!("🎵 {}", bars), classify_level(peak))
        }
        VizMode::Waveform => {
            let wave = levels
                .iter()
                .map(|&level| level_to_wave(level))
                .collect::<String>();
            (format!("∿ {}", wave), classify_level(peak))
        }
        VizMode::Peak => {
            let percent = (peak * 100.0) as u32;
            (format!("🎶 {}%", percent), classify_level(peak))
        }
        VizMode::Minimal => {
            if peak > 0.01 {
                ("♪".to_string(), classify_level(peak))
            } else {
                ("♪ --".to_string(), "normal")
            }
        }
    };

    let tooltip = generate_tooltip(levels, peak);

    WaybarOutput::builder()
        .text(text)
        .tooltip(tooltip)
        .class(class)
        .percentage((peak * 100.0) as u32)
        .build()
}

fn level_to_bar(level: f32) -> char {
    // Unicode block characters for smooth visualization
    if level < 0.125 {
        '▁'
    } else if level < 0.25 {
        '▂'
    } else if level < 0.375 {
        '▃'
    } else if level < 0.5 {
        '▄'
    } else if level < 0.625 {
        '▅'
    } else if level < 0.75 {
        '▆'
    } else if level < 0.875 {
        '▇'
    } else {
        '█'
    }
}

fn level_to_wave(level: f32) -> char {
    if level < 0.2 {
        '‗'
    } else if level < 0.4 {
        '∼'
    } else if level < 0.6 {
        '≈'
    } else if level < 0.8 {
        '∽'
    } else {
        '≋'
    }
}

fn classify_level(peak: f32) -> &'static str {
    if peak > 0.8 {
        "critical"
    } else if peak > 0.5 {
        "warning"
    } else {
        "normal"
    }
}

fn generate_tooltip(levels: &[f32], peak: f32) -> String {
    let mut tooltip = String::from("Audio Visualizer\n\n");

    tooltip.push_str(&format!("Peak Level: {:.1}%\n\n", peak * 100.0));

    tooltip.push_str("Frequency Bands:\n");
    let band_names = ["Sub", "Bass", "Low", "Mid", "Hi-Mid", "High", "Presence", "Brilliance"];

    for (i, &level) in levels.iter().enumerate() {
        let name = band_names.get(i).unwrap_or(&"Band");
        let bar = level_to_bar(level);
        tooltip.push_str(&format!("{:10}: {} {:.1}%\n", name, bar, level * 100.0));
    }

    tooltip.push_str("\nModes:\n");
    tooltip.push_str("• spectrum - Full spectrum bars\n");
    tooltip.push_str("• bars - Simple bar visualization\n");
    tooltip.push_str("• waveform - Wave pattern\n");
    tooltip.push_str("• peak - Peak level only\n");
    tooltip.push_str("• minimal - Icon only");

    tooltip
}
