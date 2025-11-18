#!/bin/bash
# Waybar + Sway Theme Switcher
# Unified theme switching for waybar and sway

WAYBAR_DIR="$HOME/.config/waybar"
SWAY_DIR="$HOME/.config/sway"
THEME_DIR="$WAYBAR_DIR/themes"
SWAY_THEME_DIR="$SWAY_DIR/themes"

# Available themes
THEMES=("tokyo-night" "ayu-dark" "catppuccin-mocha" "gruvbox-dark")

# Function to get current theme
get_current_theme() {
    if [ -L "$THEME_DIR/active.css" ]; then
        basename "$(readlink "$THEME_DIR/active.css")" .css
    else
        echo "tokyo-night"
    fi
}

# Function to cycle to next theme
cycle_theme() {
    current=$(get_current_theme)
    for i in "${!THEMES[@]}"; do
        if [[ "${THEMES[$i]}" == "$current" ]]; then
            next_index=$(( (i + 1) % ${#THEMES[@]} ))
            echo "${THEMES[$next_index]}"
            return
        fi
    done
    echo "tokyo-night"
}

# Function to switch theme
switch_theme() {
    local theme=$1

    # Validate theme exists
    if [ ! -f "$THEME_DIR/${theme}.css" ]; then
        echo "Error: Theme '${theme}' not found"
        notify-send "Theme Switcher" "Theme '${theme}' not found" -u critical
        exit 1
    fi

    # Update waybar theme symlink
    ln -sf "${theme}.css" "$THEME_DIR/active.css"

    # Update sway theme symlink if sway themes exist
    if [ -d "$SWAY_THEME_DIR" ] && [ -f "$SWAY_THEME_DIR/${theme}" ]; then
        ln -sf "${theme}" "$SWAY_THEME_DIR/active"
        # Reload sway config
        swaymsg reload >/dev/null 2>&1
    fi

    # Update terminal theme symlinks
    # Kitty
    if [ -f "$HOME/.config/kitty/themes/${theme}.conf" ]; then
        ln -sf "${theme}.conf" "$HOME/.config/kitty/themes/active.conf"
        # Reload all kitty instances
        killall -SIGUSR1 kitty 2>/dev/null || true
    fi

    # Foot
    if [ -f "$HOME/.config/foot/themes/${theme}.ini" ]; then
        ln -sf "${theme}.ini" "$HOME/.config/foot/themes/active.ini"
    fi

    # Change wallpaper for theme
    "$WAYBAR_DIR/scripts/wallpaper-manager.sh" set-for-theme "$theme" >/dev/null 2>&1

    # Restart waybar to apply new theme
    pkill waybar
    sleep 0.2
    waybar &>/dev/null &
    disown

    # Send notification
    theme_name=$(echo "$theme" | sed 's/-/ /g' | sed 's/\b\(.\)/\u\1/g')
    notify-send "Theme Switcher" "Switched to ${theme_name}" -t 2000

    echo "Switched to: ${theme}"
}

# Function to show menu with wofi
show_menu() {
    current=$(get_current_theme)
    selected=$(printf '%s\n' "${THEMES[@]}" | sed 's/-/ /g; s/\b\(.\)/\u\1/g' | wofi --dmenu --prompt "Select Theme" --width 300 --height 200)

    if [ -n "$selected" ]; then
        # Convert back to theme name
        theme=$(echo "$selected" | tr '[:upper:]' '[:lower:]' | tr ' ' '-')
        switch_theme "$theme"
    fi
}

# Function to show menu with rofi
show_menu_rofi() {
    current=$(get_current_theme)
    selected=$(printf '%s\n' "${THEMES[@]}" | sed 's/-/ /g; s/\b\(.\)/\u\1/g' | rofi -dmenu -p "Theme" -theme-str 'window {width: 300px;}')

    if [ -n "$selected" ]; then
        # Convert back to theme name
        theme=$(echo "$selected" | tr '[:upper:]' '[:lower:]' | tr ' ' '-')
        switch_theme "$theme"
    fi
}

# Main script logic
case "${1:-}" in
    tokyo-night|ayu-dark|catppuccin-mocha|gruvbox-dark)
        switch_theme "$1"
        ;;
    cycle)
        next_theme=$(cycle_theme)
        switch_theme "$next_theme"
        ;;
    menu)
        # Try wofi first, fall back to rofi
        if command -v wofi &>/dev/null; then
            show_menu
        elif command -v rofi &>/dev/null; then
            show_menu_rofi
        else
            echo "Error: Neither wofi nor rofi found"
            notify-send "Theme Switcher" "Please install wofi or rofi" -u critical
            exit 1
        fi
        ;;
    current)
        get_current_theme
        ;;
    list)
        echo "Available themes:"
        printf '  - %s\n' "${THEMES[@]}"
        echo ""
        echo "Current theme: $(get_current_theme)"
        ;;
    *)
        echo "Usage: $0 {tokyo-night|ayu-dark|catppuccin-mocha|gruvbox-dark|cycle|menu|current|list}"
        echo ""
        echo "Commands:"
        echo "  tokyo-night         Switch to Tokyo Night theme"
        echo "  ayu-dark            Switch to Ayu Dark theme"
        echo "  catppuccin-mocha    Switch to Catppuccin Mocha theme"
        echo "  gruvbox-dark        Switch to Gruvbox Dark theme"
        echo "  cycle               Cycle to next theme"
        echo "  menu                Show theme selection menu (wofi/rofi)"
        echo "  current             Print current theme name"
        echo "  list                List all available themes"
        exit 1
        ;;
esac
