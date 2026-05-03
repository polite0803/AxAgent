#!/usr/bin/env node
/**
 * AxAgent Skill 开发脚手架
 * 用法: node scripts/create-axagent-skill.mjs <skill-name> [target-dir]
 *
 * 生成完整的 skill 项目目录结构，包括 SKILL.md、manifest.json、示例资源。
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { exit } from "node:process";

const args = process.argv.slice(2);
if (args.length === 0 || args[0] === "--help" || args[0] === "-h") {
  console.log("用法: node scripts/create-axagent-skill.mjs <skill-name> [target-dir]");
  console.log("");
  console.log("示例:");
  console.log("  node scripts/create-axagent-skill.mjs project-manager");
  console.log("  node scripts/create-axagent-skill.mjs my-skill ~/.axagent/skills/");
  exit(0);
}

const skillName = args[0].toLowerCase().replace(/[^a-z0-9-]/g, "-");
const targetBase = args[1] ? resolve(args[1]) : resolve(process.cwd());
const targetDir = join(targetBase, skillName);

if (existsSync(targetDir)) {
  console.error(`❌ 目录已存在: ${targetDir}`);
  exit(1);
}

// ── 创建目录结构 ──
const dirs = [
  targetDir,
  join(targetDir, "assets"),
  join(targetDir, "dist"),
];
for (const dir of dirs) {
  mkdirSync(dir, { recursive: true });
}

// ── SKILL.md 模板 ──
const skillMd = `# ${skillName}

## 描述
简要描述这个技能的功能和用途。

## 触发条件
- 用户提到 XXX 时自动触发
- 可手动通过 /${skillName} 命令调用

## 指令

### 步骤 1: 理解用户需求
分析用户输入，确定具体要完成的任务。

### 步骤 2: 执行操作
按需使用以下工具完成任务：
- \`file_read\` / \`file_write\` — 文件读写
- \`bash\` — 执行命令
- \`web_fetch\` — 获取网络资源
- \`web_search\` — 搜索信息
- \`grep\` / \`glob\` — 代码搜索

### 步骤 3: 输出结果
将结果以清晰的结构化格式返回给用户。

## 示例
\`\`\`
用户: 帮我做 XXX
Agent: 调用 ${skillName} skill...
\`\`\`
`;

// ── manifest.json 模板 ──
const manifest = {
  name: skillName,
  version: "0.1.0",
  description: "",
  author: "",
  icon: "lucide:Puzzle",
  permissions: {
    tools: ["file_read", "bash"],
    network: [],
    commands: [],
    events: [],
  },
  dependencies: {},
  frontend: {
    navigation: [],
    pages: [],
    commands: [],
    panels: [],
    settingsSections: [],
    toolbar: [],
    chatCommand: [],
    statusBar: [],
  },
  handlers: {},
  lifecycle: {},
};

// ── 示例命令（可选）─
manifest.frontend.commands.push({
  id: "${skillName}-hello",
  label: "显示 Hello",
  category: skillName,
  icon: "lucide:MessageSquare",
  actions: [
    {
      mode: "declarative",
      action: { type: "emit", event: "skill:hello", payload: { message: "Hello from ${skillName}!" } },
    },
  ],
});

// ── 写入文件 ──
writeFileSync(join(targetDir, "SKILL.md"), skillMd, "utf-8");
writeFileSync(join(targetDir, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n", "utf-8");

// ── dist/.gitkeep ──
writeFileSync(join(targetDir, "dist", ".gitkeep"), "", "utf-8");

console.log(`✅ Skill 项目已创建: ${targetDir}`);
console.log("");
console.log("目录结构:");
console.log(`  ${skillName}/`);
console.log("  ├── SKILL.md          # Agent 指令（核心）");
console.log("  ├── manifest.json     # 清单（前端扩展 + 权限 + handlers）");
console.log("  ├── assets/           # 静态资源（图标、HTML 页面）");
console.log("  └── dist/             # 构建产物（React/WebComponent JS）");
console.log("");
console.log("下一步:");
console.log(`  1. 编辑 ${skillName}/SKILL.md 编写 Agent 指令`);
console.log(`  2. 编辑 ${skillName}/manifest.json 配置前端扩展`);
console.log(`  3. 放入静态资源到 ${skillName}/assets/`);
console.log(`  4. 运行验证: node scripts/validate-skill.mjs ${skillName}`);
