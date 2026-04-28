# AxAgent vs Hermes Agent 差距分析与追赶方案

> 分析日期：2026-04-28
> 对比基准：[NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent) v0.11.0 (2026-04-23)
> 数据来源：Hermes Agent README、AGENTS.md、RELEASE_v0.11.0.md + AxAgent 代码库实际审查
> 用途：指导后续 AxAgent 追赶 Hermes Agent 的优先级排序和方案设计

---

## 一、Hermes Agent 概况

| 指标 | 数值 |
|------|------|
| Stars | 121k |
| Forks | 18k |
| Commits | 6,329 |
| Contributors | 290+ |
| 语言 | Python 87.8% + TypeScript 8.8% |
| 最新版本 | v0.11.0 (v2026.4.23) |
| 架构 | CLI-first (Python) → Ink TUI (React) → Web Dashboard → 17 平台 Gateway |
| 许可证 | MIT |

---

## 二、功能差距矩阵

### 2.1 严重缺失（P0）— 架构级差距

| 功能 | Hermes | AxAgent | 差距等级 | 追赶建议 |
|------|--------|---------|---------|---------|
| **消息平台网关** | 17 平台 (Telegram/Discord/Slack/WhatsApp/Signal/WeChat/QQBot/DingTalk/Feishu/BlueBubbles/Matrix/Mattermost/Email/SMS/Webhook/HomeAssistant/API Server) | 无 | 🔴 重大 | Phase 1：实现 Telegram + Discord + API Server；Phase 2：扩展其余平台 |
| **多环境终端后端** | 6 种 (local/Docker/SSH/Daytona/Singularity/Modal) | 仅 local PTY | 🔴 重大 | 实现 Docker + SSH 后端 |
| **测试覆盖** | ~15k tests / 700+ 文件 | ~141 测试 | 🔴 重大 | 核心模块 >60%，关键路径 E2E 全覆盖 |
| **Prompt 缓存感知** | 完整缓存保护机制，deferred invalidation | 未实现 | 🔴 重大 | 实现 system prompt 不变性保护 + deferred reload |

### 2.2 重要缺失（P1）— 核心能力差距

| 功能 | Hermes | AxAgent | 差距等级 | 追赶建议 |
|------|--------|---------|---------|---------|
| **插件系统深度 Hooks** | pre/post_tool_call、pre/post_llm_call、transform_tool_result、transform_terminal_output、veto、register_command、dispatch_tool | 仅基础 hooks | 🟡 重要 | 扩展插件 hooks 体系 |
| **Cron 定时任务** | 自然语言计划，多平台分发，per-job toolsets | 无 | 🟡 重要 | 实现 Rust cron 引擎 + 前端管理 UI |
| **TUI (终端 UI)** | React/Ink 重写，流式输出，OSC-52 剪贴板，自动补全，虚拟化渲染 | xterm.js web terminal | 🟡 重要 | 可保持 xterm.js，增强流式体验和补全 |
| **皮肤/主题引擎** | 数据驱动 CLI theming，4 内置皮肤 + 自定义 YAML | 仅前端 CSS 主题 | 🟡 重要 | 终端/前端统一主题系统 |
| **多实例 Profiles** | 完全隔离 profile：配置/密钥/会话/技能/网关 | 无 | 🟡 重要 | 实现 profile 隔离机制 |
| **Shell 生命周期 Hooks** | shell 脚本注册为 hook callback | 无 | 🟡 重要 | 支持 shell/python 脚本作为 hooks |
| **上下文文件 (AGENTS.md)** | 项目级上下文注入，自动发现 AGENTS.md/CLAUDE.md | .axagent/memory.md (仅 Markdown) | 🟡 重要 | 兼容 AGENTS.md 格式 + 自动发现 |
| **/steer 中途干预** | 运行时代理方向调整，不中断 turn、不破坏缓存 | 无 | 🟡 重要 | 实现运行时 nudge 机制 |
| **RL 训练环境** | Atropos 集成 + 轨迹压缩用于下一代理模型训练 | RL 优化器（无训练设施） | 🟡 重要 | 集成 Atropos 或自有训练管道 |
| **Provider Transport ABC** | 可插拔传输层抽象 (Anthropic/ChatCompletions/Responses/Bedrock) | ProviderAdapter trait | 🟡 重要 | 抽象传输层为独立 trait |
| **代理中断响应** | 精细中断控制 + 中断后自动恢复 | 基础 cancel/pause | 🟡 重要 | 增强中断粒度和恢复能力 |

