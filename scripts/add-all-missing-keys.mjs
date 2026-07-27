#!/usr/bin/env node
// scripts/add-all-missing-keys.mjs
// 自动补齐所有 locale 文件中缺失的 i18n key。
// 以 key 数量最多的 locale 为参考，向其他 locale 补齐缺失 key。
// 用法：node scripts/add-all-missing-keys.mjs

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = resolve(fileURLToPath(import.meta.url), "..");
const LOCALES_DIR = resolve(__dirname, "..", "src", "i18n", "locales");

// ---- 工具函数 ----

// 递归获取所有 key（dot notation）
function getAllKeys(obj, prefix = "") {
  let keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${k}` : k;
    keys.push(fullKey);
    if (v && typeof v === "object" && !Array.isArray(v)) {
      keys = keys.concat(getAllKeys(v, fullKey));
    }
  }
  return keys;
}

// 按 dot-path 设置嵌套值（如 setNested(obj, "a.b.c", 42)）
function setNested(obj, dotPath, value) {
  const parts = dotPath.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    const part = parts[i];
    if (!(part in cur) || typeof cur[part] !== "object" || Array.isArray(cur[part])) {
      cur[part] = {};
    }
    cur = cur[part];
  }
  cur[parts[parts.length - 1]] = value;
}

// 深合并：只填充 src 有而 target 没有的 key，不覆盖已有值
function deepMerge(target, src) {
  for (const [k, v] of Object.entries(src)) {
    if (!(k in target)) {
      target[k] = v;
    } else if (v && typeof v === "object" && !Array.isArray(v)) {
      deepMerge(target[k], v);
    }
    // 如果 target 已有 k，且 v 是 object，递归；否则跳过（不覆盖）
  }
}

// ---- 主逻辑 ----

const files = readdirSync(LOCALES_DIR).filter((f) => f.endsWith(".json"));
const localeData = {};
const localeKeys = {};

// 加载所有 locale
for (const f of files) {
  const fp = join(LOCALES_DIR, f);
  const data = JSON.parse(readFileSync(fp, "utf8"));
  localeData[f] = data;
  localeKeys[f] = new Set(getAllKeys(data));
}

// 找最完整的 locale（key 数最多）
let refFile = files[0];
let refKeyCount = localeKeys[refFile].size;
for (const f of files) {
  if (localeKeys[f].size > refKeyCount) {
    refKeyCount = localeKeys[f].size;
    refFile = f;
  }
}
console.log(`参考文件: ${refFile}（${refKeyCount} 个 key）\n`);

const refData = localeData[refFile];
const refKeysList = getAllKeys(refData);

let totalAdded = 0;
const report = {};

for (const f of files) {
  if (f === refFile) continue;
  const missing = refKeysList.filter((k) => !localeKeys[f].has(k));
  if (missing.length === 0) {
    console.log(`✅ ${f} — 无缺失 key`);
    report[f] = { added: 0, missing: [] };
    continue;
  }

  // 补齐缺失 key
  let addedCount = 0;
  for (const dotPath of missing) {
    // 从 refData 中按 dotPath 读取值
    const parts = dotPath.split(".");
    let val = refData;
    for (const p of parts) {
      val = val?.[p];
    }
    if (val === undefined) continue;
    setNested(localeData[f], dotPath, val);
    addedCount++;
  }

  totalAdded += addedCount;
  report[f] = { added: addedCount, missing };
  console.log(`⬆️  ${f} — 补齐 ${addedCount} 个 key`);
}

// 写回文件
for (const f of files) {
  if (f === refFile) continue;
  if (report[f]?.added > 0) {
    const fp = join(LOCALES_DIR, f);
    writeFileSync(fp, JSON.stringify(localeData[f], null, 2) + "\n", "utf8");
  }
}

console.log(`\n✅ 完成！共补齐 ${totalAdded} 个缺失 key（参考 ${refFile}）`);
for (const [f, info] of Object.entries(report)) {
  if (info.added > 0) {
    console.log(`   ${f}: +${info.added}`);
  }
}
