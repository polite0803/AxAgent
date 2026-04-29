# Cherry Studio → AxAgent 能力移植计划

> 制定日期: 2026-04-29
> 基于: cherry-studio v1.9.4 全面分析 与 AxAgent 全面审查

---

## 1. 对比总结

| 维度 | Cherry Studio | AxAgent | 差距 |
|---|---|---|---|
| **语言/运行时** | TypeScript (Electron + Node.js) | TypeScript + Rust (Tauri) | AxAgent 技术栈更优 |
| **IPC 通道数** | 357 个 IPC | 530+ Tauri commands | 大致相当 |
| **Agent 类型** | ReAct + Claude Code + CherryClaw | ReAct + Research + Sub-agent + Team + Adversarial | AxAgent 更丰富 |
| **MCP 服务器** | 17 个内置 | 1 个（模块化框架） | **Cherry Studio 多 16 个** |
| **知识库/RAG** | EmbedJS（11 种文档格式）+ 向量搜索 | 文档解析 + sqlite-vec + 知识图谱 | AxAgent 有知识图谱优势 |
| **Memory** | 自有 MemoryService | 多命名空间 + Honcho/Mem0 | AxAgent 更先进 |
| **备份** | 本地 + WebDAV + **S3** | 本地 + WebDAV | **缺少 S3** |
| **OCR** | Tesseract + system-OCR (napi-rs) | 基于 LLM 的屏幕视觉 | **缺少传统 OCR** |
| **Channel 适配器** | Telegram + **WeChat** + **Feishu** + 统一 ChannelManager | Telegram + Discord | **缺少 WeChat/Feishu** |
| **文件操作** | 46 个通道（含 PDF 信息、文件监听、编码检测等） | 基础 CRUD | Cherry Studio 文件操作更丰富 |
| **OAuth 集成** | Anthropic + Copilot + CherryIN + VertexAI 完整 OAuth 流程 | PKCE OAuth（仅 MCP 用） | Cherry Studio 外部服务 OAuth 更全 |
| **选择助手** | 系统级文本选择工具栏 + 操作窗口 | 无 | **缺失** |
| **Obsidian 集成** | 仓库列表 + 文件浏览 | 无 | **缺失** |
| **局域网传输** | mDNS + TCP 二进制传输 | 无 | **缺失** |
| **AI 文件 API** | Gemini/Mistral/OpenAI 远程文件管理（上传/列表/删除/检索） | 仅基础 provider 适配器 | **缺失** |
| **迷你窗口** | 浮动迷你窗口（miniWindow）+ 搜索窗口 | 无 | UI 差异 |
| **Word 导出** | Markdown → Word 文档导出 | Markdown/PNG/JSON | **缺少 Word** |
| **Dify 集成** | DXT 文件处理 + Dify MCP 服务器 | 无 | **缺失** |
| **动漫/图片生成** | Paintings（AI 图片生成页面） | 有 Image Gen provider | 功能相当 |
| **翻译** | TranslatePage（独立翻译页面） | 无独立页面 | 功能差异 |
| **分析统计** | AnalyticsService（Token 用量追踪） | Gateway 中有用量统计 | 重叠 |
| **应用缓存管理** | 缓存大小计算 + 清理 | 无 | **缺失** |

---

## 2. 移植计划（7 个阶段，按优先级排列）

### 阶段 1：MCP 服务器补充（高影响，低耦合）

> 目标：将 Cherry Studio 的 17 个内置 MCP 服务器用 Rust 重写，放入 `axagent-runtime` crate

