#!/usr/bin/env bash
# scripts/check-hardcoded-i18n.sh
# i18n hardcoded string detection for CI
# Modes: --report (default) | --strict | --diff-only | --update-allowlist
#
# 检测核心已迁移到 scripts/i18n-scan.mjs（单进程 Node 实现），
# 避免原 bash 逐文件起子 shell 导致的 fork 资源耗尽，并正确处理所有注释形态。
# 本文件仅作 CLI 入口与向后兼容薄包装。
set -euo pipefail
cd "$(dirname "$0")/.."
exec node scripts/i18n-scan.mjs "$@"
