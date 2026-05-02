import fs from "fs";

const en = JSON.parse(fs.readFileSync("src/i18n/locales/en-US.json", "utf8"));

function findMissingKeys(enObj, langObj, prefix = "") {
  const results = [];
  for (const k in enObj) {
    const p = prefix ? prefix + "." + k : k;
    if (typeof enObj[k] === "object" && enObj[k] !== null && !Array.isArray(enObj[k])) {
      if (langObj[k] && typeof langObj[k] === "object") {
        results.push(...findMissingKeys(enObj[k], langObj[k], p));
      } else {
        // 整个对象缺失
        results.push({ path: p, type: "object" });
      }
    } else if (!(k in langObj)) {
      // 键缺失
      results.push({ path: p, value: enObj[k], type: "string" });
    }
  }
  return results;
}

const langFile = "src/i18n/locales/zh-CN.json";
const lang = JSON.parse(fs.readFileSync(langFile, "utf8"));
const missing = findMissingKeys(en, lang);

console.log(`=== ${langFile} (${missing.length} missing keys) ===`);
if (missing.length) {
  missing.forEach(m => {
    if (m.type === "string") {
      console.log(`  ${m.path} = "${m.value}"`);
    } else {
      console.log(`  ${m.path} = [object]`);
    }
  });
}
