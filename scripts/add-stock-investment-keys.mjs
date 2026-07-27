#!/usr/bin/env node
// scripts/add-stock-investment-keys.mjs
// 一次性脚本：给 11 个 locale 文件批量添加 Phase 1 引入的 5 个新页面的 i18n 键。
// 用法：node scripts/add-stock-investment-keys.mjs

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = resolve(fileURLToPath(import.meta.url), "..");
const LOCALES_DIR = resolve(__dirname, "..", "src", "i18n", "locales");

// 5 个新 nav 键 + 5 个新页面 section（每个 section 含 title / placeholder）
const NAV_KEYS = {
  watchlist: "Watchlist",
  screener: "Screener",
  trade: "Trade",
  backtest: "Backtest",
  compare: "Compare",
};

const PAGE_SECTIONS = {
  watchlist: {
    title: "Watchlist",
    placeholder: "Manage your watchlist and price alerts",
  },
  screener: {
    title: "Screener",
    placeholder: "Find stocks by criteria",
  },
  trade: {
    title: "Trade",
    placeholder: "Execute and review trades",
  },
  backtest: {
    title: "Backtest",
    placeholder: "Validate strategies with historical data",
  },
  compare: {
    title: "Compare",
    placeholder: "Compare stocks with peers",
  },
};

// 每种语言覆盖上面的英文默认值。zh-CN 是原生中文，其他语言用对应翻译。
// 不在表里的 locale 会用英文（与 ja.json 中 stockAnalysis.actionBuy="Buy" 等
// 已存在但未翻译的 key 保持一致风格；后续由母语者补全）。
const LOCALE_OVERRIDES = {
  "zh-CN": {
    nav: { watchlist: "自选", screener: "选股", trade: "交易", backtest: "回测", compare: "对比" },
    pages: {
      watchlist: { title: "自选股", placeholder: "管理你的自选股和价格提醒" },
      screener: { title: "选股中心", placeholder: "按条件筛选股票" },
      trade: { title: "交易与回放", placeholder: "执行与回放交易" },
      backtest: { title: "回测验证", placeholder: "用历史数据验证策略" },
      compare: { title: "对标研究", placeholder: "对比同业公司" },
    },
  },
  "zh-TW": {
    nav: { watchlist: "自選", screener: "選股", trade: "交易", backtest: "回測", compare: "對比" },
    pages: {
      watchlist: { title: "自選股", placeholder: "管理你的自選股和價格提醒" },
      screener: { title: "選股中心", placeholder: "依條件篩選股票" },
      trade: { title: "交易與回放", placeholder: "執行與回放交易" },
      backtest: { title: "回測驗證", placeholder: "用歷史資料驗證策略" },
      compare: { title: "對標研究", placeholder: "對比同業公司" },
    },
  },
  ja: {
    nav: { watchlist: "ウォッチリスト", screener: "スクリーナー", trade: "トレード", backtest: "バックテスト", compare: "比較" },
    pages: {
      watchlist: { title: "ウォッチリスト", placeholder: "ウォッチリストと価格アラートを管理" },
      screener: { title: "スクリーナー", placeholder: "条件を指定して銘柄を絞り込む" },
      trade: { title: "トレードとリプレイ", placeholder: "取引の実行と振り返り" },
      backtest: { title: "バックテスト", placeholder: "過去データで戦略を検証" },
      compare: { title: "比較", placeholder: "同業他社の銘柄と比較" },
    },
  },
  ko: {
    nav: { watchlist: "관심종목", screener: "스크리너", trade: "매매", backtest: "백테스트", compare: "비교" },
    pages: {
      watchlist: { title: "관심종목", placeholder: "관심종목과 가격 알림을 관리" },
      screener: { title: "종목 스크리너", placeholder: "조건으로 종목을 검색" },
      trade: { title: "매매 및 리플레이", placeholder: "거래 실행과 리플레이" },
      backtest: { title: "백테스트", placeholder: "과거 데이터로 전략 검증" },
      compare: { title: "비교 분석", placeholder: "동업 종목과 비교" },
    },
  },
  de: {
    nav: { watchlist: "Watchlist", screener: "Aktiensuche", trade: "Handel", backtest: "Backtest", compare: "Vergleichen" },
    pages: {
      watchlist: { title: "Watchlist", placeholder: "Watchlist und Preisalarme verwalten" },
      screener: { title: "Aktiensuche", placeholder: "Aktien nach Kriterien filtern" },
      trade: { title: "Handel und Replay", placeholder: "Trades ausführen und prüfen" },
      backtest: { title: "Backtest", placeholder: "Strategien mit historischen Daten validieren" },
      compare: { title: "Vergleich", placeholder: "Aktien mit Peers vergleichen" },
    },
  },
  fr: {
    nav: { watchlist: "Liste de suivi", screener: "Filtre", trade: "Trading", backtest: "Backtest", compare: "Comparer" },
    pages: {
      watchlist: { title: "Liste de suivi", placeholder: "Gérer la liste de suivi et les alertes" },
      screener: { title: "Filtre d'actions", placeholder: "Trouver des actions par critères" },
      trade: { title: "Trading et rejeu", placeholder: "Exécuter et revoir les trades" },
      backtest: { title: "Backtest", placeholder: "Valider les stratégies avec données historiques" },
      compare: { title: "Comparaison", placeholder: "Comparer les actions avec leurs pairs" },
    },
  },
  es: {
    nav: { watchlist: "Lista de seguimiento", screener: "Filtro", trade: "Operar", backtest: "Backtest", compare: "Comparar" },
    pages: {
      watchlist: { title: "Lista de seguimiento", placeholder: "Gestionar lista y alertas de precio" },
      screener: { title: "Filtro de acciones", placeholder: "Buscar acciones por criterios" },
      trade: { title: "Trading y reproducción", placeholder: "Ejecutar y revisar operaciones" },
      backtest: { title: "Backtest", placeholder: "Validar estrategias con datos históricos" },
      compare: { title: "Comparación", placeholder: "Comparar acciones con sus pares" },
    },
  },
  ru: {
    nav: { watchlist: "Список наблюдения", screener: "Скринер", trade: "Торговля", backtest: "Бэктест", compare: "Сравнить" },
    pages: {
      watchlist: { title: "Список наблюдения", placeholder: "Управление списком и оповещениями" },
      screener: { title: "Скринер акций", placeholder: "Найти акции по критериям" },
      trade: { title: "Торговля и воспроизведение", placeholder: "Исполнение и разбор сделок" },
      backtest: { title: "Бэктест", placeholder: "Проверка стратегий на истории" },
      compare: { title: "Сравнение", placeholder: "Сравнение с аналогами" },
    },
  },
  hi: {
    nav: { watchlist: "वॉचलिस्ट", screener: "स्क्रीनर", trade: "ट्रेड", backtest: "बैकटेस्ट", compare: "तुलना" },
    pages: {
      watchlist: { title: "वॉचलिस्ट", placeholder: "वॉचलिस्ट और मूल्य अलर्ट प्रबंधित करें" },
      screener: { title: "स्टॉक स्क्रीनर", placeholder: "मानदंडों से स्टॉक खोजें" },
      trade: { title: "ट्रेड और रीप्ले", placeholder: "ट्रेड निष्पादित और समीक्षा करें" },
      backtest: { title: "बैकटेस्ट", placeholder: "ऐतिहासिक डेटा से रणनीति सत्यापित करें" },
      compare: { title: "तुलना", placeholder: "साथियों के साथ स्टॉक तुलना" },
    },
  },
  ar: {
    nav: { watchlist: "قائمة المراقبة", screener: "المُصفّي", trade: "التداول", backtest: "اختبار رجعي", compare: "مقارنة" },
    pages: {
      watchlist: { title: "قائمة المراقبة", placeholder: "إدارة قائمة المراقبة وتنبيهات الأسعار" },
      screener: { title: "مُصفّي الأسهم", placeholder: "ابحث عن الأسهم حسب المعايير" },
      trade: { title: "التداول وإعادة التشغيل", placeholder: "تنفيذ ومراجعة الصفقات" },
      backtest: { title: "اختبار رجعي", placeholder: "تحقق من الاستراتيجيات ببيانات تاريخية" },
      compare: { title: "مقارنة", placeholder: "قارن الأسهم مع الأقران" },
    },
  },
};

