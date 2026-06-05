// Comprehensive i18n audit: find keys used in code but missing from en-US.json
const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..");
const SRC = path.join(ROOT, "src");
const LOCALES_DIR = path.join(SRC, "i18n", "locales");
const enUS = JSON.parse(fs.readFileSync(path.join(LOCALES_DIR, "en-US.json"), "utf8"));

function getAllKeys(obj, prefix = "") {
  const keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const fk = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      keys.push(...getAllKeys(v, fk));
    } else {
      keys.push(fk);
    }
  }
  return keys;
}
const allDefined = new Set(getAllKeys(enUS));

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === "dist" || entry.name === "build") { continue; }
    if (entry.name === "i18n" && dir === SRC) { continue; }
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) { walk(full, out); }
    else if (/\.(tsx?|jsx?)$/.test(entry.name)) { out.push(full); }
  }
  return out;
}
const files = walk(SRC);
console.log(`Scanning ${files.length} files...\n`);

// Each pattern is paired with the index of the captured key
const PATTERNS = [
  // t("...") or t('...')
  [/(?<![\w$.])t\s*\(\s*(['"])([^'"\n]{2,}?)\1\s*(?:[,)])/g, 2],
  // t(`...`)
  [/(?<![\w$.])t\s*\(\s*`([^`\n]{2,}?)`\s*(?:[,)])/g, 1],
  // i18next.t("...") / i18n.t("...") / translation.t("...")
  [/\b(?:i18next|i18n|translation)\s*\.\s*t\s*\(\s*(['"])([^'"\n]{2,}?)\1\s*(?:[,)])/g, 2],
  // i18next.t(`...`)
  [/\b(?:i18next|i18n|translation)\s*\.\s*t\s*\(\s*`([^`\n]{2,}?)`\s*(?:[,)])/g, 1],
  // Trans i18nKey="..."
  [/i18nKey\s*=\s*(['"])([^'"\n]{2,}?)\1/g, 2],
];

const isDynamic = (key) =>
  /\$\{/.test(key)
  || /\+\s*['"`]/.test(key)
  || /\?\s*['"`]/.test(key);

const used = new Map();
function record(key, file, line) {
  if (!key || isDynamic(key)) { return; }
  if (!key.includes(".")) { return; // skip bare words like t("ok")
   }
  if (!used.has(key)) { used.set(key, []); }
  used.get(key).push({ file, line });
}

for (const file of files) {
  const text = fs.readFileSync(file, "utf8");
  const relFile = path.relative(ROOT, file);
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s*(\/\/|\*|\/\*)/.test(line)) { continue; }
    for (const [re, keyIdx] of PATTERNS) {
      re.lastIndex = 0;
      let m;
      while ((m = re.exec(line)) !== null) {
        record(m[keyIdx], relFile, i + 1);
      }
    }
  }
}

const missing = [];
for (const [key, refs] of used) {
  if (!allDefined.has(key)) { missing.push({ key, refs }); }
}
missing.sort((a, b) => a.key.localeCompare(b.key));

console.log(`Total unique keys used in code: ${used.size}`);
console.log(`Total keys defined in en-US.json: ${allDefined.size}`);
console.log(`Missing keys: ${missing.length}\n`);

if (missing.length === 0) {
  console.log("All i18n keys are present in en-US.json");
} else {
  console.log("=== MISSING KEYS (in code, not in en-US.json) ===\n");
  for (const { key, refs } of missing) {
    console.log(`  ${key}  (used ${refs.length}x)`);
    for (const r of refs.slice(0, 5)) {
      console.log(`    ${r.file}:${r.line}`);
    }
    if (refs.length > 5) { console.log(`    ...and ${refs.length - 5} more`); }
    console.log("");
  }
}

// Also output JSON for machine processing (only if --json or argv[2] is given)
const writeJson = process.argv.includes("--json") || (process.argv[2] && !process.argv[2].startsWith("--"));
const jsonOut = (process.argv.find(a => !a.startsWith("--") && a !== process.argv[0] && a !== process.argv[1]))
  || "audit_result.json";
if (writeJson) {
  fs.writeFileSync(
    jsonOut,
    JSON.stringify(
      {
        usedCount: used.size,
        definedCount: allDefined.size,
        missingCount: missing.length,
        missing: missing.map(({ key, refs }) => ({
          key,
          count: refs.length,
          refs: refs.map(r => `${r.file}:${r.line}`),
        })),
      },
      null,
      2,
    ),
  );
  console.log(`\nWrote ${jsonOut}`);
}
