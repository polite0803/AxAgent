#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-only
//
// 干净构建/切分支/CI 专用：临时启用 sccache 跑 cargo。
//
// 背景：全局启用 sccache（RUSTC_WRAPPER）会强制关闭所有 crate 的增量编译，
//       而 src-tauri/.cargo/config.toml 特意开了 `incremental=true` 优化巨型
//       app crate 的日常迭代。因此日常 dev 不要用它，只有需要全量重建时，
//       用本脚本临时挂上 sccache，命中 3200 个第三方依赖的缓存，省去全量重编。
//
// 用法：
//   npm run build:clean                          # cargo build（默认）
//   npm run build:clean -- check                 # cargo check
//   npm run build:clean -- build --release       # 透传 cargo 参数
//   npm run build:clean -- test -p axagent-harness
//
// 说明：本脚本只在这个进程内设置 RUSTC_WRAPPER，不写入任何全局/项目配置，
//       结束即失效，日常 dev 的增量编译不受影响。

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, "..");
const srcTauri = join(projectRoot, "src-tauri");

// 探测 sccache 可执行文件：优先复用已有 RUSTC_WRAPPER，其次 CARGO_HOME/USERPROFILE 下的 bin，最后依赖 PATH。
const sccache = (() => {
  const candidates = [
    process.env.RUSTC_WRAPPER && process.env.RUSTC_WRAPPER !== "sccache"
      ? process.env.RUSTC_WRAPPER
      : null,
    join(
      process.env.CARGO_HOME || join(process.env.USERPROFILE, ".cargo"),
      "bin",
      "sccache.exe",
    ),
    join(process.env.USERPROFILE, ".cargo", "bin", "sccache.exe"),
    "sccache",
  ].filter(Boolean);
  return candidates.find((c) => c === "sccache" || existsSync(c));
})();

if (!sccache) {
  console.error("✗ 未找到 sccache，请先安装：cargo install sccache");
  process.exit(1);
}

// 兜底：确保容量上限为 50G（与 AGENTS.md 的 sccache 缓存意图一致）
process.env.SCCACHE_CACHE_SIZE = process.env.SCCACHE_CACHE_SIZE || "50G";

const userArgs = process.argv.slice(2);
const command = userArgs[0] ?? "build";
const rest = userArgs.slice(1);

console.log(
  `▶ sccache 干净构建（RUSTC_WRAPPER=${sccache}，SCCACHE_CACHE_SIZE=${process.env.SCCACHE_CACHE_SIZE}）`,
);
console.log(`▶ cargo ${userArgs.join(" ") || "build"}`);

process.env.RUSTC_WRAPPER = sccache;
const child = spawn("cargo", [command, ...rest], { cwd: srcTauri, stdio: "inherit" });
child.on("exit", (code) => process.exit(code ?? 0));