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
- **多模型对比** — 同时向多个模型提问，side-by-side 对比结果
- **函数调用** — 跨所有支持提供商的结构化函数调用
- **OpenAI Responses API** — 支持 OpenAI Responses 格式传输
- **实时 API** — 兼容 OpenAI 实时 API 的 WebSocket 事件推送

### 🔐 AI 智能体系统

智能体系统基于精密架构构建，具备以下特性：

- **ReAct 推理引擎** — 融合推理与行动，内置自验证确保任务执行可靠
- **层级规划器** — 将复杂任务分解为具有阶段和依赖关系的结构化计划
- **任务分解器** — 自动将复杂任务分解为可执行的子任务
- **深度研究** — 多源搜索编排、引用追踪与可信度评估
- **事实核查** — AI 驱动的事实验证与来源分类
- **搜索编排** — 多搜索提供商协调，支持搜索规划和结果综合
- **学术搜索** — 学术文献检索和引用分析
- **计算机控制** — AI 控制的鼠标点击、键盘输入、屏幕滚动，配合视觉模型分析
- **屏幕感知** — 截图捕获和视觉模型分析，用于 UI 元素识别
- **三级权限模式** — 默认（需要审批）、接受编辑（自动批准）、完全访问（无提示）
- **沙箱隔离** — 智能体操作严格限制在指定工作目录内
- **工具审批面板** — 实时显示工具调用请求，支持逐条审批
- **成本追踪** — 实时显示每个会话的 token 使用量和成本统计
- **暂停/恢复** — 随时暂停智能体执行，稍后恢复
- **检查点系统** — 持久化检查点用于崩溃恢复和会话重连
- **错误恢复引擎** — 自动错误分类、根因分析和恢复策略执行
- **循环检测** — 自动检测和中断智能体推理中的循环行为
- **思维链** — 智能体决策推理的可视化，逐步分解
- **主动模式** — 智能体可主动提供建议和执行操作
- **目的管理** — 维护和追踪智能体的执行目的与上下文

### 👥 多智能体协作

- **子智能体协调** — 主从架构，支持多个协作智能体
- **并行执行** — 多个智能体并行处理任务，支持依赖感知调度
- **对抗性辩论** — Pro/Con 辩论轮次，支持论点强度评分和反驳追踪
- **智能体角色** — 预定义角色（研究员、规划师、开发者、评审员、综合员）用于团队协作
- **智能体编排器** — 多智能体团队的中心化消息路由和状态管理
- **通信图谱** — 智能体交互和消息流的可视化展示
- **Swarm 集群** — 多进程智能体集群，支持权限同步和自动重连
- **Buddy 伙伴系统** — 可配置的智能体伙伴，支持物种和属性定义
- **共享记忆** — 跨智能体共享的内存空间，支持统计和查询
- **团队 Cron 注册** — 团队级别的定时任务调度

### ⭐ 技能系统

- **技能市场** — 内置市场，浏览和安装社区贡献的技能
- **技能创建** — 从提案自动创建技能，支持 Markdown 编辑器
- **技能进化** — 基于执行反馈的 AI 驱动的现有技能自动分析和改进
- **技能匹配** — 语义匹配，推荐与对话上下文相关的技能
- **技能分解** — 自动将复杂任务分解为可执行的原子技能（LLM 辅助/多轮/工作流验证）
- **生成工具** — AI 自动生成并注册新工具，扩展智能体能力
- **技能中心** — 集中的技能发现和配置管理界面
- **技能中心客户端** — 与远程技能中心集成，支持社区分享
- **技能依赖检查** — 自动检测技能依赖和工具可用性
- **技能沙箱容器** — 技能在隔离环境中安全执行

### 🔄 工作流系统

工作流引擎实现了基于 DAG 的任务编排系统：

- **可视化工作流编辑器** — 拖放式工作流设计器，支持节点连接和配置
- **丰富节点类型** — 15 种节点类型：触发器、智能体、LLM、条件、并行、循环、合并、延迟、工具、代码、子工作流、向量检索、文档解析、验证、结束
- **工作流模板** — 内置预设：代码审查、Bug 修复、文档、测试、重构、探索、性能、安全、功能开发
- **DAG 执行** — Kahn 算法拓扑排序，支持循环检测
- **并行调度** — 流水线式执行，快速步骤不等慢速步骤
- **重试策略** — 指数退避，每步可配置最大重试次数
- **部分完成** — 失败的步骤不会阻塞独立的下游步骤
- **版本管理** — 工作流模板版本控制，支持回滚
- **执行历史** — 详细记录，支持状态追踪和调试
- **AI 辅助** — AI 辅助工作流设计、节点推荐和智能体提示词优化
- **语义检查** — 工作流语义验证，检测潜在问题
- **n8n 导入** — 支持从 n8n 目录导入工作流
- **调试面板** — 工作流执行过程的实时调试和状态查看

