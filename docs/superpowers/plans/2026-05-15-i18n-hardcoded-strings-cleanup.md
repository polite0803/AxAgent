# i18n 硬编码字符串清理 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除项目所有 i18n 违规（~1500 处），建立 CI 自动化门禁防止回退。

**Architecture:** 四阶段渐进式清理。阶段 1 补全缺失 locale key + 搭建 CI 检测脚本（双防线：增量门禁 + 存量豁免清单）。后续阶段按 fallback → 硬编码 → 类型层的顺序逐批消除违规并缩减豁免清单。

**Tech Stack:** Node.js (CI 脚本), Bash (检测脚本), i18next/react-i18next (翻译框架), JSON (locale 文件), GitHub Actions (CI)

**Spec:** `docs/superpowers/specs/2026-05-15-i18n-hardcoded-strings-cleanup-design.md`

---

## 阶段 1: 紧急补洞 + 搭建防线

### Task 1: 生成缺失 key 完整清单

**Files:**
- Create: `scripts/.i18n-missing-keys.json`（临时文件，供后续 task 使用）
- Reference: `scripts/check-missing-keys.mjs`
- Reference: `scripts/untranslated-report.txt`

- [ ] **Step 1: 扫描代码中所有 `t()` 调用的 key，对比 en-US.json 找出缺失项**

```bash
cd D:/OneManager/AxAgent
# 提取所有 t("key") 和 t("key", "fallback") 中的 key
grep -roPh "t\(\s*['\"]([^'\"]+)['\"]" src/ --include='*.ts' --include='*.tsx' \
  | sed "s/.*t(['\"]//" | sed "s/['\"].*//" | sort -u > /tmp/all-t-keys.txt

# 统计: 哪些出现在 t() fallback 中但不在 en-US.json 中
node -e "
const fs = require('fs');
const enUS = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json','utf8'));
const keys = fs.readFileSync('/tmp/all-t-keys.txt','utf8').trim().split('\n');
function keyExists(obj, path) {
  const parts = path.split('.');
  let cur = obj;
  for (const p of parts) { if (cur && typeof cur === 'object' && p in cur) cur = cur[p]; else return false; }
  return typeof cur === 'string';
}
const missing = keys.filter(k => !keyExists(enUS.translation || enUS, k));
fs.writeFileSync('scripts/.i18n-missing-keys.json', JSON.stringify(missing, null, 2));
console.log('Missing keys:', missing.length);
console.log(missing.join('\n'));
"
```

- [ ] **Step 2: 补充扫描无 fallback 的 `t()` 调用中不存在的 key**

```bash
cd D:/OneManager/AxAgent
# 提取所有 t("key") 调用（无第二个参数）
grep -roPh "t\(\s*['\"]([^'\"]+)['\"]\s*\)" src/ --include='*.ts' --include='*.tsx' \
  | sed "s/.*t(['\"]//" | sed "s/['\"].*//" | sort -u > /tmp/all-t-no-fallback.txt

node -e "
const fs = require('fs');
const enUS = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json','utf8'));
const keys = fs.readFileSync('/tmp/all-t-no-fallback.txt','utf8').trim().split('\n');
function keyExists(obj, path) {
  const parts = path.split('.');
  let cur = obj;
  for (const p of parts) { if (cur && typeof cur === 'object' && p in cur) cur = cur[p]; else return false; }
  return typeof cur === 'string';
}
const byNs = {};
keys.filter(k => !keyExists(enUS.translation || enUS, k)).forEach(k => {
  const ns = k.split('.')[0];
  if (!byNs[ns]) byNs[ns] = [];
  byNs[ns].push(k);
});
console.log('Keys without fallback missing from en-US:');
Object.entries(byNs).forEach(([ns,ks]) => ks.forEach(k => console.log(k)));
console.log('Total:', Object.values(byNs).reduce((s,a) => s + a.length, 0));
"
```

- [ ] **Step 3: 合并并去重，生成最终清单**

```bash
# 两份结果手动合并到 scripts/.i18n-missing-keys.json
# 预期总数: ~68 个 key（50 个来自 fallback + 18 个空白 key）
```

### Task 2: 补充缺失 key 到 zh-CN.json

**Files:**
- Modify: `src/i18n/locales/zh-CN.json`

- [ ] **Step 1: 从代码 fallback 值提取中文翻译，写入 zh-CN.json**

```bash
cd D:/OneManager/AxAgent
# 对每个缺失 key，在代码中搜索 t("key", "fallback") 模式提取 fallback 值
node -e "
const fs = require('fs');
const path = require('path');
const missingKeys = JSON.parse(fs.readFileSync('scripts/.i18n-missing-keys.json','utf8'));

// 搜索每个缺失 key 的 fallback 值
const fallbacks = {};
const srcFiles = [];
function walk(dir) {
  fs.readdirSync(dir, {withFileTypes:true}).forEach(e => {
    const full = path.join(dir, e.name);
    if (e.isDirectory() && !['node_modules','locales','__tests__'].includes(e.name)) walk(full);
    else if (e.name.endsWith('.ts') || e.name.endsWith('.tsx')) srcFiles.push(full);
  });
}
walk('src');

for (const key of missingKeys) {
  for (const file of srcFiles) {
    const content = fs.readFileSync(file, 'utf8');
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const re = new RegExp('t\\([\"\\'']' + escapedKey + '[\"\\'']\\s*,\\s*[\"\\'']([^\"\\'']+)[\"\\'']', 'g');
    let m;
    while ((m = re.exec(content)) !== null) {
      fallbacks[key] = m[1];
      break;
    }
    if (fallbacks[key]) break;
  }
}

// 将 fallback 值按命名空间分组
const result = {};
for (const [key, value] of Object.entries(fallbacks)) {
  const parts = key.split('.');
  let cur = result;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]]) cur[parts[i]] = {};
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}

fs.writeFileSync('scripts/.i18n-zhcn-additions.json', JSON.stringify(result, null, 2));
console.log('Keys with fallback values found:', Object.keys(fallbacks).length);
console.log('Keys without fallback (need manual translation):', missingKeys.filter(k => !fallbacks[k]).length);
console.log('Manual keys:', missingKeys.filter(k => !fallbacks[k]));
" 2>&1
```

