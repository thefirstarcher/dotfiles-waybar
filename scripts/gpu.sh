#!/bin/bash
# GPU monitor - optimized with type detection cache

CACHE="/tmp/waybar-gpu-type"

# Detect GPU type once and cache it
if [[ ! -f "$CACHE" ]]; then
    if command -v nvidia-smi &>/dev/null; then
        echo "nvidia" > "$CACHE"
    elif command -v radeontop &>/dev/null; then
        echo "amd" > "$CACHE"
    elif [[ -f /sys/class/drm/card0/gt_cur_freq_mhz ]]; then
        echo "intel" > "$CACHE"
    else
        echo "none" > "$CACHE"
    fi
fi

gpu_type=$(cat "$CACHE")

case "$gpu_type" in
    nvidia)
        # Single nvidia-smi call with all fields
        read -r usage temp name mem_used mem_total <<< $(nvidia-smi --query-gpu=utilization.gpu,temperature.gpu,name,memory.used,memory.total --format=csv,noheader,nounits)
        echo "{\"text\": \"${usage}%\", \"tooltip\": \"$name\\nUsage: ${usage}% | Temp: ${temp}°C\\nMem: ${mem_used}/${mem_total} MB\", \"class\": \"custom-gpu\"}"
        ;;
    amd)
        usage=$(timeout 1 radeontop -d - -l 1 2>/dev/null | grep -o 'gpu [0-9]*' | awk '{print $2}')
        [[ -n "$usage" ]] && echo "{\"text\": \"${usage}%\", \"tooltip\": \"AMD GPU\\nUsage: ${usage}%\", \"class\": \"custom-gpu\"}" || echo '{"text": "N/A", "class": "gpu-disabled"}'
        ;;
    intel)
        read freq < /sys/class/drm/card0/gt_cur_freq_mhz
        read max_freq < /sys/class/drm/card0/gt_max_freq_mhz
        [[ $max_freq -gt 0 ]] && usage=$(awk "BEGIN {printf \"%.0f\", ($freq/$max_freq)*100}") || usage="N/A"
        echo "{\"text\": \"${usage}%\", \"tooltip\": \"Intel GPU\\n${freq}MHz / ${max_freq}MHz\\nUsage: ${usage}%\", \"class\": \"custom-gpu\"}"
        ;;
    *)
        echo '{"text": "N/A", "tooltip": "No GPU detected", "class": "gpu-disabled"}'
        ;;
esac
