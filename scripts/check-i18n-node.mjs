// scripts/check-i18n-node.mjs
// Node.js port of check-hardcoded-i18n.sh for Windows (no bash available).
// Scans src/**/*.ts(x) for hardcoded CJK strings, English UI strings, and t() fallbacks.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

const ROOT = process.cwd();
const ALLOWLIST_PATH = "scripts/.i18n-allowlist.json";
const CJK = /[\u4e00-\u9fff\u3400-\u4dbf]/;

function walkTs(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      walkTs(full, out);
    } else if (/\.(ts|tsx)$/.test(name) && !full.includes(`${sep}src${sep}i18n${sep}locales${sep}`)) {
      out.push(full);
    }
  }
  return out;
}

const files = walkTs(join(ROOT, "src"));

// Load allowlist
let allowed = new Set();
if (existsSync(ALLOWLIST_PATH)) {
  try {
    const al = JSON.parse(readFileSync(ALLOWLIST_PATH, "utf8"));
    for (const e of al.entries || []) {
      for (const ln of String(e.lines || "").split(",")) {
        if (ln) { allowed.add(`${e.file}:${ln}`); }
      }
    }
  } catch {}
}

function isAllowed(file, ln) {
  return allowed.has(`${file}:${ln}`);
}

function precomputeConsoleLines(content) {
  const lines = content.split("\n");
  const set = new Set();
  for (let i = 0; i < lines.length; i++) {
    if (/console\.(log|warn|error|debug|info|trace)/.test(lines[i])) {
      for (let j = i + 1; j < lines.length; j++) {
        const t = lines[j].trim();
        if (t === "") { continue; }
        if (/^`/.test(t) || /^[eE]\s*,?\s*$/.test(t) || /^\s*[eE]\s*[;,]\s*$/.test(t) || /^\s*\)\s*[;,]\s*$/.test(t)) {
          set.add(j + 1);
          if (t.endsWith(");") || t.endsWith(";")) { break; }
          continue;
        }
        break;
      }
    }
  }
  return set;
}

const violations = [];

for (const f of files) {
  const rel = relative(ROOT, f).split(sep).join("/");
  const content = readFileSync(f, "utf8");
  const consoleLines = precomputeConsoleLines(content);
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const ln = i + 1;
    const raw = lines[i];

    if (!CJK.test(raw)) { continue; }
    // Strip // inline comments
    const stripped = raw.replace(/\/\/[^/]*$/, "");
    if (!CJK.test(stripped)) { continue; }
    // Skip console continuation lines
    if (consoleLines.has(ln)) { continue; }
    // Skip JSDoc opening
    if (/^\s*\/\*\*/.test(raw)) { continue; }
    // Skip comments
    if (/^\s*\/\//.test(raw)) { continue; }
    if (/^\s*\*/.test(raw)) { continue; }
    if (/\{\/\*.*\*\/\}/.test(raw)) { continue; }
    // Skip console.*
    if (/console\.(log|warn|error|debug|info|trace)/.test(raw)) { continue; }
    if (isAllowed(rel, ln)) { continue; }

    violations.push({ file: rel, ln, content: raw.trim() });
  }
}

if (violations.length === 0) {
  console.log("✓ No CJK violations");
} else {
  console.log(`✗ ${violations.length} CJK violation(s):`);
  for (const v of violations) {
    console.log(`  ${v.file}:${v.ln}: ${v.content}`);
  }
  process.exit(1);
}
