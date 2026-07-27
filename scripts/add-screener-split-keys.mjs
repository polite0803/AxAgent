// 给所有 locale 添加 2 个新 screener i18n 键
// 用法:node scripts/add-screener-split-keys.mjs
import fs from "node:fs";
import path from "node:path";

const LOCALES_DIR = path.resolve("src/i18n/locales");

const TRANSLATIONS = {
  "zh-CN": { todayRecommend: "今日荐股", myFilter: "我的筛选" },
  "zh-TW": { todayRecommend: "今日薦股", myFilter: "我的篩選" },
  "en-US": { todayRecommend: "Today's Picks", myFilter: "My Filter" },
  ja: { todayRecommend: "本日の推奨", myFilter: "マイフィルター" },
  ko: { todayRecommend: "오늘의 추천", myFilter: "내 필터" },
  de: { todayRecommend: "Heutige Tipps", myFilter: "Mein Filter" },
  fr: { todayRecommend: "Recommandations du jour", myFilter: "Mon filtre" },
  es: { todayRecommend: "Recomendaciones de hoy", myFilter: "Mi filtro" },
  ru: { todayRecommend: "Рекомендации дня", myFilter: "Мой фильтр" },
  hi: { todayRecommend: "आज की सिफारिशें", myFilter: "मेरा फ़िल्टर" },
  ar: { todayRecommend: "توصيات اليوم", myFilter: "تصفيتي" },
};

const files = fs.readdirSync(LOCALES_DIR).filter((f) => f.endsWith(".json"));
for (const file of files) {
  const filePath = path.join(LOCALES_DIR, file);
  const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
  const locale = data?.meta?.locale ?? file.replace(".json", "");
  const tr = TRANSLATIONS[locale] ?? TRANSLATIONS["en-US"];

  if (!data.stockAnalysis?.settings?.screener) {
    console.warn(`  ! ${file}: no stockAnalysis.settings.screener block, skipping`);
    continue;
  }
  const s = data.stockAnalysis.settings.screener;
  let changed = false;
  for (const k of ["todayRecommend", "myFilter"]) {
    if (typeof s[k] !== "string") {
      s[k] = tr[k];
      changed = true;
    }
  }
  if (changed) {
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + "\n", "utf8");
    console.log(`  ✓ ${file}: added 2 keys`);
  } else {
    console.log(`  = ${file}: already has keys`);
  }
}
console.log("done");
