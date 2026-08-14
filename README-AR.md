# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent Poster" width="80%" />
  </a>
</p>

**AxAgent** هو عميل سطح مكتب ذكي (AI) متعدد المنصات مبني على Tauri 2 (Windows / macOS / Linux / Android / iOS)، ويُصنَّف كمنصة عمل يومية مدفوعة بالذكاء الاصطناعي للتطوير والبحث وإدارة المعرفة والأتمتة. يضم محرك وكلاء ReAct، والتوجيه المعرفي (توجيه هرمي ثلاثي المستويات + التوجيه المعزز بالاسترجاع RAR)، وتنسيق سير العمل المرئي، وقاعدة معرفة RAG محلية، وامتدادات بروتوكول MCP، وبوابة موحدة متعددة النماذج، وأتمتة المتصفح والتحكم بالحاسوب، مما ينقل الذكاء الاصطناعي من "المحادثة" إلى "التنفيذ".

> **اللغات**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## تموضع المشروع

يحل AxAgent ثلاث مشكلات أساسية:

1. **الوصول الموحد للنماذج المتعددة والجدولة الذكية** — استخدام OpenAI وAnthropic Claude وGoogle Gemini وDeepSeek وQwen وGLM وKimi وWenxin والنماذج المحلية عبر Ollama وأي واجهة API متوافقة مع OpenAI من واجهة واحدة، مع دعم التبديل التلقائي للحصص بين مفاتيح متعددة، والتوجيه الذكي حسب نوع المهمة، والمقارنة المتدفقة.
2. **حلقة مغلقة من المحادثة إلى التنفيذ** — أكثر من 163 أداة مدمجة + سير عمل مرئي + امتدادات MCP + التحكم بالمتصفح/الحاسوب، حيث يمكن للذكاء الاصطناعي معالجة الملفات وتشغيل الأكواد وإدارة Git وجدولة المهام.
3. **سيادة البيانات المحلية أولاً** — تُخزَّن سجلات المحادثة وقاعدة المعرفة والذاكرة والإعدادات في قاعدة بيانات SQLite محلية، وتُشفَّر مفاتيح API باستخدام AES-256-GCM، ويمكن تشغيل الوظائف الأساسية دون الحاجة إلى خدمات سحابية تابعة لجهات خارجية.

---

## القدرات الأساسية

### نظام التوجيه المعرفي (Cognitive Router)

يستخدم AxAgent `cognitive_query` كمدخل موحّد لجميع المحادثات، ويُسقط نية المستخدم على القدرات المحددة عبر **توجيه هرمي ثلاثي المستويات**:

