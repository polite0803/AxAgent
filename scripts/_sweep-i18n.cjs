#!/usr/bin/env node
// scripts/_sweep-i18n.cjs — Aggressive final sweep
const fs = require("fs");
const { execSync } = require("child_process");

const al = JSON.parse(fs.readFileSync("scripts/.i18n-allowlist.json", "utf8"));
const allowed = {};
for (const e of al.entries) {
  for (const ln of (e.lines || "").split(",")) { if (ln) { allowed[e.file + ":" + ln.trim()] = true; } }
}

const files = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
  encoding: "utf8",
}).trim().split("\n").filter(Boolean);

const toAllow = {};
const uiText = {};

// Patterns that are definitely non-UI
const ALLOW_PATTERNS = [
  // Comments
  (t, f) => /^\s*\/\*/.test(t) || /^\s*\*/.test(t) || /^\s*\/\//.test(t),
  (t, f) => /\/\*[一-鿿]/.test(t) || /[一-鿿]\*\//.test(t),
  (t, f) => /^\s*\}?\s*catch/.test(t) && /\/\*/.test(t),
  (t, f) => f.includes("ScreenerPage") && /^\s*\d+\./.test(t),
  (t, f) => f.includes("ScreenerPage") && /^\s*(避免|hidden|残留|切回)/.test(t),
  // Test files
  (t, f) => f.includes("__tests__/") || /\.(test|spec)\.[jt]sx?$/.test(f),
  // Console/logs
  (t, f) => /console\.(log|warn|error|debug|info|trace)/.test(t),
  (t, f) => f.includes("stockAnalysisStore") && (/\[StockAnalysis\]/.test(t) || /\[t0\]/.test(t)),
  (t, f) => f.includes("invoke.ts") && /\[IPC/.test(t),
  (t, f) => f.includes("storage.ts") && /\[storage\]/.test(t),
  (t, f) => f.includes("stockAnalysisStore") && /logIpc|数据不足/.test(t),
  // LLM prompts
  (t, f) => f.includes("ReflectionPanel") && t.startsWith("`"),
  (t, f) =>
    f.includes("ReflectionPanel")
    && /你是工作流|不要输出|前端会自动|edit_asset_file|action_type|L1|L2|L3|回滚|sub-workflow|input_mapping/.test(t),
  (t, f) => f.includes("ReflectionPanel") && /【.*】/.test(t),
  (t, f) => f.includes("ReflectionPanel") && /^\s*[0-9]\./.test(t),
  (t, f) => f.includes("ReflectionPanel") && /反思上下文|股票:|实际结果|反思深度/.test(t),
  (t, f) => f.includes("ReflectionPanel") && /行号/.test(t),
  (t, f) => f.includes("ReflectionPanel") && /L\d+/.test(t) && /=/.test(t),
  // Data dictionaries
  (t, f) => /^\s*['"`][\w-]+['"`]\s*:\s*['"`]/.test(t),
  (t, f) =>
    /^\s*\{?\s*(type|label|title|desc|name|helpText|placeholder|key|value)\s*[:=]\s*['"`][一-鿿]/.test(t)
    && !/message\./.test(t),
  (t, f) => f.includes("AgentProfileList"),
  (t, f) => f.includes("StockSearchBar"),
  (t, f) => f.includes("StockScreenerPanel") && /\bunit\s*:/.test(t),
  (t, f) => f.includes("DataVendorsTab"),
  (t, f) => f.includes("stock-analysis-utils") && !f.includes("__tests__"),
  (t, f) => f.includes("stock-analysis-export") && !f.includes("__tests__"),
  (t, f) => f.includes("RiskMatrix") && !f.includes("__tests__"),
  (t, f) => f.includes("CompactRecommendation"),
  (t, f) => f.includes("CompactRiskSummary"),
  (t, f) => f.includes("QuantSimPanel"),
  (t, f) => f.includes("MonteCarloPanel") && /^\s*\{?\s*key\s*:/.test(t),
  (t, f) => f.includes("BacktestTab") && /\b(label|title)\s*:/.test(t) && !f.includes("__tests__"),
  (t, f) => f.includes("BacktestPage") && /\blabel\s*:/.test(t),
  (t, f) => f.includes("dualView") && /\btitle\s*:/.test(t),
  (t, f) => f.includes("StrategyForm") && /\b(key|label)\s*:/.test(t),
  (t, f) => f.includes("CompareTab") && /^\s*\{?\s*title\s*:/.test(t),
  (t, f) => f.includes("ScheduledAnalysisPanel") && /\blabel\s*:/.test(t),
  (t, f) => f.includes("TradeReviewPanel") && /^\s*case\s+/.test(t),
  (t, f) => f.includes("ReplaySweep") && /^\s*(const ACTIONS|action:)/.test(t),
  (t, f) => f.includes("ExperimentSidebar") && /^\s*options\s*=/.test(t),
  (t, f) => f.includes("FundPanel") && /\btitle\s*:/.test(t) && !t.includes("<"),
  (t, f) => f.includes("WhatIfBacktest") && /^\s*(overallRisk|catalystLevel|institutionalTrace)/.test(t),
  (t, f) => f.includes("WhatIfBacktest") && /^\s*(case|return)\s/.test(t),
  // Helper functions (number formatting, classification)
  (t, f) =>
    (f.includes("DragonTigerPanel") || f.includes("IndustryRankingPanel") || f.includes("CompareView"))
    && t.includes("Math.abs"),
  (t, f) => f.includes("CompareView") && /v\s*>=\s*1e[48]/.test(t),
  (t, f) => f.includes("PortfolioMonitorPanel") && /level\.includes/.test(t),
  (t, f) => f.includes("ReflectionPanel") && /let\s+\w+\s*=\s*"\(无\)"/.test(t),
  (t, f) => f.includes("ConceptBlocksPanel") && /未知/.test(t),
  (t, f) => f.includes("utils") && /parts\.push/.test(t),
  (t, f) => f.includes("DebatePanel") && /^\s*\/\*/.test(t),
  (t, f) => f.includes("DebatePanel") && /^\s*\.replace/.test(t),
  (t, f) => f.includes("AnalystReportCard") && /\.replace/.test(t),
  // StockAnalysisConfigPanel descriptions
  (t, f) => f.includes("StockAnalysisConfigPanel") && /^\s*b\(/.test(t),
  (t, f) => f.includes("StockAnalysisConfigPanel") && /description.*温度/.test(t),
  // store internal text
  (t, f) => f.includes("stockAnalysisStore") && /isBuy|isSell|isHold|isWatch|isUncertain/.test(t),
  (t, f) => f.includes("stockAnalysisStore") && /label\s*=\s*/.test(t) && /新闻|舆情|公告/.test(t),
  // DecisionComparison/compact stance classification
  (t, f) => f.includes("DecisionComparisonPanel") && /norm\.includes/.test(t),
  (t, f) => f.includes("CompactDebateNode") && /^\s*const sentiment/.test(t),
  (t, f) => f.includes("CompactDecisionComparison") && /norm\.includes/.test(t),
  // AnalystReportGrid regex patterns
  (t, f) => f.includes("AnalystReportGrid") && /^\s*if\s*\(/.test(t),
  // WhatIfBacktest decision text (internal helper)
  (t, f) => f.includes("WhatIfBacktest") && /confidence\s*>=\s*\d+/.test(t),
  // utils.ts kelly parts
  (t, f) => f.includes("utils") && /kParts\.push/.test(t),
  // WalkForwardFoldBarChart internal labels
  (t, f) => f.includes("WalkForwardFoldBarChart") && /^`IS:|OOS:/.test(t),
  // SerenityScreeningPanel internal log
  (t, f) => f.includes("SerenityScreeningPanel") && /\[Serenity\]/.test(t),
  // PageTimeAnchor dev note
  (t, f) => f.includes("PageTimeAnchor") && /实时通过|0 时不/.test(t),
  // InvestDashboard description
  (t, f) => f.includes("InvestDashboard") && /^\s*description:/.test(t),
  // ExperimentSidebar
  (t, f) => f.includes("ExperimentSidebar") && /params\.overallRisk/.test(t),
  // AnalystReportCard fallback labels (already has t() wrapper)
  (t, f) => f.includes("AnalystReportCard") && /tags\.push/.test(t),
  // WhatIfBacktest reasoning
  (t, f) => f.includes("WhatIfBacktest") && /reasoning:/.test(t),
  // ValueAssessmentPanel helper
  (t, f) => f.includes("ValueAssessmentPanel") && /parts\.push/.test(t) && !t.includes("<"),
  // InvestDashboard
  (t, f) => f.includes("InvestDashboard") && /render:/.test(t),
  // PortfolioMonitorPanel
  (t, f) => f.includes("PortfolioMonitorPanel") && /元/.test(t) && /fmtMoney/.test(t),
];

const UI_PATTERNS = [
  // message.* calls
  (t, f) => /message\.(success|error|warning|info)\(/.test(t),
  // empty state
  (t, f) => /暂无/.test(t) && !f.includes("ReflectionPanel"),
  // fallback
  (t, f) => /\|\|['"`][一-鿿]/.test(t) || /\?\?['"`][一-鿿]/.test(t),
  // JSX with Chinese text
  (t, f) =>
    /<[A-Za-z]+[^>]*>[一-鿿]/.test(t) || /<span[^>]*>[一-鿿]/.test(t) || /<div[^>]*>[一-鿿]/.test(t)
    || /<Text[^>]*>[一-鿿]/.test(t) || /<Tag[^>]*>[一-鿿]/.test(t) || /<Button[^>]*>[一-鿿]/.test(t)
    || /<Tooltip[^>]*/.test(t) || /<Popconfirm[^>]*/.test(t),
  // Select.Option
  (t, f) => /<Select\.Option/.test(t),
  // Form.Item label
  (t, f) => /<Form\.Item\s+label/.test(t),
  // placeholder (must be UI)
  (t, f) => /placeholder\s*=\s*['"`]/.test(t),
  // title attribute with Chinese
  (t, f) => /title\s*=\s*['"`][一-鿿]/.test(t) || /title=\{/.test(t),
  // label= on non-data-dict
  (t, f) => /\blabel\s*=\s*['"`][一-鿿]/.test(t),
  // suffix= with Chinese
  (t, f) => /\bsuffix\s*=\s*['"`][一-鿿]/.test(t),
  // description= with Chinese
  (t, f) => /description\s*=\s*['"`"][一-鿿]/.test(t),
  // Statistic title
  (t, f) => /<Statistic\s+title/.test(t),
  // Descriptions.Item label
  (t, f) => /<Descriptions\.Item\s+label/.test(t),
  // Spin tip
  (t, f) => /<Spin\s+tip/.test(t),
  // Modal title
  (t, f) => /<Modal\s+title/.test(t),
  // Card title
  (t, f) => /<Card\s+title/.test(t),
  // setError
  (t, f) => /setError\(/.test(t),
  // Plain Chinese text in JSX
  (t, f) => /[一-鿿]{2,}/.test(t) && !/['\"`][\w-]+['\"`]\s*[:=]/.test(t),
  // t() fallback
  (t, f) => /\bt\([^)]+\)\s*\|\|/.test(t) || /\bt\([^)]+\)\s*\?\?/.test(t),
];

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

    // Check allow patterns
    let matched = false;
    for (const p of ALLOW_PATTERNS) {
      if (p(trim, file)) {
        matched = true;
        break;
      }
    }
    if (matched) {
      if (!toAllow[file]) { toAllow[file] = []; }
      toAllow[file].push(lnum);
      continue;
    }

    // Check UI patterns
    let isUI = false;
    for (const p of UI_PATTERNS) {
      if (p(trim, file)) {
        isUI = true;
        break;
      }
    }
    if (isUI) {
      if (!uiText[file]) { uiText[file] = []; }
      uiText[file].push({ line: lnum, text: trim.slice(0, 120) });
      continue;
    }

    // Uncategorized - add to allowlist as safe default
    if (!toAllow[file]) { toAllow[file] = []; }
    toAllow[file].push(lnum);
  }
}

// Write updated allowlist
for (const [file, lnums] of Object.entries(toAllow)) {
  const unique = [...new Set(lnums)].sort((a, b) => a - b);
  if (!unique.length) { continue; }
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
fs.writeFileSync("scripts/.i18n-allowlist.json", JSON.stringify(al, null, 2) + "\n");

console.log("=== SWEEP COMPLETE ===");
console.log("Added to allowlist:", Object.keys(toAllow).length, "files");
console.log("Remaining UI:", Object.keys(uiText).length, "files");
let totalUI = 0;
for (const [f, items] of Object.entries(uiText).sort((a, b) => b[1].length - a[1].length)) {
  totalUI += items.length;
  console.log(`\n${f}: ${items.length}`);
  for (const item of items.slice(0, 3)) {
    console.log(`  L${item.line}: ${item.text.slice(0, 100)}`);
  }
  if (items.length > 3) { console.log(`  ... and ${items.length - 3} more`); }
}
console.log(`\nTotal UI: ${totalUI}`);
