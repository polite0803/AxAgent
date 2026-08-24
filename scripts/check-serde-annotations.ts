// SPDX-License-Identifier: AGPL-3.0-only

/**
 * check-serde-annotations.ts — 检查 Rust DTO 的 serde 注解
 *
 * 目标：确保跨 IPC 边界的 Rust 结构体有正确的 serde 注解
 * 防止 snake_case 字段序列化后仍为 snake_case，导致前端读取失败
 *
 * 使用方式：
 *   npm run check:serde
 *
 * 检查规则：
 * 1. 结构体有 `Serialize, Deserialize` derive
 * 2. 但缺少 `#[serde(rename_all = "camelCase")]` 注解
 * 3. 且没有手动的 `#[serde(rename = "...")]` 注解
 * 4. 且包含 snake_case 字段名（含下划线的字段）
 * 5. 则报告错误
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

// ==================== 配置 ====================

// 扫描的目录
const SCAN_DIRS = [
  "src-tauri/crates/harness/src",
  "src-tauri/src/commands",
];

// 允许的例外（这些结构体可以保持 snake_case）
// 格式："文件路径::结构体名"
const ALLOWED_EXCEPTIONS = new Set<string>([
  // 在此添加合法的例外
]);

// 已知使用手动 rename 的结构体（不需要 rename_all）
const MANUAL_RENAME_STRUCTS = new Set<string>([
  "agent/payloads.rs::AgentQueryRequest",
  "agent/payloads.rs::AgentQueryResponse",
  "agent/payloads.rs::AgentApproveRequest",
  "agent/payloads.rs::AgentApprovePlanRequest",
  "agent/payloads.rs::AgentRespondAskRequest",
  "agent/payloads.rs::AgentCancelRequest",
  "agent/payloads.rs::AgentUpdateSessionRequest",
  "agent/payloads.rs::AgentUpdateSessionResponse",
  "agent/payloads.rs::AgentGetSessionRequest",
  "agent/payloads.rs::AgentGetSessionResponse",
  "agent/payloads.rs::AgentEnsureWorkspaceRequest",
  "agent/payloads.rs::AgentEnsureWorkspaceResponse",
  "agent/payloads.rs::AgentStatusPayload",
  "agent/payloads.rs::AgentDonePayload",
  "agent/payloads.rs::AgentContentBlock",
  "agent/payloads.rs::AgentErrorPayload",
  "agent/payloads.rs::AgentToolUsePayload",
  "agent/payloads.rs::AgentStreamTextPayload",
  "agent/payloads.rs::AgentStreamThinkingPayload",
  "agent/payloads.rs::PromptCachePayload",
]);

// ==================== 类型定义解析 ====================

interface StructInfo {
  name: string;
  hasSerialize: boolean;
  hasDeserialize: boolean;
  hasRenameAll: boolean;
  hasManualRename: boolean;
  fields: string[];
  deriveLine: number;
}

/**
 * 简单的 Rust 结构体解析器
 */
