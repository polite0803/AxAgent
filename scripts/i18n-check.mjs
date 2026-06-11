// scripts/i18n-check.mjs
//
// Read-only i18n consistency checker.
//
// Policy:
//   - `extra` keys (locale has a key not in en-US) is the dangerous drift.
//     It means a translator or developer added a string to a single locale
//     without going through the central pipeline. i18next has no way to
//     surface this to English-speaking users. → HARD FAIL.
//   - `missing` keys (locale does NOT have a key that en-US has) is normal
//     lag — translators haven't finished. i18next falls back to en-US at
//     runtime. → WARN ONLY.
//
//   See `src/i18n/NO_PSEUDO_TRANSLATION.md` for the no-pseudo-translation
//   rule that drives this policy.
//
// Usage:
//   node scripts/i18n-check.mjs                   # default: fail on extras, warn on missing
//   node scripts/i18n-check.mjs --strict          # fail on BOTH extras and missing
//   node scripts/i18n-check.mjs --report          # never exit 1; report only
//   node scripts/i18n-check.mjs --allow-missing   # also OK; same as default
//
// Allowlist (optional): `scripts/i18n-known-drift.json` — known extras/missing
// per locale, acknowledged drift. Format:
//   {
//     "<locale-file>": { "extra": ["path.to.key"], "missing": ["path.to.key"] }
//   }
//
// Exit 0:  no unallowed extras (and --strict: also no unallowed missing)
// Exit 1:  unallowed extras found (or unallowed missing in --strict mode)

import fs from "fs";
import path from "path";

const LOCALES_DIR = "./src/i18n/locales";
const BASE = "en-US.json";
const ALLOWLIST_PATH = "./scripts/i18n-known-drift.json";

const args = process.argv.slice(2);
const REPORT_ONLY = args.includes("--report");
const STRICT = args.includes("--strict"); // also fail on missing

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

function loadAllowlist() {
  if (!fs.existsSync(ALLOWLIST_PATH)) return {};
  try {
    return JSON.parse(fs.readFileSync(ALLOWLIST_PATH, "utf8"));
  } catch (e) {
    console.error(`WARN: failed to parse ${ALLOWLIST_PATH}: ${e.message}`);
    return {};
  }
}

const basePath = path.join(LOCALES_DIR, BASE);
const baseKeys = new Set(getKeys(JSON.parse(fs.readFileSync(basePath, "utf8"))));
const allowlist = loadAllowlist();

let totalUnallowedExtras = 0;
let totalUnallowedMissing = 0;
let totalAllowedExtras = 0;
let totalAllowedMissing = 0;
let totalAllowlistSize = 0;
const report = [];

for (const f of fs.readdirSync(LOCALES_DIR).filter((x) => x.endsWith(".json"))) {
  if (f === BASE) continue;
  const target = JSON.parse(fs.readFileSync(path.join(LOCALES_DIR, f), "utf8"));
  const targetKeys = new Set(getKeys(target));

  const allMissing = [...baseKeys].filter((k) => !targetKeys.has(k));
  const allExtra = [...targetKeys].filter((k) => !baseKeys.has(k));

  const allowed = allowlist[f] || { missing: [], extra: [] };
  const allowedMissing = new Set(allowed.missing || []);
  const allowedExtra = new Set(allowed.extra || []);

  const unallowedMissing = allMissing.filter((k) => !allowedMissing.has(k));
  const unallowedExtra = allExtra.filter((k) => !allowedExtra.has(k));
  const allowedMissingCount = allMissing.length - unallowedMissing.length;
  const allowedExtraCount = allExtra.length - unallowedExtra.length;
  const allowlistSize = (allowed.missing || []).length + (allowed.extra || []).length;

  totalUnallowedExtras += unallowedExtra.length;
  totalUnallowedMissing += unallowedMissing.length;
  totalAllowedExtras += allowedExtraCount;
  totalAllowedMissing += allowedMissingCount;
  totalAllowlistSize += allowlistSize;

  // Print EVERY locale's stats (even clean ones, so a maintainer can see progress)
  const tags = [];
  if (unallowedExtra.length > 0) tags.push(`FAIL-extras=${unallowedExtra.length}`);
  if (unallowedMissing.length > 0) tags.push(`WARN-missing=${unallowedMissing.length}`);
  if (tags.length === 0) tags.push("OK");
  if (allowlistSize > 0) tags.push(`(allowlist covers ${allowedExtraCount} extra + ${allowedMissingCount} missing)`);
  console.error(`[${f}] ${tags.join("  ")}`);
  if (unallowedExtra.length > 0) {
    unallowedExtra.slice(0, 5).forEach((k) => console.error(`  + extra:   ${k}`));
  }
  if (unallowedMissing.length > 0) {
    unallowedMissing.slice(0, 3).forEach((k) => console.error(`  - missing: ${k}`));
  }
  report.push({ file: f, unallowedMissing, unallowedExtra, allowlistSize });
}

console.error("");
console.error(
  `i18n check: unallowed-extras=${totalUnallowedExtras} unallowed-missing=${totalUnallowedMissing}` +
    ` allowlist-extra=${totalAllowedExtras} allowlist-missing=${totalAllowedMissing}`,
);

if (REPORT_ONLY) {
  console.error("(report mode — exiting 0 regardless)");
  process.exit(0);
}

// Default policy: fail on extras, warn on missing
if (totalUnallowedExtras > 0) {
  process.exit(1);
}

// Strict mode: also fail on missing
if (STRICT && totalUnallowedMissing > 0) {
  console.error(
    `STRICT mode: ${totalUnallowedMissing} unallowed missing keys. ` +
      `Add to scripts/i18n-known-drift.json or translate.`,
  );
  process.exit(1);
}

process.exit(0);
