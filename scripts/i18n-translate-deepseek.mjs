#!/usr/bin/env node
/**
 * i18n 跨语言 key 翻译管线（DeepSeek）。
 * 源语言统一为 en-US（仓库约定）。对每个目标语言 L（≠ en-US）：
 *   - 当前值 == en-US  → 未翻译英文，需翻译
 *   - 当前值 == zh-CN 且 != en-US → 中文泄漏（含繁体中文文件里的简体中文），需翻译
 * 翻译 en-US 值 → L，保留 {{var}} / {var} / %s / %1$s / \n / <tag>，不翻译品牌专有名词。
 * 支持 --lang <code> 仅处理一种语言；支持 --no-dprint 跳过格式化。
 */
import { spawnSync } from "node:child_process";
import { execSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";

const KEY = process.env.DEEPSEEK_API_KEY;
const BASE = "https://api.deepseek.com/v1/chat/completions";
const MODEL = "deepseek-chat"; // 服务端解析为 deepseek-v4-flash
const LOCALES_DIR = "src/i18n/locales";
const LANGS = ["zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "ru", "hi", "ar"];
const LANG_NAME = {
  "zh-CN": "Simplified Chinese (简体中文)",
  "zh-TW": "Traditional Chinese (繁體中文)",
  ja: "Japanese (日本語)",
  ko: "Korean (한국어)",
  fr: "French (Français)",
  de: "German (Deutsch)",
  es: "Spanish (Español)",
  ru: "Russian (Русский)",
  hi: "Hindi (हिन्दी)",
  ar: "Arabic (العربية)",
};

const args = process.argv.slice(2);
const onlyLang = args.find((a) => a === "--lang") ? args[args.indexOf("--lang") + 1] : null;
const noDprint = args.includes("--no-dprint");

function getAllLeafPaths(obj, prefix = "") {
  const entries = [];
  for (const k of Object.keys(obj || {})) {
    const fullKey = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] !== null && !Array.isArray(obj[k])) {
      entries.push(...getAllLeafPaths(obj[k], fullKey));
    } else {
      entries.push({ key: fullKey, value: obj[k] });
    }
  }
  return entries;
}
function setValueByPath(obj, path, value) {
  const parts = path.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]] || typeof cur[parts[i]] !== "object") { cur[parts[i]] = {}; }
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}
function isPurePlaceholder(s) {
  const t = s
    .replace(/\{\{[^}]*\}\}/g, "")
    .replace(/\{[^}]*\}/g, "")
    .replace(/%[0-9]*\$?s/g, "")
    .replace(/\\n/g, "")
    .replace(/[0-9\s\.\,\;\:\!\?\-\(\)\[\]\'\"]/g, "");
  return t.length === 0;
}

// ---- backup originals ----
const ts = new Date().toISOString().replace(/[:.]/g, "-");
const backupDir = `output/i18n_backup_${ts}`;
mkdirSync(backupDir, { recursive: true });
for (const l of LANGS) { cpSync(`${LOCALES_DIR}/${l}.json`, `${backupDir}/${l}.json`); }
console.log(`Backed up originals -> ${backupDir}`);

// ---- load en-US + zh-CN as references ----
const enData = JSON.parse(readFileSync(`${LOCALES_DIR}/en-US.json`, "utf-8"));
const zhData = JSON.parse(readFileSync(`${LOCALES_DIR}/zh-CN.json`, "utf-8"));
const enMap = Object.fromEntries(getAllLeafPaths(enData).map((e) => [e.key, e.value]));
const zhMap = Object.fromEntries(getAllLeafPaths(zhData).map((e) => [e.key, e.value]));

// ---- translation API ----
async function translateBatch(srcs, langName) {
  const sys = `You are a professional UI string localizer. Translate the given English UI strings into ${langName}.
Rules:
- Return ONLY valid JSON. Use the form {"0":"...","1":"..."} mapping each numeric index key (0..N-1) to its translation, in the SAME order as the input.
- Keep ALL placeholders exactly as-is: {{var}}, {var}, %s, %1$s, and literal \\n. Keep HTML tags like <b></b>, <a>, <code> intact.
- Do NOT translate brand or proper nouns: AxAgent, MCP, DeepSeek, OpenAI, ChatGPT, JSON, API, URL, RAG, ReAct, Token, Claude, GPT, Workflow (when used as a product term), Tauri.
- If a string is already correct in the target language or is purely a symbol/placeholder, return it unchanged.
- Output strictly valid JSON, no markdown fences, no extra text.`;
  const payload = { translations: Object.fromEntries(srcs.map((s, i) => [String(i), s])) };
  let lastRaw = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const res = await fetch(BASE, {
        method: "POST",
        headers: { "Content-Type": "application/json", Authorization: `Bearer ${KEY}` },
        body: JSON.stringify({
          model: MODEL,
          messages: [
            { role: "system", content: sys },
            { role: "user", content: JSON.stringify(payload) },
          ],
          temperature: 0.2,
          response_format: { type: "json_object" },
        }),
      });
      if (res.status !== 200) {
        lastRaw = await res.text();
        if (attempt < 2) {
          await sleep(1000 * (attempt + 1));
          continue;
        }
        throw new Error(`HTTP ${res.status}: ${lastRaw.slice(0, 200)}`);
      }
      lastRaw = await res.text();
      let j = null;
      try {
        const env = JSON.parse(lastRaw);
        // 形态 A：完整 chat.completion 信封
        let content = env?.choices?.[0]?.message?.content;
        if (typeof content !== "string") { content = lastRaw; // 形态 B：raw 本身就是翻译对象
         }
        let c = String(content).trim();
        // 剥离 ```json ... ``` 围栏
        const fence = c.match(/```(?:json)?\s*([\s\S]*?)```/i);
        if (fence) { c = fence[1].trim(); }
        try {
          j = JSON.parse(c);
        } catch {
          // 提取第一个 {...} 块（容忍前后多余文本）
          const m = c.match(/\{[\s\S]*\}/);
          if (m) { j = JSON.parse(m[0]); }
        }
      } catch {
        j = null;
      }
      if (j === null) {
        throw new Error(`no content object | raw=${lastRaw.slice(0, 300)}`);
      }
      // 将任意形态规整为与输入等长的数组
      function extractArr(obj) {
        if (Array.isArray(obj)) { return obj; }
        if (obj && typeof obj === "object") {
          const pickObj = (o) => {
            const ks = Object.keys(o).filter((k) => /^\d+$/.test(k)).sort((a, b) => Number(a) - Number(b));
            return ks.length ? ks.map((k) => o[k]) : null;
          };
          if (Array.isArray(obj.translations)) { return obj.translations; }
          if (obj.translations && typeof obj.translations === "object") {
            const r = pickObj(obj.translations);
            if (r) { return r; }
          }
          if (Array.isArray(obj.items)) { return obj.items; }
          if (obj.items && typeof obj.items === "object") {
            const r = pickObj(obj.items);
            if (r) { return r; }
          }
          const r = pickObj(obj);
          if (r) { return r; }
        }
        return null;
      }
      const arr = extractArr(j);
      if (!arr || arr.length !== srcs.length) {
        throw new Error(`bad shape: got ${arr?.length}, want ${srcs.length} | raw=${lastRaw.slice(0, 300)}`);
      }
      return arr;
    } catch (e) {
      if (attempt < 2) {
        await sleep(1000 * (attempt + 1));
        continue;
      }
      throw e;
    }
  }
}
function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

