#!/usr/bin/env node
/**
 * AxAgent Skill 校验脚本
 * 用法: node scripts/validate-skill.mjs <skill-dir>
 *
 * 验证 skill 目录的结构和内容完整性，报告所有错误和警告。
 */

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { exit } from "node:process";

const args = process.argv.slice(2);
if (args.length === 0 || args[0] === "--help" || args[0] === "-h") {
  console.log("用法: node scripts/validate-skill.mjs <skill-dir>");
  console.log("示例: node scripts/validate-skill.mjs ~/.axagent/skills/my-skill");
  exit(0);
}

const skillDir = resolve(args[0]);
const skillName = basename(skillDir);
const errors = [];
const warnings = [];

function addError(msg) {
  errors.push(msg);
}
function addWarn(msg) {
  warnings.push(msg);
}

// ── 1. 目录存在性 ──
if (!existsSync(skillDir)) {
  addError(`目录不存在: ${skillDir}`);
  printReport();
  exit(1);
}

// ── 2. SKILL.md ──
const skillMdPath = join(skillDir, "SKILL.md");
if (!existsSync(skillMdPath)) {
  addError("缺少 SKILL.md（Agent 指令文件）");
} else {
  const content = readFileSync(skillMdPath, "utf-8");
  if (content.trim().length === 0) {
    addError("SKILL.md 内容为空");
  }
  if (content.trim().length < 50) {
    addWarn("SKILL.md 内容过短（少于 50 字符），技能指令可能不够详细");
  }
}

// ── 3. manifest.json ──
const manifestPath = join(skillDir, "manifest.json");
const skillManifestPath = join(skillDir, "skill-manifest.json");

let manifest = null;
const manifestFile = existsSync(manifestPath)
  ? manifestPath
  : existsSync(skillManifestPath)
  ? skillManifestPath
  : null;

if (!manifestFile) {
  addError("缺少 manifest.json 或 skill-manifest.json");
} else {
  try {
    manifest = JSON.parse(readFileSync(manifestFile, "utf-8"));
  } catch (e) {
    addError(`manifest 文件 JSON 解析失败: ${e.message}`);
  }
}

if (manifest) {
  // 必需字段
  if (!manifest.name) { addError('manifest.json 缺少 "name" 字段'); }
  if (!manifest.version) { addWarn('manifest.json 缺少 "version" 字段'); }

  // 前端扩展验证
  if (manifest.frontend) {
    const f = manifest.frontend;
    const extTypes = [
      "navigation",
      "pages",
      "commands",
      "panels",
      "settingsSections",
      "toolbar",
      "chatCommand",
      "statusBar",
    ];
    for (const ext of extTypes) {
      if (f[ext] && !Array.isArray(f[ext])) {
        addError(`frontend.${ext} 必须是数组`);
      }
    }

    // 检查 navigation 关联的 pageId
    if (Array.isArray(f.navigation)) {
      const pageIds = new Set((f.pages || []).map((p) => p.id));
      for (const nav of f.navigation) {
        if (!nav.id) { addWarn("navigation 项缺少 id"); }
        if (nav.pageId && !pageIds.has(nav.pageId)) {
          addWarn(`navigation.${nav.id} 引用了不存在的 pageId: ${nav.pageId}`);
        }
      }
    }

    // 检查 commands 的 actions
    if (Array.isArray(f.commands)) {
      for (const cmd of f.commands) {
        if (!cmd.actions || cmd.actions.length === 0) {
          addWarn(`command "${cmd.id || cmd.label}" 缺少 actions`);
        }
        if (cmd.actions) {
          for (const action of cmd.actions) {
            validateAction(action, `command "${cmd.id || cmd.label}"`);
          }
        }
      }
    }

    // 检查 toolbar 的 actions
    if (Array.isArray(f.toolbar)) {
      for (const btn of f.toolbar) {
        if (!btn.onClick || btn.onClick.length === 0) {
          addWarn(`toolbar "${btn.id}" 缺少 onClick actions`);
        }
      }
    }

    // 检查 pages 的 componentType
    if (Array.isArray(f.pages)) {
      const validTypes = ["Html", "Iframe", "React", "WebComponent", "Markdown"];
      for (const page of f.pages) {
        if (page.componentType && !validTypes.includes(page.componentType)) {
          addError(`page "${page.id}" componentType "${page.componentType}" 无效，有效值: ${validTypes.join(", ")}`);
        }
      }
    }
  }

  // handlers 验证
  if (manifest.handlers) {
    for (const [name, handler] of Object.entries(manifest.handlers)) {
      if (!handler.mode) {
        addError(`handler "${name}" 缺少 mode 字段`);
      } else if (!["declarative", "agentic"].includes(handler.mode)) {
        addError(`handler "${name}" mode "${handler.mode}" 无效`);
      }
      if (handler.mode === "declarative" && (!handler.actions || handler.actions.length === 0)) {
        addWarn(`handler "${name}" 是 declarative 模式但缺少 actions`);
      }
      if (handler.mode === "agentic" && !handler.promptTemplate) {
        addWarn(`handler "${name}" 是 agentic 模式但缺少 promptTemplate`);
      }
    }
  }

  // 权限验证
  if (manifest.permissions) {
    if (manifest.permissions.tools && !Array.isArray(manifest.permissions.tools)) {
      addError("permissions.tools 必须是数组");
    }
    if (manifest.permissions.commands && !Array.isArray(manifest.permissions.commands)) {
      addError("permissions.commands 必须是数组");
    }
  }
}