### 2.3 次要缺失（P2）— 差异化追赶

| 功能 | Hermes | AxAgent | 差距等级 | 追赶建议 |
|------|--------|---------|---------|---------|
| **记忆提供者插件** | 8 种后端 (honcho/mem0/supermemory/byterover/hindsight/holographic/openviking/retaindb) | 自有记忆系统 | 🟢 次要 | 实现可插拔记忆后端 |
| **Skills Hub 兼容** | agentskills.io 开放标准，community hub | 自有技能系统 | 🟢 次要 | 兼容 agentskills.io 格式 |
| **ACP 集成** | VS Code / Zed / JetBrains 原生插件 | LSP 集成已实现 | 🟢 次要 | 评估需求，可做 VSCode 插件 |
| **语音/STT/TTS 多提供者** | Gemini/xAI/KittenTTS/Native 等 | 基础语音功能 | 🟢 次要 | 扩展语音提供者 |
| **Webhook 直连推送** | 零 LLM 推送通知，事件流 | 无 | 🟢 次要 | 实现 Webhook server |
| **Dashboard 插件系统** | 浏览器可扩展自定义标签/视图 | 前端组件无插件体系 | 🟢 次要 | 实现 Dashboard 插件系统 |
| **委托子代理层级** | orchestrator 角色 + configurable max_spawn_depth | agent_orchestrator 较简单 | 🟢 次要 | 增加子代理层级 |
| **Busy-session ack** | 代理运行时用户消息回复繁忙状态 | 无 | 🟢 次要 | 实现 busy state 反馈 |
| **动态 Shell 补全** | bash/zsh/fish 自动生成补全脚本 | 无 | 🟢 次要 | 生成 shell 补全脚本 |
| **会话 FTS5 搜索** | 全文搜索历史对话 + LLM 摘要 | 基础搜索 | 🟢 次要 | 增强 FTS5 搜索 |
| **Batch 轨迹生成** | 并行批量处理 + 轨迹压缩 | 基础轨迹记录 | 🟢 次要 | 增强批量处理和压缩 |
| **Docker Compose 支持** | docker-compose.yml + setup script | 无 | 🟢 次要 | 提供 Docker Compose 部署 |

---

## 三、AxAgent 相对优势（需保持和强化）

| 优势 | 对比 Hermes | 强化建议 |
|------|-----------|---------|
| **桌面 GUI 应用** | Hermes 无原生 Windows 支持 | 保持 Tauri 跨平台优势 |
| **Rust 性能/安全** | Python 运行时开销大 | 持续优化编译大小和启动速度 |
| **知识库/RAG 深度整合** | Hermes 无 RAG 能力 | 实现 RAG-aware Agent 工具选择 |
| **LSP 原生集成** | Hermes 无 LSP | 完善 LSP 诊断和代码编辑体验 |
| **工作流引擎** | Hermes 无工作流 | WorkflowEngine ↔ WorkEngine 完闭 |
| **国际化 12 语言** | 仅英文/中文 | 持续完善翻译质量 |
| **离线本地模型** | 依赖云端 API | 优化 Ollama 集成体验 |
| **API 网关** | 基础 API Server | 添加更多 API 兼容模式 |
| **MCP 协议** | 均有 | 构建 MCP 工具市场 |

---

## 四、分阶段追赶方案

整体分 **5 个阶段**，从 P0 核心差距到 P2 差异化能力，预计总工期 **120-150 人天**。

---

### Phase 1：核心基础设施补齐（P0 — 30 人天）

> 目标：补齐与 Hermes 的架构级差距，为后续所有能力打下基础

#### 1.1 消息平台网关（12 人天）

