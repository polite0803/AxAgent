/**
 * 修补语言文件：将非英文资源文件中与英文原文相同的值替换为目标翻译
 * 用法: node scripts/patch-locale.mjs <目标语言文件> <源语言文件>
 * 例如: node scripts/patch-locale.mjs zh-TW.json zh-CN.json
 */
import fs from "fs";

const targetFile = process.argv[2];
const sourceFile = process.argv[3];

if (!targetFile) {
  console.error("Usage: node scripts/patch-locale.mjs <target.json> [source.json]");
  process.exit(1);
}

const en = JSON.parse(fs.readFileSync("src/i18n/locales/en-US.json", "utf8"));
const target = JSON.parse(fs.readFileSync("src/i18n/locales/" + targetFile, "utf8"));
const source = sourceFile
  ? JSON.parse(fs.readFileSync("src/i18n/locales/" + sourceFile, "utf8"))
  : null;

let patchedCount = 0;

function patch(obj, enObj, targetObj, path = "") {
  for (const k in enObj) {
    const p = path ? path + "." + k : k;
    if (typeof enObj[k] === "string") {
      // If target value equals English value (untranslated)
      if (typeof targetObj[k] === "string" && targetObj[k] === enObj[k]) {
        let translation = null;
        if (source && sourceFile) {
          // Use source translation if available
          const keys = p.split(".");
          let src = source;
          for (const key of keys) {
            if (src && typeof src === "object") { src = src[key]; }
            else {
              src = null;
              break;
            }
          }
          if (typeof src === "string") { translation = src; }
        }
        if (translation && translation !== enObj[k]) {
          targetObj[k] = translation;
          patchedCount++;
          console.log(`  PATCHED ${p}: "${enObj[k]}" -> "${translation}"`);
        }
      }
    } else if (typeof enObj[k] === "object" && enObj[k] !== null && !Array.isArray(enObj[k])) {
      if (targetObj[k] && typeof targetObj[k] === "object") {
        patch(obj, enObj[k], targetObj[k], p);
      }
    }
  }
}

patch(target, en, target);

fs.writeFileSync("src/i18n/locales/" + targetFile, JSON.stringify(target, null, 2) + "\n", "utf8");
console.log(`\nDone. Patched ${patchedCount} entries in ${targetFile}`);
