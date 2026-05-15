#!/usr/bin/env bash
# scripts/generate-i18n-allowlist.sh
# 扫描项目中所有硬编码字符串，生成 scripts/.i18n-allowlist.json
set -euo pipefail
cd "$(dirname "$0")/.."

ALLOWLIST="scripts/.i18n-allowlist.json"

echo "=== Scanning for hardcoded i18n violations ==="

node -e "
const fs = require('fs');
const path = require('path');

const SRC_DIR = 'src';
const ALLOWLIST = '$ALLOWLIST';
const EXCLUDE_DIR = 'src/i18n/locales';

// Collect all .ts and .tsx files recursively
function collectFiles(dir) {
  const files = [];
  try {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        files.push(...collectFiles(full));
      } else if (entry.isFile() && /\.(ts|tsx)$/.test(entry.name)) {
        files.push(full);
      }
    }
  } catch (e) {
    // ignore permission errors etc.
  }
  return files;
}

function isExcluded(filePath) {
  return filePath.replace(/\\\\/g, '/').startsWith(EXCLUDE_DIR.replace(/\\\\/g, '/'));
}

// Check if a line should be excluded (matches CI check-hardcoded-i18n.sh filtering)
function isExcludedLine(line) {
  // Skip single-line comments: // ...
  if (/^\s*\/\//.test(line)) return true;
  // Skip block comment continuation lines: * ...
  if (/^\s*\*/.test(line)) return true;
  // Skip console.log/warn/error/debug/info/trace
  if (/console\.(log|warn|error|debug|info|trace)/.test(line)) return true;
  return false;
}

// Parse file lines and return matching line numbers
function findMatches(content, regex) {
  const lines = content.split('\n');
  const matching = [];
  for (let i = 0; i < lines.length; i++) {
    if (isExcludedLine(lines[i])) continue;
    if (regex.test(lines[i])) {
      matching.push(i + 1); // 1-indexed line numbers
    }
  }
  return matching;
}

// Result accumulator: file -> Set of line numbers
class FileMap {
  constructor(reason, phase) {
    this.map = new Map();
    this.reason = reason;
    this.phase = phase;
  }
  add(file, line) {
    if (!this.map.has(file)) this.map.set(file, new Set());
    this.map.get(file).add(line);
  }
  getEntries() {
    const entries = [];
    for (const [file, lines] of this.map) {
      entries.push({
        file: file.replace(/\\\\/g, '/'),
        lines: [...lines].sort((a,b) => a-b).join(','),
        reason: this.reason,
        phase: this.phase
      });
    }
    return entries;
  }
}

console.log('--- Scanning Chinese hardcoded strings ---');
const chineseRe = /[一-鿿㐀-䶿]/;
const chineseMap = new FileMap('硬编码中文字符串', 3);

console.log('--- Scanning English UI hardcoded strings ---');
const msgRe = /(message\.(success|error|warning|info)\(\s*['\"])/;
const msgMap = new FileMap('硬编码英文 UI 消息', 3);

const placeholderRe = /placeholder\s*=\s*\"[A-Za-z][^\"]{2,}\"/;
const placeholderMap = new FileMap('硬编码英文占位符', 3);

const notifRe = /notification\.\w+\(\s*\{[^}]*message\s*:\s*\"[^\"]+/;
const notifMap = new FileMap('硬编码英文通知', 3);

console.log('--- Scanning t() fallback patterns ---');
const fallbackRe = /t\(\s*['\"][^'\"]+['\"]\s*,\s*['\"][^'\"]+['\"]/;
const fallbackMap = new FileMap('t() fallback 参数', 2);

let totalFiles = 0;
const files = collectFiles(SRC_DIR);
for (const file of files) {
  if (isExcluded(file)) continue;
  const content = fs.readFileSync(file, 'utf8');
  totalFiles++;

  const chineseLines = findMatches(content, chineseRe);
  for (const l of chineseLines) chineseMap.add(file, l);

  const msgLines = findMatches(content, msgRe);
  for (const l of msgLines) msgMap.add(file, l);

  const placeholderLines = findMatches(content, placeholderRe);
  for (const l of placeholderLines) placeholderMap.add(file, l);

  const notifLines = findMatches(content, notifRe);
  for (const l of notifLines) notifMap.add(file, l);

  const fallbackLines = findMatches(content, fallbackRe);
  for (const l of fallbackLines) fallbackMap.add(file, l);
}

console.log('  Scanned ' + totalFiles + ' .ts/.tsx files');
console.log('  Chinese strings: ' + chineseMap.map.size + ' files');
console.log('  Message calls: ' + msgMap.map.size + ' files');
console.log('  Placeholders: ' + placeholderMap.map.size + ' files');
console.log('  Notifications: ' + notifMap.map.size + ' files');
console.log('  t() fallback calls: ' + fallbackMap.map.size + ' files');

// Merge entries for the same file + phase
const allEntries = [
  ...chineseMap.getEntries(),
  ...msgMap.getEntries(),
  ...placeholderMap.getEntries(),
  ...notifMap.getEntries(),
  ...fallbackMap.getEntries()
];

const merged = new Map();
for (const e of allEntries) {
  const key = e.file + '|||' + e.phase;
  if (merged.has(key)) {
    const existing = merged.get(key);
    const allLines = new Set([...existing.lines.split(',').map(Number), ...e.lines.split(',').map(Number)]);
    existing.lines = [...allLines].sort((a,b) => a-b).join(',');
  } else {
    merged.set(key, {...e});
  }
}

const result = {
  version: '1',
  generated: new Date().toISOString().split('T')[0],
  total_entries: merged.size,
  total_files: new Set([...merged.values()].map(e => e.file)).size,
  entries: [...merged.values()].sort((a,b) => a.file.localeCompare(b.file))
};

fs.writeFileSync(ALLOWLIST, JSON.stringify(result, null, 2) + '\n');
console.log('');
console.log('Allowlist generated: ' + result.total_entries + ' entries across ' + result.total_files + ' files');
const phases = {};
result.entries.forEach(e => { phases['phase'+e.phase] = (phases['phase'+e.phase]||0)+1; });
console.log('By phase:', JSON.stringify(phases));
"
echo "=== Done ==="
