#!/bin/bash
# Brightness Control Wrapper
# Provides: Brightness monitoring and control via scroll

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get current brightness
get_brightness() {
    if command -v brightnessctl &> /dev/null; then
        brightnessctl -m | cut -d',' -f4 | tr -d '%'
    elif [ -f /sys/class/backlight/*/brightness ]; then
        local current=$(cat /sys/class/backlight/*/brightness 2>/dev/null | head -1)
        local max=$(cat /sys/class/backlight/*/max_brightness 2>/dev/null | head -1)
        echo $((current * 100 / max))
    else
        echo "50"
    fi
}

# Set brightness
set_brightness() {
    local value="$1"
    if command -v brightnessctl &> /dev/null; then
        brightnessctl set "${value}%" >/dev/null 2>&1
    elif command -v light &> /dev/null; then
        light -S "$value" >/dev/null 2>&1
    fi
}

# Handle arguments
case "${1:-}" in
    "up")
        current=$(get_brightness)
        new=$((current + 5))
        [ $new -gt 100 ] && new=100
        set_brightness "$new"
        ;;
    "down")
        current=$(get_brightness)
        new=$((current - 5))
        [ $new -lt 1 ] && new=1
        set_brightness "$new"
        ;;
esac

# Get current brightness for display
BRIGHTNESS=$(get_brightness)

# Determine icon and class based on brightness level
if [ "$BRIGHTNESS" -ge 80 ]; then
    ICON="󰃠"
    CLASS="high"
elif [ "$BRIGHTNESS" -ge 50 ]; then
    ICON="󰃟"
    CLASS="medium"
elif [ "$BRIGHTNESS" -ge 20 ]; then
    ICON="󰃝"
    CLASS="low"
else
    ICON="󰃞"
    CLASS="very-low"
fi

TEXT="$ICON $BRIGHTNESS%"
TOOLTIP="Brightness: $BRIGHTNESS%\n\nScroll to adjust"

# Output JSON for Waybar
jq -nc \
    --arg text "$TEXT" \
    --arg tooltip "$TOOLTIP" \
    --arg class "$CLASS" \
    --argjson percentage "$BRIGHTNESS" \
    '{text: $text, tooltip: $tooltip, class: $class, percentage: $percentage}' 2>/dev/null || echo '{"text":"󰃞 N/A","tooltip":"Brightness control error","class":"error"}'
