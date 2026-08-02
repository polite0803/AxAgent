// Fix i18n consistency:
// 1. Add 59 USED keys to zh-CN (from en-US as reference)
// 2. Remove 536 UNUSED keys from all other language files
import fs from "node:fs";
import path from "node:path";

const DIR = "src/i18n/locales";

const usedKeys = JSON.parse(fs.readFileSync("scripts/i18n-used.json", "utf-8"));
const unusedKeys = JSON.parse(fs.readFileSync("scripts/i18n-unused.json", "utf-8"));

const zhCNPath = path.join(DIR, "zh-CN.json");
const zhCN = JSON.parse(fs.readFileSync(zhCNPath, "utf-8"));

function getVal(obj, p) {
  const keys = p.split(".");
  let c = obj;
  for (const k of keys) { if (c == null) return undefined; c = c[k]; }
  return c;
}

function setVal(obj, p, value) {
  const keys = p.split(".");
  let c = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    if (!c[keys[i]]) c[keys[i]] = {};
    c = c[keys[i]];
  }
  c[keys[keys.length - 1]] = value;
}

function delVal(obj, p) {
  const keys = p.split(".");
  let c = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    if (!c[keys[i]]) return;
    c = c[keys[i]];
  }
  delete c[keys[keys.length - 1]];
}

function cleanEmptyParents(obj, prefix = "") {
  for (const k of Object.keys(obj)) {
    const fullKey = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] !== null && !Array.isArray(obj[k])) {
      cleanEmptyParents(obj[k], fullKey);
      if (Object.keys(obj[k]).length === 0) {
        delete obj[k];
      }
    }
  }
}

// Step 1: Add USED keys to zh-CN (using en-US as reference for values)
console.log("=== Step 1: Adding USED keys to zh-CN ===");
const enUS = JSON.parse(fs.readFileSync(path.join(DIR, "en-US.json"), "utf-8"));
let addedCount = 0;

for (const key of usedKeys) {
  const existing = getVal(zhCN, key);
  if (existing !== undefined) continue;

  // Get value from en-US
  const enVal = getVal(enUS, key);
  if (enVal === undefined) {
    console.warn(`  No value in en-US for: ${key}`);
    continue;
  }

  setVal(zhCN, key, enVal);
  addedCount++;
  console.log(`  + ${key} = ${JSON.stringify(enVal).substring(0, 60)}`);
}

fs.writeFileSync(zhCNPath, JSON.stringify(zhCN, null, 2) + "\n", "utf-8");
console.log(`Added ${addedCount} keys to zh-CN`);

// Step 2: Remove UNUSED keys from all other language files
console.log("\n=== Step 2: Removing UNUSED keys from other languages ===");
const langFiles = fs.readdirSync(DIR).filter(f => f.endsWith(".json") && f !== "zh-CN.json");

for (const file of langFiles) {
  const filePath = path.join(DIR, file);
  const data = JSON.parse(fs.readFileSync(filePath, "utf-8"));
  let removedCount = 0;

  for (const key of unusedKeys) {
    if (getVal(data, key) !== undefined) {
      delVal(data, key);
      removedCount++;
    }
  }

  // Clean up empty parent objects
  cleanEmptyParents(data);

  fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + "\n", "utf-8");
  console.log(`  [${file}] removed ${removedCount} keys`);
}

// Step 3: Remove UNUSED keys from zh-CN too (in case they exist there)
console.log("\n=== Step 3: Removing UNUSED keys from zh-CN ===");
const zhCN2 = JSON.parse(fs.readFileSync(zhCNPath, "utf-8"));
let zhRemoved = 0;
for (const key of unusedKeys) {
  if (getVal(zhCN2, key) !== undefined) {
    delVal(zhCN2, key);
    zhRemoved++;
  }
}
cleanEmptyParents(zhCN2);
fs.writeFileSync(zhCNPath, JSON.stringify(zhCN2, null, 2) + "\n", "utf-8");
console.log(`Removed ${zhRemoved} unused keys from zh-CN`);

console.log("\nDone!");