// ── Action 验证辅助 ──
function validateAction(action, context) {
  if (!action) { return; }
  if (!action.mode) {
    addWarn(`${context}: action 缺少 mode 字段`);
    return;
  }
  if (!["declarative", "agentic"].includes(action.mode)) {
    addError(`${context}: 无效的 action mode "${action.mode}"`);
    return;
  }
  if (action.mode === "declarative") {
    if (!action.action || !action.action.type) {
      addError(`${context}: declarative action 缺少 action.type`);
      return;
    }
    const validTypes = ["invoke", "navigate", "emit", "store", "function", "handler", "chain"];
    if (!validTypes.includes(action.action.type)) {
      addError(`${context}: 无效的 action type "${action.action.type}"`);
    }
    if (action.action.type === "invoke" && !action.action.command) {
      addWarn(`${context}: invoke action 缺少 command`);
    }
  }
  if (action.mode === "agentic") {
    if (!action.prompt) {
      addWarn(`${context}: agentic action 缺少 prompt`);
    }
  }
}

// ── 4. 资源文件 ──
const assetsDir = join(skillDir, "assets");
if (existsSync(assetsDir)) {
  const files = readdirSync(assetsDir).filter((f) => f !== ".gitkeep");
  if (files.length === 0) {
    addWarn("assets/ 目录为空，如果页面引用了资源文件请放入此目录");
  }
} else {
  addWarn("缺少 assets/ 目录");
}

// ── 5. dist 目录 ──
const distDir = join(skillDir, "dist");
if (existsSync(distDir)) {
  const hasJs = readdirSync(distDir).some((f) => f.endsWith(".js"));
  const hasReactOrWc = manifest?.frontend?.pages?.some(
    (p) => p.componentType === "React" || p.componentType === "WebComponent",
  );
  if (hasReactOrWc && !hasJs) {
    addWarn("有 React/WebComponent 页面但 dist/ 中没有 .js 构建产物");
  }
}

// ── 输出报告 ──
printReport();

function printReport() {
  console.log("");
  console.log(`📋 Skill 校验报告: ${skillName}`);
  console.log(`   路径: ${skillDir}`);
  console.log("");

  if (errors.length === 0 && warnings.length === 0) {
    console.log("✅ 全部通过，没有发现错误或警告。");
    console.log("");
    return;
  }

  if (errors.length > 0) {
    console.log(`❌ 错误 (${errors.length}):`);
    for (const e of errors) {
      console.log(`   • ${e}`);
    }
    console.log("");
  }

  if (warnings.length > 0) {
    console.log(`⚠️  警告 (${warnings.length}):`);
    for (const w of warnings) {
      console.log(`   • ${w}`);
    }
    console.log("");
  }

  if (errors.length > 0) {
    console.log("建议: 修复以上错误后再安装此 skill。");
  }
}

exit(errors.length > 0 ? 1 : 0);
