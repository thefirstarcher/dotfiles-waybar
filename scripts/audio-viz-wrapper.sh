#!/bin/bash
# Audio Visualizer Wrapper
# Reads current mode from state file and executes audio-viz with that mode

MODE_FILE="/tmp/waybar-audio-viz-mode"
AUDIO_VIZ_BIN="$HOME/.config/waybar/rust-modules/target/release/audio-viz"

# Get current mode (default to spectrum)
if [ -f "$MODE_FILE" ]; then
    mode=$(cat "$MODE_FILE")
else
    mode="spectrum"
fi

# Execute audio-viz with current mode
exec "$AUDIO_VIZ_BIN" "$mode"
