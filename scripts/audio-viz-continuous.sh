#!/bin/bash
# Audio Visualizer Continuous Signal Sender
# Continuously sends refresh signals to waybar for real-time updates

# Send signal 10 to waybar continuously for smooth real-time visualization
while true; do
    pkill -RTMIN+10 waybar 2>/dev/null
    sleep 0.01  # 50ms = 20 FPS
done
