#!/bin/bash
# MPRIS control script for Waybar - handles play/pause/next/prev actions

ACTION="$1"

# Try to get the first active player
PLAYER=$(playerctl -l 2>/dev/null | head -n1)

if [ -z "$PLAYER" ]; then
    notify-send "MPRIS" "No media players found"
    exit 0
fi

case "$ACTION" in
    play-pause)
        playerctl -p "$PLAYER" play-pause
        ;;
    next)
        playerctl -p "$PLAYER" next
        ;;
    prev|previous)
        playerctl -p "$PLAYER" previous
        ;;
    stop)
        playerctl -p "$PLAYER" stop
        ;;
    *)
        echo "Usage: $0 {play-pause|next|prev|stop}"
        exit 1
        ;;
esac

# Trigger waybar update (signal 8)
pkill -RTMIN+8 waybar 2>/dev/null
