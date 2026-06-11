[**English**](./README-EN.md) | **简体中文** | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp&amp&utm_source=badge-featured&amp&amp;&amp;#10;&amp;amp&amp&amp;;utm_medium=badge&amp&amp;#10&amp&amp;;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>跨平台 AI 桌面/移动客户端 | 多智能体协作 | 本地优先</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## 什么是 AxAgent？

**AxAgent v2.0** 是一款功能全面的跨平台 AI 桌面/移动应用，集成了先进的 AI 智能体能力和丰富的开发者工具。它支持多模型提供商、自主管道执行、可视化工作流编排、本地知识管理、内置 API 网关，覆盖 **Windows / macOS / Linux / Android / iOS** 五大平台。

---

## 截图预览

|         对话与模型选择          |         多智能体仪表盘          |
| :-----------------------------: | :-----------------------------: |
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

|           知识库 RAG            |          记忆与上下文           |
| :-----------------------------: | :-----------------------------: |
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

|          工作流编辑器           |             API 网关             |
| :-----------------------------: | :------------------------------: |
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## 核心功能

### 🤖 AI 模型支持

- **多提供商支持** — 原生集成 OpenAI、Anthropic Claude、Google Gemini、Ollama、OpenClaw、Hermes 及所有 OpenAI 兼容 API
- **多 Key 轮换** — 为每个提供商配置多个 API Key，自动轮换分发限流
- **本地模型推理** — 完整支持 Ollama 本地模型，包含 GGUF/GGML 文件管理
- **Candle 推理引擎** — 内置 Candle 本地推理，支持 rerank/judge 接口，GGUF 按需下载
- **模型管理** — 远程模型列表获取，可自定义参数（temperature、max tokens、top-p 等）
- **流式输出** — 实时逐 token 渲染，支持可折叠的思考块（Claude 扩展思考）
- **多模型对比** — 同时向多个模型提问，side-by-side 对比结果
- **函数调用** — 跨所有支持提供商的结构化函数调用
- **OpenAI Responses API** — 支持 OpenAI Responses 格式传输
- **实时 API** — 兼容 OpenAI 实时 API 的 WebSocket 事件推送
- **AI 图像生成** — 支持 DALL-E 3 和 Flux (Replicate)，多种尺寸预设（1:1/16:9/9:16/4:3），负面提示词
- **模型智能路由** — 按任务类型自动路由到不同模型（代码审查/摘要/翻译），支持自定义路由规则
- **语音通话** — 基于 OpenAI Realtime API 的实时语音对话，支持连接/说话/监听状态切换

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
- **智能体池面板** — 可视化子智能体/Worker/工作流步骤的实时状态，支持展开查看详情
- **智能体反思面板** — 任务执行后的质量评分、效率分析、错误模式和改进建议
- **专家选择器** — 导入/导出/自定义专家角色，按类别筛选，支持内置预设和 Agency 专家
- **智能体层级树** — 可视化展示智能体之间的层级关系和协作拓扑
- **意图分类器** — 自动识别用户输入的意图类型，优化路由和响应策略
- **信念状态管理** — 维护智能体对当前上下文的理解状态
- **目标评估器** — 评估任务目标的完成度和质量
- **上下文窗口管理** — 智能管理对话上下文窗口，优化 token 使用
- **项目记忆** — 跨会话的项目级知识持久化与检索
- **知识库管理** — 知识库的创建、更新、删除和查询
- **笔记系统** — 智能体内的结构化笔记存储与检索

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
- **协作面板** — 实时协作会话管理，支持邀请码分享、参与者角色（Owner/Editor/Viewer）、权限控制
- **会话分享** — 一键生成分享链接，支持终端/文件/模型访问权限配置

### ⭐ 技能系统