| 任务 | Cherry Studio 源文件 | AxAgent 目标位置 | 工作量 | 说明 |
|---|---|---|---|---|
| **1.1 Brave Search MCP** | `src/main/mcpServers/brave-search.ts` | `crates/runtime/src/mcp/servers/brave_search.rs` | 2-3 天 | AxAgent 已有 Tavily/Zhipu，补充 Brave 作为备选搜索引擎 |
| **1.2 Sequential Thinking MCP** | `src/main/mcpServers/sequentialthinking.ts` | `crates/runtime/src/mcp/servers/sequential_thinking.rs` | 2-3 天 | 结构化思维链（思考→修正→分支），可大幅提升 Agent 推理质量 |
| **1.3 Fetch MCP** | `src/main/mcpServers/fetch.ts` | `crates/runtime/src/mcp/servers/fetch.rs` | 1-2 天 | Agent 直接发起 HTTP 请求抓取网页/API |
| **1.4 Memory MCP** | `src/main/mcpServers/memory.ts` | `crates/runtime/src/mcp/servers/memory.rs` | 2-3 天 | 将 AxAgent 已有的记忆系统暴露为标准 MCP 工具 |
| **1.5 Skills MCP** | `src/main/mcpServers/skills.ts` | `crates/runtime/src/mcp/servers/skills.rs` | 2-3 天 | 将 AxAgent 技能系统暴露为 MCP 工具 |
| **1.6 Python MCP** | `src/main/mcpServers/python.ts` | `crates/runtime/src/mcp/servers/python.rs` | 1-2 天 | Python 脚本执行 MCP，可复用 Pyodide Worker |
| **1.7 Browser MCP** | `src/main/mcpServers/browser/` | `crates/runtime/src/mcp/servers/browser.rs` | 3-4 天 | 基于 Playwright 的浏览器自动化 MCP 封装（导航、截图、填充、提取） |
| **1.8 Dify Knowledge MCP** | `src/main/mcpServers/dify-knowledge.ts` | `crates/runtime/src/mcp/servers/dify_knowledge.rs` | 2-3 天 | 连接 Dify 知识库进行检索 |
| **1.9 Workspace Memory MCP** | `src/main/mcpServers/workspaceMemory.ts` | `crates/runtime/src/mcp/servers/workspace_memory.rs` | 1-2 天 | 工作区级短期记忆 |
| **1.10 Hub MCP Bridge** | `src/main/mcpServers/hub/` | `crates/runtime/src/mcp/servers/hub.rs` | 3-4 天 | MCP Hub 桥接器，连接外部 MCP 服务市场 |
| **1.11 Filesystem MCP 增强** | `src/main/mcpServers/filesystem/` | `crates/runtime/src/mcp/servers/filesystem.rs` | 2-3 天 | 增强现有文件系统 MCP：添加行编辑、文件搜索、编码检测 |
| **1.12 Assistant MCP** | `src/main/mcpServers/assistant.ts` | `crates/runtime/src/mcp/servers/assistant.rs` | 1-2 天 | 通用助手 MCP（常见问题、配置引导） |

> **阶段 1 总工作量：21-31 天**

**实施要点**：
- 所有 MCP 服务器实现 `McpServer` trait，支持 stdio JSON-RPC
- MCP 工具发现通过 Tool Registry 自动注册到 Agent 工具面板
- 每个 MCP 服务器独立可启用/禁用，配置持久化到数据库

---

### 阶段 2：Channel 适配器扩展

> 目标：补齐 WeChat/Feishu 渠道适配器，建立统一 Channel 管理框架

| 任务 | Cherry Studio 源文件 | AxAgent 目标位置 | 工作量 | 说明 |
|---|---|---|---|---|
| **2.1 WeChat Channel** | `WeChat_QrLogin` + channels 目录 | `crates/runtime/src/channels/wechat.rs` | 5-7 天 | 扫码登录、消息收发、会话管理 |
| **2.2 Feishu/Lark Channel** | `Feishu_QrLogin` + channels 目录 | `crates/runtime/src/channels/feishu.rs` | 5-7 天 | 飞书开放平台集成、消息处理 |
| **2.3 统一 ChannelManager** | `ChannelManager.ts` | `crates/runtime/src/channels/manager.rs` | 3-4 天 | 渠道生命周期：启用/禁用、状态监控、日志聚合、健康检查 |
| **2.4 Channel 前端 UI** | `src/renderer/src/pages/settings/` 相关 | `src/components/settings/` 补充 | 2-3 天 | 渠道配置面板（扫码区域、状态展示、消息日志） |

> **阶段 2 总工作量：15-21 天**

**实施要点**：
- 复用 AxAgent 已有的 Telegram/Discord trait 抽象
- `Channel` trait：`start()` / `stop()` / `send_message()` / `health_check()`
- 统一事件流：`ChannelEvent { channel_id, event_type, data }`
- 前端参考 Telegram 配置面板模式

---

### 阶段 3：存储与备份增强

| 任务 | Cherry Studio 源文件 | AxAgent 目标位置 | 工作量 | 说明 |
|---|---|---|---|---|
| **3.1 S3 备份** | `BackupManager.ts` + `S3Storage.ts` | `crates/core/src/backup/s3.rs` | 3-4 天 | S3 备份/恢复/列表/删除，使用 `aws-sdk-s3` Rust crate |
| **3.2 备份进度推送** | `BackupProgress`/`RestoreProgress` events | Tauri event 系统 | 1-2 天 | 备份进度实时推送到前端 |
| **3.3 文件操作增强** | `FileStorage.ts` (46 通道) | `crates/core/src/files/` | 3-5 天 | PDF 元信息提取、目录监听器（hotwatch）、文本编码自动检测、图片 base64 处理、文件批量上传 |
| **3.4 应用缓存管理** | `CacheService.ts` | `crates/core/src/cache.rs` | 1-2 天 | 缓存目录大小计算、一键清理、定时自动清理 |

> **阶段 3 总工作量：8-13 天**

---

### 阶段 4：OCR 服务

