import fs from "fs";

// i18n 漂移审计 + 关键缺失键报告
//
// 旧版会在检测到缺失键时**自动用英文填充**（pseudo-translation），
// 实际并未经过翻译，污染了多语言文件并掩盖了真实漂移。
// 现在此脚本被 `scripts/i18n-check.mjs` 替代做严格的只读校验；本文件保留
// 仅用于**只读报告**—— 任何写入操作都直接报错，强制走真实翻译流程。
// 详见 `src/i18n/NO_PSEUDO_TRANSLATION.md`。
//
// 用法:  node src/i18n/compare_locales.js            # 打印漂移报告（只读）
//       (旧用法已禁用 — 写入将被拒绝)

const PSEUDO_TRANSLATION_FORBIDDEN =
  "pseudo-translation is forbidden — see src/i18n/NO_PSEUDO_TRANSLATION.md. " +
  "Use `node scripts/i18n-check.mjs` for read-only drift detection, " +
  "and run the real translation pipeline (en add → manual/service translate 12 langs) " +
  "to fill missing keys.";

function die(msg) {
  console.error(`[compare_locales.js] ${msg}`);
  process.exit(1);
}

// Hard guard: refuse to run the legacy auto-fill path even if some caller
// passes a flag we don't recognize.  Anyone wanting to write locale files
// must do so via the real translation pipeline.
if (process.argv.includes("--write") || process.argv.includes("--fill")) {
  die(PSEUDO_TRANSLATION_FORBIDDEN);
}

// 读取英文基准文件
const enFilePath = "./locales/en-US.json";
const enContent = JSON.parse(fs.readFileSync(enFilePath, "utf8"));

// 要检查的语言文件
const languages = [
  "zh-CN",
  "zh-TW",
  "ar",
  "de",
  "es",
  "fr",
  "hi",
  "ja",
  "ko",
  "ru",
];

// 递归获取所有键
function getAllKeys(obj, prefix = "") {
  let keys = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "object" && value !== null && !Array.isArray(value)) {
      keys = [...keys, ...getAllKeys(value, fullKey)];
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

// 比较两个语言文件，找出缺失的键
function findMissingKeys(baseObj, targetObj) {
  const baseKeys = getAllKeys(baseObj);
  const targetKeys = getAllKeys(targetObj);
  return baseKeys.filter((key) => !targetKeys.includes(key));
}

// 处理每个语言文件（只读模式：仅打印报告，绝不写入）
languages.forEach((lang) => {
  let filename;
  if (lang === "zh-CN" || lang === "zh-TW") {
    filename = lang;
  } else {
    filename = lang.toLowerCase();
  }
  const langFilePath = `./locales/${filename}.json`;

  if (fs.existsSync(langFilePath)) {
    const langContent = JSON.parse(fs.readFileSync(langFilePath, "utf8"));
    const missingKeys = findMissingKeys(enContent, langContent);

    console.log(`Language: ${lang}`);
    console.log(`Missing keys: ${missingKeys.length}`);
    console.log(`Missing keys: ${missingKeys.join(", ")}`);
    console.log("");
  } else {
    console.log(`Language file not found: ${langFilePath}`);
  }
  console.log("---");
});

console.log("Comparison completed (read-only).");