### 📚 知识与记忆

- **知识库（RAG）** — 多知识库支持，支持文档上传、自动解析、分块和向量索引
- **混合搜索** — 结合向量相似度搜索与 BM25 全文排名
- **重排序** — Cross-encoder 重排序，提升检索精度
- **三级召回管道** — AST 索引 + 向量搜索 + FTS5 的多级召回机制
- **知识图谱** — 知识关联的实体关系可视化（实体、属性、关系、流、接口）
- **Wiki 系统** — LLM Wiki 编译器与验证器，支持知识图谱可视化与增量同步
- **Wiki 笔记** — 双向链接笔记系统，支持图谱视图和自动链接同步
- **记忆系统** — 多命名空间记忆，支持手动录入或 AI 自动提取
- **闭环记忆** — 集成 Honcho 和 Mem0 持久化记忆提供商
- **FTS5 全文搜索** — 跨对话、文件、记忆的快速检索
- **会话搜索** — 跨所有对话会话的高级搜索
- **上下文管理** — 灵活附加文件、搜索结果、知识片段、记忆、工具输出
- **文档解析** — 多格式文档自动解析和内容提取
- **增量索引** — 文件变更的增量索引更新

### 🌐 API 网关

- **本地 API 服务器** — 内置 OpenAI 兼容、Claude 和 Gemini 接口服务器
- **外部链接** — 一键集成 Claude CLI、OpenCode，自动同步 API Key 和模型
- **Key 管理** — 生成、撤销、启用/禁用访问 Key，支持描述
- **用量分析** — 按 Key、提供商、日期的请求量和 token 使用量
- **SSL/TLS 支持** — 内置自签名证书，支持自定义证书
- **请求日志** — 完整记录所有 API 请求和响应
- **配置模板** — Claude、Codex、OpenCode、Gemini 的预建模板
- **实时 API** — 兼容 OpenAI 实时 API 的 WebSocket 事件推送
- **平台集成** — 支持钉钉、飞书、QQ、Slack、微信、WhatsApp、Telegram、Discord
- **网关诊断** — 连接诊断和程序策略管理
- **限流器** — API 请求速率限制和流量控制
- **持久化队列** — 请求持久化队列管理

### 🔧 工具与扩展

- **MCP 协议** — 完整的模型上下文协议实现，支持 stdio 和 HTTP/WebSocket 传输
- **OAuth 认证** — MCP 服务器的 OAuth 流程支持
- **MCP 自动启动** — MCP 服务器自动启动和生命周期管理
- **MCP 工具桥接** — MCP 工具与智能体工具系统的桥接
- **插件系统** — 内置/捆绑/外部三级插件架构，支持工具注册、钩子与生命周期管理
- **内置工具** — 全面的文件操作（读/写/编辑）、代码执行、搜索（Grep/Glob）、Bash、Web 搜索、Web 抓取、计划管理、Cron 调度、REPL、LSP、上下文管理、计算机控制、消息推送、待办事项等
- **工具权限系统** — 工具权限分类、规则管理和使用追踪
- **Bash 安全** — 命令解析、路径验证和沙箱安全控制
- **LSP 客户端** — 内置语言服务器协议，支持代码补全和诊断
- **AST 索引** — 代码文件的 AST 解析和索引构建
- **终端后端** — 支持本地、Docker 和 SSH 终端连接
- **浏览器自动化** — 通过 CDP 集成浏览器控制能力（导航、截图、点击、填写、文本提取等）
- **UI 自动化** — 跨平台 UI 元素识别和控制
- **Git 工具** — Git 操作，支持分支检测和冲突感知
- **工具推荐** — 基于上下文的智能工具推荐引擎
- **工具编排** — 多工具协调执行和流式输出
- **工具统计** — 工具使用频率和性能统计

### 📊 内容渲染

- **Markdown 渲染** — 完整支持代码高亮、LaTeX 数学公式、表格、任务列表
- **Monaco 代码编辑器** — 内置编辑器，支持语法高亮、复制、差异预览
- **图表渲染** — Mermaid 流程图、D2 架构图、ECharts 交互式图表
- **产物面板** — 代码片段、HTML 草稿、React 组件、Markdown 笔记，支持实时预览
- **四种预览模式** — 代码（编辑器）、分屏（并排）、预览（仅渲染）、React 组件预览
- **会话检查器** — 会话结构的树形视图，快速导航
- **引用面板** — 追踪和显示来源引用，支持可信度评分
- **信息图渲染** — 支持信息图可视化展示

### 🛡️ 数据与安全

