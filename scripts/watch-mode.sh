#!/bin/bash
# Watch Mode - Auto-rebuild and reload Waybar on file changes
# Usage: ./scripts/watch-mode.sh

set -e

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}🔍 Waybar Watch Mode${NC}"
echo -e "${BLUE}===================${NC}"
echo ""
echo -e "${YELLOW}Watching for changes in:${NC}"
echo "  • config-src/**/*.json"
echo "  • styles/**/*.css"
echo "  • rust-modules/*/src/**/*.rs"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
echo ""

# Check if inotifywait is available
if ! command -v inotifywait &> /dev/null; then
    echo -e "${RED}Error: inotifywait not found${NC}"
    echo "Install it with: sudo pacman -S inotify-tools"
    exit 1
fi

# Function to handle config changes
handle_config_change() {
    local file="$1"
    echo -e "${YELLOW}→${NC} Config changed: ${file}"
    echo -e "${YELLOW}→${NC} Rebuilding config..."

    cd "$PROJECT_DIR"
    if ./scripts/merge-configs.sh target/config; then
        echo -e "${GREEN}✓${NC} Config rebuilt"
        reload_waybar
    else
        echo -e "${RED}✗${NC} Config build failed"
    fi
}

# Function to handle style changes
handle_style_change() {
    local file="$1"
    echo -e "${YELLOW}→${NC} Style changed: ${file}"
    echo -e "${YELLOW}→${NC} Rebuilding styles..."

    cd "$PROJECT_DIR"
    if command -v sass &> /dev/null; then
        sass styles/main.css target/style.css --no-source-map 2>&1 | grep -v "Deprecation"
    else
        cp styles/main.css target/style.css
    fi

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Styles rebuilt"
        reload_waybar
    else
        echo -e "${RED}✗${NC} Style build failed"
    fi
}

# Function to handle Rust changes
handle_rust_change() {
    local file="$1"
    local module=$(echo "$file" | sed 's|.*rust-modules/\([^/]*\)/.*|\1|')

    echo -e "${YELLOW}→${NC} Rust changed: ${file}"

    if [ "$module" != "common" ]; then
        echo -e "${YELLOW}→${NC} Rebuilding module: ${module}..."
        cd "$PROJECT_DIR/rust-modules"

        if cargo build --release -p "$module" 2>&1 | grep -E "(Compiling|Finished|error)" | tail -5; then
            if [ ${PIPESTATUS[0]} -eq 0 ]; then
                echo -e "${GREEN}✓${NC} Module ${module} rebuilt"
                reload_waybar
            else
                echo -e "${RED}✗${NC} Module build failed"
            fi
        fi
    else
        echo -e "${YELLOW}→${NC} Common library changed, rebuilding all modules..."
        cd "$PROJECT_DIR/rust-modules"

        if cargo build --release 2>&1 | tail -10; then
            echo -e "${GREEN}✓${NC} All modules rebuilt"
            reload_waybar
        else
            echo -e "${RED}✗${NC} Build failed"
        fi
    fi

    cd "$PROJECT_DIR"
}

# Function to reload Waybar
reload_waybar() {
    echo -e "${YELLOW}→${NC} Reloading Waybar..."

    # Deploy configs first
    if [ -f "target/config" ]; then
        cp target/config ~/.config/waybar/config
    fi
    if [ -f "target/style.css" ]; then
        cp target/style.css ~/.config/waybar/style.css
    fi

    # Reload or restart Waybar
    if pgrep -x waybar > /dev/null; then
        if pkill -SIGUSR2 waybar 2>/dev/null; then
            echo -e "${GREEN}✓${NC} Waybar reloaded (graceful)"
        else
            killall waybar 2>/dev/null
            sleep 0.3
            waybar &>/dev/null &
            disown
            echo -e "${GREEN}✓${NC} Waybar restarted (hard)"
        fi
    else
        waybar &>/dev/null &
        disown
        echo -e "${GREEN}✓${NC} Waybar started"
    fi

    echo ""
}

# Initial build
echo -e "${YELLOW}→${NC} Running initial build..."
cd "$PROJECT_DIR"
if ./scripts/merge-configs.sh target/config 2>&1 | grep -v "Warning"; then
    echo -e "${GREEN}✓${NC} Initial config built"
else
    echo -e "${YELLOW}⚠${NC} Config build had warnings (continuing anyway)"
fi
echo ""

# Watch for changes
inotifywait -m -r -e modify,create,delete \
    --exclude '(target|\.git|\.swp|~)' \
    --format '%w%f' \
    "$PROJECT_DIR/config-src" \
    "$PROJECT_DIR/styles" \
    "$PROJECT_DIR/rust-modules" 2>/dev/null | while read file; do

    # Debounce rapid changes (wait 500ms)
    sleep 0.5

    # Determine what changed and handle accordingly
    case "$file" in
        */config-src/*.json)
            handle_config_change "$file"
            ;;
        */styles/*.css|*/styles/*.scss)
            handle_style_change "$file"
            ;;
        */rust-modules/*/src/*.rs)
            handle_rust_change "$file"
            ;;
    esac
done
