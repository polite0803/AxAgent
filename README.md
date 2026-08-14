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

**AxAgent** 是一款基于 Tauri 2 的跨平台 AI 桌面客户端（Windows / macOS / Linux / Android / iOS），定位为 AI 驱动的日常开发、研究、知识管理与自动化工作台。它内置 ReAct 智能体引擎、认知路由（三级分层路由 + 检索增强路由 RAR）、可视化工作流编排、本地 RAG 知识库、MCP 协议扩展、多模型统一网关、浏览器自动化与计算机控制等能力，让 AI 从"对话"走向"执行"。

> **语言版本**: [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## 项目定位

AxAgent 解决三个核心问题：

1. **多模型统一接入与智能调度** — 单一界面同时使用 OpenAI、Anthropic Claude、Google Gemini、DeepSeek、Qwen、GLM、Kimi、文心、Ollama 本地模型及任意 OpenAI 兼容 API，支持多 Key 配额自动轮换、按任务类型智能路由、流式对比
2. **AI 从对话到执行的闭环** — 163+ 内置工具 + 可视化工作流 + MCP 扩展 + 浏览器/计算机控制，AI 可操作文件、运行代码、管理 Git、调度任务
3. **本地优先的数据主权** — 对话记录、知识库、记忆、配置均存储于本地 SQLite 数据库，API Key 使用 AES-256-GCM 加密，无需第三方云服务即可运行核心功能

---

## 核心能力

### 认知路由系统（Cognitive Router）

AxAgent 以 `cognitive_query` 作为所有对话的统一入口，通过**三级分层路由**将用户意图映射到具体能力：

- **L1 领域路由** (`domain_router`): 规则 + LLM 兜底，识别 9 大业务领域（数据分析 / 内容创作 / 沟通 / 运维 / AI 媒体 / 金融 / 自动化 / 通用等）
- **L2 集群路由** (`cluster_router`): 领域内定位能力集群（27 个集群，覆盖 8 大业务领域）
- **L3 能力路由**: **检索增强路由（RAR）** — 从能力向量库召回 Top-K 相似工作流注入 Prompt，结合工作流 DAG 图寻径，输出路径地址（如 `/finance/stock_analysis/tech`）与执行模式
- **执行模式**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`，按置信度自动选择
- **能力系统**: 统一注册表（`CapabilityRegistry`）+ 向量索引（`CapabilityIndexer`）+ 混合检索（`CapabilityRetriever`，向量 + BM25 + 标签硬匹配 + 负样本排除）
- **系统能力隔离**: 认知编排器与业务工作流物理隔离，系统能力带 `SYSTEM_ONLY` 可见性标记，路由层内置自引用熔断，防止自我指涉悖论
- **三级路由以工作流 DAG 实现**: 4 个预设路由工作流模板（主编排 ~20 节点 + L1/L2/L3 子路由），由 `rt-workflow` 引擎执行

### 多模型引擎

- **13 种提供商适配器**: OpenAI（Chat Completions + Responses + Realtime）、Anthropic Claude、Google Gemini、DeepSeek、Qwen、GLM、Kimi、文心一言、Ollama、Llama.cpp（GGUF 本地模型）、OpenClaw、Hermes，以及所有 OpenAI 兼容 API
- **多 Key 轮换**: 同一提供商多 API Key，按配额自动轮换，单 Key 限流自动切换
- **智能路由**: 按任务类型（代码审查 / 摘要 / 翻译 / 通用）自动选择最优模型，支持自定义规则
- **提供商健康监控**: 实时追踪成功率、延迟、可用状态，支持分级自动降级
- **AI 图像生成**: DALL-E 3 和 Flux 多尺寸预设
- **实时语音**: 基于 OpenAI Realtime API 的 WebSocket 语音对话，支持打断和流式转写

### 智能体系统（ReAct 引擎）

- **层级规划器** (`hierarchical_planner`): 复杂任务分解为 Phase → Task 结构化计划，编译为 DAG 拓扑执行
- **深度研究** (`deep_research`): 多源搜索编排，含搜索计划、搜索执行、内容综合、引用追踪
- **事实核查** (`fact_checker`): AI 驱动事实验证，含来源分类器、可信度评估
- **思维树** (`tree_of_thoughts`): 多路径推理探索，分支评估与回溯
- **反思器** (`reflector`): 任务执行后自我评估与改进建议
- **自验证** (`self_verifier`): 推理结果自动校验，含循环检测
- **错误恢复** (`error_recovery_engine`): 错误类型分类 → 恢复策略选择 → 自动重试或计划调整，支持指数退避
- **A/B 测试** (`ab_testing`): 不同推理策略的对比评估
- **评估系统** (`evaluator`): 内置基准测试框架
- **LoRA 微调** (`fine_tune`): 内置训练流水线，支持 LoRA 适配器管理
- **RL 优化器** (`rl_optimizer`): 基于经验反馈的策略强化学习

**多智能体协作**:

- 主从协调架构，子智能体并行执行，依赖感知调度
- 共享黑板用于智能体间信息交换
- 对抗性辩论模式（Pro/Con 轮次与论点强度评分）
- Swarm 集群模式，多进程智能体集群
- 主动模式：智能体可主动发起建议和操作

**计算机控制**: AI 驱动鼠标点击、键盘输入、屏幕滚动，三级权限（默认/接受编辑/完全访问），沙箱路径隔离

**浏览器自动化**: 通过 CDP 协议控制浏览器，支持导航、截图、点击、表单填写、文本提取

### 技能系统

- **技能市场**: 浏览和安装社区技能
- **AI 辅助创建**: 从自然语言提案自动创建技能结构 (`skill:create`)
- **技能进化** (`evolution_engine`): 基于执行反馈自动分析和改进技能
- **语义匹配**: 根据对话上下文语义自动推荐相关技能
- **技能分解** (`skill_decomposition`): 将复杂任务自动分解为原子技能组合
- **生成工具**: AI 生成并注册新工具
- **沙箱执行**: 技能在隔离沙箱中安全执行

### 可视化工作流

基于 ReactFlow 12 的拖放式 DAG 工作流编辑器：

- **32 种节点类型**: 触发器、智能体、LLM 调用、条件分支、并行分叉、循环、合并、延迟、工具调用、代码执行、子工作流、向量检索、文档解析、验证、结束、HTTP 请求、Switch、数据库查询、通知、审批、文件操作、数据转换、Webhook 发送、日志、LLM 分类器、聚合器、邮件、辩论、Swarm、多智能体、存储、业务规则
- **Kahn 拓扑排序执行**: 自动检测循环依赖，并行流水线调度
- **内置模板**: 代码审查、Bug 修复、文档生成、测试、重构、探索、性能分析、安全审查、功能开发
- **YAML 序列化**: 工作流定义导入导出
- **版本管理**: 模板版本控制
- **AI 辅助设计**: AI 辅助工作流设计、节点推荐与诊断

### 知识管理

- **多知识库 RAG**: 文档上传 → 自动解析（PDF/DOCX/XLSX/PPTX/TXT）→ 分块 → 向量索引
- **混合检索**: 向量相似度（sqlite-vec + candle 本地嵌入）+ BM25 全文检索（FTS5），混合排序
- **Self-RAG**: 检索结果自动反思和验证
- **重排序**: Cross-encoder 结果重排序
- **知识图谱**: 实体提取 → 关系构建 → 可视化图谱
- **文件监听**: 基于 `notify` 的实时文件变更监听，自动增量索引
- **LLM Wiki**: AI 辅助的 Wiki 编译器与验证器

### 记忆系统

- **多命名空间记忆**: 按项目/主题隔离，支持手动录入与 AI 自动提取
- **持久化集成**: Honcho 和 Mem0 闭环记忆
- **用户画像**: 自动学习代码风格、技术栈偏好、沟通风格
- **风格迁移**: 提取代码风格特征 → 应用到 AI 生成代码
- **梦境整合**: 后台自动整合记忆碎片与行为模式，生成结构化知识
- **项目记忆**: 按项目维度的上下文持久化

### API 网关

内置基于 `axum` 的 HTTP + WebSocket 网关：

- **兼容端点**: OpenAI `/v1/chat/completions`、Claude Messages API、Gemini API，以及 OpenAI Responses 和 Realtime WebSocket
- **Key 管理**: 生成、撤销、启用/禁用访问密钥，支持过期时间
- **用量追踪**: 按 Key/提供商/日期的请求量和 token 消耗统计，Prometheus 指标导出
- **速率限制**: 基于 `governor` 的令牌桶算法
- **SSL/TLS**: 内置自签名证书（`rcgen`），支持自定义证书
- **外部链接**: 一键集成 Claude CLI、OpenCode 等外部工具，自动同步 API Key
- **实时门票**: 基于 HMAC 的临时认证票据，用于 WebSocket 连接安全传递
- **Server 模式**: 可选 `axagent-server` 二进制，将桌面应用能力以服务形式对外提供

### 消息平台集成

通过 `rt-messaging` 实现多平台网关，支持 **钉钉、飞书、QQ、Slack、微信、WhatsApp、Telegram、Discord** 的消息接收、命令解析与 AI 自动回复。

### 工具系统

**163+ 内置工具**，统一通过 `Tool` trait 注册，覆盖 15 大类别：

| 类别       | 工具示例                                                                                                                                                             |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 文件操作   | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, 目录/删除/移动等 11 个                                                                                       |
| Shell/Web  | `bash`, `web_fetch`, `web_search`                                                                                                                                    |
| 网络       | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                               |
| 浏览器     | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot` 等 10 个（CDP）                                                                            |
| 计算机控制 | `computer_use`（鼠标/键盘/截图）                                                                                                                                     |
| Git        | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                        |
| 知识库     | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document` 等 6 个                                                                                         |
| 任务管理   | `todo_write`, `task_*`（6 个）, `cron_*`（3 个）, `plan` 相关                                                                                                        |
| 消息推送   | `push_notification`, `send_message`, 团队协作工具                                                                                                                    |
| 数据库     | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                |
| 存储       | `get_storage_info`, `upload_storage_file`, `download_storage_file` 等 5 个                                                                                           |
| 导出/格式  | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown` 等 9 个                                                                                 |
| OCR        | `ocr_image`, `ocr_detect_langs`                                                                                                                                      |
| Obsidian   | `obsidian_search`, `obsidian_read`, `obsidian_backlinks` 等 9 个                                                                                                     |
| 其他       | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD、DevOps、RPC、测试等 |

### MCP 协议

基于 `rmcp` 的完整 MCP (Model Context Protocol) 实现：

- **传输层**: stdio 子进程 + Streamable HTTP + SSE
- **OAuth 认证**: 支持 MCP 服务器的 OAuth 授权流程
- **工具发现**: 自动发现和注册 MCP 服务器暴露的工具
- **MCP 管理器**: 服务器生命周期管理、健康检查、自动重连

### 插件系统

OpenClaw 兼容的三级插件架构（内置/捆绑/外部）：

- npm 包安装，内置市场 UI 搜索和安装
- 插件 manifest 定义、权限声明、沙箱隔离执行
- 自定义工具注册、Agent 提供者、Hook 拦截
- 技能安装器：从插件包安装技能到技能系统

### 动态 UI 引擎

- **Schema 驱动**: 通过 JSON Schema 声明式构建界面，无需写代码
- **31 个内置组件**: 容器（7）/ 数据展示（6）/ 表单（9）/ 媒体（4）/ 其他（5）
- **数据绑定**: 声明式数据源绑定与条件渲染
- **NL2UI**: 自然语言直接生成动态 UI 界面

### ACP 客户端 SDK

- **ACP（Agent Client Protocol）**: 双语言 SDK（TypeScript + Python），零第三方依赖
- 会话管理、Prompt 发送、工具调用记录、WebSocket 事件流
- 通过 `/acp/v1/*` 端点与 AxAgent 服务通信

### 安全防护

- **AES-256-GCM 加密**: API Key 和敏感配置本地加密存储（`crypto` crate）
- **提示词注入防护**: 四级防御管线（`prompt-guard`）—— 模式检测 → 分隔符转义 → XML 包装器 → 信任标签，集成到会话、提示词构建、Git、RAG 全链路
- **SSRF 防护**: URL 安全检查，阻止对内网地址的请求
- **内容过滤**: 多类型内容安全过滤
- **速率限制**: 工具调用和 API 请求令牌桶限流
- **熔断器**: 连续失败自动熔断
- **访问控制**: 基于策略的工具访问权限控制
- **沙箱隔离**: 智能体和技能执行环境隔离

### 开发者工具

- **分布式追踪** (`telemetry`): OpenTelemetry 集成，Span/Trace 可视化
- **结构化日志**: tracing-subscriber + chrono 时间戳
- **回放调试**: 智能体执行轨迹录制（`trajectory_recorder`）与回放
- **DevTools 面板**: Trace Explorer 时间线查看器、Benchmark Runner、Tool Recommender
- **基准测试**: Criterion benchmarks（tool_exec / llm_call / search）
- **CI 检查**: `npm run ci:check` 集成类型检查、lint、格式化校验

### 桌面与移动端体验

- **响应式布局**: CSS 断点自适应桌面/平板/手机（3 级设备布局：`desktop` / `tablet` / `mobile`）
- **11 种语言**: 简体中文、繁体中文、英语、日语、韩语、法语、德语、西班牙语、俄语、印地语、阿拉伯语
- **主题引擎** (`rt-theme`): 深色/浅色主题 + 多个预设，Ant Design 6 深度定制
- **Monaco 编辑器**: 语法高亮、差异预览、多语言支持
- **xterm.js 终端**: WebLinks、Unicode 11、搜索
- **虚拟滚动**: @tanstack/react-virtual + react-virtuoso
- **图表渲染**: D2 + Mermaid + Recharts + Sigma（图谱）
- **Command Palette**: Ctrl+K 全局命令面板
- **系统托盘 + 全局快捷键 + 开机自启**: 无干扰后台运行
- **自动更新**: 可配置间隔的 GitHub Releases 版本检测
- **代理支持**: HTTP / SOCKS5 代理配置
- **云工作空间**: S3 和 WebDAV 存储同步，冲突检测与双向同步

### 移动端

- Android APK/AAB（arm64-v8a, armeabi-v7a, x86_64）
- iOS IPA（arm64）
- 移动端专属适配：安全区适配、底部导航、Drawer 导航

---

## 技术架构

### 技术栈

| 层级          | 技术                                     | 版本 |
| ------------- | ---------------------------------------- | ---- |
| 桌面框架      | Tauri                                    | 2.11 |
| 前端框架      | React                                    | 19   |
| 类型系统      | TypeScript                               | 7    |
| UI 库         | Ant Design                               | 6    |
| CSS 框架      | TailwindCSS                              | 4    |
| 状态管理      | Zustand                                  | 5    |
| 路由          | React Router                             | 7    |
| 代码编辑器    | Monaco Editor                            | 0.55 |
| 终端          | xterm.js                                 | 6    |
| 工作流编辑器  | ReactFlow                                | 12   |
| 图表          | D2 + Mermaid + Recharts + Sigma          |      |
| 动画          | Framer Motion                            | 12   |
| 虚拟滚动      | @tanstack/react-virtual + react-virtuoso |      |
| 拖拽          | @dnd-kit                                 | 6    |
| Markdown 渲染 | markstream-react + stream-markdown       |      |
| 国际化        | i18next + react-i18next                  |      |
| 构建工具      | Vite                                     | 8    |
| 测试          | Vitest + Playwright                      |      |
| 格式化        | dprint（TS/JSON/Markdown/TOML）+ rustfmt |      |
| Lint          | ESLint + Oxlint + Clippy                 |      |

### 后端架构: Harness 依赖注入模式

采用 Rust workspace 架构，包含 **37 个成员**（主 crate + 35 个库 crate + schema-gen），遵循 **Harness 依赖注入架构**：

> 所有 crate 通过 axagent-harness 定义的 trait 接口解耦，运行时由 axagent-runtime 装配和注入依赖。
> 依赖方向：`具体实现 → harness ← 调用方`

**harness** 是架构基石 — 零业务逻辑、零具体实现，仅含 trait 定义、纯数据 DTO、常量和统一错误类型。被所有其他 crate 依赖，自身不依赖任何 axagent-* crate（200+ trait 定义，涵盖 Agent/Provider/Tool/RAG/存储/MCP/插件/安全/可观测性/记忆/学习/浏览器/消息/认知路由等）。

```
src-tauri/crates/
├── harness/          # 架构基石 — trait 接口、DTO、错误类型、DI 契约
├── entities/         # SeaORM 实体模型
├── dao/              # 数据访问层（CRUD）
├── migration/        # 数据库迁移
├── crypto/           # AES-256-GCM 加解密与密钥管理
├── credential/       # 凭据安全存储
├── storage/          # 文件存储抽象（本地/S3/WebDAV），ZIP 读写
├── cache/            # 内存缓存层
├── disk-cache/       # 磁盘文件级缓存
├── search/           # 检索引擎（FTS5 + sqlite-vec + candle 本地嵌入）
├── document-parser/  # 文档文本提取（PDF/DOCX/XLSX/PPTX）
├── kit/              # 通用工具集（路径/编码/哈希/日期）
├── runtime-core/     # 运行时公共类型、配置常量
├── runtime/          # 运行时服务编排 — 装配全部 crate 的 DI 容器
├── rt-workflow/      # 工作流引擎 — DAG 编排、节点执行器、YAML 序列化
├── rt-messaging/     # 消息平台网关 — 钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord
├── rt-webhook/       # 通用 Webhook 服务器
├── rt-dashboard/     # 仪表盘插件框架
├── rt-theme/         # 主题引擎
├── agent/            # AI 智能体核心 — 80+ 模块
│                     #   ReAct引擎/层级规划/深度研究/事实核查/思维树/反思/
│                     #   自验证/错误恢复/RL优化/LoRA微调/评估/工具推荐/A/B测试/
│                     #   协调器/黑板/视觉管线/Web搜索/学术搜索/Wiki编译等
├── orchestrator/     # 智能体编排 — 多智能体调度、DAG 分解、动态子图执行
├── providers/        # 模型提供商适配器（13 种）
├── tools/            # 工具体系 — Tool trait/注册表/编排/流式/沙箱/163+内置工具
├── gateway/          # API 网关 — axum HTTP/WS 服务器、OAuth、速率限制、Prometheus
├── mcp/              # MCP 协议 — stdio + Streamable HTTP + SSE，基于 rmcp
├── trajectory/       # 学习系统 — 记忆/技能进化/用户画像/梦境整合
├── plugins/          # 插件系统 — OpenClaw 兼容、npm 包安装、市场
├── telemetry/        # 可观测性 — OpenTelemetry、结构化日志、运行时指标
├── prompt-guard/     # 提示词注入防护 — L1-L4 多级检测管线
├── npm/              # npm 注册表客户端
├── crdt/             # 协同编辑数据结构
├── device/           # 设备管理
├── axagent-mobile/   # 移动端适配层
├── agent-macro/      # 智能体宏
├── agent-command-types/ # 智能体命令类型
└── schema-gen/       # 数据库 Schema 生成工具
```

### 前端架构

```
src/
├── pages/            # 页面（24 个）
│   ├── ChatPage           # 对话主界面 — 侧边栏/消息流/Agent 面板/多 Tab
│   ├── DashboardPage      # 数据仪表盘 — 用量统计/模型分布/趋势图表
│   ├── WorkflowPage       # 工作流编辑器 — ReactFlow DAG 可视化
│   ├── KnowledgeHubPage   # 知识库管理 — 文档上传/索引/检索
│   ├── MemoryPage         # 记忆管理
│   ├── SkillsPage         # 技能市场
│   ├── SettingsPage       # 设置面板 — 40+ 配置项
│   ├── TerminalPage       # 内置终端 — xterm.js
│   ├── FilesPage          # 文件管理
│   ├── GatewayLinkPage    # API 网关与外部链接管理
│   ├── QuickBarPage       # 快捷栏（独立窗口）
│   ├── DynamicUIManagerPage / DynamicPageViewer  # 动态 UI 引擎
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # 学习图谱
│   ├── FineTunePage       # LoRA 微调
│   ├── PersonaPage        # 角色管理
│   ├── WorkflowMarketplace # 工作流市场
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 个模块，500+ 组件
│   ├── chat/         # 对话（消息流/输入/ChatView/TabBar/RightPanel/附件/工具调用渲染）
│   ├── layout/       # 布局 — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader 等
│   ├── agent/        # Agent 面板/入口/迷你面板
│   ├── workflow/     # 工作流编辑器（节点/连线/面板/模板/AI辅助）
│   ├── settings/     # 设置面板（40+ 子组件）
│   ├── skill/        # 技能编辑器/渲染器/浮动面板
│   ├── dynamicUI/    # 动态 UI 组件（31 个内置组件）
│   ├── gateway/      # API 网关管理
│   ├── files/        # 文件管理
│   ├── terminal/     # 终端组件
│   ├── search/       # 搜索界面
│   ├── benchmark/    # 基准测试面板
│   ├── decomposition/# 技能分解与工具生成
│   ├── devtools/     # Trace/Span 时间线 + RL Training 面板
│   ├── approval/     # 审批流程界面
│   ├── recommendation/ # 工具/模型推荐
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # 帮助面板
│   ├── notification/ # 通知组件
│   ├── proactive/    # 主动建议
│   ├── llm-wiki/     # LLM Wiki 组件
│   ├── wiki/         # Wiki 组件
│   ├── fine-tune/    # 微调界面
│   ├── trace/        # Trace 组件
│   ├── style/        # 样式/主题
│   ├── shared/       # 共享组件（ErrorBoundary / PageContextProvider）
│   └── common/       # 通用组件（Icon 等）
│
├── stores/           # Zustand 状态管理（82 个 store）
│   ├── domain/       # 9 个核心业务 store（对话/流/压缩/偏好/多模型等）
│   ├── feature/      # 61 个功能模块 store（智能体/工作流/知识库/技能/网关/记忆/终端等）
│   ├── shared/       # 8 个跨组件共享 store（UI/标签页/工作区/后端状态等）
│   └── devtools/     # 4 个开发者工具 store
│
├── hooks/            # React Hooks（快捷键/命令面板/响应式/滚动条/主题/Avatar 等）
├── lib/              # 工具函数库（invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout 等 45+ 模块）
├── types/            # TypeScript 类型定义
├── theme/            # Shadcn 主题引擎
├── i18n/             # 11 语言翻译文件（zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar）
├── constants/        # 常量与功能开关
└── sdk/              # ACP 客户端 SDK（TypeScript + Python）
```

### 功能开关

项目通过 `featureFlags.ts` 管理渐进式功能发布：

| 开关                | 状态 | 说明                              |
| ------------------- | ---- | --------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅   | 全局 Agent Panel + 页面上下文注入 |
| `DYNAMIC_UI`        | ✅   | 动态 UI 构建引擎                  |
| `SELF_EVOLUTION_UI` | ❌   | 自我进化前端控制面                |
| `NL_EXTENSION`      | ❌   | 自然语言驱动动态业务扩展          |

### Tauri 插件

| 插件                | 用途              |
| ------------------- | ----------------- |
| `autostart`         | 开机自启          |
| `clipboard-manager` | 剪贴板读写        |
| `dialog`            | 文件选择对话框    |
| `fs`                | 文件系统访问      |
| `global-shortcut`   | 全局快捷键注册    |
| `notification`      | 系统通知          |
| `opener`            | 外部链接/文件打开 |
| `process`           | 进程管理          |
| `updater`           | 自动更新          |

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
- [Rust](https://www.rust-lang.org/) 1.75+（edition 2024）
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（MSVC + Windows SDK）
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### 开发

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # 开发模式（前端 Vite HMR + Tauri 窗口）
```

### 构建

```bash
npm run tauri build    # 桌面端生产构建

npm run tauri:android:build   # Android 构建
npm run tauri:ios:build       # iOS 构建
```

桌面端构建产物位于 `src-tauri/target/release/`。

### 测试

```bash
npm run test           # 前端单元测试（Vitest watch）
npm run test:run       # 前端单元测试（单次运行）
npm run test:e2e       # E2E 测试（Playwright）

# Rust 后端测试
cd src-tauri && cargo test

# 类型检查 & Lint
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint 格式化
npm run lint:eslint    # ESLint 检查
npm run contracts      # API 契约检查

# CI 全量检查
npm run ci:check
```

### 常用脚本

| 命令                     | 用途                 |
| ------------------------ | -------------------- |
| `npm run bump`           | 版本号升级（交互式） |
| `npm run docs`           | 生成 TypeDoc 文档    |
| `npm run skill:create`   | 创建新技能脚手架     |
| `npm run skill:validate` | 验证技能定义         |
| `npm run check:types`    | 类型一致性检查       |

---

## 平台支持

| 平台    | 架构                                  |
| ------- | ------------------------------------- |
| Windows | x86_64, ARM64                         |
| macOS   | Apple Silicon (arm64), Intel (x86_64) |
| Linux   | x86_64, ARM64                         |
| Android | arm64-v8a, armeabi-v7a, x86_64        |
| iOS     | arm64                                 |

---

## 开源协议

本项目基于 [AGPL-3.0-only](LICENSE) 协议开源。

---

## 致谢

AxAgent 构建在众多优秀开源项目之上：

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
- [Zustand](https://zustand.docs.pmnd.rs/) — 状态管理
- [Framer Motion](https://www.framer.com/motion/) — 动画库
- [Recharts](https://recharts.org/) — 图表库
