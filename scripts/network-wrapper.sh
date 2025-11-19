#!/bin/bash
# Network Monitor Wrapper - Merges netspeed + net-quality + vpn-manager
# Provides: Bandwidth + Latency + VPN status + Signal strength

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$(dirname "$SCRIPT_DIR")/bin"
STATE_FILE="/tmp/waybar-network-view-mode"

# Get current view mode (default: speed)
get_view_mode() {
    if [ -f "$STATE_FILE" ]; then
        cat "$STATE_FILE"
    else
        echo "speed"
    fi
}

# Set view mode from argument
case "${1:-}" in
    "detailed")
        echo "detailed" > "$STATE_FILE"
        ;;
    "compact")
        echo "compact" > "$STATE_FILE"
        ;;
    "speed")
        echo "speed" > "$STATE_FILE"
        ;;
esac

VIEW_MODE=$(get_view_mode)

# Collect data from all sources
SPEED_DATA=$("$BIN_DIR/netspeed" 2>/dev/null || echo '{"text":"N/A","tooltip":"Network Error","class":"error"}')
QUALITY_DATA=$("$BIN_DIR/net-quality" compact 2>/dev/null || echo '{"text":"","tooltip":""}')
VPN_DATA=$("$BIN_DIR/vpn-manager" status 2>/dev/null || echo '{"text":"","tooltip":""}')

# Parse JSON (handle both string and array for class)
SPEED_TEXT=$(echo "$SPEED_DATA" | jq -r '.text // "N/A"')
SPEED_TOOLTIP=$(echo "$SPEED_DATA" | jq -r '.tooltip // "Network"')
SPEED_CLASS=$(echo "$SPEED_DATA" | jq -r '.class // "normal" | if type == "array" then .[0] else . end')

QUALITY_TEXT=$(echo "$QUALITY_DATA" | jq -r '.text // ""')
QUALITY_TOOLTIP=$(echo "$QUALITY_DATA" | jq -r '.tooltip // ""')

VPN_TEXT=$(echo "$VPN_DATA" | jq -r '.text // ""')
VPN_TOOLTIP=$(echo "$VPN_DATA" | jq -r '.tooltip // ""')
VPN_CLASS=$(echo "$VPN_DATA" | jq -r '.class // "" | if type == "array" then .[0] else . end')

# Build combined output based on view mode
case "$VIEW_MODE" in
    "compact")
        # Just speed
        TEXT="$SPEED_TEXT"
        TOOLTIP="$SPEED_TOOLTIP"
        CLASS="$SPEED_CLASS"
        ;;
    "detailed")
        # Speed + Quality + VPN
        TEXT="$SPEED_TEXT"

        # Add VPN indicator if connected
        if [ -n "$VPN_TEXT" ] && [ "$VPN_CLASS" = "connected" ]; then
            TEXT="$TEXT "
        fi

        # Add quality indicator (latency or signal)
        if [ -n "$QUALITY_TEXT" ]; then
            LATENCY=$(echo "$QUALITY_TEXT" | grep -oP '\d+ms' | head -1 || echo "")
            if [ -n "$LATENCY" ]; then
                TEXT="$TEXT $LATENCY"
            fi
        fi

        # Build comprehensive tooltip
        TOOLTIP="$SPEED_TOOLTIP"
        if [ -n "$QUALITY_TOOLTIP" ] && [ "$QUALITY_TOOLTIP" != "null" ]; then
            TOOLTIP="$TOOLTIP\n\n$QUALITY_TOOLTIP"
        fi
        if [ -n "$VPN_TOOLTIP" ] && [ "$VPN_TOOLTIP" != "null" ]; then
            TOOLTIP="$TOOLTIP\n\n$VPN_TOOLTIP"
        fi
        TOOLTIP="$TOOLTIP\n\n[Scroll to change view: $VIEW_MODE]"
        CLASS="$SPEED_CLASS"
        ;;
    "speed"|*)
        # Speed with VPN indicator
        TEXT="$SPEED_TEXT"

        if [ -n "$VPN_TEXT" ] && [ "$VPN_CLASS" = "connected" ]; then
            TEXT="$TEXT "
        fi

        TOOLTIP="$SPEED_TOOLTIP\n$VPN_TOOLTIP"
        CLASS="$SPEED_CLASS"
        ;;
esac

# Output JSON for Waybar (compact, single line, stderr to /dev/null)
jq -nc \
    --arg text "$TEXT" \
    --arg tooltip "$TOOLTIP" \
    --arg class "$CLASS" \
    '{text: $text, tooltip: $tooltip, class: $class}' 2>/dev/null || echo '{"text":"Error","tooltip":"Network monitor error","class":"error"}'