- **توجيه المجال L1** (`domain_router`): قواعد + احتياط عبر LLM، يتعرّف على 9 مجالات أعمال رئيسية (تحليل البيانات / إنشاء المحتوى / التواصل / التشغيل والصيانة / وسائط الذكاء الاصطناعي / المالية / الأتمتة / عام، وغيرها)
- **توجيه المجموعات L2** (`cluster_router`): تحديد مجموعات القدرات داخل المجال (27 مجموعة تغطي 8 مجالات أعمال رئيسية)
- **توجيه القدرات L3**: **التوجيه المعزز بالاسترجاع (RAR)** — استرجاع أفضل K من سير العمل المشابهة من مكتبة متجهات القدرات وحقنها في الـ Prompt، مع إيجاد المسار عبر مخطط DAG لسير العمل، وإخراج عنوان المسار (مثل `/finance/stock_analysis/tech`) ووضع التنفيذ.
- **أوضاع التنفيذ**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`، تُختار تلقائيًا حسب درجة الثقة.
- **نظام القدرات**: سجل موحّد (`CapabilityRegistry`) + فهرس متجهات (`CapabilityIndexer`) + استرجاع هجين (`CapabilityRetriever`، متجهات + BM25 + مطابقة صارمة للوسوم + استبعاد العينات السلبية).
- **عزل القدرات النظامية**: عزل فيزيائي بين منسّق التوجيه المعرفي وسير عمل الأعمال، وتُوسَم القدرات النظامية بعلامة رؤية `SYSTEM_ONLY`، مع قاطع تيار مدمج ضد المرجعية الذاتية في طبقة التوجيه لمنع مفارقة الإحالة الذاتية.
- **التوجيه ثلاثي المستويات مُنفَّذ عبر DAG لسير العمل**: 4 قوالب توجيه جاهزة لسير العمل (التنسيق الرئيسي ~20 عقدة + توجيهات فرعية L1/L2/L3)، تُنفَّذ بواسطة محرك `rt-workflow`.

### محرك متعدد النماذج

- **13 محوّلًا للمزوّدين**: OpenAI (Chat Completions + Responses + Realtime) وAnthropic Claude وGoogle Gemini وDeepSeek وQwen وGLM وKimi وWenxin Yiyan وOllama وLlama.cpp (نماذج محلية بصيغة GGUF) وOpenClaw وHermes، بالإضافة إلى جميع واجهات API المتوافقة مع OpenAI.
- **تدوير المفاتيح المتعددة**: مفاتيح API متعددة لنفس المزوّد، مع تدوير تلقائي حسب الحصص، والتبديل التلقائي عند تقييد مفتاح واحد.
- **التوجيه الذكي**: اختيار النموذج الأمثل تلقائيًا حسب نوع المهمة (مراجعة الكود / التلخيص / الترجمة / عام)، مع دعم القواعد المخصصة.
- **مراقبة صحة المزوّدين**: تتبّع فوري لمعدل النجاح ووقت الاستجابة وحالة التوفر، مع دعم التخفيض التلقائي التدريجي.
- **توليد الصور بالذكاء الاصطناعي**: إعدادات مسبقة متعددة المقاسات لـ DALL-E 3 وFlux.
- **الصوت في الوقت الفعلي**: محادثة صوتية عبر WebSocket مبنية على OpenAI Realtime API، مع دعم المقاطعة والنسخ المتدفق.

### نظام الوكلاء (محرك ReAct)

- **المخطط الهرمي** (`hierarchical_planner`): تقسيم المهام المعقدة إلى خطة منظمة من المراحل (Phase) ثم المهام (Task)، وتجميعها في تنفيذ طوبولوجي عبر DAG.
- **البحث العميق** (`deep_research`): تنسيق البحث متعدد المصادر، يشمل خطة البحث وتنفيذ البحث وتجميع المحتوى وتتبّع الاستشهادات.
- **التحقق من الحقائق** (`fact_checker`): تحقق من الحقائق مدفوع بالذكاء الاصطناعي، يشمل مصنّف المصادر وتقييم الموثوقية.
- **شجرة الأفكار** (`tree_of_thoughts`): استكشاف استدلالي متعدد المسارات مع تقييم الفروع والتراجع.
- **العاكس** (`reflector`): تقييم ذاتي واقتراحات تحسين بعد تنفيذ المهمة.
- **التحقق الذاتي** (`self_verifier`): تحقق تلقائي من نتائج الاستدلال، مع كشف الحلقات.
- **استرداد الأخطاء** (`error_recovery_engine`): تصنيف نوع الخطأ ← اختيار استراتيجية الاسترداد ← إعادة المحاولة تلقائيًا أو تعديل الخطة، مع دعم التراجع الأسي.
- **اختبار A/B** (`ab_testing`): تقييم مقارن لاستراتيجيات الاستدلال المختلفة.
- **نظام التقييم** (`evaluator`): إطار عمل مدمج لاختبارات المعايير.
- **الضبط الدقيق LoRA** (`fine_tune`): خط أنابيب تدريب مدمج مع إدارة محوّلات LoRA.
- **محسّن التعلم المعزز** (`rl_optimizer`): تعلم معزز للسياسات مبني على التغذية الراجعة من التجارب.

**التعاون متعدد الوكلاء**:

- بنية تنسيق رئيسي-تابعي، مع تنفيذ متوازٍ للوكلاء الفرعيين وجدولة واعية بالتبعيات.
- لوحة مشتركة لتبادل المعلومات بين الوكلاء.
- وضع المناظرة التنافسية (جولات مؤيد/معارض مع تقييم قوة الحجج).
- وضع عنقود Swarm، عناقيد وكلاء متعددة العمليات.
- الوضع الاستباقي: يمكن للوكلاء بدء الاقتراحات والإجراءات بشكل استباقي.

**التحكم بالحاسوب**: نقر بالماوس وإدخال لوحة مفاتيح وتمرير شاشة مدفوع بالذكاء الاصطناعي، مع ثلاث مستويات من الأذونات (افتراضي / قبول التعديل / وصول كامل) وعزل مسارات عبر صندوق الرمل.

**أتمتة المتصفح**: التحكم بالمتصفح عبر بروتوكول CDP، مع دعم التنقل والتصوير والنقر وملء النماذج واستخراج النصوص.

### نظام المهارات

- **سوق المهارات**: تصفح وتثبيت مهارات المجتمع.
- **الإنشاء بمساعدة الذكاء الاصطناعي**: إنشاء بنية المهارة تلقائيًا من اقتراح بلغة طبيعية (`skill:create`).
- **تطور المهارات** (`evolution_engine`): تحليل وتحسين المهارات تلقائيًا بناءً على تغذية راجعة من التنفيذ.
- **المطابقة الدلالية**: توصية تلقائية بالمهارات ذات الصلة بناءً على دلالات سياق المحادثة.
- **تحليل المهارات** (`skill_decomposition`): تقسيم المهام المعقدة تلقائيًا إلى مجموعات من المهارات الذرية.
- **توليد الأدوات**: توليد وتسجيل أدوات جديدة بواسطة الذكاء الاصطناعي.
- **تنفيذ آمن**: تُنفَّذ المهارات بأمان داخل صندوق رمل معزول.

### سير العمل المرئي

محرر سير عمل DAG بالسحب والإفلات مبني على ReactFlow 12:

- **32 نوعًا من العقد**: المشغّلات، الوكلاء، استدعاء LLM، الفروع الشرطية، التفرع المتوازي، الحلقات، الدمج، التأخير، استدعاء الأدوات، تنفيذ الأكواد، سير العمل الفرعي، استرجاع المتجهات، تحليل المستندات، التحقق، النهاية، طلبات HTTP، Switch، استعلامات قاعدة البيانات، الإشعارات، الموافقات، عمليات الملفات، تحويل البيانات، إرسال Webhook، السجلات، مصنّفات LLM، المجمّعات، البريد، المناظرة، Swarm، الوكلاء المتعددون، التخزين، قواعد الأعمال.
- **تنفيذ الفرز الطوبولوجي Kahn**: كشف تلقائي للتبعيات الدائرية مع جدولة خطوط أنابيب متوازية.
- **قوالب مدمجة**: مراجعة الكود، إصلاح الأخطاء، توليد المستندات، الاختبار، إعادة البناء، الاستكشاف، تحليل الأداء، مراجعة الأمان، تطوير الميزات.
- **تسلسل YAML**: استيراد وتصدير تعريفات سير العمل.
- **إدارة الإصدارات**: التحكم بإصدارات القوالب.
- **التصميم بمساعدة الذكاء الاصطناعي**: تصميم سير العمل بمساعدة الذكاء الاصطناعي، مع توصية العقد والتشخيص.

### إدارة المعرفة

- **RAG متعدد قواعد المعرفة**: رفع المستندات ← التحليل التلقائي (PDF/DOCX/XLSX/PPTX/TXT) ← التقسيم إلى أجزاء ← فهرسة المتجهات.
- **الاسترجاع الهجين**: تشابه المتجهات (sqlite-vec + تضمين محلي عبر candle) + بحث نصي كامل BM25 (FTS5)، مع ترتيب هجين.
- **Self-RAG**: تفكّر وتحقق تلقائي في نتائج الاسترجاع.
- **إعادة الترتيب**: إعادة ترتيب النتائج عبر Cross-encoder.
- **الرسم البياني المعرفي**: استخراج الكيانات ← بناء العلاقات ← رسم بياني مرئي.
- **مراقبة الملفات**: مراقبة فورية لتغييرات الملفات مبنية على `notify`، مع فهرسة تزايدية تلقائية.
- **LLM Wiki**: مُجمِّع ومُتحقِّق Wiki بمساعدة الذكاء الاصطناعي.

### نظام الذاكرة

- **ذاكرة متعددة النطاقات**: عزل حسب المشروع/الموضوع، مع دعم الإدخال اليدوي والاستخراج التلقائي بالذكاء الاصطناعي.
- **تكامل الاستمرارية**: ذاكرة مغلقة الحلقة عبر Honcho وMem0.
- **الملف الشخصي للمستخدم**: تعلّم تلقائي لأسلوب الكود وتفضيلات حزمة التقنيات وأسلوب التواصل.
- **نقل الأسلوب**: استخراج خصائص أسلوب الكود ← تطبيقها على الكود المولَّد بالذكاء الاصطناعي.
- **تكامل الأحلام**: دمج تلقائي في الخلفية لشظايا الذاكرة وأنماط السلوك لتوليد معرفة منظمة.
- **ذاكرة المشروع**: استمرارية السياق على مستوى المشروع.

### بوابة API

بوابة HTTP + WebSocket مدمجة مبنية على `axum`:

- **نقاط نهاية متوافقة**: OpenAI `/v1/chat/completions` وClaude Messages API وGemini API، بالإضافة إلى OpenAI Responses وRealtime WebSocket.
- **إدارة المفاتيح**: توليد وإبطال وتفعيل/تعطيل مفاتيح الوصول، مع دعم وقت الانتهاء.
- **تتبّع الاستخدام**: إحصائيات حجم الطلبات واستهلاك الرموز (tokens) حسب المفتاح/المزوّد/التاريخ، مع تصدير مقاييس Prometheus.
- **تحديد المعدل**: خوارزمية دلو الرموز المبنية على `governor`.
- **SSL/TLS**: شهادات موقّعة ذاتيًا مدمجة (`rcgen`) مع دعم الشهادات المخصصة.
- **الربط الخارجي**: تكامل بنقرة واحدة مع أدوات خارجية مثل Claude CLI وOpenCode، مع مزامنة تلقائية لمفاتيح API.
- **تذاكر الوقت الفعلي**: تذاكر مصادقة مؤقتة مبنية على HMAC لنقل آمن لاتصالات WebSocket.
- **وضع الخادم**: ثنائي اختياري `axagent-server` يوفّر قدرات تطبيق سطح المكتب كخدمة خارجية.

### تكامل منصات الرسائل

بوابة متعددة المنصات عبر `rt-messaging`، تدعم استقبال الرسائل وتحليل الأوامر والرد التلقائي بالذكاء الاصطناعي على **DingTalk وFeishu وQQ وSlack وWeChat وWhatsApp وTelegram وDiscord**.

### نظام الأدوات

**أكثر من 163 أداة مدمجة**، تُسجَّل جميعها عبر trait `Tool`، وتغطي 15 فئة رئيسية:

| الفئة           | أمثلة على الأدوات                                                                                                                                                             |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| عمليات الملفات  | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, 11 أداة (الدليل/الحذف/النقل وغيرها)                                                                                   |
| Shell/Web       | `bash`, `web_fetch`, `web_search`                                                                                                                                             |
| الشبكة          | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                                        |
| المتصفح         | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot` و10 أدوات أخرى (CDP)                                                                                |
| التحكم بالحاسوب | `computer_use` (الماوس/لوحة المفاتيح/لقطة الشاشة)                                                                                                                             |
| Git             | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                                 |
| قاعدة المعرفة   | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document` و6 أدوات أخرى                                                                                            |
| إدارة المهام    | `todo_write`, `task_*` (6 أدوات), `cron_*` (3 أدوات), ما يتعلق بـ `plan`                                                                                                      |
| دفع الرسائل     | `push_notification`, `send_message`, أدوات التعاون الجماعي                                                                                                                    |
| قاعدة البيانات  | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                         |
| التخزين         | `get_storage_info`, `upload_storage_file`, `download_storage_file` و5 أدوات أخرى                                                                                              |
| التصدير/التنسيق | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown` و9 أدوات أخرى                                                                                    |
| OCR             | `ocr_image`, `ocr_detect_langs`                                                                                                                                               |
| Obsidian        | `obsidian_search`, `obsidian_read`, `obsidian_backlinks` و9 أدوات أخرى                                                                                                        |
| أخرى            | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD وDevOps وRPC والاختبار وغيرها |