- [ ] **Step 2: 手动补充无 fallback 的 key 的 zh-CN 翻译**

手动编辑 `scripts/.i18n-zhcn-additions.json`，为无 fallback 的 key（如 `research.mockFinding`、`skill.addFrontend` 等）填写合适的中文翻译。

- [ ] **Step 3: 合并到 zh-CN.json**

```bash
cd D:/OneManager/AxAgent
node -e "
const fs = require('fs');
const zhCN = JSON.parse(fs.readFileSync('src/i18n/locales/zh-CN.json','utf8'));
const additions = JSON.parse(fs.readFileSync('scripts/.i18n-zhcn-additions.json','utf8'));

function deepMerge(target, source) {
  for (const key of Object.keys(source)) {
    if (source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])) {
      if (!target[key]) target[key] = {};
      deepMerge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
}
deepMerge(zhCN, additions);
fs.writeFileSync('src/i18n/locales/zh-CN.json', JSON.stringify(zhCN, null, 2) + '\n');
console.log('Done. Keys merged into zh-CN.json');
"
```

- [ ] **Step 4: 验证 key 数量一致**

```bash
node -e "
const zhCN = JSON.parse(require('fs').readFileSync('src/i18n/locales/zh-CN.json','utf8'));
console.log('zh-CN.json total keys:', JSON.stringify(zhCN).length, 'bytes');
"
```

### Task 3: 补充缺失 key 到 en-US.json

**Files:**
- Modify: `src/i18n/locales/en-US.json`

- [ ] **Step 1: 为新 key 编写英文翻译，写入 en-US.json**

手动编辑 `scripts/.i18n-enus-additions.json`（从 zh-CN additions 派生结构，编写英文值），然后合并：

```bash
cd D:/OneManager/AxAgent
node -e "
const fs = require('fs');
const enUS = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json','utf8'));
// 从 zh-CN additions 复制结构，手动翻译为英文
// 此处展示合并逻辑：
const additions = JSON.parse(fs.readFileSync('scripts/.i18n-enus-additions.json','utf8'));

function deepMerge(target, source) {
  for (const key of Object.keys(source)) {
    if (source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])) {
      if (!target[key]) target[key] = {};
      deepMerge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
}
deepMerge(enUS, additions);
fs.writeFileSync('src/i18n/locales/en-US.json', JSON.stringify(enUS, null, 2) + '\n');
console.log('Done.');
"
```

- [ ] **Step 2: Commit**

```bash
git add src/i18n/locales/en-US.json src/i18n/locales/zh-CN.json
git commit -m "fix: add 68 missing i18n keys to zh-CN and en-US locale files"
```

### Task 4: 同步缺失 key 到其余 9 种语言文件

**Files:**
- Modify: `src/i18n/locales/zh-TW.json`, `src/i18n/locales/ja.json`, `src/i18n/locales/ko.json`, `src/i18n/locales/fr.json`, `src/i18n/locales/de.json`, `src/i18n/locales/es.json`, `src/i18n/locales/ru.json`, `src/i18n/locales/hi.json`, `src/i18n/locales/ar.json`

- [ ] **Step 1: 用 en-US 值填充其余 9 种语言的缺失 key**

```bash
cd D:/OneManager/AxAgent
node -e "
const fs = require('fs');
const enUS = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json','utf8'));
const missingKeys = JSON.parse(fs.readFileSync('scripts/.i18n-missing-keys.json','utf8'));

const langs = ['zh-TW', 'ja', 'ko', 'fr', 'de', 'es', 'ru', 'hi', 'ar'];
for (const lang of langs) {
  const file = 'src/i18n/locales/' + lang + '.json';
  const data = JSON.parse(fs.readFileSync(file, 'utf8'));

  for (const key of missingKeys) {
    const parts = key.split('.');
    // Navigate to the correct position in en-US
    let enVal = enUS;
    for (const p of parts) { enVal = enVal?.[p]; }

    // Navigate/create in target
    let cur = data;
    for (let i = 0; i < parts.length - 1; i++) {
      if (!cur[parts[i]]) cur[parts[i]] = {};
      cur = cur[parts[i]];
    }
    const lastPart = parts[parts.length - 1];
    if (cur[lastPart] === undefined) {
      cur[lastPart] = typeof enVal === 'string' ? (lang === 'zh-TW' ? enVal : enVal) : '';
      // For zh-TW, keep the en-US value as-is (will be manually translated later)
    }
  }

  fs.writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
  console.log('Updated: ' + lang);
}
console.log('All 9 languages synced.');
"
```

- [ ] **Step 2: Commit**

```bash
git add src/i18n/locales/zh-TW.json src/i18n/locales/ja.json src/i18n/locales/ko.json \
        src/i18n/locales/fr.json src/i18n/locales/de.json src/i18n/locales/es.json \
        src/i18n/locales/ru.json src/i18n/locales/hi.json src/i18n/locales/ar.json
git commit -m "fix: sync 68 missing i18n keys to all 9 non-primary locale files"
```

