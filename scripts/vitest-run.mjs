// Windows cwd 盘符大小写防御：规范 cwd 后再启动 vitest CLI。
// 根因（vitest#10812 / angular-cli#33559）：process.cwd() 为小写盘符（d:\）时，
// vitest module-runner 生成的模块 ID 与 runner chunk 的 canonical 路径（D:\）不一致，
// @vitest/runner 被双实例化 → 测试文件内的 describe 拿到的 runner 未初始化 →
// 首个 describe() 即崩：TypeError: Cannot read properties of undefined (reading 'config')。
// 修复：用 realpathSync.native 把 cwd 规范化为磁盘 canonical 大小写（D:\）再启动 vitest。
import { spawnSync } from "node:child_process";
import { realpathSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(here, "..");
const canonical = realpathSync.native(projectRoot);
if (process.cwd() !== canonical) {
  process.chdir(canonical);
}

const vitestEntry = path.join(canonical, "node_modules", "vitest", "vitest.mjs");
const result = spawnSync(process.execPath, [vitestEntry, ...process.argv.slice(2)], {
  stdio: "inherit",
  cwd: canonical,
});
process.exit(result.status ?? 1);
