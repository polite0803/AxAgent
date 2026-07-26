#!/usr/bin/env node
// 后端错误码 ↔ 前端 i18n 双向对齐校验（Phase 4.2）
//
// 断言：
//   1. 后端错误码（error_code.rs + harness error_codes.rs 定义的值）全部在前端
//      error 段有翻译（以 zh-CN 为源语言）；
//   2. 前端 error 段中的大写码（UPPER_CASE）全部能在后端找到定义（无孤儿码）；
//   3. 11 语言 error 段的大写码集合完全一致。
//
// 失败即说明某侧新增/删除了错误码而未同步另一侧，阻断 CI。
// camelCase 的 UI 业务错误（如 deleteFailed）不属于后端码，不纳入校验。
//
// 见 AGENTS.md「后端错误码 i18n 规范（强制）」。

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

// CONVERSATION_NOT_FOUND 形态：大写字母开头、含下划线分隔的大写段
const CODE_RE = /^[A-Z][A-Z0-9]+(?:_[A-Z0-9]+)+$/;

// 1. 收集后端码（取定义值，而非 const 名）
const backendFiles = [
  join(root, "src-tauri", "src", "commands", "error_code.rs"),
  join(root, "src-tauri", "crates", "harness", "src", "error_codes.rs"),
];
const backend = new Set();
for (const f of backendFiles) {
  if (!existsSync(f)) {
    console.error(`::warning::跳过缺失的后端码文件 ${f}`);
    continue;
  }
  const src = readFileSync(f, "utf8");
  for (const m of src.matchAll(/=\s*"([A-Z][A-Z0-9]+(?:_[A-Z0-9]+)+)"/g)) {
    backend.add(m[1]);
  }
}

// 2. 收集前端 error 段各语言大写码
const localeDir = join(root, "src", "i18n", "locales");
const langs = readdirSync(localeDir).filter((f) => f.endsWith(".json")).sort();
const frontByLang = {};
for (const lf of langs) {
  const data = JSON.parse(readFileSync(join(localeDir, lf), "utf8"));
  const errSeg = data.error ?? {};
  const codes = new Set();
  for (const k of Object.keys(errSeg)) {
    if (CODE_RE.test(k)) { codes.add(k); }
  }
  frontByLang[lf] = codes;
}

let failed = false;

// 3a. 后端 ⊆ 前端（以 zh-CN 为源）
const source = frontByLang["zh-CN.json"] ?? null;
if (!source) {
  console.error("::error::缺少 zh-CN 源语言 error 段");
  failed = true;
} else {
  const missing = [...backend].filter((c) => !source.has(c));
  if (missing.length) {
    failed = true;
    console.error(`::error::${missing.length} 个后端错误码在 zh-CN error 段缺少翻译：`);
    console.error("  " + missing.join(", "));
  }
}

// 3b. 前端大写码 ⊆ 后端（无孤儿）
for (const lf of langs) {
  const orphans = [...frontByLang[lf]].filter((c) => !backend.has(c));
  if (orphans.length) {
    failed = true;
    console.error(`::error::${lf} 存在 ${orphans.length} 个孤儿错误码（前端有、后端无定义）：`);
    console.error("  " + orphans.join(", "));
  }
}

// 3c. 各语言码集一致
const ref = source ? [...source].sort().join(",") : null;
for (const lf of langs) {
  const cur = [...frontByLang[lf]].sort().join(",");
  if (cur !== ref) {
    failed = true;
    const refSet = new Set(source ?? []);
    const missing = [...refSet].filter((c) => !frontByLang[lf].has(c));
    const extra = [...frontByLang[lf]].filter((c) => !refSet.has(c));
    console.error(
      `::error::${lf} 与 zh-CN 错误码不一致（缺：${missing.join(",") || "无"}；多：${extra.join(",") || "无"}）`,
    );
  }
}

if (failed) {
  console.error("\n错误码对齐校验失败。见 AGENTS.md 后端错误码 i18n 规范：新增后端码须同步补充 11 语言 error 段翻译。");
  process.exit(1);
}
console.log(`OK: 后端 ${backend.size} 个错误码 ↔ 前端 ${langs.length} 语言 error 段对齐一致，无孤儿码。`);
