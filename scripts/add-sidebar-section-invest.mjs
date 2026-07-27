// scripts/add-sidebar-section-invest.mjs
// 一次性为 10 个非 en-US locale 添加 sidebar.sectionInvest 翻译
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCALES = join(__dirname, "..", "src", "i18n", "locales");

// 各语言 sectionInvest 翻译(英文 "Invest" 已存在)
const TRANSLATIONS = {
  "zh-CN": "投资",
  "zh-TW": "投資",
  "ja": "投資",
  "ko": "투자",
  "de": "Investieren",
  "fr": "Investissement",
  "es": "Inversión",
  "ru": "Инвестиции",
  "hi": "निवेश",
  "ar": "الاستثمار",
};

for (const [locale, value] of Object.entries(TRANSLATIONS)) {
  const file = join(LOCALES, `${locale}.json`);
  const json = JSON.parse(readFileSync(file, "utf8"));
  if (!json.sidebar) {
    console.warn(`[skip] ${locale}.json: no sidebar block`);
    continue;
  }
  if (json.sidebar.sectionInvest) {
    console.log(`[exists] ${locale}.json sidebar.sectionInvest = ${json.sidebar.sectionInvest}`);
    continue;
  }
  // 在 sectionInfrastructure 前插入 sectionInvest
  const reordered = { sectionWork: undefined, sectionTools: undefined };
  const out = {};
  for (const [k, v] of Object.entries(json.sidebar)) {
    if (k === "sectionInfrastructure") {
      out.sectionInvest = value;
    }
    out[k] = v;
  }
  json.sidebar = out;
  writeFileSync(file, JSON.stringify(json, null, 2) + "\n", "utf8");
  console.log(`[ok] ${locale}.json: added sectionInvest = ${value}`);
}
