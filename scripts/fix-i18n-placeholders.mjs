#!/usr/bin/env node
/**
 * fix-i18n-placeholders.mjs
 *
 * Auto-detect and fix placeholder ({xxx} / {{xxx}}) inconsistencies
 * between zh-CN.json and en-US.json.
 *
 * Two types of fixes:
 *   1. Missing placeholders — en-US has a placeholder that zh-CN lacks
 *      → Add the placeholder to zh-CN text
 *   2. Extra placeholders — zh-CN has a placeholder that en-US doesn't
 *      → Remove the placeholder from zh-CN text
 *
 * Usage:
 *   node scripts/fix-i18n-placeholders.mjs
 *   node scripts/fix-i18n-placeholders.mjs --dry-run   # preview only, no writes
 */

import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const EN_PATH = path.join(ROOT, "src/i18n/locales/en-US.json");
const ZH_PATH = path.join(ROOT, "src/i18n/locales/zh-CN.json");

const DRY_RUN = process.argv.includes("--dry-run");

// ── Helper: flatten nested JSON to dot-notation keys ──
function flatten(obj, prefix = "") {
  const result = {};
  for (const [k, v] of Object.entries(obj)) {
    const full = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      Object.assign(result, flatten(v, full));
    } else if (typeof v === "string") {
      result[full] = v;
    }
  }
  return result;
}

// ── Helper: set a nested value by dot-notation key ──
function setNested(obj, key, value) {
  const parts = key.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]] || typeof cur[parts[i]] !== "object") {
      cur[parts[i]] = {};
    }
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}

// ── Helper: get a nested value by dot-notation key ──
function getNested(obj, key) {
  const parts = key.split(".");
  let cur = obj;
  for (const p of parts) {
    if (cur == null || typeof cur !== "object") { return undefined; }
    cur = cur[p];
  }
  return cur;
}

// ── Extract {{xxx}} and {xxx} placeholders ──
function extractPlaceholders(str) {
  if (typeof str !== "string") { return new Set(); }
  const matches = str.match(/\{\{?\w+\}?\}/g);
  return new Set(matches || []);
}

