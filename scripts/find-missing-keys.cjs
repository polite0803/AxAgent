const fs = require("fs");
const path = require("path");

function getAllKeys(obj, prefix = "") {
  let keys = [];
  for (const k in obj) {
    const p = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] !== null) {
      keys.push(...getAllKeys(obj[k], p));
    } else {
      keys.push(p);
    }
  }
  return keys;
}

function getAllFiles(dir, exts) {
  const files = [];
  function walk(d) {
    const entries = fs.readdirSync(d, { withFileTypes: true });
    for (const e of entries) {
      const full = path.join(d, e.name);
      if (e.isDirectory() && !e.name.includes("node_modules") && !e.name.startsWith(".")) {
        walk(full);
      } else if (e.isFile() && exts.some(ext => e.name.endsWith(ext))) {
        files.push(full);
      }
    }
  }
  walk(dir);
  return files;
}

const en = JSON.parse(fs.readFileSync("src/i18n/locales/en-US.json", "utf8"));
const enKeySet = new Set(getAllKeys(en));

// Scan all tsx files in src/components
const files = getAllFiles("src/components", [".ts", ".tsx"]);

const used = new Set();

// Match t('key') with various patterns - only keys that look like i18n keys (contain letters and dots)
const patterns = [
  /t\s*\(\s*'([a-zA-Z][a-zA-Z0-9_.]*)'\s*[,\)]/g,
  /t\s*\(\s*"([a-zA-Z][a-zA-Z0-9_.]*)"\s*[,\)]/g,
];

// Filter out non-i18n patterns
const invalidPatterns = [
  /^(html|body|div|span|p|a|button|input|select|textarea|form|label|ul|ol|li|h[1-6]|table|tr|td|th|img|br)$/i, // HTML tags
  /^[a-z]{2,4}$/, // Short words like "json", "csv", "a"
  /^\s*$/, // Whitespace
];

let totalMatches = 0;
for (const f of files) {
  const c = fs.readFileSync(f, "utf8");
  for (const p of patterns) {
    let m;
    p.lastIndex = 0;
    while ((m = p.exec(c)) !== null) {
      const key = m[1];
      // Filter out invalid keys
      const isInvalid = invalidPatterns.some(pat => pat.test(key));
      if (!isInvalid) {
        used.add(key);
        totalMatches++;
      }
    }
  }
}

const missing = [...used].filter(k => !enKeySet.has(k));
console.log("Total files scanned:", files.length);
console.log("Total t() calls found:", totalMatches);
console.log("Unique keys used:", used.size);
console.log("Missing keys (not in en-US.json):", missing.length);
console.log("");
if (missing.length > 0) {
  missing.forEach(k => console.log("  -", k));
}
