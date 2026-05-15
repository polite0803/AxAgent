const fs = require('fs');
const path = require('path');
const enUS = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json', 'utf8'));

function keyExists(obj, p) {
  const parts = p.split('.');
  let cur = obj;
  for (const part of parts) {
    if (cur && typeof cur === 'object' && part in cur) {
      cur = cur[part];
    } else {
      return false;
    }
  }
  return typeof cur === 'string';
}

// Extract key from a match - handles dynamic keys with ${...}
function isDynamicKey(key) {
  return key.includes('${');
}

// Find all files with t() calls that have fallbacks
function findFiles(dir) {
  const results = [];
  function walk(d) {
    const entries = fs.readdirSync(d, { withFileTypes: true });
    for (const e of entries) {
      const full = path.join(d, e.name);
      if (e.isDirectory()) {
        if (!['node_modules', 'locales', '__tests__'].includes(e.name)) walk(full);
      } else if (e.name.endsWith('.ts') || e.name.endsWith('.tsx')) {
        const content = fs.readFileSync(full, 'utf8');
        // Check for t() with a second argument (string or defaultValue object)
        if (/defaultValue:\s*['"`]/.test(content) || /\bt\(\s*['"`][^'"`]+['"`]\s*,\s*['"`]/.test(content)) {
          results.push(full);
        }
      }
    }
  }
  walk(dir);
  return results;
}

console.log('Finding files with t() fallbacks...');
const files = findFiles('src');
console.log(`Found ${files.length} files`);

// First verify ALL keys exist (skip dynamic keys)
console.log('\nVerifying keys...');
let missingCount = 0;
for (const file of files) {
  const content = fs.readFileSync(file, 'utf8');

  // Extract keys ONLY from calls that have fallbacks
  // Pattern A: t("key", "string")
  const reA = /\bt\(\s*(['"`])([^'"`]+)\1\s*,\s*(['"`])/g;
  let m;
  while ((m = reA.exec(content)) !== null) {
    const key = m[2];
    if (isDynamicKey(key)) continue;
    if (!keyExists(enUS, key)) {
      console.log(`  MISSING: ${key} in ${file}`);
      missingCount++;
    }
  }

  // Pattern B: t("key", { defaultValue: ... })
  const reB = /\bt\(\s*(['"`])([^'"`]+)\1\s*,\s*\{\s*defaultValue:/g;
  while ((m = reB.exec(content)) !== null) {
    const key = m[2];
    if (isDynamicKey(key)) continue;
    if (!keyExists(enUS, key)) {
      console.log(`  MISSING: ${key} in ${file}`);
      missingCount++;
    }
  }
}

if (missingCount > 0) {
  console.log(`\n${missingCount} missing keys found. STOPPING.`);
  console.log('Add these keys to en-US.json first, then re-run.');
  process.exit(1);
}
console.log('All keys exist ✓');

// ============ REMOVE FALLBACKS ============
console.log('\nRemoving fallbacks...');
let totalRemoved = 0;
const stats = {};

for (const file of files) {
  let content = fs.readFileSync(file, 'utf8');
  const original = content;
  let fileCount = 0;

  // ----------------------------------------
  // Rule 1: t("key", "string") -> t("key")
  // Handles double/single/backtick quotes for the string value
  // The key must be a static string (no ${} template expressions)
  // ----------------------------------------
  content = content.replace(
    /\bt\(\s*(['"`])([^'"`]+?)\1\s*,\s*(['"`])([^'"`]*?)\3\s*\)/g,
    (match, q, key) => {
      // Skip dynamic keys (they have template expressions)
      if (isDynamicKey(key)) return match;
      fileCount++;
      return `t(${q}${key}${q})`;
    }
  );

  // ----------------------------------------
  // Rule 2: t("key", "string", { ... }) -> t("key", { ... })
  // Key must be static (no ${})
  // ----------------------------------------
  content = content.replace(
    /\bt\(\s*(['"`])([^'"`]+?)\1\s*,\s*(['"`])([^'"`]*?)\3\s*,\s*(\{)/g,
    (match, q, key) => {
      if (isDynamicKey(key)) return match;
      fileCount++;
      return `t(${q}${key}${q}, {`;
    }
  );

  // ----------------------------------------
  // Rule 3: t("key", { defaultValue: "str" }) -> t("key")
  // ----------------------------------------
  content = content.replace(
    /\bt\(\s*(['"`])([^'"`]+?)\1\s*,\s*\{\s*defaultValue:\s*(['"`])([^'"`]*?)\3\s*\}\)/g,
    (match, q, key) => {
      if (isDynamicKey(key)) return match;
      fileCount++;
      return `t(${q}${key}${q})`;
    }
  );

  // ----------------------------------------
  // Rule 4: t("key", { defaultValue: "str", ...rest }) -> t("key", { ...rest })
  // defaultValue at the start of the object
  // ----------------------------------------
  content = content.replace(
    /\bt\(\s*(['"`])([^'"`]+?)\1\s*,\s*\{\s*defaultValue:\s*(['"`])([^'"`]*?)\3\s*,\s*/g,
    (match, q, key) => {
      if (isDynamicKey(key)) return match;
      fileCount++;
      return `t(${q}${key}${q}, { `;
    }
  );

  // ----------------------------------------
  // Rule 5: Trailing defaultValue: ,defaultValue: "str"
  // Removes remaining defaultValue properties from objects
  // Handles ,defaultValue: "str" } and ,defaultValue: "str" },
  // ----------------------------------------
  content = content.replace(
    /,\s*defaultValue:\s*(['"`])([^'"`]*?)\1\s*([,\}])/g,
    (match, _q, _val, after) => {
      fileCount++;
      // If after comma, keep the comma; if after closing brace, remove it
      return after === ',' ? ',' : '';
    }
  );

  if (content !== original) {
    fs.writeFileSync(file, content, 'utf8');
    const relPath = file.replace(/\\/g, '/').replace(/^.*?\/src\//, 'src/');
    stats[relPath] = (stats[relPath] || 0) + fileCount;
    totalRemoved += fileCount;
    // Log small changes for verification
    if (fileCount > 0) {
      console.log(`  ${fileCount} in ${relPath}`);
    }
  }
}

console.log(`\nRemoved ${totalRemoved} fallbacks from ${Object.keys(stats).length} files`);

// Write stats report
const report = { totalRemoved, filesProcessed: Object.keys(stats).length, perFile: stats };
fs.writeFileSync('scripts/.batch4-report.json', JSON.stringify(report, null, 2));
console.log('\nPer-file breakdown:');
Object.entries(stats)
  .sort((a, b) => b[1] - a[1])
  .forEach(([f, c]) => console.log(`  ${c.toString().padStart(3)}  ${f}`));
