[**English**](./README-EN.md) | **简体中文** | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp&amp&utm_source=badge-featured&amp&amp;&amp;#10;&amp;amp&amp&amp;;utm_medium=badge&amp&amp;#10&amp&amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>跨平台 AI 桌面客户端 | 多智能体协作 | 本地优先</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## 什么是 AxAgent？

AxAgent 是一款功能全面的跨平台 AI 桌面应用，集成了先进的 AI 智能体能力和丰富的开发者工具。它支持多模型提供商、自主管道执行、可视化工作流编排、本地知识管理以及内置 API 网关。

---

## 截图预览

| 对话与模型选择 | 多智能体仪表盘 |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| 知识库 RAG | 记忆与上下文 |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| 工作流编辑器 | API 网关 |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## 核心功能

### 🤖 AI 模型支持

- **多提供商支持** — 原生集成 OpenAI、Anthropic Claude、Google Gemini、Ollama、OpenClaw、Hermes 及所有 OpenAI 兼容 API
- **多 Key 轮换** — 为每个提供商配置多个 API Key，自动轮换分发限流
- **本地模型支持** — 完整支持 Ollama 本地模型，包含 GGUF/GGML 文件管理
- **模型管理** — 远程模型列表获取，可自定义参数（temperature、max tokens、top-p 等）
- **流式输出** — 实时逐 token 渲染，支持可折叠的思考块（Claude 扩展思考）
- **多模型对比** — 同时向多个模型提问， side-by-side 对比结果
- **函数调用** — 跨所有支持提供商的结构化函数调用

### 🔐 AI 智能体系统

智能体系统基于精密架构构建，具备以下特性：

- **ReAct 推理引擎** — 融合推理与行动，内置自验证确保任务执行可靠
- **层级规划器** — 将复杂任务分解为具有阶段和依赖关系的结构化计划
- **工具注册表** — 动态工具注册，支持语义版本控制和冲突检测
- **深度研究** — 多源搜索编排、引用追踪与可信度评估
- **事实核查** — AI 驱动的事实验证与来源分类
- **计算机控制** — AI 控制的鼠标点击、键盘输入、屏幕滚动，配合视觉模型分析
- **屏幕感知** — 截图捕获和视觉模型分析，用于 UI 元素识别
- **三级权限模式** — 默认（需要审批）、接受编辑（自动批准）、完全访问（无提示）
- **沙箱隔离** — 智能体操作严格限制在指定工作目录内
- **工具审批面板** — 实时显示工具调用请求，支持逐条审批
- **成本追踪** — 实时显示每个会话的 token 使用量和成本统计
- **暂停/恢复** — 随时暂停智能体执行，稍后恢复
- **检查点系统** — 持久化检查点用于崩溃恢复和会话重连
- **错误恢复引擎** — 自动错误分类和恢复策略执行

### 👥 多智能体协作

- **子智能体协调** — 主从架构，支持多个协作智能体
- **并行执行** — 多个智能体并行处理任务，支持依赖感知调度
- **对抗性辩论** — Pro/Con 辩论轮次，支持论点强度评分和反驳追踪
- **智能体角色** — 预定义角色（研究员、规划师、开发者、评审员、综合员）用于团队协作
- **智能体编排器** — 多智能体团队的中心化消息路由和状态管理
- **通信图谱** — 智能体交互和消息流的可视化展示

### ⭐ 技能系统

- **技能市场** — 内置市场，浏览和安装社区贡献的技能
- **技能创建** — 从提案自动创建技能，支持 Markdown 编辑器
- **技能进化** — 基于执行反馈的 AI 驱动的现有技能自动分析和改进
- **技能匹配** — 语义匹配，推荐与对话上下文相关的技能
- **原子技能** — 可组合成复杂工作流的细粒度技能组件
- **技能分解** — 自动将复杂任务分解为可执行的原子技能
- **生成工具** — AI 自动生成并注册新工具，扩展智能体能力
- **技能中心** — 集中的技能发现和配置管理界面
- **技能中心客户端** — 与远程技能中心集成，支持社区分享

### 🔄 工作流系统

工作流引擎实现了基于 DAG 的任务编排系统：