- **技能市场** — 内置市场，浏览和安装社区贡献的技能
- **技能创建** — 从提案自动创建技能，支持 Markdown 编辑器
- **技能进化** — 基于执行反馈的 AI 驱动的现有技能自动分析和改进
- **技能进化面板** — 可视化进化代数、最佳/平均适应度、收敛状态
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
- **Self-RAG** — 自检索增强生成，智能判断是否需要检索以及检索结果的相关性
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
- **插件系统** — OpenClaw 兼容的三级插件架构（内置/捆绑/外部），支持 npm 包安装、工具注册、钩子与生命周期管理
- **插件市场** — 内置市场 UI，支持 npm 搜索安装、确认弹窗
- **内置工具** — 全面的文件操作（读/写/编辑）、代码执行、搜索（Grep/Glob）、Bash、Web 搜索、Web 抓取、计划管理、Cron 调度、REPL、LSP、上下文管理、计算机控制、消息推送、待办事项等
- **工具权限系统** — 工具权限分类、规则管理和使用追踪
- **Bash 安全** — 命令解析、路径验证和沙箱安全控制
- **LSP 客户端** — 内置语言服务器协议，支持代码补全和诊断
- **AST 索引** — 代码文件的 AST 解析和索引构建
- **终端后端** — 支持本地、Docker 和 SSH 终端连接
- **浏览器自动化** — 通过 CDP 集成浏览器控制能力（导航、截图、点击、填写、文本提取等）
- **UI 自动化** — 跨平台 UI 元素识别和控制
- **Git 工具** — Git 操作，支持分支检测和冲突感知
- **Git 提交面板** — 可视化 Git diff 统计，AI 生成提交信息，一键暂存和提交
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
- **图表解释器** — AI 分析图表数据并可视化，支持柱状图/折线图/饼图/散点图/面积图，自动生成洞察和统计
- **差异查看器** — 对话版本对比，支持逐文件 Accept/Reject，自动语言检测
- **上下文分类栏** — 分段显示上下文各分类（消息/系统提示/知识/记忆/工具/技能）的 token 占比
- **上下文图谱** — ReactFlow 可视化对话上下文关系图（对话/模型/知识/记忆/MCP/搜索/技能节点）
- **命令建议** — 输入时自动推荐可用命令
- **引用管理器** — 追踪和分类引用来源（Web/学术/Wikipedia/GitHub/文档/新闻/博客/论坛），支持可信度评分
- **可信度徽章** — 五星评分可视化来源可信度

### 🛡️ 数据与安全

- **AES-256 加密** — API Key 和敏感数据使用 AES-256-GCM 加密
- **隔离存储** — 应用状态存储在 `~/.axagent/`，用户文件存储在 `~/Documents/axagent/`
- **自动备份** — 计划备份到本地目录或 WebDAV 存储
- **云工作空间** — 支持 S3 和 WebDAV 云存储同步，冲突检测与解决，双向同步
- **备份恢复** — 一键从历史备份恢复
- **导出选项** — PNG 截图、Markdown、纯文本、JSON 格式
- **存储管理** — 可视化磁盘使用显示和清理工具
- **文件授权** — 文件访问授权和撤销管理
- **操作审计** — 关键操作的审计日志记录

### 🖥️ 桌面体验

- **响应式布局** — 桌面/平板/手机三档自动适配（600px/900px 断点），窗口缩放实时切换
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
- **梦境状态指示器** — 实时显示后台 Dream 巩固运行状态和结果（记忆条数/模式数）
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

### 🛡️ 提示词注入防护（Prompt-Guard）

- **四级防护体系** — L1 模式检测（高风险拦截 + 中风险标记）→ L2 分隔符转义 → L3 XML 包装器 → L4 信任标签
- **Pipeline 编排器** — 多级检测管道串联，支持自定义风险阈值
- **Token Smuggling 检测** — 针对编码混淆和 token 走私攻击的专项检测
- **Strict 模式** — 严格模式测试 + 中风险原因命名 + 自定义模式文档
- **全管道集成** — 已集成到 session / prompt / git / RAG 各环节

### 📱 移动端支持

- **Android 原生** — APK/AAB 构建，支持 arm64-v8a / armeabi-v7a / x86_64
- **iOS 原生** — IPA 构建，支持 arm64
- **自适应布局** — 桌面/平板/手机三档自动适配（600px/900px CSS 断点，窗口缩放实时切换）
- **移动端导航** — Drawer 滑出导航 + 底部导航栏 + 闪现式浮动按钮
- **安全区适配** — Android 系统状态栏/导航栏 CSS env() 自适应
- **CSP 优化** — Android WebView CSP 协议白名单

---

## 技术架构

### 技术栈

| 层级           | 技术                                                   |
| -------------- | ------------------------------------------------------ |
| **框架**       | Tauri 2 + React 19 + TypeScript 6                      |
| **UI**         | Ant Design 6 + TailwindCSS 4                           |
| **状态管理**   | Zustand 5                                              |
| **路由**       | React Router 7                                         |
| **国际化**     | i18next + react-i18next                                |
| **后端**       | Rust + SeaORM 2 + SQLite                               |
| **向量数据库** | sqlite-vec                                             |
| **代码编辑器** | Monaco Editor                                          |
| **图表**       | Mermaid + D2 + ECharts（CDN）                          |
| **终端**       | xterm.js 6                                             |
| **工作流**     | ReactFlow 11                                           |
| **图表渲染**   | @antv/infographic                                      |
| **图标**       | Iconify + Lucide                                       |
| **拖拽**       | @dnd-kit                                               |
| **构建**       | Vite 8 + npm                                           |
| **测试**       | Vitest + Playwright + cargo-nextest                    |
| **格式化**     | dprint (TS/JSON) + rustfmt                             |
| **Lint**       | TS: eslint + oxlint / Rust: clippy + cargo-deny        |
| **移动端**     | Tauri Android + iOS 原生构建                           |
| **桌面端**     | Windows (MSI) · macOS (DMG) · Linux (AppImage/deb/rpm) |

