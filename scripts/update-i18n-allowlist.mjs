#!/usr/bin/env node
// Update .i18n-allowlist.json — only add non-UI technical strings.
// Run: node scripts/update-i18n-allowlist.mjs

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ALLOWLIST_PATH = path.join(__dirname, ".i18n-allowlist.json");
const ROOT = path.join(__dirname, "..");

const allowlistRaw = fs.readFileSync(ALLOWLIST_PATH, "utf8");
const allowlist = JSON.parse(allowlistRaw);
const entryMap = new Map();
for (const entry of allowlist.entries) {
  entryMap.set(entry.file, entry);
}

function* scanFiles(dir) {
  for (const name of fs.readdirSync(dir)) {
    const fp = path.join(dir, name);
    if (["node_modules", ".workbuddy"].includes(name)) { continue; }
    if (fs.statSync(fp).isDirectory()) {
      yield* scanFiles(fp);
    } else if (/\.(ts|tsx)$/.test(name) && !fp.includes("src/i18n/locales/")) {
      yield fp;
    }
  }
}

const CJK_RE = /[\u4e00-\u9fff\u3400-\u4dbf]/;

// Lines that DO contain CJK but are NOT user-facing UI text.
// These go into the allowlist. Everything else needs i18n.
function isTechnical(file, lineNum, text) {
  const trimmed = text.trim();

  // --- test files: all content is test infra ---
  if (file.includes("/__tests__/")) { return true; }

  // --- comments ---
  if (/^(\s*\/\/|\s*\*|\s*\/\*\*)/.test(text)) { return true; }

  // --- console / logIpcError: internal diagnostics ---
  if (/console\.(log|warn|error|debug|info|trace)/.test(text)) { return true; }
  if (/logIpcError/.test(text)) { return true; }

  // --- keyword-matching arrays (intent detection, not displayed) ---
  if (
    /^(\s*)["']\u[\u4e00-\u9fff]+["'],?\s*$/.test(text) && (
      file.includes("StructuredThinking")
      || file.includes("proactiveStore")
    )
  ) { return true; }

  // --- regex patterns with CJK ---
  if (/\/\(/.test(text) && /\\/.test(text)) { return true; }

  // --- JSX comments: {/* 中文 */} ---
  if (/\{\/\*/.test(text)) { return true; }

  // --- inline code comments with CJK (single-line) ---
  if (/\/\/.*[\u4e00-\u9fff]/.test(text)) { return true; }

  // --- PropertyPanel / AIPanel: all content is LLM prompt, not UI ---
  if (file.includes("PropertyPanel") || file.includes("AIPanel")) { return true; }

  // --- AgentGeneratorModal prompt templates ---
  if (file.includes("AgentGeneratorModal")) { return true; }

  // --- browserMock.ts: mock data ---
  if (file.includes("browserMock")) { return true; }

  // --- chartGenerator regex ---
  if (file.includes("chartGenerator")) { return true; }

  // --- searchUtils template strings (for LLM context) ---
  if (file.includes("searchUtils")) { return true; }

  // --- workflowLayout constants/comments ---
  if (file.includes("workflowLayout")) { return true; }

  // --- store files: comments / field annotations ---
  if (file.startsWith("src/stores/") && /\/\/.*[\u4e00-\u9fff]/.test(text)) { return true; }

  // --- types files: field comments ---
  if (file.startsWith("src/types/") && /\/\/.*[\u4e00-\u9fff]/.test(text)) { return true; }

  // --- expertPresets.ts: has nameKey/descKey i18n keys, strings are fallbacks ---
  if (file.includes("expertPresets")) { return true; }

  // --- constants.ts language labels (has labelKey fallback) ---
  if (file.includes("constants.ts") && /label.*[\u4e00-\u9fff]/.test(text)) { return true; }

  // --- shortcuts.ts app name data (interpolated into shortcutExternalConflictTip) ---
  if (file.includes("shortcuts.ts") && /\bapps\b/.test(text)) { return true; }

  // --- invoke.ts IPC diagnostic messages ---
  if (file.includes("invoke.ts")) { return true; }

  // --- ProviderSettings.tsx block comments ---
  if (file.includes("ProviderSettings") && /\/\*/.test(text)) { return true; }

  // --- useGlobalShortcutManager error messages (via i18n.t) ---
  if (file.includes("useGlobalShortcutManager") && /理由/.test(text)) { return true; }

  // --- PromptImportModal.tsx fallback name ---
  if (file.includes("PromptImportModal") && /未命名/.test(text)) { return true; }

  // --- sdk/index.ts internal error messages ---
  if (file.includes("sdk/index.ts")) { return true; }

  // --- McpContainerNode string matching ---
  if (file.includes("McpContainerNode")) { return true; }

  // --- onboardingStore error template ---
  if (file.includes("onboardingStore")) { return true; }

  // --- streamStore timeout constant ---
  if (file.includes("streamStore")) { return true; }

  // --- proactiveStore keyword matching ---
  if (file.includes("proactiveStore")) { return true; }

  // --- StructuredThinking keyword arrays ---
  if (file.includes("StructuredThinking")) { return true; }

  return false; // This IS a UI string that needs i18n
}

const uiViolations = [];

for (const fp of scanFiles(ROOT)) {
  const relPath = path.relative(ROOT, fp).replace(/\\/g, "/");
  if (!relPath.startsWith("src/")) { continue; }
  const lines = fs.readFileSync(fp, "utf8").split("\n");
  const techLines = [];

  for (let i = 0; i < lines.length; i++) {
    const ln = i + 1;
    const text = lines[i];
    if (!CJK_RE.test(text)) { continue; }

    // Skip if already in allowlist
    const existing = entryMap.get(relPath);
    if (existing) {
      const allowed = new Set(existing.lines.split(",").map(s => s.trim()));
      if (allowed.has(String(ln))) { continue; }
    }

    if (isTechnical(relPath, ln, text)) {
      techLines.push(ln);
    } else {
      uiViolations.push({ file: relPath, line: ln, text: text.trim() });
    }
  }

  if (techLines.length > 0) {
    if (entryMap.has(relPath)) {
      const ex = entryMap.get(relPath);
      const set = new Set(ex.lines.split(",").map(s => s.trim()));
      for (const ln of techLines) { set.add(String(ln)); }
      ex.lines = [...set].sort((a, b) => Number(a) - Number(b)).join(",");
    } else {
      allowlist.entries.push({
        file: relPath,
        lines: techLines.join(","),
        reason: "硬编码中文字符串",
        phase: 3,
      });
    }
  }
}

allowlist.entries.sort((a, b) => a.file.localeCompare(b.file));
allowlist.total_entries = allowlist.entries.length;
allowlist.total_files = [...new Set(allowlist.entries.map(e => e.file))].length;

fs.writeFileSync(ALLOWLIST_PATH, JSON.stringify(allowlist, null, 2) + "\n");

console.log(`Allowlist: ${allowlist.total_entries} entries in ${allowlist.total_files} files`);
console.log(`\nUI violations that need i18n (${uiViolations.length}):`);
for (const v of uiViolations.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line)) {
  console.log(`  ${v.file}:${v.line}: ${v.text}`);
}
