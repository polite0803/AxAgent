# بوابة API

## ما هي بوابة API؟

يتضمن AxAgent خادم API محلي مدمجاً يكشف مزوّديك المُكوَّنين كطرف **متوافق مع OpenAI** و**Claude الأصلي** و**Gemini الأصلي**. أي أداة أو عميل يستخدم أحد هذه البروتوكولات يمكنه استخدام AxAgent كخلفية — دون مفاتيح API منفصلة أو خدمات وسيطة مطلوبة.

حالات الاستخدام:

- تشغيل **Claude Code CLI** أو **OpenAI Codex CLI** أو **Gemini CLI** أو **OpenCode** عبر AxAgent.
- تغذية امتدادات IDE الخاصة بك بطرف واحد مُدار محلياً.
- مشاركة مجموعة من مفاتيح المزوّد عبر أدوات كثيرة مع تحديد معدل لكل مفتاح.

---

## البدء

1. افتح **الإعدادات ← بوابة API** (أو اضغط <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>).
2. انقر **ابدأ** لتشغيل خادم البوابة.
3. بشكل افتراضي، يستمع الخادم على `127.1.0.0:8080` (HTTP).

::: tip
فعّل **البدء التلقائي** في إعدادات البوابة لتشغيل الخادم تلقائياً عند إطلاق AxAgent.
:::

---

## إدارة مفاتيح API

1. اذهب إلى تبويب **مفاتيح API**.
2. انقر **إنشاء مفتاح جديد**.
3. أضف اختيارياً **وصفاً** لتعريف كل مفتاح.
4. انسخ المفتاح — يُعرض مرة واحدة فقط.

---

## قوالب التكوين

### Claude Code CLI

```bash
claude config set --global apiUrl http://127.1.0.0:8080
claude config set --global apiKey axagent-xxxx
```

### OpenAI Codex CLI

```bash
export OPENAI_BASE_URL=http://127.1.0.0:8080/v1
export OPENAI_API_KEY=axagent-xxxx
codex
```

### Gemini CLI

```bash
export GEMINI_API_BASE=http://127.1.0.0:8080
export GEMINI_API_KEY=axagent-xxxx
gemini
```

### عميل مخصص

```
Base URL:  http://127.1.0.0:8080/v1
API Key:   axagent-xxxx
```

---

## الخطوات التالية

- [البدء السريع](./getting-started) — العودة إلى دليل البدء السريع
- [إعداد المزوّدين](./providers) — إضافة المزوّدين الأولين الذين توجّه إليهم البوابة
- [خوادم MCP](./mcp) — توصيل أدوات خارجية لاستدعاء أدوات AI
