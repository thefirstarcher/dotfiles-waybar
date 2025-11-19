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

case "${1:-}" in
    "cycle")
        current=$(get_mode)
        case "$current" in
            "spectrum") new_mode="stereo" ;;
            "stereo")   new_mode="braille" ;;
            "braille")  new_mode="retro" ;;
            "retro")    new_mode="peak" ;;
            "peak")     new_mode="spectrum" ;;
            *)          new_mode="spectrum" ;;
        esac

        echo "$new_mode" > "$MODE_FILE"

        # Trigger instant update via signal
        pkill -RTMIN+10 waybar 2>/dev/null

        # Send notification
        if command -v notify-send &> /dev/null; then
            notify-send -u low -h string:x-canonical-private-synchronous:audio-viz \
                "Audio Visualizer" "Mode: ${new_mode^}" -t 1500
        fi
        ;;

    "sensitivity-up")
        current=$(get_sensitivity)
        new=$((current + 10))
        [ "$new" -gt 100 ] && new=100
        echo "$new" > "$SENSITIVITY_FILE"

        pkill -RTMIN+10 waybar 2>/dev/null

        if command -v notify-send &> /dev/null; then
            notify-send -u low -h int:value:"$new" -h string:x-canonical-private-synchronous:audio-viz \
                "Audio Visualizer" "Sensitivity: $new%" -t 1000
        fi
        ;;

    "sensitivity-down")
        current=$(get_sensitivity)
        new=$((current - 10))
        [ "$new" -lt 0 ] && new=0
        echo "$new" > "$SENSITIVITY_FILE"

        pkill -RTMIN+10 waybar 2>/dev/null

        if command -v notify-send &> /dev/null; then
             notify-send -u low -h int:value:"$new" -h string:x-canonical-private-synchronous:audio-viz \
                "Audio Visualizer" "Sensitivity: $new%" -t 1000
        fi
        ;;

    "get-mode")
        get_mode
        ;;

    "get-sensitivity")
        get_sensitivity
        ;;

    # Direct mode setting - Updated to match Rust VizMode
    "spectrum"|"stereo"|"braille"|"retro"|"peak")
        echo "$1" > "$MODE_FILE"

        pkill -RTMIN+10 waybar 2>/dev/null

        if command -v notify-send &> /dev/null; then
            notify-send -u low -h string:x-canonical-private-synchronous:audio-viz \
                "Audio Visualizer" "Mode: ${1^}" -t 1500
        fi
        ;;

    *)
        echo "Usage: $0 {cycle|spectrum|stereo|braille|retro|peak|sensitivity-up|sensitivity-down}"
        exit 1
        ;;
esac
