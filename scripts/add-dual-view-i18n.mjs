// scripts/add-dual-view-i18n.mjs
// 一次性为 11 个 locale 添加 dualView.* 翻译
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCALES = join(__dirname, "..", "src", "i18n", "locales");

const TRANSLATIONS = {
  "zh-CN": {
    "notRegistered": "未注册的 dual view: {{id}}",
    "expand": "展开为「{{title}}」",
    "expandToPanel": "展开为面板",
    "collapse": "折叠",
    "collapseToBubble": "折叠为气泡",
  },
  "zh-TW": {
    "notRegistered": "未註冊的 dual view: {{id}}",
    "expand": "展開為「{{title}}」",
    "expandToPanel": "展開為面板",
    "collapse": "折疊",
    "collapseToBubble": "折疊為氣泡",
  },
  "en-US": {
    "notRegistered": "Unregistered dual view: {{id}}",
    "expand": "Expand to {{title}}",
    "expandToPanel": "Expand to panel",
    "collapse": "Collapse",
    "collapseToBubble": "Collapse to bubble",
  },
  "ja": {
    "notRegistered": "未登録の dual view: {{id}}",
    "expand": "「{{title}}」に展開",
    "expandToPanel": "パネルに展開",
    "collapse": "折りたたみ",
    "collapseToBubble": "バブルに折りたたむ",
  },
  "ko": {
    "notRegistered": "등록되지 않은 dual view: {{id}}",
    "expand": "{{title}}로 펼치기",
    "expandToPanel": "패널로 펼치기",
    "collapse": "접기",
    "collapseToBubble": "버블로 접기",
  },
  "de": {
    "notRegistered": "Nicht registrierte Dual-View: {{id}}",
    "expand": "Zu {{title}} erweitern",
    "expandToPanel": "Zum Panel erweitern",
    "collapse": "Einklappen",
    "collapseToBubble": "Zur Blase einklappen",
  },
  "fr": {
    "notRegistered": "Vue double non enregistrée : {{id}}",
    "expand": "Étendre en {{title}}",
    "expandToPanel": "Étendre en panneau",
    "collapse": "Réduire",
    "collapseToBubble": "Réduire en bulle",
  },
  "es": {
    "notRegistered": "Vista dual no registrada: {{id}}",
    "expand": "Expandir a {{title}}",
    "expandToPanel": "Expandir a panel",
    "collapse": "Contraer",
    "collapseToBubble": "Contraer a burbuja",
  },
  "ru": {
    "notRegistered": "Незарегистрированное dual view: {{id}}",
    "expand": "Развернуть в {{title}}",
    "expandToPanel": "Развернуть в панель",
    "collapse": "Свернуть",
    "collapseToBubble": "Свернуть в пузырь",
  },
  "hi": {
    "notRegistered": "अपंजीकृत dual view: {{id}}",
    "expand": "{{title}} में विस्तृत करें",
    "expandToPanel": "पैनल में विस्तृत करें",
    "collapse": "संकुचित करें",
    "collapseToBubble": "बुलबुले में संकुचित करें",
  },
  "ar": {
    "notRegistered": "عرض مزدوج غير مسجل: {{id}}",
    "expand": "توسيع إلى {{title}}",
    "expandToPanel": "توسيع إلى لوحة",
    "collapse": "طي",
    "collapseToBubble": "طي إلى فقاعة",
  },
};

for (const [locale, keys] of Object.entries(TRANSLATIONS)) {
  const file = join(LOCALES, `${locale}.json`);
  const json = JSON.parse(readFileSync(file, "utf8"));
  if (!json.dualView) { json.dualView = {}; }
  for (const [k, v] of Object.entries(keys)) {
    json.dualView[k] = v;
  }
  writeFileSync(file, JSON.stringify(json, null, 2) + "\n", "utf8");
  console.log(`[ok] ${locale}.json: added ${Object.keys(keys).length} dualView keys`);
}
