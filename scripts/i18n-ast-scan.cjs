#!/usr/bin/env node
/**
 * 基于 TypeScript Compiler API 的 i18n 引用扫描器。
 * 模拟 i18n Ally 的精确度，找出所有 t('key') 静态 key 引用。
 */

const fs = require("fs");
const path = require("path");
const ts = require("typescript");

const SRC_DIR = path.join(__dirname, "..", "src");
const LOCALES_DIR = path.join(SRC_DIR, "i18n", "locales");

// 加载 zh-CN locale
function loadLocaleKeys() {
  const file = path.join(LOCALES_DIR, "zh-CN.json");
  const data = JSON.parse(fs.readFileSync(file, "utf-8"));

  function flatten(obj, prefix = "") {
    const result = new Set();
    if (typeof obj === "object" && obj !== null) {
      for (const [k, v] of Object.entries(obj)) {
        const p = prefix ? `${prefix}.${k}` : k;
        if (typeof v === "object" && v !== null) {
          for (const sub of flatten(v, p)) { result.add(sub); }
        } else {
          result.add(p);
        }
      }
    }
    return result;
  }
  return flatten(data);
}

// 收集源码文件
function collectFiles(dir, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (
      entry.name === "node_modules" || entry.name === "dist"
      || entry.name === "build" || entry.name === "i18n"
    ) { continue; }
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      collectFiles(full, files);
    } else if (/\.(ts|tsx|js|jsx)$/.test(entry.name)) {
      files.push(full);
    }
  }
  return files;
}

