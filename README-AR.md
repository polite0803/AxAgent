# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="ملصق AxAgent" width="80%" />
  </a>
</p>

**AxAgent** هو عميل سطح مكتب مساعد ذكاء اصطناعي متعدد المنصات مبني على Tauri 2 (Windows / macOS / Linux / Android / iOS). يدمج محرك وكيل ReAct، تنسيق سير العمل المرئي، قواعد المعرفة RAG المحلية، ملحقات بروتوكول MCP، بوابة متعددة النماذج موحدة، أتمتة المتصفح والتحكم بالكمبيوتر — ليكون محطة عمل ذكاء اصطناعي للتطوير اليومي والبحث وإدارة المعرفة والأتمتة.

> **اللغات**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## توجه المشروع

يحل AxAgent ثلاث مشكلات أساسية:

1. **وصول موحد متعدد النماذج وتوجيه ذكي** — استخدم OpenAI و Anthropic Claude و Google Gemini ونماذج Ollama المحلية وأي واجهة برمجة تطبيقات متوافقة مع OpenAI في واجهة واحدة، مع دوران تلقائي متعدد المفاتيح حسب الحصة، وتوجيه ذكي حسب نوع المهمة، ومقارنة بالبث المباشر
2. **حلقة مغلقة من المحادثة إلى التنفيذ** — 47+ أداة مدمجة + سير عمل مرئي + ملحقات MCP + متصفح/تحكم بالكمبيوتر، يمكن للذكاء الاصطناعي التعامل مع الملفات وتنفيذ التعليمات البرمجية وإدارة Git وجدولة المهام
3. **سيادة البيانات المحلية أولاً** — تُخزَّن المحادثات وقواعد المعرفة والذاكرة والتكوينات في قاعدة بيانات SQLite محلية، وتُشفَّر مفاتيح API بتشفير AES-256-GCM. تعمل الوظائف الأساسية دون خدمات سحابية خارجية

---

## القدرات الأساسية

### محرك متعدد النماذج

- **9 محولات لمقدمي الخدمات**: OpenAI (Chat Completions + Responses + Realtime)، Anthropic Claude، Google Gemini، Ollama (مع إدارة نماذج GGUF المحلية)، OpenClaw، Hermes وجميع واجهات برمجة التطبيقات المتوافقة مع OpenAI
- **دوران متعدد المفاتيح**: عدة مفاتيح API لكل مزود، دوران تلقائي حسب الحصة، تحويل تلقائي عند حد المفتاح الواحد
- **توجيه ذكي**: اختيار تلقائي للنموذج حسب نوع المهمة (مراجعة الكود / تلخيص / ترجمة / عام)، مع قواعد قابلة للتخصيص
- **مراقبة صحة المزودين**: تتبع فوري لمعدل النجاح وزمن الاستجابة والتوفر، مع تراجع تلقائي متدرج
- **توليد صور بالذكاء الاصطناعي**: DALL-E 3 و Flux (Replicate) مع إعدادات مسبقة متعددة الأحجام
- **صوت فوري**: محادثة صوتية عبر WebSocket مبنية على OpenAI Realtime API، مع دعم المقاطعة والنسخ بالبث المباشر

### نظام الوكيل (محرك ReAct)