**目标**：实现 Telegram + Discord + API Server 三个平台适配器

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/gateway/mod.rs` | 新增 | 网关核心：统一消息接收/分发/会话管理 |
| `runtime/src/gateway/platforms/mod.rs` | 新增 | 平台适配器 trait 定义 |
| `runtime/src/gateway/platforms/telegram.rs` | 新增 | Telegram Bot API 适配器（长轮询/Webhook） |
| `runtime/src/gateway/platforms/discord.rs` | 新增 | Discord Bot 适配器（Gateway Intents） |
| `runtime/src/gateway/platforms/api_server.rs` | 新增 | REST API Server（/v1/chat/completions 兼容） |
| `runtime/src/gateway/session_router.rs` | 新增 | 跨平台会话路由，支持 session_key -> agent 映射 |
| `runtime/src/gateway/platform_config.rs` | 新增 | 平台配置管理（API keys, webhook URLs, 权限） |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/settings/GatewayConfigPanel.tsx` | 新增 | 网关配置界面：平台选择/API key/回调 URL |
| `components/settings/PlatformStatusCard.tsx` | 新增 | 各平台连接状态监控卡片 |
| `components/chat/GatewaySessionBadge.tsx` | 新增 | 聊天中显示消息来源平台标识 |
| `stores/feature/gatewayStore.ts` | 新增 | 网关状态管理 store |

**验收标准**：
- [ ] Telegram Bot 可接收消息并调用 Agent 返回回复
- [ ] Discord Bot 可接收消息并返回回复
- [ ] API Server 提供 `/v1/chat/completions` 兼容端点
- [ ] 跨平台会话路由正确，同一用户不同平台消息聚合到同一 session
- [ ] 网关停止/启动不影响已建立的平台连接

#### 1.2 Prompt 缓存感知机制（6 人天）

**目标**：保护 system prompt 不变性，降低 API 调用成本

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/prompt_cache.rs` | 新增 | 缓存管理器：track system msg hash、detect invalidation |
| `runtime/src/cache_guard.rs` | 新增 | 缓存守卫：拦截可能破坏缓存的操作 |
| `agent/src/coordinator.rs` | 修改 | `execute()` 开始前检查缓存有效性 |
| `agent/src/provider_adapter.rs` | 修改 | 发送请求时注入缓存断点标记 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/chat/CacheIndicator.tsx` | 新增 | 显示当前缓存状态（valid/invalid/cost-saved） |
| `components/settings/CacheConfigPanel.tsx` | 新增 | 缓存策略配置 UI |

**验收标准**：
- [ ] 对话中修改 skills/tools 时，变更延迟到下一 session 生效
- [ ] `--now` 标志可强制立即生效
- [ ] 缓存命中时 API 调用输入 token 显著减少
- [ ] `/model` 切换正确触发缓存失效

#### 1.3 测试基础建设（12 人天）

