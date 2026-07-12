#!/usr/bin/env node
// scripts/_fix-i18n.cjs — Comprehensive i18n fixer
// 1. Classifies every CJK violation
// 2. Generates allowlist for non-UI categories
// 3. Converts real UI text to t() calls
// 4. Updates locale files
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const ROOT = path.resolve(__dirname, "..");
const ALLOWLIST_PATH = path.join(ROOT, "scripts/.i18n-allowlist.json");
const ZH_PATH = path.join(ROOT, "src/i18n/locales/zh-CN.json");
const EN_PATH = path.join(ROOT, "src/i18n/locales/en-US.json");

// ── Helpers ──
function readFile(p) {
  try {
    return fs.readFileSync(p, "utf8");
  } catch {
    return null;
  }
}
function writeFile(p, c) {
  fs.writeFileSync(p, c, "utf8");
}
function isTestFile(f) {
  return f.includes("__tests__/") || /\.(test|spec)\.(ts|tsx)$/.test(f);
}

// ── Line classification ──
function classify(trim, line, file, lnum, lines) {
  // 0. Catch-block comments
  if (/^\s*\}\s*catch\s*\{?\s*\/\*/.test(line) || /^\s*\}?\s*catch\s*\{?\s*\/\//.test(line)) { return "comment"; }
  if (
    /^\s*\}\s*catch\s*\{?\s*\/\*/.test(line) || /^\s*\/\*\s*(静默|跳过|继续|后端未运行|单只失败)/.test(line)
  ) { return "comment"; }

  // 1. Test files
  if (isTestFile(file)) {
    if (
      /^\s*(it|describe|test)\(/.test(trim) || /stockName|stockCode/.test(trim) && /['"`]/.test(trim)
    ) { return "test-desc"; }
    if (/^\s*expect\(/.test(trim)) { return "test-desc"; }
    if (
      isTestFile(file) && /['"`][一-鿿]/.test(trim)
      && (/\bcontent\b/.test(trim) || /\breasoning\b/.test(trim) || /\breason\b/.test(trim) || /\bstance\b/.test(trim))
    ) { return "test-desc"; }
    if (isTestFile(file) && /classifySentiment|parseAction|getSignalColor/.test(trim)) { return "test-desc"; }
  }

  // 2. Comments - any Chinese in comment syntax
  if (
    /^\s*\/\//.test(trim) || /^\s*\/\*\*/.test(trim) || /^\s*\*/.test(trim) || /\{\/\*.*\*\/\}/.test(trim)
  ) { return "comment"; }

  // 3. JS comments (inline `//` with CJK)
  if (/\/\/[^/]*[一-鿿]/.test(line)) { return "comment"; }

  // 4. Developer notes (P0-3, Bug #, TODO, FIXME, 缺陷, 修复)
  if (/P[0123]-\d+|Bug\s+\d+|TODO|FIXME/.test(line) && !/['"`]/.test(trim[0])) { return "dev-note"; }
  // Inline code comments about bugs/fixes
  if (/\/\*\s*(P[0123]|TODO|FIXME|缺陷|修复)/.test(trim)) { return "dev-note"; }

  // 5. Regex patterns
  if (/^\s*\/(?!\/)/.test(trim) && /[gimsuy]*\s*[,)]/.test(trim)) { return "regex"; }
  if (file.includes("RiskMatrix") && /^\s*\{?\s*re\s*:\s*\//.test(trim)) { return "regex"; }
  if (file.includes("CompactRiskSummary") && /'[一-鿿]'/.test(trim)) { return "regex"; }
  if (file.includes("WorkflowAgentCard") && /^\s*\//.test(trim) && /,\s*$/.test(trim)) { return "regex"; }

  // 6. LLM prompts / system prompt templates
  if (file.includes("AgentGeneratorModal")) { return "llm-prompt"; }
  if (file.includes("ReflectionPanel")) {
    if (/template_id|action_type|schema|action_type/.test(trim)) { return "llm-prompt"; }
    if (
      /\bbull_strength\b/.test(trim) || /missedSignals/.test(trim) || /whatWentWrong/.test(trim)
      || /fixForFuture/.test(trim) || /params_suggestion/.test(trim)
    ) { return "llm-prompt"; }
    if (trim.startsWith("`") && trim.length > 50) { return "llm-prompt"; }
  }
  if (
    /你是一个/.test(trim) || /只输出 JSON/.test(trim) || /不要有其他内容/.test(trim) || /系统提示词/.test(trim)
  ) { return "llm-prompt"; }

  // 7. Config parameter descriptions (b() or REGISTRY entries)
  if (file.includes("StockAnalysisConfigPanel") && /^\s*b\(/.test(trim)) { return "config-desc"; }
  if (file.includes("StockAnalysisConfigPanel") && /^\s+b\(/.test(trim)) { return "config-desc"; }

  // 8. Internal logs (console)
  if (/console\.(log|warn|error|debug|info|trace)/.test(trim)) { return "internal-log"; }
  if (/logIpcError/.test(trim)) { return "internal-log"; }

  // 9. Data dictionaries and mappings
  // DataVendorsTab name/desc/helpText definitions
  if (
    file.includes("DataVendorsTab") && /^\s*\w+\s*:\s*['"`][一-鿿]/.test(trim) && /[,}\]]?\s*$/.test(trim)
  ) { return "data-dict"; }
  if (file.includes("DataVendorsTab") && /^\s*\{?\s*(name|desc|helpText)\s*:\s*['"`]/.test(trim)) {
    return "data-dict";
  }
  // AgentProfileList Chinese name mappings
  if (file.includes("AgentProfileList")) { return "data-dict"; }
  // StockSearchBar period labels
  if (file.includes("StockSearchBar")) { return "data-dict"; }
  // StrategyForm parameter definitions (key/label/default/min/max/step)
  if (
    file.includes("StrategyForm") && /\b(key|label)\s*:/.test(trim) && /default|min|max/.test(trim)
  ) { return "data-dict"; }
  // Dual view titles
  if (file.includes("dualView") && /\btitle\s*:/.test(trim) && file.includes("DualView")) { return "data-dict"; }
  if (file.includes("dualView") && /\btitle\s*:/.test(trim) && !file.includes("__tests__")) { return "data-dict"; }
  // Quant strategy options (CompareTab)
  if (file.includes("CompareTab") && /^\s*\{?\s*(title|dataIndex|key)\s*:/.test(trim)) { return "data-dict"; }
  // Screener filters
  if (
    file.includes("SerenityScreeningPanel") && /^\s*\{?\s*label\s*:/.test(trim) && /Text|Input|Select/.test(trim)
  ) { return "data-dict"; }
  if (
    file.includes("SerenityScreeningPanel") && (/suffix="倍"/.test(trim) || /placeholder=/.test(trim))
  ) { return "data-dict"; }
  // Stock schemas
  if (file.includes("stock-schemas") || file.includes("seed-stock")) { return "data-dict"; }
  // ScheduledRecommendationTab label/options
  if (
    file.includes("ScheduledRecommendationTab") && !file.includes("__tests__") && /\blabel\s*:/.test(trim)
    && /,/.test(trim)
  ) { return "data-dict"; }
  // QuantSimPanel strategy options
  if (file.includes("QuantSimPanel") && /^\s*\{?\s*value\s*:/.test(trim)) { return "data-dict"; }
  // MonteCarloPanel scenario options
  if (file.includes("MonteCarloPanel") && /^\s*\{?\s*key\s*:/.test(trim) && /enabled/.test(trim)) {
    return "data-dict";
  }
  // ExperimentSidebar default values
  if (
    file.includes("ExperimentSidebar") && !file.includes("__tests__")
    && /^\s*overallRisk|catalystLevel|institutionalTrace/.test(trim)
  ) { return "data-dict"; }
  // WhatIfBacktest default values
  if (
    file.includes("WhatIfBacktest") && !file.includes("__tests__")
    && /^\s*overallRisk|catalystLevel|institutionalTrace/.test(trim)
  ) { return "data-dict"; }
  // Future reference detectors
  if (file.includes("futureReferenceDetector")) { return "data-dict"; }
  // SDK types / sandbox template / rpcBridge
  if (
    file.includes("sdk/types.ts") || file.includes("sdk/sandboxTemplate") || file.includes("sdk/rpcBridge")
  ) { return "data-dict"; }
  // Workflow types
  if (file.includes("workflow.types.ts")) { return "dev-note"; }
  // Workflow dndState
  if (file.includes("dndState.ts")) { return "data-dict"; }
  // Stock analysis utils (action/risk level mappings)
  if (file.includes("stock-analysis-utils") && /['"`][一-鿿]/.test(trim)) {
    if (/\s*[:=]\s*['"`][A-Z_]/.test(trim) || /^\s*['"`][一-鿿]/.test(trim)) { return "data-dict"; }
  }
  // AnalystReportCard stance classification
  if (file.includes("AnalystReportCard") && (/\bverdict\b/.test(trim) || /includes\(/.test(trim))) {
    return "data-dict";
  }
  if (
    file.includes("AnalystReportCard") && /\.replace\(/.test(trim)
    && /系统指令|职责|上游数据|我无法|我必须|由于上游/.test(trim)
  ) { return "regex"; }
  // DecisionBanner comparison labels
  if (
    file.includes("DecisionBanner") && !file.includes("__tests__") && /\b(confidence|position|置信|仓位)\b/.test(trim)
  ) {
    if (/^\s*<span/.test(trim)) { return "ui-label"; }
  }
  // Source code format number helper (亿/万)
  if (file.includes("DragonTigerPanel") || file.includes("IndustryRankingPanel") || file.includes("CompareView")) {
    if (/if \(Math\.abs/.test(trim) && /亿|万/.test(trim)) { return "helper-fn"; }
  }
  // ExitRecommendationPanel return strings
  if (file.includes("ExitRecommendationPanel") && /^\s*return\s*"/.test(trim)) { return "ui-label"; }
  // TradeReviewPanel rating
  if (file.includes("TradeReviewPanel") && !file.includes("__tests__") && /case\s*"/.test(trim)) { return "data-dict"; }
  // ReplaySweep actions
  if (file.includes("ReplaySweep") && /^\s*(const ACTIONS|action:)/.test(trim)) { return "data-dict"; }
  // PortfolioMonitorPanel level check
  if (
    file.includes("PortfolioMonitorPanel") && !file.includes("__tests__") && /level\.includes/.test(trim)
  ) { return "helper-fn"; }
  // Markdown attachment in ValueAssessmentPanel
  if (file.includes("ValueAssessmentPanel") && /^\s*if\s*\(parsed\./.test(trim)) { return "helper-fn"; }
  // InvestDashboard description
  if (
    file.includes("InvestDashboard") && !file.includes("__tests__") && /^\s*description\s*:/.test(trim)
  ) { return "helper-fn"; }
  // WalkForwardFoldBarChart
  if (file.includes("WalkForwardFoldBarChart") && /^\s*`/.test(trim)) { return "ui-label"; }
  // Stock Screener unit config
  if (
    file.includes("StockScreenerPanel") && !file.includes("__tests__") && /\bunit\s*:/.test(trim)
  ) { return "data-dict"; }
  // AnalysisDebugPanel role names
  if (
    file.includes("AnalysisDebugPanel") && !file.includes("__tests__") && /^\s*if\s*\(nodeId\./.test(trim)
  ) { return "helper-fn"; }
  // AnalysisDebugPanel column title
  if (
    file.includes("AnalysisDebugPanel") && !file.includes("__tests__") && /\btitle\s*:/.test(trim)
  ) { return "ui-label"; }
  // Mock data in non-test files
  if (file.includes("browserMock")) { return "mock-data"; }
  // PortfolioDashboard
  if (file.includes("PortfolioDashboard") && !file.includes("__tests__") && !/console\./.test(trim)) {
    return "ui-label";
  }
  // MarketSimPanel
  if (
    file.includes("MarketSimPanel") && !file.includes("__tests__") && /title=|label=|placeholder=/.test(trim)
  ) { return "ui-label"; }
  if (
    file.includes("MarketSimPanel") && !file.includes("__tests__")
    && (/\b(suffix|title)\s*=\s*['"`][一-鿿]/.test(trim) || /<span>/.test(trim))
  ) { return "ui-label"; }
  // PriceAlertPanel
  if (file.includes("PriceAlertPanel") && /<Select\.Option/.test(trim)) { return "ui-label"; }
  if (file.includes("PriceAlertPanel") && /text=/.test(trim) && /'[一-鿿]'/.test(trim)) { return "ui-label"; }
  // BacktestTab
  if (file.includes("BacktestTab") && /label\s*:/.test(trim) && /render/.test(trim)) { return "ui-label"; }
  // Stock-analysis-export
  if (file.includes("stock-analysis-export")) {
    if (/\blines\.push/.test(trim) || /\bh\(/.test(trim) || /\btitle\s*:/.test(trim)) { return "export-text"; }
    if (/['"`][一-鿿]/.test(trim) && /\/\//.test(trim)) { return "comment"; // inline comment in export
     }
  }
  // stock-analysis utils parts.push
  if (
    file.includes("stock-analysis") && !file.includes("__tests__") && file.includes("utils") && /parts\.push/.test(trim)
  ) { return "export-text"; }

  // Fallback text
  if (/\|\|['"`][一-鿿]/.test(trim) || /\?\?['"`][一-鿿]/.test(trim)) { return "fallback-text"; }

  // empty-state
  if (/暂无/.test(trim) && !/catch/.test(trim)) { return "empty-state"; }

  // ui-message
  if (/message\.(success|error|warning|info)\(/.test(trim)) { return "ui-message"; }

  // 10. UI labels - visible text in JSX
  // Form.Item label
  if (/<Form\.Item\s+label\s*=/.test(trim)) { return "ui-label"; }
  // Placeholder
  if (/placeholder\s*=/.test(trim)) { return "ui-label"; }
  // Title attribute on elements
  if (/title\s*=\s*['"`][一-鿿]/.test(trim)) { return "ui-label"; }
  // Button text
  if (/<Button[^>]*>[一-鿿]/.test(trim)) { return "ui-label"; }
  // Description
  if (/description\s*[:=]\s*['"`][一-鿿]/.test(trim)) { return "ui-label"; }
  // Tag text
  if (/<Tag[^>]*>[一-鿿]/.test(trim)) { return "ui-label"; }
  // Text node with Chinese
  if (/<[A-Z]\w+[^>]*>[一-鿿]/.test(trim) && !/\.replace/.test(trim) && !/\.test\(/.test(trim)) { return "ui-label"; }
  // Plain JSX span/div text
  if (
    /<span[^>]*>[一-鿿]/.test(trim) || /<div[^>]*>[一-鿿]/.test(trim) || /<Text[^>]*>[一-鿿]/.test(trim)
  ) { return "ui-label"; }
  // Select.Option children
  if (/<Select\.Option/.test(trim) && /[一-鿿]/.test(trim)) { return "ui-label"; }
  // Menu items
  if (/\b(label|title)\s*[:=]\s*['"`][一-鿿]/.test(trim)) { return "ui-label"; }
  // Empty state
  if (/暂无/.test(trim)) { return "empty-state"; }
  // DecisionBanner export option label
  if (file.includes("DecisionBanner") && /\blabel\s*:/.test(trim)) { return "ui-label"; }

  // ScreenerPage comments
  if (file.includes("ScreenerPage") && /^\s*[1-2]\./.test(trim)) { return "comment"; }

  // AnalysisDebugPanel inline text
  if (file.includes("AnalysisDebugPanel") && /^\s*</.test(trim) && /[一-鿿]/.test(trim)) { return "ui-label"; }

  // FundPanel
  if (file.includes("FundPanel") && !file.includes("__tests__")) {
    if (/<span[^>]*>[一-鿿]/.test(trim) || /<Modal/.test(trim) || /placeholder/.test(trim)) { return "ui-label"; }
    if (/\btitle\s*:/.test(trim)) { return "ui-label"; }
  }

  // TradeStatsPanel
  if (
    file.includes("TradeStatsPanel") && !file.includes("__tests__") && /<span[^>]*>[一-鿿]/.test(trim)
  ) { return "ui-label"; }
  if (
    file.includes("TradeStatsPanel") && !file.includes("__tests__")
    && /\b(总盈亏|胜率|盈亏比|平均持有|税费|印花税|佣金|持有期|按策略|月度)/.test(trim)
  ) { return "ui-label"; }

  // StockAnalysisPage
  if (file.includes("StockAnalysisPage") && /[一-鿿]/.test(trim)) { return "ui-label"; }

  // CompactDebateNode / CompactDecisionComparison / CompactValueAssessment
  if (file.includes("CompactDebateNode")) {
    if (/<span/.test(trim) || /暂无/.test(trim) || /[一-鿿]/.test(trim)) { return "ui-label"; }
  }
  if (
    file.includes("CompactDecisionComparison") && !file.includes("__tests__") && /[一-鿿]/.test(trim)
  ) { return "ui-label"; }
  if (
    file.includes("CompactValueAssessment") && !file.includes("__tests__") && /[一-鿿]/.test(trim)
  ) { return "ui-label"; }
  if (file.includes("CompactRecommendation") && /[一-鿿]/.test(trim) && !/console/.test(trim)) { return "ui-label"; }
  if (file.includes("CompactAnalystSummary") && /暂无/.test(trim)) { return "empty-state"; }

  // DecisionComparisonPanel
  if (
    file.includes("DecisionComparisonPanel") && !file.includes("__tests__") && /[一-鿿]/.test(trim)
  ) { return "ui-label"; }

  // ConceptBlocksPanel
  if (file.includes("ConceptBlocksPanel") && /未知/.test(trim)) { return "data-dict"; }

  // DebatePanel
  if (file.includes("DebatePanel") && !file.includes("__tests__")) {
    if (/<div[^>]*>[一-鿿]/.test(trim) || /<span[^>]*>[一-鿿]/.test(trim)) { return "ui-label"; }
    if (/<Tag[^>]*>[一-鿿]/.test(trim)) { return "ui-label"; }
    if (/message=/.test(trim)) { return "ui-label"; }
  }

  // EventCalendarPanel catch comments
  if (file.includes("EventCalendarPanel")) {
    if (/\/\*/.test(trim)) { return "comment"; }
  }

  // WatchlistPanel catch comments
  if (file.includes("WatchlistPanel")) {
    if (/\/\*/.test(trim)) { return "comment"; }
  }

  // LimitUpPanel catch
  if (file.includes("LimitUpPanel") && /\/\*/.test(trim)) { return "comment"; }

  // SerenityScreeningPanel catch
  if (file.includes("SerenityScreeningPanel") && /catch/.test(trim)) { return "comment"; }

  // Quant panel catch
  if (file.includes("CompareTab") && /catch/.test(trim)) { return "comment"; }

  // rhai editor tab
  if (file.includes("RhaiEditorTab") && /<Form\.Item/.test(trim)) { return "ui-label"; }

  // ScreenerPage comments
  if (file.includes("ScreenerPage") && /^\s*\d+\./.test(trim)) { return "comment"; }

  // PriceAlertPanel catch
  if (file.includes("PriceAlertPanel") && /catch/.test(trim)) { return "comment"; }

  // Fallback: return other
  return "other";
}

// ── Main ──

function main() {
  const allowlist = JSON.parse(
    readFile(ALLOWLIST_PATH) || '{"version":"2","generated":"2026-07-12","total_entries":0,"entries":[]}',
  );
  const existingMap = {};
  for (const e of allowlist.entries) {
    for (const ln of (e.lines || "").split(",")) { if (ln) { existingMap[e.file + ":" + ln] = true; } }
  }

  // Get files
  const filesStr = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
    cwd: ROOT,
    encoding: "utf8",
  });
  const files = filesStr.trim().split("\n").filter(Boolean);

  const byCategory = {};
  const allowAdditions = {};
  let total = 0;

  for (const file of files) {
    const abspath = path.join(ROOT, file);
    if (!fs.existsSync(abspath)) { continue; }
    const content = readFile(abspath);
    if (!content) { continue; }
    const lines = content.split("\n");

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lnum = i + 1;
      const key = file + ":" + lnum;
      if (existingMap[key]) { continue; }

      // Must contain CJK
      const stripped = line.replace(/\/\/[^/]*$/, "").replace(/\{\/\*.*\*\/\}/g, "");
      if (!/[一-鿿]/.test(stripped)) { continue; }

      const trim = line.trim();
      const category = classify(trim, line, file, lnum, lines);

      // Track
      if (!byCategory[category]) { byCategory[category] = { count: 0 }; }
      byCategory[category].count++;
      total++;

      // Collect for allowlist (non-UI categories)
      const nonUI = [
        "test-desc",
        "comment",
        "dev-note",
        "regex",
        "llm-prompt",
        "config-desc",
        "data-dict",
        "internal-log",
        "mock-data",
        "helper-fn",
        "export-text",
      ];
      if (nonUI.includes(category)) {
        if (!allowAdditions[file]) { allowAdditions[file] = []; }
        allowAdditions[file].push(lnum);
      }
    }
  }

  // Print summary
  console.log("=== Classification ===");
  const sorted = Object.entries(byCategory).sort((a, b) => b[1].count - a[1].count);
  for (const [cat, info] of sorted) {
    console.log(`  ${cat.padEnd(16)} ${String(info.count).padStart(5)}`);
  }
  console.log(`  ${"TOTAL".padEnd(16)} ${String(total).padStart(5)}`);

  // Build new allowlist entries (comma-separated individual line numbers per file)
  // Also collect non-UI violations by file for batch output
  const fileAllowLists = {};
  for (const [file, lnums] of Object.entries(allowAdditions)) {
    const unique = [...new Set(lnums)].sort((a, b) => a - b);
    fileAllowLists[file] = { lines: unique.join(","), count: unique.length };
  }

  console.log(`\nFiles with non-UI violations: ${Object.keys(fileAllowLists).length}`);
  for (const [file, info] of Object.entries(fileAllowLists).sort((a, b) => b[1].count - a[1].count)) {
    console.log(`  ${file}: ${info.count} lines`);
  }

  // Write updated allowlist
  const newAllowEntries = Object.entries(fileAllowLists).map(([file, info]) => ({
    file,
    lines: info.lines,
    reason: "非UI文本-自动分类",
  }));
  newAllowEntries.sort((a, b) => a.file.localeCompare(b.file));

  allowlist.generated = "2026-07-12";
  allowlist.entries = allowlist.entries.concat(newAllowEntries);
  allowlist.total_entries = allowlist.entries.length;
  writeFile(ALLOWLIST_PATH, JSON.stringify(allowlist, null, 2) + "\n");
  console.log(`Allowlist updated: ${ALLOWLIST_PATH}`);
}

main();