// ── Manual fix map: key → new zh-CN value ──
// These are crafted to match en-US placeholders while reading naturally in Chinese.
const MANUAL_FIXES = {
  // ── Missing placeholders: add ──
  // en: "{{count}} attempts" → zh: "{{count}} 次尝试"
  "chat.workflow.attempts": "{{count}} 次尝试",

  // en: "Failed to place node: {{error}}" → zh: "节点放置失败：{{error}}"
  "workflow.nodeDropFailed": "节点放置失败：{{error}}",

  // en: 'Are you sure you want to delete template "{{name}}"?' → zh: '确认删除模板 "{{name}}"？'
  "workflow.confirmDeleteTemplate": '确认删除模板 "{{name}}"？',

  // en: "New {{type}}" → zh: "新建{{type}}"
  "workflow.newNode": "新建{{type}}",

  // en: "{{count}} nodes pasted" → zh: "{{count}} 个节点已粘贴"
  "workflow.nodesPasted": "{{count}} 个节点已粘贴",

  // en: "Loaded version {{version}}" → zh: "已加载版本 {{version}}"
  "workflow.versionHistory.loadedVersion": "已加载版本 {{version}}",

  // en: "Version History - {{name}}" → zh: "版本历史 - {{name}}"
  "workflow.versionHistory.title": "版本历史 - {{name}}",

  // en: "Version {{version}}" → zh: "版本 {{version}}"
  "workflow.versionHistory.version": "版本 {{version}}",

  // en: "Version {{version}} loaded" → zh: "版本 {{version}} 已加载"
  "workflow.versionLoaded": "版本 {{version}} 已加载",

  // en: "Version History - {{name}}" → zh: "版本历史 - {{name}}"
  "workflow.versionHistoryTitle": "版本历史 - {{name}}",

  // en: "Version {{version}}" → zh: "版本号 {{version}}"
  "workflow.versionNumber": "版本号 {{version}}",

  // en: "Knowledge Base: {{name}}" → zh: "知识库：{{name}}"
  "workflow.props.contextKnowledgeBase": "知识库：{{name}}",

  // en: "Branches ({{count}})" → zh: "分支（{{count}}）"
  "workflow.props.branches": "分支（{{count}}）",

  // en: "Loop Body Steps ({{count}})" → zh: "循环体步骤（{{count}}）"
  "workflow.props.loopBodySteps": "循环体步骤（{{count}}）",

  // en: "Input Count ({{count}})" → zh: "输入数量（{{count}}）"
  "workflow.props.inputCount": "输入数量（{{count}}）",

  // en: "Conditions ({{count}})" → zh: "条件（{{count}}）"
  "workflow.props.conditions": "条件（{{count}}）",

  // en: "~ {{count}} minutes" → zh: "约 {{count}} 分钟"
  "workflow.props.aboutMinutes": "约 {{count}} 分钟",

  // en: "{{count}} seconds" → zh: "{{count}} 秒"
  "workflow.props.aboutSeconds": "{{count}} 秒",

  // en: "Branch {{index}}" → zh: "分支 {{index}}"
  "workflow.props.branchTitle": "分支 {{index}}",

  // en: "Nodes: {{count}}" → zh: "节点：{{count}}"
  "workflow.statusBar.nodes": "节点：{{count}}",

  // en: "Edges: {{count}}" → zh: "连线：{{count}}"
  "workflow.statusBar.edges": "连线：{{count}}",

  // en: "{{count}} errors" → zh: "{{count}} 个错误"
  "workflow.statusBar.errors": "{{count}} 个错误",

  // en: "{{count}} warnings" → zh: "{{count}} 个警告"
  "workflow.statusBar.warnings": "{{count}} 个警告",

  // en: "{{count}} errors" → zh: "{{count}} 个错误"
  "workflow.statusBar.error": "{{count}} 个错误",

  // en: "{{count}} warnings" → zh: "{{count}} 个警告"
  "workflow.statusBar.warning": "{{count}} 个警告",

  // en: "+{{count}} more" → zh: "+{{count}} 更多条件"
  "workflow.conditionNode.moreConditions": "+{{count}} 更多条件",

  // en: "Max {{count}} iterations" → zh: "最大 {{count}} 次迭代"
  "workflow.loopNode.maxIterations": "最大 {{count}} 次迭代",

  // en: "{{count}} steps" → zh: "{{count}} 步"
  "workflow.loopNode.steps": "{{count}} 步",

  // en: "Branch {{index}}" → zh: "分支 {{index}}"
  "workflow.parallelNode.branch": "分支 {{index}}",

  // en: "{{count}} more branches" → zh: "{{count}} 更多分支"
  "workflow.parallelNode.moreBranches": "{{count}} 更多分支",

  // en: "{{count}} inputs" → zh: "{{count}} 个输入参数"
  "workflow.toolNode.inputCount": "{{count}} 个输入参数",

  // en: "Retry (max {{count}} times)" → zh: "重试（最大 {{count}} 次）"
  "workflow.validationNode.retry": "重试（最大 {{count}} 次）",

  // ── Extra placeholders: remove ──
  // en has no placeholders; zh had {{items}}
  "workflow.aiAssist.loop.continueHint": "例如：'当 index < 数组长度时继续'",

  // en has no placeholders; zh had {{current}} and {{available}}
  // Match en: "Describe parallel branch inputs and the target output."
  "workflow.aiAssist.parallel.branchesHint": "描述并行分支的输入和目标输出。",

  // en has no placeholders; zh had {{current}} and {{input}}
  // Match en: "List all cases and their trigger conditions."
  "workflow.aiAssist.switch.casesHint": "列出所有分支及其触发条件。",

  // en: "All data in this view..." (no placeholder)
  // zh had "{{date}}" — remove it
  "timeTravel.replayBadge.tooltip": "此视图中的所有资料、提示词与结论都以当前时间为锚点。不可用于实盘交易。",
};

