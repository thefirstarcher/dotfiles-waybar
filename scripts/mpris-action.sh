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
    volume-up)
        playerctl -p "$PLAYER" volume 0.05+
        ;;
    volume-down)
        playerctl -p "$PLAYER" volume 0.05-
        ;;
    seek-forward)
        playerctl -p "$PLAYER" position 10+
        ;;
    seek-backward)
        playerctl -p "$PLAYER" position 10-
        ;;
    seek-forward-short)
        playerctl -p "$PLAYER" position 5+
        ;;
    seek-backward-short)
        playerctl -p "$PLAYER" position 5-
        ;;
    *)
        echo "Usage: $0 {play-pause|next|prev|stop|volume-up|volume-down|seek-forward|seek-backward}"
        exit 1
        ;;
esac

# Trigger waybar update (signal 8)
pkill -RTMIN+8 waybar 2>/dev/null
