#!/usr/bin/env node

/**
 * AxAgent 本地 CI 模拟脚本
 * 按"最便宜最先失败"顺序执行所有 CI 检查步骤
 *
 * 用法:
 *   node scripts/ci-check.mjs           # 完整检查
 *   node scripts/ci-check.mjs --quick   # 快速检查 (dprint + rustfmt + tsc)
 *   node scripts/ci-check.mjs --frontend-only
 *   node scripts/ci-check.mjs --rust-only
 *   node scripts/ci-check.mjs --skip-rust  # 跳过 Rust 检查（无 Rust 环境时）
 */

import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const srcTauri = resolve(root, "src-tauri");

// 参数解析
const args = process.argv.slice(2);
const quick = args.includes("--quick");
const frontendOnly = args.includes("--frontend-only");
const rustOnly = args.includes("--rust-only");
const skipRust = args.includes("--skip-rust");

const hasRust = existsSync(resolve(srcTauri, "Cargo.toml"));
const canRunRust = hasRust && !skipRust && !frontendOnly;
const canRunFrontend = !rustOnly;

// 颜色输出
const c = {
  reset: "\x1b[0m",
  bold: "\x1b[1m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
};
const ok = `${c.green}✓${c.reset}`;
const fail = `${c.red}✗${c.reset}`;

let failures = 0;

function step(label, cmd, opts = {}) {
  const displayLabel = label.padEnd(52);
  process.stdout.write(`  ${displayLabel}`);
  try {
    execSync(cmd, { stdio: "pipe", cwd: opts.cwd || root, ...opts });
    console.log(`${ok}`);
    return true;
  } catch (e) {
    console.log(`${fail}`);
    const stderr = e.stderr?.toString().trim() || "";
    const stdout = e.stdout?.toString().trim() || "";
    const output = [stderr, stdout].filter(Boolean).join("\n");
    // 只打印最后 20 行，避免刷屏
    const lines = output.split("\n");
    const tail = lines.slice(-20).join("\n");
    console.log(`\n${c.red}${tail}${c.reset}\n`);
    failures++;
    if (!opts.continueOnError) {
      console.log(`${c.bold}${c.red}→ 检查失败，中断执行。请修复上述错误后重新运行。${c.reset}`);
      process.exit(1);
    }
    return false;
  }
}

// ── 入口 ────────────────────────────────────────────────
console.log(`\n${c.bold}AxAgent CI 本地检查${c.reset}`);
console.log(
  `${c.cyan}模式: ${
    quick ? "快速 (dprint + rustfmt + tsc)" : frontendOnly ? "仅前端" : rustOnly ? "仅 Rust" : "完整"
  }${c.reset}`,
);
console.log(`${c.cyan}平台: ${process.platform} | Node: ${process.version}${c.reset}`);
if (!hasRust) { console.log(`${c.yellow}Rust 环境未检测到，自动跳过 Rust 检查${c.reset}`); }
console.log();

const startedAt = Date.now();

// ── 前端检查 ────────────────────────────────────────────
if (canRunFrontend) {
  console.log(`${c.bold}[前端检查]${c.reset}`);

  step("dprint 格式化检查", "npx dprint check");

  if (!quick) {
    step("ESLint 检查", "npx eslint src --max-warnings=0");
  }

  step("TypeScript 类型检查", "npx tsc --noEmit");

  if (!quick && !frontendOnly) {
    step("Vitest 单元测试", "npx vitest run");
  }
}

// ── Rust 检查 ────────────────────────────────────────────
if (canRunRust && !quick) {
  console.log(`\n${c.bold}[Rust 检查]${c.reset}`);

  step(
    "cargo fmt 格式化检查",
    "cargo fmt --check --all",
    { cwd: srcTauri, env: { ...process.env } },
  );

  step(
    "cargo clippy (deny warnings)",
    "cargo clippy --all-targets --all-features -- -D warnings",
    { cwd: srcTauri, timeout: 10 * 60 * 1000, env: { ...process.env } },
  );

  step(
    "cargo test 单元测试 (库 crate)",
    "cargo test --workspace --exclude axagent",
    { cwd: srcTauri, timeout: 10 * 60 * 1000, env: { ...process.env } },
  );
}

// 快速模式中的 Rust 格式化
if (canRunRust && quick) {
  console.log(`\n${c.bold}[Rust 快速检查]${c.reset}`);
  step(
    "cargo fmt 格式化检查",
    "cargo fmt --check --all",
    { cwd: srcTauri, env: { ...process.env } },
  );
}

// ── 结果 ──────────────────────────────────────────────────
const elapsed = ((Date.now() - startedAt) / 1000).toFixed(1);
console.log(
  `\n${c.bold}${failures === 0 ? c.green : c.red}${
    failures === 0 ? "全部检查通过!" : `${failures} 项检查失败`
  }${c.reset} (耗时 ${elapsed}s)\n`,
);

process.exit(failures > 0 ? 1 : 0);
