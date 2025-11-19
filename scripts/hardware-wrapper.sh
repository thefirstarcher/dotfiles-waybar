#!/bin/bash
# Hardware/Peripherals Wrapper - Merges bluetooth-mgr + privacy-monitor
# Provides: Bluetooth status + Privacy indicators (mic/camera/screen)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$(dirname "$SCRIPT_DIR")/bin"

# Collect data from both sources
BT_DATA=$("$BIN_DIR/bluetooth-mgr" status 2>/dev/null || echo '{"text":"","tooltip":"Bluetooth Error","class":"error"}')
PRIVACY_DATA=$("$BIN_DIR/privacy-monitor" 2>/dev/null || echo '{"text":"","tooltip":""}')

# Parse JSON (handle both string and array for class field)
BT_TEXT=$(echo "$BT_DATA" | jq -r '.text // ""')
BT_TOOLTIP=$(echo "$BT_DATA" | jq -r '.tooltip // "Bluetooth"')
BT_CLASS=$(echo "$BT_DATA" | jq -r '.class // "disconnected" | if type == "array" then .[0] else . end')

PRIVACY_TEXT=$(echo "$PRIVACY_DATA" | jq -r '.text // ""')
PRIVACY_TOOLTIP=$(echo "$PRIVACY_DATA" | jq -r '.tooltip // ""')
PRIVACY_CLASS=$(echo "$PRIVACY_DATA" | jq -r '.class // "idle" | if type == "array" then .[0] else . end')

# Combine outputs
TEXT_PARTS=()
TOOLTIP_PARTS=()
CLASSES=()

# Add Bluetooth info
if [ -n "$BT_TEXT" ]; then
    TEXT_PARTS+=("$BT_TEXT")
fi
if [ -z "$BT_TEXT" ] && [ "$BT_CLASS" = "disabled" ]; then
    TEXT_PARTS+=("")
fi
if [ -z "$BT_TEXT" ] && [ "$BT_CLASS" = "disconnected" ]; then
    TEXT_PARTS+=("")
fi

TOOLTIP_PARTS+=("$BT_TOOLTIP")
[ -n "$BT_CLASS" ] && CLASSES+=("bt-$BT_CLASS")

# Add Privacy indicators
if [ -n "$PRIVACY_TEXT" ] && [ "$PRIVACY_CLASS" = "active" ]; then
    TEXT_PARTS+=("$PRIVACY_TEXT")
    TOOLTIP_PARTS+=("$PRIVACY_TOOLTIP")
    CLASSES+=("privacy-$PRIVACY_CLASS")
fi

# Build final output
if [ ${#TEXT_PARTS[@]} -eq 0 ]; then
    FINAL_TEXT=""
else
    FINAL_TEXT=$(IFS=' '; echo "${TEXT_PARTS[*]}")
fi

FINAL_TOOLTIP=$(printf "%s\n" "${TOOLTIP_PARTS[@]}")

# Primary class based on priority: privacy active > bt connected > bt disconnected > bt disabled
if [ "$PRIVACY_CLASS" = "active" ]; then
    PRIMARY_CLASS="privacy-active"
elif [ "$BT_CLASS" = "connected" ]; then
    PRIMARY_CLASS="connected"
elif [ "$BT_CLASS" = "disconnected" ]; then
    PRIMARY_CLASS="disconnected"
else
    PRIMARY_CLASS="disabled"
fi

# Output JSON for Waybar (compact, single line, stderr to /dev/null)
jq -nc \
    --arg text "$FINAL_TEXT" \
    --arg tooltip "$FINAL_TOOLTIP" \
    --arg class "$PRIMARY_CLASS" \
    '{text: $text, tooltip: $tooltip, class: $class}' 2>/dev/null || echo '{"text":"Error","tooltip":"Hardware monitor error","class":"error"}'
