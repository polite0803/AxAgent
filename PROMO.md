# AxAgent ⚡ AI Native OS

> **不只是 AI 聊天。是你的个人 AI 操作系统。**

<!-- 纯文本 badge，无需外部图片 -->

`📦 v2.5.0` `🖥 Win / macOS / Linux / Android / iOS` `🦀 32 Crates`
`📄 56.3 万行` `📝 AGPL-3.0` `🌐 11 语言`

---

## 📊 数据一览

| 指标       | 数字                              | 指标            | 数字     |
| ---------- | --------------------------------- | --------------- | -------- |
| 代码行数   | **56.3 万**                       | Rust Crates     | **32**   |
| 内置工具   | **47+**                           | Provider 适配器 | **8**    |
| 消息平台   | **9**                             | 工作流节点类型  | **17**   |
| 前端组件   | **200+**                          | Zustand Stores  | **62**   |
| 智能体模块 | **80+**                           | i18n 语言       | **11**   |
| 支持平台   | **5** (Win/Mac/Linux/Android/iOS) | Trait 接口      | **200+** |

---

## 🚀 解决三大核心问题

### 1. 🎛️ 多模型统一调度

在单一界面中同时使用 OpenAI、Anthropic Claude、Google Gemini、Ollama 本地模型及任何 OpenAI 兼容 API。支持多 Key 轮换、智能模型路由、流式对比、提供商健康监控。

### 2. 🔧 AI 能力工具化

将 AI 从"对话"扩展到"执行"——通过 **47+ 内置工具**、**可视化 DAG 工作流**、**MCP 协议扩展**、**CDP 浏览器自动化**和**计算机控制**，让 AI 直接操作文件、运行代码、管理 Git、调度任务。

### 3. 🔒 本地优先的数据主权

AI 对话、知识库、记忆、配置文件均存储在本地 SQLite 中。API Key 使用 **AES-256-GCM** 加密。无需第三方云服务即可运行核心功能。

---

## 🧠 十大功能域

### 能力层级一：模型与推理

| 功能域               | 关键能力                                                             |
| -------------------- | -------------------------------------------------------------------- |
| **🤖 模型基础设施**  | 8 家 Provider 适配器 · 多 Key 轮换 · 智能路由 · 健康监控 · 自动降级  |
| **🧩 智能体推理**    | ReAct 引擎 · 层级规划 · 深度研究 · 思维树 · 反思 · 自验证 · 错误恢复 |
| **👥 多 Agent 协作** | 主从协调 · 共享黑板 · 对抗辩论 · Swarm 集群 · CRDT                   |
| **🛠️ 工具生态**       | MCP 协议 · 47+ 内置工具 · 沙箱隔离 · 自动注册                        |

### 能力层级二：学习与知识

| 功能域            | 关键能力                                                      |
| ----------------- | ------------------------------------------------------------- |
| **🧬 技能进化**   | 遗传算法演进 · AI 辅助创建 · 语义匹配 · 自动分解              |
| **🎯 自学习系统** | RL 优化器 · Dream 梦境整合 · 用户画像 · 风格迁移 · LoRA 微调  |
| **📚 知识 RAG**   | 三级召回 · 知识图谱 · 混合检索 · Self-RAG · 重排序 · 文件监听 |

### 能力层级三：连接与安全

| 功能域          | 关键能力                                                                 |
| --------------- | ------------------------------------------------------------------------ |
| **🌐 API 网关** | Axum HTTP/WS 服务器 · OpenAI 兼容端点 · Key 管理 · 用量追踪 · Prometheus |
| **💬 消息平台** | 钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord — 9 平台统一接入       |
| **🛡️ 安全防护**  | Prompt-Guard L1-L4 · AES-256-GCM · SSRF 防护 · 熔断器 · 沙箱隔离         |

### 能力层级四：开发者体验

| 功能域       | 关键能力                                                                                  |
| ------------ | ----------------------------------------------------------------------------------------- |
| **🔬 DevEx** | 分布式追踪 · 回放调试 · Trace 时间线 · Criterion 基准测试 · Monaco 编辑器 · xterm.js 终端 |

---

## 🏗️ 技术架构

### Harness 依赖倒置模式

```
                    ┌─────────────────────────┐
                    │    Runtime Wiring        │
                    │   (axagent-runtime)      │
                    │    依赖注入容器           │
                    └──────────┬──────────────┘
                               │ 通过 trait 注入实现
           ┌───────────────────┼───────────────────┐
           ▼                   ▼                   ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│   Consumer       │  │   Implementor    │  │   Hybrid         │
│   ───────────    │  │   ───────────    │  │   ───────────    │
│   agent          │  │   dao/storage    │  │   tools          │
│   gateway        │  │   mcp/crypto     │  │   rt-messaging   │
│   orchestrator   │  │   providers/     │  │   rt-workflow    │
│   runtime-core   │  │   search/...     │  │                  │
└────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘
         │                     │                     │
         └──────────┬──────────┴──────────┬──────────┘
                    ▼                     ▼
         ┌─────────────────────┐  ┌─────────────────────┐
         │  axagent-harness    │  │  axagent-entities   │
         │  200+ trait 接口    │  │  SeaORM 数据模型    │
         │  纯 DTO + 契约      │  │  仅依赖 harness     │
         └─────────────────────┘  └─────────────────────┘
```