**目标**：核心模块覆盖率 >60%，关键路径 E2E 全覆盖

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/agent/tests/` | 新增 | Agent 模块测试（coordinator, react_engine, planner） |
| `crates/runtime/tests/` | 新增 | Runtime 模块测试（git_tools, collaboration, benchmarks） |
| `crates/core/tests/` | 新增 | Core 模块测试（builtin_tools registry dispatch） |
| `tests/e2e/` | 新增 | E2E 测试（Playwright）：chat flow、agent execution |

**验收标准**：
- [ ] 每个 crate 至少 `tests/` 目录 + 5 个基础测试文件
- [ ] E2E 覆盖：启动 → 对话 → 工具调用 → 结果返回 完整流程
- [ ] `cargo test` 全覆盖通过

---

### Phase 2：多环境与扩展能力（P0+P1 — 28 人天）

> 目标：补齐运行环境多样性，大幅提升可用性

#### 2.1 多环境终端后端（10 人天）

**目标**：支持 Docker + SSH 两种新终端后端

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/terminal/docker_backend.rs` | 新增 | Docker 后端：创建容器、执行命令、文件同步 |
| `runtime/src/terminal/ssh_backend.rs` | 新增 | SSH 后端：连接管理、PTY 分配、命令执行 |
| `runtime/src/terminal/backend_trait.rs` | 新增 | 统一 TerminalBackend trait |
| `runtime/src/pty.rs` | 修改 | 重构为 backend 驱动，支持动态切换 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/terminal/TerminalBackendSelector.tsx` | 新增 | 终端后端选择器（local/docker/ssh） |
| `components/terminal/DockerConfigModal.tsx` | 新增 | Docker 连接配置弹窗 |
| `components/terminal/SshConfigModal.tsx` | 新增 | SSH 连接配置弹窗 |

**验收标准**：
- [ ] Docker 后端可创建临时容器并执行命令
- [ ] SSH 后端可连接远程服务器并分配 PTY
- [ ] 三种后端（local/docker/ssh）可在终端中即时切换
- [ ] 文件在不同后端之间可正确同步

#### 2.2 插件 Hooks 体系扩展（8 人天）

**目标**：支持 pre/post_tool_call、transform、veto、register_command 等高级 hooks

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/plugin_hooks.rs` | 新增 | 统一 Hook trait：PreToolCall、PostToolCall、PreLlmCall 等 |
| `runtime/src/hook_chain.rs` | 新增 | Hook 链式执行：支持 veto、transform、observe |
| `core/src/builtin_tools.rs` | 修改 | dispatch 前调用 pre_tool_call hook |
| `core/src/builtin_tools.rs` | 修改 | dispatch 后调用 post_tool_call + transform_tool_result |
| `agent/src/coordinator.rs` | 修改 | LLM 调用前后插入 pre/post_llm_call hooks |
| `runtime/src/transform_pipeline.rs` | 新增 | 工具输出/终端输出 transform pipeline |

**验收标准**：
- [ ] `pre_tool_call` 可 veto 工具执行
- [ ] `post_tool_call` 可在工具执行后触发回调
- [ ] `transform_tool_result` 可改写工具输出
- [ ] `pre_llm_call` / `post_llm_call` 可在 LLM 调用前后注入逻辑

#### 2.3 Cron 定时任务系统（6 人天）

**目标**：支持自然语言配置定时任务，自动执行并通知

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/cron/mod.rs` | 新增 | Cron 引擎：cron 表达式解析 + 调度 |
| `runtime/src/cron/scheduler.rs` | 新增 | 调度器：tokio timer + job queue |
| `runtime/src/cron/job_store.rs` | 新增 | 定时任务持久化存储 |
| `runtime/src/cron/executor.rs` | 新增 | 任务执行器：调度 agent 执行任务 |
| `core/src/builtin_tools.rs` | 修改 | 添加 `cron_add` / `cron_list` / `cron_delete` 工具 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/settings/CronManager.tsx` | 新增 | 定时任务管理界面 |
| `components/chat/CronResultMessage.tsx` | 新增 | 定时任务执行结果展示 |

**验收标准**：
- [ ] 支持标准 cron 表达式配置定时任务
- [ ] 支持自然语言创建任务（"每天早上 9 点检查邮件"）
- [ ] 任务执行结果通过消息平台推送（telegram/discord/web）
- [ ] 支持 per-job enabled_toolsets 配置

#### 2.4 Provider Transport 抽象层（4 人天）

**目标**：将 provider 适配抽象为可插拔的传输层

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `providers/src/transport.rs` | 新增 | Transport trait 定义 |
| `providers/src/transport/chat_completions.rs` | 新增 | OpenAI Chat Completions transport |
| `providers/src/transport/anthropic.rs` | 新增 | Anthropic Messages API transport |
| `providers/src/transport/responses.rs` | 新增 | OpenAI Responses API transport |
| `providers/src/lib.rs` | 修改 | 重构 ProviderAdapter 为 transport 驱动 |

**验收标准**：
- [ ] 三种 transport 可独立测试
- [ ] 添加新 provider 只需实现 Transport trait
- [ ] 向后兼容现有 ProviderAdapter 接口

---

### Phase 3：智能体深度能力（P1 — 24 人天）