- **AES-256 加密** — API Key 和敏感数据使用 AES-256-GCM 加密
- **隔离存储** — 应用状态存储在 `~/.axagent/`，用户文件存储在 `~/Documents/axagent/`
- **自动备份** — 计划备份到本地目录或 WebDAV 存储
- **备份恢复** — 一键从历史备份恢复
- **导出选项** — PNG 截图、Markdown、纯文本、JSON 格式
- **存储管理** — 可视化磁盘使用显示和清理工具
- **文件授权** — 文件访问授权和撤销管理
- **操作审计** — 关键操作的审计日志记录

### 🖥️ 桌面体验

- **主题引擎** — 深色/浅色主题，支持跟随系统或手动偏好
- **界面语言** — 11 种语言：简体中文、繁体中文、英语、日语、韩语、法语、德语、西班牙语、俄语、印地语、阿拉伯语
- **系统托盘** — 最小化到托盘，不中断后台服务
- **置顶窗口** — 窗口置顶于其他窗口之上
- **全局快捷键** — 可自定义快捷键调出主窗口
- **QuickBar** — 快速访问浮动条，一键唤起
- **开机自启** — 可选在系统启动时运行
- **代理支持** — HTTP 和 SOCKS5 代理配置
- **自动更新** — 自动检查版本，有更新时提示
- **命令面板** — `Cmd/Ctrl+K` 快速访问命令
- **引导向导** — 首次使用的交互式引导和 Ollama 检测
- **通知中心** — 统一的应用内通知管理

### 🔬 高级功能

- **深度研究** — 多源搜索、引用追踪、可信度评估与内容综合
- **事实核查** — AI 驱动的事实验证与来源分类
- **Cron 调度器** — 自动化任务调度，支持每日/每周/每月模板和自定义 cron 表达式
- **Webhook 系统** — 事件订阅，支持工具完成、智能体错误、会话结束通知
- **用户画像** — 自动学习代码风格、命名规范、缩进、注释风格、沟通偏好
- **RL 优化器** — 强化学习优化工具选择和任务策略
- **LoRA 微调** — 使用 LoRA 进行本地训练的自定义模型适配
- **主动建议** — 基于对话内容和用户模式的上下文感知提示
- **上下文预测** — 预测用户下一步操作并预取相关资源
- **梦境整合** — 后台自动整合记忆与模式，优化长期知识
- **错误恢复** — 自动错误分类、根因分析和恢复建议
- **开发者工具** — Trace、Span、时间线可视化，用于调试和性能分析
- **基准测试系统** — SWE-bench / Terminal-bench 任务性能评估和指标，带评分卡
- **风格迁移** — 将学习的代码风格偏好应用到生成的代码
- **仪表盘插件** — 可扩展的仪表盘，支持自定义面板和小组件
- **协作共享** — CRDT 实时协作与一键会话分享
- **浏览器扩展** — Wiki Clipper 浏览器扩展，快速剪藏网页到 LLM Wiki
- **Python SDK** — 提供 Python SDK 用于与 AxAgent 集成
- **智能路由** — 请求智能路由和分类
- **语义缓存** — 基于语义的响应缓存，减少重复计算
- **上下文压缩** — 自动压缩长上下文，优化 token 使用
- **消息批量处理** — 消息批量发送和优化
- **连接池** — 数据库和 API 连接池管理
- **特性开关** — 可配置的功能特性开关系统
- **策略引擎** — 权限和操作策略的集中管理
- **资源治理** — 智能体资源使用限制和治理
- **LAN 传输** — 局域网文件传输能力

---

## 技术架构

### 技术栈

| 层级 | 技术 |
|------|------|
| **框架** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **状态管理** | Zustand 5 |
| **路由** | React Router 7 |
| **国际化** | i18next + react-i18next |
| **后端** | Rust + SeaORM 2 + SQLite |
| **向量数据库** | sqlite-vec |
| **代码编辑器** | Monaco Editor |
| **图表** | Mermaid + D2 + ECharts（CDN） |
| **终端** | xterm.js 6 |
| **工作流** | ReactFlow 11 |
| **构建** | Vite 8 + npm |

### Rust 后端架构

后端组织为 Rust workspace，包含 10 个专业化的 crates：

