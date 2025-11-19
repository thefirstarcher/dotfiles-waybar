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
            "spectrum") echo "bars" > "$MODE_FILE" ;;
            "bars") echo "wave" > "$MODE_FILE" ;;
            "wave") echo "dots" > "$MODE_FILE" ;;
            "dots") echo "spectrum" > "$MODE_FILE" ;;
        esac
        # Signal Waybar to refresh
        pkill -RTMIN+8 waybar
        ;;
    "sensitivity-up")
        current=$(get_sensitivity)
        new=$((current + 10))
        [ $new -gt 100 ] && new=100
        echo "$new" > "$SENSITIVITY_FILE"
        ;;
    "sensitivity-down")
        current=$(get_sensitivity)
        new=$((current - 10))
        [ $new -lt 0 ] && new=0
        echo "$new" > "$SENSITIVITY_FILE"
        ;;
    "get-mode")
        get_mode
        ;;
    "get-sensitivity")
        get_sensitivity
        ;;
esac