- **المخطط الهرمي** (`hierarchical_planner`): تحليل المهام المعقدة إلى خطط منظمة مرحلة → مهمة، تُجمَّع إلى تنفيذ طوبولوجي DAG
- **البحث المعمق** (`deep_research`): تنسيق بحث متعدد المصادر يشمل تخطيط البحث والتنفيذ وتوليف المحتوى وتتبع الاستشهادات
- **مدقق الحقائق** (`fact_checker`): تدقيق حقائق مدعوم بالذكاء الاصطناعي مع مصنف مصادر وتقييم للمصداقية
- **شجرة الأفكار** (`tree_of_thoughts`): استكشاف استدلال متعدد المسارات مع تقييم الفروع والتراجع
- **العاكس** (`reflector`): تقييم ذاتي بعد التنفيذ واقتراحات للتحسين
- **المدقق الذاتي** (`self_verifier`): تحقق تلقائي من نتائج الاستدلال مع كشف الدورات
- **استرداد الأخطاء** (`error_recovery_engine`): تصنيف نوع الخطأ → اختيار استراتيجية الاسترداد → إعادة محاولة تلقائية أو تعديل الخطة، مع تراجع أسي
- **اختبار A/B** (`ab_testing`): تقييم مقارن لاستراتيجيات استدلال مختلفة
- **نظام التقييم** (`evaluator`): إطار معايير مدمج
- **ضبط LoRA الدقيق** (`fine_tune`): خط تدريب مدمج مع إدارة محولات LoRA
- **محسن RL** (`rl_optimizer`): تعلم تعزيز السياسات بناءً على تغذية راجعة من التجربة

**تعاون متعدد الوكلاء**:

- بنية تنسيق رئيسي-تابع مع تنفيذ متواز للوكلاء الفرعيين وجدولة واعية بالاعتماديات
- لوحة مشتركة لتبادل المعلومات بين الوكلاء
- وضع مناظرة تنافسية (جولات مع/ضد مع تسجيل قوة الحجج)
- وضع Swarm لعناقيد الوكلاء متعددة العمليات
- وضع استباقي: يمكن للوكلاء بدء الاقتراحات والعمليات

**التحكم بالكمبيوتر**: نقرات فأرة وإدخال لوحة مفاتيح وتمرير شاشة مدعومة بالذكاء الاصطناعي، مع ثلاثة مستويات أذونات (افتراضي / قبول التعديلات / وصول كامل) وعزل مسارات الصندوق الرملي

**أتمتة المتصفح**: تحكم بالمتصفح عبر بروتوكول CDP، مع التنقل ولقطات الشاشة والنقرات وملء النماذج واستخراج النص

### نظام المهارات

- **سوق المهارات**: تصفح وتثبيت مهارات المجتمع
- **إنشاء بمساعدة الذكاء الاصطناعي**: إنشاء تلقائي لهياكل المهارات من اقتراحات اللغة الطبيعية (`skill:create`)
- **تطور المهارات** (`evolution_engine`): تحليل وتحسين تلقائي للمهارات بناءً على تغذية راجعة من التنفيذ
- **مطابقة دلالية**: توصية دلالية سياقية للمهارات
- **تحليل المهارات** (`skill_decomposition`): تحليل تلقائي للمهام المعقدة إلى تركيبات مهارات ذرية
- **أدوات مُنشأة**: أدوات جديدة ينشئها ويسجلها الذكاء الاصطناعي
- **تنفيذ معزول**: تنفذ المهارات في بيئات صندوق رملي معزولة

### سير العمل المرئي

محرر سير عمل DAG بالسحب والإفلات مبني على ReactFlow 12:

- **17 نوع عقدة**: مُشغِّل، وكيل، استدعاء LLM، تفرع شرطي، تفرع متوازي، حلقة تكرارية، دمج، تأخير، استدعاء أداة، تنفيذ كود، سير عمل فرعي، بحث متجهي، تحليل مستند، تحقق، نهاية، قاعدة عمل، دور وكيل
- **تنفيذ ترتيب طوبولوجي Kahn**: كشف تلقائي للدورات، جدولة متوازية للخطوط
- **قوالب مدمجة**: مراجعة الكود، إصلاح الأخطاء، التوثيق، الاختبار، إعادة الهيكلة، الاستكشاف، تحليل الأداء، تدقيق الأمان، تطوير الميزات
- **تسلسل YAML**: استيراد/تصدير تعريفات سير العمل
- **إدارة الإصدارات**: تحكم بإصدارات القوالب
- **تصميم بمساعدة الذكاء الاصطناعي**: تصميم سير عمل وتوصية عقد بمساعدة الذكاء الاصطناعي

### إدارة المعرفة