> 目标：提升代理智能度，接近 Hermes 的用户体验

#### 3.1 多实例 Profiles（6 人天）

**目标**：支持完全隔离的多 profile，每 profile 独立配置/密钥/会话/技能

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/profile.rs` | 新增 | Profile 管理器：创建/切换/删除/列出 |
| `runtime/src/profile_manager.rs` | 新增 | Profile 数据隔离层：config/db/sessions/skills 路径映射 |
| `core/src/unified_config.rs` | 修改 | 支持 `~/.axagent/profiles/<name>/` 路径 |
| `core/src/db.rs` | 修改 | 每个 profile 独立 SQLite 数据库 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/settings/ProfileSelector.tsx` | 新增 | Profile 切换下拉菜单 |
| `components/settings/ProfileManager.tsx` | 新增 | 创建/删除 profile 管理界面 |

**验收标准**：
- [ ] 创建新 profile 完全隔离配置、会话历史、技能
- [ ] 启动时通过命令行 `--profile <name>` 或 UI 切换
- [ ] 不同 profile 的 API 密钥完全独立

#### 3.2 Shell 生命周期 Hooks（3 人天）

**目标**：支持 shell/python 脚本注册为生命周期钩子

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/shell_hooks.rs` | 新增 | Shell hook 执行器：subprocess 调用 + stdin/stdout 传递 |
| `runtime/src/hook_config.rs` | 新增 | Hook 配置加载：`~/.axagent/hooks/` 目录扫描 |
| `runtime/src/lib.rs` | 修改 | 注册 shell hooks 模块 |

**验收标准**：
- [ ] `~/.axagent/hooks/pre_tool_call.sh` 在每次工具调用前执行
- [ ] hook 脚本通过 stdin 接收 JSON 上下文，stdout 返回结果
- [ ] hook 脚本返回 `{"veto": true}` 可阻止工具执行

#### 3.3 /steer 中途干预机制（3 人天）

**目标**：运行时注入方向调整指令，不中断 turn、不破坏缓存

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `agent/src/steer_manager.rs` | 新增 | Steer 管理器：接收 nudge 消息，在下次 tool_call 后注入 |
| `agent/src/coordinator.rs` | 修改 | execute loop 中检查 steer queue |
| `runtime/src/commands.rs` | 修改 | 添加 `/steer <prompt>` 命令 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/chat/SteerInput.tsx` | 新增 | 运行时 nudge 输入框 |

**验收标准**：
- [ ] 代理运行中输入 `/steer <instruction>`，代理在下个工具调用后看到该指令
- [ ] steer 不中断当前 turn，不破坏 prompt cache
- [ ] steer 消息在 system prompt 中标注为临时注入

#### 3.4 AGENTS.md 上下文文件兼容（4 人天）

**目标**：兼容 Hermes 的 AGENTS.md / CLAUDE.md 项目上下文文件格式

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `agent/src/context_files.rs` | 新增 | 上下文文件解析器：自动发现 + 合并 AGENTS.md/CLAUDE.md/.axagent/memory.md |
| `agent/src/coordinator.rs` | 修改 | 构建 system prompt 时加载上下文文件 |
| `runtime/src/git_context.rs` | 修改 | 扩展 git 上下文包含项目级文件发现 |

**验收标准**：
- [ ] 项目根目录的 AGENTS.md 自动注入到 system prompt
- [ ] CLAUDE.md 格式兼容读取
- [ ] 多层目录（root + subdir）上下文文件叠加
- [ ] `/context reload` 命令重新加载

#### 3.5 RL 训练环境集成（5 人天）

**目标**：集成 Atropos 或自有训练管道，支持从轨迹数据训练改进模型

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `trajectory/src/training_env.rs` | 新增 | 训练环境抽象：任务定义、奖励计算、评估 |
| `trajectory/src/trajectory_compressor.rs` | 新增 | 轨迹压缩器：生成训练格式数据 |
| `trajectory/src/rl_trainer.rs` | 新增 | RL 训练协调器 |
| `agent/src/rl_optimizer/` | 修改 | 对接训练管道 |