function parseStructs(filePath: string): StructInfo[] {
  const content = readFileSync(filePath, "utf-8");
  const structs: StructInfo[] = [];

  // 匹配 pub struct Name { ... } 或 pub struct Name(...);
  const structRegex = /pub\s+struct\s+(\w+)\s*(?:<[^>]*>)?\s*(?:where\s+[^{]+)?\{/g;

  let match: RegExpExecArray | null;
  while ((match = structRegex.exec(content)) !== null) {
    const name = match[1];
    const structStart = match.index;

    // 向前查找 derive 宏
    const beforeContent = content.substring(0, structStart);
    const deriveMatch = beforeContent.match(/#\[derive\(([^)]*)\)\][\s\S]*$/);
    const deriveLine = beforeContent.split("\n").length;

    let hasSerialize = false;
    let hasDeserialize = false;
    let hasRenameAll = false;
    let hasManualRename = false;

    if (deriveMatch) {
      const derive = deriveMatch[1];
      hasSerialize = /Serialize/.test(derive);
      hasDeserialize = /Deserialize/.test(derive);
    }

    // 检查是否有 serde rename_all 注解
    const serdeAttrRegex = /#\[serde\([^\]]*\)\]/g;
    const beforeSerdeMatch = beforeContent.match(serdeAttrRegex);
    if (beforeSerdeMatch) {
      for (const attr of beforeSerdeMatch) {
        if (attr.includes("rename_all")) {
          hasRenameAll = true;
        }
        if (attr.includes("rename =")) {
          hasManualRename = true;
        }
      }
    }

    // 提取结构体字段
    const braceStart = content.indexOf("{", structStart);
    const braceEnd = findMatchingBrace(content, braceStart);
    const body = content.substring(braceStart + 1, braceEnd);

    const fields: string[] = [];
    const fieldRegex = /pub\s+(\w+)\s*:/g;
    let fieldMatch: RegExpExecArray | null;
    while ((fieldMatch = fieldRegex.exec(body)) !== null) {
      fields.push(fieldMatch[1]);
    }

    structs.push({
      name,
      hasSerialize,
      hasDeserialize,
      hasRenameAll,
      hasManualRename,
      fields,
      deriveLine,
    });
  }

  return structs;
}

/**
 * 找到匹配的右大括号
 */
function findMatchingBrace(content: string, start: number): number {
  let depth = 0;
  for (let i = start; i < content.length; i++) {
    if (content[i] === "{") depth++;
    if (content[i] === "}") {
      depth--;
      if (depth === 0) return i;
    }
  }
  return content.length;
}

// ==================== 检查逻辑 ====================

interface Violation {
  file: string;
  structName: string;
  snakeCaseFields: string[];
  deriveLine: number;
  suggestion: string;
}

/**
 * 检查单个文件
 */
function checkFile(filePath: string): Violation[] {
  const violations: Violation[] = [];
  const relPath = relative(process.cwd(), filePath);

  const structs = parseStructs(filePath);

  for (const structInfo of structs) {
    // 只检查有 Serialize 和 Deserialize 的结构体
    if (!structInfo.hasSerialize || !structInfo.hasDeserialize) continue;

    // 检查是否在例外列表中
    const exceptionKey = `${relPath}::${structInfo.name}`;
    if (ALLOWED_EXCEPTIONS.has(exceptionKey)) continue;

    // 已知使用手动 rename 的结构体
    const manualRenameKey = `${relPath.split("\\").pop()?.replace(/\\/g, "/")}::${structInfo.name}`;
    if (MANUAL_RENAME_STRUCTS.has(manualRenameKey)) continue;

    // 检查是否已经有 rename_all 或手动 rename
    if (structInfo.hasRenameAll || structInfo.hasManualRename) continue;

    // 找出 snake_case 字段
    const snakeCaseFields = structInfo.fields.filter((f) => f.includes("_"));

    if (snakeCaseFields.length === 0) continue;

    violations.push({
      file: relPath,
      structName: structInfo.name,
      snakeCaseFields,
      deriveLine: structInfo.deriveLine,
      suggestion: `在 derive 宏后添加 #[serde(rename_all = "camelCase")]`,
    });
  }

  return violations;
}

// ==================== 主流程 ====================

function main(): void {
  const allViolations: Violation[] = [];

  for (const dir of SCAN_DIRS) {
    const fullPath = join(process.cwd(), dir);
    if (!statSync(fullPath).isDirectory()) continue;

    const files = findRsFiles(fullPath);
    for (const file of files) {
      const violations = checkFile(file);
      allViolations.push(...violations);
    }
  }

  if (allViolations.length > 0) {
    console.error("\n❌ 检查失败：发现缺少 serde 注解的 DTO");
    console.error("=".repeat(80));

    // 按文件分组
    const grouped = new Map<string, Violation[]>();
    for (const v of allViolations) {
      const existing = grouped.get(v.file) || [];
      existing.push(v);
      grouped.set(v.file, existing);
    }

    for (const [file, violations] of grouped) {
      console.error(`\n📁 ${file}`);
      for (const v of violations) {
        console.error(`   结构体: ${v.structName} (line ~${v.deriveLine})`);
        console.error(`   snake_case 字段: ${v.snakeCaseFields.join(", ")}`);
        console.error(`   建议: ${v.suggestion}`);
      }
    }

    console.error("\n" + "=".repeat(80));
    console.error("💡 修复指南：");
    console.error("   1. 在 derive 宏后添加 #[serde(rename_all = \"camelCase\")]");
    console.error("   2. 或者为每个字段添加 #[serde(rename = \"camelCaseName\")]");
    console.error("   3. 如果是内部使用的结构体，考虑添加到 ALLOWED_EXCEPTIONS 列表");
    console.error("   4. 如果使用手动 rename，添加到 MANUAL_RENAME_STRUCTS 列表\n");

    process.exit(1);
  } else {
    console.log("✅ 检查通过：所有 DTO 都有正确的 serde 注解");
    process.exit(0);
  }
}

/**
 * 递归查找 Rust 文件
 */
function findRsFiles(dir: string): string[] {
  const files: string[] = [];
  const entries = readdirSync(dir);

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      if (!entry.startsWith("__") && entry !== "target" && entry !== "node_modules") {
        files.push(...findRsFiles(fullPath));
      }
    } else if (entry.endsWith(".rs")) {
      files.push(fullPath);
    }
  }

  return files;
}

main();
