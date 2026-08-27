// scripts/scan-missing-i18n-keys.cjs
// 扫描 src/**/*.{ts,tsx} 中所有静态 t("...") 调用，与 zh-CN.json 比对，输出缺失 key 报告。
// 用法: node scripts/scan-missing-i18n-keys.cjs [--json]
"use strict";
const fs = require("fs");
const path = require("path");

const ROOT = path.join(__dirname, "..");
const LOCALE = path.join(ROOT, "src", "i18n", "locales", "zh-CN.json");
const SRC = path.join(ROOT, "src");

const zh = JSON.parse(fs.readFileSync(LOCALE, "utf8"));
const get = (o, k) => k.split(".").reduce((x, p) => (x && typeof x === "object" ? x[p] : undefined), o);

const SKIP_DIRS = new Set(["node_modules", "dist", "i18n", "__tests__", "__mocks__", "e2e", "fixtures"]);

function walk(dir, out) {
  for (const n of fs.readdirSync(dir)) {
    const p = path.join(dir, n);
    const st = fs.statSync(p);
    if (st.isDirectory()) {
      if (SKIP_DIRS.has(n)) continue;
      walk(p, out);
    } else if (/\.(tsx|ts)$/.test(n) && !/\.(test|spec)\.tsx?$/.test(n)) {
      out.push(p);
    }
  }
}

const files = [];
walk(SRC, files);

const missing = new Map();
// t("key") 静态调用；排除 .t( 方法调用（前面是字母/点）与对象属性访问
const re = /[^A-Za-z.$)\]}]\bt\(\s*"([^"]+)"/g;

for (const f of files) {
  const src = fs.readFileSync(f, "utf8");
  let m;
  while ((m = re.exec(src)) !== null) {
    const key = m[1];
    if (key.includes("${") || key.includes("+") || key.includes("{{")) continue; // 动态 key 跳过
    if (typeof get(zh, key) !== "string") {
      if (!missing.has(key)) missing.set(key, new Set());
      missing.get(key).add(f.replace(/\\/g, "/").replace(path.join(ROOT, "src") + "/", ""));
    }
  }
}

const sorted = [...missing.entries()].sort((a, b) => b[1].size - a[1].size);
const bySeg = {};
for (const [k, v] of sorted) {
  const seg = k.split(".")[0];
  if (!bySeg[seg]) bySeg[seg] = { count: 0, keys: [] };
  bySeg[seg].count++;
  bySeg[seg].keys.push({ key: k, files: [...v] });
}

const segStats = Object.entries(bySeg).sort((a, b) => b[1].count - a[1].count)
  .map(([segment, info]) => ({ segment, count: info.count }));

if (process.argv.includes("--json")) {
  const report = {
    generatedAt: new Date().toISOString(),
    summary: {
      totalMissing: missing.size,
      scannedFiles: files.length,
      multiFileRefs: [...missing.values()].filter((s) => s.size > 1).length,
    },
    segmentStats: segStats,
    missing: Object.fromEntries(sorted.map(([k, v]) => [k, [...v]])),
  };
  fs.writeFileSync(path.join(ROOT, "output", "i18n-missing-keys-report.json"), JSON.stringify(report, null, 2));
  console.log("报告已写入 output/i18n-missing-keys-report.json");
  console.log("缺失总数:", missing.size, "| 扫描文件:", files.length);
  console.log("Top 15 段:");
  for (const s of segStats.slice(0, 15)) console.log("  " + s.segment.padEnd(25) + s.count);
} else {
  console.log("缺失 key 总数:", missing.size, "| 扫描文件:", files.length);
  console.log("被 2+ 文件引用的缺失 key:", [...missing.values()].filter((s) => s.size > 1).length);
  console.log("\nTop 20:");
  for (const [k, v] of sorted.slice(0, 20)) {
    console.log("  " + k.padEnd(58) + v.size + " 个文件");
  }
}
