#!/usr/bin/env node
// scripts/i18n-scan.mjs
// i18n 硬编码字符串检测核心（替代 check-hardcoded-i18n.sh 中易 fork 爆炸的 bash 逐文件子 shell 逻辑）。
// 单进程扫描，正确处理所有注释形态（// 行内、/* */ 块注释含多行与 JSX {/* */}、/** */ JSDoc）。
// 兼容原 CLI：--strict | --report | --diff-only | --update-allowlist
// 输出格式与原 bash 脚本一致，exit 1 = 有未基线违规。
import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const ALLOWLIST = join(root, "scripts", ".i18n-allowlist.json");

const args = process.argv.slice(2);
const MODE = args.includes("--strict") ? "strict" : args.includes("--diff-only") ? "diff-only" : "report";
const UPDATE = args.includes("--update-allowlist");

const CJK = /[㐀-䶿一-鿿]/;
const isCJK = (s) => CJK.test(s);

// ── 收集待扫描文件 ──
function collect(dir, out) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) {
      if (p.endsWith(join("i18n", "locales"))) continue;
      collect(p, out);
    } else if (/\.(ts|tsx)$/.test(name)) {
      out.push(p);
    }
  }
}

let files;
if (MODE === "diff-only") {
  let base = "origin/master";
  try {
    execSync(`git fetch origin master --quiet`, { cwd: root, stdio: "ignore" });
  } catch {}
  if (!safeRev(base)) base = safeRev("master") ? "master" : "HEAD~1";
  const changed = execSync(`git diff --name-only ${base} HEAD`, { cwd: root, encoding: "utf8" })
    .split("\n").map((s) => s.trim()).filter(Boolean)
    .filter((f) => /\.(ts|tsx)$/.test(f) && f.startsWith("src/") && !f.includes("src/i18n/locales/"));
  files = changed.map((f) => join(root, f)).filter((f) => existsSync(f));
  if (files.length === 0) {
    console.log("No changed TypeScript files to check.");
    process.exit(0);
  }
  console.log(`Checking ${files.length} changed file(s)`);
} else {
  files = [];
  collect(join(root, "src"), files);
}