### Task 5: 搭建生成初始 allowlist 的脚本

**Files:**
- Create: `scripts/generate-i18n-allowlist.sh`

- [ ] **Step 1: 创建脚本**

```bash
#!/usr/bin/env bash
# scripts/generate-i18n-allowlist.sh
# 扫描项目中所有硬编码字符串，生成 scripts/.i18n-allowlist.json
# 用法: bash scripts/generate-i18n-allowlist.sh

set -euo pipefail
cd "$(dirname "$0")/.."

ALLOWLIST_FILE="scripts/.i18n-allowlist.json"
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "=== Scanning for hardcoded Chinese strings ==="

# 1. 扫描中文硬编码 (CJK Unified Ideographs)
echo "--- Chinese hardcoded strings ---"
grep -rPn '[\x{4e00}-\x{9fff}\x{3400}-\x{4dbf}]' src/ \
  --include='*.ts' --include='*.tsx' \
  | grep -v 'src/i18n/locales/' \
  | grep -v '^\s*//' \
  | grep -v '/\*' \
  > "$TEMP_DIR/chinese-vars.txt" || true

# 2. 扫描英文 UI 硬编码模式
echo "--- English UI hardcoded strings ---"
# message.success/error/warning/info
grep -rPn "(message\.(success|error|warning|info)\(\s*['\"])" src/ \
  --include='*.ts' --include='*.tsx' \
  > "$TEMP_DIR/english-messages.txt" || true

# placeholder="..."
grep -rPn 'placeholder\s*=\s*"[^"]{2,}"' src/ \
  --include='*.ts' --include='*.tsx' \
  > "$TEMP_DIR/english-placeholders.txt" || true

# Form.Item label="..."
grep -rPn 'label\s*=\s*"[A-Z][^"]{2,80}"' src/ \
  --include='*.ts' --include='*.tsx' \
  > "$TEMP_DIR/english-labels.txt" || true

# 3. 扫描 t() fallback 模式
echo "--- t() fallback patterns ---"
grep -rPn 't\(\s*['\''][^'\'']+['\'']\s*,\s*['\''][^'\'']+' src/ \
  --include='*.ts' --include='*.tsx' \
  > "$TEMP_DIR/fallback-patterns.txt" || true

# 汇总结果生成 JSON
node -e "
const fs = require('fs');
const path = require('path');

const entries = [];

// 解析中文硬编码文件
try {
  const chinese = fs.readFileSync('$TEMP_DIR/chinese-vars.txt', 'utf8').trim().split('\n').filter(Boolean);
  const fileMap = {};
  for (const line of chinese) {
    const [filePath, lineNum] = line.split(':');
    if (!filePath || !lineNum) continue;
    const relPath = filePath.replace(/^\.\//, '');
    if (!fileMap[relPath]) fileMap[relPath] = [];
    fileMap[relPath].push(parseInt(lineNum));
  }
  for (const [file, lines] of Object.entries(fileMap)) {
    entries.push({
      file,
      lines: lines.sort((a,b) => a-b).join(','),
      reason: '硬编码中文字符串',
      phase: 3
    });
  }
} catch(e) {}

// 解析英文 UI 消息
for (const f of ['english-messages.txt','english-placeholders.txt','english-labels.txt']) {
  try {
    const content = fs.readFileSync('\$TEMP_DIR/' + f, 'utf8').trim().split('\n').filter(Boolean);
    const fileMap = {};
    for (const line of content) {
      const [filePath, lineNum] = line.split(':');
      if (!filePath || !lineNum) continue;
      const relPath = filePath.replace(/^\.\//, '');
      if (!fileMap[relPath]) fileMap[relPath] = [];
      fileMap[relPath].push(parseInt(lineNum));
    }
    for (const [file, lines] of Object.entries(fileMap)) {
      entries.push({
        file,
        lines: lines.sort((a,b) => a-b).join(','),
        reason: '硬编码英文 UI 文本',
        phase: 3
      });
    }
  } catch(e) {}
}

// 解析 t() fallback
try {
  const fallbacks = fs.readFileSync('$TEMP_DIR/fallback-patterns.txt', 'utf8').trim().split('\n').filter(Boolean);
  const fileMap = {};
  for (const line of fallbacks) {
    const [filePath, lineNum] = line.split(':');
    if (!filePath || !lineNum) continue;
    const relPath = filePath.replace(/^\.\//, '');
    if (!fileMap[relPath]) fileMap[relPath] = new Set();
    fileMap[relPath].add(parseInt(lineNum));
  }
  for (const [file, lines] of Object.entries(fileMap)) {
    entries.push({
      file,
      lines: [...lines].sort((a,b) => a-b).join(','),
      reason: 't() fallback 参数',
      phase: 2
    });
  }
} catch(e) {}

// 合并同文件同 phase 的 entry
const merged = new Map();
for (const e of entries) {
  const key = e.file + '|||' + e.phase;
  if (merged.has(key)) {
    const existing = merged.get(key);
    const allLines = new Set([
      ...existing.lines.split(',').map(Number),
      ...e.lines.split(',').map(Number)
    ]);
    existing.lines = [...allLines].sort((a,b) => a-b).join(',');
  } else {
    merged.set(key, {...e});
  }
}

const result = {
  version: '1',
  generated: new Date().toISOString().split('T')[0],
  total_entries: merged.size,
  entries: [...merged.values()].sort((a,b) => a.file.localeCompare(b.file))
};

fs.writeFileSync(
  '$ALLOWLIST_FILE',
  JSON.stringify(result, null, 2) + '\n'
);
console.log('Allowlist generated: ' + result.total_entries + ' entries');
console.log('Phases: ', result.entries.reduce((acc, e) => {
  acc['phase'+e.phase] = (acc['phase'+e.phase] || 0) + 1;
  return acc;
}, {}));
"
echo "=== Done. Allowlist saved to $ALLOWLIST_FILE ==="
```