// 用 TypeScript AST 提取所有 t('key') 调用
function extractKeysFromAst(filePath) {
  const code = fs.readFileSync(filePath, "utf-8");
  const source = ts.createSourceFile(
    filePath,
    code,
    ts.ScriptTarget.ESNext,
    true,
    filePath.endsWith(".tsx") || filePath.endsWith(".jsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );

  const keys = new Set();
  const prefixes = new Set();
  const refs = []; // {key, file, line, col}

  // 节点访问器
  function visit(node) {
    // t('static.key')
    if (ts.isCallExpression(node)) {
      const expr = node.expression;
      const isT = (ts.isIdentifier(expr) && expr.text === "t")
        || (ts.isPropertyAccessExpression(expr) && expr.name.text === "t")
        || (ts.isIdentifier(expr) && ["Trans", "trans"].includes(expr.text))
        || (ts.isPropertyAccessExpression(expr) && ["Trans", "trans"].includes(expr.name.text));
      if (isT && node.arguments.length >= 1) {
        const arg = node.arguments[0];
        extractStringFromArg(arg, keys, prefixes, filePath, node);
      }
    }
    // <Trans i18nKey="key">
    if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
      const tagName = ts.getTagNameForNode ? ts.getTagNameForNode(node) : node.tagName;
      if (ts.isIdentifier(tagName) && tagName.text === "Trans") {
        for (const attr of node.attributes.properties) {
          if (ts.isJsxAttribute(attr) && ts.isIdentifier(attr.name) && attr.name.text === "i18nKey") {
            if (attr.initializer && ts.isStringLiteral(attr.initializer)) {
              keys.add(attr.initializer.text);
              refs.push({ key: attr.initializer.text, file: filePath, line: getLine(node) });
            }
          }
        }
      }
    }
    // 通用：任何字面量属性名为 i18nKey/labelKey 的对象
    if (ts.isPropertyAssignment(node)) {
      if (
        ts.isIdentifier(node.name)
        && /^(i18nKey|labelKey|nameKey|descriptionKey|titleKey|tabKey|keyName|translationKey)$/.test(node.name.text)
      ) {
        if (node.initializer && ts.isStringLiteral(node.initializer)) {
          keys.add(node.initializer.text);
          refs.push({ key: node.initializer.text, file: filePath, line: getLine(node) });
        }
      }
    }

    ts.forEachChild(node, visit);
  }

  function extractStringFromArg(arg, keys, prefixes, filePath, callNode) {
    // 'static.key'
    if (ts.isStringLiteral(arg) || ts.isNoSubstitutionTemplateLiteral(arg)) {
      if (arg.text.includes(".")) {
        keys.add(arg.text);
        refs.push({ key: arg.text, file: filePath, line: getLine(callNode) });
      }
    }
    // `prefix.${var}` — NoSubstitutionTemplateLiteral handled above
    // Template literal with expression: `prefix.${var}` or `prefix.${var}.suffix`
    if (ts.isTemplateExpression(arg) || ts.isTemplateHead(arg) || ts.isTemplateMiddle(arg)) {
      // Already handled by NoSubstitutionTemplateLiteral for static
    }
    if (ts.isTemplateExpression(arg)) {
      // `prefix.${var}` — record prefix
      if (arg.head && ts.isTemplateHead(arg.head)) {
        const prefix = arg.head.text;
        if (prefix) { prefixes.add(prefix); }
      }
      // `prefix.${var}.suffix` — prefix before first interpolation
      // head.text ends with anything before the ${
      // For now just record head.text + head.text as prefix
    }
    // 三元表达式：condition ? 'k1' : 'k2'
    if (ts.isConditionalExpression(arg)) {
      extractStringFromArg(arg.whenTrue, keys, prefixes, filePath, callNode);
      extractStringFromArg(arg.whenFalse, keys, prefixes, filePath, callNode);
    }
  }

  function getLine(node) {
    if (node.getSourceFile && node.getSourceFile().getLineAndCharacterOfPosition) {
      return node.getSourceFile().getLineAndCharacterOfPosition(node.getStart()).line + 1;
    }
    return 0;
  }

  visit(source);
  return { keys, prefixes, refs };
}

// 主流程
function main() {
  console.log("=".repeat(60));
  console.log("TypeScript AST i18n 扫描器");
  console.log("=".repeat(60));

  const defined = loadLocaleKeys();
  console.log(`Locale 已定义 key: ${defined.size}`);

  const files = collectFiles(SRC_DIR);
  console.log(`源码文件: ${files.length}`);

  const allKeys = new Set();
  const allPrefixes = new Set();
  const refsByKey = new Map();

  let processed = 0;
  for (const file of files) {
    try {
      const { keys, prefixes, refs } = extractKeysFromAst(file);
      for (const k of keys) {
        allKeys.add(k);
        if (!refsByKey.has(k)) { refsByKey.set(k, []); }
        refsByKey.get(k).push(...refs);
      }
      for (const p of prefixes) { allPrefixes.add(p); }
    } catch (e) {
      console.warn(`解析失败: ${file}: ${e.message}`);
    }
    processed++;
  }

  console.log(`解析成功: ${processed}/${files.length}`);
  console.log(`静态 key 引用: ${allKeys.size}`);
  console.log(`动态 prefix: ${allPrefixes.size}`);

  // 未定义
  const undefined = [...allKeys].filter(k => !defined.has(k)).sort();
  console.log(`未定义: ${undefined.length}`);

  // 分类
  const byPrefix = {};
  for (const k of undefined) {
    const p = k.split(".")[0];
    byPrefix[p] = (byPrefix[p] || 0) + 1;
  }

  console.log("\n按 prefix 分组:");
  for (const [p, n] of Object.entries(byPrefix).sort((a, b) => b[1] - a[1])) {
    console.log(`  ${p}: ${n}`);
  }

  // 输出未定义列表
  console.log("\n未定义 key 列表:");
  for (const k of undefined) {
    const refs = refsByKey.get(k) || [];
    const firstRef = refs[0];
    if (firstRef) {
      const rel = path.relative(SRC_DIR, firstRef.file);
      console.log(`  ❌ ${k}  (${rel}:${firstRef.line})`);
    } else {
      console.log(`  ❌ ${k}`);
    }
  }
}

main();