- **RAG متعدد قواعد المعرفة**: رفع المستندات → تحليل تلقائي (PDF/DOCX/XLSX/PPTX/TXT) → تجزئة → فهرسة متجهية
- **بحث هجين**: تشابه متجهي (sqlite-vec + embeddings محلية candle) + بحث نصي كامل BM25 (FTS5)، ترتيب هجين
- **Self-RAG**: تفكير وتحقق تلقائي من نتائج البحث
- **إعادة الترتيب**: إعادة ترتيب النتائج عبر cross-encoder
- **رسم بياني معرفي**: استخراج الكيانات → بناء العلاقات → رسم بياني مرئي
- **مراقبة الملفات**: مراقبة تغييرات الملفات الفورية عبر `notify`، فهرسة تزايدية تلقائية
- **LLM Wiki**: مترجم ومدقق Wiki بمساعدة الذكاء الاصطناعي

### نظام الذاكرة

- **ذاكرة متعددة النطاقات**: عزل حسب المشروع/الموضوع، إدخال يدوي واستخراج تلقائي بالذكاء الاصطناعي
- **تكامل دائم**: ذاكرة حلقة مغلقة Honcho و Mem0
- **ملف المستخدم**: تعلم تلقائي لأسلوب البرمجة وتفضيلات الحزمة التقنية وأسلوب التواصل
- **نقل الأسلوب**: استخراج خصائص أسلوب الكود → تطبيقها على الكود المُنشأ بالذكاء الاصطناعي
- **دمج Dream**: دمج تلقائي في الخلفية لشظايا الذاكرة وأنماط السلوك في معرفة منظمة
- **ذاكرة المشروع**: استمرارية سياق مستوى المشروع

### بوابة API

بوابة HTTP + WebSocket مدمجة مبنية على `axum`:

- **نقاط نهاية متوافقة**: OpenAI `/v1/chat/completions`، Claude Messages API، Gemini API، بالإضافة إلى OpenAI Responses و Realtime WebSocket
- **إدارة المفاتيح**: إنشاء وإبطال وتبديل تفعيل/تعطيل مفاتيح الوصول مع دعم انتهاء الصلاحية
- **تتبع الاستخدام**: إحصائيات عدد الطلبات واستهلاك الرموز حسب المفتاح/المزود/التاريخ، تصدير مقاييس Prometheus
- **تحديد المعدل**: خوارزمية دلو الرموز عبر `governor`
- **SSL/TLS**: شهادات ذاتية التوقيع مدمجة (`rcgen`)، دعم شهادات مخصصة
- **ربط خارجي**: تكامل بنقرة واحدة مع Claude CLI و OpenCode وأدوات خارجية أخرى، مزامنة تلقائية لمفاتيح API
- **تذاكر فورية**: تذاكر مصادقة مؤقتة مبنية على HMAC لنقل آمن لاتصالات WebSocket

### تكامل منصات المراسلة

بوابة متعددة المنصات عبر `rt-messaging`، تدعم استقبال الرسائل وتحليل الأوامر والرد التلقائي بالذكاء الاصطناعي لـ **DingTalk و Feishu و QQ و Slack و WeChat و WhatsApp و Telegram و Discord**.

### نظام الأدوات

47+ أداة مدمجة، مسجلة بشكل موحد عبر trait `Tool`:

| الفئة             | الأدوات                                                                                                                                                                                                    |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| عمليات الملفات    | `file_read`، `file_write`، `file_edit`، `file_system`                                                                                                                                                      |
| تنفيذ الكود       | `bash`، `repl`                                                                                                                                                                                             |
| البحث             | `grep`، `glob`                                                                                                                                                                                             |
| المتصفح           | `browser` (CDP)                                                                                                                                                                                            |
| التحكم بالكمبيوتر | `computer_use` (فأرة/لوحة مفاتيح/لقطة شاشة)                                                                                                                                                                |
| الويب             | `web_search`، `web_fetch`                                                                                                                                                                                  |
| قاعدة المعرفة     | `knowledge`، `document`                                                                                                                                                                                    |
| Git               | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| أدوات التطوير     | `lsp`، `workspace`                                                                                                                                                                                         |
| إدارة المهام      | `plan`، `task_system`، `todo_write`، `cron`                                                                                                                                                                |
| المراسلة          | `push_notification`، `messaging`                                                                                                                                                                           |
| قاعدة البيانات    | `database`                                                                                                                                                                                                 |
| التخزين           | `storage`                                                                                                                                                                                                  |
| أخرى              | `agent`، `agent_memory`، `context`، `export`، `integration`، `media`، `media_delivery`، `migration_tool`، `monitor`، `obsidian`، `ocr`، `personality`، `shared_path`، `system_info`، `testing`، `worktree` |

