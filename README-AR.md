[**English**](./README-EN.md) | [简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | **العربية**

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp;amp;utm_source=badge-featured&amp;amp;utm_medium=badge&amp;amp;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>عميل AI مكتبي متعدد المنصات | تعاون متعدد الوكلاء | محلي أولاً</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## ما هو AxAgent؟

AxAgent هو تطبيق مكتبي متعدد المنصات غني بالميزات، يدمج قدرات وكلاء AI المتقدمة وأدوات مطور شاملة. يدعم مزودي نماذج متعددين، وتنفيذ خطوط الأنابيب المستقلة، وتنسيق سير العمل المرئي، وإدارة المعرفة المحلية، وبوابة API مدمجة.

---

## معاينة اللقطات

| المحادثة واختيار النماذج | لوحة تحكم الوكلاء المتعددين |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| قاعدة المعرفة RAG | الذاكرة والسياق |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| محرر سير العمل | بوابة API |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## الميزات الأساسية

### 🤖 دعم نماذج AI

- **دعم متعدد المزودين** — تكامل أصلي مع OpenAI وAnthropic Claude وGoogle Gemini وOllama وOpenClaw وHermes وجميع واجهات برمجة التطبيقات المتوافقة مع OpenAI
- **تدوير مفاتيح متعددة** — تكوين مفاتيح API متعددة لكل مزود مع التدوير التلقائي لتوزيع ضغط حدود المعدل
- **دعم النماذج المحلية** — دعم كامل لنماذج Ollama المحلية، بما في ذلك إدارة ملفات GGUF/GGML
- **إدارة النماذج** — جلب قوائم النماذج عن بُعد، تخصيص المعاملات (temperature، max tokens، top-p، إلخ)
- **الإخراج المتدفق** — عرض في الوقت الفعلي رمزاً بعد رمز مع كتل تفكير قابلة للطي (تفكير Claude الموسع)
- **مقارنة النماذج المتعددة** — طرح أسئلة على نماذج متعددة في آنٍ واحد، مقارنة النتائج جنباً إلى جنب
- **استدعاء الدوال** — استدعاء دوال منظمة عبر جميع المزودين المدعومين
- **واجهة استجابات OpenAI** — دعم نقل تنسيق استجابات OpenAI
- **واجهة API في الوقت الفعلي** — دفع أحداث WebSocket المتوافق مع واجهة OpenAI Realtime API

### 🔐 نظام وكلاء AI

نظام الوكلاء مبني على بنية متقنة، مع الميزات التالية:

- **محرك استدلال ReAct** — دمج الاستدلال والعمل، مع التحقق الذاتي المدمج لضمان تنفيذ المهام بشكل موثوق
- **المخطط الهرمي** — تفكيك المهام المعقدة إلى خطط منظمة بمراحل وتبعيات
- **مفكك المهام** — تقسيم المهام المعقدة تلقائياً إلى مهام فرعية قابلة للتنفيذ
- **البحث المتعمق** — تنسيق بحث متعدد المصادر وتتبع الاقتباسات وتقييم الموثوقية
- **التحقق من الحقائق** — تحقق مدعوم بالذكاء الاصطناعي من الحقائق وتصنيف المصادر
- **تنسيق البحث** — تنسيق مزودي بحث متعددين مع دعم تخطيط البحث وتوليف النتائج
- **البحث الأكاديمي** — استرجاع الأدبيات الأكاديمية وتحليل الاقتباسات
- **التحكم في الكمبيوتر** — نقرات الماوس وإدخال لوحة المفاتيح والتمرير المدعوم بالذكاء الاصطناعي مع تحليل نماذج الرؤية
- **إدراك الشاشة** — التقاط لقطات الشاشة وتحليل نماذج الرؤية لتحديد عناصر واجهة المستخدم
- **وضع أذونات ثلاثي المستويات** — افتراضي (يتطلب موافقة)، قبول التعديلات (موافقة تلقائية)، وصول كامل (بدون مطالبات)
- **عزل الصندوق الرملي** — عمليات الوكيل مقيدة بشكل صارم بدليل العمل المحدد
- **لوحة موافقة الأدوات** — عرض في الوقت الفعلي لطلبات استدعاء الأدوات مع دعم الموافقة على كل أداة
- **تتبع التكلفة** — إحصائيات استخدام الرموز والتكلفة في الوقت الفعلي لكل جلسة
- **إيقاف/استئناف** — إيقاف تنفيذ الوكيل في أي وقت واستئنافه لاحقاً
- **نظام نقاط التفتيش** — نقاط تفتيش دائمة لاستعادة الأعطال وإعادة اتصال الجلسات
- **محرك استعادة الأخطاء** — تصنيف تلقائي للأخطاء وتحليل السبب الجذري وتنفيذ استراتيجيات الاستعادة
- **اكتشاف الحلقات** — اكتشاف وقطع سلوك التكرار في استدلال الوكيل تلقائياً
- **سلسلة الأفكار** — تصور استدلال قرارات الوكيل، تفكيك خطوة بخطوة
- **الوضع الاستباقي** — يمكن للوكيل تقديم اقتراحات وإجراءات استباقياً
- **إدارة الأغراض** — صيانة وتتبع غرض تنفيذ الوكيل وسياقه

### 👥 تعاون الوكلاء المتعددين

- **تنسيق الوكلاء الفرعيين** — بنية رئيسي-تابع، دعم وكلاء متعاونين متعددين
- **التنفيذ المتوازي** — معالجة مهام الوكلاء المتعددين بالتوازي مع دعم جدولة مدركة للتبعيات
- **النقاش المضاد** — جولات نقاش مؤيد/معارض مع دعم تسجيل قوة الحجج وتتبع الردود
- **أدوار الوكلاء** — أدوار محددة مسبقاً (باحث، مخطط، مطور، مراجع، مُلخّص) لعمل الفريق
- **منسق الوكلاء** — توجيه رسائل مركزي وإدارة حالة لفريق وكلاء متعددين
- **رسم الاتصالات** — عرض مرئي لتفاعلات الوكلاء وتدفق الرسائل
- **مجموعة Swarm** — مجموعة وكلاء متعددة العمليات مع دعم مزامنة الأذونات وإعادة الاتصال التلقائي
- **نظام رفيق Buddy** — وكلاء رفقاء قابلون للتكوين مع دعم تعريف الأنواع والسمات
- **الذاكرة المشتركة** — مساحة ذاكرة مشتركة بين الوكلاء مع دعم الإحصائيات والاستعلامات
- **تسجيل Cron للفريق** — جدولة مهام مجدولة على مستوى الفريق

### ⭐ نظام المهارات

- **سوق المهارات** — سوق مدمج لتصفح وتثبيت المهارات التي ساهم بها المجتمع
- **إنشاء المهارات** — إنشاء مهارات تلقائياً من المقترحات مع دعم محرر Markdown
- **تطور المهارات** — تحليل وتحسين تلقائي مدعوم بالذكاء الاصطناعي للمهارات الموجودة بناءً على ملاحظات التنفيذ
- **مطابقة المهارات** — مطابقة دلالية، توصية بمهارات متعلقة بسياق المحادثة
- **تفكيك المهارات** — تقسيم المهام المعقدة تلقائياً إلى مهارات ذرية قابلة للتنفيذ (مساعدة LLM/متعدد الجولات/تحقق سير العمل)
- **الأدوات المولدة** — ينشئ الذكاء الاصطناعي تلقائياً ويسجل أدوات جديدة لتوسيع قدرات الوكيل
- **مركز المهارات** — واجهة مركزية لاكتشاف المهارات وإدارة التكوين
- **عميل مركز المهارات** — تكامل مع مركز المهارات عن بُعد مع دعم المشاركة المجتمعية
- **فحص تبعيات المهارات** — فحص تلقائي لتبعيات المهارات وتوافر الأدوات
- **حاوية صندوق رمل المهارات** — تنفيذ آمن للمهارات في بيئة معزولة

### 🔄 نظام سير العمل

ينفذ محرك سير العمل نظام تنسيق مهام قائم على DAG:

- **محرر سير العمل المرئي** — مصمم سير عمل قابل للسحب والإفلات مع دعم توصيل وتكوين العقد
- **أنواع عقد غنية** — 15 نوع عقدة: مشغل، وكيل، LLM، شرطي، متوازي، حلقة، دمج، تأخير، أداة، كود، سير عمل فرعي، استرجاع متجه، تحليل مستندات، تحقق، نهاية
- **قوالب سير العمل** — إعدادات مسبقة مدمجة: مراجعة الكود، إصلاح الأخطاء، التوثيق، الاختبار، إعادة الهيكلة، الاستكشاف، الأداء، الأمان، تطوير الميزات
- **تنفيذ DAG** — ترتيب طوبولوجي بخوارزمية Kahn مع دعم اكتشاف الحلقات
- **الجدولة المتوازية** — تنفيذ خط أنابيب، الخطوات السريعة لا تنتظر البطيئة
- **استراتيجية إعادة المحاولة** — تراجع أسي، أقصى عدد من المحاولات قابل للتكوين لكل خطوة
- **الإكمال الجزئي** — الخطوات الفاشلة لا تحظر الخطوات اللاحقة المستقلة
- **إدارة الإصدارات** — تحكم في إصدار قوالب سير العمل مع دعم التراجع
- **تاريخ التنفيذ** — تسجيل مفصل مع دعم تتبع الحالة وتصحيح الأخطاء
- **مساعدة AI** — تصميم سير العمل بمساعدة AI وتوصية العقد وتحسين مطالبات الوكيل
- **التحقق الدلالي** — تحقق دلالي لسير العمل لتحديد المشاكل المحتملة
- **استيراد n8n** — دعم استيراد سير العمل من دليل n8n
- **لوحة التصحيح** — تصحيح أخطاء وحالة تنفيذ سير العمل في الوقت الفعلي

### 📚 المعرفة والذاكرة

- **قاعدة المعرفة (RAG)** — دعم قواعد معرفة متعددة، رفع المستندات، تحليل تلقائي، تقسيم وفهرسة متجهة
- **البحث الهجين** — دمج بحث تشابه المتجهات مع ترتيب BM للنص الكامل
- **إعادة الترتيب** — إعادة ترتيب عبر المشفر المتقاطع لتحسين دقة الاسترجاع
- **خط أنابيب الاستدعاء ثلاثي المستويات** — آلية استدعاء متعددة المستويات عبر فهرس AST + بحث متجه + FTS5
- **رسم المعرفة** — تصور علاقات كيانات المعرفة (كيان، سمة، علاقة، تدفق، واجهة)
- **نظام Wiki** — مترجم والتحقق من LLM Wiki مع دعم تصور رسم المعرفة والمزامنة التزايدية
- **ملاحظات Wiki** — نظام ملاحظات بروابط ثنائية الاتجاه مع دعم عرض الرسم والمزامنة التلقائية للروابط
- **نظام الذاكرة** — ذاكرة متعددة مساحات الأسماء مع دعم الإدخال اليدوي أو الاستخراج التلقائي بالذكاء الاصطناعي
- **ذاكرة الحلقة المغلقة** — تكامل مزودي الذاكرة الدائمة Honcho وMem0
- **بحث النص الكامل FTS5** — استرجاع سريع عبر المحادثات والملفات والذكريات
- **بحث الجلسات** — بحث متقدم عبر جميع جلسات المحادثة
- **إدارة السياق** — إرفاق ملفات ونتائج بحث ومقتطفات معرفية وذكريات ومخرجات أدوات بمرونة
- **محلل المستندات** — تحليل تلقائي واستخراج محتوى من مستندات متعددة التنسيقات
- **الفهرس التزايدي** — تحديثات فهرس تزايدية لتغييرات الملفات

### 🌐 بوابة API

- **خادم API محلي** — خادم API محلي مدمج متوافق مع OpenAI وClaude وGemini
- **الروابط الخارجية** — تكامل بنقرة واحدة مع Claude CLI وOpenCode مع مزامنة تلقائية لمفاتيح API والنماذج
- **إدارة المفاتيح** — توليد مفاتيح الوصول وإلغاؤها وتمكينها/تعطيلها مع دعم الوصف
- **تحليلات الاستخدام** — حجم الطلبات واستخدام الرموز حسب المفتاح والمزود والتاريخ
- **دعم SSL/TLS** — شهادات موقّعة ذاتياً مدمجة مع دعم الشهادات المخصصة
- **سجلات الطلبات** — تسجيل كامل لجميع طلبات واستجابات API
- **قوالب التكوين** — قوالب مسبقة البناء لـ Claude وCodex وOpenCode وGemini
- **واجهة API في الوقت الفعلي** — دفع أحداث WebSocket المتوافق مع واجهة OpenAI Realtime API
- **تكامل المنصات** — دعم DingTalk وFeishu وQQ وSlack وWeChat وWhatsApp وTelegram وDiscord
- **تشخيصات البوابة** — تشخيصات الاتصال وإدارة سياسات البرنامج
- **محدد المعدل** — تحديد معدل طلبات API والتحكم في حركة المرور
- **الطابور الدائم** — إدارة طابور الطلبات الدائمة

### 🔧 الأدوات والامتدادات

- **بروتوكول MCP** — تطبيق كامل لبروتوكول سياق النموذج مع دعم نقل stdio وHTTP/WebSocket
- **مصادقة OAuth** — دعم تدفق مصادقة OAuth لخوادم MCP
- **البدء التلقائي لـ MCP** — بدء تلقائي لخوادم MCP وإدارة دورة الحياة
- **جسر أدوات MCP** — ربط أدوات MCP مع نظام أدوات الوكيل
- **نظام الإضافات** — بنية إضافات ثلاثية المستويات (مدمجة/مجمعة/خارجية) مع دعم تسجيل الأدوات والخطافات وإدارة دورة الحياة
- **الأدوات المدمجة** — عمليات ملفات شاملة (قراءة/كتابة/تحرير)، تنفيذ الكود، بحث (Grep/Glob)، Bash، بحث الويب، جلب الويب، إدارة الخطط، جدولة Cron، REPL، LSP، إدارة السياق، التحكم في الكمبيوتر، دفع الرسائل، قوائم المهام، إلخ
- **نظام أذونات الأدوات** — تصنيف أذونات الأدوات وإدارة القواعد وتتبع الاستخدام
- **أمان Bash** — تحليل الأوامر والتحقق من المسارات والتحكم في أمان الصندوق الرملي
- **عميل LSP** — بروتوكول خادم اللغة المدمج مع دعم إكمال الكود والتشخيصات
- **فهرس AST** — تحليل وبناء فهرس AST لملفات الكود
- **واجهة الطرفية الخلفية** — دعم اتصالات الطرفية المحلية وDocker وSSH
- **أتمتة المتصفح** — التحكم في المتصفح عبر تكامل CDP (التنقل، لقطات الشاشة، النقر، التعبئة، استخراج النص، إلخ)
- **أتمتة واجهة المستخدم** — تحديد والتحكم في عناصر واجهة المستخدم عبر المنصات
- **أدوات Git** — عمليات Git مع دعم اكتشاف الفروع والوعي بالتعارضات
- **توصية الأدوات** — محرك توصية ذكي بالأدوات بناءً على السياق
- **تنسيق الأدوات** — تنفيذ منسق لأدوات متعددة وإخراج متدفق
- **إحصائيات الأدوات** — إحصائيات تكرار استخدام الأدوات والأداء

### 📊 عرض المحتوى

- **عرض Markdown** — دعم كامل لتمييز الكود وصيغ الرياضيات LaTeX والجداول وقوائم المهام
- **محرر كود Monaco** — محرر مدمج مع دعم تمييز بناء الجملة والنسخ ومعاينة الفروقات
- **عرض المخططات** — مخططات تدفق Mermaid ومخططات بنية D2 ومخططات ECharts التفاعلية
- **لوحة Artifact** — مقتطفات كود ومسودات HTML ومكونات React وملاحظات Markdown مع دعم المعاينة في الوقت الفعلي
- **أوضاع المعاينة الأربعة** — كود (محرر)، مقسم (جنباً إلى جنب)، معاينة (عرض فقط)، معاينة مكون React
- **مفتش الجلسات** — عرض شجري لبنية الجلسة للتنقل السريع
- **لوحة الاقتباسات** — تتبع وعرض اقتباسات المصادر مع دعم تسجيل الموثوقية
- **عرض الرسوم البيانية المعلوماتية** — دعم العرض المرئي للرسوم البيانية المعلوماتية

### 🛡️ البيانات والأمان

- **تشفير AES-256** — مفاتيح API والبيانات الحساسة مشفرة بـ AES-256-GCM
- **تخزين معزول** — حالة التطبيق في `~/.axagent/`، ملفات المستخدم في `~/Documents/axagent/`
- **نسخ احتياطي تلقائي** — نسخ احتياطية مجدولة إلى دليل محلي أو تخزين WebDAV
- **استعادة النسخ الاحتياطي** — استعادة بنقرة واحدة من النسخ الاحتياطية التاريخية
- **خيارات التصدير** — لقطات PNG، Markdown، نص عادي، تنسيق JSON
- **إدارة التخزين** — عرض مرئي لاستخدام القرص وأدوات التنظيف
- **تفويض الملفات** — إدارة تفويض وإلغاء تفويض الوصول إلى الملفات
- **تدقيق العمليات** — تسجيل سجل تدقيق للعمليات الحرجة

### 🖥️ تجربة سطح المكتب

- **محرك السمات** — سمات داكنة/فاتحة مع دعم اتباع تفضيلات النظام أو الإعداد اليدوي
- **لغة الواجهة** — 11 لغة: الصينية المبسطة، الصينية التقليدية، الإنجليزية، اليابانية، الكورية، الفرنسية، الألمانية، الإسبانية، الروسية، الهندية، العربية
- **صينية النظام** — التصغير إلى صينية النظام دون مقاطعة الخدمات الخلفية
- **دائماً في المقدمة** — تثبيت النافذة فوق النوافذ الأخرى
- **اختصارات عامة** — اختصارات قابلة للتخصيص لاستدعاء النافذة الرئيسية
- **QuickBar** — شريط عائم للوصول السريع، استدعاء بنقرة واحدة
- **بدء تلقائي** — تشغيل اختياري عند بدء تشغيل النظام
- **دعم الوكيل** — تكوين وكيل HTTP وSOCKS5
- **تحديث تلقائي** — فحص تلقائي للإصدارات مع تنبيه عند توفر تحديثات
- **لوحة الأوامر** — `Cmd/Ctrl+K` للوصول سريع إلى الأوامر
- **معالج الإعداد** — إرشاد تفاعلي للاستخدام الأول واكتشاف Ollama
- **مركز الإشعارات** — إدارة إشعارات موحدة داخل التطبيق

### 🔬 ميزات متقدمة

- **البحث المتعمق** — بحث متعدد المصادر وتتبع الاقتباسات وتقييم الموثوقية وتوليف المحتوى
- **التحقق من الحقائق** — تحقق مدعوم بالذكاء الاصطناعي من الحقائق وتصنيف المصادر
- **مجدول Cron** — جدولة مهام تلقائية مع قوالب يومية/أسبوعية/شهرية ودعم تعبيرات cron المخصصة
- **نظام Webhook** — اشتراك الأحداث مع دعم إشعارات إكمال الأدوات وأخطاء الوكلاء وانتهاء الجلسات
- **ملف المستخدم** — تعلم تلقائي لأسلوب الكود واصطلاحات التسمية والمسافات البادئة وأسلوب التعليقات وتفضيلات التواصل
- **محسن RL** — تحسين التعلم المعزز لاختيار الأدوات واستراتيجيات المهام
- **ضبط LoRA الدقيق** — تكييف النماذج المخصصة باستخدام التدريب المحلي مع LoRA
- **اقتراحات استباقية** — مطالبات مدركة للسياق بناءً على محتوى المحادثة وأنماط المستخدم
- **توقع السياق** — توقع الإجراء التالي للمستخدم وجلب الموارد ذات الصلة مسبقاً
- **تكامل الأحلام** — تكامل تلقائي للذكريات والأنماط في الخلفية لتحسين المعرفة طويلة المدى
- **استعادة الأخطاء** — تصنيف تلقائي للأخطاء وتحليل السبب الجذري واقتراحات الاستعادة
- **أدوات المطور** — Trace وSpan وتصور الجدول الزمني لتصحيح الأخطاء وتحليل الأداء
- **نظام المعايير** — تقييم أداء مهام SWE-bench / Terminal-bench ومقاييس مع بطاقات تسجيل
- **نقل الأسلوب** — تطبيق تفضيلات أسلوب الكود المُتعلمة على الكود المُولّد
- **إضافات لوحة التحكم** — لوحة تحكم قابلة للتوسيع مع دعم ألواح وعناصر واجهة مستخدم مخصصة
- **المشاركة التعاونية** — تعاون في الوقت الفعلي عبر CRDT ومشاركة جلسات بنقرة واحدة
- **امتداد المتصفح** — امتداد متصفح Wiki Clipper، قص صفحات الويب بسرعة إلى LLM Wiki
- **Python SDK** — توفير Python SDK للتكامل مع AxAgent
- **الموجه الذكي** — توجيه وتصنيف الطلبات بذكاء
- **ذاكرة التخزين المؤقت الدلالية** — تخزين مؤقت للاستجابات قائم على الدلالات لتقليل الحسابات المتكررة
- **ضغط السياق** — ضغط تلقائي للسياقات الطويلة لتحسين استخدام الرموز
- **معالجة الرسائل الدفعية** — إرسال وتحسين الرسائل على دفعات
- **تجمع الاتصالات** — إدارة تجمع اتصالات قواعد البيانات وAPI
- **علامات الميزات** — نظام علامات ميزات قابل للتكوين
- **محرك السياسات** — إدارة مركزية لأذونات وسياسات العمليات
- **حاكم الموارد** — حدود وحوكمة استخدام موارد الوكيل
- **نقل LAN** — قدرة نقل الملفات عبر الشبكة المحلية

---

## البنية التقنية

### مجموعة التقنيات

| الطبقة | التقنية |
|--------|---------|
| **الإطار** | Tauri 2 + React 19 + TypeScript 6 |
| **واجهة المستخدم** | Ant Design 6 + TailwindCSS 4 |
| **إدارة الحالة** | Zustand 5 |
| **التوجيه** | React Router 7 |
| **التدويل** | i18next + react-i18next |
| **الواجهة الخلفية** | Rust + SeaORM 2 + SQLite |
| **قاعدة بيانات المتجهات** | sqlite-vec |
| **محرر الكود** | Monaco Editor |
| **المخططات** | Mermaid + D2 + ECharts (CDN) |
| **الطرفية** | xterm.js 6 |
| **سير العمل** | ReactFlow 11 |
| **البناء** | Vite 8 + npm |

### البنية الخلفية لـ Rust

الواجهة الخلفية منظّمة كمساحة عمل Rust مع 10 حزم متخصصة:

```
src-tauri/crates/
├── agent/         # جوهر وكيل AI
│   ├── react_engine.rs          # محرك استدلال ReAct
│   ├── coordinator.rs           # تنسيق الوكلاء
│   ├── hierarchical_planner.rs  # تفكيك المهام
│   ├── task_decomposer.rs       # تفكيك المهام الفرعية
│   ├── self_verifier.rs         # التحقق من المخرجات
│   ├── verification_agent.rs    # وكيل التحقق
│   ├── error_recovery_engine.rs # محرك استعادة الأخطاء
│   ├── error_classifier.rs      # تصنيف الأخطاء
│   ├── recovery_strategies.rs   # استراتيجيات الاستعادة
│   ├── loop_detector.rs         # اكتشاف الحلقات
│   ├── vision_pipeline.rs       # إدراك الشاشة
│   ├── deep_research.rs         # البحث المتعمق
│   ├── fact_checker.rs          # التحقق من الحقائق
│   ├── research_agent.rs        # وكيل البحث
│   ├── search_planner.rs        # تخطيط البحث
│   ├── search_orchestrator.rs   # تنسيق البحث
│   ├── academic_search.rs       # البحث الأكاديمي
│   ├── source_validator.rs      # التحقق من المصادر
│   ├── source_classifier.rs     # تصنيف المصادر
│   ├── credibility_evaluator.rs # تقييم الموثوقية
│   ├── citation_tracker.rs      # تتبع الاقتباسات
│   ├── content_synthesizer.rs   # توليف المحتوى
│   ├── outline_builder.rs       # بناء المخطط
│   ├── reference_builder.rs     # بناء المراجع
│   ├── proactive_mode.rs        # الوضع الاستباقي
│   ├── purpose_manager.rs       # إدارة الأغراض
│   ├── graph_insights.rs        # رؤى الرسم البياني
│   ├── insight_generator.rs     # توليد الرؤى
│   ├── schema_manager.rs        # إدارة Schema
│   ├── ingest_pipeline.rs       # خط أنابيب استيعاب البيانات
│   ├── session_manager.rs       # إدارة الجلسات
│   ├── health_checker.rs        # فحص الصحة
│   ├── metrics.rs               # جمع المقاييس
│   ├── evaluator/               # تقييم المعايير
│   ├── fine_tune/               # ضبط LoRA الدقيق
│   ├── rl_optimizer/            # تحسين استراتيجية RL
│   └── tool_recommender/        # محرك توصية الأدوات
│
├── core/          # الأدوات الأساسية
│   ├── db.rs                   # قاعدة بيانات SeaORM
│   ├── vector_store.rs         # تكامل sqlite-vec
│   ├── rag.rs                  # طبقة تجريد RAG
│   ├── hybrid_search.rs        # بحث متجه + FTS5
│   ├── recall_pipeline.rs      # خط أنابيب الاستدعاء ثلاثي المستويات
│   ├── crypto.rs               # تشفير AES-256
│   ├── mcp_client.rs           # عميل بروتوكول MCP
│   ├── browser_automation.rs   # أتمتة المتصفح
│   ├── computer_control.rs     # التحكم في الكمبيوتر
│   ├── screen_vision.rs        # رؤية الشاشة
│   ├── screen_capture.rs       # التقاط لقطات الشاشة
│   ├── ui_automation.rs        # أتمتة واجهة المستخدم
│   ├── ast_index.rs            # فهرس AST
│   ├── incremental_indexer.rs  # الفهرس التزايدي
│   ├── document_parser.rs      # محلل المستندات
│   ├── markdown_parser.rs      # محلل Markdown
│   ├── text_chunker.rs         # تقسيم النص
│   ├── token_counter.rs        # عداد الرموز
│   ├── token_budget.rs         # ميزانية الرموز
│   ├── file_index.rs           # فهرس الملفات
│   ├── file_authorizer.rs      # تفويض الملفات
│   ├── file_store.rs           # تخزين الملفات
│   ├── cache.rs                # إدارة التخزين المؤقت
│   ├── disk_cache.rs           # تخزين مؤقت للقرص
│   ├── cache_persister.rs      # استمرارية التخزين المؤقت
│   ├── cache_snapshot.rs       # لقطة التخزين المؤقت
│   ├── vector_cache.rs         # تخزين مؤقت للمتجهات
│   ├── marketplace_service.rs  # خدمة السوق
│   ├── marketplace.rs          # تجريد السوق
│   ├── operation_audit.rs      # تدقيق العمليات
│   ├── unified_config.rs       # تكوين موحد
│   ├── platform_config.rs      # تكوين المنصة
│   ├── command_validator.rs    # التحقق من الأوامر
│   ├── shell_parser.rs         # محلل Shell
│   ├── output_processor.rs     # معالج المخرجات
│   ├── storage_inventory.rs    # جرد التخزين
│   ├── storage_migration.rs    # ترحيل التخزين
│   ├── storage_paths.rs        # مسارات التخزين
│   ├── s3_backup.rs            # نسخ احتياطي S3
│   ├── webdav.rs               # مزامنة WebDAV
│   ├── git_tools.rs            # أدوات Git
│   ├── sandbox_runner.rs       # مشغل الصندوق الرملي
│   ├── search.rs               # تجريد البحث
│   ├── reranker.rs             # إعادة الترتيب
│   ├── model_knowledge.rs      # معرفة النماذج
│   ├── prompt_template.rs      # قالب المطالبات
│   ├── preset_templates.rs     # قوالب مسبقة
│   ├── workflow_types.rs       # أنواع سير العمل
│   ├── workflow_version.rs     # إصدار سير العمل
│   ├── path_vars.rs            # متغيرات المسار
│   ├── entity/                 # كيانات SeaORM (40+ جدول)
│   └── repo/                   # مستودعات البيانات (30+ مستودع)
│
├── gateway/       # بوابة API
│   ├── server.rs               # خادم HTTP
│   ├── handlers.rs             # معالجو API
│   ├── routes.rs               # تعريفات المسارات
│   ├── auth.rs                 # المصادقة
│   ├── middleware.rs           # البرمجيات الوسيطة
│   ├── metrics.rs              # جمع المقاييس
│   ├── native.rs               # التكامل الأصلي
│   ├── marketplace_handlers.rs # واجهة السوق
│   └── realtime.rs             # دعم WebSocket
│
├── plugins/       # نظام الإضافات
│   ├── hooks.rs                # مشغل الخطافات
│   ├── agent_provider.rs       # مزود الوكلاء
│   ├── test_isolation.rs       # عزل الاختبار
│   └── lib.rs                  # سجل الإضافات ودورة الحياة
│
├── providers/     # محولات النماذج
│   ├── adapter.rs              # واجهة المحول
│   ├── registry.rs             # سجل المزودين
│   ├── openai.rs               # واجهة OpenAI API
│   ├── openai_responses.rs     # واجهة استجابات OpenAI
│   ├── anthropic.rs            # واجهة Claude API
│   ├── gemini.rs               # واجهة Gemini API
│   ├── ollama.rs               # Ollama محلي
│   ├── openclaw.rs             # OpenClaw
│   ├── hermes.rs               # Hermes
│   ├── image_gen.rs            # توليد الصور
│   ├── realtime_client.rs      # عميل API في الوقت الفعلي
│   └── transport/              # طبقة النقل (Chat Completions / Responses / Anthropic)
│
├── runtime/       # خدمات وقت التشغيل
│   ├── session.rs              # إدارة الجلسات
│   ├── workflow_engine.rs      # تنسيق DAG
│   ├── work_engine/            # محرك العمل (منفذو العقد + المجدول + طبقة التخزين المؤقت)
│   ├── mcp.rs                  # خادم MCP
│   ├── mcp_client.rs           # عميل MCP
│   ├── mcp_server.rs           # تنفيذ خادم MCP
│   ├── mcp_stdio.rs            # نقل MCP stdio
│   ├── mcp_autostart.rs        # بدء MCP التلقائي
│   ├── mcp_lifecycle_hardened.rs # إدارة دورة حياة MCP
│   ├── mcp_tool_bridge.rs      # جسر أدوات MCP
│   ├── cron/                   # جدولة المهام
│   ├── terminal/               # واجهة الطرفية الخلفية (محلي/Docker/SSH)
│   ├── benchmarks/             # SWE-bench / Terminal-bench
│   ├── collaboration/          # تعاون CRDT ومشاركة الجلسات
│   ├── tool_generator/         # توليد أدوات AI
│   ├── message_gateway/        # تكامل المنصات (DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
│   ├── buddy/                  # نظام رفيق Buddy (أنواع/سمات/مدير)
│   ├── swarm/                  # مجموعة Swarm (واجهة خلفية للعمليات/مزامنة الأذونات/إعادة الاتصال)
│   ├── tasks/                  # مهام الخلفية (أحلام/وكلاء بعيدون/زملاء في العملية)
│   ├── adversarial_debate.rs   # النقاش المضاد
│   ├── agent_orchestrator.rs   # منسق الوكلاء المتعددين
│   ├── agent_roles.rs          # أدوار الوكلاء
│   ├── webhook_dispatcher.rs   # مرسل Webhook
│   ├── webhook_server.rs       # خادم Webhook
│   ├── session_search.rs       # بحث الجلسات
│   ├── dashboard_plugin.rs     # إضافة لوحة التحكم
│   ├── dashboard_registry.rs   # سجل لوحة التحكم
│   ├── permissions.rs          # إدارة الأذونات
│   ├── permission_enforcer.rs  # إنفاذ الأذونات
│   ├── policy_engine.rs        # محرك السياسات
│   ├── trust_resolver.rs       # محلل الثقة
│   ├── resource_governor.rs    # حاكم الموارد
│   ├── green_contract.rs       # العقد الأخضر
│   ├── feature_flags.rs        # علامات الميزات
│   ├── module_switch.rs        # مفتاح الوحدات
│   ├── mode_selector.rs        # محدد الوضع
│   ├── config.rs               # تكوين وقت التشغيل
│   ├── config_validate.rs      # التحقق من التكوين
│   ├── prompt.rs               # إدارة المطالبات
│   ├── prompt_cache.rs         # تخزين مؤقت للمطالبات
│   ├── compact.rs              # ضغط السياق
│   ├── summary_compression.rs  # ضغط الملخصات
│   ├── compact_thresholds.rs   # عتبات الضغط
│   ├── compact_warning.rs      # تحذير الضغط
│   ├── reactive_compact.rs     # ضغط تفاعلي
│   ├── session_memory_compact.rs # ضغط ذاكرة الجلسة
│   ├── message_importance.rs   # تقييم أهمية الرسائل
│   ├── message_batching.rs     # معالجة الرسائل الدفعية
│   ├── rate_limiter.rs         # محدد المعدل
│   ├── connection_pool.rs      # تجمع الاتصالات
│   ├── persistent_queue.rs     # طابور دائم
│   ├── persistent_queue_manager.rs # مدير الطابور
│   ├── health_check.rs         # فحص الصحة
│   ├── cache_guard.rs          # حارس التخزين المؤقت
│   ├── checkpoint.rs           # نقطة تفتيش
│   ├── branch_lock.rs          # قفل الفرع
│   ├── stale_base.rs           # اكتشاف الأساس القديم
│   ├── watch_patterns.rs       # أنماط المراقبة
│   ├── lan_transfer.rs         # نقل LAN
│   ├── tls_config.rs           # تكوين TLS
│   ├── sse.rs                  # تدفق أحداث SSE
│   ├── api_server.rs           # خادم API
│   ├── gateway_auth.rs         # مصادقة البوابة
│   ├── gateway_metrics.rs      # مقاييس البوابة
│   ├── bash.rs                 # تنفيذ Bash
│   ├── bash_validation.rs      # التحقق من Bash
│   ├── shell_hooks.rs          # خطافات Shell
│   ├── shell_completer.rs      # مكمل Shell
│   ├── terminal_analyzer.rs    # محلل الطرفية
│   ├── git_context.rs          # سياق Git
│   ├── git_tools.rs            # أدوات Git
│   ├── file_ops.rs             # عمليات الملفات
│   ├── hooks.rs                # إدارة الخطافات
│   ├── hook_chain.rs           # سلسلة الخطافات
│   ├── hook_config.rs          # تكوين الخطافات
│   ├── plugin_hooks.rs         # خطافات الإضافات
│   ├── plugin_lifecycle.rs     # دورة حياة الإضافات
│   ├── profile.rs              # الملف الشخصي
│   ├── profile_manager.rs      # مدير الملف الشخصي
│   ├── oauth.rs                # مصادقة OAuth
│   ├── usage.rs                # إحصائيات الاستخدام
│   ├── bootstrap.rs            # التشغيل الأولي
│   ├── worker_boot.rs          # تشغيل Worker
│   ├── fork_bridge.rs          # جسر Fork
│   ├── task_packet.rs          # حزمة المهام
│   ├── task_router.rs          # موجه المهام
│   ├── task_registry.rs        # سجل المهام
│   ├── transform_pipeline.rs   # خط أنابيب التحويل
│   ├── transport_handlers.rs   # معالجات النقل
│   ├── general_engine.rs       # المحرك العام
│   ├── engine_bridge.rs        # جسر المحرك
│   ├── conversation.rs         # إدارة المحادثات
│   ├── session_control.rs      # التحكم في الجلسة
│   ├── shared_memory.rs        # الذاكرة المشتركة
│   ├── validation_executor.rs  # منفذ التحقق
│   ├── recovery_recipes.rs     # وصفات الاستعادة
│   ├── error_recovery.rs       # استعادة الأخطاء
│   ├── theme_engine.rs         # محرك السمات
│   ├── token_budget_predictor.rs # توقع ميزانية الرموز
│   ├── team_cron_registry.rs   # تسجيل Cron للفريق
│   ├── module_dream.rs         # وحدة الأحلام
│   ├── json.rs                 # أدوات JSON
│   └── lane_events.rs          # أحداث Lane
│
├── telemetry/     # القياس عن بعد والتتبع
│   ├── tracer.rs              # التتبع الموزع
│   ├── metrics.rs             # جمع المقاييس
│   ├── span.rs                # إدارة Span
│   ├── event.rs               # تعريفات الأحداث
│   ├── collector.rs           # جمع البيانات
│   ├── exporter.rs            # تصدير البيانات
│   └── storage.rs             # واجهة التخزين الخلفية
│
├── tools/         # نظام الأدوات
│   ├── registry.rs             # سجل الأدوات
│   ├── builtin_tools.rs        # تعريفات الأدوات المدمجة
│   ├── builtin_handlers.rs     # معالجات الأدوات المدمجة
│   ├── orchestration.rs        # تنسيق الأدوات
│   ├── streaming.rs            # الإخراج المتدفق
│   ├── stats.rs                # إحصائيات الاستخدام
│   ├── recorder.rs             # تسجيل التنفيذ
│   ├── agent_def_loader.rs     # محمل تعريف الوكيل
│   ├── agent_def_types.rs      # أنواع تعريف الوكيل
│   ├── bash/                   # أداة Bash (محلل/صندوق رمل/أمان/تحقق من المسار)
│   ├── hooks/                  # خطافات (سجل/منفذ)
│   ├── mcp/                    # أدوات MCP (سجل/OAuth/غلاف)
│   ├── permissions/            # أذونات (مصنف/قواعد/متتبع)
│   └── tools/                  # تنفيذ أدوات محددة
│       ├── agent.rs            # أداة الوكيل
│       ├── bash.rs             # تنفيذ Bash
│       ├── context.rs          # إدارة السياق
│       ├── cron.rs             # جدولة Cron
│       ├── glob.rs             # مطابقة ملفات glob
│       ├── grep.rs             # بحث المحتوى
│       ├── lsp.rs              # أداة LSP
│       ├── monitor.rs          # أداة المراقبة
│       ├── plan.rs             # أداة الخطة
│       ├── repl.rs             # أداة REPL
│       ├── skill.rs            # أداة المهارة
│       ├── web_fetch.rs        # جلب الويب
│       ├── web_search.rs       # بحث الويب
│       ├── file_read.rs        # قراءة الملفات
│       ├── file_write.rs       # كتابة الملفات
│       ├── file_edit.rs        # تحرير الملفات
│       ├── computer_use.rs     # التحكم في الكمبيوتر
│       ├── messaging.rs        # إرسال الرسائل
│       ├── push_notification.rs # إشعارات الدفع
│       ├── task_system.rs      # نظام المهام
│       ├── todo_write.rs       # كتابة قوائم المهام
│       └── batch_missing.rs    # اكتشاف الدفعات المفقودة
│
├── trajectory/    # نظام التعلم
│   ├── memory.rs              # إدارة الذاكرة
│   ├── memory_provider.rs     # واجهة مزود الذاكرة
│   ├── auto_memory.rs         # استخراج الذاكرة التلقائي
│   ├── skill.rs               # نظام المهارات
│   ├── skill_manager.rs       # مدير المهارات
│   ├── skill_evolution.rs     # تطور المهارات
│   ├── skill_matcher.rs       # مطابقة المهارات
│   ├── skill_proposal.rs      # مقترحات المهارات
│   ├── skills_hub_adapter.rs  # محول مركز المهارات
│   ├── skills_hub_client.rs   # عميل مركز المهارات
│   ├── skill_decomposition/   # تفكيك المهارات (مساعدة LLM/متعدد الجولات/تحقق سير العمل/تحليل الأدوات)
│   ├── rl.rs                  # إشارات مكافأة RL
│   ├── rl_trainer.rs          # مدرب RL
│   ├── training_env.rs        # بيئة التدريب
│   ├── behavior_learner.rs    # تعلم السلوك
│   ├── behavior_tracker.rs    # تتبع السلوك
│   ├── pattern.rs             # التعرف على الأنماط
│   ├── pattern_analyzer.rs    # تحليل الأنماط
│   ├── user_profile.rs        # ملف المستخدم
│   ├── preference_learner.rs  # تعلم التفضيلات
│   ├── adaptation.rs          # التكيف
│   ├── dream_consolidation.rs # تكامل الأحلام
│   ├── parallel_execution.rs  # خدمة التنفيذ المتوازي
│   ├── style_extractor.rs     # استخراج الأسلوب
│   ├── style_applier.rs       # تطبيق الأسلوب
│   ├── style_vectorizer.rs    # تحويل الأسلوب إلى متجهات
│   ├── style_migrator.rs      # نقل الأسلوب
│   ├── suggestion_engine.rs   # محرك الاقتراحات
│   ├── proactive_assistant.rs # المساعد الاستباقي
│   ├── context_predictor.rs   # توقع السياق
│   ├── task_prefetcher.rs     # جلب المهام المسبق
│   ├── reminder_manager.rs    # إدارة التذكيرات
│   ├── nudge.rs               # نظام التنبيهات اللطيفة
│   ├── insight.rs             # توليد الرؤى
│   ├── compactor.rs           # ضغط البيانات
│   ├── trajectory.rs          # إدارة المسار
│   ├── trajectory_compressor.rs # ضغط المسار
│   ├── sub_agent.rs           # وكيل فرعي
│   ├── batch.rs               # معالجة الدفعات
│   ├── context.rs             # إدارة السياق
│   ├── fts5.rs                # بحث FTS5
│   ├── hooks.rs               # خطافات
│   ├── storage.rs             # التخزين
│   ├── scheduled_task.rs      # المهام المجدولة
│   └── memory_providers/      # مزودو الذاكرة (Honcho/Mem0/حلقة مغلقة/خدمات)
│
└── migration/     # ترحيل قاعدة البيانات
    └── m20240101_000001~000010  # 10 ملفات ترحيل
```

### البنية الأمامية

```
src/
├── stores/                    # إدارة حالة Zustand
│   ├── domain/               # حالة الأعمال الأساسية
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # حالة الوحدات الوظيفية (30+ مخزن)
│   │   ├── agentStore.ts
│   │   ├── agentProfileStore.ts
│   │   ├── appConfigStore.ts
│   │   ├── backupStore.ts
│   │   ├── buddyStore.ts
│   │   ├── categoryStore.ts
│   │   ├── decompositionStore.ts
│   │   ├── dreamStore.ts
│   │   ├── executionStore.ts
│   │   ├── expertStore.ts
│   │   ├── fileStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── gatewayLinkStore.ts
│   │   ├── generatedToolStore.ts
│   │   ├── helpStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── llmWikiStore.ts
│   │   ├── localToolStore.ts
│   │   ├── memoryStore.ts
│   │   ├── mcpStore.ts
│   │   ├── nudgeStore.ts
│   │   ├── onboardingStore.ts
│   │   ├── planStore.ts
│   │   ├── platformStore.ts
│   │   ├── proactiveStore.ts
│   │   ├── promptTemplateStore.ts
│   │   ├── providerStore.ts
│   │   ├── searchStore.ts
│   │   ├── settingsStore.ts
│   │   ├── skillExtensionStore.ts
│   │   ├── skillStore.ts
│   │   ├── styleStore.ts
│   │   ├── terminalStore.ts
│   │   ├── themeStore.ts
│   │   ├── trajectoryStore.ts
│   │   ├── userProfileStore.ts
│   │   ├── wikiStore.ts
│   │   ├── workEngineStore.ts
│   │   └── workflowEditorStore.ts
│   ├── devtools/              # حالة أدوات المطور
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # الحالة المشتركة
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # مكونات React (24 وحدة)
│   ├── chat/                # واجهة المحادثة (90+ مكون)
│   ├── workflow/            # محرر سير العمل (عقد/لوحات/قوالب/مساعدة AI)
│   ├── gateway/             # واجهة بوابة API
│   ├── settings/            # لوحة الإعدادات (40+ مكون)
│   ├── terminal/            # واجهة الطرفية
│   ├── skill/               # محرر وعارض المهارات
│   ├── benchmark/           # لوحة المعايير
│   ├── decomposition/       # تفكيك المهارات وتوليد الأدوات
│   ├── files/               # صفحة إدارة الملفات
│   ├── fine-tune/           # تكوين ضبط LoRA الدقيق
│   ├── link/                # إدارة الروابط الخارجية
│   ├── llm-wiki/            # محرر LLM Wiki
│   ├── proactive/           # نظام الاقتراحات الاستباقية
│   ├── recommendation/      # لوحة توصية الأدوات
│   ├── wiki/                # إدارة Wiki
│   ├── devtools/            # جدول زمني لـ Trace/Span
│   ├── style/               # نقل أسلوب الكود
│   ├── layout/              # مكونات التخطيط (شريط العنوان/الشريط الجانبي/لوحة الأوامر)
│   ├── help/                # لوحة المساعدة
│   ├── onboarding/          # معالج الإعداد
│   ├── notification/        # مركز الإشعارات
│   ├── search/              # بحث الجلسات
│   ├── common/              # مكونات مشتركة
│   └── shared/              # مكونات مشاركة
│
├── pages/                    # مكونات الصفحات (22 صفحة)
│   ├── ChatPage.tsx
│   ├── KnowledgePage.tsx
│   ├── KnowledgeHubPage.tsx
│   ├── MemoryPage.tsx
│   ├── WorkflowPage.tsx
│   ├── WorkflowMarketplace.tsx
│   ├── GatewayPage.tsx
│   ├── GatewayLinkPage.tsx
│   ├── LinkPage.tsx
│   ├── FilesPage.tsx
│   ├── FineTunePage.tsx
│   ├── SkillsPage.tsx
│   ├── WikiPage.tsx
│   ├── WikiEditorPage.tsx
│   ├── WikiGraphPage.tsx
│   ├── LlmWikiPage.tsx
│   ├── LlmWikiEditorPage.tsx
│   ├── IngestPage.tsx
│   ├── QuickBarPage.tsx
│   ├── SettingsPage.tsx
│   └── DevTools/
│       ├── TraceExplorer.tsx
│       ├── BenchmarkRunner.tsx
│       └── ToolRecommender.tsx
│
├── hooks/                    # خطافات React (10)
├── lib/                      # دوال مساعدة (بما في ذلك Web Worker)
├── types/                    # تعريفات أنواع TypeScript (22)
├── sdk/                      # SDK (بما في ذلك Python SDK)
└── i18n/                     # ترجمات 11 لغة
```

### دعم المنصات

| المنصة | البنية |
|--------|--------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

## البدء السريع

### تنزيل الإصدارات المبنية مسبقاً

انتقل إلى صفحة [Releases](https://github.com/polite0803/AxAgent/releases) وقم بتنزيل المثبّت الخاص بمنصتك.

### البناء من المصدر

#### المتطلبات الأساسية

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + Rust MSVC targets

#### خطوات البناء

```bash
# استنساخ المستودع
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# تثبيت التبعيات
npm install

# وضع التطوير
npm run tauri dev

# بناء الواجهة الأمامية فقط
npm run build

# بناء تطبيق سطح المكتب
npm run tauri build
```

توجد مخرجات البناء في `src-tauri/target/release/`.

### الاختبارات

```bash
# اختبارات الوحدة
npm run test

# اختبارات E2E
npm run test:e2e

# فحص الأنواع
npm run typecheck

# تنسيق الكود
npm run format

# فحص CI
npm run ci:check
```

---

## هيكل المشروع

```
AxAgent/
├── src/                         # مصدر الواجهة الأمامية (React + TypeScript)
│   ├── components/              # مكونات React (24 وحدة)
│   │   ├── chat/               # واجهة المحادثة (90+ مكون)
│   │   ├── workflow/           # مكونات محرر سير العمل
│   │   ├── gateway/            # مكونات بوابة API
│   │   ├── settings/           # لوحة الإعدادات (40+ مكون)
│   │   ├── terminal/           # مكونات الطرفية
│   │   ├── skill/              # محرر وعارض المهارات
│   │   ├── benchmark/          # المعايير
│   │   ├── decomposition/      # تفكيك المهارات
│   │   ├── files/              # إدارة الملفات
│   │   ├── fine-tune/          # ضبط LoRA الدقيق
│   │   ├── link/               # الروابط الخارجية
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # اقتراحات استباقية
│   │   ├── recommendation/     # توصية الأدوات
│   │   ├── wiki/               # إدارة Wiki
│   │   ├── devtools/           # أدوات المطور
│   │   ├── style/              # أسلوب الكود
│   │   ├── layout/             # مكونات التخطيط
│   │   ├── help/               # لوحة المساعدة
│   │   ├── onboarding/         # معالج الإعداد
│   │   ├── notification/       # مركز الإشعارات
│   │   ├── search/             # بحث الجلسات
│   │   ├── common/             # مكونات مشتركة
│   │   └── shared/             # مكونات مشاركة
│   ├── pages/                   # مكونات الصفحات (22 صفحة)
│   ├── stores/                  # إدارة حالة Zustand
│   │   ├── domain/            # حالة الأعمال الأساسية (6 مخازن)
│   │   ├── feature/           # حالة الوحدات الوظيفية (30+ مخزن)
│   │   ├── devtools/          # حالة أدوات المطور (5 مخازن)
│   │   └── shared/            # الحالة المشتركة (4 مخازن)
│   ├── hooks/                   # خطافات React (10)
│   ├── lib/                     # دوال مساعدة (بما في ذلك Web Worker)
│   ├── types/                   # تعريفات أنواع TypeScript (22)
│   ├── sdk/                     # SDK (بما في ذلك Python SDK)
│   └── i18n/                    # ترجمات 11 لغة
│
├── src-tauri/                    # مصدر الواجهة الخلفية (Rust)
│   ├── crates/                  # مساحة عمل Rust (10 حزم)
│   │   ├── agent/             # جوهر وكيل AI
│   │   ├── core/              # قاعدة البيانات والتشفير وRAG
│   │   ├── gateway/           # خادم بوابة API
│   │   ├── plugins/           # نظام الإضافات
│   │   ├── providers/         # محولات مزودي النماذج
│   │   ├── runtime/           # خدمات وقت التشغيل
│   │   ├── tools/             # نظام الأدوات
│   │   ├── trajectory/        # الذاكرة والتعلم
│   │   ├── telemetry/         # التتبع والمقاييس
│   │   └── migration/         # ترحيل قاعدة البيانات
│   └── src/                    # نقطة دخول Tauri (70+ وحدة أوامر)
│
├── extension/                  # امتداد المتصفح (Wiki Clipper)
├── e2e/                        # اختبارات Playwright E2E
├── scripts/                    # نصوص البناء والأدوات
└── website/                    # موقع المشروع (VitePress)
```

## دليل البيانات

```
~/.axagent/                      # دليل التكوين
├── axagent.db                   # قاعدة بيانات SQLite
├── master.key                   # مفتاح AES-256 الرئيسي
├── vector_db/                   # قاعدة بيانات المتجهات (sqlite-vec)
└── ssl/                         # شهادات SSL

~/Documents/axagent/            # دليل ملفات المستخدم
├── images/                     # مرفقات الصور
├── files/                      # مرفقات الملفات
└── backups/                    # ملفات النسخ الاحتياطي
```

---

## الأسئلة الشائعة

### macOS: «التطبيق تالف» أو «لا يمكن التحقق من المطور»

نظراً لأن التطبيق غير موقّع من Apple:

**1. السماح بالتطبيقات من «أي مكان»**
```bash
sudo spctl --master-disable
```

ثم انتقل إلى **إعدادات النظام ← الخصوصية والأمان ← الأمان** وحدد **أي مكان**.

**2. إزالة سمة الحجر الصحي**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. خطوة إضافية لـ macOS Ventura+**
انتقل إلى **إعدادات النظام ← الخصوصية والأمان**، وانقر على **فتح على أي حال**.

---

## المجتمع

- [LinuxDO](https://linux.do)

## الترخيص

هذا المشروع مرخّص بموجب ترخيص [AGPL-3.0](LICENSE).