**验收标准**：
- [ ] 轨迹数据可导出为标准训练格式（JSONL）
- [ ] 压缩后的轨迹保留关键决策点
- [ ] 奖励信号计算管线可用

#### 3.6 代理中断精细控制（3 人天）

**目标**：增强中断粒度和自动恢复能力

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `agent/src/interrupt.rs` | 新增 | 中断管理器：支持 soft/hard/graceful 三种中断级别 |
| `agent/src/coordinator.rs` | 修改 | 中断状态管理 + 自动恢复逻辑 |
| `agent/src/recovery_strategies.rs` | 修改 | 连接中断后自动恢复策略 |

**验收标准**：
- [ ] `/stop` 不重置 session（只停止当前 turn）
- [ ] 网关重启后自动恢复未完成的代理任务
- [ ] 中断响应延迟 < 1s

---

### Phase 4：生态与平台扩展（P2 — 22 人天）

> 目标：构建开放生态，扩展到更多平台和场景

#### 4.1 记忆提供者插件（5 人天）

**目标**：支持可插拔记忆后端，兼容 honcho/mem0 等

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `trajectory/src/memory_provider.rs` | 新增 | MemoryProvider trait 定义 |
| `trajectory/src/memory/honcho_provider.rs` | 新增 | Honcho dialectic memory 后端 |
| `trajectory/src/memory/mem0_provider.rs` | 新增 | Mem0 memory 后端 |
| `trajectory/src/memory_manager.rs` | 新增 | 统一记忆管理器，按配置选择后端 |

**验收标准**：
- [ ] 可在配置文件中切换记忆后端（`memory.provider: "honcho"`）
- [ ] MemoryProvider trait 支持 sync_turn/prefetch/shutdown 生命周期
- [ ] 第三方可实现自定义 memory provider plugin

#### 4.2 Skills Hub 兼容（4 人天）

**目标**：兼容 agentskills.io 开放标准，可导入/导出社区技能

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `trajectory/src/skills_hub_client.rs` | 新增 | Skills Hub API 客户端：搜索/下载/发布技能 |
| `trajectory/src/skills_hub_adapter.rs` | 新增 | 格式转换器：HERMES SKILL.md ↔ AxAgent skill format |
| `trajectory/src/skill_manager.rs` | 修改 | 添加 `skills install official/<category>/<skill>` 命令 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/chat/SkillsHubBrowser.tsx` | 新增 | Skills Hub 浏览/搜索/安装界面 |

**验收标准**：
- [ ] 可从 agentskills.io 搜索和安装技能
- [ ] 安装的技能自动转换为 AxAgent 格式
- [ ] 支持 `hermes skills install` 命令格式

#### 4.3 Dashboard 插件系统（5 人天）

**目标**：Web Dashboard 支持第三方插件扩展自定义标签和视图

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/dashboard_plugin.rs` | 新增 | Dashboard 插件协议定义 |
| `runtime/src/dashboard_registry.rs` | 新增 | 插件注册表：扫描 + 加载 + 生命周期管理 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/dashboard/PluginSlot.tsx` | 新增 | 插件渲染槽位组件 |
| `components/dashboard/DashboardShell.tsx` | 新增 | 可扩展 Dashboard 容器，支持动态加载插件面板 |
| `components/dashboard/PluginGallery.tsx` | 新增 | 插件浏览和管理界面 |

**验收标准**：
- [ ] 第三方可开发 Dashboard 插件并注册
- [ ] 插件提供自定义标签页和侧边栏面板
- [ ] 插件热加载/卸载不影响主应用稳定性

#### 4.4 Webhook 直连推送（4 人天）

**目标**：支持 Webhook 订阅 + 零 LLM 推送通知 + 事件流

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/webhook_server.rs` | 新增 | Webhook HTTP server：订阅注册 + 事件推送 |
| `runtime/src/webhook_subscription.rs` | 新增 | 订阅管理器：URL 注册、验证、重试 |
| `runtime/src/webhook_dispatcher.rs` | 新增 | 事件分发器：tool_complete / agent_error / session_end 等 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/settings/WebhookConfig.tsx` | 新增 | Webhook 订阅配置界面 |

**验收标准**：
- [ ] 支持注册外部 Webhook URL
- [ ] 工具执行完成/代理错误/会话结束等事件自动推送
- [ ] 支持 direct-delivery 模式（零 LLM 消耗）

#### 4.5 消息平台扩展（4 人天）

**目标**：新增 WeChat + Slack + WhatsApp 三个平台

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/gateway/platforms/slack.rs` | 新增 | Slack App 适配器 |
| `runtime/src/gateway/platforms/whatsapp.rs` | 新增 | WhatsApp Business API 适配器 |