- **可视化工作流编辑器** — 拖放式工作流设计器，支持节点连接和配置
- **丰富节点类型** — 14 种节点类型：触发器、智能体、LLM、条件、并行、循环、合并、延迟、工具、代码、原子技能、向量检索、文档解析、验证
- **工作流模板** — 内置预设：代码审查、Bug 修复、文档、测试、重构、探索、性能、安全、功能开发
- **DAG 执行** — Kahn 算法拓扑排序，支持循环检测
- **并行调度** — 流水线式执行，快速步骤不等慢速步骤
- **重试策略** — 指数退避，每步可配置最大重试次数
- **部分完成** — 失败的步骤不会阻塞独立的下游步骤
- **版本管理** — 工作流模板版本控制，支持回滚
- **执行历史** — 详细记录，支持状态追踪和调试
- **AI 辅助** — AI 辅助工作流设计和优化

### 📚 知识与记忆

- **知识库（RAG）** — 多知识库支持，支持文档上传、自动解析、分块和向量索引
- **混合搜索** — 结合向量相似度搜索与 BM25 全文排名
- **重排序** — Cross-encoder 重排序，提升检索精度
- **知识图谱** — 知识关联的实体关系可视化
- **Wiki 系统** — LLM Wiki 编译器与验证器，支持知识图谱可视化与增量同步
- **记忆系统** — 多命名空间记忆，支持手动录入或 AI 自动提取
- **闭环记忆** — 集成 Honcho 和 Mem0 持久化记忆提供商
- **FTS5 全文搜索** — 跨对话、文件、记忆的快速检索
- **会话搜索** — 跨所有对话会话的高级搜索
- **上下文管理** — 灵活附加文件、搜索结果、知识片段、记忆、工具输出

### 🌐 API 网关

- **本地 API 服务器** — 内置 OpenAI 兼容、Claude 和 Gemini 接口服务器
- **外部链接** — 一键集成 Claude CLI、OpenCode，自动同步 API Key
- **Key 管理** — 生成、撤销、启用/禁用访问 Key，支持描述
- **用量分析** — 按 Key、提供商、日期的请求量和 token 使用量
- **SSL/TLS 支持** — 内置自签名证书，支持自定义证书
- **请求日志** — 完整记录所有 API 请求和响应
- **配置模板** — Claude、Codex、OpenCode、Gemini 的预建模板
- **实时 API** — 兼容 OpenAI 实时 API 的 WebSocket 事件推送
- **平台集成** — 支持钉钉、飞书、QQ、Slack、微信、WhatsApp、Telegram、Discord

### 🔧 工具与扩展

- **MCP 协议** — 完整的模型上下文协议实现，支持 stdio 和 HTTP/WebSocket 传输
- **OAuth 认证** — MCP 服务器的 OAuth 流程支持
- **插件系统** — 内置/捆绑/外部三级插件架构，支持工具注册、钩子与生命周期管理
- **内置工具** — 全面的文件操作、代码执行、搜索等工具集
- **LSP 客户端** — 内置语言服务器协议，支持代码补全和诊断
- **代码引擎** — 轻量级代码搜索运行时，AST 索引 + 三级召回管道
- **终端后端** — 支持本地、Docker 和 SSH 终端连接
- **浏览器自动化** — 通过 CDP 集成浏览器控制能力
- **UI 自动化** — 跨平台 UI 元素识别和控制
- **Git 工具** — Git 操作，支持分支检测和冲突感知
- **工具推荐** — 基于上下文的智能工具推荐引擎

### 📊 内容渲染

- **Markdown 渲染** — 完整支持代码高亮、LaTeX 数学公式、表格、任务列表
- **Monaco 代码编辑器** — 内置编辑器，支持语法高亮、复制、差异预览
- **图表渲染** — Mermaid 流程图、D2 架构图、ECharts 交互式图表
- **产物面板** — 代码片段、HTML 草稿、React 组件、Markdown 笔记，支持实时预览
- **三种预览模式** — 代码（编辑器）、分屏（并排）、预览（仅渲染）
- **会话检查器** — 会话结构的树形视图，快速导航
- **引用面板** — 追踪和显示来源引用，支持可信度评分

### 🛡️ 数据与安全

- **AES-256 加密** — API Key 和敏感数据使用 AES-256-GCM 加密
- **隔离存储** — 应用状态存储在 `~/.axagent/`，用户文件存储在 `~/Documents/axagent/`
- **自动备份** — 计划备份到本地目录或 WebDAV 存储
- **备份恢复** — 一键从历史备份恢复
- **导出选项** — PNG 截图、Markdown、纯文本、JSON 格式
- **存储管理** — 可视化磁盘使用显示和清理工具

### 🖥️ 桌面体验

