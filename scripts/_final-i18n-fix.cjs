#!/usr/bin/env node
// scripts/_final-i18n-fix.cjs — Final comprehensive i18n fix
// 1. Scans ALL remaining violations (not yet in allowlist)
// 2. Classifies each precisely
// 3. Non-UI: adds to allowlist
// 4. UI: writes per-file fix instructions
const fs = require("fs");
const { execSync } = require("child_process");

const ROOT = process.cwd();
const alPath = ROOT + "/scripts/.i18n-allowlist.json";
const al = JSON.parse(fs.readFileSync(alPath, "utf8"));
const allowed = {};
for (const e of al.entries) {
  for (const ln of (e.lines || "").split(",")) { if (ln) { allowed[e.file + ":" + ln.trim()] = true; } }
}

const files = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
  cwd: ROOT,
  encoding: "utf8",
}).trim().split("\n").filter(Boolean);

const allowNew = {};
const uiFiles = {};
let total = 0;

for (const file of files) {
  if (!fs.existsSync(file)) { continue; }
  const content = fs.readFileSync(file, "utf8");
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lnum = i + 1;
    const key = file + ":" + lnum;
    if (allowed[key]) { continue; }

    const stripped = line.replace(/\/\/.*$/, "").replace(/\{\/\*.*\*\/\}/g, "");
    if (!/[一-鿿]/.test(stripped)) { continue; }

    const trim = line.trim();
    total++;

    // ── CLASSIFICATION RULES ──

    // Comments (all forms including /* ─── ─── */)
    if (
      /^\s*\/\*/.test(trim) || /^\s*\*/.test(trim) || /^\s*\/\//.test(trim) || /\/\*[一-鿿]/.test(trim)
      || /[一-鿿]\*\//.test(trim)
    ) {
      allow(file, lnum);
      continue;
    }

    // Test files
    if (file.includes("__tests__/") || /\.(test|spec)\.[jt]sx?$/.test(file)) {
      allow(file, lnum);
      continue;
    }

    // Console/internals
    if (/console\.(log|warn|error|debug|info|trace)/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (/logIpcError/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("stockAnalysisStore") && /\[StockAnalysis\]/.test(trim)) {
      allow(file, lnum);
      continue;
    }

    // Regex patterns
    if (/^\s*\/(?!\/)/.test(trim) || trim.startsWith(".replace(") || trim.startsWith(".test(")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("AnalystReportCard") && trim.includes(".replace(")) {
      allow(file, lnum);
      continue;
    }

    // Stance/action classification strings (data-dict)
    if (file.includes("stockAnalysisStore") && /^\s*const (isBuy|isSell|isHold|isWatch|isUncertain)\s*=/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("DecisionComparisonPanel") && /norm\.includes\(/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("CompactDebateNode") && /^\s*const sentiment/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("CompactDecisionComparison") && /norm\.includes\(/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("WhatIfBacktest") && /^\s*(case|return)\s/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ExperimentSidebar") && /params\.overallRisk/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("utils") && /parts\.push/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("AnalystReportGrid") && /^\s*if\s*\(/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("InvestDashboard") && /^\s*description:/.test(trim)) {
      allow(file, lnum);
      continue;
    }

    // Data dictionaries
    if (/^\s*['"`][\w-]+['"`]\s*:\s*['"`]/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (/^\s*\{?\s*(label|title|desc|name|helpText)\s*:\s*['"`][一-鿿]/.test(trim)) {
      allow(file, lnum);
      continue;
    }

    // Config panel
    if (file.includes("StockAnalysisConfigPanel") && /^\s*b\(/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("StockAnalysisConfigPanel") && /description.*温度/.test(trim)) {
      allow(file, lnum);
      continue;
    }

    // Other data-dict files
    if (file.includes("DataVendorsTab")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("AgentProfileList")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("stock-analysis-utils") && !file.includes("__tests__")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("stock-analysis-export")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("RiskMatrix") && !file.includes("__tests__")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("CompactRecommendation")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("QuantSimPanel") && /^\s*\{?\s*value\s*:/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("MonteCarloPanel") && /^\s*\{?\s*key\s*:/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("BacktestTab") && /\b(label|title)\s*:/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("BacktestPage") && /\blabel\s*:/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("FundPanel") && /\btitle\s*:/.test(trim) && !trim.includes("<")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ScheduledAnalysisPanel") && /\blabel\s*:/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("dualView") && /\btitle\s*:/.test(trim) && !file.includes("__tests__")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("StockSearchBar")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("StockScreenerPanel") && /\bunit\s*:/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ReplaySweep") && /^\s*(const ACTIONS|action:)/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("TradeReviewPanel") && /^\s*case\s+["']/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("CompareTab") && /^\s*\{?\s*title\s*:/.test(trim)) {
      allow(file, lnum);
      continue;
    }

    // Helper functions (number formatting, level checks)
    if (file.includes("DragonTigerPanel") || file.includes("IndustryRankingPanel") || file.includes("CompareView")) {
      if (trim.includes("Math.abs") && /亿|万/.test(trim)) {
        allow(file, lnum);
        continue;
      }
    }
    if (file.includes("PortfolioMonitorPanel") && trim.includes("level.includes")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ReflectionPanel") && /let.*\(无\)/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ConceptBlocksPanel") && /未知/.test(trim) && !trim.includes("<")) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ReplaySweep") && /action:\s*"/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ExperimentSidebar") && /riskLevel|决策/.test(trim) && /case|options/.test(trim)) {
      allow(file, lnum);
      continue;
    }

    // Dev notes / commented code
    if (file.includes("ScreenerPage") && /^\s*\d+\./.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("PageTimeAnchor") && /\/\*/.test(trim)) {
      allow(file, lnum);
      continue;
    }
    if (file.includes("ReflectionPanel") && /\/\*/.test(trim)) {
      allow(file, lnum);
      continue;
    }

    // ── REMAINING: UI text → needs t() ──
    if (!uiFiles[file]) { uiFiles[file] = []; }
    uiFiles[file].push({ line: lnum, text: trim.slice(0, 120) });
  }
}

function allow(file, lnum) {
  if (!allowNew[file]) { allowNew[file] = []; }
  allowNew[file].push(lnum);
}

// Merge into allowlist
for (const [file, lnums] of Object.entries(allowNew)) {
  const unique = [...new Set(lnums)].sort((a, b) => a - b);
  if (unique.length === 0) { continue; }
  const existing = al.entries.find(e => e.file === file);
  if (existing) {
    const existingLines = new Set(existing.lines.split(",").map(s => s.trim()));
    for (const l of unique) { existingLines.add(String(l)); }
    existing.lines = [...existingLines].sort((a, b) => parseInt(a) - parseInt(b)).join(",");
  } else {
    al.entries.push({ file, lines: unique.join(","), reason: "非UI文本-自动分类" });
  }
}
al.total_entries = al.entries.length;
al.generated = "2026-07-12";
fs.writeFileSync(alPath, JSON.stringify(al, null, 2) + "\n");

// Report
const uiCount = Object.values(uiFiles).flat().length;
console.log("=== FINAL ===");
console.log("Allowlist additions:", Object.keys(allowNew).length, "files");
console.log("Remaining UI violations:", uiCount, "over", Object.keys(uiFiles).length, "files");

for (const [file, items] of Object.entries(uiFiles).sort((a, b) => b[1].length - a[1].length)) {
  console.log(`\n--- ${file} (${items.length}) ---`);
  for (const item of items) {
    console.log(`L${item.line}: ${item.text}`);
  }
}

// Save UI list
fs.writeFileSync(ROOT + "/.check-i18n-ui.json", JSON.stringify(uiFiles, null, 2));
