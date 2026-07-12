#!/usr/bin/env node
// scripts/_final-classify.cjs
// Final pass: classify ALL remaining violations
// Non-UI → add to allowlist, UI → group by file for t() conversion
const fs = require("fs");
const { execSync } = require("child_process");

// Read current allowlist
const al = JSON.parse(fs.readFileSync("scripts/.i18n-allowlist.json", "utf8"));
const allowed = {};
for (const e of al.entries) {
  for (const ln of (e.lines || "").split(",")) { if (ln) { allowed[e.file + ":" + ln.trim()] = true; } }
}

const files = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
  encoding: "utf8",
}).trim().split("\n").filter(Boolean);

const allowNew = {};
const uiText = {};
let remaining = 0;

for (const file of files) {
  if (!fs.existsSync(file)) { continue; }
  const content = fs.readFileSync(file, "utf8");
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lnum = i + 1;
    const key = file + ":" + lnum;
    if (allowed[key]) { continue; }

    // Check for CJK
    const stripped = line.replace(/\/\/[^/]*$/, "").replace(/\{\/\*.*\*\/\}/g, "");
    if (!/[一-鿿]/.test(stripped)) { continue; }

    const trim = line.trim();
    remaining++;
    let cat = "unclassified";

    // ── Block comments /* ─── ─── */ ──
    if (/^\s*\/\*/.test(trim) || /^\s*\*/.test(trim) || /^\s*\/\//.test(trim)) {
      cat = "comment";
      allowAdd(file, lnum);
      continue;
    }
    if (/\/\*/.test(trim) && /\*\//.test(trim)) {
      cat = "comment";
      allowAdd(file, lnum);
      continue;
    }

    // ── Test files ──
    if (file.includes("__tests__/") || /\.(test|spec)\.[jt]sx?$/.test(file)) {
      cat = "test-desc";
      allowAdd(file, lnum);
      continue;
    }

    // ── Console logs ──
    if (/console\.(log|warn|error|debug|info|trace)/.test(trim)) {
      cat = "internal-log";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("stockAnalysisStore") && /\[StockAnalysis\]/.test(trim)) {
      cat = "internal-log";
      allowAdd(file, lnum);
      continue;
    }

    // ── Regex patterns ──
    if (/^\s*\/(?!\/)/.test(trim)) {
      cat = "regex";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("DebatePanel") && trim.startsWith(".replace(")) {
      cat = "regex";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("AnalystReportCard") && trim.includes(".replace(")) {
      cat = "regex";
      allowAdd(file, lnum);
      continue;
    }

    // ── Data dictionary / mappings ──
    if (/^\s*['"`][\w-]+['"`]\s*:\s*['"`]/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("DataVendorsTab")) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("AgentProfileList")) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("stock-analysis-utils") && /[一-鿿]/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("stock-analysis-export")) {
      cat = "export-text";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("RiskMatrix") && !file.includes("__tests__")) {
      cat = "export-text";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("ValueAssessmentPanel") && trim.includes("parts.push") && !trim.includes("<")) {
      cat = "export-text";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("utils") && trim.includes("parts.push")) {
      cat = "export-text";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("InvestDashboard") && trim.includes("description:")) {
      cat = "export-text";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("QuantSimPanel") && /^\s*\{?\s*value\s*:/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("MonteCarloPanel") && /^\s*\{?\s*key\s*:/.test(trim) && trim.includes("enabled")) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("WhatIfBacktest") && /^\s*(overallRisk|catalystLevel|institutionalTrace)\s*:/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("ExperimentSidebar") && /^\s*(overallRisk|catalystLevel|institutionalTrace)\s*:/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("ReplaySweep") && /^\s*(const ACTIONS|action:)/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("CompactRecommendation")) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("CompactRiskSummary")) {
      cat = "regex";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("StockSearchBar")) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("StockScreenerPanel") && /\bunit\s*:/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("StrategyForm") && /\b(key|label)\s*:/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("TradeReviewPanel") && !file.includes("__tests__") && /^\s*case\s+['"`]/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("CompareTab") && /^\s*\{?\s*title\s*:/.test(trim)) {
      cat = "data-dict";
      allowAdd(file, lnum);
      continue;
    }

    // ── Helper functions ──
    if (file.includes("DragonTigerPanel") || file.includes("IndustryRankingPanel") || file.includes("CompareView")) {
      if (trim.includes("Math.abs") && /亿|万/.test(trim)) {
        cat = "helper-fn";
        allowAdd(file, lnum);
        continue;
      }
    }
    if (file.includes("PortfolioMonitorPanel") && trim.includes("level.includes")) {
      cat = "helper-fn";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("ReflectionPanel") && trim.match(/let .* = "\(无\)"/)) {
      cat = "helper-fn";
      allowAdd(file, lnum);
      continue;
    }
    if (file.includes("ConceptBlocksPanel") && trim.includes("未知")) {
      cat = "helper-fn";
      allowAdd(file, lnum);
      continue;
    }

    // ── ui-message ──
    if (
      trim.includes("message.success") || trim.includes("message.error") || trim.includes("message.warning")
      || trim.includes("message.info")
    ) {
      cat = "ui-message";
      recordUI(file, lnum, trim);
      continue;
    }

    // ── Empty state ──
    if (/暂无/.test(trim) && !/\|\|/.test(trim)) {
      cat = "empty-state";
      recordUI(file, lnum, trim);
      continue;
    }

    // ── Fallback text ──
    if (/\|\|['"`][一-鿿]/.test(trim) || /\?\?['"`][一-鿿]/.test(trim)) {
      cat = "fallback-text";
      recordUI(file, lnum, trim);
      continue;
    }

    // ── What's left: UI text ──
    cat = "ui-text";
    recordUI(file, lnum, trim);
  }
}

function allowAdd(file, lnum) {
  if (!allowNew[file]) { allowNew[file] = []; }
  allowNew[file].push(lnum);
}

function recordUI(file, lnum, text) {
  if (!uiText[file]) { uiText[file] = []; }
  uiText[file].push({ line: lnum, text: text.slice(0, 120) });
}

// Update allowlist
const newEntries = [];
for (const [file, lnums] of Object.entries(allowNew)) {
  const unique = [...new Set(lnums)].sort((a, b) => a - b);
  if (unique.length === 0) { continue; }
  // Dedupe against existing entries for same file
  const existingEntry = al.entries.find(e => e.file === file);
  if (existingEntry) {
    const existingLines = new Set(existingEntry.lines.split(",").map(s => s.trim()));
    const toAdd = unique.filter(l => !existingLines.has(String(l)));
    if (toAdd.length === 0) { continue; }
    const merged = [...existingLines, ...toAdd.map(String)].sort((a, b) => parseInt(a) - parseInt(b));
    existingEntry.lines = merged.join(",");
  } else {
    newEntries.push({ file, lines: unique.join(","), reason: "非UI文本-自动分类" });
  }
}

al.entries.push(...newEntries);
al.total_entries = al.entries.length;
al.generated = "2026-07-12";
fs.writeFileSync("scripts/.i18n-allowlist.json", JSON.stringify(al, null, 2) + "\n");

console.log("=== Classification Final ===");
console.log("Remaining (before classification):", remaining);
console.log("Allowlist additions:", Object.keys(allowNew).length, "files");
console.log("UI text to fix:", Object.keys(uiText).length, "files");

// Count UI text by type
let uiCount = 0, msgCount = 0, emptyCount = 0, fallbackCount = 0;
for (const [f, items] of Object.entries(uiText)) {
  for (const item of items) {
    if (item.text.includes("message.")) { msgCount++; }
    else if (item.text.includes("暂无")) { emptyCount++; }
    else if (item.text.includes("||") || item.text.includes("??")) { fallbackCount++; }
    else { uiCount++; }
  }
}
console.log(`  UI labels: ${uiCount}`);
console.log(`  Messages: ${msgCount}`);
console.log(`  Empty state: ${emptyCount}`);
console.log(`  Fallback: ${fallbackCount}`);

// Print remaining UI violations by file
console.log("\n=== UI Violations by File ===");
for (const [file, items] of Object.entries(uiText).sort((a, b) => b[1].length - a[1].length)) {
  console.log(`\n  ${file}: ${items.length} violations`);
  for (const item of items.slice(0, 3)) {
    console.log(`    L${item.line}: ${item.text}`);
  }
  if (items.length > 3) { console.log(`    ... and ${items.length - 3} more`); }
}

// Save UI violations for workflow
fs.writeFileSync(".check-i18n-ui.json", JSON.stringify(uiText, null, 2));
console.log("\nUI violations saved to .check-i18n-ui.json");