function applyOverrides(localeName) {
  const override = LOCALE_OVERRIDES[localeName];
  if (override) { return override; }
  return { nav: { ...NAV_KEYS }, pages: JSON.parse(JSON.stringify(PAGE_SECTIONS)) };
}

function processFile(localeName) {
  const fp = join(LOCALES_DIR, `${localeName}.json`);
  const data = JSON.parse(readFileSync(fp, "utf8"));
  const override = applyOverrides(localeName);

  // 1. Add nav keys (after stockAnalysis in the nav object, if present; else at end)
  if (!data.nav) { data.nav = {}; }
  for (const [k, v] of Object.entries(override.nav)) {
    if (!(k in data.nav)) {
      data.nav[k] = v;
    }
  }
  // Re-order nav to keep stable: existing keys first, new ones appended (we don't reorder)

  // 2. Add 5 page sections at top-level (only if not already present)
  for (const [section, kv] of Object.entries(override.pages)) {
    if (!data[section]) { data[section] = {}; }
    for (const [k, v] of Object.entries(kv)) {
      if (!(k in data[section])) {
        data[section][k] = v;
      }
    }
  }

  writeFileSync(fp, JSON.stringify(data, null, 2) + "\n", "utf8");
  const added = Object.keys(override.nav).filter((k) => !(k in data.nav) || false).length;
  console.log(`✓ ${localeName}.json updated`);
}

const localeFiles = readdirSync(LOCALES_DIR).filter((f) => f.endsWith(".json"));
for (const f of localeFiles) {
  const name = f.replace(/\.json$/, "");
  processFile(name);
}

console.log(`\n✅ Processed ${localeFiles.length} locale files`);
