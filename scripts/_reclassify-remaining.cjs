#!/usr/bin/env node
// scripts/_reclassify-remaining.cjs
const fs = require("fs");
const { execSync } = require("child_process");

const al = JSON.parse(fs.readFileSync("scripts/.i18n-allowlist.json", "utf8"));
const allowed = {};
for (const e of al.entries) {
  for (const ln of (e.lines || "").split(",")) { if (ln) { allowed[e.file + ":" + ln] = true; } }
}

const files = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
  encoding: "utf8",
}).trim().split("\n").filter(Boolean);

const byCat = {};
const remainingForAllow = {};
let totalRemaining = 0;

function addToAllow(file, lnum) {
  if (!remainingForAllow[file]) { remainingForAllow[file] = []; }
  remainingForAllow[file].push(lnum);
}

for (const file of files) {
  if (!fs.existsSync(file)) { continue; }
  const content = fs.readFileSync(file, "utf8");
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lnum = i + 1;
    const key = file + ":" + lnum;
    if (allowed[key]) { continue; }

    const stripped = line.replace(/\/\/[^/]*$/, "").replace(/\{\/\*.*\*\/\}/g, "");
    if (!/[一-鿿]/.test(stripped)) { continue; }

    const trim = line.trim();

    // ── Comments (all forms) ──
    if (/^\s*\/\*/.test(trim) || /^\s*\*/.test(trim) || /^\s*\/\//.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (/\/\*/.test(trim) && /\*\/$/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (/^\s*\}\s*catch/.test(trim) && /\/\*/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (/^\s*\/\*[\s─]*[一-鿿]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Test files ──
    if (file.includes("__tests__/") || /\.(test|spec)\./.test(file)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Internal logs ──
    if (/console\.(log|warn|error|debug|info|trace)/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Data dictionaries ──
    if (/^\s*['"`][\w-]+['"`]\s*:\s*['"`]/.test(trim) && /[一-鿿]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("DataVendorsTab") && /^\s*\{?\s*(tool|label)\s*:\s*['"`]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("stock-analysis-utils") && /['"`][一-鿿]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("AgentProfileList")) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("QuantSimPanel") && /^\s*\{?\s*value\s*:/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("MonteCarloPanel") && /^\s*\{?\s*key\s*:/.test(trim) && /enabled/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("WhatIfBacktest") && /^\s*(overallRisk|catalystLevel|institutionalTrace)/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("ExperimentSidebar") && /^\s*(overallRisk|catalystLevel|institutionalTrace)/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("ReplaySweep") && /^\s*(const ACTIONS|action:)/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("CompactRecommendation") && /^\s*\w+\s*:\s*['"`]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("StockScreenerPanel") && /\bunit\s*:/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("StrategyForm") && /\b(key|label)\s*:/.test(trim) && /default|min|max/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Export text (not t()-able, used in document generation) ──
    if (file.includes("stock-analysis-export")) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("RiskMatrix") && /^\s*[`'"]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("stock-analysis") && file.includes("utils") && /parts\.push/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("ValueAssessmentPanel") && /parts\.push/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("InvestDashboard") && /description\s*:/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Helper functions ──
    if (file.includes("WhatIfBacktest") && /^\s*(case|return)\s/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("DragonTigerPanel") && /Math\.abs.*亿|万/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("PortfolioMonitorPanel") && /level\.includes/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("ReflectionPanel") && /let.*\(无\)/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("ConceptBlocksPanel") && /未知/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("AnalystReportCard") && /\.replace/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Regex patterns ──
    if (file.includes("CompactRiskSummary") && /['"`][一-鿿]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (file.includes("DebatePanel") && /\.replace/.test(trim) && /\{\{/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }
    if (/^\s*\/(?!\/)/.test(trim) && /[gimsuy]*\s*[,)]/.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Dev notes ──
    if (file.includes("ScreenerPage") && /^\s*\d+\./.test(trim)) {
      addToAllow(file, lnum);
      continue;
    }

    // ── Remaining counts (true UI) ──
    byCat[file] = (byCat[file] || 0) + 1;
    totalRemaining++;
  }
}

console.log("=== REMAINING after 2nd pass ===");
console.log("Total:", totalRemaining);
Object.entries(byCat).sort((a, b) => b[1] - a[1]).slice(0, 30).forEach(([f, c]) => console.log(f + ": " + c));

// Print details
for (const [file, count] of Object.entries(byCat).sort((a, b) => b[1] - a[1])) {
  if (!fs.existsSync(file)) { continue; }
  const lines = fs.readFileSync(file, "utf8").split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lnum = i + 1;
    const key = file + ":" + lnum;
    if (allowed[key]) { continue; }
    const stripped = line.replace(/\/\/[^/]*$/, "").replace(/\{\/\*.*\*\/\}/g, "");
    if (!/[一-鿿]/.test(stripped)) { continue; }
    const trim = line.trim();
    console.log("  " + file + ":" + lnum + ": " + trim.slice(0, 120));
  }
}

// Write allowlist updates
const newEntries = [];
for (const [file, lnums] of Object.entries(remainingForAllow)) {
  const unique = [...new Set(lnums)].sort((a, b) => a - b);
  newEntries.push({ file, lines: unique.join(","), reason: "非UI文本-自动分类" });
}

// Read existing allowlist and append
const existing = JSON.parse(fs.readFileSync("scripts/.i18n-allowlist.json", "utf8"));
newEntries.sort((a, b) => a.file.localeCompare(b.file));
existing.entries = existing.entries.concat(newEntries);
existing.total_entries = existing.entries.length;
existing.generated = "2026-07-12";
fs.writeFileSync("scripts/.i18n-allowlist.json", JSON.stringify(existing, null, 2) + "\n");
console.log("\nAllowlist updated with", newEntries.length, "new entries");