> 目标：补充传统 OCR 能力，与现有 LLM Vision 互补

| 任务 | Cherry Studio 源文件 | AxAgent 目标位置 | 工作量 | 说明 |
|---|---|---|---|---|
| **4.1 OCR 核心服务** | `ocr/OcrService.ts` | `crates/runtime/src/ocr/` | 3-4 天 | 集成 `@napi-rs/system-ocr` 或 `leptess`（Tesseract Rust 绑定） |
| **4.2 OCR MCP 工具** | 新建 | `crates/runtime/src/mcp/servers/ocr.rs` | 1-2 天 | 暴露 OCR 为 MCP 工具，供 Agent 调用 |
| **4.3 前端 OCR 面板** | `useOcr.ts` + 相关组件 | `src/components/settings/` | 1 天 | OCR 提供商选择、结果预览 |

> **阶段 4 总工作量：5-7 天**

**技术选择**：
- Windows: Windows OCR API（系统内置）
- macOS: Vision 框架
- Linux: Tesseract
- 备选：`leptess` crate 统一方案

---

### 阶段 5：外部服务与 OAuth 集成

| 任务 | Cherry Studio 源文件 | AxAgent 目标位置 | 工作量 | 说明 |
|---|---|---|---|---|
| **5.1 Obsidian Vault 集成** | `ObsidianVaultService.ts` | `crates/runtime/src/obsidian.rs` | 2-3 天 | 读取 Obsidian vault 列表和文件，作为知识源注入 Agent |
| **5.2 AI 文件 API** | `remotefile/`（OpenAI/Gemini/Mistral） | `crates/providers/src/files/` | 4-6 天 | Gemini File API、OpenAI File API、Mistral File API：上传/列表/检索/删除 |
| **5.3 Word 导出** | `ExportService.ts` | `crates/runtime/src/export.rs` | 2-3 天 | Markdown → Word 文档生成（考虑 `docx-rs` crate） |
| **5.4 Dify 集成** | `DxtService.ts` + `dify-knowledge` | `crates/runtime/src/dify.rs` | 2-3 天 | DXT 文件导入解析、Dify API 连接器 |
| **5.5 Anthropic OAuth** | `AnthropicService.ts` | `crates/runtime/src/oauth/anthropic.rs` | 2-3 天 | Anthropic OAuth 授权流程（PKCE + 本地回调） |
| **5.6 Copilot OAuth** | `CopilotService.ts` | `crates/runtime/src/oauth/copilot.rs` | 2-3 天 | GitHub Copilot Token 获取 |
| **5.7 VertexAI OAuth** | `VertexAIService.ts` | `crates/runtime/src/oauth/vertex.rs` | 2-3 天 | GCP ADC 认证 + Token 缓存 |

> **阶段 5 总工作量：16-24 天**

---

### 阶段 6：用户体验功能

| 任务 | Cherry Studio 源文件 | AxAgent 目标位置 | 工作量 | 说明 |
|---|---|---|---|---|
| **6.1 文本选择助手** | `SelectionService.ts` (17 通道) | `crates/runtime/src/selection.rs` + Tauri 平台 API | 5-8 天 | 系统级文本选择检测 + 浮动工具栏 + AI 操作 |
| **6.2 局域网传输** | `LocalTransferService.ts` + `lanTransfer/` | `crates/runtime/src/lan_transfer.rs` | 5-7 天 | mDNS 服务发现（`mdns-sd`）+ TCP 文件传输（`tokio::net`） |
| **6.3 迷你浮动窗口** | `miniWindow` 相关 | Tauri 多窗口 | 3-5 天 | 独立浮动聊天窗口（Tauri multiwebview） |

> **阶段 6 总工作量：13-20 天**

**技术风险**：
- 文本选择助手：Tauri 无内置文本选择 API，Windows 需 COM `IAccessible`，macOS 需 `AXUIElement`，Linux 需 AT-SPI
- 局域网传输：mDNS + serde 序列化二进制协议

---

### 阶段 7：Agent 能力补强

| 任务 | Cherry Studio 源文件 | AxAgent 目标位置 | 工作量 | 说明 |
|---|---|---|---|---|
| **7.1 Claude Code SDK 集成** | `claudecode/` 服务目录 | `crates/agent/src/claude_code/` | 3-5 天 | Claude-Code CLI 包装器 + 工具权限代理 |
| **7.2 CherryClaw 风格 Agent** | `cherryclaw/`（heartbeat/prompt/seeding） | `crates/agent/src/cherry_claw/` | 3-5 天 | 自定义心跳、prompt 模板、工作区播种 |
| **7.3 内置 Agent 模板** | `builtin-agents/cherry-assistant/` | `src/assets/agent_templates/` | 2-3 天 | 预配置 Agent（助手、代码审查、文档撰写） |
| **7.4 Scheduler + Agent 定时触发** | `SchedulerService.ts` + `TaskService.ts` | `crates/runtime/src/scheduler/` | 3-4 天 | Agent 定时任务：预定义 + CRON 表达式，复用 AxAgent 已有 Cron 引擎 |
| **7.5 Agent 技能沙箱** | `skills/SkillService.ts` + security | `crates/agent/src/skills/sandbox.rs` | 2-3 天 | 技能权限模型：路径白名单、网络访问控制、执行超时 |

