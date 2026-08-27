// 一次性修复：为 11 种语言补全缺失的 timeTravel 段（含 pageAnchor.live/replay 等 21 个 key）
// 用法: node scripts/fix-time-travel-i18n.js
// 备份在 output/backup-2026-08-27/
"use strict";
const fs = require("fs");
const path = require("path");

const LOCALES_DIR = path.join(__dirname, "..", "src", "i18n", "locales");

// 各语言 timeTravel 段（key 结构以当前组件调用为准：pageAnchor/datePicker/degradedMarker/badge/tour/sweep）
const TRANSLATIONS = {
  "zh-CN": {
    pageAnchor: {
      live: "实时分析",
      replay: "历史回放",
      untilDate: "回放至 {{date}}",
    },
    datePicker: {
      hint: "请选择过去的交易日，今天及未来日期被禁用（封闭世界假设）。",
      placeholder: "选择回放日期",
      ok: "进入回放",
      cancel: "取消",
    },
    degradedMarker: {
      tooltip: "部分分析方法在回放模式下不可用",
      labelWithCount: "降级 {{n}} 项",
    },
    badge: {
      sweep: "批次回测",
      replay: "回放 {{date}}",
      replayTooltip: "此视图中的所有数据、提示词与结论都以 {{date}} 为锚点，不可用于实盘交易。",
    },
    tour: {
      title: "时间旅行模式已上线",
      close: "关闭",
      body: "把分析和荐股都锚定到过去的某一日，可以安全地回测策略、避免前视偏差。系统会拒绝未来日期，违反时间约束的输出会在时间线上高亮。",
      stepAnchor: "随时点击「实时分析」或「历史回放」胶囊进入回放模式。",
      gotIt: "知道了",
    },
    sweep: {
      total: "回测总数",
      accuracy: "准确率",
      alpha: "Alpha",
      sharpe: "Sharpe",
    },
  },
  "zh-TW": {
    pageAnchor: {
      live: "即時分析",
      replay: "歷史回放",
      untilDate: "回放至 {{date}}",
    },
    datePicker: {
      hint: "請選擇過去的交易日。今天及未來日期被禁用（封閉世界假設）。",
      placeholder: "選擇回放日期",
      ok: "進入回放",
      cancel: "取消",
    },
    degradedMarker: {
      tooltip: "回放模式下資金流向/融資融券/北向持倉等資料無歷史語意，已被跳過。",
      labelWithCount: "已降級 · {{n}} 項",
    },
    badge: {
      sweep: "批次回測",
      replay: "回放 {{date}}",
      replayTooltip: "此檢視中的所有資料、提示詞與結論都以 {{date}} 為錨點，不可用於實盤交易。",
    },
    tour: {
      title: "時間旅行模式已上線",
      close: "關閉",
      body: "把分析和荐股都錨定到過去的某一日，可以安全地回測策略、避免前視偏差。系統會拒絕未來日期，違反時間約束的輸出會在時間線上高亮。",
      stepAnchor: "隨時點擊「即時分析」或「歷史回放」膠囊進入回放模式。",
      gotIt: "知道了",
    },
    sweep: {
      total: "回測總數",
      accuracy: "準確率",
      alpha: "Alpha",
      sharpe: "Sharpe",
    },
  },
  "en-US": {
    pageAnchor: {
      live: "Live",
      replay: "Replay",
      untilDate: "As of {{date}}",
    },
    datePicker: {
      hint: "Select a past trading day. Today and future dates are disabled (closed-world assumption).",
      placeholder: "Select replay date",
      ok: "Enter Replay",
      cancel: "Cancel",
    },
    degradedMarker: {
      tooltip: "In replay mode, capital flows, margin data, northbound positions, etc. have no historical semantics and are skipped.",
      labelWithCount: "Degraded · {{n}} items",
    },
    badge: {
      sweep: "Batch Backtest",
      replay: "Replay {{date}}",
      replayTooltip: "All data, prompts and conclusions in this view are anchored at {{date}}. Not for live trading.",
    },
    tour: {
      title: "Time Travel Mode Available",
      close: "Close",
      body: "Anchor all analysis and recommendations to a past date to safely backtest strategies and avoid look-ahead bias. Future dates are rejected, and outputs violating the time constraint are highlighted on the timeline.",
      stepAnchor: 'Click the "Live" or "Replay" pill anytime to enter replay mode.',
      gotIt: "Got It",
    },
    sweep: {
      total: "Total Backtests",
      accuracy: "Accuracy",
      alpha: "Alpha",
      sharpe: "Sharpe",
    },
  },
  ja: {
    pageAnchor: {
      live: "リアルタイム分析",
      replay: "履歴リプレイ",
      untilDate: "{{date}} 時点",
    },
    datePicker: {
      hint: "過去の取引日を選択してください。今日と未来の日付は無効です（閉じた世界の仮定）。",
      placeholder: "リプレイ日を選択",
      ok: "リプレイ開始",
      cancel: "キャンセル",
    },
    degradedMarker: {
      tooltip: "リプレイモードでは資金フロー・信用取引・北向き保有などに履歴の意味がなく、スキップされます。",
      labelWithCount: "降格 · {{n}} 件",
    },
    badge: {
      sweep: "バッチバックテスト",
      replay: "リプレイ {{date}}",
      replayTooltip: "このビューのすべてのデータ・プロンプト・結論は {{date}} に固定されています。実取引には使用できません。",
    },
    tour: {
      title: "タイムトラベルモードが利用可能です",
      close: "閉じる",
      body: "分析と推奨を過去の日付に固定することで、戦略を安全にバックテストし、先読みバイアスを回避できます。未来の日付は拒否され、時間制約に違反する出力はタイムライン上で強調表示されます。",
      stepAnchor: "いつでも「リアルタイム分析」または「履歴リプレイ」をクリックしてリプレイモードに入ります。",
      gotIt: "了解",
    },
    sweep: {
      total: "バックテスト数",
      accuracy: "精度",
      alpha: "Alpha",
      sharpe: "シャープ",
    },
  },
  ko: {
    pageAnchor: {
      live: "실시간 분석",
      replay: "과거 재생",
      untilDate: "{{date}} 기준",
    },
    datePicker: {
      hint: "과거 거래일을 선택하세요. 오늘과 미래 날짜는 비활성화됩니다(폐쇄 세계 가정).",
      placeholder: "재생 날짜 선택",
      ok: "재생 시작",
      cancel: "취소",
    },
    degradedMarker: {
      tooltip: "재생 모드에서는 자금 흐름/신용 거래/북향 보유 등에 과거 의미가 없어 건너뜁니다.",
      labelWithCount: "저하 · {{n}}개",
    },
    badge: {
      sweep: "일괄 백테스트",
      replay: "재생 {{date}}",
      replayTooltip: "이 보기의 모든 데이터·프롬프트·결론은 {{date}}에 고정됩니다. 실거래에 사용할 수 없습니다.",
    },
    tour: {
      title: "타임트래블 모드 사용 가능",
      close: "닫기",
      body: "분석과 추천을 과거 날짜에 고정하면 전략을 안전하게 백테스트하고 선견 편향을 피할 수 있습니다. 미래 날짜는 거부되며 시간 제약을 위반한 출력은 타임라인에서 강조 표시됩니다.",
      stepAnchor: "언제든지 '실시간 분석' 또는 '과거 재생'을 클릭하여 재생 모드에 들어가세요.",
      gotIt: "확인",
    },
    sweep: {
      total: "백테스트 수",
      accuracy: "정확도",
      alpha: "Alpha",
      sharpe: "샤프",
    },
  },
  fr: {
    pageAnchor: {
      live: "Analyse en direct",
      replay: "Relecture",
      untilDate: "Au {{date}}",
    },
    datePicker: {
      hint: "Sélectionnez un jour de bourse passé. Aujourd'hui et les dates futures sont désactivées (hypothèse du monde clos).",
      placeholder: "Choisir la date de relecture",
      ok: "Entrer en relecture",
      cancel: "Annuler",
    },
    degradedMarker: {
      tooltip: "En mode relecture, les flux de capitaux, le crédit-marge, les positions nordbound, etc. n'ont pas de sens historique et sont ignorés.",
      labelWithCount: "Dégradé · {{n}} éléments",
    },
    badge: {
      sweep: "Backtest par lots",
      replay: "Relecture {{date}}",
      replayTooltip: "Toutes les données, invites et conclusions de cette vue sont ancrées au {{date}}. Ne pas utiliser pour le trading réel.",
    },
    tour: {
      title: "Mode voyage dans le temps disponible",
      close: "Fermer",
      body: "Ancrez toutes les analyses et recommandations à une date passée pour tester des stratégies en toute sécurité et éviter le biais de regard en avant. Les dates futures sont rejetées et les sorties violant la contrainte de temps sont mises en évidence sur la frise.",
      stepAnchor: "Cliquez à tout moment sur « Analyse en direct » ou « Relecture » pour entrer en mode relecture.",
      gotIt: "Compris",
    },
    sweep: {
      total: "Total backtests",
      accuracy: "Précision",
      alpha: "Alpha",
      sharpe: "Sharpe",
    },
  },
  de: {
    pageAnchor: {
      live: "Live-Analyse",
      replay: "Replay",
      untilDate: "Stand {{date}}",
    },
    datePicker: {
      hint: "Wählen Sie einen vergangenen Handelstag. Heute und zukünftige Daten sind deaktiviert (Closed-World-Annahme).",
      placeholder: "Replay-Datum wählen",
      ok: "Replay starten",
      cancel: "Abbrechen",
    },
    degradedMarker: {
      tooltip: "Im Replay-Modus haben Kapitalflüsse, Margin-Daten, Nordbound-Positionen usw. keine historische Bedeutung und werden übersprungen.",
      labelWithCount: "Degradiert · {{n}} Elemente",
    },
    badge: {
      sweep: "Batch-Backtest",
      replay: "Replay {{date}}",
      replayTooltip: "Alle Daten, Prompts und Schlussfolgerungen in dieser Ansicht sind auf {{date}} verankert. Nicht für den Live-Handel geeignet.",
    },
    tour: {
      title: "Zeitreisemodus verfügbar",
      close: "Schließen",
      body: "Verankern Sie alle Analysen und Empfehlungen an einem vergangenen Datum, um Strategien sicher zu backtesten und Look-ahead-Bias zu vermeiden. Zukünftige Daten werden abgelehnt und Ausgaben, die die Zeitbeschränkung verletzen, werden in der Zeitleiste hervorgehoben.",
      stepAnchor: "Klicken Sie jederzeit auf „Live-Analyse“ oder „Replay“, um in den Replay-Modus zu wechseln.",
      gotIt: "Verstanden",
    },
    sweep: {
      total: "Backtests gesamt",
      accuracy: "Genauigkeit",
      alpha: "Alpha",
      sharpe: "Sharpe",
    },
  },
  es: {
    pageAnchor: {
      live: "Análisis en vivo",
      replay: "Reproducción",
      untilDate: "Hasta {{date}}",
    },
    datePicker: {
      hint: "Seleccione un día de negociación pasado. Hoy y las fechas futuras están deshabilitadas (supuesto de mundo cerrado).",
      placeholder: "Seleccionar fecha de reproducción",
      ok: "Entrar en reproducción",
      cancel: "Cancelar",
    },
    degradedMarker: {
      tooltip: "En modo reproducción, los flujos de capital, los datos de margen, las posiciones northbound, etc. no tienen sentido histórico y se omiten.",
      labelWithCount: "Degradado · {{n}} elementos",
    },
    badge: {
      sweep: "Backtest por lotes",
      replay: "Reproducción {{date}}",
      replayTooltip: "Todos los datos, indicaciones y conclusiones de esta vista están anclados a {{date}}. No apto para trading en vivo.",
    },
    tour: {
      title: "Modo viaje en el tiempo disponible",
      close: "Cerrar",
      body: "Ancle todos los análisis y recomendaciones a una fecha pasada para probar estrategias de forma segura y evitar el sesgo de mirar hacia adelante. Las fechas futuras se rechazan y las salidas que violan la restricción temporal se resaltan en la línea de tiempo.",
      stepAnchor: "Haga clic en «Análisis en vivo» o «Reproducción» en cualquier momento para entrar en el modo de reproducción.",
      gotIt: "Entendido",
    },
    sweep: {
      total: "Total de backtests",
      accuracy: "Precisión",
      alpha: "Alpha",
      sharpe: "Sharpe",
    },
  },
  ru: {
    pageAnchor: {
      live: "Анализ в реальном времени",
      replay: "Повтор",
      untilDate: "По состоянию на {{date}}",
    },
    datePicker: {
      hint: "Выберите прошедший торговый день. Сегодня и будущие даты недоступны (допущение закрытого мира).",
      placeholder: "Выберите дату повтора",
      ok: "Войти в повтор",
      cancel: "Отмена",
    },
    degradedMarker: {
      tooltip: "В режиме повтора потоки капитала, маржинальные данные, северные позиции и т. д. не имеют исторического смысла и пропускаются.",
      labelWithCount: "Деградация · {{n}} эл.",
    },
    badge: {
      sweep: "Пакетный бэктест",
      replay: "Повтор {{date}}",
      replayTooltip: "Все данные, подсказки и выводы в этом представлении привязаны к {{date}}. Не для реальной торговли.",
    },
    tour: {
      title: "Доступен режим путешествия во времени",
      close: "Закрыть",
      body: "Привяжите все анализы и рекомендации к прошлой дате, чтобы безопасно тестировать стратегии и избегать смещения заглядывания вперёд. Будущие даты отклоняются, а выходы, нарушающие временное ограничение, выделяются на временной шкале.",
      stepAnchor: "В любой момент нажмите «Анализ в реальном времени» или «Повтор», чтобы войти в режим повтора.",
      gotIt: "Понятно",
    },
    sweep: {
      total: "Всего бэктестов",
      accuracy: "Точность",
      alpha: "Альфа",
      sharpe: "Шарп",
    },
  },
  hi: {
    pageAnchor: {
      live: "लाइव विश्लेषण",
      replay: "रीप्ले",
      untilDate: "{{date}} तक",
    },
    datePicker: {
      hint: "कोई पिछला ट्रेडिंग दिन चुनें। आज और भविष्य की तारीखें अक्षम हैं (बंद-विश्व धारणा)।",
      placeholder: "रीप्ले तिथि चुनें",
      ok: "रीप्ले में जाएँ",
      cancel: "रद्द करें",
    },
    degradedMarker: {
      tooltip: "रीप्ले मोड में, पूंजी प्रवाह, मार्जिन डेटा, नॉर्थबाउंड पोजीशन आदि का कोई ऐतिहासिक अर्थ नहीं होता और छोड़ दिया जाता है।",
      labelWithCount: "निम्नीकृत · {{n}} आइटम",
    },
    badge: {
      sweep: "बैच बैकटेस्ट",
      replay: "रीप्ले {{date}}",
      replayTooltip: "इस दृश्य के सभी डेटा, प्रॉम्प्ट और निष्कर्ष {{date}} पर आधारित हैं। लाइव ट्रेडिंग के लिए नहीं।",
    },
    tour: {
      title: "टाइम ट्रैवल मोड उपलब्ध",
      close: "बंद करें",
      body: "सभी विश्लेषणों और सिफारिशों को किसी पिछली तारीख से जोड़ें ताकि रणनीतियों का सुरक्षित बैकटेस्ट हो सके और आगे-देखने की पूर्वाग्रह से बचा जा सके। भविष्य की तारीखें अस्वीकार कर दी जाती हैं और समय बाधा का उल्लंघन करने वाले आउटपुट टाइमलाइन पर हाइलाइट किए जाते हैं।",
      stepAnchor: "रीप्ले मोड में जाने के लिए किसी भी समय «लाइव विश्लेषण» या «रीप्ले» पर क्लिक करें।",
      gotIt: "समझ गया",
    },
    sweep: {
      total: "कुल बैकटेस्ट",
      accuracy: "सटीकता",
      alpha: "अल्फा",
      sharpe: "शार्प",
    },
  },
  ar: {
    pageAnchor: {
      live: "تحليل مباشر",
      replay: "إعادة التشغيل",
      untilDate: "حتى {{date}}",
    },
    datePicker: {
      hint: "اختر يوم تداول سابق. اليوم والتواريخ المستقبلية معطلة (افتراض العالم المغلق).",
      placeholder: "اختر تاريخ إعادة التشغيل",
      ok: "دخول إعادة التشغيل",
      cancel: "إلغاء",
    },
    degradedMarker: {
      tooltip: "في وضع إعادة التشغيل، لا تحتوي تدفقات رأس المال وبيانات الهامش والمراكز الشمالية وما إلى ذلك على معنى تاريخي ويتم تخطيها.",
      labelWithCount: "تدهور · {{n}} عنصر",
    },
    badge: {
      sweep: "اختبار خلفي مجمع",
      replay: "إعادة تشغيل {{date}}",
      replayTooltip: "جميع البيانات والموجهات والاستنتاجات في هذا العرض مثبتة على {{date}}. غير مناسب للتداول الفعلي.",
    },
    tour: {
      title: "وضع السفر عبر الزمن متاح",
      close: "إغلاق",
      body: "ثبّت جميع التحليلات والتوصيات على تاريخ سابق لاختبار الاستراتيجيات بأمان وتجنب الانحياز للنظر إلى المستقبل. يتم رفض التواريخ المستقبلية، وتُبرز المخرجات المخالفة للقيد الزمني على الخط الزمني.",
      stepAnchor: "انقر في أي وقت على «تحليل مباشر» أو «إعادة التشغيل» للدخول إلى وضع إعادة التشغيل.",
      gotIt: "فهمت",
    },
    sweep: {
      total: "إجمالي الاختبارات",
      accuracy: "الدقة",
      alpha: "ألفا",
      sharpe: "شارب",
    },
  },
};

let modified = 0;
for (const [lang, tt] of Object.entries(TRANSLATIONS)) {
  const file = path.join(LOCALES_DIR, `${lang}.json`);
  const raw = fs.readFileSync(file, "utf8");
  const j = JSON.parse(raw);
  if ("timeTravel" in j) {
    console.log(`[skip] ${lang}: timeTravel 已存在`);
    continue;
  }
  // 按字母序插入: thinking < timeTravel < titlebar
  const keys = Object.keys(j);
  const insertAfter = keys.lastIndexOf("thinking");
  const idx = insertAfter >= 0 ? insertAfter + 1 : keys.length;
  const out = {};
  for (const [i, k] of keys.entries()) {
    if (i === idx) out.timeTravel = tt;
    out[k] = j[k];
  }
  if (!("timeTravel" in out)) out.timeTravel = tt;
  fs.writeFileSync(file, JSON.stringify(out, null, 2) + "\n", "utf8");
  modified++;
  console.log(`[ok] ${lang}: timeTravel 已插入 (${Object.keys(tt).length} 子段)`);
}
console.log(`\n完成，修改 ${modified} 个文件`);
