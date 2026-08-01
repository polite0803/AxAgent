// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 契约一致性核对 CI 脚本
 *
 * 覆盖项目 AGENTS.md「禁区」「后端错误码 i18n 规范」中可静态自动化的契约:
 *   A. Tauri 命令两步注册:  #[tauri::command] 定义必须出现在 register_commands.rs 的 generate_handler![]
 *   B. 错误码 ↔ i18n 翻译:  error_code(s).rs 中的错误码值 ⊆ 11 语言 locale 的 error 段 key
 *   C. i18n key 完整性:      以 zh-CN 为源, 其余 10 语言缺失/多余的 key
 *   D. Harness 依赖方向:     consumer crate 不得直接依赖 harness 之外的 axagent-* crate
 *   E. (warning, 默认关闭)   前后端 DTO 粗对齐: 命令返回类型名应在 src/types 有同名导出
 *
 * 用法:
 *   node scripts/check-contracts.mjs            # 跑 A-D (error 级会 fail)
 *   node scripts/check-contracts.mjs --only=a,b # 只跑指定项
 *   node scripts/check-contracts.mjs --dto      # 额外开启 E (仅 warning)
 *
 * 退出码: 任何 error 级问题 -> 1; 仅 warning -> 0
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SRC_TAURI = join(ROOT, "src-tauri");
const FRONTEND = join(ROOT, "src");
const LOCALES_DIR = join(FRONTEND, "i18n", "locales");
const ZH = join(LOCALES_DIR, "zh-CN.json");

/** consumer crate 列表: 仅允许依赖 axagent-harness */
const CONSUMER_CRATES = ["agent", "gateway", "orchestrator", "runtime-core"];

const errors = [];
const warnings = [];
let hasError = false;
function fail(...m) {
  errors.push(m.join(" "));
  hasError = true;
}
function warn(...m) {
  warnings.push(m.join(" "));
}

// ---------- 工具 ----------
function walk(dir, ext, out = []) {
  if (!existsSync(dir)) { return out; }
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    const s = statSync(p);
    if (s.isDirectory()) { walk(p, ext, out); }
    else if (p.endsWith(ext)) { out.push(p); }
  }
  return out;
}
function read(p) {
  return readFileSync(p, "utf8");
}
/** 限制单条打印数量, 避免海量缺失 key 刷屏 */
function printList(title, items, limit = 50) {
  const sorted = [...items].sort();
  console.log(`\n${title} (${sorted.length}):`);
  sorted.slice(0, limit).forEach((x) => console.log("  " + x));
  if (sorted.length > limit) { console.log(`  ... 另有 ${sorted.length - limit} 条未显示`); }
}

// ---------- A. 命令两步注册 ----------
function extractHandlerBlock(src) {
  const start = src.indexOf("generate_handler![");
  if (start < 0) { return ""; }
  let depth = 0;
  let i = start;
  for (; i < src.length; i++) {
    if (src[i] === "[") { depth++; }
    else if (src[i] === "]") {
      depth--;
      if (depth === 0) { break; }
    }
  }
  return src.slice(start, i + 1);
}
function checkCommandRegistration() {
  const regFile = join(SRC_TAURI, "src", "register_commands.rs");
  const regSrc = read(regFile);
  const block = extractHandlerBlock(regSrc);
  const registered = new Set();
  for (const m of block.matchAll(/^\s*(?:commands::)?(?:[\w]+::)+\w+\s*,?\s*$/gm)) {
    const parts = m[0].split("::");
    registered.add(parts[parts.length - 1].replace(/[,\s]/g, ""));
  }

  const cmdFiles = walk(join(SRC_TAURI, "src"), ".rs");
  const reDef = /#\[(?:tauri::)?command\][\s\S]*?(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)/g;
  const defined = new Set();
  for (const f of cmdFiles) {
    const srcClean = read(f).replace(/\/\/.*$/gm, "");
    for (const m of srcClean.matchAll(reDef)) {
      defined.add(m[1]);
    }
  }

  const unregistered = [...defined].filter((n) => !registered.has(n));
  const orphanReg = [...registered].filter((n) => !defined.has(n));
  if (unregistered.length) { printList("[A] 已定义但未注册到 generate_handler! (前端 invoke 会 404)", unregistered); }
  unregistered.forEach((n) => fail(`[A] 命令已定义但未注册: ${n}`));
  if (orphanReg.length) { orphanReg.forEach((n) =>
      warn(`[A] generate_handler! 注册但无 #[tauri::command] 定义: ${n}`)
    ); }
}

// ---------- B. 错误码 ↔ i18n 翻译 ----------
function extractErrorCodes() {
  const files = [
    join(SRC_TAURI, "crates", "harness", "src", "error_codes.rs"),
    join(SRC_TAURI, "src", "commands", "error_code.rs"),
  ];
  const codes = new Set();
  for (const f of files) {
    if (!existsSync(f)) { continue; }
    const src = read(f);
    for (const m of src.matchAll(/(?:pub\s+)?const\s+\w+\s*:\s*&str\s*=\s*"([A-Z][A-Z0-9_]*)"/g)) {
      codes.add(m[1]);
    }
  }
  return codes;
}
function localeErrorKeys(p) {
  const obj = JSON.parse(read(p));
  return obj.error && typeof obj.error === "object" ? Object.keys(obj.error) : [];
}
function checkErrorCodeI18n() {
  const codes = extractErrorCodes();
  if (!codes.size) {
    warn("[B] 未提取到任何错误码常量, 请检查 error_code(s).rs 路径");
    return;
  }
  const locales = readdirSync(LOCALES_DIR).filter((f) => f.endsWith(".json"));
  const missingByLocale = {};
  for (const loc of locales) {
    const keys = new Set(localeErrorKeys(join(LOCALES_DIR, loc)));
    const missing = [...codes].filter((c) => !keys.has(c));
    if (missing.length) { missingByLocale[loc] = missing; }
  }
  for (const [loc, missing] of Object.entries(missingByLocale)) {
    printList(`[B] 错误码在 ${loc} 的 error 段缺失翻译`, missing);
    missing.forEach((c) => fail(`[B] 错误码 ${c} 在 ${loc} 缺失翻译`));
  }
  // orphan: locale error 段里像错误码格式但不在常量表中的 key
  for (const loc of locales) {
    const keys = localeErrorKeys(join(LOCALES_DIR, loc));
    keys
      .filter((k) => /^[A-Z][A-Z0-9_]{3,}$/.test(k) && !codes.has(k))
      .forEach((k) => warn(`[B] ${loc} error 段含未定义错误码格式的 key: ${k}`));
  }
}

