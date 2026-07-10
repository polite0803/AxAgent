# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent Poster" width="80%" />
  </a>
</p>

**AxAgent** 是一款开源的跨平台 AI 助手桌面客户端，支持 **Windows / macOS / Linux / Android / iOS** 五大平台。它不只是聊天界面——集成了 ReAct 智能体引擎、可视化工作流编排、本地 RAG 知识库、MCP 协议扩展、多模型统一网关、浏览器自动化、计算机控制等能力，可作为日常开发、研究、知识管理和自动化工作的 AI 工作台。

> **语言版本**: [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 项目定位

AxAgent 解决了三个核心问题：

1. **多模型统一调度**: 在单一界面中同时使用 OpenAI、Anthropic Claude、Google Gemini、Ollama 本地模型及任何 OpenAI 兼容 API，支持多 Key 轮换、智能模型路由、流式对比
2. **AI 能力工具化**: 将 AI 从"对话"扩展到"执行"——通过 47+ 内置工具、可视化工作流、MCP 扩展、浏览器自动化和计算机控制，让 AI 直接操作文件、运行代码、管理 Git、调度任务
3. **本地优先的数据主权**: AI 对话、知识库、记忆、配置文件均存储在本地 SQLite 数据库中，API Key 使用 AES-256-GCM 加密，无需第三方云服务即可运行核心功能

---

## 核心能力

### 多模型引擎

- **9 种提供商适配器**: OpenAI (Chat Completions + Responses + Realtime)、Anthropic Claude、Google Gemini、Ollama (含 GGUF 管理)、OpenClaw、Hermes、以及所有 OpenAI 兼容 API
- **多 Key 轮换**: 为同一提供商配置多个 API Key，按配额自动轮换，避免单 Key 限流中断
- **智能路由**: 按任务类型（代码审查 / 摘要 / 翻译 / 通用）自动选择最合适的模型，支持自定义路由规则
- **提供商健康监控**: 实时追踪各提供商的成功率、延迟和可用状态，支持分层级自动降级（ProviderTier）
- **AI 图像生成**: DALL-E 3 和 Flux (Replicate) 多尺寸预设
- **实时语音**: 基于 OpenAI Realtime API 的 WebSocket 语音对话，支持打断和流式转写

### 智能体系统

整个智能体系统构建在 **ReAct (Reasoning + Acting) 引擎** 之上，包含以下实际实现的子系统：

- **层级规划器** (`hierarchical_planner`): 将复杂任务分解为带依赖关系的 Phase → Task 结构化计划，编译为 DAG 拓扑执行
- **深度研究** (`deep_research`): 多源搜索编排，包含搜索计划（`search_planner`）、搜索执行（`search_orchestrator`）、内容综合（`content_synthesizer`）、引用追踪（`citation_tracker`）
- **事实核查** (`fact_checker`): AI 驱动的事实验证，包含来源分类器（`source_classifier`）、来源验证器（`source_validator`）、可信度评估（`credibility_evaluator`）
- **思维树** (`tree_of_thoughts`): 多路径推理探索，分支评估与回溯
- **反思器** (`reflector`): 任务执行后的自我评估与改进建议生成
- **自验证** (`self_verifier`): 推理结果的自动校验，循环检测（`cycle_detector`）避免无限推理
- **错误恢复** (`error_recovery_engine`): 分类错误类型 → 选择恢复策略 → 自动重试或调整计划，支持指数退避
- **A/B 测试** (`ab_testing`): 不同推理策略的对比评估
- **评估系统** (`evaluator`): 内置基准测试框架，支持数据集、指标、报告生成
- **LoRA 微调** (`fine_tune`): 内置训练流水线，支持 LoRA 适配器管理
- **RL 优化器** (`rl_optimizer`): 基于经验反馈的策略强化学习，包含经验回放、策略梯度
- **工具推荐** (`tool_recommender`): 基于上下文的工具使用模式分析和推荐

**多智能体协作**:

- 主从协调架构（`coordinator`），子智能体并行执行，依赖感知调度
- 共享黑板（`shared_blackboard`）用于智能体间信息交换
- 对抗性辩论模式，Pro/Con 轮次与论点强度评分
- Swarm 集群模式，多进程智能体集群支持权限同步与自动重连
- 主动模式（`proactive_mode`）: 智能体可主动发起建议和操作

**计算机控制**: AI 驱动的鼠标点击、键盘输入、屏幕滚动，三级权限（默认 / 接受编辑 / 完全访问），沙箱路径隔离

**浏览器自动化**: 通过 CDP 协议控制浏览器，支持导航、截图、点击、表单填写、文本提取、页面状态监控

### 技能系统

- **技能市场**: 浏览和安装社区技能
- **AI 辅助创建**: 从自然语言提案自动创建技能结构
- **技能进化** (`evolution_engine`): 基于执行反馈自动分析并改进技能
- **语义匹配** (`skill`): 根据对话上下文语义匹配相关技能，自动推荐
- **技能分解** (`skill_decomposition`): 将复杂任务自动分解为原子技能组合
- **生成工具** (`generated_tool`): AI 生成并注册新工具
- **沙箱执行** (`sandbox`): 技能在隔离的沙箱环境中安全执行

### 可视化工作流

基于 ReactFlow 12 的拖放式 DAG 工作流编辑器：

- **17 种节点类型**: 触发器、智能体、LLM 调用、条件分支、并行分叉、循环、合并、延迟、工具调用、代码执行、子工作流、向量检索、文档解析、验证、结束、业务规则、Agent 角色
- **Kahn 拓扑排序执行**: 自动检测循环依赖，并行流水线调度
- **内置模板**: 代码审查、Bug 修复、文档生成、测试、重构、探索、性能分析、安全审查、功能开发
- **YAML 序列化**: 工作流定义支持 YAML 格式导入导出
- **版本管理**: 工作流模板版本控制
- **AI 辅助**: AI 辅助工作流设计和节点推荐

### 知识管理

- **多知识库 RAG**: 文档上传 → 自动解析（PDF/DOCX/XLSX/PPTX/TXT）→ 分块 → 向量索引
- **混合检索**: 向量相似度（sqlite-vec + candle 本地嵌入）+ BM25 全文检索（FTS5），混合排序
- **Self-RAG**: 自检索增强生成，检索结果自动反思和验证
- **重排序**: Cross-encoder 结果重排序提升精度
- **知识图谱**: 实体提取（`EntityExtractor`）→ 关系构建 → 可视化图谱
- **文件监听**: 基于 `notify` 的实时文件变更监听，自动增量索引
- **LLM Wiki**: AI 辅助的 Wiki 编译器与验证器，支持 Wiki 裁剪浏览器扩展

### 记忆系统

- **多命名空间记忆**: 按项目/主题隔离，支持手动录入与 AI 自动提取
- **持久化集成**: Honcho 和 Mem0 闭环记忆
- **用户画像** (`user_profile` / `profile`): 自动学习代码风格（缩进/命名/注释）、技术栈偏好、沟通风格
- **风格迁移** (`style`): 提取代码风格特征 → 应用到 AI 生成代码
- **梦境整合** (`dream`): 后台自动整合记忆碎片与行为模式，生成结构化知识
- **项目记忆** (`project_memory`): 按项目维度的上下文持久化

### API 网关

内置基于 `axum` 的 HTTP + WebSocket 网关服务器：

- **兼容端点**: OpenAI `/v1/chat/completions`、Claude Messages API、Gemini API，以及 OpenAI Responses 和 Realtime WebSocket
- **Key 管理**: 生成、撤销、启用/禁用访问密钥，支持过期时间设置
- **用量追踪**: 按 Key、提供商、日期的请求量和 token 消耗统计，Prometheus 指标导出
- **速率限制**: 基于 `governor` 的令牌桶算法，可配置的速率限制策略
- **SSL/TLS**: 内置自签名证书（`rcgen`），支持自定义证书
- **外部链接**: 一键集成 Claude CLI、OpenCode 等外部工具，自动同步 API Key
- **实时门票**: 基于 HMAC 的临时认证票据，用于 WebSocket 实时连接安全传递

### 消息平台集成

通过 `rt-messaging` crate 实现的消息平台网关，支持:

钉钉、飞书、QQ、Slack、微信、WhatsApp、Telegram、Discord

支持 Webhook 消息接收、命令解析、AI 回复自动回传。

### 工具系统

47 个内置工具，所有工具统一通过 `Tool` trait 注册：

| 类别       | 工具                                                                                                                                                                                                       |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 文件操作   | `file_read`, `file_write`, `file_edit`, `file_system` (列表/搜索/元数据)                                                                                                                                   |
| 代码执行   | `bash`, `repl`                                                                                                                                                                                             |
| 搜索       | `grep`, `glob`                                                                                                                                                                                             |
| 浏览器     | `browser` (CDP 控制)                                                                                                                                                                                       |
| 计算机控制 | `computer_use` (鼠标/键盘/截图)                                                                                                                                                                            |
| Web        | `web_search`, `web_fetch`                                                                                                                                                                                  |
| 知识库     | `knowledge`, `document` (文档解析)                                                                                                                                                                         |
| Git        | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| 开发工具   | `lsp` (语言服务器协议), `workspace`                                                                                                                                                                        |
| 任务管理   | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| 消息推送   | `push_notification`, `messaging`                                                                                                                                                                           |
| 数据库     | `database`                                                                                                                                                                                                 |
| 存储       | `storage`                                                                                                                                                                                                  |
| 其他       | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP 协议

基于 `rmcp` crate 的完整 MCP (Model Context Protocol) 实现：

- **传输层**: stdio 子进程 + Streamable HTTP + WebSocket
- **OAuth 认证**: 支持 MCP 服务器的 OAuth 授权流程
- **工具发现**: 自动发现和注册 MCP 服务器暴露的工具
- **MCP 管理器**: 服务器生命周期管理、健康检查、自动重连

### 插件系统

OpenClaw 兼容的三级插件架构（内置 / 捆绑 / 外部），支持：

- npm 包安装，内置市场 UI 支持搜索和安装
- 插件 manifest 定义、权限声明、沙箱隔离执行
- 自定义工具注册、Agent 提供者、Hook 拦截
- 技能安装器：从插件包中安装技能到技能系统

### 安全防护

- **AES-256-GCM 加密**: API Key 和敏感配置的本地加密存储（`crypto` crate）
- **提示词注入防护**: 四级防御管线（`prompt-guard`）——模式检测 → 分隔符转义 → XML 包装器 → 信任标签，集成到会话、提示词构建、Git、RAG 全链路
- **SSRF 防护** (`ssrf_guard`): URL 安全检查，阻止对内网地址的请求
- **内容过滤** (`content_filter`): 多类型内容安全过滤
- **速率限制** (`rate_limiter`): 工具调用和 API 请求的令牌桶限流
- **熔断器** (`circuit_breaker`): 连续失败自动熔断，保护系统稳定性
- **访问控制** (`tool_access`): 基于策略的工具访问权限控制
- **沙箱隔离**: 智能体和技能的执行环境隔离

### 开发者体验

- **分布式追踪** (`telemetry`): OpenTelemetry 集成，支持 Span/Trace 可视化
- **遥测** (`telemetry`): 结构化日志、运行时指标、性能事件采集
- **回放调试**: 智能体执行轨迹录制（`trajectory_recorder`）与回放
- **DevTools 面板**: 前端内置的 Trace/Span 时间线查看器
- **基准测试框架**: Criterion benchmarks (tool_exec / llm_call / search)，SWE-bench 和 Terminal-bench 评估

### 桌面与移动端体验

- **响应式布局**: CSS 断点自适应桌面 / 平板 / 手机（600px / 900px）
- **11 种语言**: 简体中文、繁体中文、英语、日语、韩语、法语、德语、西班牙语、俄语、印地语、阿拉伯语
- **主题引擎** (`rt-theme`): 深色/浅色主题，跟随系统或手动切换，Ant Design 6 深度定制
- **Monaco 编辑器**: 内置代码编辑器，支持语法高亮、差异预览、多语言
- **xterm.js 终端**: 内置终端模拟器，支持 WebLinks、Unicode 11、搜索
- **D2 / Mermaid / ECharts**: 架构图、流程图、交互图表渲染
- **会话分享**: 一键生成分享链接，可配置访问权限
- **系统托盘 + 全局快捷键 + 开机自启**: 无干扰后台运行
- **自动更新**: 自动检测 GitHub Releases 版本更新
- **代理支持**: HTTP 和 SOCKS5 代理配置
- **云工作空间**: S3 和 WebDAV 存储同步，冲突检测与双向同步

### 移动端

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- 移动端专属适配：安全区适配、底部导航栏、Drawer 导航

---

## 技术架构

### 技术栈

| 层级          | 技术                                     |
| ------------- | ---------------------------------------- |
| 桌面框架      | Tauri 2.11                               |
| 前端框架      | React 19 + TypeScript 6                  |
| UI 库         | Ant Design 6 + TailwindCSS 4             |
| 状态管理      | Zustand 5                                |
| 路由          | React Router 7                           |
| 代码编辑器    | Monaco Editor                            |
| 终端          | xterm.js 6                               |
| 工作流编辑器  | ReactFlow 12                             |
| 图表          | D2 + Mermaid + Recharts + ECharts        |
| 虚拟滚动      | @tanstack/react-virtual + react-virtuoso |
| 拖拽          | @dnd-kit                                 |
| Markdown 渲染 | markstream-react + stream-markdown       |
| 国际化        | i18next + react-i18next                  |
| 构建工具      | Vite 8                                   |
| 测试          | Vitest + Playwright + cargo-nextest      |
| 格式化        | dprint (TS/JSON/Markdown/TOML) + rustfmt |
| Lint          | ESLint + Oxlint + Clippy + cargo-deny    |

### 后端架构: Harness 依赖注入模式

后端采用 Rust workspace 架构，包含 **32 个 crate**，遵循 **Harness 架构模式**:

```
所有 crate 通过 axagent-harness 定义的 trait 接口解耦，
运行时由 axagent-runtime 装配和注入依赖。

依赖方向：具体实现 → harness ← 调用方
```

**harness** 是架构基石——零业务逻辑、零具体实现，仅包含 trait 定义、纯数据 DTO、常量和统一错误类型。它被所有其他 crate 依赖，自身不依赖任何其他 axagent-* crate。

```
src-tauri/crates/
├── harness/          # 架构基石 — trait 接口、DTO、统一错误类型、DI 契约
│                     #   200+ trait 定义涵盖: Agent/Provider/Tool/RAG/存储/
│                     #   MCP/插件/安全/可观测性/记忆/学习/浏览器/消息等
│
├── entities/         # SeaORM 实体模型
├── dao/              # 数据访问层（CRUD）
├── migration/        # 数据库迁移
│
├── crypto/           # AES-256-GCM 加解密与密钥管理
├── credential/       # 凭据安全存储（API Key 等）
├── storage/          # 文件存储抽象（本地 / S3 / WebDAV），支持 ZIP 读写
├── cache/            # 通用缓存层（内存）
├── disk-cache/       # 磁盘文件级缓存
├── search/           # 检索引擎（FTS5 + sqlite-vec + candle 嵌入）
├── document-parser/  # 文档文本提取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集 — 路径/编码/哈希/日期等
│
├── runtime-core/     # 运行时公共类型、配置常量
├── runtime/          # 运行时服务编排 — 装配全部 30+ crate，是 Harness DI 的运行时容器
│                     #   管理: 会话/终端/Webhook/限流/权限/SSRF/事件总线/状态
├── rt-workflow/      # 工作流引擎 — DAG 编排、节点执行器、YAML 序列化
├── rt-messaging/     # 消息平台网关 — 钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 服务器与事件分发
├── rt-dashboard/     # 仪表盘插件框架
├── rt-theme/         # 主题引擎 — 深色/浅色切换逻辑
│
├── agent/            # AI 智能体核心 — 80+ 模块
│                     #   ReAct引擎/层级规划/深度研究/事实核查/思维树/反思/
│                     #   自验证/错误恢复/RL优化/LoRA微调/评估/工具推荐/A/B测试/
│                     #   协调器/黑板/视觉管线/Web搜索/学术搜索/Wiki编译等
│
├── orchestrator/     # 智能体编排 — 多智能体调度、DAG 分解、动态子图执行
├── providers/        # 模型提供商适配器 — OpenAI/Anthropic/Gemini/Ollama/
│                     #   OpenClaw/Hermes/图像生成(DALL-E/Flux)/Realtime/Responses
├── tools/            # 工具体系 — Tool trait/注册表/编排/流式/沙箱/47+内置工具
├── gateway/          # API 网关 — axum HTTP/WS 服务器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 协议 — stdio + Streamable HTTP，基于 rmcp
├── trajectory/       # 学习系统 — 记忆/技能进化/用户画像/梦境整合
├── plugins/          # 插件系统 — OpenClaw 兼容、npm 包安装、市场
├── telemetry/        # 可观测性 — OpenTelemetry、结构化日志、运行时指标
├── prompt-guard/     # 提示词注入防护 — L1-L4 多级检测管线
├── npm/              # npm 注册表客户端
└── schema-gen/       # 数据库 Schema 生成工具
```

### 前端架构

```
src/
├── pages/            # 22 个页面
│   ├── ChatPage          # 对话主界面
│   ├── WorkflowPage      # 工作流编辑器
│   ├── GatewayPage       # API 网关管理
│   ├── KnowledgeHubPage  # 知识库管理
│   ├── MemoryPage        # 记忆管理
│   ├── SkillsPage        # 技能市场
│   ├── SettingsPage      # 设置面板
│   ├── DashboardPage     # 数据仪表盘
│   ├── TerminalPage      # 终端
│   ├── FilesPage         # 文件管理
│   ├── GatewayLinkPage   # 外部链接管理
│   ├── LinkPage          # 集成链接
│   ├── WikiEditorPage    # Wiki 编辑器
│   ├── WikiEditPage      # Wiki 编辑
│   ├── WikiGraphPage     # Wiki 知识图谱
│   ├── FineTunePage      # LoRA 微调
│   ├── PersonaPage       # 角色管理
│   ├── QuickBarPage      # 快捷栏
│   ├── IngestPage        # 文档摄入
│   ├── WorkflowMarketplace # 工作流市场
│   ├── DynamicUIManagerPage # 动态 UI 管理
│   └── DynamicPageViewer    # 动态页面查看
│
├── components/       # 24 个模块, 200+ 组件
│   ├── chat/         # 对话界面（消息流/输入/附件/工具调用/产物/思考块等）
│   ├── workflow/     # 工作流编辑器（节点/连线/面板/模板/AI辅助）
│   ├── gateway/      # API 网关管理界面
│   ├── settings/     # 设置面板（40+ 子组件）
│   ├── skill/        # 技能编辑器与渲染器
│   ├── benchmark/    # 基准测试面板
│   ├── decomposition/# 技能分解与工具生成
│   ├── devtools/     # Trace/Span 时间线
│   ├── layout/       # 布局（标题栏/侧边栏/命令面板）
│   └── ...
│
├── stores/           # 62 个 Zustand store
│   ├── domain/       # 核心业务状态
│   ├── feature/      # 功能模块状态（44 个）
│   └── devtools/     # 开发者工具状态
│
├── hooks/            # React Hooks
├── lib/              # 工具函数 + Web Workers
├── types/            # TypeScript 类型定义
├── sdk/              # 外部集成 SDK
└── i18n/             # 11 语言翻译 (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
```

---

## 数据目录

```
~/.axagent/                    # 应用配置
├── axagent.db                 # SQLite 主数据库 (SeaORM)
├── master.key                 # AES-256 主密钥
├── vector_db/                 # sqlite-vec 向量索引
└── ssl/                       # 自签名 SSL 证书

~/Documents/axagent/          # 用户文件
├── images/                   # 图片附件
├── files/                    # 文件附件
└── backups/                  # 自动备份
```

---

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+, edition 2024
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 构建

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 开发模式
npm run tauri build    # 生产构建
```

构建产物位于 `src-tauri/target/release/`。

### 测试

```bash
npm run test           # 前端单元测试 (Vitest watch)
npm run test:run       # 前端单元测试 (单次)
npm run test:e2e       # E2E 测试 (Playwright)

# Rust 后端测试
cd src-tauri && cargo nextest run
cd src-tauri && cargo test

# 类型检查 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format

# CI 全量检查
npm run ci:check
```

---

## 平台支持

| 平台    | 架构                                    |
| ------- | --------------------------------------- |
| Windows | x86_64, ARM64                           |
| macOS   | Apple Silicon (arm64), Intel (x86_64)   |
| Linux   | x86_64, ARM64                           |
| Android | arm64-v8a, armeabi-v7a, x86_64 (模拟器) |
| iOS     | arm64                                   |

---

## 开源协议

本项目基于 [AGPL-3.0-only](LICENSE) 协议开源。

---

## 致谢

AxAgent 构建在众多优秀开源项目之上，包括但不限于:

- [Tauri](https://tauri.app/) — 跨平台桌面框架
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — 前端 UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — 向量检索
- [candle](https://github.com/huggingface/candle) — 本地嵌入推理
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — 可视化工作流编辑器
- [axum](https://github.com/tokio-rs/axum) — HTTP 框架
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — 代码编辑器
- [xterm.js](https://xtermjs.org/) — 终端模拟器
