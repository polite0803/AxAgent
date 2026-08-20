// SPDX-License-Identifier: AGPL-3.0-only

/**
 * check-paired-fields.ts — 检查分离的字段对
 *
 * 目标：确保"必须同步的多字段对"都使用 Paired/NullablePaired 模式
 * 防止开发者直接创建分离的字段对（如 a: string | null; b: string | null）
 *
 * 使用方式：
 *   npm run check:paired
 *
 * 检查规则：
 * 1. 同一类型中，如果存在命名模式为 `xxxA` + `xxxB` 的字段对
 *    （如 defaultProviderId + defaultModelId）
 * 2. 且类型都是 `string | null` 或类似的可空基本类型
 * 3. 但字段是分离的（没有使用 Paired/NullablePaired）
 * 4. 则报告错误，要求使用 NullablePaired
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

// ==================== 配置 ====================

// 扫描的目录
const SCAN_DIRS = [
  "src/types",
  "src/stores",
];

// 需要检查的命名模式（字段名必须匹配这些模式才会被认为是"可能的字段对"）
// 格式：[前缀, 后缀A, 后缀B]
// 例如：["default", "ProviderId", "ModelId"] 表示 defaultProviderId + defaultModelId
const PAIRED_PATTERNS: Array<{
  prefix: string;
  suffixA: string;
  suffixB: string;
  description: string;
}> = [
  { prefix: "default", suffixA: "ProviderId", suffixB: "ModelId", description: "默认模型" },
  { prefix: "titleSummary", suffixA: "ProviderId", suffixB: "ModelId", description: "标题摘要模型" },
  { prefix: "compression", suffixA: "ProviderId", suffixB: "ModelId", description: "压缩模型" },
  // 新增模式时在此添加
  // { prefix: "yourPrefix", suffixA: "FieldA", suffixB: "FieldB", description: "描述" },
];

// 允许的例外（这些字段对可以保持分离）
// 格式："类型名.字段名A" 或 "类型名.字段名B"
const ALLOWED_EXCEPTIONS = new Set<string>([
  // 在此添加合法的例外
]);

// ==================== 类型定义解析 ====================

interface InterfaceInfo {
  name: string;
  fields: Map<string, string>; // fieldName -> typeString
}

/**
 * 简单的 TypeScript 接口解析器
 * 不使用 AST，使用正则表达式提取接口定义
 */
function parseInterfaces(filePath: string): InterfaceInfo[] {
  const content = readFileSync(filePath, "utf-8");
  const interfaces: InterfaceInfo[] = [];

  // 匹配 interface Xxx { ... } 或 type Xxx = { ... }
  const interfaceRegex = /(?:export\s+)?(?:interface|type)\s+(\w+)\s*(?:=)?\s*\{([^}]*)\}/g;

  let match: RegExpExecArray | null;
  while ((match = interfaceRegex.exec(content)) !== null) {
    const name = match[1];
    const body = match[2];
    const fields = new Map<string, string>();

    // 提取字段
    const fieldRegex = /(\w+)\s*:\s*([^;]+)/g;
    let fieldMatch: RegExpExecArray | null;
    while ((fieldMatch = fieldRegex.exec(body)) !== null) {
      const fieldName = fieldMatch[1];
      const fieldType = fieldMatch[2].trim();
      fields.set(fieldName, fieldType);
    }

    interfaces.push({ name, fields });
  }

  return interfaces;
}

// ==================== 检查逻辑 ====================

interface Violation {
  file: string;
  interfaceName: string;
  fieldA: string;
  fieldB: string;
  patternDesc: string;
  suggestion: string;
}

/**
 * 检查单个接口
 */
