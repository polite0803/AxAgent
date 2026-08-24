#!/usr/bin/env node
// scripts/i18n-convert.mjs
// 真实 t() i18n 转换工具（AST 位置级改写，避免正则破坏 JSX/模板字符串）。
//
// 策略：
//  - 用 TypeScript 编译器 API 解析每个目标文件，收集候选字符串字面量
//    （StringLiteral 含 CJK / JsxText 含 CJK），跳过已是 t()/i18n.t() 参数、
//    对象 key、import 路径、JSX 属性名等。
//  - 生成嵌套 key：<文件命名空间>.<sN>，并把源文本作为 defaultValue 兜底。
//    例：t("components.chat.Foo.s1", "原始文本") / i18n.t("lib.bar.s1", "原始文本")
//  - 作用域：文件已 useTranslation → 组件作用域用 t()；否则模块作用域用 i18n.t()
//    （自动补 import i18n from "@/i18n"）。
//  - 将 (key, 源文本) 合并写入 11 个 locale 文件（zh-CN=源文本，其余 10 语言=源文本兜底）。
//  - 从 scripts/.i18n-allowlist.json 移除已转换行。
//
// 用法：
//   node scripts/i18n-convert.mjs --dry-run --file src/lib/foo.ts
//   node scripts/i18n-convert.mjs --file src/lib/foo.ts
//   node scripts/i18n-convert.mjs --all            # 处理 allowlist 全部文件（慎）
import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import * as ts from "typescript";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const SRC = join(root, "src");
const ALLOWLIST = join(root, "scripts", ".i18n-allowlist.json");
const LOCALES_DIR = join(SRC, "i18n", "locales");

const args = process.argv.slice(2);
const DRY = args.includes("--dry-run");
const ALL = args.includes("--all");
const FILE_ARG = (() => {
  const i = args.indexOf("--file");
  return i >= 0 ? args[i + 1] : null;
})();

const CJK = /[㐀-䶿一-鿿]/;
const isCJK = (s) => CJK.test(s);

// ── 加载 allowlist（文件:行 → true）──
let allowSet = new Set();
try {
  const al = JSON.parse(readFileSync(ALLOWLIST, "utf8"));
  for (const e of al.entries || []) {
    const nf = normalizePath(join(root, e.file));
    for (const ln of (e.lines || "").split(",")) {
      if (ln) allowSet.add(nf + ":" + ln);
    }
  }
} catch {}

function normalizePath(p) {
  let s = p;
  if (s.startsWith(root)) s = s.slice(root.length);
  s = s.replace(/\\/g, "/").replace(/^\/+/, "");
  return s;
}

// ── 收集待处理文件 ──
function collectFiles(dir, out) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) {
      if (p.endsWith(join("i18n", "locales"))) continue;
      collectFiles(p, out);
    } else if (/\.(ts|tsx)$/.test(name)) {
      const rel = normalizePath(p);
      if (/\/__tests__\/|\.test\.|\.spec\./.test(rel)) continue;
      out.push(p);
    }
  }
}

let targets = [];
if (FILE_ARG) {
  const fp = join(root, FILE_ARG);
  if (!existsSync(fp)) { console.error("文件不存在:", fp); process.exit(2); }
  targets = [fp];
} else if (ALL) {
  collectFiles(SRC, targets);
} else {
  console.error("请指定 --file <path> 或 --all");
  process.exit(2);
}

// ── 命名空间：src/components/chat/Foo.tsx → components.chat.Foo ──
function nsFromFile(absPath) {
  const rel = normalizePath(absPath).replace(/^src\//, "").replace(/\.(ts|tsx)$/, "");
  return rel;
}

// 把 "a.b.c" 设进嵌套对象
function setNested(obj, path, value) {
  const parts = path.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    const k = parts[i];
    if (typeof cur[k] !== "object" || cur[k] === null) cur[k] = {};
    cur = cur[k];
  }
  cur[parts[parts.length - 1]] = value;
}

const LOCALE_KEYS = ["zh-CN", "zh-TW", "en-US", "ja", "ko", "fr", "de", "es", "ru", "hi", "ar"];

// 加载全部 locale
function loadLocales() {
  const map = {};
  for (const k of LOCALE_KEYS) {
    const f = join(LOCALES_DIR, k + ".json");
    map[k] = JSON.parse(readFileSync(f, "utf8"));
  }
  return map;
}

// ── 判断节点是否已被翻译（父节点是 t()/i18n.t() 调用参数）──
function isInsideTCall(node, sf) {
  let p = node.parent;
  while (p) {
    if (
      ts.isCallExpression(p) &&
      ts.isPropertyAccessExpression(p.expression) &&
      (p.expression.name.text === "t") &&
      p.arguments.includes(node)
    ) {
      return true;
    }
    // i18n.t(...)
    if (
      ts.isCallExpression(p) &&
      ts.isPropertyAccessExpression(p.expression) &&
      p.expression.name.text === "t" &&
      ts.isPropertyAccessExpression(p.expression.expression) &&
      p.expression.expression.name.text === "i18n" &&
      p.arguments.includes(node)
    ) {
      return true;
    }
    p = p.parent;
  }
  return false;
}

// ── 判断 StringLiteral 是否处于「不应翻译」的位置 ──
function isSkipLiteral(node, sf) {
  const parent = node.parent;
  // 对象属性 key（"foo": bar）
  if (ts.isPropertyAssignment(parent) && parent.name === node) return true;
  // 计算属性 [node]
  if (ts.isComputedPropertyName(parent) && parent.expression === node) return true;
  // import/export 路径
  if (ts.isImportDeclaration(parent) || ts.isExportDeclaration(parent)) {
    if (parent.moduleSpecifier === node) return true;
  }
  if (ts.isModuleDeclaration(parent) && parent.name === node) return true;
  // require("...")
  if (ts.isCallExpression(parent) && parent.arguments.includes(node)) {
    const callee = parent.expression;
    if (ts.isIdentifier(callee) && callee.text === "require") return true;
  }
  // JSX 属性名（name= 位置，但属性名是 Identifier 不是 StringLiteral；这里兜底）
  if (ts.isJsxAttribute(parent) && parent.name === node) return true;
  return false;
}