```
src-tauri/crates/
├── agent/         # AI 智能体核心
│   ├── react_engine.rs          # ReAct 推理引擎
│   ├── coordinator.rs           # 智能体协调
│   ├── hierarchical_planner.rs  # 任务分解
│   ├── task_decomposer.rs       # 子任务分解
│   ├── self_verifier.rs         # 输出验证
│   ├── verification_agent.rs    # 验证智能体
│   ├── error_recovery_engine.rs # 错误恢复引擎
│   ├── error_classifier.rs      # 错误分类
│   ├── recovery_strategies.rs   # 恢复策略
│   ├── loop_detector.rs         # 循环检测
│   ├── vision_pipeline.rs       # 屏幕感知
│   ├── deep_research.rs         # 深度研究
│   ├── fact_checker.rs          # 事实核查
│   ├── research_agent.rs        # 研究智能体
│   ├── search_planner.rs        # 搜索规划
│   ├── search_orchestrator.rs   # 搜索编排
│   ├── academic_search.rs       # 学术搜索
│   ├── source_validator.rs      # 来源验证
│   ├── source_classifier.rs     # 来源分类
│   ├── credibility_evaluator.rs # 可信度评估
│   ├── citation_tracker.rs      # 引用追踪
│   ├── content_synthesizer.rs   # 内容综合
│   ├── outline_builder.rs       # 大纲构建
│   ├── reference_builder.rs     # 参考构建
│   ├── proactive_mode.rs        # 主动模式
│   ├── purpose_manager.rs       # 目的管理
│   ├── graph_insights.rs        # 图谱洞察
│   ├── insight_generator.rs     # 洞察生成
│   ├── schema_manager.rs        # Schema 管理
│   ├── ingest_pipeline.rs       # 数据摄取管道
│   ├── session_manager.rs       # 会话管理
│   ├── health_checker.rs        # 健康检查
│   ├── metrics.rs               # 指标收集
│   ├── evaluator/               # 基准测试评估
│   ├── fine_tune/               # LoRA 微调
│   ├── rl_optimizer/            # RL 策略优化
│   └── tool_recommender/        # 工具推荐引擎
│
├── core/          # 核心工具
│   ├── db.rs                   # SeaORM 数据库
│   ├── vector_store.rs         # sqlite-vec 集成
│   ├── rag.rs                  # RAG 抽象层
│   ├── hybrid_search.rs        # 向量 + FTS5 搜索
│   ├── recall_pipeline.rs      # 三级召回管道
│   ├── crypto.rs               # AES-256 加密
│   ├── mcp_client.rs           # MCP 协议客户端
│   ├── browser_automation.rs   # 浏览器自动化
│   ├── computer_control.rs     # 计算机控制
│   ├── screen_vision.rs        # 屏幕视觉
│   ├── screen_capture.rs       # 屏幕截图
│   ├── ui_automation.rs        # UI 自动化
│   ├── ast_index.rs            # AST 索引
│   ├── incremental_indexer.rs  # 增量索引
│   ├── document_parser.rs      # 文档解析
│   ├── markdown_parser.rs      # Markdown 解析
│   ├── text_chunker.rs         # 文本分块
│   ├── token_counter.rs        # Token 计数
│   ├── token_budget.rs         # Token 预算
│   ├── file_index.rs           # 文件索引
│   ├── file_authorizer.rs      # 文件授权
│   ├── file_store.rs           # 文件存储
│   ├── cache.rs                # 缓存管理
│   ├── disk_cache.rs           # 磁盘缓存
│   ├── cache_persister.rs      # 缓存持久化
│   ├── cache_snapshot.rs       # 缓存快照
│   ├── vector_cache.rs         # 向量缓存
│   ├── marketplace_service.rs  # 市场服务
│   ├── marketplace.rs          # 市场抽象
│   ├── operation_audit.rs      # 操作审计
│   ├── unified_config.rs       # 统一配置
│   ├── platform_config.rs      # 平台配置
│   ├── command_validator.rs    # 命令验证
│   ├── shell_parser.rs         # Shell 解析
│   ├── output_processor.rs     # 输出处理
│   ├── storage_inventory.rs    # 存储清单
│   ├── storage_migration.rs    # 存储迁移
│   ├── storage_paths.rs        # 存储路径
│   ├── s3_backup.rs            # S3 备份
│   ├── webdav.rs               # WebDAV 同步
│   ├── git_tools.rs            # Git 工具
│   ├── sandbox_runner.rs       # 沙箱运行器
│   ├── search.rs               # 搜索抽象
│   ├── reranker.rs             # 重排序
│   ├── model_knowledge.rs      # 模型知识
│   ├── prompt_template.rs      # 提示词模板
│   ├── preset_templates.rs     # 预设模板
│   ├── workflow_types.rs       # 工作流类型
│   ├── workflow_version.rs     # 工作流版本
│   ├── path_vars.rs            # 路径变量
│   ├── entity/                 # SeaORM 实体（40+ 表）
│   └── repo/                   # 数据仓库（30+ 仓库）
│
├── gateway/       # API 网关
│   ├── server.rs               # HTTP 服务器
│   ├── handlers.rs             # API 处理器
│   ├── routes.rs               # 路由定义
│   ├── auth.rs                 # 认证
│   ├── middleware.rs           # 中间件
│   ├── metrics.rs              # 指标收集
│   ├── native.rs               # 原生集成
│   ├── marketplace_handlers.rs # 市场接口
│   └── realtime.rs             # WebSocket 支持
│
├── plugins/       # 插件系统
│   ├── hooks.rs                # 钩子运行器
│   ├── agent_provider.rs       # 智能体提供者
│   ├── test_isolation.rs       # 测试隔离
│   └── lib.rs                  # 插件注册表与生命周期
│
├── providers/     # 模型适配器
│   ├── adapter.rs              # 适配器接口
│   ├── registry.rs             # 提供商注册表
│   ├── openai.rs               # OpenAI API
│   ├── openai_responses.rs     # OpenAI Responses API
│   ├── anthropic.rs            # Claude API
│   ├── gemini.rs               # Gemini API
│   ├── ollama.rs               # Ollama 本地
│   ├── openclaw.rs             # OpenClaw
│   ├── hermes.rs               # Hermes
│   ├── image_gen.rs            # 图像生成
│   ├── realtime_client.rs      # 实时 API 客户端
│   └── transport/              # 传输层（Chat Completions / Responses / Anthropic）
│
├── runtime/       # 运行时服务
│   ├── session.rs              # 会话管理
│   ├── workflow_engine.rs      # DAG 编排
│   ├── work_engine/            # 工作引擎（节点执行器 + 调度器 + 缓存层）
│   ├── mcp.rs                  # MCP 服务器
│   ├── mcp_client.rs           # MCP 客户端
│   ├── mcp_server.rs           # MCP 服务器实现
│   ├── mcp_stdio.rs            # MCP stdio 传输
│   ├── mcp_autostart.rs        # MCP 自动启动
│   ├── mcp_lifecycle_hardened.rs # MCP 生命周期管理
│   ├── mcp_tool_bridge.rs      # MCP 工具桥接
│   ├── cron/                   # 任务调度
│   ├── terminal/               # 终端后端（本地/Docker/SSH）
│   ├── benchmarks/             # SWE-bench / Terminal-bench
│   ├── collaboration/          # CRDT 协作与会话共享
│   ├── tool_generator/         # AI 工具生成
│   ├── message_gateway/        # 平台集成（钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord）
│   ├── buddy/                  # Buddy 伙伴系统（物种/属性/管理器）
│   ├── swarm/                  # Swarm 集群（进程后端/权限同步/重连）
│   ├── tasks/                  # 后台任务（梦境/远程智能体/进程内队友）
│   ├── adversarial_debate.rs   # 对抗性辩论
│   ├── agent_orchestrator.rs   # 多智能体编排
│   ├── agent_roles.rs          # 智能体角色
│   ├── webhook_dispatcher.rs   # Webhook 分发
│   ├── webhook_server.rs       # Webhook 服务器
│   ├── session_search.rs       # 会话搜索
│   ├── dashboard_plugin.rs     # 仪表盘插件
│   ├── dashboard_registry.rs   # 仪表盘注册表
│   ├── permissions.rs          # 权限管理
│   ├── permission_enforcer.rs  # 权限执行
│   ├── policy_engine.rs        # 策略引擎
│   ├── trust_resolver.rs       # 信任解析
│   ├── resource_governor.rs    # 资源治理
│   ├── green_contract.rs       # 绿色合约
│   ├── feature_flags.rs        # 特性开关
│   ├── module_switch.rs        # 模块切换
│   ├── mode_selector.rs        # 模式选择
│   ├── config.rs               # 运行时配置
│   ├── config_validate.rs      # 配置验证
│   ├── prompt.rs               # 提示词管理
│   ├── prompt_cache.rs         # 提示词缓存
│   ├── compact.rs              # 上下文压缩
│   ├── summary_compression.rs  # 摘要压缩
│   ├── compact_thresholds.rs   # 压缩阈值
│   ├── compact_warning.rs      # 压缩警告
│   ├── reactive_compact.rs     # 响应式压缩
│   ├── session_memory_compact.rs # 会话记忆压缩
│   ├── message_importance.rs   # 消息重要性评估
│   ├── message_batching.rs     # 消息批量处理
│   ├── rate_limiter.rs         # 限流器
│   ├── connection_pool.rs      # 连接池
│   ├── persistent_queue.rs     # 持久化队列
│   ├── persistent_queue_manager.rs # 队列管理器
│   ├── health_check.rs         # 健康检查
│   ├── cache_guard.rs          # 缓存守护
│   ├── checkpoint.rs           # 检查点
│   ├── branch_lock.rs          # 分支锁
│   ├── stale_base.rs           # 过期基线检测
│   ├── watch_patterns.rs       # 监视模式
│   ├── lan_transfer.rs         # LAN 传输
│   ├── tls_config.rs           # TLS 配置
│   ├── sse.rs                  # SSE 事件流
│   ├── api_server.rs           # API 服务器
│   ├── gateway_auth.rs         # 网关认证
│   ├── gateway_metrics.rs      # 网关指标
│   ├── bash.rs                 # Bash 执行
│   ├── bash_validation.rs      # Bash 验证
│   ├── shell_hooks.rs          # Shell 钩子
│   ├── shell_completer.rs      # Shell 补全
│   ├── terminal_analyzer.rs    # 终端分析
│   ├── git_context.rs          # Git 上下文
│   ├── git_tools.rs            # Git 工具
│   ├── file_ops.rs             # 文件操作
│   ├── hooks.rs                # 钩子管理
│   ├── hook_chain.rs           # 钩子链
│   ├── hook_config.rs          # 钩子配置
│   ├── plugin_hooks.rs         # 插件钩子
│   ├── plugin_lifecycle.rs     # 插件生命周期
│   ├── profile.rs              # 配置文件
│   ├── profile_manager.rs      # 配置管理器
│   ├── oauth.rs                # OAuth 认证
│   ├── usage.rs                # 用量统计
│   ├── bootstrap.rs            # 引导启动
│   ├── worker_boot.rs          # Worker 启动
│   ├── fork_bridge.rs          # Fork 桥接
│   ├── task_packet.rs          # 任务包
│   ├── task_router.rs          # 任务路由
│   ├── task_registry.rs        # 任务注册表
│   ├── transform_pipeline.rs   # 转换管道
│   ├── transport_handlers.rs   # 传输处理器
│   ├── general_engine.rs       # 通用引擎
│   ├── engine_bridge.rs        # 引擎桥接
│   ├── conversation.rs         # 对话管理
│   ├── session_control.rs      # 会话控制
│   ├── shared_memory.rs        # 共享内存
│   ├── validation_executor.rs  # 验证执行器
│   ├── recovery_recipes.rs     # 恢复配方
│   ├── error_recovery.rs       # 错误恢复
│   ├── theme_engine.rs         # 主题引擎
│   ├── token_budget_predictor.rs # Token 预算预测
│   ├── team_cron_registry.rs   # 团队 Cron 注册
│   ├── module_dream.rs         # 梦境模块
│   ├── json.rs                 # JSON 工具
│   └── lane_events.rs          # Lane 事件
│
├── telemetry/     # 遥测与追踪
│   ├── tracer.rs              # 分布式追踪
│   ├── metrics.rs             # 指标收集
│   ├── span.rs                # Span 管理
│   ├── event.rs               # 事件定义
│   ├── collector.rs           # 数据收集
│   ├── exporter.rs            # 数据导出
│   └── storage.rs             # 存储后端
│
├── tools/         # 工具系统
│   ├── registry.rs             # 工具注册表
│   ├── builtin_tools.rs        # 内置工具定义
│   ├── builtin_handlers.rs     # 内置工具处理器
│   ├── orchestration.rs        # 工具编排
│   ├── streaming.rs            # 流式输出
│   ├── stats.rs                # 使用统计
│   ├── recorder.rs             # 执行记录
│   ├── agent_def_loader.rs     # 智能体定义加载
│   ├── agent_def_types.rs      # 智能体定义类型
│   ├── bash/                   # Bash 工具（解析器/沙箱/安全/路径验证）
│   ├── hooks/                  # 钩子（注册表/执行器）
│   ├── mcp/                    # MCP 工具（注册表/OAuth/包装器）
│   ├── permissions/            # 权限（分类器/规则/追踪器）
│   └── tools/                  # 具体工具实现
│       ├── agent.rs            # 智能体工具
│       ├── bash.rs             # Bash 执行
│       ├── context.rs          # 上下文管理
│       ├── cron.rs             # Cron 调度
│       ├── glob.rs             # 文件通配
│       ├── grep.rs             # 内容搜索
│       ├── lsp.rs              # LSP 工具
│       ├── monitor.rs          # 监控工具
│       ├── plan.rs             # 计划工具
│       ├── repl.rs             # REPL 工具
│       ├── skill.rs            # 技能工具
│       ├── web_fetch.rs        # Web 抓取
│       ├── web_search.rs       # Web 搜索
│       ├── file_read.rs        # 文件读取
│       ├── file_write.rs       # 文件写入
│       ├── file_edit.rs        # 文件编辑
│       ├── computer_use.rs     # 计算机控制
│       ├── messaging.rs        # 消息发送
│       ├── push_notification.rs # 推送通知
│       ├── task_system.rs      # 任务系统
│       ├── todo_write.rs       # 待办事项
│       └── batch_missing.rs    # 批量缺失检测
│
├── trajectory/    # 学习系统
│   ├── memory.rs              # 记忆管理
│   ├── memory_provider.rs     # 记忆提供商接口
│   ├── auto_memory.rs         # 自动记忆提取
│   ├── skill.rs               # 技能系统
│   ├── skill_manager.rs       # 技能管理器
│   ├── skill_evolution.rs     # 技能进化
│   ├── skill_matcher.rs       # 技能匹配
│   ├── skill_proposal.rs      # 技能提案
│   ├── skills_hub_adapter.rs  # 技能中心适配器
│   ├── skills_hub_client.rs   # 技能中心客户端
│   ├── skill_decomposition/   # 技能分解（LLM 辅助/多轮/工作流验证/工具解析）
│   ├── rl.rs                  # RL 奖励信号
│   ├── rl_trainer.rs          # RL 训练器
│   ├── training_env.rs        # 训练环境
│   ├── behavior_learner.rs    # 行为学习
│   ├── behavior_tracker.rs    # 行为追踪
│   ├── pattern.rs             # 模式识别
│   ├── pattern_analyzer.rs    # 模式分析
│   ├── user_profile.rs        # 用户画像
│   ├── preference_learner.rs  # 偏好学习
│   ├── adaptation.rs          # 适应性调整
│   ├── dream_consolidation.rs # 梦境整合
│   ├── parallel_execution.rs  # 并行执行服务
│   ├── style_extractor.rs     # 风格提取
│   ├── style_applier.rs       # 风格应用
│   ├── style_vectorizer.rs    # 风格向量化
│   ├── style_migrator.rs      # 风格迁移
│   ├── suggestion_engine.rs   # 建议引擎
│   ├── proactive_assistant.rs # 主动助手
│   ├── context_predictor.rs   # 上下文预测
│   ├── task_prefetcher.rs     # 任务预取
│   ├── reminder_manager.rs    # 提醒管理
│   ├── nudge.rs               # 轻推系统
│   ├── insight.rs             # 洞察生成
│   ├── compactor.rs           # 数据压缩
│   ├── trajectory.rs          # 轨迹管理
│   ├── trajectory_compressor.rs # 轨迹压缩
│   ├── sub_agent.rs           # 子智能体
│   ├── batch.rs               # 批量处理
│   ├── context.rs             # 上下文管理
│   ├── fts5.rs                # FTS5 搜索
│   ├── hooks.rs               # 钩子
│   ├── storage.rs             # 存储
│   ├── scheduled_task.rs      # 定时任务
│   └── memory_providers/      # 记忆提供商（Honcho/Mem0/闭环/服务）
│
└── migration/     # 数据库迁移
    └── m20240101_000001~000010  # 10 个迁移文件
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
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/               # 功能模块状态（30+ store）
│   │   ├── agentStore.ts
│   │   ├── agentProfileStore.ts
│   │   ├── appConfigStore.ts
│   │   ├── backupStore.ts
│   │   ├── buddyStore.ts
│   │   ├── categoryStore.ts
│   │   ├── decompositionStore.ts
│   │   ├── dreamStore.ts
│   │   ├── executionStore.ts
│   │   ├── expertStore.ts
│   │   ├── fileStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── gatewayLinkStore.ts
│   │   ├── generatedToolStore.ts
│   │   ├── helpStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── llmWikiStore.ts
│   │   ├── localToolStore.ts
│   │   ├── memoryStore.ts
│   │   ├── mcpStore.ts
│   │   ├── nudgeStore.ts
│   │   ├── onboardingStore.ts
│   │   ├── planStore.ts
│   │   ├── platformStore.ts
│   │   ├── proactiveStore.ts
│   │   ├── promptTemplateStore.ts
│   │   ├── providerStore.ts
│   │   ├── searchStore.ts
│   │   ├── settingsStore.ts
│   │   ├── skillExtensionStore.ts
│   │   ├── skillStore.ts
│   │   ├── styleStore.ts
│   │   ├── terminalStore.ts
│   │   ├── themeStore.ts
│   │   ├── trajectoryStore.ts
│   │   ├── userProfileStore.ts
│   │   ├── wikiStore.ts
│   │   ├── workEngineStore.ts
│   │   └── workflowEditorStore.ts
│   ├── devtools/              # 开发者工具状态
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/                # 共享状态
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # React 组件（24 个模块）
│   ├── chat/                # 对话界面（90+ 组件）
│   ├── workflow/            # 工作流编辑器（节点/面板/模板/AI 辅助）
│   ├── gateway/             # API 网关 UI
│   ├── settings/            # 设置面板（40+ 组件）
│   ├── terminal/            # 终端 UI
│   ├── skill/               # 技能编辑器与渲染器
│   ├── benchmark/           # 基准测试面板
│   ├── decomposition/       # 技能分解与工具生成
│   ├── files/               # 文件管理页面
│   ├── fine-tune/           # LoRA 微调配置
│   ├── link/                # 外部链接管理
│   ├── llm-wiki/            # LLM Wiki 编辑器
│   ├── proactive/           # 主动建议系统
│   ├── recommendation/      # 工具推荐面板
│   ├── wiki/                # Wiki 管理
│   ├── devtools/            # Trace/Span 时间线
│   ├── style/               # 代码风格迁移
│   ├── layout/              # 布局组件（标题栏/侧边栏/命令面板）
│   ├── help/                # 帮助面板
│   ├── onboarding/          # 引导向导
│   ├── notification/        # 通知中心
│   ├── search/              # 会话搜索
│   ├── common/              # 通用组件
│   └── shared/              # 共享组件
│
├── pages/                    # 页面组件（22 个页面）
│   ├── ChatPage.tsx
│   ├── KnowledgePage.tsx
│   ├── KnowledgeHubPage.tsx
│   ├── MemoryPage.tsx
│   ├── WorkflowPage.tsx
│   ├── WorkflowMarketplace.tsx
│   ├── GatewayPage.tsx
│   ├── GatewayLinkPage.tsx
│   ├── LinkPage.tsx
│   ├── FilesPage.tsx
│   ├── FineTunePage.tsx
│   ├── SkillsPage.tsx
│   ├── WikiPage.tsx
│   ├── WikiEditorPage.tsx
│   ├── WikiGraphPage.tsx
│   ├── LlmWikiPage.tsx
│   ├── LlmWikiEditorPage.tsx
│   ├── IngestPage.tsx
│   ├── QuickBarPage.tsx
│   ├── SettingsPage.tsx
│   └── DevTools/
│       ├── TraceExplorer.tsx
│       ├── BenchmarkRunner.tsx
│       └── ToolRecommender.tsx
│
├── hooks/                    # React hooks（10 个）
├── lib/                      # 工具函数（含 Web Worker）
├── types/                    # TypeScript 类型定义（22 个）
├── sdk/                      # SDK（含 Python SDK）
└── i18n/                     # 11 种语言翻译
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

# 代码格式化
npm run format

# CI 检查
npm run ci:check
```

