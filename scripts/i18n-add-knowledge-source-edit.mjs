// 追加知识源编辑相关 keys（editTitle/updateFailed/updateSuccess/save）到 10 种语言
// 用法: node scripts/i18n-add-knowledge-source-edit.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = "src/i18n/locales";
const files = ["ar.json", "de.json", "en-US.json", "es.json", "fr.json", "hi.json", "ja.json", "ko.json", "ru.json", "zh-TW.json"];

const extra = {
  "ar": { "editTitle": "تعديل المصدر", "updateFailed": "فشل التحديث", "updateSuccess": "تم الحفظ", "save": "حفظ" },
  "de": { "editTitle": "Quelle bearbeiten", "updateFailed": "Aktualisierung fehlgeschlagen", "updateSuccess": "Gespeichert", "save": "Speichern" },
  "en-US": { "editTitle": "Edit Source", "updateFailed": "Update failed", "updateSuccess": "Saved", "save": "Save" },
  "es": { "editTitle": "Editar fuente", "updateFailed": "Error al actualizar", "updateSuccess": "Guardado", "save": "Guardar" },
  "fr": { "editTitle": "Modifier la source", "updateFailed": "Échec de la mise à jour", "updateSuccess": "Enregistré", "save": "Enregistrer" },
  "hi": { "editTitle": "स्रोत संपादित करें", "updateFailed": "अद्यतन विफल", "updateSuccess": "सहेजा गया", "save": "सहेजें" },
  "ja": { "editTitle": "ソースを編集", "updateFailed": "更新に失敗", "updateSuccess": "保存しました", "save": "保存" },
  "ko": { "editTitle": "소스 편집", "updateFailed": "업데이트 실패", "updateSuccess": "저장됨", "save": "저장" },
  "ru": { "editTitle": "Изменить источник", "updateFailed": "Ошибка обновления", "updateSuccess": "Сохранено", "save": "Сохранить" },
  "zh-TW": { "editTitle": "編輯知識源", "updateFailed": "更新失敗", "updateSuccess": "已儲存", "save": "儲存" },
};

let ok = 0;
for (const f of files) {
  const lang = f.replace(/\.json$/, "");
  const path = join(dir, f);
  const raw = JSON.parse(readFileSync(path, "utf8"));
  const ks = raw.sourceManager?.knowledgeSource ?? {};
  let changed = false;
  for (const [k, v] of Object.entries(extra[lang])) {
    if (ks[k] === undefined) {
      ks[k] = v;
      changed = true;
    }
  }
  if (changed) {
    raw.sourceManager.knowledgeSource = ks;
    writeFileSync(path, JSON.stringify(raw, null, 2) + "\n", "utf8");
    ok++;
  }
}
console.log(`done: ${ok}/${files.length} files updated`);