### بروتوكول MCP

تنفيذ كامل لبروتوكول MCP (Model Context Protocol) مبني على `rmcp`:

- **النقل**: عملية فرعية stdio + Streamable HTTP + WebSocket
- **مصادقة OAuth**: تدفق تفويض OAuth لخوادم MCP
- **اكتشاف الأدوات**: اكتشاف وتسجيل تلقائي للأدوات المعروضة من خوادم MCP
- **مدير MCP**: إدارة دورة حياة الخادم، فحوصات الصحة، إعادة اتصال تلقائية

### نظام الملحقات

بنية ملحقات ثلاثية المستويات متوافقة مع OpenClaw (مدمجة / مجمعة / خارجية):

- تثبيت حزم npm مع واجهة سوق للبحث والتثبيت
- تعريف بيان الملحق، إعلان الأذونات، تنفيذ معزول في صندوق رملي
- تسجيل أدوات مخصصة، مزودي وكلاء، اعتراض Hooks
- مُثبِّت المهارات: تثبيت المهارات من حزم الملحقات إلى نظام المهارات

### الأمان

- **تشفير AES-256-GCM**: تخزين محلي مشفر لمفاتيح API والتكوينات الحساسة (crate `crypto`)
- **حماية من حقن الأوامر**: خط دفاع رباعي المستويات (`prompt-guard`) — كشف الأنماط → تهريب المحددات → غلاف XML → علامات ثقة، مدمج عبر سلسلة المحادثة/بناء الأوامر/Git/RAG
- **حماية SSRF**: فحص أمان عناوين URL لمنع الطلبات إلى عناوين الشبكة الداخلية
- **تصفية المحتوى**: تصفية أمان محتوى متعدد الأنواع
- **تحديد المعدل**: تحديد دلو الرموز لاستدعاءات الأدوات وطلبات API
- **قاطع الدائرة**: قطع تلقائي عند الفشل المتتالي
- **التحكم بالوصول**: تحكم بصلاحيات الوصول للأدوات مبني على السياسات
- **عزل الصندوق الرملي**: عزل بيئة تنفيذ الوكلاء والمهارات

### أدوات المطور

- **تتبع موزع** (`telemetry`): تكامل OpenTelemetry مع تصور Span/Trace
- **تسجيل منظم**: tracing-subscriber + طوابع زمنية chrono
- **تصحيح بالإعادة**: تسجيل مسارات تنفيذ الوكيل (`trajectory_recorder`) وإعادة التشغيل
- **لوحة DevTools**: عارض الخط الزمني Trace Explorer، Benchmark Runner، Tool Recommender
- **معايير**: معايير Criterion (tool_exec / llm_call / search)
- **فحوصات CI**: `npm run ci:check` يدمج فحص الأنواع والتدقيق والتحقق من التنسيق

### تجربة سطح المكتب والجوال

