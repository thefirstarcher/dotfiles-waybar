#!/bin/bash
# Strip ANSI escape codes from command output
# Usage: strip-ansi.sh <command> [args...]

"$@" 2>&1 | sed 's/\x1b\[[0-9;]*[a-zA-Z]//g' | sed 's/\x1b\[[0-9;]*q//g'
