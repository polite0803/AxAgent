// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import enUS from "./locales/en-US.json";
import zhCN from "./locales/zh-CN.json";

// 仅同步 bundle 默认语言（zh-CN）和回退语言（en-US），
// 其余 9 种语言按需动态 import，减少首屏 JS 解析与传输体积。
const LAZY_LOCALES: Record<string, () => Promise<{ default: Record<string, unknown> }>> = {
  "zh-TW": () => import("./locales/zh-TW.json"),
  ja: () => import("./locales/ja.json"),
  ko: () => import("./locales/ko.json"),
  fr: () => import("./locales/fr.json"),
  de: () => import("./locales/de.json"),
  es: () => import("./locales/es.json"),
  ru: () => import("./locales/ru.json"),
  hi: () => import("./locales/hi.json"),
  ar: () => import("./locales/ar.json"),
};

i18n.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: zhCN },
    "en-US": { translation: enUS },
  },
  lng: "zh-CN",
  fallbackLng: "en-US",
  interpolation: { escapeValue: false },
  // 允许 resources 中只包含部分语言，其余语言运行时动态加载
  partialBundledLanguages: true,
});

// 拦截 changeLanguage：切换前先加载对应 locale 资源，避免切换瞬间显示未翻译 key。
// 所有调用方（AppInitializer / App.tsx / TitleBar / GeneralSettings）无需改动。
const originalChangeLanguage = i18n.changeLanguage.bind(i18n);
i18n.changeLanguage = (async (lng?: string) => {
  if (lng && !i18n.hasResourceBundle(lng, "translation") && lng in LAZY_LOCALES) {
    try {
      const mod = await LAZY_LOCALES[lng]();
      i18n.addResourceBundle(lng, "translation", mod.default, true, true);
    } catch (e) {
      console.warn(`[i18n] Failed to load locale "${lng}":`, e);
    }
  }
  return originalChangeLanguage(lng);
}) as typeof i18n.changeLanguage;

export default i18n;