- **تخطيط متجاوب**: تكيف سطح المكتب/الجهاز اللوحي/الجوال عبر نقاط توقف CSS (3 مستويات: `desktop` / `tablet` / `mobile`)
- **11 لغة**: الصينية المبسطة، الصينية التقليدية، الإنجليزية، اليابانية، الكورية، الفرنسية، الألمانية، الإسبانية، الروسية، الهندية، العربية
- **محرك السمات** (`rt-theme`): سمات داكنة/فاتحة + إعدادات مسبقة متعددة (بما في ذلك سمة 21th أحادية المسافة)، تخصيص عميق لـ Ant Design 6
- **محرر Monaco**: تمييز نحوي، معاينة الفروقات، دعم متعدد اللغات
- **طرفية xterm.js**: WebLinks، Unicode 11، بحث
- **تمرير افتراضي**: @tanstack/react-virtual + react-virtuoso
- **عرض الرسوم البيانية**: D2 + Mermaid + Recharts
- **قائمة نسخ عالمية**: قائمة نسخ مخصصة، قمع قائمة السياق الأصلية
- **لوحة أوامر**: لوحة أوامر عالمية Ctrl+K
- **شريط النظام + اختصارات عالمية + تشغيل تلقائي**: تشغيل خلفي غير متطفل
- **تحديث تلقائي**: فحص إصدارات GitHub Releases بفاصل زمني قابل للتكوين
- **دعم البروكسي**: تكوين بروكسي HTTP / SOCKS5
- **مساحة عمل سحابية**: مزامنة تخزين S3 و WebDAV مع كشف التعارضات والمزامنة ثنائية الاتجاه

### الجوال

- Android APK/AAB (arm64-v8a، armeabi-v7a، x86_64)
- iOS IPA (arm64)
- تكيفات خاصة بالجوال: هوامش المنطقة الآمنة، تنقل سفلي، تنقل درج

---

## البنية التقنية

### الحزمة التقنية

| الطبقة                | التقنية                                  | الإصدار |
| --------------------- | ---------------------------------------- | ------- |
| إطار سطح المكتب       | Tauri                                    | 2.11    |
| إطار الواجهة الأمامية | React                                    | 19      |
| نظام الأنواع          | TypeScript                               | 7       |
| مكتبة UI              | Ant Design                               | 6       |
| إطار CSS              | TailwindCSS                              | 4       |
| إدارة الحالة          | Zustand                                  | 5       |
| التوجيه               | React Router                             | 7       |
| محرر الكود            | Monaco Editor                            | 0.55    |
| الطرفية               | xterm.js                                 | 6       |
| محرر سير العمل        | ReactFlow                                | 12      |
| الرسوم البيانية       | D2 + Mermaid + Recharts                  |         |
| الرسوم المتحركة       | Framer Motion                            | 12      |
| التمرير الافتراضي     | @tanstack/react-virtual + react-virtuoso |         |
| السحب والإفلات        | @dnd-kit                                 | 6       |
| عرض Markdown          | markstream-react + stream-markdown       |         |
| i18n                  | i18next + react-i18next                  |         |
| أداة البناء           | Vite                                     | 8       |
| الاختبار              | Vitest + Playwright                      |         |
| التنسيق               | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| التدقيق               | ESLint + Oxlint + Clippy                 |         |

### بنية الواجهة الخلفية: حقن تبعيات Harness

بنية مساحة عمل Rust بـ **32 crate**، تتبع نمط **Harness DI**:

> جميع الـ crates مفصولة عبر واجهات trait المعرفة بواسطة axagent-harness، ويقوم axagent-runtime بتجميع وحقن التبعيات في وقت التشغيل.
> اتجاه التبعية: `التطبيقات الملموسة → harness ← المستدعيون`

**harness** هو حجر الزاوية المعماري — بدون منطق أعمال، بدون تطبيقات ملموسة، يحتوي فقط على تعريفات trait و DTOs بيانات خالصة وثوابت وأنواع أخطاء موحدة. تعتمد عليه جميع الـ crates الأخرى ولا يعتمد هو على أي crate من axagent-* (200+ تعريف trait تغطي Agent/Provider/Tool/RAG/التخزين/MCP/الملحقات/الأمان/المراقبة/الذاكرة/التعلم/المتصفح/المراسلة وغيرها).