---

## 项目结构

```
AxAgent/
├── src/                         # 前端源码 (React + TypeScript)
│   ├── components/              # React 组件（24 个模块）
│   │   ├── chat/               # 对话界面（90+ 组件）
│   │   ├── workflow/           # 工作流编辑器组件
│   │   ├── gateway/            # API 网关组件
│   │   ├── settings/           # 设置面板（40+ 组件）
│   │   ├── terminal/           # 终端组件
│   │   ├── skill/              # 技能编辑器与渲染器
│   │   ├── benchmark/          # 基准测试
│   │   ├── decomposition/      # 技能分解
│   │   ├── files/              # 文件管理
│   │   ├── fine-tune/          # LoRA 微调
│   │   ├── link/               # 外部链接
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # 主动建议
│   │   ├── recommendation/     # 工具推荐
│   │   ├── wiki/               # Wiki 管理
│   │   ├── devtools/           # 开发者工具
│   │   ├── style/              # 代码风格
│   │   ├── layout/             # 布局组件
│   │   ├── help/               # 帮助面板
│   │   ├── onboarding/         # 引导向导
│   │   ├── notification/       # 通知中心
│   │   ├── search/             # 会话搜索
│   │   ├── common/             # 通用组件
│   │   └── shared/             # 共享组件
│   ├── pages/                   # 页面组件（22 个页面）
│   ├── stores/                  # Zustand 状态管理
│   │   ├── domain/            # 核心业务状态（6 个 store）
│   │   ├── feature/           # 功能模块状态（30+ store）
│   │   ├── devtools/          # 开发者工具状态（5 个 store）
│   │   └── shared/            # 共享状态（4 个 store）
│   ├── hooks/                   # React hooks（10 个）
│   ├── lib/                     # 工具函数（含 Web Worker）
│   ├── types/                   # TypeScript 类型定义（22 个）
│   ├── sdk/                     # SDK（含 Python SDK）
│   └── i18n/                    # 11 种语言翻译
│
├── src-tauri/                    # 后端源码 (Rust)
│   ├── crates/                  # Rust workspace（10 个 crates）
│   │   ├── agent/             # AI 智能体核心
│   │   ├── core/              # 数据库、加密、RAG
│   │   ├── gateway/           # API 网关服务器
│   │   ├── plugins/           # 插件系统
│   │   ├── providers/         # 模型提供商适配器
│   │   ├── runtime/           # 运行时服务
│   │   ├── tools/             # 工具系统
│   │   ├── trajectory/        # 记忆与学习
│   │   ├── telemetry/         # 追踪与指标
│   │   └── migration/         # 数据库迁移
│   └── src/                    # Tauri 入口点（70+ 命令模块）
│
├── extension/                  # 浏览器扩展（Wiki Clipper）
├── e2e/                        # Playwright E2E 测试
├── scripts/                    # 构建与工具脚本
└── website/                    # 项目网站（VitePress）
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
