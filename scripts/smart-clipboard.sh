#!/usr/bin/env bash
# Smart Clipboard - Shows clipboard item count and preview
# Lightweight and fast - no caching

set -euo pipefail

# Get clipboard history count (fast)
CLIP_COUNT=$(cliphist list 2>/dev/null | wc -l || echo "0")

if [[ "$CLIP_COUNT" -gt 0 ]]; then
    # Get latest clip (first 30 chars) only if we have items
    LATEST=$(cliphist list 2>/dev/null | head -1 | cut -c1-30 | tr '\n' ' ' || echo "Empty")
    echo "{\"text\": \"📋 $CLIP_COUNT\", \"tooltip\": \"Latest: $LATEST\", \"class\": \"active\"}"
else
    echo "{\"text\": \"📋\", \"tooltip\": \"Clipboard empty\", \"class\": \"empty\"}"
fi