function checkInterface(
  iface: InterfaceInfo,
  filePath: string,
): Violation[] {
  const violations: Violation[] = [];

  for (const pattern of PAIRED_PATTERNS) {
    const fieldA = `${pattern.prefix}${pattern.suffixA}`;
    const fieldB = `${pattern.prefix}${pattern.suffixB}`;

    const typeFieldA = `${pattern.prefix}${capitalize(pattern.suffixA)}`;
    const typeFieldB = `${pattern.prefix}${capitalize(pattern.suffixB)}`;

    // 检查接口中是否存在这对字段
    const hasFieldA = iface.fields.has(fieldA) || iface.fields.has(typeFieldA);
    const hasFieldB = iface.fields.has(fieldB) || iface.fields.has(typeFieldB);

    if (!hasFieldA || !hasFieldB) continue;

    // 检查是否在例外列表中
    const keyA = `${iface.name}.${fieldA}`;
    const keyB = `${iface.name}.${fieldB}`;
    if (ALLOWED_EXCEPTIONS.has(keyA) || ALLOWED_EXCEPTIONS.has(keyB)) continue;

    // 检查字段类型
    const typeA = iface.fields.get(fieldA) ?? iface.fields.get(typeFieldA);
    const typeB = iface.fields.get(fieldB) ?? iface.fields.get(typeFieldB);

    if (!typeA || !typeB) continue;

    // 判断是否使用了 Paired 模式
    // 合法模式：NullablePaired<A, B>、NullableModelRef、ModelSelection 等
    const pairedPattern = /Paired<|NullablePaired<|ModelRef|ModelSelection/;
    const isPairedA = pairedPattern.test(typeA);
    const isPairedB = pairedPattern.test(typeB);

    if (isPairedA || isPairedB) continue; // 已使用 Paired 模式

    // 报告违规
    const suggestedField = mapToPairedField(pattern.prefix);
    violations.push({
      file: relative(process.cwd(), filePath),
      interfaceName: iface.name,
      fieldA,
      fieldB,
      patternDesc: pattern.description,
      suggestion: `将 ${fieldA} 和 ${fieldB} 合并为单一字段 ${suggestedField}: NullableModelRef`,
    });
  }

  return violations;
}

/**
 * 将字段前缀映射到建议的合并字段名
 */
function mapToPairedField(prefix: string): string {
  const mappings: Record<string, string> = {
    default: "defaultModel",
    titleSummary: "titleSummaryModel",
    compression: "compressionModel",
    // 新增映射
  };
  return mappings[prefix] ?? `${prefix}Model`;
}

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

// ==================== 主流程 ====================

function main(): void {
  const allViolations: Violation[] = [];

  for (const dir of SCAN_DIRS) {
    const fullPath = join(process.cwd(), dir);
    if (!statSync(fullPath).isDirectory()) continue;

    const files = findTsFiles(fullPath);
    for (const file of files) {
      const interfaces = parseInterfaces(file);
      for (const iface of interfaces) {
        const violations = checkInterface(iface, file);
        allViolations.push(...violations);
      }
    }
  }

  if (allViolations.length > 0) {
    console.error("\n❌ 检查失败：发现分离的字段对");
    console.error("=".repeat(80));

    for (const v of allViolations) {
      console.error(`\n📁 ${v.file}`);
      console.error(`   接口: ${v.interfaceName}`);
      console.error(`   发现: ${v.fieldA} + ${v.fieldB}`);
      console.error(`   描述: ${v.patternDesc}`);
      console.error(`   建议: ${v.suggestion}`);
    }

    console.error("\n" + "=".repeat(80));
    console.error("💡 修复指南：");
    console.error("   1. 在 src/types/paired.ts 中查看 NullablePaired 类型");
    console.error("   2. 将两个字段合并为单一的 NullablePaired 字段");
    console.error("   3. 使用 Paired.fromNullable() 在边界层转换数据");
    console.error("   4. 使用 paired.ts 中的工具函数访问字段值\n");

    process.exit(1);
  } else {
    console.log("✅ 检查通过：所有字段对都使用了 Paired 模式");
    process.exit(0);
  }
}

/**
 * 递归查找 TypeScript 文件
 */
function findTsFiles(dir: string): string[] {
  const files: string[] = [];
  const entries = readdirSync(dir);

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      if (!entry.startsWith("__") && entry !== "node_modules") {
        files.push(...findTsFiles(fullPath));
      }
    } else if (entry.endsWith(".ts") && !entry.endsWith(".d.ts")) {
      files.push(fullPath);
    }
  }

  return files;
}

main();