**验收标准**：
- [ ] Slack 可接收频道消息并回复
- [ ] WhatsApp 可接收消息并回复

---

### Phase 5：体验极致打磨（P2 — 16 人天）

> 目标：打磨 UI 体验，接近 Hermes 的成熟度

#### 5.1 TUI 终端增强（4 人天）

**目标**：增强 xterm.js 终端体验，接近 Ink TUI

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/terminal/XtermEnhancement.tsx` | 新增 | 虚拟化渲染、OSC-52 剪贴板 |
| `components/terminal/SlashCompleter.tsx` | 新增 | 斜杠命令自动补全 |
| `components/terminal/PathCompleter.tsx` | 新增 | 文件路径 Tab 补全 |
| `components/terminal/StatusBarWidget.tsx` | 新增 | 状态栏：git branch、计时器、token 计数 |

**验收标准**：
- [ ] 长输出虚拟化渲染
- [ ] `/` 命令自动补全
- [ ] 路径 Tab 补全
- [ ] 状态栏实时显示 git 分支和耗时

#### 5.2 主题/皮肤引擎（4 人天）

**目标**：统一终端和前端主题系统

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `core/src/theme_engine.rs` | 新增 | 主题引擎：YAML 加载、变量解析、皮肤切换 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/settings/ThemeManager.tsx` | 新增 | 主题浏览器和管理界面 |
| `stores/feature/themeStore.ts` | 新增 | 主题状态 store + 运行时切换 |

**验收标准**：
- [ ] 4 个内置主题（default/ares/mono/slate）
- [ ] 用户自定义 YAML 皮肤，`~/.axagent/skins/`
- [ ] 终端和 Dashboard 主题实时同步

#### 5.3 会话全文搜索（3 人天）

**目标**：FTS5 全文搜索历史对话，LLM 摘要辅助

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `core/src/session_search.rs` | 新增 | FTS5 全文搜索引擎 |
| `core/src/db.rs` | 修改 | 添加 FTS5 虚拟表 + 全文索引 |

**前端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `components/chat/SessionSearchPanel.tsx` | 新增 | 会话搜索面板 + 结果预览 |

**验收标准**：
- [ ] 支持关键词全文搜索历史对话
- [ ] LLM 辅助搜索结果总结
- [ ] 搜索结果可跳转到原始会话

#### 5.4 动态 Shell 补全（2 人天）

**目标**：为 bash/zsh/fish 生成补全脚本

**后端工作**：
| 文件 | 操作 | 说明 |
|------|------|------|
| `runtime/src/shell_completion.rs` | 新增 | 补全脚本生成器：bash/zsh/fish 格式 |

**验收标准**：
- [ ] `hermes completion bash` 生成 bash 补全脚本
- [ ] 补全覆盖所有 CLI 命令和子命令

#### 5.5 其他细节打磨（3 人天）

| 功能 | 说明 |
|------|------|
| Busy-session ack | 代理运行时用户消息回复繁忙状态 |
| Streaming cursor 过滤 | 防止流式输出光标符号污染其他平台 |
| Activity heartbeats | 防止网关空闲超时 |
| ESC 取消 secret/sudo 提示 | 更清晰的跳过消息 |
| Docker Compose 部署 | `docker-compose.yml` + 一键部署脚本 |

---

## 五、工作量汇总

