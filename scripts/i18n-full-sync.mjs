#!/usr/bin/env node
/**
 * 完整 i18n 同步:
 * 1. 补全 zh-CN 的 36 个缺失 key (用中文翻译)
 * 2. 用 union 全部 key 构建全集
 * 3. 每种语言缺失 key 用对应语言翻译填充（无翻译则选已有语言的 fallback）
 */
import { readFileSync, writeFileSync } from "node:fs";

function getAllKeys(obj, prefix = "") {
  const keys = [];
  for (const k of Object.keys(obj || {})) {
    const fullKey = prefix ? prefix + "." + k : k;
    if (typeof obj[k] === "object" && obj[k] !== null && !Array.isArray(obj[k])) {
      keys.push(...getAllKeys(obj[k], fullKey));
    } else { keys.push(fullKey); }
  }
  return keys;
}
function setValueByPath(obj, path, value) {
  const parts = path.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]] || typeof cur[parts[i]] !== "object") { cur[parts[i]] = {}; }
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}
function getValueByPath(obj, path) {
  const parts = path.split(".");
  let cur = obj;
  for (const p of parts) {
    if (cur == null || typeof cur !== "object") { return undefined; }
    cur = cur[p];
  }
  return cur;
}

// ── Step 1: 加载所有语言 ──
const langs = ["zh-CN", "en-US", "zh-TW", "ja", "ko", "fr", "de", "es", "ru", "hi", "ar"];
const data = {};
for (const l of langs) {
  data[l] = JSON.parse(readFileSync(`src/i18n/locales/${l}.json`, "utf-8"));
}

// ── Step 2: 补全 zh-CN 的 36 个缺失 key ──
const zhCNMissing = {
  "common.success": "成功",
  "common.config": "配置",
  "common.quality": "质量",
  "common.entropyCoeff": "熵系数",
  "common.gamma": "γ (折扣率)",
  "common.lambda": "λ (TD)",
  "common.rewardScale": "奖励缩放",
  "common.rlEngine": "RL 引擎",
  "common.rewardWeights.task_completion": "任务完成",
  "common.rewardWeights.reasoning_quality": "推理质量",
  "common.rewardWeights.tool_efficiency": "工具效率",
  "common.rewardWeights.pattern_match": "模式匹配",
  "common.rewardWeights.error_recovery": "错误恢复",
  "common.rewardWeights.user_feedback": "用户反馈",
  "profile.documentStyle": "文档风格",
  "style.applied": "已应用",
  "style.applyCodeStyle": "应用代码风格",
  "style.applyDocStyle": "应用文档风格",
  "style.exportProfile": "导出配置",
  "style.importProfile": "导入配置",
  "style.learnFromCode": "从代码学习",
  "style.learnFromMessages": "从消息学习",
  "settings.bocha": "Bocha",
  "settings.zhipu": "智谱",
  "skills.atomic": "原子",
  "skills.cancel": "取消",
  "skills.convertToWorkflow": "转换为工作流",
  "skills.createNewSkill": "创建新技能",
  "skills.dismiss": "忽略",
  "skills.extractAtomic": "提取原子技能",
  "skills.newest": "最新",
  "skills.popular": "热门",
  "skills.proposals": "提案",
  "skills.save": "保存",
  "skills.sort": "排序",
  "agent.submitted": "已提交",
};

for (const [key, val] of Object.entries(zhCNMissing)) {
  setValueByPath(data["zh-CN"], key, val);
}
console.log("zh-CN: +" + Object.keys(zhCNMissing).length + " keys added");

// ── Step 3: 构建全集 ──
const allKeys = new Set();
for (const l of langs) { getAllKeys(data[l]).forEach(k => allKeys.add(k)); }
console.log("Complete key set: " + allKeys.size + " keys");

// ── Step 4: 为每种语言补全缺失 key ──
// 查找优先级: 该语言已有 > en-US > zh-TW > zh-CN
const lookupOrder = ["en-US", "zh-TW", "ja", "ko", "de", "fr", "es", "zh-CN"];

for (const lang of langs) {
  if (lang === "zh-CN") { continue; // zh-CN already complete now
   }

  const langKeys = new Set(getAllKeys(data[lang]));
  const missing = [...allKeys].filter(k => !langKeys.has(k));
  if (missing.length === 0) {
    console.log(lang + ": already complete");
    continue;
  }

  let filled = 0;
  for (const key of missing) {
    // Try lookup order
    let val = undefined;
    for (const src of lookupOrder) {
      if (src === lang) { continue; }
      val = getValueByPath(data[src], key);
      if (val !== undefined) { break; }
    }
    if (val !== undefined) {
      setValueByPath(data[lang], key, val);
      filled++;
    }
  }
  console.log(lang + ": " + langKeys.size + " → " + (langKeys.size + filled) + " (+" + filled + ")");
}

// ── Step 5: 写入 ──
for (const l of langs) {
  writeFileSync(`src/i18n/locales/${l}.json`, JSON.stringify(data[l], null, 2) + "\n", "utf-8");
}
console.log("\nAll 11 language files synced.");