- **主题引擎** — 深色/浅色主题，支持跟随系统或手动偏好
- **界面语言** — 11 种语言：简体中文、繁体中文、英语、日语、韩语、法语、德语、西班牙语、俄语、印地语、阿拉伯语
- **系统托盘** — 最小化到托盘，不中断后台服务
- **置顶窗口** — 窗口置顶于其他窗口之上
- **全局快捷键** — 可自定义快捷键调出主窗口
- **开机自启** — 可选在系统启动时运行
- **代理支持** — HTTP 和 SOCKS5 代理配置
- **自动更新** — 自动检查版本，有更新时提示
- **命令面板** — `Cmd/Ctrl+K` 快速访问命令

### 🔬 高级功能

- **深度研究** — 多源搜索、引用追踪、可信度评估与内容综合
- **事实核查** — AI 驱动的事实验证与来源分类
- **Cron 调度器** — 自动化任务调度，支持每日/每周/每月模板和自定义 cron 表达式
- **Webhook 系统** — 事件订阅，支持工具完成、智能体错误、会话结束通知
- **用户画像** — 自动学习代码风格、命名规范、缩进、注释风格、沟通偏好
- **RL 优化器** — 强化学习优化工具选择和任务策略
- **LoRA 微调** — 使用 LoRA 进行本地训练的自定义模型适配
- **主动建议** — 基于对话内容和用户模式的上下文感知提示
- **梦境整合** — 后台自动整合记忆与模式，优化长期知识
- **思维链** — 智能体决策推理的可视化，逐步分解
- **错误恢复** — 自动错误分类、根因分析和恢复建议
- **开发者工具** — Trace、Span、时间线可视化，用于调试和性能分析
- **基准测试系统** — SWE-bench / Terminal-bench 任务性能评估和指标，带评分卡
- **风格迁移** — 将学习的代码风格偏好应用到生成的代码
- **仪表盘插件** — 可扩展的仪表盘，支持自定义面板和小组件
- **协作共享** — CRDT 实时协作与一键会话分享
- **浏览器扩展** — Chrome 扩展，快速与 AxAgent 交互

---

## 技术架构

### 技术栈

| 层级 | 技术 |
|------|------|
| **框架** | Tauri 2 + React 19 + TypeScript |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **状态管理** | Zustand 5 |
| **国际化** | i18next + react-i18next |
| **后端** | Rust + SeaORM + SQLite |
| **向量数据库** | sqlite-vec |
| **代码编辑器** | Monaco Editor |
| **图表** | Mermaid + D2 + ECharts（CDN） |
| **终端** | xterm.js |
| **构建** | Vite + npm |

### Rust 后端架构

后端组织为 Rust workspace，包含专业化的 crates：

