// 脚本：找出缺失的命令注册并生成注册代码
import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, resolve, dirname, sep } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const SRC_TAURI = join(ROOT, "src-tauri");
const COMMANDS_DIR = join(SRC_TAURI, "src", "commands") + sep;

function walk(dir, ext, out = []) {
  if (!existsSync(dir)) return out;
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, ext, out);
    else if (p.endsWith(ext)) out.push(p);
  }
  return out;
}

// Step 1: Get registered commands from register_commands.rs
const regFile = join(SRC_TAURI, "src", "register_commands.rs");
const regSrc = readFileSync(regFile, "utf8");
const start = regSrc.indexOf("generate_handler![");
let registered = new Set();
if (start >= 0) {
  let depth = 0, i = start;
  for (; i < regSrc.length; i++) {
    if (regSrc[i] === "[") depth++;
    else if (regSrc[i] === "]") { depth--; if (depth === 0) break; }
  }
  const block = regSrc.slice(start, i + 1);
  for (const m of block.matchAll(/^\s*(?:commands::)?(?:[\w]+::)+\w+\s*,?\s*$/gm)) {
    const parts = m[0].split("::");
    registered.add(parts[parts.length - 1].replace(/[,\s]/g, ""));
  }
}

// Step 2: Find all defined commands
const cmdFiles = walk(join(SRC_TAURI, "src"), ".rs");
const reDef = /#\[(?:tauri::)?command\][\s\S]*?(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)/g;

const defined = new Map();

for (const f of cmdFiles) {
  // Get relative path from commands directory
  let relPath = f.slice(COMMANDS_DIR.length);
  // Normalize to forward slashes
  relPath = relPath.replace(/\\/g, "/");
  const srcClean = readFileSync(f, "utf8").replace(/\/\/.*$/gm, "");

  for (const m of srcClean.matchAll(reDef)) {
    const fnName = m[1];
    let modulePath = "";
    if (relPath.endsWith(".rs")) {
      const withoutExt = relPath.slice(0, -3);
      modulePath = "commands::" + withoutExt.replace(/\//g, "::");
    }
    if (!defined.has(fnName)) {
      defined.set(fnName, { file: relPath, modulePath });
    }
  }
}

// Step 3: Find missing
const missing = [];
for (const [fnName, info] of defined) {
  if (!registered.has(fnName)) {
    missing.push({ fnName, ...info });
  }
}

console.log("已定义命令数:", defined.size);
console.log("已注册命令数:", registered.size);
console.log("缺失命令数:", missing.length);

// Group by module
const byModule = {};
for (const m of missing) {
  if (!byModule[m.modulePath]) byModule[m.modulePath] = [];
  byModule[m.modulePath].push(m.fnName);
}

// Generate registration code
let output = "";
for (const [mod, fns] of Object.entries(byModule).sort()) {
  output += `// ${mod.replace(/^commands::/, "")}\n`;
  fns.sort().forEach(fn => {
    output += `${mod}::${fn},\n`;
  });
  output += "\n";
}

// Write to file
const outputFile = join(ROOT, "scripts", "_missing_commands.txt");
writeFileSync(outputFile, output, "utf8");
console.log(`\n缺失命令注册代码已写入: ${outputFile}`);

console.log("\n按模块分组:");
for (const [mod, fns] of Object.entries(byModule).sort()) {
  console.log(`  ${mod} (${fns.length} 个)`);
  fns.sort().forEach(fn => console.log(`    - ${fn}`));
}