```
src-tauri/crates/
├── harness/          # حجر الزاوية المعماري — واجهات trait، DTOs، أنواع الأخطاء، عقود DI
├── entities/         # نماذج كيانات SeaORM
├── dao/              # طبقة الوصول للبيانات (CRUD)
├── migration/        # ترحيل قاعدة البيانات
├── crypto/           # تشفير/فك تشفير AES-256-GCM وإدارة المفاتيح
├── credential/       # تخزين آمن لبيانات الاعتماد
├── storage/          # تجريد تخزين الملفات (محلي/S3/WebDAV)، قراءة/كتابة ZIP
├── cache/            # طبقة تخزين مؤقت في الذاكرة
├── disk-cache/       # تخزين مؤقت للملفات على القرص
├── search/           # محرك بحث (FTS5 + sqlite-vec + embeddings محلية candle)
├── document-parser/  # استخراج نصوص المستندات (PDF/DOCX/XLSX/PPTX)
├── kit/              # أدوات عامة (مسارات/ترميز/تجزئة/تواريخ)
├── runtime-core/     # أنواع وقت تشغيل عامة، ثوابت تكوين
├── runtime/          # تنسيق خدمات وقت التشغيل — حاوية DI تجمع جميع الـ 30+ crate
├── rt-workflow/      # محرك سير العمل — تنسيق DAG، منفذي العقد، تسلسل YAML
├── rt-messaging/     # بوابة منصات المراسلة — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # خادم Webhook عام
├── rt-dashboard/     # إطار ملحقات لوحة التحكم
├── rt-theme/         # محرك السمات
├── agent/            # نواة وكيل الذكاء الاصطناعي — 80+ وحدة
│                     #   محركReAct/تخطيطهرمي/بحثمعمق/تدقيقحقائق/شجرةأفكار/
│                     #   تفكير/تحققذاتي/استردادأخطاء/تحسينRL/ضبطLoRA/
│                     #   تقييم/توصيةأدوات/اختبارAB/منسق/لوحة/خطرؤية/
│                     #   بحثويب/بحثأكاديمي/تجميعWiki وغيرها
├── orchestrator/     # تنسيق الوكلاء — جدولة متعددة الوكلاء، تحليل DAG، تنفيذ رسوم فرعية ديناميكي
├── providers/        # محولات مزودي النماذج
├── tools/            # نظام الأدوات — trait Tool/سجل/تنسيق/بث/صندوق رملي/47+ أداة مدمجة
├── gateway/          # بوابة API — خادم HTTP/WS axum، OAuth، تحديد المعدل، Prometheus
├── mcp/              # بروتوكول MCP — stdio + Streamable HTTP، مبني على rmcp
├── trajectory/       # نظام التعلم — ذاكرة/تطور المهارات/ملفات المستخدم/دمج Dream
├── plugins/          # نظام الملحقات — متوافق مع OpenClaw، تثبيت حزم npm، سوق
├── telemetry/        # المراقبة — OpenTelemetry، تسجيل منظم، مقاييس وقت التشغيل
├── prompt-guard/     # حماية حقن الأوامر — خط كشف متعدد المستويات L1-L4
├── npm/              # عميل سجل npm
└── schema-gen/       # أداة إنشاء مخطط قاعدة البيانات
```

### بنية الواجهة الأمامية