- [ ] **Step 2: 运行脚本生成初始 allowlist**

```bash
bash scripts/generate-i18n-allowlist.sh
```

- [ ] **Step 3: Commit**

```bash
git add scripts/generate-i18n-allowlist.sh scripts/.i18n-allowlist.json
git commit -m "feat: add i18n allowlist generator script and initial allowlist"
```

### Task 6: 搭建 CI 检测脚本

**Files:**
- Create: `scripts/check-hardcoded-i18n.sh`

- [ ] **Step 1: 创建检测脚本**

```bash
#!/usr/bin/env bash
# scripts/check-hardcoded-i18n.sh
# i18n 硬编码字符串检测脚本
# 模式:
#   --strict    : CI 阻断模式，新增违规 → exit 1
#   --report    : 本地报告模式，仅输出统计
#   --diff-only : 仅检查 git diff 中的新增行（增量模式）
# 用法: bash scripts/check-hardcoded-i18n.sh [--strict] [--report] [--diff-only]

set -euo pipefail
cd "$(dirname "$0")/.."

MODE="report"
ALLOWLIST="scripts/.i18n-allowlist.json"
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT
EXIT_CODE=0

# 解析命令行参数
for arg in "$@"; do
  case $arg in
    --strict) MODE="strict" ;;
    --report) MODE="report" ;;
    --diff-only) MODE="diff-only" ;;
    *) echo "Unknown option: $arg"; exit 2 ;;
  esac
done

# 确定检查范围
if [ "$MODE" = "diff-only" ]; then
  # 仅检查相对于 master 的 diff
  BASE_REF="origin/master"
  git fetch origin master --quiet 2>/dev/null || true
  CHANGED_FILES=$(git diff --name-only "$BASE_REF" HEAD | grep -E '\.(ts|tsx)$' | grep '^src/' | grep -v 'src/i18n/locales/' || true)
  if [ -z "$CHANGED_FILES" ]; then
    echo "No changed files to check."
    exit 0
  fi
  echo "Checking changed files:"
  echo "$CHANGED_FILES"
else
  CHANGED_FILES=$(find src -name '*.ts' -o -name '*.tsx' | grep -v 'src/i18n/locales/')
fi

# 从 allowlist 构建忽略列表
IGNORE_PATTERNS="$TEMP_DIR/ignore-patterns.txt"
node -e "
const fs = require('fs');
try {
  const al = JSON.parse(fs.readFileSync('$ALLOWLIST', 'utf8'));
  const lines = [];
  for (const e of al.entries || []) {
    for (const ln of e.lines.split(',')) {
      lines.push(e.file + ':' + ln);
    }
  }
  fs.writeFileSync('$IGNORE_PATTERNS', lines.join('\n'));
} catch(e) { fs.writeFileSync('$IGNORE_PATTERNS', ''); }
"

# 辅助函数: 检查某行是否在 allowlist 中
is_allowed() {
  local file="$1" line="$2"
  grep -qF "${file}:${line}" "$IGNORE_PATTERNS" 2>/dev/null
}

# === 规则 1: 中文硬编码 ===
echo "=== Rule 1: Hardcoded Chinese (CJK) strings ==="
VIOLATIONS_FILE="$TEMP_DIR/violations-chinese.txt"

for f in $CHANGED_FILES; do
  [ -f "$f" ] || continue
  grep -nP '[\x{4e00}-\x{9fff}\x{3400}-\x{4dbf}]' "$f" | while IFS=: read -r lnum content; do
    # 跳过注释行
    [[ "$content" =~ ^[[:space:]]*// ]] && continue
    [[ "$content" =~ ^[[:space:]]*\* ]] && continue
    [[ "$content" =~ ^[[:space:]]*/\* ]] && continue
    # 跳过 console.log/warn/error/debug
    [[ "$content" =~ console\.(log|warn|error|debug|info) ]] && continue
    # 检查 allowlist
    if ! is_allowed "$f" "$lnum"; then
      echo "$f:$lnum:$content" >> "$VIOLATIONS_FILE"
    fi
  done
done

if [ -f "$VIOLATIONS_FILE" ]; then
  V_COUNT=$(wc -l < "$VIOLATIONS_FILE")
  echo "  Found $V_COUNT new hardcoded Chinese string(s):"
  cat "$VIOLATIONS_FILE" | while IFS= read -r line; do
    echo "    $line"
  done
  EXIT_CODE=1
else
  echo "  No violations found."
fi

# === 规则 2: 英文 UI 硬编码 ===
echo "=== Rule 2: Hardcoded English UI strings ==="
VIOLATIONS_FILE2="$TEMP_DIR/violations-english.txt"

for f in $CHANGED_FILES; do
  [ -f "$f" ] || continue
  # message.success/error/warning/info("...")
  grep -nP "(message\.(success|error|warning|info)\(\s*['\"])" "$f" | while IFS=: read -r lnum content; do
    if ! is_allowed "$f" "$lnum"; then
      echo "$f:$lnum:EN_message:$content" >> "$VIOLATIONS_FILE2"
    fi
  done
  # placeholder="..."
  grep -nP 'placeholder\s*=\s*"[^"]{2,}"' "$f" | while IFS=: read -r lnum content; do
    if ! is_allowed "$f" "$lnum"; then
      echo "$f:$lnum:EN_placeholder:$content" >> "$VIOLATIONS_FILE2"
    fi
  done
  # notification.* message/description
  grep -nP '(notification\.\w+\(\s*\{[^}]*message\s*:\s*"[^"]+")' "$f" | while IFS=: read -r lnum content; do
    if ! is_allowed "$f" "$lnum"; then
      echo "$f:$lnum:EN_notification:$content" >> "$VIOLATIONS_FILE2"
    fi
  done
done

if [ -f "$VIOLATIONS_FILE2" ]; then
  V_COUNT2=$(wc -l < "$VIOLATIONS_FILE2")
  echo "  Found $V_COUNT2 new hardcoded English UI string(s):"
  cat "$VIOLATIONS_FILE2" | while IFS= read -r line; do
    echo "    $line"
  done
  EXIT_CODE=1
else
  echo "  No violations found."
fi

# === 规则 3: t() fallback 模式 (warning only) ===
echo "=== Rule 3: t() fallback patterns (WARNING) ==="
VIOLATIONS_FILE3="$TEMP_DIR/violations-fallback.txt"

for f in $CHANGED_FILES; do
  [ -f "$f" ] || continue
  grep -nP "t\(\s*['\"][^'\"]+['\"]\s*,\s*['\"][^'\"]+['\"]" "$f" | while IFS=: read -r lnum content; do
    if ! is_allowed "$f" "$lnum"; then
      echo "  WARNING: $f:$lnum $content"
      echo "$f:$lnum:$content" >> "$VIOLATIONS_FILE3"
    fi
  done
done

if [ -f "$VIOLATIONS_FILE3" ]; then
  V_COUNT3=$(wc -l < "$VIOLATIONS_FILE3")
  echo "  Found $V_COUNT3 t() fallback(s)."
fi

# === 汇总 ===
echo ""
echo "=== Summary ==="
if [ $EXIT_CODE -eq 0 ]; then
  echo "All i18n checks passed."
else
  echo "i18n violations detected. Add them to i18n or update scripts/.i18n-allowlist.json."
  echo ""
  echo "To regenerate the allowlist:"
  echo "  bash scripts/generate-i18n-allowlist.sh"
  echo ""
  echo "To run locally:"
  echo "  bash scripts/check-hardcoded-i18n.sh --report"
fi

exit $EXIT_CODE
```