> **阶段 7 总工作量：13-20 天**

---

## 3. 工作量汇总

| 阶段 | 主题 | 最小天数 | 最大天数 |
|---|---|---|---|
| 1 | MCP 服务器补充 | 21 | 31 |
| 2 | Channel 适配器 | 15 | 21 |
| 3 | 存储与备份 | 8 | 13 |
| 4 | OCR 服务 | 5 | 7 |
| 5 | 外部服务与 OAuth | 16 | 24 |
| 6 | 用户体验功能 | 13 | 20 |
| 7 | Agent 能力补强 | 13 | 20 |
| **合计** | | **91** | **136** |

> **人力估算**：
> - 单人串行：**13-20 周**
> - 3 人并行：**6-9 周**（阶段 1、2/3、5 可同时推进）

---

## 4. 依赖关系图

```
                     ┌─────────────────┐
                     │  阶段2 Channels │ ← 独立
                     └─────────────────┘
                     ┌─────────────────┐
                     │  阶段3 存储备份  │ ← 独立
                     └────────┬────────┘
                              │ (AI文件API依赖)
                     ┌────────▼────────┐
                     │  阶段5 外部集成  │
                     └────────┬────────┘
                              │ (OAuth依赖)
┌─────────────────┐  ┌────────▼────────┐
│  阶段1 MCP服务   │  │  阶段7 Agent    │
│  (独立)         │  │  能力补强        │
└─────────────────┘  └─────────────────┘
┌─────────────────┐
│  阶段4 OCR      │ ← 独立
└─────────────────┘
┌─────────────────┐
│  阶段6 UX功能   │ ← 独立（部分依赖平台API）
└─────────────────┘
```

**建议执行顺序**：阶段 1 + 阶段 3 → 阶段 2 + 阶段 5 → 阶段 4 + 阶段 6 + 阶段 7

---

## 5. 关键技术决策

| 决策点 | 方案 | 理由 |
|---|---|---|
| MCP 服务器语言 | Rust（`axagent-runtime`） | 复用现有 MCP 框架 stdio transport |
| WeChat/Feishu 实现 | 复用 Telegram trait 抽象 | 保持一致的 Channel 接口 |
| S3 备份 | `aws-sdk-s3` Rust crate | 原生性能，无需 Node.js bridge |
| OCR | `leptess`（跨平台）+ 平台原生 API 备选 | 最小依赖 |
| 局域网传输 | `mdns-sd` + `tokio::net::TcpStream` + serde | 纯 Rust 生态 |
| 文本选择助手 | 平台原生 Accessibility API | Tauri 无跨平台文本选择 API |
| Word 导出 | `docx-rs` crate | 纯 Rust，无 Node 依赖 |
| Dify | 独立 HTTP 客户端 + MCP 服务器 | 解耦，可选安装 |

---

## 6. 不建议移植的功能

| 功能 | 原因 |
|---|---|
| **Express REST API** | AxAgent 已有 Axum API Gateway，功能更全、性能更好 |
| **Store Sync（Redux）** | AxAgent 使用 Zustand，架构不同 |
| **Nutstore 集成** | 国内网盘服务，通用性低 |
| **CherryIN OAuth** | 内嵌服务（Cherry Studio 官方平台），与 AxAgent 无关 |
| **OVMS（设备端视觉模型）** | Windows + Intel CPU 专用，通用性低 |
| **CherryAI Signature** | Cherry Studio 平台签名，AxAgent 无此需求 |
| **Protocol Client（自定义 URL scheme）** | AxAgent 已有 Tauri deep link |
| **Python Service（独立服务）** | AxAgent 已有 Pyodide Worker + Sandbox 执行器 |

---

## 7. 风险评估

| 风险 | 影响阶段 | 缓解方案 |
|---|---|---|
| WeChat SDK 接口变更 | 2 | 使用稳定版 API，版本锁定 |
| 文本选择助手跨平台 API 差异大 | 6 | 先实现 Windows，macOS/Linux 增量支持 |
| Claude Code SDK 无 Rust 绑定 | 7 | CLI 子进程 + JSON-RPC 通信 |
| mDNS 跨平台兼容性 | 6 | mdns-sd crate 已支持 Win/Mac/Linux |
| Tauri 多窗口 API 稳定性 | 6 | 使用 Tauri 2.x stable 多窗口 API |