// ---- concurrency pool ----
async function mapPool(items, worker, concurrency) {
  const results = new Array(items.length);
  let idx = 0;
  async function run() {
    while (idx < items.length) {
      const cur = idx++;
      try {
        results[cur] = await worker(items[cur]);
      } catch (e) {
        results[cur] = { error: String(e) };
      }
    }
  }
  const pool = Array.from({ length: Math.min(concurrency, items.length) }, run);
  await Promise.all(pool);
  return results;
}

const langsToDo = onlyLang ? [onlyLang] : LANGS;
const BATCH = 40;
const CONC = 5;

for (const lang of langsToDo) {
  const data = JSON.parse(readFileSync(`${LOCALES_DIR}/${lang}.json`, "utf-8"));
  const entries = getAllLeafPaths(data);
  const enEntries = getAllLeafPaths(enData);
  const enKeySet = new Set(enEntries.map((e) => e.key));
  const toTranslate = []; // {key, src}
  const seen = new Set();
  for (const { key, value } of entries) {
    if (typeof value !== "string") { continue; }
    const enVal = enMap[key];
    const zhVal = zhMap[key];
    const isUntranslatedEn = enVal !== undefined && value === enVal;
    const isChineseLeak = zhVal !== undefined && value === zhVal && value !== enVal && lang !== "zh-CN";
    if (isUntranslatedEn || isChineseLeak) {
      const src = lang === "zh-CN" && isChineseLeak ? zhVal : enVal;
      if (src === undefined || typeof src !== "string") { continue; }
      if (isPurePlaceholder(src)) {
        setValueByPath(data, key, src);
        continue;
      }
      const dedupe = `${key}`; // key-level; but we dedup by src below
      toTranslate.push({ key, src });
    }
  }
  // dedup by src to minimize API calls
  const srcToKeys = new Map();
  for (const t of toTranslate) {
    if (!srcToKeys.has(t.src)) { srcToKeys.set(t.src, []); }
    srcToKeys.get(t.src).push(t.key);
  }
  const uniqueSrcs = [...srcToKeys.keys()];
  console.log(`\n[${lang}] keys=${toTranslate.length}, uniqueStrings=${uniqueSrcs.length}`);

  const batches = [];
  for (let i = 0; i < uniqueSrcs.length; i += BATCH) { batches.push(uniqueSrcs.slice(i, i + BATCH)); }

  let done = 0;
  const translated = await mapPool(batches, async (batch) => {
    try {
      const out = await translateBatch(batch, LANG_NAME[lang]);
      done += batch.length;
      if (done % 200 < BATCH) { console.log(`  ${lang} progress: ${done}/${uniqueSrcs.length}`); }
      return out;
    } catch (e) {
      console.error(`  [${lang}] BATCH ERROR:`, e && e.message ? e.message : e);
      throw e;
    }
  }, CONC);

  const failures = [];
  translated.forEach((res, bi) => {
    const batch = batches[bi];
    if (res && res.error) {
      for (const s of batch) { for (const k of srcToKeys.get(s)) { failures.push(k); } }
      return;
    }
    batch.forEach((src, i) => {
      const tr = res[i];
      if (typeof tr !== "string") {
        for (const k of srcToKeys.get(src)) { failures.push(k); }
        return;
      }
      for (const k of srcToKeys.get(src)) { setValueByPath(data, k, tr); }
    });
  });

  writeFileSync(`${LOCALES_DIR}/${lang}.json`, JSON.stringify(data, null, 2) + "\n", "utf-8");
  console.log(`[${lang}] written. failures=${failures.length}`);
  if (failures.length) {
    writeFileSync(`output/i18n_failures_${lang}.json`, JSON.stringify(failures, null, 2), "utf-8");
  }

  if (!noDprint) {
    try {
      const r = spawnSync("npx", ["--no-install", "dprint", "fmt", `${LOCALES_DIR}/${lang}.json`], {
        shell: true,
        encoding: "utf-8",
      });
      if (r.status !== 0) {
        console.log(`  dprint warn: ${(r.stderr || r.stdout || "").slice(0, 200)}`);
      }
    } catch (e) {
      console.log(`  dprint skip: ${e.message}`);
    }
  }
}
console.log("\nAll languages processed.");