### بروتوكول MCP

تنفيذ كامل لبروتوكول MCP (Model Context Protocol) مبني على `rmcp`:

- **طبقة النقل**: عمليات فرعية stdio + Streamable HTTP + SSE.
- **مصادقة OAuth**: دعم تدفق تفويض OAuth لخوادم MCP.
- **اكتشاف الأدوات**: اكتشاف وتسجيل تلقائي للأدوات التي تُعرّضها خوادم MCP.
- **مدير MCP**: إدارة دورة حياة الخوادم وفحوصات الصحة وإعادة الاتصال التلقائي.

### نظام الإضافات

بنية إضافات ثلاثية المستويات متوافقة مع OpenClaw (مدمجة / مرفقة / خارجية):

- تثبيت عبر حزم npm، مع واجهة سوق مدمجة للبحث والتثبيت.
- تعريف manifest للإضافات، وإعلان الأذونات، وتنفيذ معزول في صندوق الرمل.
- تسجيل أدوات مخصصة، ومزوّدو الوكلاء (Agent)، واعتراض Hook.
- مثبّت المهارات: تثبيت المهارات من حزم الإضافات إلى نظام المهارات.

### محرك واجهة المستخدم الديناميكية

- **مدفوع بالمخطط**: بناء الواجهات تصريحيًا عبر JSON Schema دون كتابة أكواد.
- **31 مكوّنًا مدمجًا**: الحاويات (7) / عرض البيانات (6) / النماذج (9) / الوسائط (4) / أخرى (5).
- **ربط البيانات**: ربط تصريحي لمصادر البيانات مع العرض الشرطي.
- **NL2UI**: توليد واجهات UI ديناميكية مباشرة من اللغة الطبيعية.

