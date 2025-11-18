#!/bin/bash
# Merge modular JSON configs into single Waybar config

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$PROJECT_DIR/config-src"

# Use provided output path or default
OUTPUT_FILE="${1:-$PROJECT_DIR/target/config}"
OUTPUT_DIR="$(dirname "$OUTPUT_FILE")"

mkdir -p "$OUTPUT_DIR"

# Check if jq is available
if ! command -v jq &> /dev/null; then
    echo "Warning: jq not found, using basic merge"
    # Fallback: just copy base config if jq not available
    if [ -f "$CONFIG_DIR/base.json" ]; then
        cp "$CONFIG_DIR/base.json" "$OUTPUT_FILE"
    fi
    exit 0
fi

# Start with base config
if [ ! -f "$CONFIG_DIR/base.json" ]; then
    echo "Error: base.json not found"
    exit 1
fi

BASE=$(cat "$CONFIG_DIR/base.json")

# Merge system modules if they exist
if [ -f "$CONFIG_DIR/modules/system.json" ]; then
    SYSTEM_MODULES=$(cat "$CONFIG_DIR/modules/system.json")
    BASE=$(echo "$BASE" | jq --argjson modules "$SYSTEM_MODULES" '. + $modules')
fi

# Merge extended modules if they exist
if [ -f "$CONFIG_DIR/modules/extended.json" ]; then
    EXTENDED_MODULES=$(cat "$CONFIG_DIR/modules/extended.json")
    BASE=$(echo "$BASE" | jq --argjson modules "$EXTENDED_MODULES" '. + $modules')
fi

# Write final config
echo "$BASE" | jq '.' > "$OUTPUT_FILE"

echo "✓ Config merged to $OUTPUT_FILE"