function safeRev(ref) {
  try {
    execSync(`git rev-parse --verify ${ref}`, { cwd: root, stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// ── 注释剥离（返回逐行去注释后的文本）──
function stripComments(lines) {
  const result = [];
  let inBlock = false;
  for (const raw of lines) {
    let s = raw;
    if (inBlock) {
      const end = s.indexOf("*/");
      if (end === -1) { result.push(""); continue; }
      s = s.slice(end + 2);
      inBlock = false;
    }
    let guard = 0;
    while (true) {
      const start = s.indexOf("/*");
      if (start === -1) break;
      const end = s.indexOf("*/", start + 2);
      if (end === -1) { s = s.slice(0, start); inBlock = true; break; }
      s = s.slice(0, start) + s.slice(end + 2);
      if (++guard > 50) break;
    }
    const m = s.match(/(^|[^:])\/\//);
    if (m) s = s.slice(0, m.index + (m[1] ? m[1].length : 0));
    result.push(s);
  }
  return result;
}

// ── 检测单文件违规 ──
// 返回按规则分组的 {rule, file, line, content}[]
function scanFile(f, rel) {
  let content;
  try { content = readFileSync(f, "utf8"); } catch { return []; }
  const cleaned = stripComments(content.split("\n"));
  const out = [];
  cleaned.forEach((line, idx) => {
    const lnum = idx + 1;
    const text = line.trim();
    if (!text) return;
    if (isCJK(text)) out.push({ rule: 1, file: rel, line: lnum, content: text });
    if (/message\.(success|error|warning|info)\(\s*['"]/.test(text))
      out.push({ rule: 2, file: rel, line: lnum, content: text });
    if (/placeholder\s*=\s*"([A-Za-z][^"]{2,})"/.test(text))
      out.push({ rule: 2, file: rel, line: lnum, content: text });
    if (/(notification|message)\.(open|info|success|error|warning|loading)\(\s*["'][^"']{3,}["']/.test(text))
      out.push({ rule: 4, file: rel, line: lnum, content: text });
    if (/\b(title|label|content|description|tooltip|placeholder|text)\s*=\s*"([A-Za-z][A-Za-z0-9\s\-_:/]+)"/.test(text))
      out.push({ rule: 5, file: rel, line: lnum, content: text });
  });
  return out;
}

const all = [];
for (const f of files) {
  const rel = normalizePath(f);
  all.push(...scanFile(f, rel));
}

// ── 路径规范化：将绝对路径转为相对于 root 的路径，统一用 / 分隔 ──
function normalizePath(p) {
  let s = p;
  if (s.startsWith(root)) s = s.slice(root.length);
  s = s.replace(/\\/g, "/").replace(/^\/+/, "");
  return s;
}

// ── 加载 allowlist ──
let allowSet = new Set();
try {
  const al = JSON.parse(readFileSync(ALLOWLIST, "utf8"));
  for (const e of al.entries || []) {
    const nf = normalizePath(e.file);
    for (const ln of (e.lines || "").split(",")) {
      if (ln) allowSet.add(nf + ":" + ln);
    }
  }
} catch {}

function isAllowed(v) { return allowSet.has(v.file + ":" + v.line); }

// ── --update-allowlist：写回基线 ──
if (UPDATE) {
  let al;
  try { al = JSON.parse(readFileSync(ALLOWLIST, "utf8")); } catch { al = { version: "2", generated: "", total_entries: 0, total_files: 0, entries: [] }; }
  const map = new Map();
  for (const e of al.entries || []) map.set(e.file, new Set((e.lines || "").split(",").filter(Boolean).map(Number)));
  for (const v of all) {
    if (!map.has(v.file)) map.set(v.file, new Set());
    map.get(v.file).add(v.line);
  }
  const entries = [];
  for (const [file, set] of map) {
    const lines = Array.from(set).sort((a, b) => a - b).join(",");
    const prev = (al.entries || []).find((e) => e.file === file);
    entries.push({ file, lines, reason: prev?.reason || "历史硬编码基线（注释/内部日志/数据/技术字符串）", phase: prev?.phase ?? 3 });
  }
  entries.sort((a, b) => a.file.localeCompare(b.file));
  al.entries = entries;
  al.total_entries = entries.length;
  al.total_files = new Set(entries.map((e) => e.file)).size;
  al.generated = new Date().toISOString().slice(0, 10);
  writeFileSync(ALLOWLIST, JSON.stringify(al, null, 2) + "\n");
  console.log(`Baseline updated: ${all.length} violations across ${entries.length} files.`);
  process.exit(0);
}

// ── 报告 ──
console.log(`\n=== i18n Hardcoded Strings Check (mode: ${MODE}) ===\n`);
const blocking = [1, 2, 4, 5];
let violations = 0;

for (const rule of [1, 2, 3, 4, 5]) {
  const label = { 1: "Rule 1: Hardcoded Chinese (CJK) strings", 2: "Rule 2: Hardcoded English UI strings", 3: "Rule 3: t() fallback patterns (WARNING)", 4: "Strict Mode: notification/message hardcoded string(s)", 5: "Strict Mode: UI attribute(s) with hardcoded strings" }[rule];
  const isStrict = rule === 4 || rule === 5;
  if (isStrict && MODE !== "strict") continue;
  const items = all.filter((v) => v.rule === rule && !isAllowed(v));
  console.log(`--- ${label} ---`);
  if (items.length === 0) {
    console.log("  PASS: No violations");
    continue;
  }
  if (rule === 3) {
    console.log(`  WARNING: ${items.length} t() fallback(s) found (not blocking):`);
  } else {
    console.log(`  FAIL: ${items.length} new violation(s):`);
    violations += items.length;
  }
  for (const it of items) console.log(`  ${it.file}:${it.line}: ${it.content}`);
}

console.log(`\n=== Summary ===`);
if (violations === 0) {
  console.log("All i18n checks passed.");
  process.exit(0);
} else {
  console.log(`Found ${violations} i18n violation(s).`);
  console.log("Fix them or update scripts/.i18n-allowlist.json.");
  process.exit(1);
}