### 平台支持

| 平台    | 架构                                    |
| ------- | --------------------------------------- |
| Windows | x86_64, ARM64                           |
| macOS   | Apple Silicon (arm64), Intel (x86_64)   |
| Linux   | x86_64, ARM64                           |
| Android | arm64-v8a, armeabi-v7a, x86_64 (模拟器) |
| iOS     | arm64                                   |

### Rust 后端架构

后端组织为 Rust workspace，包含 **18 个** 专业化的 crates：

```
src-tauri/crates/
├── agent/            # AI 智能体核心（ReAct 引擎、协调、规划、深度研究、事实核查等）
├── core/             # 核心工具（数据库、RAG、加密、MCP、浏览器自动化、AST 索引等）
├── providers/        # 模型提供商适配器（OpenAI、Anthropic、Gemini、Ollama、OpenClaw 等）
├── runtime-core/     # 运行时抽象层（公共类型、trait 定义、配置）
├── runtime/          # 运行时服务（会话管理、MCP、终端、限流、Webhook、权限等）
├── rt-workflow/      # 工作流引擎（DAG 编排、节点执行器、调度器）
├── rt-messaging/     # 消息网关（钉钉/飞书/QQ/Slack/微信/WhatsApp/Telegram/Discord 集成）
├── rt-webhook/       # Webhook 服务器与分发
├── rt-dashboard/     # 仪表盘插件系统
├── rt-theme/         # 主题引擎
├── gateway/          # API 网关（HTTP 服务器、认证、路由、OpenAI 兼容接口）
├── tools/            # 工具系统（注册表、编排、流式输出、40+ 内置工具）
├── trajectory/       # 学习系统（记忆、技能、RL、用户画像、梦境整合）
├── telemetry/        # 遥测与分布式追踪
├── plugins/          # 插件系统（OpenClaw 兼容，npm 包安装）
├── prompt-guard/     # 提示词注入防护（L1-L4 多级检测与防御）
├── migration/        # 数据库迁移
├── npm/              # npm 包解析与注册表
└── code_engine/      # Candle 本地推理引擎（已弃用，功能已整合至 core）
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
npm run test          # Vitest watch
npm run test:run      # Vitest 单次运行

# E2E 测试
npm run test:e2e      # Playwright
npm run test:e2e:ui   # Playwright UI 模式

# Rust 后端测试
cd src-tauri && cargo nextest run   # cargo-nextest（快 2-3x）
cd src-tauri && cargo test          # 标准测试

# 类型检查
npm run typecheck     # TypeScript
cd src-tauri && cargo clippy -- -D warnings  # Rust

# 代码格式化
npm run format        # dprint
cd src-tauri && cargo fmt

# CI 全量检查
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
│   ├── pages/                   # 页面组件（18 个页面）
│   ├── stores/                  # Zustand 状态管理（62 个 store）
│   │   ├── domain/            # 核心业务状态（9 个）
│   │   ├── feature/           # 功能模块状态（44 个）
│   │   ├── devtools/          # 开发者工具状态（5 个）
│   │   └── shared/            # 共享状态（4 个）
│   ├── hooks/                   # React hooks
│   ├── lib/                     # 工具函数（含 Web Worker）
│   ├── types/                   # TypeScript 类型定义
│   ├── sdk/                     # SDK（含 Python SDK）
│   └── i18n/                    # 11 种语言翻译
│
├── src-tauri/                    # 后端源码 (Rust)
│   ├── crates/                  # Rust workspace（18 个 crates）
│   │   ├── agent/             # AI 智能体核心
│   │   ├── core/              # 数据库、加密、RAG、MCP
│   │   ├── providers/         # 模型提供商适配器
│   │   ├── runtime-core/      # 运行时抽象层
│   │   ├── runtime/           # 运行时服务
│   │   ├── rt-workflow/       # 工作流引擎
│   │   ├── rt-messaging/      # 消息网关
│   │   ├── rt-webhook/        # Webhook 服务器
│   │   ├── rt-dashboard/      # 仪表盘插件
│   │   ├── rt-theme/          # 主题引擎
│   │   ├── gateway/           # API 网关服务器
│   │   ├── tools/             # 工具系统
│   │   ├── trajectory/        # 记忆与学习
│   │   ├── telemetry/         # 追踪与指标
│   │   ├── plugins/           # 插件系统
│   │   ├── prompt-guard/      # 提示词注入防护
│   │   ├── migration/         # 数据库迁移
│   │   └── npm/               # npm 包解析
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