- [ ] **Step 2: 执行权限 + 本地验证**

```bash
chmod +x scripts/check-hardcoded-i18n.sh
# 在本地运行 report 模式验证功能
bash scripts/check-hardcoded-i18n.sh --report
echo "Exit code: $?"
```

- [ ] **Step 3: Commit**

```bash
git add scripts/check-hardcoded-i18n.sh
git commit -m "feat: add i18n hardcoded strings CI check script"
```

### Task 7: 集成到 CI pipeline

**Files:**
- Modify: `.github/workflows/pr-ci.yml`

- [ ] **Step 1: 在 `frontend-check` job 末尾添加 i18n 检查步骤**

在 `.github/workflows/pr-ci.yml` 的 `frontend-check` job 中，`Build frontend` 步骤之后添加：

```yaml
      - name: Check i18n hardcoded strings
        run: bash scripts/check-hardcoded-i18n.sh --diff-only
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/pr-ci.yml
git commit -m "ci: add i18n hardcoded strings check to PR CI pipeline"
```

### Task 8: 运行 typecheck + test 确认无误

- [ ] **Step 1: 运行 typecheck**

```bash
cd D:/OneManager/AxAgent
npm run typecheck
```

- [ ] **Step 2: 运行前端测试**

```bash
npm run test:run
```

- [ ] **Step 3: 运行 i18n 检查确认通过**

```bash
bash scripts/check-hardcoded-i18n.sh --report
```

---

## 阶段 2: 消除 t() fallback

### Task 9: Batch 1 — 高频文件 fallback 消除

**Files:**
- Modify: `src/components/chat/ExpertSelector.tsx`, `src/components/help/HelpPanel.tsx` (~100 处)

- [ ] **Step 1: 提取 ExpertSelector.tsx 中所有 t() fallback 调用的 key**

```bash
cd D:/OneManager/AxAgent
grep -n "t(" src/components/chat/ExpertSelector.tsx | grep -oP "t\(\s*'([^']+)'\s*,\s*'" | sort -u
```

- [ ] **Step 2: 验证这些 key 在 en-US.json 中存在**

```bash
node -e "
const fs = require('fs');
const enUS = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json','utf8'));
// 手动列出 Step 1 找到的 key，逐一验证
const keys = ['expertSelector.xxx', '...']; // 替换为实际 key
keys.forEach(k => {
  const parts = k.split('.');
  let cur = enUS;
  for (const p of parts) cur = cur?.[p];
  console.log(k, typeof cur === 'string' ? 'EXISTS' : 'MISSING');
});
"
```

- [ ] **Step 3: 对于存在的 key，移除 fallback 参数**

对每个 fallback 调用 `t("expertSelector.xxx", "中文兜底")`，若 key 在 en-US.json 中已存在，改为 `t("expertSelector.xxx")`。

