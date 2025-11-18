#!/usr/bin/env bash
# Workspace-aware information display for waybar
# Shows context-sensitive information based on workspace purpose

set -euo pipefail

# Load common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWAY_SCRIPT_DIR="$HOME/.config/sway/scripts"
if [[ -f "$SWAY_SCRIPT_DIR/lib/common.sh" ]]; then
    source "$SWAY_SCRIPT_DIR/lib/common.sh"
fi

# Get current workspace info
WS_INFO=$(swaymsg -t get_workspaces | jq -r '.[] | select(.focused==true) | {num: .num, name: .name, output: .output}' 2>/dev/null)
WS_NUM=$(echo "$WS_INFO" | jq -r '.num // 1')
WS_OUTPUT=$(echo "$WS_INFO" | jq -r '.output // "unknown"')

# Get workspace category from config
WS_CATEGORY=$(get_workspace_property "$WS_NUM" "category" 2>/dev/null || echo "default")
WS_DESC=$(get_workspace_property "$WS_NUM" "description" 2>/dev/null || echo "Workspace $WS_NUM")

# Build context-specific info based on category
case "$WS_CATEGORY" in
    monitoring)
        # Show system metrics
        load=$(awk '{print $1}' /proc/loadavg 2>/dev/null || echo "?")
        containers=$(docker ps -q 2>/dev/null | wc -l || echo "0")
        text="⚙ $load"
        [[ "$containers" -gt 0 ]] && text="$text | 🐳 $containers"
        tooltip="Load: $load | Containers: $containers | $WS_DESC"
        ;;
    browser)
        # Show browser workspace info
        text="󰈹"
        tooltip="$WS_DESC"
        ;;
    communication)
        # Show notifications
        notifs=$(makoctl list 2>/dev/null | jq -r 'length' 2>/dev/null || echo "0")
        # Remove any whitespace that might cause arithmetic issues
        notifs=$(echo "$notifs" | tr -d '[:space:]')
        if [[ "${notifs:-0}" -gt 0 ]] 2>/dev/null; then
            text="󰭹 $notifs"
            tooltip="$notifs notifications | $WS_DESC"
        else
            text="󰭹"
            tooltip="No notifications | $WS_DESC"
        fi
        ;;
    terminal)
        # Show SSH sessions
        ssh_count=$(pgrep -c '^ssh$' 2>/dev/null || echo "0")
        # Remove any whitespace/newlines that might cause arithmetic issues
        ssh_count=$(echo "$ssh_count" | tr -d '[:space:]')
        if [[ "${ssh_count:-0}" -gt 0 ]] 2>/dev/null; then
            text="󰆍 $ssh_count"
            tooltip="$ssh_count SSH sessions | $WS_DESC"
        else
            text="󰆍"
            tooltip="$WS_DESC"
        fi
        ;;
    development)
        # Show code workspace info
        text="󰨞"
        tooltip="$WS_DESC"
        ;;
    *)
        # Default - show output name for multi-display awareness
        if [[ "$WS_OUTPUT" != "unknown" ]]; then
            output_short=$(echo "$WS_OUTPUT" | sed 's/HDMI-A-/H/; s/DP-/D/; s/eDP-/L/')
            text="󰍹 $output_short"
            tooltip="$WS_DESC | Output: $WS_OUTPUT"
        else
            text=""
            tooltip="$WS_DESC"
        fi
        ;;
esac

# Output JSON for waybar
echo "{\"text\": \"$text\", \"tooltip\": \"$tooltip\", \"class\": \"$WS_CATEGORY\"}"
