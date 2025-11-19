#!/bin/bash
# Audio Visualizer Mode Controller
# Controls visualization mode and sensitivity

set -euo pipefail

MODE_FILE="/tmp/waybar-audio-viz-mode"
SENSITIVITY_FILE="/tmp/waybar-audio-viz-sensitivity"

# Get current mode
get_mode() {
    if [ -f "$MODE_FILE" ]; then
        cat "$MODE_FILE"
    else
        echo "spectrum"
    fi
}

# Get current sensitivity (0-100)
get_sensitivity() {
    if [ -f "$SENSITIVITY_FILE" ]; then
        cat "$SENSITIVITY_FILE"
    else
        echo "50"
    fi
}

# No need to update config - wrapper will read mode file on next interval

case "${1:-}" in
    "cycle")
        current=$(get_mode)
        case "$current" in
            "spectrum") new_mode="bars" ;;
            "bars") new_mode="waveform" ;;
            "waveform") new_mode="peak" ;;
            "peak") new_mode="minimal" ;;
            "minimal") new_mode="spectrum" ;;
            *) new_mode="spectrum" ;;
        esac

        # Just save the mode - wrapper will pick it up on next interval
        echo "$new_mode" > "$MODE_FILE"

        # Trigger instant update via signal
        pkill -RTMIN+10 waybar 2>/dev/null

        # Send notification if available
        if command -v notify-send &> /dev/null; then
            notify-send -u low "Audio Visualizer" "Mode: $new_mode" -t 1500
        fi
        ;;
    "sensitivity-up")
        current=$(get_sensitivity)
        new=$((current + 10))
        [ $new -gt 100 ] && new=100
        echo "$new" > "$SENSITIVITY_FILE"

        # Trigger instant update
        pkill -RTMIN+10 waybar 2>/dev/null

        if command -v notify-send &> /dev/null; then
            notify-send -u low "Audio Visualizer" "Sensitivity: $new%" -t 1000
        fi
        ;;
    "sensitivity-down")
        current=$(get_sensitivity)
        new=$((current - 10))
        [ $new -lt 0 ] && new=0
        echo "$new" > "$SENSITIVITY_FILE"

        # Trigger instant update
        pkill -RTMIN+10 waybar 2>/dev/null

        if command -v notify-send &> /dev/null; then
            notify-send -u low "Audio Visualizer" "Sensitivity: $new%" -t 1000
        fi
        ;;
    "get-mode")
        get_mode
        ;;
    "get-sensitivity")
        get_sensitivity
        ;;
    "spectrum"|"bars"|"waveform"|"peak"|"minimal")
        # Set specific mode directly
        echo "$1" > "$MODE_FILE"

        # Trigger instant update
        pkill -RTMIN+10 waybar 2>/dev/null

        if command -v notify-send &> /dev/null; then
            notify-send -u low "Audio Visualizer" "Mode: $1" -t 1500
        fi
        ;;
    *)
        echo "Usage: $0 {cycle|spectrum|bars|waveform|peak|minimal|sensitivity-up|sensitivity-down|get-mode|get-sensitivity}"
        exit 1
        ;;
esac
