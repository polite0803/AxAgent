const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..", "src", "i18n", "locales");
const BASE = "en-US.json";

function getKeys(obj, prefix) {
  prefix = prefix || "";
  const keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const full = prefix ? prefix + "." + k : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      keys.push(...getKeys(v, full));
    } else {
      keys.push(full);
    }
  }
  return keys;
}

function getValueByPath(obj, path) {
  const parts = path.split(".");
  let cur = obj;
  for (const p of parts) {
    if (!cur || typeof cur !== "object") { return undefined; }
    cur = cur[p];
  }
  return cur;
}

function setValueByPath(obj, path, value) {
  const parts = path.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]] || typeof cur[parts[i]] !== "object") {
      cur[parts[i]] = {};
    }
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}

console.log("=== i18n Audit ===");

const baseData = JSON.parse(fs.readFileSync(path.join(ROOT, BASE), "utf8"));
const baseKeys = new Set(getKeys(baseData));
console.log(`Base (en-US): ${baseKeys.size} keys`);

const files = fs.readdirSync(ROOT).filter(function(x) {
  return x.endsWith(".json") && x !== BASE;
});

for (const file of files) {
  const lang = file.replace(".json", "");
  const data = JSON.parse(fs.readFileSync(path.join(ROOT, file), "utf8"));
  const keys = new Set(getKeys(data));
  const missing = [...baseKeys].filter(function(k) {
    return !keys.has(k);
  });
  const extra = [...keys].filter(function(k) {
    return !baseKeys.has(k);
  });

  let status = "";
  if (missing.length > 0) { status += " missing=" + missing.length; }
  if (extra.length > 0) { status += " extra=" + extra.length; }
  if (!status) { status = " OK"; }

  console.log(`${file}: ${keys.size} keys${status}`);

  if (missing.length > 0) {
    console.log("  Missing keys (first 10):");
    missing.slice(0, 10).forEach(function(k) {
      console.log("    - " + k);
    });
  }
  if (extra.length > 0) {
    console.log("  Extra keys (first 10):");
    extra.slice(0, 10).forEach(function(k) {
      console.log("    + " + k);
    });
  }
}

console.log("\nDone.");
