// 将缺失命令添加到 register_commands.rs
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// Read the register_commands.rs file
const regFile = join(ROOT, "src-tauri", "src", "register_commands.rs");
let content = readFileSync(regFile, "utf8");

// Read the missing commands
const missingFile = join(ROOT, "scripts", "_missing_commands.txt");
const missing = readFileSync(missingFile, "utf8");

// Find the last ] in generate_handler![ block
const lastBracket = content.lastIndexOf("]");
if (lastBracket === -1) {
  console.error("Cannot find closing bracket");
  process.exit(1);
}

// Add the missing commands before the closing bracket
const insertContent =
  '\n            // ── AxInvest 业务域命令（stock_analysis/opc/quant/market_sim 等）──\n' +
  missing.trim() +
  "\n";

content =
  content.slice(0, lastBracket) +
  insertContent +
  "        " +
  content.slice(lastBracket);

writeFileSync(regFile, content, "utf8");
console.log("Successfully added missing commands to register_commands.rs");
console.log("File size:", content.length, "bytes");
