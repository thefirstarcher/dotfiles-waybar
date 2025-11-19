#!/bin/bash
# System Monitor Wrapper - Merges sys-monitor + thermal + process + disk data
# Provides: CPU + Memory + Disk + Temperature + Top processes

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$(dirname "$SCRIPT_DIR")/bin"
STATE_FILE="/tmp/waybar-system-view-mode"

# Get current view mode (default: detailed)
get_view_mode() {
    if [ -f "$STATE_FILE" ]; then
        cat "$STATE_FILE"
    else
        echo "detailed"
    fi
}

# Cycle view mode
cycle_view() {
    local current=$(get_view_mode)
    local next="detailed"

    case "$current" in
        "detailed") next="compact" ;;
        "compact") next="cpu-focus" ;;
        "cpu-focus") next="mem-focus" ;;
        "mem-focus") next="thermal-focus" ;;
        "thermal-focus") next="detailed" ;;
    esac

    echo "$next" > "$STATE_FILE"
}

# Handle arguments
case "${1:-}" in
    "cycle-up")
        cycle_view
        pkill -RTMIN+11 waybar 2>/dev/null
        exit 0
        ;;
    "cycle-down")
        cycle_view
        pkill -RTMIN+11 waybar 2>/dev/null
        exit 0
        ;;
esac

VIEW_MODE=$(get_view_mode)

# Collect data from all sources
SYS_DATA=$("$BIN_DIR/sys-monitor" detailed 2>/dev/null || echo '{"text":"N/A","tooltip":"Error","class":"error"}')
THERMAL_DATA=$("$BIN_DIR/thermal-monitor" compact 2>/dev/null || echo '{"text":"","tooltip":""}')
PROCESS_DATA=$("$BIN_DIR/process-monitor" compact 2>/dev/null || echo '{"text":"","tooltip":""}')
GPU_DATA=$("$BIN_DIR/gpu-monitor" 2>/dev/null || echo '{"text":"","tooltip":""}')

# Parse JSON using jq (handle both string and array for class)
SYS_TEXT=$(echo "$SYS_DATA" | jq -r '.text // "N/A"')
SYS_TOOLTIP=$(echo "$SYS_DATA" | jq -r '.tooltip // "System Monitor"')
SYS_CLASS=$(echo "$SYS_DATA" | jq -r '.class // "normal" | if type == "array" then .[0] else . end')

THERMAL_TEXT=$(echo "$THERMAL_DATA" | jq -r '.text // ""')
THERMAL_TOOLTIP=$(echo "$THERMAL_DATA" | jq -r '.tooltip // ""')

PROCESS_TEXT=$(echo "$PROCESS_DATA" | jq -r '.text // ""')
PROCESS_TOOLTIP=$(echo "$PROCESS_DATA" | jq -r '.tooltip // ""')

GPU_TEXT=$(echo "$GPU_DATA" | jq -r '.text // ""')
GPU_TOOLTIP=$(echo "$GPU_DATA" | jq -r '.tooltip // ""')

# Build combined output based on view mode
case "$VIEW_MODE" in
    "compact")
        # Just CPU and Memory
        TEXT=$(echo "$SYS_TEXT" | cut -d' ' -f1-4)
        TOOLTIP="$SYS_TOOLTIP"
        ;;
    "cpu-focus")
        # CPU + Thermal
        TEXT=$(echo "$SYS_TEXT" | cut -d' ' -f1-2)
        if [ -n "$THERMAL_TEXT" ]; then
            TEXT="$TEXT $THERMAL_TEXT"
        fi
        TOOLTIP="$SYS_TOOLTIP\n\n$THERMAL_TOOLTIP"
        ;;
    "mem-focus")
        # Memory focused
        TEXT=$(echo "$SYS_TEXT" | awk '{print $3, $4}')
        TOOLTIP="$SYS_TOOLTIP"
        ;;
    "thermal-focus")
        # Temperature focused
        TEXT="$THERMAL_TEXT"
        TOOLTIP="$THERMAL_TOOLTIP\n\n$SYS_TOOLTIP"
        ;;
    "detailed"|*)
        # Full: CPU + Memory + Disk + Temp
        TEXT="$SYS_TEXT"
        if [ -n "$THERMAL_TEXT" ]; then
            TEMP_ONLY=$(echo "$THERMAL_TEXT" | grep -oP '\d+°C' | head -1 || echo "")
            if [ -n "$TEMP_ONLY" ]; then
                TEXT="$TEXT  $TEMP_ONLY"
            fi
        fi

        # Build comprehensive tooltip
        TOOLTIP="$SYS_TOOLTIP"
        if [ -n "$GPU_TOOLTIP" ] && [ "$GPU_TOOLTIP" != "null" ]; then
            TOOLTIP="$TOOLTIP\n\n━━━ GPU ━━━\n$GPU_TOOLTIP"
        fi
        if [ -n "$THERMAL_TOOLTIP" ] && [ "$THERMAL_TOOLTIP" != "null" ]; then
            TOOLTIP="$TOOLTIP\n\n━━━ Thermal ━━━\n$THERMAL_TOOLTIP"
        fi
        if [ -n "$PROCESS_TOOLTIP" ] && [ "$PROCESS_TOOLTIP" != "null" ]; then
            TOOLTIP="$TOOLTIP\n\n━━━ Processes ━━━\n$PROCESS_TOOLTIP"
        fi
        TOOLTIP="$TOOLTIP\n\n[Scroll to change view: $VIEW_MODE]"
        ;;
esac

# Output JSON for Waybar (compact, single line, stderr to /dev/null)
jq -nc \
    --arg text "$TEXT" \
    --arg tooltip "$TOOLTIP" \
    --arg class "$SYS_CLASS" \
    '{text: $text, tooltip: $tooltip, class: $class}' 2>/dev/null || echo '{"text":"Error","tooltip":"System monitor error","class":"error"}'