- [ ] **Step 4: 对于缺失的 key，先在 locale 文件中补充，再移除 fallback**

- [ ] **Step 5: 对 HelpPanel.tsx 重复 Step 1-4**

- [ ] **Step 6: 更新 allowlist，移除这两个文件的 fallback 条目**

```bash
node -e "
const fs = require('fs');
const al = JSON.parse(fs.readFileSync('scripts/.i18n-allowlist.json','utf8'));
al.entries = al.entries.filter(e =>
  e.file !== 'src/components/chat/ExpertSelector.tsx' &&
  e.file !== 'src/components/help/HelpPanel.tsx'
);
fs.writeFileSync('scripts/.i18n-allowlist.json', JSON.stringify(al, null, 2) + '\n');
console.log('Removed ExpertSelector and HelpPanel from allowlist');
"
```

- [ ] **Step 7: 运行 typecheck 并 commit**

```bash
npm run typecheck
git add src/components/chat/ExpertSelector.tsx src/components/help/HelpPanel.tsx scripts/.i18n-allowlist.json
git commit -m "fix: remove t() fallback parameters in ExpertSelector and HelpPanel"
```

### Task 10-12: Batch 2-4（其余文件 fallback 消除）

（结构与 Task 9 相同，依次处理设计文档中列出的各批次文件。每批完成后：
1. 验证 key 存在于 11 种语言文件
2. 移除 fallback 参数
3. 更新 allowlist
4. typecheck + commit）

Batch 2 文件: `KnowledgePage.tsx`, `WelcomeWizard.tsx`, `AcpSettings.tsx` (~82 处)
Batch 3 文件: `WikiDetailPanel.tsx`, `ContextGraphPanel.tsx`, `AgentGeneratorModal.tsx` (~61 处)
Batch 4 文件: 其余 60+ 文件 (~397 处)

---

## 阶段 3: 迁移硬编码 → t()

### Task 13: Layer 1 — 用户可见 UI 组件（~200 处，分 4 个子批次）

#### Subtask 13a: CitationManager + CredibilityBadge + BuddyMessage

**Files:**
- Modify: `src/components/chat/CitationManager.tsx`, `src/components/chat/CredibilityBadge.tsx`, `src/components/chat/BuddyMessage.tsx`
- Modify: `src/i18n/locales/en-US.json`, `src/i18n/locales/zh-CN.json`

- [ ] **Step 1: 列出 CitationManager.tsx 中所有硬编码中文**

当前违规：
- Line 30-38: `getSourceTypeName()` 返回值 "网页"、"学术" 等 → 改为 key 返回 + 调用方 `t()` 包裹
- Line 59: `引用管理 ({citations.length})` → `t("citationManager.title", { count: citations.length })`
- Line 63: `添加引用` → `t("citationManager.addCitation")`
- Line 71: `报告中使用的引用 ({citationsInReport.length})` → `t("citationManager.inReport", { count })`
- Line 121: `未使用的引用 ({citationsNotInReport.length})` → `t("citationManager.notInReport", { count })`
- Line 180: `暂无引用，请从搜索结果中添加` → `t("citationManager.empty")`
- Line 222: `来源类型分布:` → `t("citationManager.sourceDistribution")`

- [ ] **Step 2: 向 locale 文件添加 citationManager 命名空间下的所有 key**

```json
// en-US.json
"citationManager": {
  "title": "Citation Manager ({{count}})",
  "addCitation": "Add Citation",
  "inReport": "Citations in Report ({{count}})",
  "notInReport": "Unused Citations ({{count}})",
  "empty": "No citations yet. Add from search results.",
  "sourceDistribution": "Source Distribution:",
  "sourceType": { "web": "Web", "academic": "Academic", "wiki": "Wikipedia", "doc": "Document", "news": "News", "blog": "Blog", "forum": "Forum", "unknown": "Unknown" }
}
```

- [ ] **Step 3: 替换 CitationManager.tsx 中的硬编码为 t() 调用**

- [ ] **Step 4: 对 CredibilityBadge.tsx 和 BuddyMessage.tsx 重复 Step 1-3**

- [ ] **Step 5: 更新 allowlist + typecheck + commit**

#### Subtask 13b: ToolManager + BaseModal + WelcomeWizard

**Files:**
- Modify: `src/components/settings/ToolManager.tsx`, `src/components/shared/BaseModal.tsx`, `src/components/onboarding/WelcomeWizard.tsx`

（处理方式同 13a：列出违规 → 添加 locale key → 替换为 t() → 更新 allowlist → commit）

#### Subtask 13c: decomposition/ + devtools/ + benchmark/

**Files:**
- Modify: `src/components/decomposition/ToolDependencyList.tsx`, `src/components/decomposition/ToolInstallPanel.tsx`
- Modify: `src/components/devtools/TraceList.tsx`, `src/components/devtools/SpanTree.tsx`, `src/components/devtools/TraceFilters.tsx`
- Modify: `src/components/benchmark/BenchmarkConfig.tsx`

（处理方式同 13a）

#### Subtask 13d: skill/ + 其余组件

**Files:**
- Modify: `src/components/skill/FrontendEditorModal.tsx`, `src/components/skill/SkillSandboxContainer.tsx`
- Modify: `src/components/shared/ChartRenderer.tsx`, `src/components/shared/SearchProviderIcon.tsx`

（处理方式同 13a）

### Task 14: Layer 2 — Store 消息（~45 处）

