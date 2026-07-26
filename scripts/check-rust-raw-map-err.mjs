#!/usr/bin/env node
// 后端错误码 i18n 守卫（Phase 4.1）
//
// 禁止 src-tauri/src/commands 下出现「裸」map_err(|x| x.to_string())。
// 后端命令错误必须携带错误码（ErrorResponse）返回，供前端按 error.{CODE} 翻译，
// 否则该错误在切语言后会退回原始英文串、破坏 i18n。
//
// 正确写法：
//   .map_err(|e| String::from(
//     crate::commands::error::ErrorResponse::from_error(
//       e, crate::commands::error::ErrorCategory::Unrecoverable)))
//
// 见 AGENTS.md「后端错误码 i18n 规范（强制）」。

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const target = join(root, "src-tauri", "src", "commands");

// 精确匹配「身份式」unwrap：map_err(|IDENT| IDENT.to_string())
const RE = /map_err\(\|(\w+)\|\s*\1\.to_string\(\)\)/;

function walk(dir, out) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) { walk(p, out); }
    else if (/\.rs$/.test(name)) { out.push(p); }
  }
}

const files = [];
walk(target, files);

let violations = 0;
for (const f of files) {
  const rel = f.replace(root + "/", "");
  const lines = readFileSync(f, "utf8").split("\n");
  lines.forEach((line, i) => {
    if (RE.test(line)) {
      violations++;
      console.error(`::error file=${rel},line=${i + 1}::裸 map_err(|x| x.to_string()) 禁止：后端错误必须带错误码返回`);
      console.error(`   ${rel}:${i + 1}: ${line.trim()}`);
    }
  });
}

if (violations > 0) {
  console.error(`\n发现 ${violations} 处裸 map_err(|x| x.to_string())。`);
  console.error(
    "正确写法：map_err(|e| String::from(crate::commands::error::ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)))",
  );
  process.exit(1);
}
console.log("OK: src-tauri/src/commands 未发现裸 map_err(|x| x.to_string())");
