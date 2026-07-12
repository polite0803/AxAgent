#!/usr/bin/env node
// scripts/_gen-i18n-fix.cjs
// Step 1: Classify all CJK violations across src/
// Step 2: Generate updated allowlist for non-UI categories
// Step 3: Output UI text violations that need t() conversion
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const ROOT = path.resolve(__dirname, "..");
const ALLOWLIST_PATH = path.join(ROOT, "scripts/.i18n-allowlist.json");

// ── Helpers ──

function isTestFile(file) {
  return file.includes("__tests__/") || /\.(test|spec)\.(ts|tsx)$/.test(file);
}
function readFile(p) {
  try {
    return fs.readFileSync(p, "utf8");
  } catch {
    return null;
  }
}

// ── Main ──

function main() {
  // Read existing allowlist
  const allowlistRaw = readFile(ALLOWLIST_PATH);
  const allowlist = allowlistRaw
    ? JSON.parse(allowlistRaw)
    : { version: "2", generated: new Date().toISOString().slice(0, 10), total_entries: 0, entries: [] };
  const existingMap = {}; // "file:line" -> true
  for (const e of allowlist.entries) {
    for (const ln of (e.lines || "").split(",")) {
      if (ln) { existingMap[e.file + ":" + ln] = true; }
    }
  }

  // Get all TS/TSX files
  const filesStr = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
    cwd: ROOT,
    encoding: "utf8",
  });
  const files = filesStr.trim().split("\n").filter(Boolean);

  // Classify
  const byCategory = {};
  const byFile = {};
  const lineInfo = {}; // "file:line" -> { category, content }

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

      // Skip if already allowed
      if (existingMap[key]) { continue; }

      // Must contain CJK
      const stripped = line.replace(/\/\/[^/]*$/, "").replace(/\{\/\*.*\*\/\}/g, "");
      if (!/[一-鿿]/.test(stripped)) { continue; }

      // Skip JSDoc, line comments, block comment continuations
      if (/^\s*\/\*\*/.test(line) || /^\s*\/\//.test(line) || /^\s*\*/.test(line)) { continue; }

      const trim = line.trim();
      let category = "";

      // Test file descriptions
      if (isTestFile(file) && /^\s*(it|describe|test)\(/.test(trim)) { category = "test-desc"; }
      // JSX comments
      else if (/\{\/\*/.test(trim) && /\*\/\}/.test(trim)) { category = "comment"; }
      // Inline code comments (after stripping)
      else if (
        /\/\/[^/]/.test(line) && /[一-鿿]/.test(line.replace(/\/\/[^/]*[一-鿿][^/]*$/, "")) === false
      ) { category = "comment"; } // Regex patterns
      else if (
        /^\s*\/(?!\/)/.test(trim) || /new\s+RegExp/.test(trim) || /\.replace\(/.test(trim) || /\.test\(/.test(trim)
      ) {
        // Check if seems like regex with Chinese
        if (/[一-鿿]/.test(trim) && !/['"`]/.test(trim[0])) { category = "regex"; }
        else { /* might be a string with .replace() */ }
      } // logIpcError
      else if (/logIpcError/.test(trim)) { category = "internal-log"; }
      // LLM prompts
      else if (/你是一个|只输出 JSON|系统提示词|你是.*智能体/.test(trim)) { category = "llm-prompt"; }
      // Config parameter descriptions (b() calls with desc)
      else if (/^\s*b\(/.test(trim) || /^\s*b\s*\(/.test(trim)) { category = "config-desc"; }
      else if (file.includes("StockAnalysisConfigPanel") && /^\s+b\(/.test(trim)) { category = "config-desc"; }
      // Data dictionaries (key: "value" patterns)
      else if (/^\s*['"`][\w-]+['"`]\s*:\s*['"`][一-鿿]/.test(trim)) { category = "data-dict"; }
      else if (/^\s*\{?\s*(label|title|desc|helpText)\s*:\s*['"`]/.test(trim)) { category = "data-dict"; }
      // Mock data
      else if (file.includes("browserMock")) { category = "mock-data"; }
      // Strategy form label/placeholder definitions
      else if (
        file.includes("StrategyForm") && (/\blabel\s*:/.test(trim) || /\bplaceholder\s*:/.test(trim))
      ) { category = "data-dict"; } // Future reference detectors
      else if (file.includes("futureReferenceDetector")) { category = "data-dict"; }
      // Screener panel columns config
      else if (file.includes("SerenityScreeningPanel") && /^\s*\{?label\s*:/.test(trim)) { category = "data-dict"; }
      // Workflow type definitions
      else if (file.includes("workflow.types.ts")) { category = "dev-note"; }
      // SDK types / sandbox template
      else if (file.includes("sdk/types.ts") || file.includes("sdk/sandboxTemplate")) { category = "data-dict"; }
      // Risk matrix arrays of { re: /.../, score: ... }
      else if (file.includes("RiskMatrix") || file.includes("CompactRiskSummary")) { category = "regex"; }
      // Stock schemas (title/description/header/tabLabel)
      else if (file.includes("stock-schemas") || file.includes("seed-stock")) { category = "data-dict"; }
      // Developer notes
      else if (/P[0123]-\d+|Bug\s+\d+|TODO|FIXME|缺陷/.test(trim) && !/['"`]/.test(trim[0])) { category = "dev-note"; }
      else if (/\/\*\s*(P[0123]|TODO|FIXME|缺陷)/.test(trim)) { category = "dev-note"; }
      // Agent generator modal (LLM prompt template)
      else if (file.includes("AgentGeneratorModal")) { category = "llm-prompt"; }
      // Reflection panel LLM prompts
      else if (file.includes("ReflectionPanel") && /\bschema\b|action_type|output/.test(trim)) {
        category = "llm-prompt";
      } // Stock analysis export (title/lines.push patterns)
      else if (file.includes("stock-analysis-export") && (/\blines\.push/.test(trim) || /\bh\(/.test(trim))) {
        category = "ui-label";
      } // Stock analysis utils (mappings)
      else if (file.includes("stock-analysis-utils") && /['"`][一-鿿]/.test(trim)) { category = "data-dict"; }
      // DataVendorsTab tool/label/category definitions
      else if (
        file.includes("DataVendorsTab")
        && (/^\s*\{?\s*(tool|label)\s*:/.test(trim) || /^\s*(quote|klines|financials|news|money_flow)/.test(trim))
      ) { category = "data-dict"; } // ScheduledRecommendationTab presets
      else if (file.includes("ScheduledRecommendationTab") && /\blabel\s*:/.test(trim)) { category = "data-dict"; }
      // Empty state / fallback
      else if (/暂无/.test(trim)) { category = "empty-state"; }
      // message.* calls
      else if (/message\.(success|error|warning|info)\(/.test(trim)) { category = "ui-message"; }
      // AgentProfileList display name mappings
      else if (file.includes("AgentProfileList")) { category = "data-dict"; }
      // Stock-search-bar period labels
      else if (file.includes("StockSearchBar")) { category = "data-dict"; }
      // quant StrategyForm
      else if (file.includes("StrategyForm.tsx") && /\b(key|label)\s*:/.test(trim)) { category = "data-dict"; }
      // Dual view titles
      else if (file.includes("dualView") && /\btitle\s*:/.test(trim)) { category = "data-dict"; }
      // WalkForwardFoldBarChart axis labels
      else if (file.includes("WalkForwardFoldBarChart") && /^\s*`/.test(trim)) { category = "ui-label"; }
      else { category = "other"; } // Needs human review

      if (!category) { continue; }

      // Record
      if (!byCategory[category]) { byCategory[category] = { count: 0, files: {} }; }
      byCategory[category].count++;
      if (!byCategory[category].files[file]) { byCategory[category].files[file] = []; }
      byCategory[category].files[file].push(lnum);

      if (!byFile[file]) { byFile[file] = {}; }
      if (!byFile[file][category]) { byFile[file][category] = []; }
      byFile[file][category].push(lnum);

      lineInfo[key] = { category, content: trim.slice(0, 120) };
    }
  }

  // Save full report
  const reportPath = path.join(ROOT, ".check-i18n-report.json");
  fs.writeFileSync(reportPath, JSON.stringify({ byCategory, byFile, lineInfo }, null, 2));

  // Print summary
  console.log("=== Classification Summary ===");
  const sortedCats = Object.entries(byCategory).sort((a, b) => b[1].count - a[1].count);
  for (const [cat, info] of sortedCats) {
    const fc = Object.keys(info.files).length;
    console.log(`  ${cat.padEnd(18)} ${String(info.count).padStart(5)}  (${fc} files)`);
  }
  console.log(`\n  TOTAL: ${Object.keys(lineInfo).length} violations`);
  console.log(`\nReport saved to ${reportPath}`);

  // Generate allowlist structure grouped by file
  const fileGroups = {};
  const nonUICategories = [
    "test-desc",
    "comment",
    "regex",
    "internal-log",
    "llm-prompt",
    "config-desc",
    "data-dict",
    "mock-data",
    "dev-note",
  ];
  for (const [key, info] of Object.entries(lineInfo)) {
    if (!nonUICategories.includes(info.category)) { continue; }
    const [file, lnum] = key.split(":");
    if (!fileGroups[file]) { fileGroups[file] = []; }
    fileGroups[file].push(parseInt(lnum));
  }

  console.log("\n=== Non-UI violations to allowlist ===");
  const allowAdditions = [];
  for (const [file, lnums] of Object.entries(fileGroups)) {
    lnums.sort((a, b) => a - b);
    // Group consecutive runs
    const ranges = [];
    let start = lnums[0], end = lnums[0];
    for (let i = 1; i < lnums.length; i++) {
      if (lnums[i] === end + 1) { end = lnums[i]; }
      else {
        ranges.push(start === end ? `${start}` : `${start}-${end}`);
        start = end = lnums[i];
      }
    }
    ranges.push(start === end ? `${start}` : `${start}-${end}`);

    // Group into chunks of up to 50 lines per entry
    for (const r of ranges) {
      allowAdditions.push({ file, lines: r, reason: `非UI文本-${r.includes("-") ? "多行" : ""}` });
    }
  }
  console.log(`  ${allowAdditions.length} allowlist entry groups to add`);

  // Output UI text violations that need t() conversion
  const uiCategories = ["ui-label", "ui-message", "empty-state", "fallback-text", "other"];
  const uiViolations = {};
  for (const [key, info] of Object.entries(lineInfo)) {
    if (!uiCategories.includes(info.category)) { continue; }
    const [file, lnum] = key.split(":");
    if (!uiViolations[file]) { uiViolations[file] = []; }
    uiViolations[file].push({ line: parseInt(lnum), category: info.category, content: info.content });
  }

  console.log("\n=== UI text violations needing t() conversion ===");
  for (const [file, viols] of Object.entries(uiViolations).sort()) {
    console.log(`  ${file}: ${viols.length} violations`);
    for (const v of viols.slice(0, 5)) {
      console.log(`    L${v.line}: [${v.category}] ${v.content}`);
    }
    if (viols.length > 5) { console.log(`    ... and ${viols.length - 5} more`); }
  }

  // Write UI violations list for workflow
  const uiPath = path.join(ROOT, ".check-i18n-ui.json");
  fs.writeFileSync(uiPath, JSON.stringify(uiViolations, null, 2));
  console.log(`\nUI violations saved to ${uiPath}`);
}

main();