```
src/
├── pages/            # الصفحات (23+ بما في ذلك الصفحات الفرعية)
│   ├── ChatPage           # واجهة الدردشة — شريط جانبي/تدفق الرسائل/لوحة Agent/علامات تبويب متعددة
│   ├── DashboardPage      # لوحة التحكم — إحصائيات الاستخدام/توزيع النماذج/رسوم بيانية للاتجاهات
│   ├── WorkflowPage       # محرر سير العمل — تصور ReactFlow DAG
│   ├── KnowledgeHubPage   # إدارة قاعدة المعرفة — رفع/فهرسة/بحث
│   ├── MemoryPage         # إدارة الذاكرة
│   ├── SkillsPage         # سوق المهارات
│   ├── SettingsPage       # لوحة الإعدادات — 40+ عنصر تكوين
│   ├── TerminalPage       # طرفية مدمجة — xterm.js
│   ├── FilesPage          # إدارة الملفات
│   ├── GatewayLinkPage    # بوابة API وإدارة الربط الخارجي
│   ├── QuickBarPage       # شريط سريع (نافذة مستقلة)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # محرك UI ديناميكي
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # رسم بياني للتعلم
│   ├── FineTunePage       # ضبط LoRA الدقيق
│   ├── PersonaPage        # إدارة الشخصيات
│   ├── WorkflowMarketplace # سوق سير العمل
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 28 وحدة، 450+ مكون
│   ├── chat/         # الدردشة (تدفق الرسائل/الإدخال/ChatView/TabBar/RightPanel/المرفقات/عرض استدعاءات الأدوات)
│   ├── layout/       # التخطيط — 17 مكون
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal وغيرها
│   ├── agent/        # لوحة Agent/مدخل/لوحة مصغرة
│   ├── workflow/     # محرر سير العمل (عقد/حواف/لوحات/قوالب/مساعدة ذكاء اصطناعي)
│   ├── settings/     # لوحة الإعدادات (40+ مكون فرعي)
│   ├── skill/        # محرر مهارات/عارض/لوحات عائمة
│   ├── dynamicUI/    # سجل مكونات UI الديناميكية (26 مكون مدمج)
│   ├── gateway/      # إدارة بوابة API
│   ├── files/        # إدارة الملفات
│   ├── terminal/     # مكونات الطرفية
│   ├── search/       # واجهة البحث
│   ├── benchmark/    # لوحة المعايير
│   ├── decomposition/# تحليل المهارات وتوليد الأدوات
│   ├── devtools/     # خط زمني Trace/Span + لوحة RL Training
│   ├── approval/     # واجهة سير عمل الموافقة
│   ├── recommendation/ # توصية أدوات/نماذج
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # لوحة المساعدة
│   ├── notification/ # مكونات الإشعارات
│   ├── proactive/    # اقتراحات استباقية
│   ├── llm-wiki/     # مكونات LLM Wiki
│   ├── wiki/         # مكونات Wiki
│   ├── fine-tune/    # واجهة الضبط الدقيق
│   ├── trace/        # مكونات Trace
│   ├── style/        # النمط/السمة
│   ├── shared/       # مكونات مشتركة (ErrorBoundary / PageContextProvider)
│   └── common/       # مكونات عامة (Icon وغيرها)
│
├── stores/           # إدارة حالة Zustand
│   ├── domain/       # 10 مخازن أعمال أساسية (محادثة/تدفق/ضغط/تفضيلات/نماذج متعددة وغيرها)
│   ├── feature/      # 48 مخزن وحدات وظيفية (وكيل/سير عمل/معرفة/مهارات/بوابة/ذاكرة/طرفية وغيرها)
│   └── devtools/     # 4 مخازن أدوات مطور
│
├── hooks/            # React Hooks (اختصارات/لوحة أوامر/متجاوب/شريط تمرير/سمة/صورة رمزية وغيرها)
├── lib/              # مكتبة أدوات (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout وغيرها — 45+ وحدة)
├── types/            # تعريفات أنواع TypeScript
├── theme/            # محرك سمات Shadcn
├── i18n/             # ملفات ترجمة 11 لغة (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # ثوابت وعلامات ميزات
└── sdk/              # SDK تكامل خارجي
```

### علامات الميزات

يدير المشروع طرح الميزات التدريجي عبر `featureFlags.ts`:

| العلامة             | الحالة | الوصف                                         |
| ------------------- | ------ | --------------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅     | لوحة Agent عالمية + حقن سياق الصفحة           |
| `DYNAMIC_UI`        | ✅     | محرك بناء UI ديناميكي                         |
| `SELF_EVOLUTION_UI` | ❌     | لوحة تحكم تطور ذاتي للواجهة الأمامية          |
| `NL_EXTENSION`      | ❌     | ملحقات أعمال ديناميكية مدفوعة باللغة الطبيعية |

### ملحقات Tauri

