#!/usr/bin/env node
// scripts/_classify-i18n.cjs
// Scan all TS/TSX source files for CJK violations, classify each line,
// and output a categorized JSON for further processing.
//
// Categories:
//   test-desc       — it() / describe() test descriptions
//   comment         — code comments (//, /* */, JSX {/* */})
//   jsx-comment     — {/* ... */} JSX embedded comments
//   console-log     — console.* internal log messages
//   llm-prompt      — LLM system prompts, template strings for AI
//   regex           — regex patterns (RegExp / new RegExp)
//   internal-log    — logIpcError, tracing, debug messages
//   dev-note        — TODO / FIXME / P0-3 / developer notes in strings
//   data-dict       — data dictionaries, mappings, constant objects
//   mock-data       — test mock data fixtures
//   empty-state     — "暂无"/"等待" empty state placeholder text
//   ui-label        — UI text visible to user (table titles, labels, buttons, placeholders)
//   ui-message      — message.success/error/warning/info calls
//   fallback-text   — default/fallback UI text strings
//   other           — uncategorized

const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..");

// ── File classification helpers ──

function isTestFile(file) {
  return file.includes("__tests__/") || file.endsWith(".test.ts") || file.endsWith(".test.tsx");
}

function readLines(filePath) {
  try {
    const content = fs.readFileSync(filePath, "utf8");
    return content.split("\n");
  } catch {
    return null;
  }
}

// ── Line classifiers ──

