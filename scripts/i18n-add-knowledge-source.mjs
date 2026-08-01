// 批量给 10 种语言插入 sourceManager.tab.sources + sourceManager.knowledgeSource 段
// 用法: node scripts/i18n-add-knowledge-source.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = "src/i18n/locales";
const files = ["ar.json", "de.json", "en-US.json", "es.json", "fr.json", "hi.json", "ja.json", "ko.json", "ru.json", "zh-TW.json"];

const translations = {
  "tab.sources": {
    "ar": "مصادر المعرفة",
    "de": "Wissensquellen",
    "en-US": "Sources",
    "es": "Fuentes",
    "fr": "Sources",
    "hi": "ज्ञान स्रोत",
    "ja": "ナレッジソース",
    "ko": "지식 소스",
    "ru": "Источники",
    "zh-TW": "知識源",
  },
  "knowledgeSource": {
    "ar": {
      "actions": "إجراءات", "configure": "تكوين", "createFailed": "فشل إنشاء مصدر المعرفة", "createSubmit": "إنشاء",
      "createSuccess": "تم إنشاء مصدر المعرفة", "createTitle": "إضافة مصدر معرفة", "created": "تم إنشاء صفحة",
      "deleteConfirm": "هل تريد حذف مصدر المعرفة؟", "empty": "لا توجد مصادر، أدخل URL للجلب أو انقر لإضافة",
      "fetchAll": "جلب الكل", "fetchAllDone": "اكتمل الجلب الدفعي ({{total}} مصادر)", "fetchAllPartial": "اكتمل الجلب: {{total}} مصادر، {{errors}} فشلت",
      "fetchFailed": "فشل الجلب", "fetchNow": "جلب الآن", "fetchToWiki": "جلب إلى الويكي", "lastFetched": "آخر جلب",
      "pathRequired": "أدخل URL أو مسارًا", "schedule": "جدولة", "scheduleHint": "تعبير cron من 5 حقول، مثل 0 3 * * *",
      "skipped": "المحتوى لم يتغير، تم التخطي", "sourcePath": "URL / مسار", "sourceType": "النوع",
      "status": "الحالة", "statusActive": "مفعل", "statusPaused": "متوقف", "title": "العنوان",
      "titlePlaceholder": "اختياري: عنوان الصفحة", "titleRequired": "أدخل العنوان", "updated": "تم تحديث الصفحة",
      "urlInvalid": "صيغة URL غير صالحة", "urlPlaceholder": "أدخل URL لجلب صفحة الويكي", "urlRequired": "أدخل URL",
    },
    "de": {
      "actions": "Aktionen", "configure": "Konfigurieren", "createFailed": "Quelle konnte nicht erstellt werden", "createSubmit": "Erstellen",
      "createSuccess": "Wissensquelle erstellt", "createTitle": "Wissensquelle hinzufügen", "created": "Seite erstellt",
      "deleteConfirm": "Wissensquelle wirklich löschen?", "empty": "Keine Quellen. URL eingeben oder neu anlegen",
      "fetchAll": "Alle abrufen", "fetchAllDone": "Batch-Abruf abgeschlossen ({{total}} Quellen)", "fetchAllPartial": "Abruf abgeschlossen: {{total}} Quellen, {{errors}} fehlgeschlagen",
      "fetchFailed": "Abruf fehlgeschlagen", "fetchNow": "Jetzt abrufen", "fetchToWiki": "In Wiki übernehmen", "lastFetched": "Zuletzt abgerufen",
      "pathRequired": "URL oder Pfad eingeben", "schedule": "Zeitplan", "scheduleHint": "5-Felder-Cron, z. B. 0 3 * * *",
      "skipped": "Inhalt unverändert, übersprungen", "sourcePath": "URL / Pfad", "sourceType": "Typ",
      "status": "Status", "statusActive": "Aktiv", "statusPaused": "Pausiert", "title": "Titel",
      "titlePlaceholder": "Optional: Seitentitel", "titleRequired": "Titel eingeben", "updated": "Seite aktualisiert",
      "urlInvalid": "Ungültige URL", "urlPlaceholder": "URL eingeben, um als Wiki-Seite zu übernehmen", "urlRequired": "URL eingeben",
    },
    "en-US": {
      "actions": "Actions", "configure": "Configure", "createFailed": "Failed to create source", "createSubmit": "Create",
      "createSuccess": "Knowledge source created", "createTitle": "Add Knowledge Source", "created": "Page created",
      "deleteConfirm": "Delete this knowledge source?", "empty": "No sources yet. Enter a URL or add one",
      "fetchAll": "Fetch All", "fetchAllDone": "Batch fetch completed ({{total}} sources)", "fetchAllPartial": "Fetch done: {{total}} sources, {{errors}} failed",
      "fetchFailed": "Fetch failed", "fetchNow": "Fetch now", "fetchToWiki": "Fetch to Wiki", "lastFetched": "Last fetched",
      "pathRequired": "Enter URL or path", "schedule": "Schedule", "scheduleHint": "5-field cron, e.g. 0 3 * * *",
      "skipped": "Content unchanged, skipped", "sourcePath": "URL / Path", "sourceType": "Type",
      "status": "Status", "statusActive": "Active", "statusPaused": "Paused", "title": "Title",
      "titlePlaceholder": "Optional: page title", "titleRequired": "Enter title", "updated": "Page updated",
      "urlInvalid": "Invalid URL", "urlPlaceholder": "Enter a web URL to fetch as a Wiki page", "urlRequired": "Enter URL",
    },
    "es": {
      "actions": "Acciones", "configure": "Configurar", "createFailed": "No se pudo crear la fuente", "createSubmit": "Crear",
      "createSuccess": "Fuente de conocimiento creada", "createTitle": "Agregar fuente de conocimiento", "created": "Página creada",
      "deleteConfirm": "¿Eliminar esta fuente?", "empty": "Sin fuentes. Ingresa una URL o agrega una",
      "fetchAll": "Obtener todo", "fetchAllDone": "Obtención completada ({{total}} fuentes)", "fetchAllPartial": "Listo: {{total}} fuentes, {{errors}} fallaron",
      "fetchFailed": "Error al obtener", "fetchNow": "Obtener ahora", "fetchToWiki": "Obtener a Wiki", "lastFetched": "Última obtención",
      "pathRequired": "Ingresa URL o ruta", "schedule": "Programación", "scheduleHint": "Cron de 5 campos, ej. 0 3 * * *",
      "skipped": "Contenido sin cambios, omitido", "sourcePath": "URL / Ruta", "sourceType": "Tipo",
      "status": "Estado", "statusActive": "Activo", "statusPaused": "En pausa", "title": "Título",
      "titlePlaceholder": "Opcional: título de página", "titleRequired": "Ingresa título", "updated": "Página actualizada",
      "urlInvalid": "URL inválida", "urlPlaceholder": "Ingresa una URL para convertirla en página Wiki", "urlRequired": "Ingresa URL",
    },
    "fr": {
      "actions": "Actions", "configure": "Configurer", "createFailed": "Échec de création de la source", "createSubmit": "Créer",
      "createSuccess": "Source de connaissances créée", "createTitle": "Ajouter une source", "created": "Page créée",
      "deleteConfirm": "Supprimer cette source ?", "empty": "Aucune source. Saisissez une URL ou ajoutez-en une",
      "fetchAll": "Tout récupérer", "fetchAllDone": "Récupération terminée ({{total}} sources)", "fetchAllPartial": "Terminé : {{total}} sources, {{errors}} échecs",
      "fetchFailed": "Échec de la récupération", "fetchNow": "Récupérer", "fetchToWiki": "Importer dans Wiki", "lastFetched": "Dernière récupération",
      "pathRequired": "Saisissez une URL ou un chemin", "schedule": "Planification", "scheduleHint": "Cron 5 champs, ex. 0 3 * * *",
      "skipped": "Contenu inchangé, ignoré", "sourcePath": "URL / Chemin", "sourceType": "Type",
      "status": "Statut", "statusActive": "Actif", "statusPaused": "En pause", "title": "Titre",
      "titlePlaceholder": "Optionnel : titre de page", "titleRequired": "Saisissez un titre", "updated": "Page mise à jour",
      "urlInvalid": "URL invalide", "urlPlaceholder": "Saisissez une URL à importer comme page Wiki", "urlRequired": "Saisissez une URL",
    },
    "hi": {
      "actions": "कार्रवाइयाँ", "configure": "कॉन्फ़िगर करें", "createFailed": "स्रोत बनाने में विफल", "createSubmit": "बनाएँ",
      "createSuccess": "ज्ञान स्रोत बनाया गया", "createTitle": "ज्ञान स्रोत जोड़ें", "created": "पृष्ठ बनाया गया",
      "deleteConfirm": "इस स्रोत को हटाएँ?", "empty": "कोई स्रोत नहीं। URL दर्ज करें या जोड़ें",
      "fetchAll": "सभी लाएँ", "fetchAllDone": "बैच पूर्ण ({{total}} स्रोत)", "fetchAllPartial": "पूर्ण: {{total}} स्रोत, {{errors}} विफल",
      "fetchFailed": "लाने में विफल", "fetchNow": "अभी लाएँ", "fetchToWiki": "Wiki में लाएँ", "lastFetched": "अंतिम बार",
      "pathRequired": "URL या पथ दर्ज करें", "schedule": "अनुसूची", "scheduleHint": "5-फ़ील्ड cron, जैसे 0 3 * * *",
      "skipped": "सामग्री अपरिवर्तित, छोड़ा गया", "sourcePath": "URL / पथ", "sourceType": "प्रकार",
      "status": "स्थिति", "statusActive": "सक्रिय", "statusPaused": "रोका गया", "title": "शीर्षक",
      "titlePlaceholder": "वैकल्पिक: पृष्ठ शीर्षक", "titleRequired": "शीर्षक दर्ज करें", "updated": "पृष्ठ अद्यतन",
      "urlInvalid": "अमान्य URL", "urlPlaceholder": "Wiki पृष्ठ बनाने हेतु URL दर्ज करें", "urlRequired": "URL दर्ज करें",
    },
    "ja": {
      "actions": "操作", "configure": "設定", "createFailed": "ソースの作成に失敗", "createSubmit": "作成",
      "createSuccess": "ナレッジソースを作成しました", "createTitle": "ナレッジソースを追加", "created": "ページを作成しました",
      "deleteConfirm": "このソースを削除しますか？", "empty": "ソースがありません。URLを入力するか追加してください",
      "fetchAll": "すべて取得", "fetchAllDone": "一括取得完了（{{total}} ソース）", "fetchAllPartial": "取得完了: {{total}} ソース、{{errors}} 失敗",
      "fetchFailed": "取得に失敗", "fetchNow": "今すぐ取得", "fetchToWiki": "Wikiに取り込む", "lastFetched": "最終取得",
      "pathRequired": "URLまたはパスを入力", "schedule": "スケジュール", "scheduleHint": "5フィールドcron、例: 0 3 * * *",
      "skipped": "内容に変化なし、スキップ", "sourcePath": "URL / パス", "sourceType": "タイプ",
      "status": "状態", "statusActive": "有効", "statusPaused": "一時停止", "title": "タイトル",
      "titlePlaceholder": "任意: ページタイトル", "titleRequired": "タイトルを入力", "updated": "ページを更新しました",
      "urlInvalid": "URLが無効です", "urlPlaceholder": "Wikiページ化するURLを入力", "urlRequired": "URLを入力",
    },
    "ko": {
      "actions": "작업", "configure": "설정", "createFailed": "소스 생성 실패", "createSubmit": "만들기",
      "createSuccess": "지식 소스 생성됨", "createTitle": "지식 소스 추가", "created": "페이지 생성됨",
      "deleteConfirm": "이 소스를 삭제할까요?", "empty": "소스가 없습니다. URL을 입력하거나 추가하세요",
      "fetchAll": "모두 가져오기", "fetchAllDone": "일괄 가져오기 완료 ({{total}}개 소스)", "fetchAllPartial": "완료: {{total}}개, {{errors}}개 실패",
      "fetchFailed": "가져오기 실패", "fetchNow": "지금 가져오기", "fetchToWiki": "Wiki로 가져오기", "lastFetched": "마지막 가져오기",
      "pathRequired": "URL 또는 경로 입력", "schedule": "일정", "scheduleHint": "5필드 cron, 예: 0 3 * * *",
      "skipped": "내용 변경 없음, 건너뜀", "sourcePath": "URL / 경로", "sourceType": "유형",
      "status": "상태", "statusActive": "활성", "statusPaused": "일시 중지", "title": "제목",
      "titlePlaceholder": "선택: 페이지 제목", "titleRequired": "제목 입력", "updated": "페이지 업데이트됨",
      "urlInvalid": "잘못된 URL", "urlPlaceholder": "Wiki 페이지로 만들 URL 입력", "urlRequired": "URL 입력",
    },
    "ru": {
      "actions": "Действия", "configure": "Настроить", "createFailed": "Не удалось создать источник", "createSubmit": "Создать",
      "createSuccess": "Источник знаний создан", "createTitle": "Добавить источник знаний", "created": "Страница создана",
      "deleteConfirm": "Удалить этот источник?", "empty": "Нет источников. Введите URL или добавьте",
      "fetchAll": "Получить все", "fetchAllDone": "Пакетное получение завершено ({{total}} источников)", "fetchAllPartial": "Готово: {{total}} источников, {{errors}} ошибок",
      "fetchFailed": "Ошибка получения", "fetchNow": "Получить сейчас", "fetchToWiki": "В Wiki", "lastFetched": "Последнее получение",
      "pathRequired": "Введите URL или путь", "schedule": "Расписание", "scheduleHint": "Cron из 5 полей, напр. 0 3 * * *",
      "skipped": "Содержимое не изменилось, пропущено", "sourcePath": "URL / Путь", "sourceType": "Тип",
      "status": "Статус", "statusActive": "Активен", "statusPaused": "Приостановлен", "title": "Заголовок",
      "titlePlaceholder": "Необязательно: заголовок страницы", "titleRequired": "Введите заголовок", "updated": "Страница обновлена",
      "urlInvalid": "Некорректный URL", "urlPlaceholder": "Введите URL для загрузки как страницу Wiki", "urlRequired": "Введите URL",
    },
    "zh-TW": {
      "actions": "操作", "configure": "設定", "createFailed": "建立知識源失敗", "createSubmit": "建立",
      "createSuccess": "知識源已建立", "createTitle": "新增知識源", "created": "已新建頁面",
      "deleteConfirm": "確定刪除該知識源？", "empty": "暫無知識源，輸入 URL 抓取或點擊新增",
      "fetchAll": "全部抓取", "fetchAllDone": "批次抓取完成（{{total}} 個源）", "fetchAllPartial": "批次抓取完成：{{total}} 個源，{{errors}} 個失敗",
      "fetchFailed": "抓取失敗", "fetchNow": "立即抓取", "fetchToWiki": "抓取到 Wiki", "lastFetched": "上次抓取",
      "pathRequired": "請輸入 URL 或路徑", "schedule": "排程", "scheduleHint": "5 欄位 cron，如 0 3 * * *",
      "skipped": "內容未變化，已跳過", "sourcePath": "URL / 路徑", "sourceType": "類型",
      "status": "狀態", "statusActive": "啟用", "statusPaused": "暫停", "title": "標題",
      "titlePlaceholder": "可選：頁面標題", "titleRequired": "請輸入標題", "updated": "已更新頁面",
      "urlInvalid": "URL 格式無效", "urlPlaceholder": "輸入網頁 URL 抓取為 Wiki 頁面", "urlRequired": "請輸入 URL",
    },
  },
};

let ok = 0;
for (const f of files) {
  const path = join(dir, f);
  const raw = JSON.parse(readFileSync(path, "utf8"));
  const sm = raw.sourceManager ?? {};
  if (!sm.tab) sm.tab = {};
  if (sm.tab.sources === undefined) sm.tab.sources = translations["tab.sources"][f];
  if (!sm.knowledgeSource) sm.knowledgeSource = translations.knowledgeSource[f];
  writeFileSync(path, JSON.stringify(raw, null, 2) + "\n", "utf8");
  ok++;
}
console.log(`done: ${ok}/${files.length} files`);
