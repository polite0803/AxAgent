// scripts/_i18n-prune-ja.mjs — one-off for Task 1.4
// Removes any key from ja.json that en-US.json does not have.
// Writes the list of removed keys (one per line, with value) to
// docs/baseline/ja-extra-keys.txt for archival/audit.

import fs from "fs";

const LOCALES_DIR = "./src/i18n/locales";
const TARGET = "ja.json";
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

function getValueByKey(obj, key) {
  return key.split(".").reduce((acc, curr) => (acc == null ? acc : acc[curr]), obj);
}

function pruneExtras(baseObj, targetObj) {
  // Walk targetObj and remove any leaf whose dotted-path is not in base.
  const baseKeys = new Set(getKeys(baseObj));
  const removed = [];

  function visit(node, prefix) {
    if (!node || typeof node !== "object" || Array.isArray(node)) return;
    for (const k of Object.keys(node)) {
      const full = prefix ? `${prefix}.${k}` : k;
      const v = node[k];
      if (v && typeof v === "object" && !Array.isArray(v)) {
        visit(v, full);
        // After recursion, if the sub-object is now empty, drop it
        if (Object.keys(v).length === 0) {
          delete node[k];
        }
      } else {
        if (!baseKeys.has(full)) {
          removed.push(`${full}\t${JSON.stringify(v)}`);
          delete node[k];
        }
      }
    }
  }

  visit(targetObj, "");
  return { targetObj, removed };
}

const baseObj = JSON.parse(fs.readFileSync(`${LOCALES_DIR}/${BASE}`, "utf8"));
const targetObj = JSON.parse(fs.readFileSync(`${LOCALES_DIR}/${TARGET}`, "utf8"));
const { targetObj: pruned, removed } = pruneExtras(baseObj, targetObj);

// Drop empty top-level (or nested) containers that resulted from pruning
function stripEmpty(o) {
  if (!o || typeof o !== "object" || Array.isArray(o)) return o;
  for (const k of Object.keys(o)) {
    o[k] = stripEmpty(o[k]);
    if (o[k] && typeof o[k] === "object" && !Array.isArray(o[k]) && Object.keys(o[k]).length === 0) {
      delete o[k];
    }
  }
  return o;
}
stripEmpty(pruned);

fs.writeFileSync(`${LOCALES_DIR}/${TARGET}`, JSON.stringify(pruned, null, 2) + "\n", "utf8");
fs.mkdirSync("docs/baseline", { recursive: true });
fs.writeFileSync("docs/baseline/ja-extra-keys.txt", removed.join("\n") + "\n", "utf8");
console.log(`pruned ${removed.length} extra keys from ${TARGET} → docs/baseline/ja-extra-keys.txt`);