function classifyLine(line, file, lnum, context) {
  const trimmed = line.trim();
  const trimmedLower = trimmed.toLowerCase();
  const stripped = trimmed.replace(/\/\/.*$/, "").trim();

  // 1. Test file descriptions
  if (isTestFile(file)) {
    if (/^(it|describe|test)\(/.test(trimmed)) { return "test-desc"; }
    if (lnum === 0 && /^import/.test(trimmed)) { return "test-desc"; // unlikely but safe
     }
  }

  // 2. Comments (line comments, block comments, JSX comments)
  if (
    /^\/\//.test(trimmed) || /^\/\*\*/.test(trimmed) || /^\s*\*/.test(trimmed) || /^\s*\/\*/.test(trimmed)
    || /\{\/\*.*\*\/\}/.test(trimmed)
  ) {
    return "comment";
  }

  // 3. JSX embedded comments {/* ... */}
  if (/\{\/\*/.test(trimmed) && /\*\/\}/.test(trimmed)) {
    return "jsx-comment";
  }

  // 4. Regex literals and RegExp constructors
  if (
    /^\s*\/\^?\[/.test(trimmed) || /^\s*\/[^\/]+\/[gimsuy]*\s*[,;\)]/.test(trimmed) || /new\s+RegExp\(/.test(trimmed)
  ) {
    return "regex";
  }
  if (/^\s*[\[\(]\s*\//.test(trimmed) && /\/[gimsuy]*\s*[,\]]/.test(trimmed)) {
    return "regex";
  }

  // 5. LLM prompts
  if (
    /prompt/i.test(context) && trimmed.length > 80
    && (trimmed.includes("你") || trimmed.includes("请输出") || trimmed.includes("system"))
  ) {
    return "llm-prompt";
  }
  if (/system_prompt/.test(trimmed) || /你是一个/.test(trimmed) || /你是.*智能体/.test(trimmed)) {
    return "llm-prompt";
  }
  if (/只输出 JSON/.test(trimmed) || /不要有其他内容/.test(trimmed)) {
    return "llm-prompt";
  }

  // 6. Internal log messages (console.* already excluded by the check script)
  if (/logIpcError/.test(trimmed) || /logIpc/.test(trimmed)) {
    return "internal-log";
  }
  if (/^\s*\[(StockAnalysis|Serenity|startup|DEBUG|INFO|WARN)\]/.test(trimmed)) {
    return "internal-log";
  }
  if (/^\s*['"`]\[[A-Za-z]/.test(trimmed) && /['"`]\s*[+,]/.test(trimmed)) {
    return "internal-log";
  }

  // 7. Developer notes (TODO, FIXME, P0-3, Bug #)
  if (/TODO|FIXME|P[0123]-\d+|Bug \d+|修复.*[死避].*|缺陷/.test(trimmed) && !/['"`]/.test(trimmed[0])) {
    return "dev-note";
  }
  if (/\/\*\s*(P[0123]|TODO|FIXME|缺陷|修复)/.test(trimmed)) {
    return "dev-note";
  }

  // 8. Data dictionaries / mappings
  if (/^\s*['"`][a-z][a-z_-]+['"`]\s*:\s*['"`]/.test(trimmed) && /[,}]?\s*$/.test(trimmed)) {
    return "data-dict";
  }
  if (/^\s*(key|value):\s*['"`]/.test(trimmedLower)) {
    return "data-dict";
  }

  // 9. Mock data files
  if (file.includes("browserMock") || file.includes("mock")) {
    return "mock-data";
  }

  // 10. message.success/error/warning/info
  if (/message\.(success|error|warning|info)\(/.test(trimmed)) {
    return "ui-message";
  }

  // 11. Empty state placeholders
  if (
    /暂无|等待|加载中/.test(trimmed)
    && (trimmed.includes("暂无") || trimmed.includes("等待") || trimmed.includes("加载"))
  ) {
    return "empty-state";
  }

  // 12. Regex patterns in the risk matrix / keyword matching files
  if (file.includes("RiskMatrix") || file.includes("CompactRiskSummary") || file.includes("CompactRecommendation")) {
    if (/^\s*\{?\s*(re|label)\s*:/.test(trimmed)) { return "regex"; }
  }

  // 13. Stock analysis utility patterns (stance/action/sentiment mappings)
  if (file.includes("stock-analysis-utils") || file.includes("stock-analysis")) {
    if (/['"`](买入|卖出|增持|减持|持有|观望|看多|看空|中性|低|中|高|极高)['"`]\s*:/.test(trimmed)) {
      return "data-dict";
    }
  }

  // 14. Strategy form / config parameter definitions
  if (file.includes("StrategyForm") || file.includes("StockAnalysisConfigPanel")) {
    // These are config defs, not UI text (the label is a field, not rendered directly)
    if (/\b(label|description)\s*:/.test(trimmed)) { return "data-dict"; }
  }

  // 15. Future reference detectors
  if (file.includes("futureReferenceDetector")) {
    return "data-dict";
  }

  // 16. Agent generator / LLM prompt templates
  if (file.includes("AgentGeneratorModal") || file.includes("ReflectionPanel")) {
    if (trimmed.startsWith("`") || trimmed.startsWith('"') || trimmed.startsWith("'")) { return "llm-prompt"; }
  }

  // 17. Plugin/system description strings
  if (file.includes("DataVendorsTab") && /\b(name|desc|helpText)\s*:/.test(trimmed)) {
    return "data-dict";
  }

  // 18. seed-stock-schemas / stock-schemas / evolution-drift-schema
  if (file.includes("stock-schemas") || file.includes("seed-stock")) {
    if (
      /\b(title|description|tabLabel|header)\s*:/.test(trimmed) || /\bplaceholder\s*:/.test(trimmed)
    ) { return "data-dict"; }
  }

  // 19. Workflow type definitions
  if (file.includes("workflow.types.ts")) {
    return "dev-note";
  }

  // 20. SDK types
  if (file.includes("sdk/types.ts") || file.includes("sdk/sandboxTemplate") || file.includes("sdk/rpcBridge")) {
    return "data-dict";
  }

  // 21. Export helpers (stock-analysis-export.ts)
  if (file.includes("stock-analysis-export")) {
    if (/\btitle\b/.test(trimmed) || /\blines\.push/.test(trimmed) || /\bh\(/.test(trimmed)) { return "ui-label"; }
  }

  // 22. Default UI labels (likely visible to user)
  if (
    /\blabel\s*[:=]\s*['"`]/.test(trimmed) && !file.includes("StrategyForm")
    && !file.includes("StockAnalysisConfigPanel")
  ) {
    return "ui-label";
  }
  if (/\btitle\s*[:=]\s*['"`]/.test(trimmed) && !file.includes("stock-schemas") && !file.includes("seed-stock")) {
    return "ui-label";
  }
  if (/placeholder\s*[:=]\s*['"`]/.test(trimmed)) {
    return "ui-label";
  }

  // 23. Table columns with title
  if (/\btitle\s*:\s*['"`]/.test(trimmed) && /dataIndex|key\s*:/.test(trimmed)) {
    return "ui-label";
  }

  // 24. Fallback text patterns
  if (trimmed.includes("??") || /^\s*['"`][^'"]{2,}['"`]\s*\|\||\|\|\s*['"`][^'"]{2,}['"`]/.test(trimmed)) {
    return "fallback-text";
  }

  // Default: need human review
  return "other";
}

// ── Main ──

function main() {
  // Get all TS/TSX files
  const { execSync } = require("child_process");
  const filesStr = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
    cwd: ROOT,
    encoding: "utf8",
  });
  const files = filesStr.trim().split("\n").filter(Boolean);

  const byFile = {};
  const byCategory = {};
  let total = 0;

  for (const file of files) {
    if (!fs.existsSync(path.join(ROOT, file))) { continue; }

    // Read the allowlist for this file to skip already-allowed lines
    const lines = readLines(path.join(ROOT, file));
    if (!lines) { continue; }

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lnum = i + 1;

      // Skip lines that don't contain CJK at all
      if (!/[一-鿿㐀-䶿]/.test(line)) { continue; }

      // Strip inline // comments before CJK check
      const stripped = line.replace(/\/\/[^/]*$/, "");
      if (!/[一-鿿㐀-䶿]/.test(stripped)) { continue; }

      // Skip JSDoc/block comment opening lines
      if (/^\s*\/\*\*/.test(line)) { continue; }

      // Skip line comments
      if (/^\s*\/\//.test(line)) { continue; }

      // Skip block comment continuation lines
      if (/^\s*\*/.test(line)) { continue; }

      // Skip import/require lines with CJK comments
      if (/^\s*(import|require|from)\s/.test(line) && !/[一-鿿㐀-䶿]{2,}/.test(stripped)) { continue; }

      // Get context from surrounding lines
      const beforeLine = i > 0 ? lines[i - 1] : "";
      const context = beforeLine + "\n" + line;

      const category = classifyLine(line, file, lnum, context);

      // Count
      if (!byCategory[category]) { byCategory[category] = { count: 0, files: {} }; }
      byCategory[category].count++;
      if (!byCategory[category].files[file]) { byCategory[category].files[file] = []; }
      byCategory[category].files[file].push(lnum);

      if (!byFile[file]) { byFile[file] = {}; }
      if (!byFile[file][category]) { byFile[file][category] = []; }
      byFile[file][category].push(lnum);

      total++;
    }
  }

  // Sort categories by count desc
  const sortedCategories = Object.entries(byCategory).sort((a, b) => b[1].count - a[1].count);

  console.log(`\n=== i18n Violation Classification Report ===`);
  console.log(`Total: ${total} violations across ${Object.keys(byFile).length} files\n`);

  console.log("Breakdown by category:");
  console.log("-".repeat(80));
  for (const [cat, info] of sortedCategories) {
    const fileCount = Object.keys(info.files).length;
    console.log(`  ${cat.padEnd(20)} ${String(info.count).padStart(5)}  (${fileCount} files)`);
  }

  // For "other" and "ui-label" categories, list the actual lines
  const actionableCategories = ["other"];
  for (const cat of actionableCategories) {
    if (!byCategory[cat]) { continue; }
    console.log(`\n--- ${cat.toUpperCase()} (needs human review) ---`);
    const files = Object.keys(byCategory[cat].files).sort();
    for (const f of files) {
      const lnumStr = byCategory[cat].files[f].join(",");
      const hitLines = byCategory[cat].files[f].map((ln) => `${ln}: ${lines[ln - 1]}`).join("\n    ");
      console.log(`  ${f}:${lnumStr}`);
      console.log(`    ${hitLines}`);
    }
  }

  // Output JSON for further processing
  const resultPath = path.join(ROOT, ".check-i18n-report.json");
  fs.writeFileSync(resultPath, JSON.stringify({ total, byCategory, byFile }, null, 2));
  console.log(`\nFull report written to ${resultPath}`);
}

main();
