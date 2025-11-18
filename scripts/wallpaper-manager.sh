#!/bin/bash
# Wallpaper Manager - Manages wallpapers per theme

WALLPAPER_BASE="$HOME/Pictures/wallpapers/themes"
CURRENT_WALL_FILE="$HOME/.config/sway/current-wallpaper"

# Function to get random wallpaper for theme
get_random_wallpaper() {
    local theme=$1
    local theme_dir="$WALLPAPER_BASE/$theme"

    # If theme directory doesn't exist or is empty, return default
    if [ ! -d "$theme_dir" ]; then
        echo ""
        return
    fi

    # Find all image files recursively
    local wallpapers=()
    while IFS= read -r -d '' file; do
        wallpapers+=("$file")
    done < <(find "$theme_dir" -type f \( -iname "*.png" -o -iname "*.jpg" -o -iname "*.jpeg" \) -print0)

    # If no wallpapers found, return empty
    if [ ${#wallpapers[@]} -eq 0 ]; then
        echo ""
        return
    fi

    # Select random wallpaper
    local random_index=$((RANDOM % ${#wallpapers[@]}))
    echo "${wallpapers[$random_index]}"
}

# Function to set wallpaper
set_wallpaper() {
    local wallpaper=$1

    # If wallpaper is empty, don't change
    if [ -z "$wallpaper" ]; then
        return
    fi

    # Set wallpaper using swaymsg
    swaymsg output "*" bg "$wallpaper" fill

    # Save current wallpaper
    echo "$wallpaper" > "$CURRENT_WALL_FILE"
}

# Function to get current wallpaper
get_current_wallpaper() {
    if [ -f "$CURRENT_WALL_FILE" ]; then
        cat "$CURRENT_WALL_FILE"
    else
        echo ""
    fi
}

# Main logic
case "${1:-}" in
    set-for-theme)
        theme=${2:-tokyo-night}
        wallpaper=$(get_random_wallpaper "$theme")
        if [ -n "$wallpaper" ]; then
            set_wallpaper "$wallpaper"
            echo "Set wallpaper for $theme: $(basename "$wallpaper")"
        else
            echo "No wallpapers found for $theme"
        fi
        ;;
    set)
        wallpaper=$2
        if [ -f "$wallpaper" ]; then
            set_wallpaper "$wallpaper"
            echo "Set wallpaper: $(basename "$wallpaper")"
        else
            echo "Error: Wallpaper file not found: $wallpaper"
            exit 1
        fi
        ;;
    current)
        get_current_wallpaper
        ;;
    list)
        theme=${2:-tokyo-night}
        echo "Wallpapers for $theme:"
        find "$WALLPAPER_BASE/$theme" -type f \( -iname "*.png" -o -iname "*.jpg" -o -iname "*.jpeg" \) 2>/dev/null | while read -r file; do
            echo "  - $(basename "$file")"
        done
        ;;
    *)
        echo "Usage: $0 {set-for-theme THEME|set WALLPAPER|current|list [THEME]}"
        echo ""
        echo "Commands:"
        echo "  set-for-theme THEME    Set random wallpaper for theme"
        echo "  set WALLPAPER          Set specific wallpaper"
        echo "  current                Show current wallpaper path"
        echo "  list [THEME]           List wallpapers for theme"
        exit 1
        ;;
esac
