import fs from "fs";

const zh = JSON.parse(fs.readFileSync("src/i18n/locales/zh-CN.json", "utf8"));
const ar = JSON.parse(fs.readFileSync("src/i18n/locales/ar.json", "utf8"));

function getAllKeys(obj, prefix = "") {
  const r = [];
  for (const k of Object.keys(obj || {})) {
    const f = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] && !Array.isArray(obj[k])) {
      r.push(...getAllKeys(obj[k], f));
    } else {
      r.push(f);
    }
  }
  return r;
}

const zhKeys = new Set(getAllKeys(zh));
const arKeys = new Set(getAllKeys(ar));
const missing = [...zhKeys].filter((k) => !arKeys.has(k));
const extra = [...arKeys].filter((k) => !zhKeys.has(k));

console.log("zh-CN keys:", zhKeys.size);
console.log("ar.json keys:", arKeys.size);
console.log("Missing in ar:", missing.length);
console.log("Extra in ar:", extra.length);

console.log("\n=== Missing keys in ar.json ===");
missing.forEach((k) => console.log(" - " + k));

if (extra.length > 0) {
  console.log("\n=== Extra keys in ar.json (not in zh-CN) ===");
  extra.forEach((k) => console.log(" - " + k));
}
