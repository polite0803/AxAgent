#!/usr/bin/env node
/**
 * scripts/check-id-validation.mjs
 *
 * 数据边界检查脚本 - 在 CI 中运行
 *
 * 检查内容：
 * 1. 模板字符串中使用可能为 undefined/null 的变量拼接 ID
 *    危险模式：`${a}::${b}` 其中 a 或 b 可能为 undefined
 * 2. 直接将 "undefined"/"null" 字符串赋值给 ID 字段
 * 3. 绕过 validators.ts 验证工具直接操作 ID
 *
 * 退出码：0 = 通过，1 = 发现违规
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const SRC_DIR = join(process.cwd(), "src");

// 需要检查的文件模式
const FILE_EXTENSIONS = [".ts", ".tsx"];

// 已知的误报文件（注释或模板字符串中包含危险模式，但实际不是 ID 拼接）
const FALSE_POSITIVE_FILES = [
  "src/lib/validators.ts", // safeJoinIds 函数的文档注释
  "src/sdk/sandboxTemplate.ts", // HTML 模板字符串
];

// 危险模式定义
const DANGEROUS_PATTERNS = [
  {
    name: "模板字符串拼接可能为 undefined 的 ID",
    // 匹配 ${...}::${...} 模式，其中变量可能为 undefined
    regex: /`[^`]*\$\{[^}]+\}[^`]*::[^`]*\$\{[^}]+\}[^`]*`/g,
    severity: "error",
    description:
      "使用模板字符串拼接 ID 对，如果变量为 undefined 会生成 'undefined::xxx' 脏数据",
    suggestion:
      '使用 safeJoinIds([a, b], "::") 替代模板字符串，自动过滤 undefined/null',
  },
  {
    name: "直接使用 'undefined' 字符串",
    regex: /===?\s*["']undefined["']|["']undefined["']\s*===?/g,
    severity: "warning",
    description: "直接与 'undefined' 字符串比较，可能遗漏了其他无效值检查",
    suggestion: "使用 isValidId() 或 sanitizeId() 统一验证",
  },
  {
    name: "直接使用 'null' 字符串",
    regex: /===?\s*["']null["']|["']null["']\s*===?/g,
    severity: "warning",
    description: "直接与 'null' 字符串比较，可能遗漏了其他无效值检查",
    suggestion: "使用 isValidId() 或 sanitizeId() 统一验证",
  },
];

/**
 * 递归获取所有源文件
 */
function getSourceFiles(dir, fileList = []) {
  const entries = readdirSync(dir);

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      if (!entry.startsWith("__") && entry !== "node_modules") {
        getSourceFiles(fullPath, fileList);
      }
    } else if (FILE_EXTENSIONS.some((ext) => entry.endsWith(ext))) {
      fileList.push(fullPath);
    }
  }

  return fileList;
}

/**
 * 检查单个文件
 */
function checkFile(filePath) {
  const violations = [];
  const content = readFileSync(filePath, "utf-8");
  const lines = content.split("\n");

  // 跳过 node_modules 和 __tests__ 目录
  const relativePath = relative(process.cwd(), filePath);
  if (
    relativePath.includes("node_modules")
    || relativePath.includes("__tests__")
    || relativePath.includes("test")
  ) {
    return violations;
  }
  
  // 跳过已知的误报文件
  const normalizedPath = relativePath.replace(/\\/g, "/");
  if (FALSE_POSITIVE_FILES.some((f) => normalizedPath.endsWith(f))) {
    return violations;
  }

  // 找出注释行（单行注释 // 和块注释 /* ... */）
  const commentLineSet = new Set();
  let inBlockComment = false;
  
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (inBlockComment) {
      commentLineSet.add(i);
      if (trimmed.includes("*/")) {
        inBlockComment = false;
      }
      continue;
    }
    if (trimmed.startsWith("//")) {
      commentLineSet.add(i);
      continue;
    }
    if (trimmed.startsWith("/*") || trimmed.startsWith("*")) {
      commentLineSet.add(i);
      if (!trimmed.includes("*/")) {
        inBlockComment = true;
      }
    }
  }

  for (const pattern of DANGEROUS_PATTERNS) {
    const regex = new RegExp(pattern.regex.source, "g");
    let match;

    while ((match = regex.exec(content)) !== null) {
      // 找到匹配的行号
      const matchStart = match.index;
      const lineNumber = content.substring(0, matchStart).split("\n").length;
      const lineIndex = lineNumber - 1;

      // 跳过注释行
      if (commentLineSet.has(lineIndex)) {
        continue;
      }

      violations.push({
        file: relativePath,
        line: lineNumber,
        pattern: pattern.name,
        severity: pattern.severity,
        description: pattern.description,
        suggestion: pattern.suggestion,
        matched: match[0].substring(0, 80), // 截断匹配内容
      });
    }
  }

  return violations;
}

/**
 * 主函数
 */
function main() {
  console.log("=== ID 数据边界检查 ===\n");

  const files = getSourceFiles(SRC_DIR);
  console.log(`检查 ${files.length} 个源文件...\n`);

  const allViolations = [];
  const errorViolations = [];
  const warningViolations = [];

  for (const file of files) {
    const violations = checkFile(file);
    for (const v of violations) {
      allViolations.push(v);
      if (v.severity === "error") {
        errorViolations.push(v);
      } else {
        warningViolations.push(v);
      }
    }
  }

  if (allViolations.length === 0) {
    console.log("✅ 通过：未发现数据边界违规");
    process.exit(0);
  }

  // 输出错误级别违规
  if (errorViolations.length > 0) {
    console.error(`❌ 发现 ${errorViolations.length} 个错误级别违规：\n`);
    for (const v of errorViolations) {
      console.error(`  [ERROR] ${v.file}:${v.line}`);
      console.error(`          模式: ${v.pattern}`);
      console.error(`          描述: ${v.description}`);
      console.error(`          建议: ${v.suggestion}`);
      console.error(`          匹配: ...${v.matched}...\n`);
    }
  }

  // 输出警告级别违规
  if (warningViolations.length > 0) {
    console.warn(`⚠️  发现 ${warningViolations.length} 个警告级别违规：\n`);
    for (const v of warningViolations) {
      console.warn(`  [WARN] ${v.file}:${v.line}`);
      console.warn(`         模式: ${v.pattern}`);
      console.warn(`         建议: ${v.suggestion}\n`);
    }
  }

  // 只有错误级别才导致 CI 失败
  if (errorViolations.length > 0) {
    console.log(`\n❌ ${errorViolations.length} 个错误必须修复后才能提交`);
    process.exit(1);
  } else {
    console.log(`\n⚠️  仅有警告级别违规，不阻塞提交`);
    process.exit(0);
  }
}

main();