### 32 个 Crate 分层

```
src-tauri/crates/
├── foundation/          harness · entities · disk-cache · rt-dashboard · rt-theme
├── implementor/         dao · storage · migration · crypto · credential
│                        cache · search · document-parser · kit · mcp
│                        providers · trajectory · plugins · telemetry
│                        prompt-guard · npm · schema-gen
├── hybrid/              tools · rt-messaging · rt-workflow · rt-webhook
├── consumer/            agent · orchestrator · runtime-core · gateway
└── wiring/              runtime · src/commands · src/init
```

### 前端架构

```
React 19 + TypeScript 6 + Ant Design 6 + TailwindCSS 4
├── 22 个页面          Chat / Workflow / Gateway / Knowledge / Memory /
│                       Skills / Settings / Dashboard / Terminal / Files / ...
├── 24 个组件模块      200+ 组件
├── 62 个 Zustand Store   domain / feature / shared / devtools
├── 11 种语言           zh-CN / zh-TW / en-US / ja / ko / fr / de / es / ru / hi / ar
└── 技术栈              Vite 8 · ReactFlow 12 · Monaco · xterm.js · D2 · Mermaid
```

---

## 🌊 AI 浪潮中的站位

> 2026 年 AI 行业趋势：**本地化 · Agent化 · 开源化**

AxAgent 在这三个方向上都做了深度押注：

| 行业趋势                   | AxAgent 的站位                                                                                          |
| -------------------------- | ------------------------------------------------------------------------------------------------------- |
| 🔐 **数据主权 & 本地运行** | 所有核心能力本地运行，不依赖云服务。Ollama 集成支持完全离线推理                                         |
| 🤖 **Agent 化是下一阶段**  | 80+ 智能体模块 + ReAct 引擎 + 层级规划 + 深度研究 + 多 Agent 协作 — 不是轻量级封装，是生产级 Agent 框架 |
| 🧩 **MCP 协议标准化**      | 完整 MCP 实现（stdio / HTTP / WebSocket / OAuth），与新兴工具生态对齐                                   |
| 🌍 **开源生态驱动**        | AGPL-3.0 开源，32 个 Rust crate 独立可复用，社区插件体系                                                |
| 🏢 **成本效率**            | Token 价格持续下降 + 本地模型成熟 + 智能路由 = 用最低成本做最多的事                                     |

---

## ⚡ Top 5 亮点

1. **🥇 架构独特性** — Harness 依赖倒置模式，200+ trait 解耦，32 个 crate 零循环依赖，Rust 工程范本
2. **🥇 能力广度** — 从 ReAct 到 RL，从 RAG 到 MCP，从 API 网关到消息平台，一个客户端拥有全部
3. **🥇 自学习系统** — RL 优化器 + 梦境整合 + 用户画像 + 风格迁移，AI 越用越懂你
4. **🥇 安全深度** — Prompt-Guard L1-L4 四级防护 + AES-256-GCM 加密 + SSRF 防护 + 沙箱隔离
5. **🥇 跨平台覆盖** — Windows / macOS / Linux / Android / iOS，一套代码 5 平台

---

## 🚀 快速开始

```bash
# 1. 下载 Release
# Windows / macOS / Linux → GitHub Releases
https://github.com/polite0803/AxAgent/releases

# 2. 或从源码构建
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev

# 3. 配置 API Key → 开始对话
```

**系统要求**：Node.js 22+ · Rust 2024 Edition · Tauri 2.11

> 浏览器模式（无需 Tauri）：`npm run dev`，走 localStorage mock 全功能可用

---

## 📦 生态系统

```
AxAgent (AI OS 基座)
├── 🏦 AxInvest        — 个人投资追踪（开发中）
├── 🏢 AxOPC           — 一人公司管理（规划中）
├── 🎲 AxSim           — 模拟推演（展望中）
│
├── 🧩 AxHub / OpenClaw — AI 插件市场（46 插件）
└── 🔀 CCSwitch        — Claude/DeepSeek 代理路由
```

---

## 📜 许可

AGPL-3.0 — 开放源代码，欢迎贡献。

<p align="center">
<a href="https://github.com/polite0803/AxAgent">⭐ Star on GitHub — polite0803/AxAgent</a>
</p>

---

_Generated with ⚡ · AxAgent v2.5.0 · 2026-07-10_
