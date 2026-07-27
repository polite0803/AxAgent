// i18n 键同步脚本 — 以 en-US.json 为参考，向其他语言补全缺失的键
// 用法: node scripts/sync-i18n.mjs [--dry-run]

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const localesDir = resolve(__dirname, "..", "src", "i18n", "locales");
const dryRun = process.argv.includes("--dry-run");

// 读取参考文件 (en-US)
const refPath = resolve(localesDir, "en-US.json");
const ref = JSON.parse(readFileSync(refPath, "utf8"));

// 深度遍历提取所有叶子键路径
function collectLeafPaths(obj, prefix = "") {
  const paths = [];
  for (const [k, v] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      paths.push(...collectLeafPaths(v, path));
    } else {
      paths.push(path);
    }
  }
  return paths;
}

// 用路径设置嵌套值 (不覆盖已有值)
function setByPath(obj, path, defaultValue) {
  const parts = path.split(".");
  let current = obj;
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    if (i === parts.length - 1) {
      if (!(part in current)) {
        current[part] = defaultValue;
      }
    } else {
      if (!(part in current) || typeof current[part] !== "object" || Array.isArray(current[part])) {
        current[part] = {};
      }
      current = current[part];
    }
  }
}

const refPaths = collectLeafPaths(ref);
console.log(`参考文件 en-US.json: ${refPaths.length} 个叶子键`);

const localeFiles = readdirSync(localesDir)
  .filter(f => f.endsWith(".json") && f !== "en-US.json");

let totalAdded = 0;

for (const file of localeFiles) {
  const filePath = resolve(localesDir, file);
  const data = JSON.parse(readFileSync(filePath, "utf8"));
  const beforePaths = collectLeafPaths(data);

  let added = 0;
  for (const path of refPaths) {
    const parts = path.split(".");
    // 深度检查路径是否存在且有值
    let current = data;
    let exists = true;
    for (const part of parts) {
      if (!current || typeof current !== "object" || Array.isArray(current) || !(part in current)) {
        exists = false;
        break;
      }
      current = current[part];
    }
    // 也检查值是否是有效叶子（非对象，或空对象）
    if (
      exists && current && typeof current === "object" && !Array.isArray(current) && Object.keys(current).length > 0
    ) {
      exists = false; // 这是一个中间节点变成了叶子值的情况
    }

    if (!exists) {
      // 使用英文值作为默认，标记为 TODO
      let refValue = ref;
      for (const part of parts) {
        refValue = refValue?.[part];
      }
      const defaultValue = typeof refValue === "string"
        ? `[TODO] ${refValue}`
        : refValue ?? `[TODO: ${path}]`;
      setByPath(data, path, defaultValue);
      added++;
    }
  }

  if (added > 0) {
    if (!dryRun) {
      writeFileSync(filePath, JSON.stringify(data, null, 2) + "\n", "utf8");
    }
    console.log(`${file}: +${added} 键 (${beforePaths.length} → ${beforePaths.length + added})`);
    totalAdded += added;
  } else {
    console.log(`${file}: 已同步，无需更新`);
  }
}

console.log(`\n总计添加: ${totalAdded} 个键${dryRun ? " (dry-run, 未写入)" : ""}`);