### SDK عميل ACP

- **ACP (Agent Client Protocol)**: SDK ثنائي اللغة (TypeScript + Python) دون أي تبعيات خارجية.
- إدارة الجلسات، إرسال الـ Prompt، تسجيل استدعاءات الأدوات، تدفق أحداث WebSocket.
- التواصل مع خدمة AxAgent عبر نقاط النهاية `/acp/v1/*`.

### الأمان

- **تشفير AES-256-GCM**: تخزين مشفّر محليًا لمفاتيح API والإعدادات الحساسة (crate `crypto`).
- **الحماية من حقن الـ Prompt**: خط دفاع من أربع مستويات (`prompt-guard`) — كشف الأنماط ← ترميز الفواصل ← غلاف XML ← وسوم الثقة، مدمج في كامل سلسلة الجلسات وبناء الـ Prompt وGit وRAG.
- **الحماية من SSRF**: فحص أمان لعناوين URL، ومنع الطلبات إلى العناوين الداخلية.
- **تصفية المحتوى**: تصفية أمان متعددة الأنواع للمحتوى.
- **تحديد المعدل**: تقييد دلو الرموز لاستدعاءات الأدوات وطلبات API.
- **قاطع التيار**: قطع تلقائي عند الفشل المتكرر.
- **التحكم بالوصول**: التحكم بأذونات الوصول إلى الأدوات بناءً على السياسات.
- **عزل صندوق الرمل**: عزل بيئات تنفيذ الوكلاء والمهارات.

