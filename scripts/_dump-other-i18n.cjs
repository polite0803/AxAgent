#!/usr/bin/env node
// scripts/_dump-other-i18n.cjs
// Quick dump of uncategorized CJK violations for analysis
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const ROOT = path.resolve(__dirname, "..");
const filesStr = execSync('find src -name "*.ts" -o -name "*.tsx" | grep -v "src/i18n/locales/" | sort', {
  cwd: ROOT,
  encoding: "utf8",
});
const files = filesStr.trim().split("\n").filter(Boolean);

const EXCLUDE_DIRS = ["__tests__/", "node_modules"];
const IGNORE_PATTERNS = [
  /^\/\/\s/,
  /^\s*\/\//,
  /^\s*\*/,
  /^\s*\/\*\*/,
  /^\s*import\s/,
  /^\s*from\s/,
  /console\.(log|warn|error|debug|info|trace)/,
];

let count = 0;
for (const file of files) {
  if (EXCLUDE_DIRS.some((d) => file.includes(d))) { continue; }
  if (!fs.existsSync(path.join(ROOT, file))) { continue; }
  const content = fs.readFileSync(path.join(ROOT, file), "utf8");
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lnum = i + 1;
    if (!/[一-鿿㐀-䶿]/.test(line)) { continue; }
    const stripped = line.replace(/\/\/[^/]*$/, "");
    if (!/[一-鿿㐀-䶿]/.test(stripped)) { continue; }
    if (IGNORE_PATTERNS.some((p) => p.test(line))) { continue; }

    const trim = line.trim();

    // Check if it matches known data-dict patterns
    if (/^\s*['"`][a-zA-Z][\w-]+['"`]\s*:\s*['"`]/.test(trim)) { continue; }

    // LLM prompt
    if (/你是一个/.test(trim) || /只输出 JSON/.test(trim) || /不构成投资/.test(trim)) { continue; }

    // Regex pattern lines
    if (/^\s*\/\^/.test(trim) || /^\s*\{?\s*re\s*:\s*\//.test(trim)) { continue; }

    // JSX comments
    if (/\{\/\*/.test(trim) && /\*\/\}/.test(trim)) { continue; }

    // Data dictionary in RiskMatrix / CompactRiskSummary
    if (trim.match(/["'`][一-鿿]/) && /:\s*["'`]/.test(trim) && /[,}\]]?\s*$/.test(trim)) { continue; }

    count++;
    if (count <= 200) {
      const shortFile = path.relative(ROOT, file);
      console.log(`${shortFile}:${lnum}: ${trim.slice(0, 120)}`);
    }
  }
}
console.log(`\n--- Total uncategorized: ${count} ---`);
