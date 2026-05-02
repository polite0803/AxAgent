import fs from "fs";

const en = JSON.parse(fs.readFileSync("src/i18n/locales/en-US.json", "utf8"));

function findUntranslated(enObj, langObj, prefix = "") {
  const results = [];
  for (const k in enObj) {
    const p = prefix ? prefix + "." + k : k;
    if (typeof enObj[k] === "string" && typeof langObj[k] === "string") {
      if (enObj[k] === langObj[k]) {
        results.push({ path: p, value: enObj[k] });
      }
    } else if (typeof enObj[k] === "object" && enObj[k] !== null && !Array.isArray(enObj[k])) {
      if (langObj[k] && typeof langObj[k] === "object") {
        results.push(...findUntranslated(enObj[k], langObj[k], p));
      }
    }
  }
  return results;
}

const langs = [
  "ar.json",
  "de.json",
  "es.json",
  "fr.json",
  "hi.json",
  "ja.json",
  "ko.json",
  "ru.json",
  "zh-CN.json",
  "zh-TW.json",
];

const output = [];
let totalAll = 0;
for (const file of langs) {
  const lang = JSON.parse(fs.readFileSync("src/i18n/locales/" + file, "utf8"));
  const u = findUntranslated(en, lang);
  output.push(`=== ${file} (${u.length} untranslated) ===`);
  if (u.length) {
    u.forEach(s => output.push(`  ${s.path} = "${s.value}"`));
    totalAll += u.length;
  }
  output.push("");
}
output.push(`Total untranslated across all languages: ${totalAll}`);
fs.writeFileSync("scripts/untranslated-report.txt", output.join("\n"), "utf8");
console.log("Report written to scripts/untranslated-report.txt");
console.log(output.join("\n"));