### أدوات المطورين

- **التتبّع الموزع** (`telemetry`): تكامل OpenTelemetry مع تصور Span/Trace.
- **سجلات منظمة**: tracing-subscriber + طوابع زمنية chrono.
- **تصحيح إعادة التشغيل**: تسجيل مسارات تنفيذ الوكلاء (`trajectory_recorder`) وإعادة تشغيلها.
- **لوحة DevTools**: عارض الخط الزمني Trace Explorer، ومشغّل المعايير Benchmark Runner، وموصي الأدوات Tool Recommender.
- **اختبارات المعايير**: معايير Criterion (tool_exec / llm_call / search).
- **فحوصات CI**: `npm run ci:check` يدمج فحص الأنواع والـ lint والتحقق من التنسيق.

### تجربة سطح المكتب والأجهزة المحمولة

- **تخطيط متجاوب**: نقاط فاصلة CSS تتكيف مع سطح المكتب/الجهاز اللوحي/الهاتف (3 مستويات لتخطيط الأجهزة: `desktop` / `tablet` / `mobile`).
- **11 لغة**: الصينية المبسطة، الصينية التقليدية، الإنجليزية، اليابانية، الكورية، الفرنسية، الألمانية، الإسبانية، الروسية، الهندية، العربية.
- **محرك السمات** (`rt-theme`): سمات داكنة/فاتحة + عدة إعدادات مسبقة، مع تخصيص عميق لـ Ant Design 6.
- **محرر Monaco**: تلوين صياغي، معاينة الفروقات، دعم متعدد اللغات.
- **طرفية xterm.js**: WebLinks وUnicode 11 والبحث.
- **التمرير الافتراضي**: @tanstack/react-virtual + react-virtuoso.
- **عرض الرسوم البيانية**: D2 + Mermaid + Recharts + Sigma (للرسوم البيانية).
- **لوحة الأوامر**: لوحة أوامر عامة عبر Ctrl+K.
- **درج النظام + اختصارات لوحة المفاتيح العامة + التشغيل التلقائي**: تشغيل في الخلفية دون إزعاج.
- **التحديث التلقائي**: كشف إصدارات GitHub Releases بفاصل زمني قابل للتكوين.
- **دعم الوكيل**: إعداد وكيل HTTP / SOCKS5.
- **مساحة عمل سحابية**: مزامنة تخزين S3 وWebDAV، مع كشف التعارضات والمزامنة ثنائية الاتجاه.

### الأجهزة المحمولة

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- تكييف خاص بالأجهزة المحمولة: تكييف منطقة الأمان، التنقل السفلي، تنقل Drawer.

---

## البنية التقنية

