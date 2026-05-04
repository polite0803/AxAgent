#!/usr/bin/env node
/** 补全所有语言文件缺失的 key，以 zh-CN 作为 fallback 值 */
import { readFileSync, writeFileSync } from "node:fs";

function getAllKeys(obj, prefix = "") {
  const keys = [];
  for (const k of Object.keys(obj || {})) {
    const fullKey = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] !== null && !Array.isArray(obj[k])) {
      keys.push(...getAllKeys(obj[k], fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

function getValueByPath(obj, path) {
  const parts = path.split(".");
  let cur = obj;
  for (const p of parts) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = cur[p];
  }
  return cur;
}

function setValueByPath(obj, path, value) {
  const parts = path.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]] || typeof cur[parts[i]] !== "object") {
      cur[parts[i]] = {};
    }
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}

const zh = JSON.parse(readFileSync("src/i18n/locales/zh-CN.json", "utf-8"));
const zhKeys = new Set(getAllKeys(zh));
console.log(`zh-CN (base): ${zhKeys.size} keys`);

const langs = ["en-US", "zh-TW", "ja", "ko", "fr", "de", "es", "ru", "hi", "ar"];

for (const lang of langs) {
  const path = `src/i18n/locales/${lang}.json`;
  const data = JSON.parse(readFileSync(path, "utf-8"));
  const langKeys = new Set(getAllKeys(data));

  const missing = [...zhKeys].filter((k) => !langKeys.has(k));
  const extra = [...langKeys].filter((k) => !zhKeys.has(k));

  // 补全缺失 key，值为 zh-CN 的值作为 fallback
  for (const key of missing) {
    const value = getValueByPath(zh, key);
    if (value !== undefined) {
      setValueByPath(data, key, value);
    }
  }

  writeFileSync(path, JSON.stringify(data, null, 2) + "\n", "utf-8");
  console.log(`${lang}: ${langKeys.size} keys → added ${missing.length} missing (${extra.length} extra kept)`);
}

console.log("\nAll language files now aligned with zh-CN key set.");