// ── 主转换 ──
const allPairs = []; // {key, source}
const removedAllowLines = []; // "file:line"

for (const absPath of targets) {
  const rel = normalizePath(absPath);
  const code = readFileSync(absPath, "utf8");
  const sf = ts.createSourceFile(rel, code, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);

  // 文件是否已使用 useTranslation（组件作用域）
  const usesTranslation = /useTranslation/.test(code) || /from ["']react-i18next["']/.test(code);
  const ns = nsFromFile(absPath);

  // 收集候选节点（带位置）
  const cands = [];
  function walk(node) {
    if (ts.isStringLiteral(node)) {
      const text = node.text;
      if (isCJK(text) && !isInsideTCall(node, sf) && !isSkipLiteral(node, sf)) {
        cands.push({ node, kind: "str", text, line: sf.getLineAndCharacterOfPosition(node.getStart()).line + 1 });
      }
    } else if (ts.isJsxText(node)) {
      const text = node.text;
      const trimmed = text.replace(/\s+/g, "");
      if (trimmed && isCJK(trimmed) && !isInsideTCall(node, sf)) {
        cands.push({ node, kind: "jsx", text: trimmed, raw: text, line: sf.getLineAndCharacterOfPosition(node.getStart()).line + 1 });
      }
    }
    ts.forEachChild(node, walk);
  }
  walk(sf);

  if (cands.length === 0) continue;

  // 仅处理 allowlist 中基线过的行（避免误改非基线内容）
  const fileCands = cands.filter((c) => allowSet.has(rel + ":" + c.line));
  if (fileCands.length === 0) {
    // 非基线但有 CJK —— 跳过，保持谨慎
    continue;
  }

  // 生成替换（按位置从后往前，避免偏移）
  fileCands.sort((a, b) => b.node.getStart() - a.node.getStart());
  let newCode = code;
  let idx = 0;
  for (const c of fileCands) {
    idx += 1;
    const key = `${ns}.s${idx}`;
    const src = c.text;
    allPairs.push({ key, source: src });
    let replacement;
    if (c.kind === "jsx") {
      // JSX 文本 → {t("key","text")}
      const expr = usesTranslation ? `t("${key}", "${escapeForT(src)}")` : `i18n.t("${key}", "${escapeForT(src)}")`;
      replacement = `{${expr}}`;
    } else {
      const expr = usesTranslation ? `t("${key}", "${escapeForT(src)}")` : `i18n.t("${key}", "${escapeForT(src)}")`;
      replacement = expr;
    }
    const start = c.node.getStart();
    const end = c.node.getEnd();
    newCode = newCode.slice(0, start) + replacement + newCode.slice(end);
    removedAllowLines.push(rel + ":" + c.line);
  }

  // 模块作用域且未 import i18n → 补 import
  if (!usesTranslation) {
    if (!/import\s+i18n\s+from\s+["']@\/i18n["']/.test(newCode)) {
      newCode = `import i18n from "@/i18n";\n` + newCode;
    }
  }

  if (DRY) {
    console.log(`\n[DRY] ${rel} — ${fileCands.length} 处，命名空间 ${ns}，作用域 ${usesTranslation ? "t()" : "i18n.t()"}`);
    for (const c of fileCands.slice().reverse()) {
      console.log(`  L${c.line}: ${c.text.slice(0, 40)}`);
    }
  } else {
    writeFileSync(absPath, newCode, "utf8");
    console.log(`已转换 ${rel} — ${fileCands.length} 处`);
  }
}

function escapeForT(s) {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n").replace(/\r/g, "");
}

// ── 写入 locale ──
if (!DRY && allPairs.length > 0) {
  const locales = loadLocales();
  for (const { key, source } of allPairs) {
    for (const lk of LOCALE_KEYS) {
      setNested(locales[lk], key, source);
    }
  }
  for (const lk of LOCALE_KEYS) {
    writeFileSync(join(LOCALES_DIR, lk + ".json"), JSON.stringify(locales[lk], null, 2) + "\n", "utf8");
  }
  console.log(`已写入 ${allPairs.length} 个 key 到 ${LOCALE_KEYS.length} 个 locale`);
}

// ── 更新 allowlist（移除已转换行）──
if (!DRY && removedAllowLines.length > 0) {
  const al = JSON.parse(readFileSync(ALLOWLIST, "utf8"));
  const removeSet = new Set(removedAllowLines);
  const newEntries = [];
  for (const e of al.entries || []) {
    const nf = normalizePath(join(root, e.file));
    const lines = (e.lines || "").split(",").filter(Boolean).map(Number).filter((ln) => !removeSet.has(nf + ":" + ln));
    if (lines.length > 0) {
      newEntries.push({ ...e, lines: lines.sort((a, b) => a - b).join(",") });
    }
  }
  al.entries = newEntries;
  al.total_entries = newEntries.length;
  al.total_files = new Set(newEntries.map((e) => e.file)).size;
  writeFileSync(ALLOWLIST, JSON.stringify(al, null, 2) + "\n", "utf8");
  console.log(`已更新 allowlist，移除 ${removedAllowLines.length} 行`);
}

if (DRY) {
  console.log(`\n[DRY-RUN] 将转换 ${allPairs.length} 处，未写入任何文件。`);
}
