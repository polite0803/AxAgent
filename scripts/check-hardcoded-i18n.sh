#!/usr/bin/env bash
# scripts/check-hardcoded-i18n.sh
# i18n hardcoded string detection for CI
# Modes: --report (default) | --strict | --diff-only
set -euo pipefail
export LC_ALL=C.UTF-8
cd "$(dirname "$0")/.."

MODE="report"
ALLOWLIST="scripts/.i18n-allowlist.json"
TEMP_DIR=".check-i18n-tmp-$$"
mkdir -p "$TEMP_DIR"
trap "rm -rf $TEMP_DIR" EXIT
EXIT_CODE=0

for arg in "$@"; do
  case $arg in
    --strict) MODE="strict" ;;
    --report) MODE="report" ;;
    --diff-only) MODE="diff-only" ;;
    *) echo "Unknown option: $arg"; exit 2 ;;
  esac
done

echo "=== i18n Hardcoded Strings Check (mode: $MODE) ==="

# Determine files to scan
if [ "$MODE" = "diff-only" ]; then
  # Detect base reference: try origin/master, then local master, then HEAD~1
  BASE_REF="origin/master"
  git fetch origin master --quiet 2>/dev/null || true
  if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
    if git rev-parse --verify "master" >/dev/null 2>&1; then
      BASE_REF="master"
    else
      BASE_REF="HEAD~1"
    fi
  fi
  CHANGED_FILES=$(git diff --name-only "$BASE_REF" HEAD 2>/dev/null | grep -E '\.(ts|tsx)$' | grep '^src/' | grep -v 'src/i18n/locales/' || true)
  if [ -z "$CHANGED_FILES" ]; then
    echo "No changed TypeScript files to check."
    exit 0
  fi
  echo "Checking $(echo "$CHANGED_FILES" | wc -l) changed file(s)"
else
  CHANGED_FILES=$(find src -name '*.ts' -o -name '*.tsx' | grep -v 'src/i18n/locales/' | sort)
fi

# Build ignore patterns from allowlist
node -e "
const fs = require('fs');
try {
  const al = JSON.parse(fs.readFileSync('$ALLOWLIST', 'utf8'));
  const lines = [];
  for (const e of al.entries || []) {
    for (const ln of (e.lines || '').split(',')) {
      if (ln) lines.push(e.file + ':' + ln);
    }
  }
  fs.writeFileSync('$TEMP_DIR/ignored.txt', lines.join('\n'));
} catch(e) { fs.writeFileSync('$TEMP_DIR/ignored.txt', ''); }
"

is_allowed() {
  grep -qxF "${1}:${2}" "$TEMP_DIR/ignored.txt" 2>/dev/null
}

VIOLATIONS=0

# Rule 1: Chinese CJK characters
echo ""
echo "--- Rule 1: Hardcoded Chinese (CJK) strings ---"
> "$TEMP_DIR/r1.txt"
for f in $CHANGED_FILES; do
  [ -f "$f" ] || continue
  grep -nP '[\x{4e00}-\x{9fff}\x{3400}-\x{4dbf}]' "$f" 2>/dev/null | while IFS=: read -r lnum content; do
    # Skip comments
    [[ "$content" =~ ^[[:space:]]*// ]] && continue
    [[ "$content" =~ ^[[:space:]]*\* ]] && continue
    # Skip console.*
    [[ "$content" =~ console\.(log|warn|error|debug|info|trace) ]] && continue
    # Check allowlist
    if ! is_allowed "$f" "$lnum"; then
      echo "  $f:$lnum: $content" >> "$TEMP_DIR/r1.txt"
    fi
  done || true
done

if [ -s "$TEMP_DIR/r1.txt" ]; then
  count=$(wc -l < "$TEMP_DIR/r1.txt")
  echo "  FAIL: $count new violation(s):"
  cat "$TEMP_DIR/r1.txt"
  VIOLATIONS=$((VIOLATIONS + count))
  EXIT_CODE=1
else
  echo "  PASS: No violations"
fi

# Rule 2: English UI hardcoded strings
echo ""
echo "--- Rule 2: Hardcoded English UI strings ---"
> "$TEMP_DIR/r2.txt"
for f in $CHANGED_FILES; do
  [ -f "$f" ] || continue
  # message.success/error/warning/info("...")
  grep -nP "(message\.(success|error|warning|info)\(\s*['\"])" "$f" 2>/dev/null | while IFS=: read -r lnum content; do
    if ! is_allowed "$f" "$lnum"; then
      echo "  $f:$lnum: $content" >> "$TEMP_DIR/r2.txt"
    fi
  done || true
  # placeholder="..."
  grep -nP 'placeholder\s*=\s*"[A-Za-z][^"]{2,}"' "$f" 2>/dev/null | while IFS=: read -r lnum content; do
    if ! is_allowed "$f" "$lnum"; then
      echo "  $f:$lnum: $content" >> "$TEMP_DIR/r2.txt"
    fi
  done || true
done

if [ -s "$TEMP_DIR/r2.txt" ]; then
  count=$(wc -l < "$TEMP_DIR/r2.txt")
  echo "  FAIL: $count new violation(s):"
  cat "$TEMP_DIR/r2.txt"
  VIOLATIONS=$((VIOLATIONS + count))
  EXIT_CODE=1
else
  echo "  PASS: No violations"
fi

# Rule 3: t() fallback patterns (WARNING only)
echo ""
echo "--- Rule 3: t() fallback patterns (WARNING) ---"
> "$TEMP_DIR/r3.txt"
for f in $CHANGED_FILES; do
  [ -f "$f" ] || continue
  grep -nP "t\(\s*['\"][^'\"]+['\"]\s*,\s*['\"][^'\"]+['\"]" "$f" 2>/dev/null | while IFS=: read -r lnum content; do
    if ! is_allowed "$f" "$lnum"; then
      echo "  WARNING: $f:$lnum: $content" >> "$TEMP_DIR/r3.txt"
    fi
  done || true
done

if [ -s "$TEMP_DIR/r3.txt" ]; then
  count=$(wc -l < "$TEMP_DIR/r3.txt")
  echo "  WARNING: $count t() fallback(s) found (not blocking):"
  cat "$TEMP_DIR/r3.txt"
else
  echo "  No t() fallbacks found"
fi

echo ""
echo "=== Summary ==="
if [ $EXIT_CODE -eq 0 ]; then
  echo "All i18n checks passed."
else
  echo "Found $VIOLATIONS i18n violation(s)."
  echo "Fix them or update scripts/.i18n-allowlist.json."
fi

exit $EXIT_CODE
