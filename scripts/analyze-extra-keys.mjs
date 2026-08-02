// Analyze which extra i18n keys are used in code
const fs = require("fs");
const path = require("path");

const DIR = "src/i18n/locales";

function flatten(obj, prefix = "", out = new Set()) {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? prefix + "." + k : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      flatten(v, key, out);
    } else {
      out.add(key);
    }
  }
  return out;
}

// Load zh-CN keys
const zhData = JSON.parse(fs.readFileSync(path.join(DIR, "zh-CN.json"), "utf-8"));
const zhKeys = flatten(zhData);

const files = fs.readdirSync(DIR).filter((f) => f !== "zh-CN.json" && f.endsWith(".json"));

// Collect all unique extra keys
const extraKeys = new Set();
for (const file of files) {
  const data = JSON.parse(fs.readFileSync(path.join(DIR, file), "utf-8"));
  const keys = flatten(data);
  for (const k of keys) {
    if (!zhKeys.has(k)) {
      extraKeys.add(k);
    }
  }
}

console.log("Unique extra keys (not in zh-CN):", extraKeys.size);

// Walk source files
const SRC = "src";
const tsFiles = [];
function walkDir(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walkDir(full);
    else if (/\.(ts|tsx|js|jsx)$/.test(entry.name)) tsFiles.push(full);
  }
}
walkDir(SRC);
console.log("Source files scanned:", tsFiles.length);

// Check each extra key in code
const usedInCode = new Set();
const unusedInCode = new Set();

for (const key of extraKeys) {
  let found = false;
  for (const file of tsFiles) {
    const content = fs.readFileSync(file, "utf-8");
    if (content.includes(key)) {
      found = true;
      break;
    }
  }
  if (found) usedInCode.add(key);
  else unusedInCode.add(key);
}

console.log("Used in code:", usedInCode.size);
console.log("Unused in code:", unusedInCode.size);

// Save results
fs.writeFileSync("scripts/i18n-extra-used.json", JSON.stringify([...usedInCode].sort(), null, 2));
fs.writeFileSync("scripts/i18n-extra-unused.json", JSON.stringify([...unusedInCode].sort(), null, 2));
console.log("Saved results");

// Show unused by namespace
const unusedByNs = new Map();
for (const k of unusedInCode) {
  const ns = k.split(".")[0];
  if (!unusedByNs.has(ns)) unusedByNs.set(ns, []);
  unusedByNs.get(ns).push(k);
}
console.log("\nUnused keys by namespace:");
for (const [ns, keys] of [...unusedByNs.entries()].sort((a, b) => b[1].length - a[1].length)) {
  console.log("  " + ns + ": " + keys.length + " keys");
}

// Show used keys
if (usedInCode.size > 0) {
  console.log("\nUsed in code:");
  for (const k of [...usedInCode].sort()) {
    console.log("  " + k);
  }
}
