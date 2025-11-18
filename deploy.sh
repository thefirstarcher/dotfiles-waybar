#!/bin/bash
# Deploy built Waybar configuration to ~/.config/waybar

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/target"
RUST_TARGET="$SCRIPT_DIR/rust-modules/target/release"
INSTALL_DIR="$HOME/.config/waybar"
BIN_DIR="$INSTALL_DIR/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}📦 Deploying Waybar configuration${NC}"
echo ""

# Check if build artifacts exist
if [ ! -f "$BUILD_DIR/config" ]; then
    echo -e "${RED}Error: Build artifacts not found. Run 'task default' first.${NC}"
    exit 1
fi

# Create bin directory
mkdir -p "$BIN_DIR"

# Install binaries from Rust target
echo -e "${YELLOW}→${NC} Installing Rust binaries..."
BINS_INSTALLED=0
for bin in sys-monitor netspeed updates-monitor gpu-monitor mpris-control \
           privacy-monitor weather-fetch bluetooth-mgr vpn-manager \
           clipboard-mgr net-quality app-volume-mixer audio-viz \
           waybar-daemon theme-switcher; do
    if [ -f "$RUST_TARGET/$bin" ]; then
        cp "$RUST_TARGET/$bin" "$BIN_DIR/"
        chmod +x "$BIN_DIR/$bin"
        BINS_INSTALLED=$((BINS_INSTALLED + 1))
    fi
done
echo -e "${GREEN}✓${NC} Installed $BINS_INSTALLED binaries to $BIN_DIR"

# Backup existing config if it exists
if [ -f "$INSTALL_DIR/config" ] && [ ! -L "$INSTALL_DIR/config" ]; then
    BACKUP="$INSTALL_DIR/config.backup-$(date +%s)"
    echo -e "${YELLOW}→${NC} Backing up existing config to $(basename $BACKUP)"
    cp "$INSTALL_DIR/config" "$BACKUP"
fi

# Backup existing style if it exists
if [ -f "$INSTALL_DIR/style.css" ] && [ ! -L "$INSTALL_DIR/style.css" ]; then
    BACKUP="$INSTALL_DIR/style.css.backup-$(date +%s)"
    echo -e "${YELLOW}→${NC} Backing up existing style to $(basename $BACKUP)"
    cp "$INSTALL_DIR/style.css" "$BACKUP"
fi

# Install config and styles
echo -e "${YELLOW}→${NC} Installing config..."
cp "$BUILD_DIR/config" "$INSTALL_DIR/config"
echo -e "${GREEN}✓${NC} Config installed"

echo -e "${YELLOW}→${NC} Installing styles..."
cp "$BUILD_DIR/style.css" "$INSTALL_DIR/style.css"
echo -e "${GREEN}✓${NC} Styles installed"

# Restart waybar
echo ""
echo -e "${YELLOW}→${NC} Restarting Waybar..."
if pgrep -x waybar > /dev/null; then
    pkill -SIGUSR2 waybar 2>/dev/null && echo -e "${GREEN}✓${NC} Waybar reloaded" || {
        killall waybar 2>/dev/null
        sleep 0.5
        waybar &>/dev/null &
        disown
        echo -e "${GREEN}✓${NC} Waybar restarted"
    }
else
    waybar &>/dev/null &
    disown
    echo -e "${GREEN}✓${NC} Waybar started"
fi

echo ""
echo -e "${GREEN}✅ Deployment complete!${NC}"
echo ""
echo -e "${BLUE}Installed to:${NC} $INSTALL_DIR"
echo -e "${BLUE}Binaries at:${NC} $BIN_DIR"