| Phase | 核心内容 | 人天 | 关键交付物 |
|-------|---------|------|-----------|
| Phase 1 | 消息平台网关 + Prompt 缓存 + 测试 | 30 | 3 平台网关、缓存保护、>500 测试 |
| Phase 2 | 多环境终端 + 插件 Hooks + Cron + Transport | 28 | Docker/SSH 后端、Hook 体系、定时任务 |
| Phase 3 | Profiles + Steer + 上下文文件 + RL + 中断 | 24 | 多 profile、中途干预、训练管道 |
| Phase 4 | 记忆插件 + Skills Hub + Dashboard 插件 + Webhook | 22 | 可插拔记忆、外部技能、Webhook 推送 |
| Phase 5 | TUI 增强 + 主题 + 搜索 + 补全 + 细节 | 16 | 终端体验、皮肤系统、全文搜索 |
| **总计** | | **120** | |

---

## 六、Hermes Agent 独有核心功能详解（参考实现）

### 6.1 消息平台网关架构

```
hermes gateway
  ├── gateway/run.py          # 网关主循环
  ├── gateway/session.py      # 会话管理
  └── gateway/platforms/      # 17 平台适配器
      ├── telegram.py          # Telegram Bot API
      ├── discord.py           # Discord Bot
      ├── slack.py             # Slack App
      ├── whatsapp.py          # WhatsApp Business API
      ├── signal.py            # Signal CLI bridge
      ├── matrix.py            # Matrix 协议
      ├── mattermost.py        # Mattermost
      ├── email.py             # SMTP/IMAP
      ├── sms.py               # Twilio SMS
      ├── dingtalk.py          # 钉钉
      ├── wecom.py             # 企业微信
      ├── weixin.py            # 微信
      ├── feishu.py            # 飞书
      ├── qqbot.py             # QQ 机器人
      ├── bluebubbles.py       # iMessage
      ├── homeassistant.py     # Home Assistant
      └── webhook.py           # Webhook
```

### 6.2 插件 Hooks 体系

```python
# Hermes 插件 hooks
class PluginHooks:
    pre_tool_call(tool_name, args) -> bool | None   # 可 veto 工具执行
    post_tool_call(tool_name, args, result) -> None
    pre_llm_call(messages, tools) -> None
    post_llm_call(response) -> None
    on_session_start(session_id) -> None
    on_session_end(session_id) -> None
    transform_tool_result(tool_name, result) -> str
    transform_terminal_output(output) -> str
    register_command() -> CommandDef
    dispatch_tool(tool_name, args) -> str
```

### 6.3 定时任务系统

```yaml
# Hermes cron 配置
cron:
  jobs:
    - schedule: "0 9 * * *"
      prompt: "Generate a daily summary of my inbox"
      platform: "telegram"
      enabled_toolsets: ["gmail"]
    - schedule: "0 0 * * 0"
      prompt: "Run weekly code audit on my repos"
      platform: "discord"
```

### 6.4 Prompt 缓存策略

Hermes 的核心缓存保护规则：

1. **不改变历史 context** — 对话中不修改 system message
2. **不改变 toolsets** — 工具集在 turn 内不可变
3. **不重载记忆** — memory 不在 mid-conversation 重载
4. **延迟生效** — slash command 状态变更默认延迟到下一 session，`--now` 强制立即生效
5. **唯一允许的 context 变更** — context compression

---

## 七、总结

### 最大差距（需优先追赶）

1. **消息平台网关** — 完全缺失，Hermes 有 17 个平台适配器
2. **多环境终端** — 仅 local，Hermes 支持 Docker/SSH/Modal 等
3. **测试覆盖** — 141 vs ~15,000 测试
4. **Prompt 缓存** — 无缓存保护机制
5. **插件 Hooks** — 基础 vs 10+ hook types
6. **Cron 定时任务** — 完全缺失

### AxAgent 护城河（需保持）

1. **桌面原生 GUI** — Hermes 无 Windows 桌面端
2. **Rust 性能** — 编译型语言的性能/安全优势
3. **知识库/RAG** — Hermes 无此能力
4. **LSP 集成** — 原生代码智能
5. **工作流引擎** — 独特的可视化工作流
6. **国际化** — 12 语言支持
