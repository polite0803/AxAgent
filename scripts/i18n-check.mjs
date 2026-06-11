// scripts/i18n-check.mjs
//
// Read-only i18n consistency checker.
// Compares every locale file against `en-US.json` and fails on any
// missing/extra keys.  All new translation keys must be added through
// the real translation pipeline (see `src/i18n/NO_PSEUDO_TRANSLATION.md`);
// auto-filling with English (pseudo-translation) is forbidden.
//
// Usage:   node scripts/i18n-check.mjs
// Exit 0:  all locales match en-US exactly
// Exit 1:  drift detected; offending keys printed to stderr

import fs from "fs";
import path from "path";

const LOCALES_DIR = "./src/i18n/locales";
const BASE = "en-US.json";

function getKeys(obj, prefix = "") {
  const keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const full = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      keys.push(...getKeys(v, full));
    } else {
      keys.push(full);
    }
  }
  return keys;
}

const basePath = path.join(LOCALES_DIR, BASE);
const baseKeys = new Set(getKeys(JSON.parse(fs.readFileSync(basePath, "utf8"))));

let failed = false;
for (const f of fs.readdirSync(LOCALES_DIR).filter((x) => x.endsWith(".json"))) {
  if (f === BASE) continue;
  const target = JSON.parse(fs.readFileSync(path.join(LOCALES_DIR, f), "utf8"));
  const targetKeys = new Set(getKeys(target));
  const missing = [...baseKeys].filter((k) => !targetKeys.has(k));
  const extra = [...targetKeys].filter((k) => !baseKeys.has(k));
  if (missing.length || extra.length) {
    console.error(`[${f}] missing=${missing.length} extra=${extra.length}`);
    missing.slice(0, 5).forEach((k) => console.error(`  - missing: ${k}`));
    extra.slice(0, 5).forEach((k) => console.error(`  + extra:   ${k}`));
    failed = true;
  }
}
process.exit(failed ? 1 : 0);
