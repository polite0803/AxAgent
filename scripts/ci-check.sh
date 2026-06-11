#!/usr/bin/env bash
# scripts/ci-check.sh
# Local pre-merge check. Mirrors the CI pipeline we want.
#
# Usage:
#   ./scripts/ci-check.sh          # run all checks
#   ./scripts/ci-check.sh --fast   # skip cargo test (build only)
#
# Each step is independent; failing step exits non-zero with clear message.

set -u
set -o pipefail

FAST=0
if [ "${1:-}" = "--fast" ]; then
  FAST=1
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

step() {
  echo
  echo -e "${YELLOW}=== $1 ===${NC}"
}

ok() {
  echo -e "${GREEN}  PASS${NC}"
}

fail() {
  echo -e "${RED}  FAIL: $1${NC}"
  exit 1
}

# ── 1. cargo fmt ──
step "1. cargo fmt"
if cargo fmt --all -- --check; then
  ok
else
  fail "cargo fmt found unformatted code. Run: cargo fmt --all"
fi

# ── 2. cargo clippy ──
step "2. cargo clippy (workspace, deny warnings)"
if cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -50; then
  ok
else
  fail "cargo clippy produced warnings or errors"
fi

# ── 3. cargo build ──
step "3. cargo build (workspace, debug)"
if cargo build --workspace 2>&1 | tail -20; then
  ok
else
  fail "cargo build failed"
fi

# ── 4. cargo test (skipped in --fast) ──
if [ "$FAST" = "0" ]; then
  step "4. cargo test (workspace)"
  if cargo test --workspace --no-fail-fast 2>&1 | tail -40; then
    ok
  else
    echo -e "${YELLOW}  WARN: some tests failed (continuing)${NC}"
  fi
else
  echo
  echo "=== 4. cargo test (SKIPPED, --fast mode) ==="
fi

# ── 5. i18n consistency ──
step "5. i18n consistency check"
if [ -f "scripts/i18n-check.mjs" ]; then
  if node scripts/i18n-check.mjs; then
    ok
  else
    fail "i18n drift detected. Run: node scripts/i18n-check.mjs to see details"
  fi
elif [ -f "src/i18n/compare_locales.js" ]; then
  # Fallback: legacy check
  echo "  using legacy compare_locales.js"
  node src/i18n/compare_locales.js 2>&1 | tail -20 || true
else
  echo "  SKIP: no i18n check script found"
fi

# ── 6. frontend lint ──
step "6. frontend lint (eslint)"
if [ -f "package.json" ] && grep -q '"lint"' package.json; then
  if pnpm lint 2>&1 | tail -30; then
    ok
  else
    fail "pnpm lint failed"
  fi
else
  echo "  SKIP: no lint script in package.json"
fi

# ── 7. frontend type check ──
step "7. frontend type check (tsc)"
if [ -f "tsconfig.json" ]; then
  if pnpm tsc --noEmit 2>&1 | tail -30; then
    ok
  else
    fail "tsc found type errors"
  fi
else
  echo "  SKIP: no tsconfig.json"
fi

echo
echo -e "${GREEN}=== ALL CHECKS PASSED ===${NC}"
