#!/bin/bash
# Network speed monitor - optimized version

# Cache file for interface (avoids repeated ip route calls)
CACHE="/tmp/waybar-netspeed-iface"

# Get interface (use cache if < 30 seconds old)
if [[ -f "$CACHE" ]] && [[ $(($(date +%s) - $(stat -c %Y "$CACHE"))) -lt 30 ]]; then
    iface=$(cat "$CACHE")
else
    iface=$(ip route show default | awk 'NR==1 {print $5}')
    echo "$iface" > "$CACHE" 2>/dev/null
fi

[[ -z "$iface" ]] && { echo '{"text": "N/A", "tooltip": "No network"}'; exit 0; }

# Read stats (single read per file)
read rx1 < /sys/class/net/"$iface"/statistics/rx_bytes
read tx1 < /sys/class/net/"$iface"/statistics/tx_bytes

sleep 1

read rx2 < /sys/class/net/"$iface"/statistics/rx_bytes
read tx2 < /sys/class/net/"$iface"/statistics/tx_bytes

# Calculate speeds (KB/s)
rx_speed=$(( (rx2 - rx1) / 1024 ))
tx_speed=$(( (tx2 - tx1) / 1024 ))

# Format speeds inline (avoid function calls)
if [[ $rx_speed -lt 1024 ]]; then
    rx_fmt="${rx_speed}K"
else
    rx_fmt="$(awk "BEGIN {printf \"%.1f\", $rx_speed/1024}")M"
fi

if [[ $tx_speed -lt 1024 ]]; then
    tx_fmt="${tx_speed}K"
else
    tx_fmt="$(awk "BEGIN {printf \"%.1f\", $tx_speed/1024}")M"
fi

# Output
echo "{\"text\": \"↓${rx_fmt} ↑${tx_fmt}\", \"tooltip\": \"$iface\\n↓${rx_fmt}/s ↑${tx_fmt}/s\", \"class\": \"custom-netspeed\"}"