// ---------- C. i18n key 完整性 ----------
function flatten(obj, prefix = "", out = new Set()) {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) { flatten(v, key, out); }
    else { out.add(key); }
  }
  return out;
}
function checkI18nKeys() {
  const zh = flatten(JSON.parse(read(ZH)));
  const locales = readdirSync(LOCALES_DIR).filter((f) => f !== "zh-CN.json" && f.endsWith(".json"));
  for (const loc of locales) {
    const keys = flatten(JSON.parse(read(join(LOCALES_DIR, loc))));
    const missing = [...zh].filter((k) => !keys.has(k));
    const extra = [...keys].filter((k) => !zh.has(k));
    if (missing.length) { printList(`[C] ${loc} 相对 zh-CN 缺失的 i18n key`, missing); }
    missing.forEach((k) => fail(`[C] ${loc} 缺失 i18n key: ${k}`));
    extra.forEach((k) => warn(`[C] ${loc} 含 zh-CN 没有的多余 key: ${k}`));
  }
}

// ---------- D. Harness 依赖方向 ----------
function checkHarnessDirection() {
  for (const name of CONSUMER_CRATES) {
    const toml = join(SRC_TAURI, "crates", name, "Cargo.toml");
    if (!existsSync(toml)) {
      warn(`[D] consumer crate 不存在: ${name}`);
      continue;
    }
    const src = read(toml);
    const axagent = new Set();
    for (const b of src.matchAll(/\[dependencies(?:\.[\w-]+)?\][\s\S]*?(?=\n\[|\Z)/g)) {
      for (const m of b[0].matchAll(/^\s*([\w-]+)\s*=/gm)) {
        if (m[1].startsWith("axagent-")) { axagent.add(m[1]); }
      }
    }
    const violations = [...axagent].filter((d) => d !== "axagent-harness");
    if (violations.length) {
      printList(`[D] consumer crate "${name}" 越界依赖实现层 (仅允许 axagent-harness)`, violations);
      violations.forEach((d) => fail(`[D] ${name} 越界依赖: ${d}`));
    }
  }
}

// ---------- E. 前后端 DTO 粗对齐 (warning only) ----------
function checkDto() {
  const cmdFiles = walk(join(SRC_TAURI, "src"), ".rs");
  const tsTypes = new Set();
  for (const f of walk(join(FRONTEND, "types"), ".ts")) {
    const src = read(f);
    for (const m of src.matchAll(/export\s+(?:interface|type)\s+(\w+)/g)) { tsTypes.add(m[1]); }
  }
  const primitives = new Set([
    "String",
    "bool",
    "u8",
    "u16",
    "u32",
    "u64",
    "i8",
    "i16",
    "i32",
    "i64",
    "f32",
    "f64",
    "usize",
    "isize",
  ]);
  const re =
    /#\[tauri::command\][\s\S]*?pub\s+(?:async\s+)?fn\s+(\w+)\s*\([^)]*\)\s*(?:->\s*([\w<>:,\s]+?))?\s*(?:\{|;)/g;
  for (const f of cmdFiles) {
    const src = read(f);
    let m;
    while ((m = re.exec(src))) {
      const ret = m[2];
      if (!ret) { continue; }
      const inner = ret
        .replace(/(?:Result|Option|Vec|HashMap|BTreeMap|std::|crate::|axagent_\w+::)[\s<>:,]*|[\s<>:,]+/g, " ")
        .trim()
        .split(/\s+/)[0];
      if (/^[A-Z]\w+$/.test(inner) && !primitives.has(inner) && !tsTypes.has(inner)) {
        warn(`[E] 命令 ${m[1]} 返回类型 ${inner} 在 src/types 无同名导出 (可能需对齐 DTO)`);
      }
    }
  }
}

// ---------- 主流程 ----------
const argv = process.argv.slice(2);
let only = null;
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === "--only") { only = argv[i + 1].split(","); }
  else if (argv[i].startsWith("--only=")) { only = argv[i].slice("--only=".length).split(","); }
}
const enableDto = argv.includes("--dto");
const has = (x) => !only || only.includes(x);

console.log("=== 契约一致性核对 (contract-consistency) ===");
if (has("a")) { checkCommandRegistration(); }
if (has("b")) { checkErrorCodeI18n(); }
if (has("c")) { checkI18nKeys(); }
if (has("d")) { checkHarnessDirection(); }
if (enableDto) { checkDto(); }

if (warnings.length) {
  console.log(`\n[WARNINGS] (${warnings.length}):`);
  warnings.slice(0, 50).forEach((w) => console.log("  " + w));
  if (warnings.length > 50) { console.log(`  ... 另有 ${warnings.length - 50} 条`); }
}
console.log(`\n[汇总] errors=${errors.length} warnings=${warnings.length}`);
console.log(`结果: ${hasError ? "FAIL" : "PASS"}`);
process.exit(hasError ? 1 : 0);
