// Check which "extra" keys (in other langs but not zh-CN) are actually used in code
import fs from "node:fs";
import path from "node:path";

const LOCALES = "src/i18n/locales";
const SRC = "src";

const zhCN = JSON.parse(fs.readFileSync(path.join(LOCALES, "zh-CN.json"), "utf-8"));

function getAllPaths(obj, prefix = "") {
  const paths = [];
  for (const k of Object.keys(obj)) {
    const fullKey = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] !== null && !Array.isArray(obj[k])) {
      paths.push(...getAllPaths(obj[k], fullKey));
    } else {
      paths.push(fullKey);
    }
  }
  return paths;
}

function getVal(obj, p) {
  const keys = p.split(".");
  let c = obj;
  for (const k of keys) {
    if (c == null) return undefined;
    c = c[k];
  }
  return c;
}

const zhCNPaths = new Set(getAllPaths(zhCN));

// Collect all extra keys from other language files (union)
const extraKeys = new Set();
const files = fs.readdirSync(LOCALES).filter(f => f.endsWith(".json") && f !== "zh-CN.json");

for (const file of files) {
  const data = JSON.parse(fs.readFileSync(path.join(LOCALES, file), "utf-8"));
  for (const p of getAllPaths(data)) {
    if (!zhCNPaths.has(p)) {
      extraKeys.add(p);
    }
  }
}

console.log(`Total extra keys (not in zh-CN): ${extraKeys.size}`);

// Build a set of all i18n references in code by scanning source files
// Patterns: t('key'), t("key"), useTranslation().t('key'), etc.
// We search for the key path components

// First, get all TS/TSX files
function getAllFiles(dir, exts) {
  const result = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) {
      result.push(...getAllFiles(full, exts));
    } else if (exts.some(ext => e.name.endsWith(ext))) {
      result.push(full);
    }
  }
  return result;
}

const sourceFiles = getAllFiles(SRC, [".ts", ".tsx", ".js", ".jsx"]);
console.log(`Scanning ${sourceFiles.length} source files for i18n key usage...`);

// Read all source files content once
const fileContents = new Map();
for (const f of sourceFiles) {
  try {
    fileContents.set(f, fs.readFileSync(f, "utf-8"));
  } catch {}
}

// Check each extra key if it's used in any source file
const USED = [];
const UNUSED = [];

for (const key of extraKeys) {
  // For a key like "learningGraph.stockAnalysis.configDescriptions.ruleRsiOverbought"
  // search for parts of it in t() calls
  // We search for distinctive sub-parts to avoid false positives
  
  // Strategy: build search patterns that would indicate usage
  // Key usage typically: t('full.key.path') or t("full.key.path")
  // Also used via t('prefix') and then sub-key access
  
  // Let's search for the leaf key name in context of its parent
  const parts = key.split(".");
  
  // Build patterns to search:
  // 1. The full key in quotes
  // 2. The last 2 segments (more distinctive)
  // 3. The last segment (may be common)
  
  const patterns = [
    `'${key}'`,
    `"${key}"`,
  ];
  
  // Also search for partial paths (e.g., if code does t('learningGraph')['stockAnalysis']...)
  // But that's too complex. Let's focus on the common pattern: t('full.key')
  
  let found = false;
  for (const [file, content] of fileContents) {
    for (const pat of patterns) {
      if (content.includes(pat)) {
        found = true;
        break;
      }
    }
    if (found) break;
  }
  
  if (found) {
    USED.push(key);
  } else {
    UNUSED.push(key);
  }
}

console.log(`\nUsed in code: ${USED.length}`);
console.log(`Not used in code: ${UNUSED.length}`);

console.log("\n=== USED keys (sample 50) ===");
USED.slice(0, 50).forEach(k => console.log(`  ${k}`));
if (USED.length > 50) console.log(`  ... and ${USED.length - 50} more`);

console.log("\n=== UNUSED keys (sample 50) ===");
UNUSED.slice(0, 50).forEach(k => console.log(`  ${k}`));
if (UNUSED.length > 50) console.log(`  ... and ${UNUSED.length - 50} more`);

// Save the full lists for next step
fs.writeFileSync("scripts/i18n-used.json", JSON.stringify(USED, null, 2));
fs.writeFileSync("scripts/i18n-unused.json", JSON.stringify(UNUSED, null, 2));
console.log("\nSaved full lists to scripts/i18n-used.json and scripts/i18n-unused.json");