### حزمة التقنيات

| الطبقة                | التقنية                                  | الإصدار |
| --------------------- | ---------------------------------------- | ------- |
| إطار سطح المكتب       | Tauri                                    | 2.11    |
| إطار الواجهة الأمامية | React                                    | 19      |
| نظام الأنواع          | TypeScript                               | 7       |
| مكتبة واجهة المستخدم  | Ant Design                               | 6       |
| إطار CSS              | TailwindCSS                              | 4       |
| إدارة الحالة          | Zustand                                  | 5       |
| التوجيه               | React Router                             | 7       |
| محرر الأكواد          | Monaco Editor                            | 0.55    |
| الطرفية               | xterm.js                                 | 6       |
| محرر سير العمل        | ReactFlow                                | 12      |
| الرسوم البيانية       | D2 + Mermaid + Recharts + Sigma          |         |
| الرسوم المتحركة       | Framer Motion                            | 12      |
| التمرير الافتراضي     | @tanstack/react-virtual + react-virtuoso |         |
| السحب والإفلات        | @dnd-kit                                 | 6       |
| عرض Markdown          | markstream-react + stream-markdown       |         |
| التدويل               | i18next + react-i18next                  |         |
| أداة البناء           | Vite                                     | 8       |
| الاختبار              | Vitest + Playwright                      |         |
| التنسيق               | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Lint                  | ESLint + Oxlint + Clippy                 |         |

### البنية الخلفية: نمط حقن التبعيات Harness

تعتمد بنية Rust workspace، وتضم **37 عضوًا** (الـ crate الرئيسي + 35 crate مكتبة + schema-gen)، وتتبع **بنية حقن التبعيات Harness**:

> جميع الـ crates مفكوكة عبر واجهات trait المعرّفة في axagent-harness، ويتم تجميع وحقن التبعيات في وقت التشغيل بواسطة axagent-runtime.
> اتجاه التبعية: `التنفيذ المحدد → harness ← المستدعي`

**harness** هو حجر الأساس في البنية — صفر منطق أعمال، صفر تنفيذ محدد، ويحتوي فقط على تعريفات trait وDTO للبيانات النقية وثوابت وأنواع أخطاء موحدة. تعتمد عليه جميع الـ crates الأخرى، ولا يعتمد هو على أي crate من نوع axagent-* (أكثر من 200 تعريف trait تغطي Agent/Provider/Tool/RAG/التخزين/MCP/الإضافات/الأمان/المراقبة/الذاكرة/التعلم/المتصفح/الرسائل/التوجيه المعرفي وغيرها).

```
src-tauri/crates/
├── harness/          # 架构基石 — trait 接口、DTO、错误类型、DI 契约
├── entities/         # SeaORM 实体模型
├── dao/              # 数据访问层（CRUD）
├── migration/        # 数据库迁移
├── crypto/           # AES-256-GCM 加解密与密钥管理
├── credential/       # 凭据安全存储
├── storage/          # 文件存储抽象（本地/S3/WebDAV），ZIP 读写
├── cache/            # 内存缓存层
├── disk-cache/       # 磁盘文件级缓存
├── search/           # 检索引擎（FTS5 + sqlite-vec + candle 本地嵌入）
├── document-parser/  # 文档文本提取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集（路径/编码/哈希/日期）
├── runtime-core/     # 运行时公共类型、配置常量
├── runtime/          # 运行时服务编排 — 装配全部 crate 的 DI 容器
├── rt-workflow/      # 工作流引擎 — DAG 编排、节点执行器、YAML 序列化
├── rt-messaging/     # 消息平台网关 — 钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 服务器
├── rt-dashboard/     # 仪表盘插件框架
├── rt-theme/         # 主题引擎
├── agent/            # AI 智能体核心 — 80+ 模块
│                     #   ReAct引擎/层级规划/深度研究/事实核查/思维树/反思/
│                     #   自验证/错误恢复/RL优化/LoRA微调/评估/工具推荐/A/B测试/
│                     #   协调器/黑板/视觉管线/Web搜索/学术搜索/Wiki编译等
├── orchestrator/     # 智能体编排 — 多智能体调度、DAG 分解、动态子图执行
├── providers/        # 模型提供商适配器（13 种）
├── tools/            # 工具体系 — Tool trait/注册表/编排/流式/沙箱/163+内置工具
├── gateway/          # API 网关 — axum HTTP/WS 服务器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 协议 — stdio + Streamable HTTP + SSE，基于 rmcp
├── trajectory/       # 学习系统 — 记忆/技能进化/用户画像/梦境整合
├── plugins/          # 插件系统 — OpenClaw 兼容、npm 包安装、市场
├── telemetry/        # 可观测性 — OpenTelemetry、结构化日志、运行时指标
├── prompt-guard/     # 提示词注入防护 — L1-L4 多级检测管线
├── npm/              # npm 注册表客户端
├── crdt/             # 协同编辑数据结构
├── device/           # 设备管理
├── axagent-mobile/   # 移动端适配层
├── agent-macro/      # 智能体宏
├── agent-command-types/ # 智能体命令类型
└── schema-gen/       # 数据库 Schema 生成工具
```

