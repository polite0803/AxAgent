// scripts/_i18n-prune-extra-keys.mjs
//
// One-shot: remove the known extra keys (keys present in a locale but not in en-US)
// that were identified by `scripts/i18n-check.mjs`.
//
// IMPORTANT: This script removes ONLY leaf keys (string values) and ONLY the
// specific paths the check script reports. It does NOT recursively prune.
//
// Usage:  node scripts/_i18n-prune-extra-keys.mjs [--dry-run]
//
// Output: per-locale summary of removed keys; in --dry-run, no files are written.

import fs from "fs";
import path from "path";

const LOCALES_DIR = "./src/i18n/locales";
const BASE = "en-US.json";
const DRY_RUN = process.argv.includes("--dry-run");

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

function deleteKeyPath(obj, dottedPath) {
  const parts = dottedPath.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur || typeof cur !== "object" || !(parts[i] in cur)) return false;
    cur = cur[parts[i]];
  }
  if (!cur || typeof cur !== "object") return false;
  const leaf = parts[parts.length - 1];
  if (!(leaf in cur)) return false;
  delete cur[leaf];
  // Walk back and clean up empty containers
  for (let i = parts.length - 1; i > 0; i--) {
    const parent = parts.slice(0, i).reduce((o, p) => (o && p in o ? o[p] : null), obj);
    if (parent && typeof parent === "object" && Object.keys(parent).length === 0) {
      const grand = parts.slice(0, i - 1).reduce((o, p) => (o && p in o ? o[p] : null), obj);
      if (grand && typeof grand === "object") delete grand[parts[i - 1]];
    }
  }
  return true;
}

const en = JSON.parse(fs.readFileSync(path.join(LOCALES_DIR, BASE), "utf8"));
const enKeys = new Set(getKeys(en));

let totalRemoved = 0;
const summary = [];

for (const f of fs.readdirSync(LOCALES_DIR).filter((x) => x.endsWith(".json"))) {
  if (f === BASE) continue;
  const targetPath = path.join(LOCALES_DIR, f);
  const target = JSON.parse(fs.readFileSync(targetPath, "utf8"));
  const before = getKeys(target).length;
  const extras = getKeys(target).filter((k) => !enKeys.has(k));
  let removed = 0;
  for (const k of extras) {
    if (deleteKeyPath(target, k)) removed++;
  }
  if (removed > 0 && !DRY_RUN) {
    fs.writeFileSync(targetPath, JSON.stringify(target, null, 2) + "\n");
  }
  const after = getKeys(target).length;
  totalRemoved += removed;
  summary.push({ file: f, before, after, removed });
}

console.log("en-US key count:", enKeys.size);
console.table(summary);
console.log("Total extras pruned:", totalRemoved, DRY_RUN ? "(dry run — no files written)" : "");
