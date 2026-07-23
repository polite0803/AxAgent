// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 从 zh-CN.json 同步缺失的 i18n key 到其他 10 个语言文件。
 *
 * 策略:
 *   - 以 zh-CN.json 为权威源
 *   - 对每个其他语言文件,递归找出 zh-CN 存在但该文件缺失的 key
 *   - 将缺失的 key 从 zh-CN 复制过去作为占位符(值为 zh-CN 的中文)
 *   - 已有的 key 不被覆盖,保持现有翻译
 *   - 保持目标文件原有 key 顺序,新增 key 追加到所属对象末尾
 *
 * 用法: node scripts/sync-i18n-from-zhcn.cjs
 * 可重入: 已存在的 key 不会被修改
 */

const fs = require("fs");
const path = require("path");

const LOCALES_DIR = path.resolve(__dirname, "..", "src", "i18n", "locales");
const ZH_CN_PATH = path.join(LOCALES_DIR, "zh-CN.json");

/** 递归合并: 将 source 中存在但 target 缺失的 key 复制过去,返回新增 key 计数 */
function mergeMissing(target, source, fileLabel, keyPath = "") {
  let added = 0;
  for (const [k, srcVal] of Object.entries(source)) {
    const curPath = keyPath ? `${keyPath}.${k}` : k;
    if (!(k in target)) {
      // 缺失: 深拷贝 source 的值过去(对象则递归拷贝)
      if (srcVal && typeof srcVal === "object" && !Array.isArray(srcVal)) {
        target[k] = {};
        for (const [ck, cv] of Object.entries(srcVal)) {
          target[k][ck] = cv && typeof cv === "object" && !Array.isArray(cv)
            ? JSON.parse(JSON.stringify(cv))
            : cv;
        }
      } else {
        target[k] = srcVal;
      }
      added++;
    } else if (
      srcVal && typeof srcVal === "object" && !Array.isArray(srcVal)
      && target[k] && typeof target[k] === "object" && !Array.isArray(target[k])
    ) {
      // 两边都是对象,递归合并
      added += mergeMissing(target[k], srcVal, fileLabel, curPath);
    }
    // 若 source 是对象但 target 是基本类型(或反之),保留 target 现有值,不覆盖
  }
  return added;
}

function main() {
  if (!fs.existsSync(ZH_CN_PATH)) {
    console.error(`[FATAL] zh-CN.json 不存在: ${ZH_CN_PATH}`);
    process.exit(1);
  }
  const zhCN = JSON.parse(fs.readFileSync(ZH_CN_PATH, "utf8"));
  const files = fs.readdirSync(LOCALES_DIR).filter(
    (f) => f.endsWith(".json") && f !== "zh-CN.json",
  );

  let totalAdded = 0;
  for (const file of files) {
    const full = path.join(LOCALES_DIR, file);
    const original = fs.readFileSync(full, "utf8");
    const obj = JSON.parse(original);
    const added = mergeMissing(obj, zhCN, file);
    if (added > 0) {
      // 保持 2 空格缩进 + 末尾换行
      const out = JSON.stringify(obj, null, 2) + "\n";
      fs.writeFileSync(full, out, "utf8");
      console.log(`[OK] ${file}: 新增 ${added} 个 key`);
    } else {
      console.log(`[SKIP] ${file}: 无缺失 key`);
    }
    totalAdded += added;
  }
  console.log(`\n[汇总] 共新增 ${totalAdded} 个 key 到 ${files.length} 个语言文件`);
}

main();