### البنية الأمامية

```
src/
├── pages/            # 页面（24 个）
│   ├── ChatPage           # 对话主界面 — 侧边栏/消息流/Agent 面板/多 Tab
│   ├── DashboardPage      # 数据仪表盘 — 用量统计/模型分布/趋势图表
│   ├── WorkflowPage       # 工作流编辑器 — ReactFlow DAG 可视化
│   ├── KnowledgeHubPage   # 知识库管理 — 文档上传/索引/检索
│   ├── MemoryPage         # 记忆管理
│   ├── SkillsPage         # 技能市场
│   ├── SettingsPage       # 设置面板 — 40+ 配置项
│   ├── TerminalPage       # 内置终端 — xterm.js
│   ├── FilesPage          # 文件管理
│   ├── GatewayLinkPage    # API 网关与外部链接管理
│   ├── QuickBarPage       # 快捷栏（独立窗口）
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 动态 UI 引擎
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 学习图谱
│   ├── FineTunePage       # LoRA 微调
│   ├── PersonaPage        # 角色管理
│   ├── WorkflowMarketplace # 工作流市场
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 个模块，500+ 组件
│   ├── chat/         # 对话（消息流/输入/ChatView/TabBar/RightPanel/附件/工具调用渲染）
│   ├── layout/       # 布局 — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader 等
│   ├── agent/        # Agent 面板/入口/迷你面板
│   ├── workflow/     # 工作流编辑器（节点/连线/面板/模板/AI辅助）
│   ├── settings/     # 设置面板（40+ 子组件）
│   ├── skill/        # 技能编辑器/渲染器/浮动面板
│   ├── dynamicUI/    # 动态 UI 组件（31 个内置组件）
│   ├── gateway/      # API 网关管理
│   ├── files/        # 文件管理
│   ├── terminal/     # 终端组件
│   ├── search/       # 搜索界面
│   ├── benchmark/    # 基准测试面板
│   ├── decomposition/# 技能分解与工具生成
│   ├── devtools/     # Trace/Span 时间线 + RL Training 面板
│   ├── approval/     # 审批流程界面
│   ├── recommendation/ # 工具/模型推荐
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # 帮助面板
│   ├── notification/ # 通知组件
│   ├── proactive/    # 主动建议
│   ├── llm-wiki/     # LLM Wiki 组件
│   ├── wiki/         # Wiki 组件
│   ├── fine-tune/    # 微调界面
│   ├── trace/        # Trace 组件
│   ├── style/        # 样式/主题
│   ├── shared/       # 共享组件（ErrorBoundary / PageContextProvider）
│   └── common/       # 通用组件（Icon 等）
│
├── stores/           # Zustand 状态管理（82 个 store）
│   ├── domain/       # 9 个核心业务 store（对话/流/压缩/偏好/多模型等）
│   ├── feature/      # 61 个功能模块 store（智能体/工作流/知识库/技能/网关/记忆/终端等）
│   ├── shared/       # 8 个跨组件共享 store（UI/标签页/工作区/后端状态等）
│   └── devtools/     # 4 个开发者工具 store
│
├── hooks/            # React Hooks（快捷键/命令面板/响应式/滚动条/主题/Avatar 等）
├── lib/              # 工具函数库（invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 等 45+ 模块）
├── types/            # TypeScript 类型定义
├── theme/            # Shadcn 主题引擎
├── i18n/             # 11 语言翻译文件（zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar）
├── constants/        # 常量与功能开关
└── sdk/              # ACP 客户端 SDK（TypeScript + Python）
```

### مفاتيح الميزات

يدير المشروع الإصدار التدريجي للميزات عبر `featureFlags.ts`:

