#!/usr/bin/env node
/**
 * 检测非英文 locale 文件中值等于 en-US 的未翻译条目。
 * 输出 JSON 报告到 stdout。
 */
import { readFileSync, writeFileSync } from "node:fs";

function getAllLeafPaths(obj, prefix = "") {
  const entries = [];
  for (const k of Object.keys(obj || {})) {
    const fullKey = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] !== null && !Array.isArray(obj[k])) {
      entries.push(...getAllLeafPaths(obj[k], fullKey));
    } else {
      entries.push({ key: fullKey, value: obj[k] });
    }
  }
  return entries;
}

function setValueByPath(obj, path, value) {
  const parts = path.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]] || typeof cur[parts[i]] !== "object") { cur[parts[i]] = {}; }
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}

const langs = ["en-US", "zh-CN", "zh-TW", "ja", "ko", "ru", "de", "es", "fr", "hi", "ar"];
const data = {};
for (const l of langs) {
  data[l] = JSON.parse(readFileSync(`src/i18n/locales/${l}.json`, "utf-8"));
}

// en-US reference as flat map
const enEntries = getAllLeafPaths(data["en-US"]);
const enMap = Object.fromEntries(enEntries.map(e => [e.key, e.value]));

// Detect untranslated per non-English locale
const report = {};
for (const lang of langs) {
  if (lang === "en-US") { continue; }

  const langEntries = getAllLeafPaths(data[lang]);
  const untranslated = [];
  const missing = [];

  const langKeySet = new Set(langEntries.map(e => e.key));

  for (const { key, value } of langEntries) {
    const enVal = enMap[key];
    if (enVal !== undefined && typeof value === "string" && value === enVal) {
      const topCat = key.split(".")[0];
      untranslated.push({ key, value, category: topCat });
    }
  }

  // Also find keys in en-US that don't exist at all in this language
  for (const { key } of enEntries) {
    if (!langKeySet.has(key)) {
      missing.push({ key, value: enMap[key] });
    }
  }

  report[lang] = { untranslated, missing, totalUntranslated: untranslated.length, totalMissing: missing.length };
}

// Summary by category for each language
console.log("=== 未翻译条目统计 ===\n");
for (const lang of langs) {
  if (lang === "en-US") { continue; }
  const r = report[lang];
  const cats = {};
  for (const e of r.untranslated) {
    cats[e.category] = (cats[e.category] || 0) + 1;
  }
  console.log(`${lang}: ${r.totalUntranslated} 未翻译, ${r.totalMissing} 缺失`);
  const sorted = Object.entries(cats).sort((a, b) => b[1] - a[1]);
  for (const [cat, count] of sorted.slice(0, 10)) {
    console.log(`  ${cat}: ${count}`);
  }
  console.log();
}

// Write full report
const outPath = "scripts/.i18n-untranslated-report.json";
writeFileSync(outPath, JSON.stringify(report, null, 2), "utf-8");
console.log(`完整报告已写入: ${outPath}`);