```
src-tauri/crates/
├── agent/         # AI 智能体核心
│   ├── react_engine.rs          # ReAct 推理引擎
│   ├── tool_registry.rs         # 动态工具注册
│   ├── coordinator.rs           # 智能体协调
│   ├── hierarchical_planner.rs  # 任务分解
│   ├── self_verifier.rs         # 输出验证
│   ├── error_recovery_engine.rs # 错误处理
│   ├── vision_pipeline.rs       # 屏幕感知
│   ├── deep_research.rs         # 深度研究
│   ├── fact_checker.rs          # 事实核查
│   ├── research_agent.rs        # 研究智能体
│   ├── local_tool_registry.rs   # 本地工具注册
│   ├── evaluator/               # 基准测试评估
│   ├── fine_tune/               # LoRA 微调
│   ├── rl_optimizer/            # RL 策略优化
│   └── tool_recommender/        # 工具推荐引擎
│
├── code_engine/   # 代码引擎
│   └── lib.rs                  # 轻量代码搜索运行时（AST + 三级召回）
│
├── core/          # 核心工具
│   ├── db.rs                   # SeaORM 数据库
│   ├── vector_store.rs         # sqlite-vec 集成
│   ├── rag.rs                  # RAG 抽象层
│   ├── hybrid_search.rs        # 向量 + FTS5 搜索
│   ├── crypto.rs               # AES-256 加密
│   ├── mcp_client.rs           # MCP 协议客户端
│   ├── browser_automation.rs   # 浏览器自动化
│   ├── incremental_indexer.rs  # 增量索引
│   ├── marketplace_service.rs  # 市场服务
│   └── storage_migration.rs    # 存储迁移
│
├── gateway/       # API 网关
│   ├── server.rs               # HTTP 服务器
│   ├── handlers.rs              # API 处理器
│   ├── auth.rs                 # 认证
│   ├── marketplace_handlers.rs  # 市场接口
│   └── realtime.rs             # WebSocket 支持
│
├── plugins/       # 插件系统
│   ├── hooks.rs                # 钩子运行器
│   └── lib.rs                  # 插件注册表与生命周期
│
├── providers/     # 模型适配器
│   ├── openai.rs              # OpenAI API
│   ├── anthropic.rs           # Claude API
│   ├── gemini.rs              # Gemini API
│   ├── ollama.rs              # Ollama 本地
│   ├── openclaw.rs            # OpenClaw
│   └── hermes.rs              # Hermes
│
├── runtime/       # 运行时服务
│   ├── session.rs             # 会话管理
│   ├── workflow_engine.rs     # DAG 编排
│   ├── work_engine/           # 工作引擎（节点执行器）
│   ├── mcp.rs                 # MCP 服务器
│   ├── cron/                  # 任务调度
│   ├── terminal/              # 终端后端（本地/Docker/SSH）
│   ├── benchmarks/            # SWE-bench / Terminal-bench
│   ├── collaboration/         # CRDT 协作与会话共享
│   ├── tool_generator/        # AI 工具生成
│   ├── message_gateway/       # 平台集成（钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord）
│   ├── adversarial_debate.rs  # 对抗性辩论
│   ├── agent_orchestrator.rs  # 多智能体编排
│   ├── webhook_dispatcher.rs  # Webhook 分发
│   ├── session_search.rs      # 会话搜索
│   └── dashboard_plugin.rs    # 仪表盘插件
│
├── telemetry/     # 遥测与追踪
│   ├── tracer.rs              # 分布式追踪
│   ├── metrics.rs             # 指标收集
│   └── span.rs                # Span 管理
│
└── trajectory/    # 学习系统
    ├── memory.rs              # 记忆管理
    ├── skill.rs               # 技能系统
    ├── rl.rs                  # RL 奖励信号
    ├── behavior_learner.rs    # 模式学习
    ├── user_profile.rs        # 用户画像
    ├── auto_memory.rs         # 自动记忆提取
    ├── dream_consolidation.rs # 梦境整合
    ├── parallel_execution.rs  # 并行执行服务
    ├── style_extractor.rs     # 风格提取
    ├── style_applier.rs       # 风格应用
    ├── suggestion_engine.rs   # 建议引擎
    ├── atomic_skill/          # 原子技能执行器
    ├── memory_providers/      # 记忆提供商（Honcho/Mem0/闭环）
    └── skill_decomposition/   # 技能分解（LLM 辅助/多轮）
```

### 前端架构

```
src/
├── stores/                    # Zustand 状态管理
│   ├── domain/               # 核心业务状态
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   └── workspaceStore.ts
│   ├── feature/               # 功能模块状态
│   │   ├── agentStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── workflowEditorStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── atomicSkillStore.ts
│   │   ├── expertStore.ts
│   │   ├── memoryStore.ts
│   │   ├── skillStore.ts
│   │   ├── workEngineStore.ts
│   │   └── ...（30+ 功能模块）
│   ├── devtools/              # 开发者工具状态
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   └── fineTuneStore.ts
│   └── shared/                # 共享状态
│
├── components/
│   ├── chat/                # 对话界面（80+ 组件）
│   ├── workflow/            # 工作流编辑器
│   ├── gateway/             # API 网关 UI
│   ├── settings/            # 设置面板
│   ├── terminal/            # 终端 UI
│   ├── atomicSkill/         # 原子技能编辑器
│   ├── benchmark/           # 基准测试面板
│   ├── decomposition/       # 技能分解与工具生成
│   ├── files/               # 文件管理页面
│   ├── fine-tune/           # LoRA 微调配置
│   ├── link/                # 外部链接管理
│   ├── llm-wiki/            # LLM Wiki 编辑器
│   ├── proactive/           # 主动建议系统
│   ├── recommendation/      # 工具推荐面板
│   ├── wiki/                # Wiki 管理
│   ├── workEngine/          # 工作引擎控制
│   ├── devtools/            # Trace/Span 时间线
│   ├── style/               # 代码风格迁移
│   ├── rl/                  # RL 训练监控
│   └── shared/              # 共享组件
│
└── pages/                    # 页面组件
    ├── ChatPage.tsx
    ├── KnowledgePage.tsx
    ├── MemoryPage.tsx
    ├── WorkflowPage.tsx
    ├── WorkflowMarketplace.tsx
    ├── GatewayPage.tsx
    ├── LinkPage.tsx
    ├── FilesPage.tsx
    ├── FineTunePage.tsx
    ├── SkillsPage.tsx
    ├── WikiPage.tsx
    ├── LlmWikiPage.tsx
    ├── PromptTemplatesPage.tsx
    ├── IngestPage.tsx
    └── SettingsPage.tsx
```