| المفتاح             | الحالة | الوصف                                      |
| ------------------- | ------ | ------------------------------------------ |
| `AGENT_IN_THE_LOOP` | ✅     | لوحة Agent عامة + حقن سياق الصفحة          |
| `DYNAMIC_UI`        | ✅     | محرك بناء واجهة المستخدم الديناميكية       |
| `SELF_EVOLUTION_UI` | ❌     | سطح تحكم أمامي للتطور الذاتي               |
| `NL_EXTENSION`      | ❌     | توسيع أعمال ديناميكي مدفوع باللغة الطبيعية |

### إضافات Tauri

| الإضافة             | الغرض                               |
| ------------------- | ----------------------------------- |
| `autostart`         | التشغيل التلقائي عند الإقلاع        |
| `clipboard-manager` | قراءة وكتابة الحافظة                |
| `dialog`            | مربع حوار اختيار الملفات            |
| `fs`                | الوصول إلى نظام الملفات             |
| `global-shortcut`   | تسجيل اختصارات لوحة المفاتيح العامة |
| `notification`      | إشعارات النظام                      |
| `opener`            | فتح الروابط/الملفات الخارجية        |
| `process`           | إدارة العمليات                      |
| `updater`           | التحديث التلقائي                    |

---

## أدلة البيانات

```
~/.axagent/                    # 应用配置
├── axagent.db                 # SQLite 主数据库 (SeaORM)
├── master.key                 # AES-256 主密钥
├── vector_db/                 # sqlite-vec 向量索引
└── ssl/                       # 自签名 SSL 证书

~/Documents/axagent/          # 用户文件
├── images/                   # 图片附件
├── files/                    # 文件附件
└── backups/                  # 自动备份
```

---

## بدء سريع

### المتطلبات البيئية

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+（edition 2024）
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（MSVC + Windows SDK）
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### التطوير

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 开发模式（前端 Vite HMR + Tauri 窗口）
```

### البناء

```bash
npm run tauri build    # 桌面端生产构建

npm run tauri:android:build   # Android 构建
npm run tauri:ios:build       # iOS 构建
```

توجد مخرجات بناء سطح المكتب في `src-tauri/target/release/`.

### الاختبار

```bash
npm run test           # 前端单元测试（Vitest watch）
npm run test:run       # 前端单元测试（单次运行）
npm run test:e2e       # E2E 测试（Playwright）

# Rust 后端测试
cd src-tauri && cargo test

# 类型检查 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint 格式化
npm run lint:eslint    # ESLint 检查
npm run contracts      # API 契约检查

# CI 全量检查
npm run ci:check
```

### السكربتات الشائعة

| الأمر                    | الغرض                      |
| ------------------------ | -------------------------- |
| `npm run bump`           | ترقية رقم الإصدار (تفاعلي) |
| `npm run docs`           | توليد وثائق TypeDoc        |
| `npm run skill:create`   | إنشاء هيكل مهارة جديدة     |
| `npm run skill:validate` | التحقق من تعريف المهارة    |
| `npm run check:types`    | فحص اتساق الأنواع          |

---

## المنصات المدعومة

| المنصة  | البنية                                |
| ------- | ------------------------------------- |
| Windows | x86_64, ARM64                         |
| macOS   | Apple Silicon (arm64), Intel (x86_64) |
| Linux   | x86_64, ARM64                         |
| Android | arm64-v8a, armeabi-v7a, x86_64        |
| iOS     | arm64                                 |

---

## رخصة المصدر المفتوح

هذا المشروع مفتوح المصدر بموجب رخصة [AGPL-3.0-only](LICENSE).

---

## شكر وتقدير

بُني AxAgent فوق العديد من المشاريع مفتوحة المصدر المتميزة:

- [Tauri](https://tauri.app/) — إطار سطح المكتب متعدد المنصات
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — واجهة المستخدم الأمامية
- [SeaORM](https://www.sea-ql.org/SeaORM/) — ORM للغة Rust
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — استرجاع المتجهات
- [candle](https://github.com/huggingface/candle) — استدلال التضمين المحلي
- [rmcp](https://github.com/nicholasxjy/rmcp) — SDK لبروتوكول MCP في Rust
- [ReactFlow](https://reactflow.dev/) — محرر سير العمل المرئي
- [axum](https://github.com/tokio-rs/axum) — إطار HTTP
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — محرر الأكواد
- [xterm.js](https://xtermjs.org/) — محاكي الطرفية
- [Zustand](https://zustand.docs.pmnd.rs/) — إدارة الحالة
- [Framer Motion](https://www.framer.com/motion/) — مكتبة الرسوم المتحركة
- [Recharts](https://recharts.org/) — مكتبة الرسوم البيانية
