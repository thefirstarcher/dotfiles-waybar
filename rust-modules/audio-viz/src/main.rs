use anyhow::{Context, Result};
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use rustfft::{num_complex::Complex, FftPlanner};
use std::env;
use waybar_common::WaybarOutput;

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u8 = 2;
const BUFFER_SIZE: usize = 2048;
const NUM_BARS: usize = 10; // Increased slightly for better resolution

#[derive(Debug, Clone, Copy)]
enum VizMode {
    Spectrum, // Standard Blocks
    Stereo,   // Split Left/Right
    Braille,  // High Res Dots
    Retro,    // ASCII style
    Peak,     // Just volume
}

impl VizMode {
    fn from_str(s: &str) -> Self {
        match s {
            "spectrum" => VizMode::Spectrum,
            "stereo" => VizMode::Stereo,
            "braille" => VizMode::Braille,
            "retro" => VizMode::Retro,
            "peak" => VizMode::Peak,
            _ => VizMode::Spectrum,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("spectrum");
    let viz_mode = VizMode::from_str(mode);

    match run_visualization(viz_mode) {
        Ok(output) => output.print(),
        Err(e) => {
            WaybarOutput::builder()
                .text(" err")
                .tooltip(format!("Error: {}", e))
                .build()
                .print();
        }
    }
}

fn run_visualization(mode: VizMode) -> Result<WaybarOutput> {
    match capture_audio_snapshot() {
        // We now get Left and Right channels separately
        Ok((left_samples, right_samples)) => {
            // Analyze Left
            let (left_bands, left_peak) = analyze_channel(&left_samples)?;

            // Analyze Right (Only needed for Stereo mode, but cheap to calculate)
            let (right_bands, right_peak) = analyze_channel(&right_samples)?;

            // Average peak for general display
            let peak = (left_peak + right_peak) / 2.0;

            Ok(format_output(mode, &left_bands, &right_bands, peak))
        }
        Err(_) => Ok(WaybarOutput::builder().text(" --").build()),
    }
}

// Updated to return a tuple of (Left Channel, Right Channel)
fn capture_audio_snapshot() -> Result<(Vec<f32>, Vec<f32>)> {
    let spec = Spec {
        format: Format::FLOAT32NE,
        channels: CHANNELS,
        rate: SAMPLE_RATE,
    };

    let simple = Simple::new(
        None,
        "waybar-viz",
        Direction::Record,
        None,
        "Visualization",
        &spec,
        None,
        None,
    )
    .context("PulseAudio Error")?;

    let mut buffer = vec![0u8; BUFFER_SIZE * 4 * CHANNELS as usize];
    simple.read(&mut buffer).context("Read Error")?;

    let mut left = Vec::with_capacity(BUFFER_SIZE);
    let mut right = Vec::with_capacity(BUFFER_SIZE);

    // De-interleave the stereo buffer
    for chunk in buffer.chunks_exact(4 * CHANNELS as usize) {
        let l_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
        let r_bytes = [chunk[4], chunk[5], chunk[6], chunk[7]];

        left.push(f32::from_ne_bytes(l_bytes));
        right.push(f32::from_ne_bytes(r_bytes));
    }

    Ok((left, right))
}

fn analyze_channel(samples: &[f32]) -> Result<(Vec<f32>, f32)> {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(BUFFER_SIZE);

    let mut complex_samples: Vec<Complex<f32>> = samples
        .iter()
        .take(BUFFER_SIZE)
        .enumerate()
        .map(|(i, &s)| {
            let window =
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / BUFFER_SIZE as f32).cos());
            Complex::new(s * window, 0.0)
        })
        .collect();

    complex_samples.resize(BUFFER_SIZE, Complex::new(0.0, 0.0));
    fft.process(&mut complex_samples);

    let magnitudes: Vec<f32> = complex_samples
        .iter()
        .skip(2)
        .take(BUFFER_SIZE / 2)
        .map(|c| c.norm())
        .collect();

    // Reuse the improved pink-noise logic from before
    let bands = group_into_bands(&magnitudes, NUM_BARS);
    Ok((bands, peak))
}

fn group_into_bands(magnitudes: &[f32], num_bands: usize) -> Vec<f32> {
    let total_bins = magnitudes.len();
    let mut bands = vec![0.0; num_bands];

    for (i, band) in bands.iter_mut().enumerate() {
        let start = (total_bins as f32 * (i as f32 / num_bands as f32).powi(2)) as usize;
        let end = (total_bins as f32 * ((i + 1) as f32 / num_bands as f32).powi(2)) as usize;
        let start = start.min(total_bins.saturating_sub(1));
        let end = end.max(start + 1).min(total_bins);

        let sum: f32 = magnitudes[start..end].iter().sum();
        let avg = sum / (end - start) as f32;

        // Pink noise compensation + Soft limiter
        let bias = 1.0 + (i as f32 * 0.8);
        *band = (avg * bias).powf(0.6);
    }

    let max_val = bands.iter().cloned().fold(0.0f32, f32::max);
    if max_val > 0.0001 {
        for band in &mut bands {
            *band /= max_val;
        }
    }
    bands
}

fn format_output(mode: VizMode, left: &[f32], right: &[f32], peak: f32) -> WaybarOutput {
    let text = match mode {
        // 1. Standard Block Mode
        VizMode::Spectrum => left.iter().map(|&l| get_bar_char(l)).collect::<String>(),

        // 2. Stereo Mode: Left grows Left, Right grows Right (Center split)
        // Looks like:  ▂▃▄▅ | ▅▄▃▂
        VizMode::Stereo => {
            let l_str: String = left
                .iter()
                .rev()
                .take(5)
                .map(|&l| get_bar_char(l))
                .collect();
            let r_str: String = right.iter().take(5).map(|&r| get_bar_char(r)).collect();
            format!("{}│{}", l_str, r_str)
        }

        // 3. Braille Mode (High Resolution)
        // Uses patterns like ⣿ for full and ⣀ for low
        VizMode::Braille => left
            .iter()
            .map(|&l| get_braille_char(l))
            .collect::<String>(),

        // 4. Retro Mode (ASCII)
        // Uses = and -
        VizMode::Retro => left
            .iter()
            .map(|&l| {
                if l > 0.6 {
                    '='
                } else if l > 0.3 {
                    '-'
                } else {
                    '_'
                }
            })
            .collect::<String>(),

        VizMode::Peak => format!("VOL {:.0}%", peak * 100.0),
    };

    WaybarOutput::builder()
        .text(text)
        .tooltip(format!("Mode: {:?}\nPeak: {:.1}%", mode, peak * 100.0))
        .class(if peak > 0.8 { "critical" } else { "normal" })
        .build()
}

fn get_bar_char(level: f32) -> char {
    match level {
        l if l < 0.125 => ' ',
        l if l < 0.25 => '▂',
        l if l < 0.375 => '▃',
        l if l < 0.5 => '▄',
        l if l < 0.625 => '▅',
        l if l < 0.75 => '▆',
        l if l < 0.875 => '▇',
        _ => '█',
    }
}

// New Braille Character Map
fn get_braille_char(level: f32) -> char {
    match level {
        l if l < 0.1 => '⠀', // Empty
        l if l < 0.2 => '⡀',
        l if l < 0.35 => '⣀',
        l if l < 0.5 => '⣄',
        l if l < 0.6 => '⣤',
        l if l < 0.75 => '⣦',
        l if l < 0.9 => '⣶',
        _ => '⣿', // Full
    }
}