### 平台支持

| 平台 | 架构 |
|------|------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

## 快速开始

### 下载预构建版本

访问 [Releases](https://github.com/polite0803/AxAgent/releases) 页面，下载适合您平台的安装程序。

### 从源码构建

#### 环境要求

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + Rust MSVC targets

#### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 仅构建前端
npm run build

# 构建桌面应用
npm run tauri build
```

构建产物位于 `src-tauri/target/release/`。

### 测试

```bash
# 单元测试
npm run test

# E2E 测试
npm run test:e2e

# 类型检查
npm run typecheck
```

---

## 项目结构

```
AxAgent/
├── src/                         # 前端源码 (React + TypeScript)
│   ├── components/              # React 组件
│   │   ├── chat/               # 对话界面（80+ 组件）
│   │   ├── workflow/           # 工作流编辑器组件
│   │   ├── gateway/            # API 网关组件
│   │   ├── settings/           # 设置面板
│   │   ├── terminal/          # 终端组件
│   │   ├── atomicSkill/       # 原子技能编辑器
│   │   ├── benchmark/         # 基准测试
│   │   ├── decomposition/     # 技能分解
│   │   ├── files/             # 文件管理
│   │   ├── fine-tune/         # LoRA 微调
│   │   ├── link/              # 外部链接
│   │   ├── llm-wiki/          # LLM Wiki
│   │   ├── proactive/         # 主动建议
│   │   ├── recommendation/    # 工具推荐
│   │   ├── wiki/              # Wiki 管理
│   │   ├── workEngine/        # 工作引擎
│   │   ├── devtools/          # 开发者工具
│   │   ├── style/             # 代码风格
│   │   ├── rl/                # RL 训练
│   │   └── shared/            # 共享组件
│   ├── pages/                   # 页面组件（15+ 页面）
│   ├── stores/                  # Zustand 状态管理
│   │   ├── domain/            # 核心业务状态
│   │   ├── feature/           # 功能模块状态（30+ store）
│   │   ├── devtools/          # 开发者工具状态
│   │   └── shared/            # 共享状态
│   ├── hooks/                   # React hooks（16 个）
│   ├── lib/                     # 工具函数（含 Web Worker）
│   ├── types/                   # TypeScript 类型定义
│   └── i18n/                    # 11 种语言翻译
│
├── src-tauri/                    # 后端源码 (Rust)
│   ├── crates/                  # Rust workspace（11 个 crates）
│   │   ├── agent/             # AI 智能体核心
│   │   ├── code_engine/       # 代码搜索引擎
│   │   ├── core/              # 数据库、加密、RAG
│   │   ├── gateway/           # API 网关服务器
│   │   ├── plugins/           # 插件系统
│   │   ├── providers/         # 模型提供商适配器
│   │   ├── runtime/           # 运行时服务
│   │   ├── trajectory/       # 记忆与学习
│   │   ├── telemetry/        # 追踪与指标
│   │   └── migration/        # 数据库迁移
│   └── src/                    # Tauri 入口点
│
├── extension/                  # 浏览器扩展（Chrome）
├── e2e/                        # Playwright E2E 测试
├── scripts/                    # 构建与工具脚本
└── website/                    # 项目网站
```

## 数据目录

```
~/.axagent/                      # 配置目录
├── axagent.db                   # SQLite 数据库
├── master.key                   # AES-256 主密钥
├── vector_db/                   # 向量数据库 (sqlite-vec)
└── ssl/                         # SSL 证书

~/Documents/axagent/            # 用户文件目录
├── images/                     # 图片附件
├── files/                      # 文件附件
└── backups/                    # 备份文件
```

---

## 常见问题

### macOS：提示「应用已损坏」或「无法验证开发者」

由于应用未经过 Apple 签名：

**1. 允许运行「任何来源」的应用**
```bash
sudo spctl --master-disable
```

然后前往 **系统设置 → 隐私与安全性 → 安全性**，选择 **任何来源**。

**2. 移除隔离属性**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. macOS Ventura+ 额外步骤**
前往 **系统设置 → 隐私与安全性**，点击 **仍要打开**。

---

## 社区

- [LinuxDO](https://linux.do)

## 开源协议

本项目基于 [AGPL-3.0](LICENSE) 协议开源。