**Files:**
- Modify: `src/stores/feature/agentStore.ts`, `src/stores/domain/conversationStore.ts`, `src/stores/feature/expertStore.ts`, `src/stores/feature/planStore.ts`, `src/stores/feature/topicGroupStore.ts`, `src/stores/feature/buddyStore.ts`, `src/stores/feature/skillExtensionStore.ts`
- Modify: `src/i18n/locales/en-US.json`, `src/i18n/locales/zh-CN.json`

- [ ] **Step 1: agentStore.ts 迁移**

将 `` `Agent 执行完成 · ${turns} 轮${cost}` `` 和 `` `Agent 执行失败: ${errMsg}` `` 等消息改为从 `t()` 获取模板：
```typescript
// 需要先让 store 能访问 t 函数。store 中可以通过 i18n 实例直接获取：
import i18n from "@/i18n";
// 使用: i18n.t("agentStore.completed", { turns, cost })
```

- [ ] **Step 2: conversationStore.ts 迁移**

将 "Agent 模式需要 Tauri 桌面端环境"、"正在思考..." 等改为 `i18n.t()`。
添加 locale key: `agentMode.requiresTauri`, `agentMode.requiresTauriDetail`, `agentMode.thinking`, `agentMode.timeout`.

- [ ] **Step 3: 其余 store 文件迁移（proactiveStore.ts 加注释豁免，不译）**

对 `proactiveStore.ts` 中的意图关键词，添加注释：
```typescript
// i18n-note: 以下关键词用于 NLP 意图检测，非 UI 文本，不翻译
```

- [ ] **Step 4: 更新 allowlist + typecheck + commit**

### Task 15: Layer 3 — lib 工具库（~160 处，按分类处理）

（按照设计文档的分类处理原则，对每个 lib 文件执行相应操作：
- 翻译 → `actionRouter.ts`, `memoryUtils.ts`, `exportChat.ts`, `skillPermissions.ts`
- 标记豁免 → `browserMock.ts`, `searchUtils.ts`, `chartGenerator.ts`
- 复用已有 → `constants.ts`）

### Task 16: Layer 4 — Rust 后端（低优先，~70 处）

（将 Rust 错误消息改为错误码模式，LLM 提示标记豁免）

---

## 阶段 4: 重构类型/数据层

### Task 17: 重构 localTool.ts

**Files:**
- Modify: `src/types/localTool.ts`
- Modify: `src/i18n/locales/en-US.json`, `src/i18n/locales/zh-CN.json`

- [ ] **Step 1: 确认 ToolCategoryLabels 和 PermissionModeLabels 零引用**

```bash
cd D:/OneManager/AxAgent
grep -r "ToolCategoryLabels\|PERMISSION_MODE_LABELS\|PermissionModeLabels" src/ --include='*.ts' --include='*.tsx'
```

- [ ] **Step 2: 若零引用，直接删除；若有引用，替换为 t() 方案**

```typescript
// 删除前:
export const ToolCategoryLabels: Record<ToolCategory, string> = {
  [ToolCategory.FileRead]: "文件读取",
  // ...
};

// 删除后: 在使用侧用 t(`toolCategory.${tool.category}`)
```

- [ ] **Step 3: 添加 locale key**

```json
// en-US.json
"toolCategory": {
  "fileRead": "File Read", "fileWrite": "File Write", "shellCommand": "Shell Command",
  "networkRequest": "Network Request", "systemTool": "System Tool", "agentTool": "Agent Tool",
  "versionControl": "Version Control", "automation": "Automation", "communication": "Communication",
  "aiMedia": "AI Media", "externalIntegration": "External Integration", "storageManagement": "Storage Management",
  "knowledgeBase": "Knowledge Base", "browser": "Browser", "desktopControl": "Desktop Control"
},
"permissionMode": {
  "readOnly": "Read Only", "allow": "Allow", "workspaceWrite": "Workspace Write",
  "fullAccess": "Full Access", "alwaysAsk": "Always Ask"
}
```

- [ ] **Step 4: Commit**

### Task 18: 重构 expert.ts

**Files:**
- Modify: `src/types/expert.ts`
- Modify: `src/components/settings/AgentProfileManager.tsx`
- Modify: `src/i18n/locales/en-US.json`, `src/i18n/locales/zh-CN.json`

- [ ] **Step 1: 将 EXPERT_CATEGORIES 改为纯 key 数组**

```typescript
// 改前:
export const EXPERT_CATEGORIES: Record<string, string> = {
  general: "通用", development: "开发", security: "安全",
  data: "数据", ops: "运维", design: "设计", writing: "写作", business: "商业",
};

// 改后:
export const EXPERT_CATEGORY_KEYS = [
  "general", "development", "security", "data", "ops", "design", "writing", "business",
] as const;
export type ExpertCategoryKey = (typeof EXPERT_CATEGORY_KEYS)[number];
```

- [ ] **Step 2: 在 locale 文件中添加对应 key**

```json
// en-US.json
"expertCategory": {
  "general": "General", "development": "Development", "security": "Security",
  "data": "Data", "ops": "Operations", "design": "Design", "writing": "Writing", "business": "Business"
}
```

- [ ] **Step 3: 更新所有引用方使用 `t("expertCategory." + category)`**

- [ ] **Step 4: 清理 AgentProfileManager.tsx 中重复定义的分类名**

- [ ] **Step 5: Commit**

### Task 19: 重构 evaluator.ts

**Files:**
- Modify: `src/types/evaluator.ts`

- [ ] **Step 1: 重构 getDifficultyLabel 和 getCategoryLabel 为返回 key**

