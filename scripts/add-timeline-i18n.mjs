// scripts/add-timeline-i18n.mjs
// 一次性为 11 个 locale 添加 stockAnalysis.timeline.* 翻译
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCALES = join(__dirname, "..", "src", "i18n", "locales");

const TRANSLATIONS = {
  "zh-CN": {
    "phase.scan": "扫描",
    "phase.diagnose": "诊断",
    "phase.debate": "辩论",
    "phase.decide": "决策",
    "empty": "该阶段暂无节点",
    "idleHint": "开始分析后,这里会展示完整的决策时间线",
    "emptyHint": "等待工作流推送节点…",
    "sendToChat": "发送到对话",
  },
  "zh-TW": {
    "phase.scan": "掃描",
    "phase.diagnose": "診斷",
    "phase.debate": "辯論",
    "phase.decide": "決策",
    "empty": "該階段暫無節點",
    "idleHint": "開始分析後,這裡會展示完整的決策時間線",
    "emptyHint": "等待工作流推送節點…",
    "sendToChat": "發送到對話",
  },
  "en-US": {
    "phase.scan": "Scan",
    "phase.diagnose": "Diagnose",
    "phase.debate": "Debate",
    "phase.decide": "Decide",
    "empty": "No nodes in this phase",
    "idleHint": "Once you start an analysis, the full decision timeline will appear here",
    "emptyHint": "Waiting for workflow to push nodes…",
    "sendToChat": "Send to chat",
  },
  "ja": {
    "phase.scan": "スキャン",
    "phase.diagnose": "診断",
    "phase.debate": "討論",
    "phase.decide": "決定",
    "empty": "このフェーズにノードがありません",
    "idleHint": "分析を開始すると、完全な決定タイムラインがここに表示されます",
    "emptyHint": "ワークフローのノードを待機中…",
    "sendToChat": "チャットに送信",
  },
  "ko": {
    "phase.scan": "스캔",
    "phase.diagnose": "진단",
    "phase.debate": "토론",
    "phase.decide": "결정",
    "empty": "이 단계에 노드 없음",
    "idleHint": "분석을 시작하면 전체 결정 타임라인이 여기에 표시됩니다",
    "emptyHint": "워크플로 노드 대기 중…",
    "sendToChat": "채팅으로 보내기",
  },
  "de": {
    "phase.scan": "Scannen",
    "phase.diagnose": "Diagnose",
    "phase.debate": "Debatte",
    "phase.decide": "Entscheidung",
    "empty": "Keine Knoten in dieser Phase",
    "idleHint": "Sobald Sie eine Analyse starten, erscheint hier die vollständige Entscheidungszeitleiste",
    "emptyHint": "Warte auf Workflow-Knoten…",
    "sendToChat": "An Chat senden",
  },
  "fr": {
    "phase.scan": "Analyse",
    "phase.diagnose": "Diagnostic",
    "phase.debate": "Débat",
    "phase.decide": "Décision",
    "empty": "Aucun nœud dans cette phase",
    "idleHint": "Une fois l'analyse lancée, la chronologie complète apparaîtra ici",
    "emptyHint": "En attente des nœuds du flux…",
    "sendToChat": "Envoyer au chat",
  },
  "es": {
    "phase.scan": "Escaneo",
    "phase.diagnose": "Diagnóstico",
    "phase.debate": "Debate",
    "phase.decide": "Decisión",
    "empty": "Sin nodos en esta fase",
    "idleHint": "Al iniciar un análisis, la línea de tiempo completa aparecerá aquí",
    "emptyHint": "Esperando nodos del flujo…",
    "sendToChat": "Enviar al chat",
  },
  "ru": {
    "phase.scan": "Сканирование",
    "phase.diagnose": "Диагностика",
    "phase.debate": "Дебаты",
    "phase.decide": "Решение",
    "empty": "В этой фазе нет узлов",
    "idleHint": "После запуска анализа здесь появится полная временная шкала решений",
    "emptyHint": "Ожидание узлов рабочего процесса…",
    "sendToChat": "Отправить в чат",
  },
  "hi": {
    "phase.scan": "स्कैन",
    "phase.diagnose": "निदान",
    "phase.debate": "बहस",
    "phase.decide": "निर्णय",
    "empty": "इस चरण में कोई नोड नहीं",
    "idleHint": "विश्लेषण शुरू करने के बाद पूर्ण निर्णय टाइमलाइन यहाँ दिखेगी",
    "emptyHint": "वर्कफ़्लो नोड्स की प्रतीक्षा…",
    "sendToChat": "चैट पर भेजें",
  },
  "ar": {
    "phase.scan": "مسح",
    "phase.diagnose": "تشخيص",
    "phase.debate": "نقاش",
    "phase.decide": "قرار",
    "empty": "لا توجد عقد في هذه المرحلة",
    "idleHint": "بعد بدء التحليل، سيظهر الجدول الزمني الكامل هنا",
    "emptyHint": "في انتظار عقد سير العمل…",
    "sendToChat": "إرسال إلى الدردشة",
  },
};

let count = 0;
for (const [locale, keys] of Object.entries(TRANSLATIONS)) {
  const file = join(LOCALES, `${locale}.json`);
  const json = JSON.parse(readFileSync(file, "utf8"));
  if (!json.stockAnalysis) {
    console.warn(`[skip] ${locale}.json: no stockAnalysis block`);
    continue;
  }
  if (!json.stockAnalysis.timeline) { json.stockAnalysis.timeline = {}; }
  for (const [k, v] of Object.entries(keys)) {
    json.stockAnalysis.timeline[k] = v;
  }
  writeFileSync(file, JSON.stringify(json, null, 2) + "\n", "utf8");
  count++;
  console.log(`[ok] ${locale}.json: added ${Object.keys(keys).length} timeline keys`);
}
console.log(`\nDone: ${count} locales updated`);
