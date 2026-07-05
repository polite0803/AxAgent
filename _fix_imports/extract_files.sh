#!/bin/bash
# Extract files for a given error pattern
# Usage: extract_files.sh "cannot find type.*ErrorResponse"
ERR_FILE="d:/OneManager/AxAgent/_fix_imports/cargo_errors.txt"
PATTERN="$1"
grep -B1 "$PATTERN" "$ERR_FILE" | grep "^---> " | sed 's/---> //' | sed 's/:[0-9]*:[0-9]*$//' | sed 's/\\/\//g' | sed 's/src\//d:\/OneManager\/AxAgent\/src-tauri\/src\//' | sort -u