```typescript
// 改前:
export function getDifficultyLabel(d: Difficulty): string {
  switch (d) { case Difficulty.Easy: return "简单"; ... }
}

// 改后: 返回 locale key，调用方需要 t() 包裹
export function getDifficultyKey(d: Difficulty): string {
  switch (d) { case Difficulty.Easy: return "difficulty.easy"; ... }
}
```

- [ ] **Step 2: 更新所有调用方使用 `t(getDifficultyKey(d))`**

- [ ] **Step 3: 添加 locale key + commit**

### Task 20: 重构 expertPresets.ts

**Files:**
- Modify: `src/data/expertPresets.ts`
- Modify: `src/i18n/locales/en-US.json`, `src/i18n/locales/zh-CN.json`

- [ ] **Step 1: 将预设名称和描述迁移到 locale 文件**

为每个预设添加 key: `expertPreset.{id}.name`, `expertPreset.{id}.description`

- [ ] **Step 2: 修改 preset 结构，名称/描述改为 key 引用**

```typescript
// 改前:
{ id: "general-assistant", name: "通用助手", description: "适用于各种通用任务...", systemPrompt: "你是一个..." }

// 改后: 名称和描述从 i18n 获取，systemPrompt 保留
{ id: "general-assistant", nameKey: "expertPreset.general-assistant.name", descKey: "expertPreset.general-assistant.description", systemPrompt: "你是一个..." }
```

- [ ] **Step 3: 更新所有使用预设的组件，使用 `t(preset.nameKey)`**

- [ ] **Step 4: Commit**

### Task 21: 最终收尾 — 清空 allowlist + 升级 CI 为 error 模式

- [ ] **Step 1: 确认 allowlist 已清空**

```bash
node -e "
const al = JSON.parse(require('fs').readFileSync('scripts/.i18n-allowlist.json','utf8'));
console.log('Remaining entries:', al.entries.length);
if (al.entries.length > 0) {
  console.log('Files remaining:');
  al.entries.forEach(e => console.log('  ' + e.file + ' (' + e.reason + ')'));
}
"
```

- [ ] **Step 2: 将 CI 检测从 warning 升级为 error**

修改 `scripts/check-hardcoded-i18n.sh`，将规则 3（t() fallback）也从 warning 升级为 exit 1。

修改 `.github/workflows/pr-ci.yml`，将 `--diff-only` 改为 `--strict`。

- [ ] **Step 3: 最终 commit**

```bash
git add -A
git commit -m "feat: complete i18n hardcoded strings cleanup, upgrade CI to strict mode"
```

---

## 附录

### 手动翻译清单（阶段 1）

以下 key 的英文翻译需人工编写（Phase 1 Task 3）：

**advancedSettings (9 keys):**
- `bashSecurity` → "Bash Security"
- `defaultPermission` → "Default Permission"
- `networkCmdDetect` → "Network Command Detection"
- `networkConfirm` → "Network Confirm"
- `perm.acceptEdits` → "Accept Edits"
- `perm.default` → "Default"
- `perm.fullAccess` → "Full Access"
- `permissionStrategy` → "Permission Strategy"
- `shellConfirm` → "Shell Confirm"

**benchmark (5 keys):**
- `configTitle` → "Benchmark Configuration"
- `empty` → "No benchmark tasks"
- `selectFirst` → "Select a task first"
- `selectTitle` → "Select Task"
- `title` → "Benchmark"

**chat.contextGraph (2 keys):**
- `hideType` → "Hide Type"
- `showType` → "Show Type"

**expertSelector (7 keys):**
- `builtinAlreadyImported` → "Already Imported"
- `builtinImported` → "Built-in Expert Imported"
- `builtinImportedLabel` → "Imported"
- `importBuiltin` → "Import Built-in"
- `importBuiltinBtn` → "Import"

**fineTune (7 keys):**
- `stats.completed` → "Completed"
- `stats.failed` → "Failed"
- `stats.running` → "Running"
- `stats.total` → "Total"
- `tab.dataset` → "Dataset"
- `tab.loraConfig` → "LoRA Config"
- `tab.training` → "Training"
- `title` → "Fine-Tuning"

**gateway (1 key):**
- `tab.monitor` → "Monitor"

**wiki (20 keys):**
- `browse` → "Browse"
- `dailyNote` → "Daily Note"
- `edges` → "Edges"
- `export` → "Export"
- `exportHtml` → "Export HTML"
- `exportMarkdown` → "Export Markdown"
- `exportPdf` → "Export PDF"
- `fromTemplate` → "From Template"
- `history` → "History"
- `import` → "Import"
- `importObsidian` → "Import Obsidian"
- `importObsidianDesc` → "Import vault from Obsidian"
- `nodes` → "Nodes"
- `preview` → "Preview"
- `quickCapture` → "Quick Capture"
- `refresh` → "Refresh"
- `source` → "Source"
- `startImport` → "Start Import"
- `vaultPath` → "Vault Path"
- `wiki` → "Wiki"

### 需要手动翻译的无 fallback key（18 个）

- `research.mockFinding` → "Mock research finding..."
- `skill.addFrontend` → "Add Frontend"
- `skill.editFrontend` → "Edit Frontend"
- `skill.exportFailed` → "Export Failed"
- `skill.exportPublishable` → "Export Publishable"
- `skill.exported` → "Exported"
- `skill.hasUI` → "Has UI"
- `wiki.exportResult` → "Export Result"
- `wiki.exportedPdf` → "Exported PDF"
- `wiki.filteredByTag` → "Filtered by Tag"
- 其余按实际扫描结果补充