| الملحق              | الغرض                          |
| ------------------- | ------------------------------ |
| `autostart`         | تشغيل تلقائي عند الإقلاع       |
| `clipboard-manager` | قراءة/كتابة الحافظة            |
| `dialog`            | حوارات اختيار الملفات          |
| `fs`                | الوصول لنظام الملفات           |
| `global-shortcut`   | تسجيل اختصارات عالمية          |
| `notification`      | إشعارات النظام                 |
| `opener`            | فتح روابط/ملفات خارجية         |
| `process`           | إدارة العمليات                 |
| `updater`           | تحديث تلقائي                   |
| `mcp-bridge`        | جسر بروتوكول MCP (غير Android) |

---

## دليل البيانات

```
~/.axagent/                    # تكوين التطبيق
├── axagent.db                 # قاعدة بيانات SQLite الرئيسية (SeaORM)
├── master.key                 # مفتاح رئيسي AES-256
├── vector_db/                 # فهرس متجهي sqlite-vec
└── ssl/                       # شهادات SSL ذاتية التوقيع

~/Documents/axagent/          # ملفات المستخدم
├── images/                   # مرفقات الصور
├── files/                    # مرفقات الملفات
└── backups/                  # نسخ احتياطية تلقائية
```

---

## بداية سريعة

### المتطلبات الأساسية

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+ (edition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### التطوير

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # وضع التطوير (Vite HMR + نافذة Tauri)
```

### البناء

```bash
npm run tauri build    # بناء إنتاج سطح المكتب

npm run tauri:android:build   # بناء Android
npm run tauri:ios:build       # بناء iOS
```

مخرجات بناء سطح المكتب موجودة في `src-tauri/target/release/`.

### الاختبار

```bash
npm run test           # اختبارات وحدة الواجهة الأمامية (Vitest watch)
npm run test:run       # اختبارات وحدة الواجهة الأمامية (تشغيل فردي)
npm run test:e2e       # اختبارات E2E (Playwright)

# اختبارات Rust الخلفية
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# فحص الأنواع والتدقيق
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # تنسيق dprint
npm run lint:eslint    # فحص ESLint
npm run contracts      # فحص عقود API

# فحص CI كامل
npm run ci:check
```

### البرامج النصية

| الأمر                    | الغرض                   |
| ------------------------ | ----------------------- |
| `npm run bump`           | ترقية تفاعلية للإصدار   |
| `npm run docs`           | إنشاء توثيق TypeDoc     |
| `npm run skill:create`   | إنشاء هيكل مهارة جديد   |
| `npm run skill:validate` | التحقق من تعريف المهارة |
| `npm run check:types`    | فحص اتساق الأنواع       |

---

## دعم المنصات

| المنصة  | المعمارية                             |
| ------- | ------------------------------------- |
| Windows | x86_64, ARM64                         |
| macOS   | Apple Silicon (arm64), Intel (x86_64) |
| Linux   | x86_64, ARM64                         |
| Android | arm64-v8a, armeabi-v7a, x86_64        |
| iOS     | arm64                                 |

---

## الترخيص

هذا المشروع مفتوح المصدر تحت ترخيص [AGPL-3.0-only](LICENSE).

---

## الشكر والتقدير

بُني AxAgent على العديد من المشاريع مفتوحة المصدر المتميزة:

- [Tauri](https://tauri.app/) — إطار سطح مكتب متعدد المنصات
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — واجهة المستخدم الأمامية
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — بحث متجهي
- [candle](https://github.com/huggingface/candle) — استدلال embeddings محلي
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — محرر سير عمل مرئي
- [axum](https://github.com/tokio-rs/axum) — إطار HTTP
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — محرر كود
- [xterm.js](https://xtermjs.org/) — محاكي طرفية
- [Zustand](https://zustand.docs.pmnd.rs/) — إدارة حالة
- [Framer Motion](https://www.framer.com/motion/) — مكتبة رسوم متحركة
- [Recharts](https://recharts.org/) — مكتبة رسوم بيانية
