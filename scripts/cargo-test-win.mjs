#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-only
//
// 解决 Windows 上 `cargo test` 报 STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139) 的问题。
//
// 根因：Rust 测试 harness 在 panic/fail 时调用 TaskDialogIndirect，
//       该函数在 comctl32 v6.0 中导出，v5.82 中不存在。
//       Windows 11 默认 System32 里只有 v5.82，v6.0 在 WinSxS 等待 manifest 激活。
//       test binary 没带 manifest → 加载 v5.82 → 找不到入口点崩溃。
//
// 解法：用 mt.exe 把 app.manifest（声明 comctl32 v6.0 依赖）嵌入到每个 test binary。
//
// 默认行为：
//   - cargo test --workspace --no-run  编译全部 workspace test target
//   - 给 target/debug/deps/*.exe 与 target/debug/*.exe 下所有测试二进制嵌入 manifest
//   - cargo test --workspace --no-fail-fast  全盘跑，一次性暴露所有失败
//   - 默认 --format=terse，压缩输出便于全量诊断
//
// 用法：
//   npm run test:rust                                  # 全盘(默认 no-fail-fast, terse)
//   npm run test:rust -- <crate>::<test_filter>        # 指定过滤(仍 no-fail-fast)
//   npm run test:rust -- --no-fail-fast -- --nocapture # 透传 cargo / test 参数

import { execSync, spawn } from "node:child_process";
import { existsSync, readdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, "..");
const srcTauri = join(projectRoot, "src-tauri");
const manifestPath = join(srcTauri, "app.manifest");
const depsDir = join(srcTauri, "target", "debug", "deps");
const debugDir = join(srcTauri, "target", "debug");

// 1. 编译（不跑）所有 workspace test target
console.log("▶ cargo test --workspace --no-run");
try {
  execSync("cargo test --workspace --no-run", { cwd: srcTauri, stdio: "inherit" });
} catch (e) {
  console.error("✗ cargo test --workspace --no-run 编译失败");
  process.exit(e.status ?? 1);
}

const userArgs = process.argv.slice(2);
const embed = process.platform === "win32" && existsSync(manifestPath);

if (embed) {
  // 测试二进制命名规则: <crate>-<16位hex hash>.exe
  const TEST_BIN_RE = /^[a-zA-Z0-9_-]+-[0-9a-f]{16}\.exe$/;
  const collect = (dir) =>
    existsSync(dir)
      ? readdirSync(dir)
        .filter((f) => TEST_BIN_RE.test(f))
        .map((f) => join(dir, f))
      : [];
  const binaries = [...collect(depsDir), ...collect(debugDir)].sort(
    (a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs,
  );
  console.log(`▶ 找到 ${binaries.length} 个测试二进制，逐个嵌入 comctl32 v6.0 manifest`);
  let embedded = 0;
  for (const exe of binaries) {
    try {
      execSync(
        `mt.exe -nologo -manifest "${manifestPath}" -outputresource:"${exe};#1"`,
        { stdio: "ignore" },
      );
      embedded += 1;
    } catch {
      // 部分二进制可能无 RT_MANIFEST 资源或为非 PE文件，忽略
    }
  }
  console.log(`  ✓ 成功嵌入 ${embedded}/${binaries.length}`);
} else if (process.platform === "win32") {
  console.warn(`⚠ 未找到 ${manifestPath}，跳过 manifest 嵌入（可能触发 DLL 崩溃）`);
}

// 2. 实际跑 test（默认 --no-fail-fast，一次性暴露所有失败）
const args = ["test", "--workspace"];
if (!userArgs.includes("--no-fail-fast")) { args.push("--no-fail-fast"); }
args.push(...userArgs);
if (!userArgs.includes("--")) { args.push("--", "--format=terse"); }

console.log(`▶ cargo ${args.join(" ")}`);
const child = spawn("cargo", args, { cwd: srcTauri, stdio: "inherit" });
child.on("exit", (code) => process.exit(code ?? 0));
