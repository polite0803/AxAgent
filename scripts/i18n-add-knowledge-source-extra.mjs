// 追加 knowledgeSource 扩展 keys（GitHub 导入 / sitemap / 定时刷新）到 10 种语言
// 用法: node scripts/i18n-add-knowledge-source-extra.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = "src/i18n/locales";
const files = ["ar.json", "de.json", "en-US.json", "es.json", "fr.json", "hi.json", "ja.json", "ko.json", "ru.json", "zh-TW.json"];

const extra = {
  "ar": {
    "cronRequired": "أدخل تعبير cron", "githubImport": "استيراد GitHub", "githubImportTitle": "استيراد قاعدة معرفة مفتوحة",
    "githubPath": "دليل المستندات", "githubPathHint": "الدليل المراد استيراده داخل المستودع، الافتراضي docs",
    "githubRepo": "عنوان المستودع", "githubRepoRequired": "أدخل owner/repo", "importFailed": "فشل الاستيراد",
    "importSubmit": "استيراد", "scheduleFailed": "فشل تسجيل الجدولة", "scheduleSuccess": "تم تسجيل الجدولة ({{cron}})",
    "scheduleSync": "تسجيل الجدولة", "sitemapDone": "تم إنشاء {{count}} مصادر من sitemap", "sitemapFailed": "فشل جلب sitemap",
    "sitemapImport": "دفعة الموقع", "sitemapSubmit": "جلب", "sitemapTitle": "جلب دفعة من sitemap.xml",
    "sitemapUrl": "عنوان الموقع", "sitemapUrlRequired": "أدخل عنوان الموقع",
  },
  "de": {
    "cronRequired": "Cron-Ausdruck eingeben", "githubImport": "GitHub-Import", "githubImportTitle": "Open-Source-Wissensbasis importieren",
    "githubPath": "Dokumentverzeichnis", "githubPathHint": "Zu importierendes Verzeichnis im Repo, Standard: docs",
    "githubRepo": "Repository", "githubRepoRequired": "owner/repo eingeben", "importFailed": "Import fehlgeschlagen",
    "importSubmit": "Importieren", "scheduleFailed": "Zeitplan-Registrierung fehlgeschlagen", "scheduleSuccess": "Zeitplan registriert ({{cron}})",
    "scheduleSync": "Zeitplan registrieren", "sitemapDone": "{{count}} Quellen aus sitemap erstellt", "sitemapFailed": "Sitemap-Abruf fehlgeschlagen",
    "sitemapImport": "Site-Batch", "sitemapSubmit": "Abrufen", "sitemapTitle": "Batch-Abruf aus sitemap.xml",
    "sitemapUrl": "Site-URL", "sitemapUrlRequired": "Site-URL eingeben",
  },
  "en-US": {
    "cronRequired": "Enter cron expression", "githubImport": "GitHub Import", "githubImportTitle": "Import Open-Source Knowledge Base",
    "githubPath": "Docs directory", "githubPathHint": "Directory to import inside the repo, default: docs",
    "githubRepo": "Repository", "githubRepoRequired": "Enter owner/repo", "importFailed": "Import failed",
    "importSubmit": "Import", "scheduleFailed": "Failed to register schedule", "scheduleSuccess": "Schedule registered ({{cron}})",
    "scheduleSync": "Register schedule", "sitemapDone": "Created {{count}} sources from sitemap", "sitemapFailed": "Sitemap fetch failed",
    "sitemapImport": "Site batch", "sitemapSubmit": "Fetch", "sitemapTitle": "Batch fetch from sitemap.xml",
    "sitemapUrl": "Site URL", "sitemapUrlRequired": "Enter site URL",
  },
  "es": {
    "cronRequired": "Ingresa expresión cron", "githubImport": "Importar GitHub", "githubImportTitle": "Importar base de conocimiento de código abierto",
    "githubPath": "Directorio de docs", "githubPathHint": "Directorio a importar dentro del repo, por defecto docs",
    "githubRepo": "Repositorio", "githubRepoRequired": "Ingresa owner/repo", "importFailed": "Error al importar",
    "importSubmit": "Importar", "scheduleFailed": "Error al registrar horario", "scheduleSuccess": "Horario registrado ({{cron}})",
    "scheduleSync": "Registrar horario", "sitemapDone": "Creadas {{count}} fuentes desde sitemap", "sitemapFailed": "Error al obtener sitemap",
    "sitemapImport": "Lote de sitio", "sitemapSubmit": "Obtener", "sitemapTitle": "Obtener lote desde sitemap.xml",
    "sitemapUrl": "URL del sitio", "sitemapUrlRequired": "Ingresa URL del sitio",
  },
  "fr": {
    "cronRequired": "Saisissez une expression cron", "githubImport": "Import GitHub", "githubImportTitle": "Importer une base de connaissances open source",
    "githubPath": "Dossier docs", "githubPathHint": "Dossier à importer dans le dépôt, défaut : docs",
    "githubRepo": "Dépôt", "githubRepoRequired": "Saisissez owner/repo", "importFailed": "Échec de l'import",
    "importSubmit": "Importer", "scheduleFailed": "Échec d'enregistrement du planning", "scheduleSuccess": "Planning enregistré ({{cron}})",
    "scheduleSync": "Enregistrer le planning", "sitemapDone": "{{count}} sources créées depuis le sitemap", "sitemapFailed": "Échec du sitemap",
    "sitemapImport": "Lot du site", "sitemapSubmit": "Récupérer", "sitemapTitle": "Récupération par lot depuis sitemap.xml",
    "sitemapUrl": "URL du site", "sitemapUrlRequired": "Saisissez l'URL du site",
  },
  "hi": {
    "cronRequired": "cron अभिव्यक्ति दर्ज करें", "githubImport": "GitHub आयात", "githubImportTitle": "ओपन-सोर्स ज्ञान आधार आयात करें",
    "githubPath": "दस्तावेज़ निर्देशिका", "githubPathHint": "रिपो में आयात करने हेतु निर्देशिका, डिफ़ॉल्ट docs",
    "githubRepo": "रिपॉजिटरी", "githubRepoRequired": "owner/repo दर्ज करें", "importFailed": "आयात विफल",
    "importSubmit": "आयात", "scheduleFailed": "अनुसूची पंजीकरण विफल", "scheduleSuccess": "अनुसूची पंजीकृत ({{cron}})",
    "scheduleSync": "अनुसूची पंजीकृत करें", "sitemapDone": "sitemap से {{count}} स्रोत बनाए गए", "sitemapFailed": "sitemap लाने में विफल",
    "sitemapImport": "साइट बैच", "sitemapSubmit": "लाएँ", "sitemapTitle": "sitemap.xml से बैच लाएँ",
    "sitemapUrl": "साइट URL", "sitemapUrlRequired": "साइट URL दर्ज करें",
  },
  "ja": {
    "cronRequired": "cron式を入力", "githubImport": "GitHubインポート", "githubImportTitle": "オープンソース知識ベースをインポート",
    "githubPath": "ドキュメントディレクトリ", "githubPathHint": "リポジトリ内のインポート対象ディレクトリ（既定: docs）",
    "githubRepo": "リポジトリ", "githubRepoRequired": "owner/repo を入力", "importFailed": "インポート失敗",
    "importSubmit": "インポート", "scheduleFailed": "スケジュール登録に失敗", "scheduleSuccess": "スケジュールを登録しました（{{cron}}）",
    "scheduleSync": "スケジュール登録", "sitemapDone": "sitemapから{{count}}ソースを作成", "sitemapFailed": "sitemap取得失敗",
    "sitemapImport": "サイト一括", "sitemapSubmit": "取得", "sitemapTitle": "sitemap.xmlから一括取得",
    "sitemapUrl": "サイトURL", "sitemapUrlRequired": "サイトURLを入力",
  },
  "ko": {
    "cronRequired": "cron 표현식을 입력하세요", "githubImport": "GitHub 가져오기", "githubImportTitle": "오픈소스 지식 베이스 가져오기",
    "githubPath": "문서 디렉터리", "githubPathHint": "저장소 내 가져올 디렉터리, 기본 docs",
    "githubRepo": "저장소", "githubRepoRequired": "owner/repo 입력", "importFailed": "가져오기 실패",
    "importSubmit": "가져오기", "scheduleFailed": "일정 등록 실패", "scheduleSuccess": "일정 등록됨 ({{cron}})",
    "scheduleSync": "일정 등록", "sitemapDone": "sitemap에서 {{count}}개 소스 생성", "sitemapFailed": "sitemap 가져오기 실패",
    "sitemapImport": "사이트 일괄", "sitemapSubmit": "가져오기", "sitemapTitle": "sitemap.xml에서 일괄 가져오기",
    "sitemapUrl": "사이트 URL", "sitemapUrlRequired": "사이트 URL 입력",
  },
  "ru": {
    "cronRequired": "Введите cron-выражение", "githubImport": "Импорт GitHub", "githubImportTitle": "Импорт открытой базы знаний",
    "githubPath": "Каталог docs", "githubPathHint": "Каталог для импорта в репозитории, по умолчанию docs",
    "githubRepo": "Репозиторий", "githubRepoRequired": "Введите owner/repo", "importFailed": "Ошибка импорта",
    "importSubmit": "Импортировать", "scheduleFailed": "Не удалось зарегистрировать расписание", "scheduleSuccess": "Расписание зарегистрировано ({{cron}})",
    "scheduleSync": "Зарегистрировать расписание", "sitemapDone": "Создано {{count}} источников из sitemap", "sitemapFailed": "Ошибка получения sitemap",
    "sitemapImport": "Пакет сайта", "sitemapSubmit": "Получить", "sitemapTitle": "Пакетное получение из sitemap.xml",
    "sitemapUrl": "URL сайта", "sitemapUrlRequired": "Введите URL сайта",
  },
  "zh-TW": {
    "cronRequired": "請輸入 cron 表達式", "githubImport": "GitHub 匯入", "githubImportTitle": "匯入開源知識庫",
    "githubPath": "文件目錄", "githubPathHint": "倉庫內要匯入的目錄，預設 docs",
    "githubRepo": "倉庫位址", "githubRepoRequired": "請輸入 owner/repo", "importFailed": "匯入失敗",
    "importSubmit": "匯入", "scheduleFailed": "註冊定時刷新失敗", "scheduleSuccess": "定時刷新已註冊（{{cron}}）",
    "scheduleSync": "註冊定時刷新", "sitemapDone": "已從 sitemap 建立 {{count}} 個知識源", "sitemapFailed": "sitemap 抓取失敗",
    "sitemapImport": "站點批次", "sitemapSubmit": "抓取", "sitemapTitle": "從 sitemap.xml 批次抓取",
    "sitemapUrl": "站點位址", "sitemapUrlRequired": "請輸入站點位址",
  },
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