// ── Main ──
function main() {
  const enRaw = JSON.parse(readFileSync(EN_PATH, "utf-8"));
  const zhRaw = JSON.parse(readFileSync(ZH_PATH, "utf-8"));
  const enFlat = flatten(enRaw);
  const zhFlat = flatten(zhRaw);

  const missingFixes = [];
  const extraFixes = [];

  // Phase 1: Detect issues
  for (const [key, enVal] of Object.entries(enFlat)) {
    const zhVal = zhFlat[key];
    if (!zhVal) { continue; }

    const enPH = extractPlaceholders(enVal);
    const zhPH = extractPlaceholders(zhVal);

    for (const ph of enPH) {
      if (!zhPH.has(ph)) {
        missingFixes.push({ key, enVal, zhVal, ph });
      }
    }

    for (const ph of zhPH) {
      if (!enPH.has(ph)) {
        extraFixes.push({ key, enVal, zhVal, ph });
      }
    }
  }

  console.log("=== Placeholder Fix Report ===\n");
  console.log(`Missing placeholders (en has, zh lacks): ${missingFixes.length}`);
  console.log(`Extra placeholders (zh has, en lacks):   ${extraFixes.length}\n`);

  // Phase 2: Apply fixes
  let fixedCount = 0;
  let skippedCount = 0;

  // ── Fix missing placeholders via manual map ──
  for (const { key } of missingFixes) {
    if (MANUAL_FIXES[key]) {
      const newVal = MANUAL_FIXES[key];
      const currentZh = getNested(zhRaw, key);
      if (currentZh !== newVal) {
        console.log(`FIX (missing): [${key}]`);
        console.log(`  en:  ${enFlat[key]}`);
        console.log(`  old: ${currentZh}`);
        console.log(`  new: ${newVal}`);
        if (!DRY_RUN) {
          setNested(zhRaw, key, newVal);
        }
        fixedCount++;
      } else {
        skippedCount++;
      }
    } else {
      console.log(`SKIP (missing): [${key}] — no manual fix provided`);
      skippedCount++;
    }
  }

  // ── Fix extra placeholders via manual map ──
  for (const { key } of extraFixes) {
    if (MANUAL_FIXES[key]) {
      const newVal = MANUAL_FIXES[key];
      const currentZh = getNested(zhRaw, key);
      if (currentZh !== newVal) {
        console.log(`FIX (extra): [${key}]`);
        console.log(`  en:  ${enFlat[key]}`);
        console.log(`  old: ${currentZh}`);
        console.log(`  new: ${newVal}`);
        if (!DRY_RUN) {
          setNested(zhRaw, key, newVal);
        }
        fixedCount++;
      } else {
        skippedCount++;
      }
    } else {
      console.log(`SKIP (extra): [${key}] — no manual fix provided`);
      skippedCount++;
    }
  }

  console.log(`\n=== Result ===`);
  console.log(`Fixed:    ${fixedCount}`);
  console.log(`Skipped:  ${skippedCount}`);
  console.log(`Expected: ${missingFixes.length + extraFixes.length}`);

  if (fixedCount === 0 && skippedCount > 0) {
    console.log("\nAll fixes were already applied or all files up to date.");
  }

  // Write back
  if (!DRY_RUN && fixedCount > 0) {
    writeFileSync(ZH_PATH, JSON.stringify(zhRaw, null, 2) + "\n", "utf-8");
    console.log(`\n✅ zh-CN.json updated.`);
  } else if (DRY_RUN && fixedCount > 0) {
    console.log(`\n🔍 Dry-run mode — no files written. Run without --dry-run to apply.`);
  } else {
    console.log(`\n✅ No changes needed.`);
  }
}

main();
