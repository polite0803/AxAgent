# AxAgent 项目架构分析报告

> 生成日期: 2026-07-01
> 分析范围: D:\OneManager\AxAgent

---

## 1. 整体目录结构

### 1.1 项目根目录

```
AxAgent/
├── index.html                  # 入口 HTML
├── package.json                # 前端依赖与脚本
├── settings.json               # 用户/项目配置文件
├── AGENTS.md / CLAUDE.md       # Agent 使用说明
├── src-tauri/                  # Rust 后端 (Tauri + 多 crate 工作空间)
├── src/                        # React 前端 (TypeScript)
├── .codeartsdoer/              # IDE 插件/规则/技能配置
│   ├── agents/                 # CodeArtsDoer 代理定义
│   ├── mcp/                    # MCP 配置
│   ├── rule/                   # 规则文件
│   ├── skills/                 # 本地技能定义
│   └── node_modules/           # @opencode-ai/plugin, @opencode-ai/sdk
├── .github/                    # CI/CD 工作流
├── .next/                      # Next.js 构建缓存
└── ...
```

### 1.2 Rust 后端 crate 布局 (src-tauri/crates/)

| Crate             | 职责                                                                          |
| ----------------- | ----------------------------------------------------------------------------- |
| `harness`         | 底层契约层 — trait 接口、DTO、常量、错误类型。零业务逻辑，所有实现的依赖锚点  |
| `core`            | 核心模块聚合 — re-export harness + 各子系统（缓存、加密、存储、搜索、RAG 等） |
| `entities`        | SeaORM 实体定义（数据库 Schema）                                              |
| `dao`             | 数据访问层 — repository 模式 + 数据库迁移                                     |
| `plugins`         | 插件系统核心 — 发现、加载、生命周期管理、MCP 启动、技能安装、Agent 注册       |
| `tools`           | 工具系统 — 内置工具实现、工具注册表、权限校验、插件 SDK、批处理               |
| `agent`           | Agent 引擎 — 协调器、ReAct 引擎、任务分解、思维链、轨迹记录、RL 优化、反思    |
| `runtime-core`    | 运行时核心原语 — Session、Config、Hooks、Permissions、Feature Flags           |
| `runtime`         | 运行时 — 后端抽象、执行器、调度器、团队协作、Swarm、基准测试                  |
| `rt-workflow`     | 工作流引擎 — DAG 执行器、节点调度、YAML 导入导出、表达式引擎、触发系统        |
| `rt-dashboard`    | Dashboard 插件注册系统                                                        |
| `rt-messaging`    | 消息网关 — 多平台消息（微信/钉钉/飞书/Slack/Telegram 等）                     |
| `rt-webhook`      | Webhook 订阅与触发                                                            |
| `rt-theme`        | 主题引擎                                                                      |
| `orchestrator`    | 高级编排器 — LLM 驱动的任务分解、子图生成、执行监控、重规划                   |
| `mcp`             | MCP 协议实现 — 客户端、服务器、OAuth、健康检查                                |
| `gateway`         | HTTP/WS API 网关 — 路由、中间件、认证、流式传输                               |
| `providers`       | LLM 提供商适配器 — OpenAI/Anthropic/Gemini 等多厂商适配                       |
| `search`          | 搜索子系统 — 向量搜索、混合搜索、文件索引、RAG 管道                           |
| `storage`         | 存储子系统 — 文件存储、云存储、WebDAV、同步冲突处理                           |
| `cache`           | 缓存层                                                                        |
| `crypto`          | 加密工具                                                                      |
| `billing`         | 计费模块                                                                      |
| `document-parser` | 文档解析器                                                                    |
| `npm`             | NPM 注册表服务                                                                |
| `prompt-guard`    | Prompt 安全防护                                                               |
| `trajectory`      | 轨迹分析 — 原子技能提取、技能分解、记忆提供者                                 |
| `telemetry`       | 遥测                                                                          |
| `migration`       | 数据库迁移工具                                                                |
| `kit`             | 工具箱 — 统一配置、技能目录、命令验证、沙箱运行器等                           |

### 1.3 Tauri 主程序 (src-tauri/src/)

```
src-tauri/src/
├── main.rs                    # Tauri 应用入口
├── lib.rs                     # 库入口
├── register_commands.rs       # 所有 Tauri 命令注册（约 400+ 命令）
├── app_state.rs               # 全局应用状态
├── shared_state.rs            # 共享状态
├── state.rs                   # 状态管理
├── commands/                  # Tauri 命令实现
│   ├── plugin.rs              # 插件 CRUD 命令
│   ├── skills.rs              # 技能管理命令 (2315 行)
│   ├── skills_hub.rs          # 技能市场命令
│   ├── skill_decomposition.rs # 技能分解命令
│   ├── workflow_ai.rs         # AI 工作流命令
│   ├── workflow_template.rs   # 工作流模板命令
│   ├── workflow_yaml.rs       # 工作流 YAML 导入导出
│   ├── work_engine.rs         # 工作引擎命令
│   ├── mcp.rs                 # MCP 命令
│   ├── dynamic_ui.rs          # 动态 UI 命令
│   └── ...                    # 其他约 100 个命令模块
├── init/                      # 应用初始化
└── smart_router/              # 智能路由
```

### 1.4 前端目录结构 (src/)

```
src/
├── components/
│   ├── agent/                 # Agent 相关组件
│   ├── dynamicUI/             # 动态 UI 组件系统
│   │   ├── containers/        # 布局容器 (Row/Column/Grid/Card/Tabs/Accordion)
│   │   ├── data/              # 数据展示 (Table/Chart/Timeline/TreeView/ListView)
│   │   ├── form/              # 表单组件 (Input/Select/DatePicker/Switch 等)
│   │   ├── media/             # 媒体展示 (Markdown/CodeEditor/FilePreview)
│   │   └── misc/              # 杂项 (Button/Image/Progress/Tag/Divider)
│   ├── skill/                 # 技能管理组件 (编辑器/版本/依赖检查/A/B测试等)
│   ├── workflow/              # 工作流编辑器
│   │   ├── Canvas/            # 画布
│   │   ├── Nodes/             # 节点组件
│   │   ├── Panels/            # 面板 (属性/调试)
│   │   ├── Edges/             # 边
│   │   ├── Hooks/             # React Hooks
│   │   ├── Templates/         # 模板
│   │   └── types/             # 类型定义
│   ├── gateway/               # 网关组件
│   ├── settings/              # 设置组件
│   ├── chat/                  # 聊天组件
│   └── ...
├── lib/
│   ├── dynamicUI/             # 动态 UI 引擎
│   │   ├── ComponentRegistry.ts    # 组件注册表 (Map<string, Component>)
│   │   ├── registerBuiltins.ts     # 内置组件注册 (20+ 组件)
│   │   ├── DataBindingEngine.ts    # 数据绑定引擎
│   │   ├── EventHandlerEngine.ts   # 事件处理引擎
│   │   ├── ConditionalRenderer.ts  # 条件渲染
│   │   ├── nl2ui.ts                # 自然语言生成 UI
│   │   ├── SchemaValidator.ts      # Schema 校验
│   │   └── useDataSource.ts        # 数据源 Hook
│   └── workers/               # Web Workers
├── stores/
│   ├── domain/                # 领域状态 (conversationStore, preferenceStore 等)
│   └── feature/               # 功能状态
│       ├── pluginStore.ts         # 插件状态 (安装/启用/禁用/卸载)
│       ├── skillStore.ts          # 技能状态
│       ├── skillExtensionStore.ts # 技能扩展状态
│       ├── workflowStore.ts       # 工作流状态 (CRUD/执行/版本)
│       ├── workflowEditorStore.ts # 工作流编辑器状态
│       ├── workEngineStore.ts     # 工作引擎状态
│       ├── dynamicUIStore.ts      # 动态 UI 状态
│       ├── mcpStore.ts            # MCP 状态
│       ├── agentStore.ts          # Agent 状态
│       ├── appConfigStore.ts      # 应用配置状态
│       └── ...
├── types/                     # TypeScript 类型定义
├── sdk/python/                # Python SDK (axagent_sdk)
└── pages/                     # 页面组件
    ├── Workflow/              # 工作流页面
    └── DevTools/              # 开发者工具页面
```

---

## 2. 插件、技能、工作流、动态UI 相关模块总览

### 2.1 插件系统 (Plugin System)

#### Rust 后端

| 文件                                              | 关键内容                                                                                                                                                                                                |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/crates/plugins/src/types.rs`           | `PluginManifest`、`PluginMetadata`、`PluginHooks`、`PluginLifecycle`、`PluginToolManifest`、`PluginSkillEntry`、`PluginAgentDefInternal`、`PluginMcpServer`、`PluginPermission`、`PluginDashboardPanel` |
| `src-tauri/crates/plugins/src/core.rs`            | `Plugin` trait（含 metadata/hooks/lifecycle/tools/mcp_servers/skills 方法）、`PluginDefinition` 枚举 (Builtin/Bundled/External)、`PluginRegistry`、`RegisteredPlugin`、`PluginSummary`                  |
| `src-tauri/crates/plugins/src/manager.rs`         | `PluginManager`（2112 行）— 插件发现/加载/安装/卸载/更新、启用/禁用、manifest 校验                                                                                                                      |
| `src-tauri/crates/plugins/src/hooks.rs`           | `HookRunner`、`HookEvent` 枚举 (PreToolUse/PostToolUse/PostToolUseFailure)                                                                                                                              |
| `src-tauri/crates/plugins/src/agent_provider.rs`  | `PluginAgentRegistry` — 插件提供的 Agent 定义注册表                                                                                                                                                     |
| `src-tauri/crates/plugins/src/skill_installer.rs` | `SkillInstaller` — 将插件中声明的技能文件部署到系统技能目录                                                                                                                                             |
| `src-tauri/crates/plugins/src/mcp_launcher.rs`    | `McpLauncher` — 启动/管理插件声明的 MCP 服务器进程                                                                                                                                                      |
| `src-tauri/crates/tools/src/plugin_sdk.rs`        | SDK 层：`AxAgentPlugin` trait（含 manifest/initialize/execute_tool/shutdown）、`PluginBuilder`、`SdkPluginRegistry`                                                                                     |
| `src-tauri/src/commands/plugin.rs`                | Tauri 命令：`plugin_list`、`plugin_validate_source`、`plugin_install`、`plugin_enable/disable`、`plugin_uninstall`、`plugin_update`                                                                     |

#### 前端

| 文件                                | 关键内容                                          |
| ----------------------------------- | ------------------------------------------------- |
| `src/stores/feature/pluginStore.ts` | Zustand store — 插件加载/安装/启用/禁用/卸载/更新 |

### 2.2 技能系统 (Skill System)

#### Rust 后端

| 文件                                                   | 关键内容                                                   |
| ------------------------------------------------------ | ---------------------------------------------------------- |
| `src-tauri/crates/trajectory/src/atomic_skill/`        | 原子技能提取                                               |
| `src-tauri/crates/trajectory/src/skill_decomposition/` | 技能分解                                                   |
| `src-tauri/crates/plugins/src/skill_installer.rs`      | 插件技能部署到 `{config_home}/skills/`                     |
| `src-tauri/src/commands/skills.rs`                     | Tauri 命令（2315 行）— 技能 CRUD、搜索、版本管理、导入导出 |
| `src-tauri/src/commands/skills_hub.rs`                 | 技能市场命令                                               |
| `src-tauri/src/commands/skill_decomposition.rs`        | 技能分解命令                                               |

#### 前端

| 文件                                            | 关键内容           |
| ----------------------------------------------- | ------------------ |
| `src/components/skill/SkillPageRenderer.tsx`    | 技能页面渲染器     |
| `src/components/skill/SkillMarkdownPage.tsx`    | 技能 Markdown 页面 |
| `src/components/skill/SkillPanels.tsx`          | 技能面板           |
| `src/components/skill/SkillToolbar.tsx`         | 技能工具栏         |
| `src/components/skill/SkillVersionTimeline.tsx` | 技能版本时间线     |
| `src/components/skill/SkillDependencyCheck.tsx` | 技能依赖检查       |
| `src/components/skill/SkillABTestResults.tsx`   | 技能 A/B 测试      |
| `src/components/skill/SkillEvolutionViewer.tsx` | 技能演化查看器     |
| `src/components/skill/ActionChainEditor.tsx`    | 动作链编辑器       |
| `src/stores/feature/skillStore.ts`              | 技能状态管理       |
| `src/stores/feature/skillExtensionStore.ts`     | 技能扩展状态管理   |

### 2.3 工作流系统 (Workflow System)

#### Rust 后端

| 文件                                                  | 关键内容                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/crates/harness/src/workflow_types.rs`      | 核心工作流类型定义（1796 行）：`WorkflowNode`（28 种节点）、`WorkflowEdge`、`JsonSchema`、`Variable`、`TriggerConfig`、`AgentNodeConfig`、`RetryConfig`、`ToolDef`、`NodeKind` 等                                                                                                                                                                                                                                                                                                                                                                                                |
| `src-tauri/crates/rt-workflow/src/workflow_engine.rs` | `Workflow` 运行时容器、`NodeRuntimeState`、`WorkflowStatus`、`WorkflowError`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `src-tauri/crates/rt-workflow/src/work_engine/`       | 节点执行器集合（46 个文件）：`agent_executor.rs`、`llm_executor.rs`、`tool_executor.rs`、`parallel_executor.rs`、`condition_executor.rs`、`loop_executor.rs`、`switch_executor.rs`、`debate_executor.rs`、`merge_executor.rs`、`subworkflow_executor.rs`、`code_executor.rs`、`http_request_executor.rs`、`database_query_executor.rs`、`file_operation_executor.rs`、`email_executor.rs`、`notification_executor.rs`、`webhook_send_executor.rs`、`vector_retrieve_executor.rs`、`data_transformer_executor.rs`、`llm_classifier_executor.rs`、`document_parser_executor.rs` 等 |
| `src-tauri/crates/rt-workflow/src/agent_roles.rs`     | `AgentRole` 枚举 (Coordinator/Researcher/Developer/Reviewer/Browser/Synthesizer/Planner/Executor) 含 system_prompt                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `src-tauri/crates/rt-workflow/src/yaml_io.rs`         | 工作流 YAML 导入/导出                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `src-tauri/crates/rt-workflow/src/expression_engine/` | 表达式引擎                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src-tauri/crates/rt-workflow/src/trigger/`           | 触发器系统                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src-tauri/crates/orchestrator/src/`                  | 高级编排器 — `OrchestratorExecutor`、`SubTask`、`DecompositionPlan`、`OrchestrationStrategy`（Ordered/FanOut/Pipeline/Race/Debate/Dynamic）                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `src-tauri/src/commands/workflow_ai.rs`               | AI 工作流 Tauri 命令                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `src-tauri/src/commands/workflow_template.rs`         | 工作流模板命令                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `src-tauri/src/commands/workflow_yaml.rs`             | YAML 导入导出命令                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `src-tauri/src/commands/work_engine.rs`               | 工作引擎命令                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

#### 前端

| 文件                                              | 关键内容                                                         |
| ------------------------------------------------- | ---------------------------------------------------------------- |
| `src/components/workflow/`                        | 完整工作流编辑器 UI（Canvas/Nodes/Panels/Edges/Hooks/Templates） |
| `src/components/workflow/types/workflow.types.ts` | 前端工作流类型定义（1358 行）                                    |
| `src/stores/feature/workflowStore.ts`             | 工作流状态管理                                                   |
| `src/stores/feature/workflowEditorStore.ts`       | 编辑器状态                                                       |
| `src/stores/feature/workEngineStore.ts`           | 执行引擎状态                                                     |

### 2.4 动态 UI 系统 (Dynamic UI)

#### 后端

| 文件                                   | 关键内容                                                |
| -------------------------------------- | ------------------------------------------------------- |
| `src-tauri/src/commands/dynamic_ui.rs` | Tauri 命令                                              |
| `src-tauri/crates/rt-dashboard/src/`   | `DashboardPlugin`、`DashboardRegistry` — 仪表盘插件系统 |

#### 前端

| 文件                                             | 关键内容                                                                                                                                                                               |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib/dynamicUI/ComponentRegistry.ts`         | 组件注册表 — Map<string, Component>，支持命名空间隔离                                                                                                                                  |
| `src/lib/dynamicUI/registerBuiltins.ts`          | 内置组件注册（20+ 组件：Container/Row/Column/Grid/Card/Tabs/Accordion/DataTable/Chart/Timeline/TreeView/ListView/Form/Input/Select/DatePicker/Switch/Markdown/CodeEditor/FilePreview） |
| `src/lib/dynamicUI/nl2ui.ts`                     | 自然语言→UI Schema 转换引擎                                                                                                                                                            |
| `src/lib/dynamicUI/DataBindingEngine.ts`         | 数据绑定引擎                                                                                                                                                                           |
| `src/lib/dynamicUI/EventHandlerEngine.ts`        | 事件处理引擎                                                                                                                                                                           |
| `src/lib/dynamicUI/ConditionalRenderer.ts`       | 条件渲染                                                                                                                                                                               |
| `src/lib/dynamicUI/SchemaValidator.ts`           | Schema 校验                                                                                                                                                                            |
| `src/components/dynamicUI/DynamicUIRenderer.tsx` | 动态 UI 渲染器（根据 JSON Schema 递归渲染）                                                                                                                                            |
| `src/components/dynamicUI/DynamicUIPreview.tsx`  | 动态 UI 预览                                                                                                                                                                           |
| `src/stores/feature/dynamicUIStore.ts`           | 动态 UI 状态管理                                                                                                                                                                       |

---

## 3. 核心 Trait / Interface / 抽象层

### 3.1 Harness 契约层 (src-tauri/crates/harness/src/)

这是整个架构的依赖反转核心，所有上层模块通过此处的 trait 相互解耦。

| Trait/Interface       | 文件                               | 说明                                                                                                                           |
| --------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `ProviderAdapter`     | `provider.rs`                      | LLM 提供商统一接口 — `chat()` / `chat_stream()` / `list_models()` / `embed()`                                                  |
| `Tool`                | `tool.rs`                          | 工具接口 — 所有工具必须实现；含 `name()` / `description()` / `category()` / `call()` / `validate()` / `check_permissions()`    |
| `ToolRegistry`        | `registry.rs`                      | 工具注册表抽象 — `get()` / `find()` / `list()` / `execute_tool()`                                                              |
| `ProviderRegistry`    | `registry.rs`                      | Provider 注册表抽象 — `get(provider_type)`                                                                                     |
| `Persistence`         | `persistence_mod`                  | 持久化层抽象                                                                                                                   |
| `PromptGuard`         | `prompt_guard.rs`                  | Prompt 安全防护接口                                                                                                            |
| `StorageBackend`      | `storage_backend.rs`               | 存储后端抽象                                                                                                                   |
| `WorkflowNode` (enum) | `workflow_types.rs`                | 28 种节点类型联合体（TriggerNode/AgentNode/ToolNode/LLMNode/ConditionNode/LoopNode/ParallelNode/MergeNode/SubWorkflowNode 等） |
| `BusinessRule`        | `business_rules.rs`                | 业务规则接口                                                                                                                   |
| `PluginHook`          | `runtime-core/src/plugin_hooks.rs` | 插件 Hook 接口 — `on_session_start/end()` / `on_before_tool_call()` / `on_after_tool_call()`                                   |

### 3.2 插件系统 Trait

| Trait           | 文件                      | 说明                                                                                                                                              |
| --------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Plugin`        | `plugins/src/core.rs`     | 插件核心接口 — `metadata()` / `hooks()` / `lifecycle()` / `tools()` / `mcp_servers()` / `skills()` / `validate()` / `initialize()` / `shutdown()` |
| `AxAgentPlugin` | `tools/src/plugin_sdk.rs` | SDK 层插件接口 — `manifest()` / `initialize()` / `execute_tool()` / `shutdown()`                                                                  |

### 3.3 Agent 系统核心结构

| 结构                | 文件                            | 说明                                                                                         |
| ------------------- | ------------------------------- | -------------------------------------------------------------------------------------------- |
| `AgentImpl` (trait) | `agent/src/coordinator.rs`      | Agent 实现接口 — `initialize()` / `execute()` / `shutdown()`                                 |
| `AgentConfig`       | `agent/src/agent_config.rs`     | Agent 配置                                                                                   |
| `AgentRole` (enum)  | `harness/src/workflow_types.rs` | 8 种内置角色：Coordinator/Researcher/Developer/Reviewer/Browser/Synthesizer/Planner/Executor |
| `AgentNodeConfig`   | `harness/src/workflow_types.rs` | 工作流中 Agent 节点配置 — system_prompt/tools/model/temperature 等                           |
| `PluginAgentDef`    | `plugins/src/agent_provider.rs` | 插件提供的 Agent 定义                                                                        |

### 3.4 运行时核心抽象

| 类型                  | 文件                                | 说明                                                                          |
| --------------------- | ----------------------------------- | ----------------------------------------------------------------------------- |
| `RuntimeConfig`       | `runtime-core/src/config/types.rs`  | 完整运行时配置 — plugins/MCP/hooks/oauth/model/sandbox/features               |
| `RuntimePluginConfig` | `runtime-core/src/config/types.rs`  | 插件配置 — enabled_plugins/external_dirs/install_root 等                      |
| `RuntimeHookConfig`   | `runtime-core/src/config/types.rs`  | Hook 配置 — pre_tool_use/post_tool_use/subagent_start/stop 等 14 种事件       |
| `ConfigLoader`        | `runtime-core/src/config/loader.rs` | 配置加载器 — 发现 settings.json / .claw.json / settings.local.json 并深度合并 |

---

## 4. Agent 注册/发现机制 与 消息路由

### 4.1 Agent 注册与发现

| 机制                        | 位置                             | 说明                                                                                                        |
| --------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **插件 Agent 注册**         | `plugins/src/agent_provider.rs`  | `PluginAgentRegistry` 使用 `HashMap<String, PluginAgentDef>` + `RwLock` 实现线程安全的 Agent 注册/注销/查询 |
| **全局单例**                | `plugins/src/agent_provider.rs`  | `GLOBAL_PLUGIN_AGENTS: LazyLock<PluginAgentRegistry>` 提供全局唯一 Agent 注册表                             |
| **插件清单中的 Agent 声明** | `plugins/src/types.rs`           | `PluginManifest.agents: Vec<PluginAgentDefInternal>` — 插件在 manifest 中声明其提供的 Agent                 |
| **运行时 Agent 角色**       | `rt-workflow/src/agent_roles.rs` | `AgentRole` 枚举定义 8 种内置角色，查找优先级：DB `agent_roles` 表 → 配置文件 → 内置枚举                    |
| **Tauri 命令**              | `src/commands/agent_role.rs`     | `agent_role_list` / `agent_role_get` 等命令                                                                 |

### 4.2 消息路由机制

| 组件                     | 位置                             | 说明                                                                |
| ------------------------ | -------------------------------- | ------------------------------------------------------------------- |
| **Gateway HTTP/WS 路由** | `gateway/src/routes.rs`          | REST API 路由定义                                                   |
| **Gateway 中间件**       | `gateway/src/middleware.rs`      | 认证、限流、日志中间件                                              |
| **会话路由**             | `runtime/src/session_router.rs`  | 会话级消息路由                                                      |
| **消息网关**             | `runtime/src/message_gateway.rs` | 跨平台消息分发（微信/钉钉/飞书/Slack/Telegram/QQ/Discord/WhatsApp） |
| **Tauri 命令注册**       | `src/register_commands.rs`       | 约 400+ 个 Tauri 命令统一注册，前端通过 `invoke()` 调用             |
| **Agent 事件总线**       | `agent/src/event_bus.rs`         | `AgentEventBus` — Agent 间松耦合事件通信                            |
| **工作流调度器**         | `runtime/src/scheduler.rs`       | `PriorityScheduler` — 任务优先级调度                                |

---

## 5. 配置管理相关模块

### 5.1 配置加载链路

```
ConfigLoader.discover()
  ├── ~/.claw.json                    (User 层，低优先级)
  ├── {config_home}/settings.json     (User 层)
  ├── {cwd}/.claw.json               (Project 层)
  ├── {cwd}/.claw/settings.json      (Project 层)
  └── {cwd}/.claw/settings.local.json (Local 层，最高优先级)
      ↓ 深度合并 (deep_merge_objects)
  RuntimeConfig { schema_version, merged, feature_config }
```

### 5.2 关键配置模块

| 模块                   | 文件                                   | 说明                                                                                    |
| ---------------------- | -------------------------------------- | --------------------------------------------------------------------------------------- |
| `ConfigLoader`         | `runtime-core/src/config/loader.rs`    | 配置发现、加载、合并                                                                    |
| `RuntimeConfig`        | `runtime-core/src/config/types.rs`     | 合并后的完整运行时配置                                                                  |
| `RuntimeFeatureConfig` | `runtime-core/src/config/types.rs`     | 解析后的特性配置 — hooks/plugins/MCP/oauth/model/aliases/permission/sandbox/features    |
| `RuntimePluginConfig`  | `runtime-core/src/config/types.rs`     | 插件配置 — enabled_plugins/external_directories/install_root/registry_path/bundled_root |
| `RuntimeHookConfig`    | `runtime-core/src/config/types.rs`     | Hook 命令配置（14 种生命周期事件）                                                      |
| `McpConfigCollection`  | `runtime-core/src/config/types.rs`     | MCP 服务器配置集合（Stdio/WebSocket/Remote/ManagedProxy/SDK）                           |
| `PluginManagerConfig`  | `plugins/src/manager.rs`               | 插件管理器配置 — config_home/enabled_plugins/external_dirs/install_root                 |
| `PluginRegistryReport` | `plugins/src/core.rs`                  | 插件注册报告 — 成功加载的插件 + 失败列表                                                |
| `AppState`             | `src/app_state.rs`                     | Tauri 全局应用状态（持有 PluginManager 等所有子系统引用）                               |
| `appConfigStore.ts`    | `src/stores/feature/appConfigStore.ts` | 前端配置状态管理                                                                        |

### 5.3 插件配置加载流程

```
1. PluginManagerConfig 从 RuntimePluginConfig 构建
2. PluginManager 通过 discover() 扫描三类插件：
   - Builtin: 内置于二进制
   - Bundled: {bundled_root} 目录
   - External: {external_directories} 和 {install_root}/installed.json 注册
3. 加载每个插件目录下的 plugin.json (或 .claude-plugin/plugin.json)
4. 解析 PluginManifest → 校验 → 注册到 PluginRegistry
5. 按 enabled_plugins 配置启用/禁用
6. SkillInstaller 部署插件技能到 {config_home}/skills/
7. McpLauncher 启动插件声明的 MCP 服务器
8. PluginAgentRegistry 注册插件提供的 Agent 定义
```

---

## 6. 扩展能力总结

AxAgent 作为能力基座，提供了以下扩展维度：

| 扩展维度           | 机制                                                                                | 关键接口                                                       |
| ------------------ | ----------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **插件 (Plugin)**  | Builtin/Bundled/External 三级插件系统，通过 `plugin.json` 声明能力                  | `Plugin` trait + `PluginManager`                               |
| **技能 (Skill)**   | 插件可声明 `skills: [{name, path}]`，通过 `SkillInstaller` 部署；也支持独立技能管理 | `PluginSkillEntry` + `SkillInstaller` + `skills.rs` Tauri 命令 |
| **工具 (Tool)**    | 插件可声明自定义工具 (`PluginToolManifest`)，或通过 SDK `AxAgentPlugin` 实现        | `Tool` trait (harness) + `PluginTool`                          |
| **MCP 服务器**     | 插件可声明 MCP 服务器，由 `McpLauncher` 管理进程生命周期                            | `PluginMcpServer` + `McpLauncher`                              |
| **工作流节点**     | 28 种节点类型可扩展，通过 `WorkflowNode` 枚举 + `node_executor_trait.rs`            | `NodeExecutor` trait + `NodeDispatcher`                        |
| **Agent 角色**     | 插件可声明 Agent 定义 (`PluginAgentDef`)，运行时通过 DB/配置/枚举三层查找           | `PluginAgentRegistry` + `AgentRole`                            |
| **动态 UI**        | 组件注册表 + JSON Schema → 递归渲染 + NL2UI 转换                                    | `ComponentRegistry` + `DynamicUIRenderer`                      |
| **Dashboard 面板** | 插件可声明 `dashboard_panels: [{id, component_name, position}]`                     | `DashboardRegistry` + `DashboardPlugin`                        |
| **Hook 拦截**      | 14 种生命周期 Hook 事件，支持 Shell 脚本和内联 `PluginHook` trait                   | `HookRunner` + `PluginHook` trait                              |
| **编排策略**       | 6 种编排策略 (Ordered/FanOut/Pipeline/Race/Debate/Dynamic)                          | `OrchestrationStrategy` + `OrchestratorExecutor`               |
| **Provider 扩展**  | LLM 提供商通过 `ProviderAdapter` trait 统一接入                                     | `ProviderAdapter` trait                                        |

---

## 8. 业务接入最佳实践深度调研

> 本章涵盖 Plugin 能力边界、Skill 定义格式、工作流节点类型、编排器策略、动态 UI 机制和现有配置样本六个方面。所有结论基于真实代码分析。

### 8.1 Plugin 的能力边界

#### 8.1.1 Plugin Trait 的 9 个方法（`plugins/src/core.rs`）

`Plugin` trait 是插件系统的最低抽象层，所有插件类型（Builtin / Bundled / External）都必须实现此 trait。完整方法签名：

| # | 方法            | 返回类型                                   | 语义                                                                          |
| - | --------------- | ------------------------------------------ | ----------------------------------------------------------------------------- |
| 1 | `metadata()`    | `&PluginMetadata`                          | 静态元信息（name/version/description/author/homepage/repository）             |
| 2 | `hooks()`       | `Option<&'static [PluginHookDefinition]>`  | 声明此插件拦截哪些生命周期事件；`None` = 不注册任何 Hook                      |
| 3 | `lifecycle()`   | `PluginLifecycle`                          | 声明支持的阶段：`OnLoad` / `OnEnable` / `OnDisable` / `OnUninstall`           |
| 4 | `tools()`       | `Option<&'static [PluginToolDefinition]>`  | 插件提供的工具定义（name + description + inputSchema + optional permissions） |
| 5 | `mcp_servers()` | `Option<&'static [PluginMcpServerConfig]>` | 插件托管的 MCP 服务器（command/args/env/healthCheck）                         |
| 6 | `skills()`      | `Option<&'static [PluginSkillEntry]>`      | 插件附带的技能文件（name + path 指向 .md 文件）                               |
| 7 | `validate()`    | `Result<(), PluginValidationError>`        | 插件自校验（manifest 一致性、资源可达性）                                     |
| 8 | `initialize()`  | `Result<(), PluginError>`                  | 一次性初始化（连接数据库、预热缓存）                                          |
| 9 | `shutdown()`    | `Result<(), PluginError>`                  | 优雅关闭（断开连接、保存状态）                                                |

**关键约束**：`Plugin` trait 上没有 `agents()` 和 `dashboards()` 方法。Agent 和 Dashboard 的注册走完全独立的通道（见 8.1.4）。

#### 8.1.2 PluginManifest 完整字段（`plugins/src/types.rs`）

`PluginManifest` 定义了插件对外声明的能力集合，共 **16 个字段**：

| 字段               | 类型                          | 说明                                                             |
| ------------------ | ----------------------------- | ---------------------------------------------------------------- |
| `id`               | `String`                      | 全局唯一标识符（如 `com.example.myplugin`）                      |
| `name`             | `String`                      | 显示名称                                                         |
| `version`          | `String`                      | 语义版本号                                                       |
| `description`      | `String`                      | 简短描述                                                         |
| `author`           | `Option<String>`              | 作者                                                             |
| `homepage`         | `Option<String>`              | 项目主页                                                         |
| `repository`       | `Option<String>`              | 代码仓库                                                         |
| `license`          | `Option<String>`              | 许可证                                                           |
| `icon`             | `Option<String>`              | 图标路径                                                         |
| `min_app_version`  | `Option<String>`              | 最低 AxAgent 版本要求                                            |
| `tools`            | `Vec<PluginToolManifest>`     | 通过 stdin/stdout 子进程通信的外部工具                           |
| `skills`           | `Vec<PluginSkillEntry>`       | 技能文件（仅 name + path）                                       |
| `agents`           | `Vec<PluginAgentDefInternal>` | Agent 定义（9 个子字段，含 tools / disallowed_tools / model 等） |
| `mcp_servers`      | `Vec<PluginMcpServer>`        | MCP 服务器定义                                                   |
| `dashboard_panels` | `Vec<PluginDashboardPanel>`   | 仪表盘面板（id / component_name / position / size）              |
| `scenarios`        | `Vec<String>`                 | 适用场景标签（用于技能市场推荐）                                 |

**能力声明的自由度**：Manifest 中的声明本质上是**静态能力清单**。插件可以声明它提供 Tool / Skill / Agent / MCP / Dashboard 五种能力中的任意组合。但声明只是第一步——Tool 需要实现 `PluginToolManifest` 中的子进程或 WASM 接口；Skill 需要对应的 .md 文件真实存在；Agent 需要实现 `PluginAgentDefInternal` 并注册到 `PluginAgentRegistry`；MCP 服务器需要可执行文件在运行时可达。

**能力的限制（Plugin 不能做的事）**：

1. **不能在工作流编辑器中声明新节点类型**——WorkflowNode 是编译期枚举，不支持动态扩展
2. **不能修改核心调度逻辑**——NodeDispatcher 的 HashMap 路由是内部实现细节，不对外暴露注册接口
3. **不能覆盖内置 Agent 角色**——AgentRole 枚举（Coordinator / Researcher 等 8 种）在 harness crate 中硬编码
4. **不能绕过 PluginManager 的生命周期管理**——插件加载/卸载严格走 PluginManager 的队列调度
5. **工具通信有两个层级**：trait 上的 `tools()` 返回进程内工具定义；manifest 上的 `tools` 字段声明子进程工具。两者针对不同的工具实现模式（SDK 内联 vs 独立进程）

#### 8.1.3 PluginDefinition 枚举

```rust
pub enum PluginDefinition {
    Builtin(BuiltinPlugin),     // 编译期内联，二进制内嵌，无独立目录
    Bundled(BundledPlugin),     // 随应用分发，位于 {bundled_root}/
    External(ExternalPlugin),   // 用户安装或外部搜索目录发现
}
```

三种变体的区别在于**发现和加载机制**：Builtin 通过 Rust 编译期静态注册；Bundled 在启动时从磁盘扫描；External 从 `external_directories` 和 `install_root/installed.json` 动态发现。

#### 8.1.4 能力声明的下游消费路径（完整链路）

**工具（Tool）消费链路**：

```
Plugin::tools() / PluginManifest.tools
  → PluginRegistry::aggregated_tools()   [遍历已启用插件 → 去重 → 重名检测]
  → ToolRegistry::register()             [注册到中心化工具注册表]
  → ToolExecutor::execute()              [工作流节点按 tool_name 精确匹配]
  → LLM function calling                 [Agent 节点将工具列表转换为 LLM schema]
```

**技能（Skill）消费链路**：

```
Plugin::skills() / PluginManifest.skills
  → SkillInstaller::install_plugin_skills()  [复制 .md 文件到 {config_home}/skills/{plugin_id}/]
  → skills_dir() 文件系统索引               [技能 CRUD 命令通过文件路径访问]
  → SkillManager / SkillMatcher              [运行时按关键词匹配加载]
```

**Agent 消费链路（独立通道）**：

```
PluginManifest.agents
  → PluginAgentRegistry::register_plugin_agents()  [HashMap<String, PluginAgentDef> + RwLock]
  → GLOBAL_PLUGIN_AGENTS 全局单例                   [跨 crate 通过 LazyLock 访问]
  → AgentExecutor / AgentCoordinator                [按 agent_type 字符串查找]
```

与 Plugin trait 解耦：Agent 注册发生在 `PluginManager` 的加载流程中，不通过 `Plugin` trait 的任何方法。

**Dashboard 消费链路**：

```
PluginManifest.dashboard_panels
  → DashboardRegistry::register_plugin_panels()
  → DashboardPluginAdapter                         [适配为 DashboardPlugin trait]
  → DynamicUIRenderer                              [前端按 component_name 查找注册表中的组件]
```

**MCP 服务器消费链路**：

```
Plugin::mcp_servers() / PluginManifest.mcp_servers
  → McpLauncher::launch_plugin_mcp()
  → MCP 客户端进程管理（启动/健康检查/关闭/重启）
  → MCP 工具自动发现（通过 tools/list 协议）
```

### 8.2 Skill 的完整定义格式

#### 8.2.1 技能文件物理格式

技能以目录为单位组织，存储在 `{config_home}/skills/{skill_name}/`。目录中至少包含一个 `SKILL.md` 文件作为技能入口，格式为 **YAML frontmatter + Markdown body**：

```markdown
---
name: my-skill
description: A skill that does something useful
version: 1.0.0
metadata:
  hermes:
    tags: [python, automation]
    category: development
    related_skills: []
---

# Skill Content

技能的实际指令内容，包含操作步骤、工具使用说明、代码示例等。
支持完整的 Markdown 语法，包括代码块（含语言标注）、表格、列表等。
```

#### 8.2.2 skill-manifest.json 格式

安装/部署技能时，系统自动生成或合并 `skill-manifest.json`：

```json
{
  "source_kind": "github",
  "source_ref": "owner/repo",
  "branch": "main",
  "commit": "abc1234",
  "installed_at": "2026-07-01T12:00:00Z",
  "installed_via": "marketplace",
  "versions": [
    { "version": "abc1234", "installed_at": "...", "commit": "abc1234" }
  ],
  "scenarios": ["code-generation", "testing"],
  "dependencies": [{ "name": "other-skill", "version_constraint": ">=1.0.0", "required": true }]
}
```

#### 8.2.3 HermesMetadata 完整字段（`trajectory/src/skill.rs`）

`HermesMetadata` 是 Skill 元数据的核心结构，定义了技能的可替代性和依赖关系：

| 字段                    | 类型                           | 用途                                                       |
| ----------------------- | ------------------------------ | ---------------------------------------------------------- |
| `tags`                  | `Vec<String>`                  | 分类标签（用于搜索和推荐）                                 |
| `category`              | `String`                       | 类别（默认为 "general"）                                   |
| `fallback_for_toolsets` | `Vec<String>`                  | 此技能可替代的工具集——当指定工具不可用时自动回退           |
| `requires_toolsets`     | `Vec<String>`                  | 此技能依赖的工具集——缺失时技能不可用                       |
| `config`                | `Vec<SkillConfig>`             | 用户可配置参数（key + description + default + prompt）     |
| `source_kind`           | `Option<String>`               | 来源类型：`"plugin"` / `"github"` / `"local"` / `"manual"` |
| `source_ref`            | `Option<String>`               | 来源引用（如 GitHub owner/repo）                           |
| `commit`                | `Option<String>`               | 安装时的 Git commit hash                                   |
| `skill_dependencies`    | `Option<Vec<SkillDependency>>` | 对其他技能的依赖声明                                       |

**`fallback_for_toolsets` 与 `requires_toolsets` 的设计意图**：这两个字段实现了技能的**可替代性模型**。一个技能可以声明"当工具集 X 不可用时，我可以作为替代方案"（fallback）；同时声明"我需要工具集 Y 才能正常运行"（requires）。这为运行时的能力协商提供了数据基础。

#### 8.2.4 技能的存储和索引方式

**目录结构**：

```
{config_home}/skills/
├── skill-name-1/
│   ├── SKILL.md                  # 主技能文件（YAML frontmatter + Markdown）
│   ├── skill-manifest.json       # 安装元数据（自动生成）
│   └── sub-docs/                 # 附属文档（可选，递归深度 ≤5 层）
│       ├── example.md
│       └── reference.md
├── skill-name-2/
│   └── ...
```

**内容收集**：`collect_markdown_files()` 递归扫描技能目录中的所有 `.md` 文件（深度限制 5 层，单文件 ≤5MB，总量 ≤10MB）。

**索引机制**：技能通过两个维度被索引和查找：

1. **文件系统直接访问**：`skills_dir().join(name)` — 技能目录通过约定的名称直接定位
2. **Plugin 注册表索引**：`PluginManager.plugin_registry_report()` 返回所有插件含技能信息的摘要列表
3. **关键词匹配**：`SkillMatcher` 使用约 25 个类别的 `KeywordPatterns` HashMap 进行关键词匹配（非向量语义匹配）

**安装来源**：

| 来源              | 触发方式                          | 安装逻辑                                                                                               |
| ----------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------ |
| GitHub            | `install_skill("owner/repo")`     | `git clone --depth 1` → 解析 → 写 `skill-manifest.json`                                                |
| GitHub (fallback) | Git 不可用时                      | API zipball 下载 → 路径遍历验证（三阶段：enclosed_name 预检 → extract → 解压后二次 CANONICALIZE 验证） |
| 本地目录          | `install_skill("/path/to/skill")` | 直接 `copy_dir_recursive` → 写 manifest                                                                |

#### 8.2.5 技能版本管理

- `install_skill` 时自动记录 commit 和 installed_at
- `skill-manifest.json` 中 `versions` 数组保留最近 10 个版本
- `rollback_skill(name, target_version)` 支持回滚到指定版本（仅 GitHub 来源）
- `check_skill_dependencies()` 自动校验依赖技能的安装状态

### 8.3 工作流的节点类型和能力边界

#### 8.3.1 完整节点类型列表

`harness/src/workflow_types.rs` 中 `WorkflowNode` 枚举实际包含 **32 种**节点类型（报告中此前统计为 28 种，经代码复核后更正）：

| #  | 变体名            | 配置结构                           | 说明                         |
| -- | ----------------- | ---------------------------------- | ---------------------------- |
| 1  | `Trigger`         | `TriggerNodeConfig`                | 定时/Webhook 等触发条件      |
| 2  | `LLM`             | `LLMNodeConfig`                    | 纯 LLM 文本生成              |
| 3  | `Agent`           | `AgentNodeConfig`                  | 完整 Agent（含工具调用循环） |
| 4  | `Tool`            | `ToolNodeConfig`                   | 单个工具调用                 |
| 5  | `Condition`       | `ConditionNodeConfig`              | 条件分支                     |
| 6  | `Loop`            | `LoopNodeConfig` + `sub_graph`     | 循环（含子图）               |
| 7  | `Switch`          | `SwitchNodeConfig`                 | 多路分支                     |
| 8  | `Parallel`        | `ParallelNodeConfig` + `sub_graph` | 并行执行（含子图）           |
| 9  | `Merge`           | `MergeConfig`                      | 合并多个输入                 |
| 10 | `SubWorkflow`     | `SubWorkflowConfig`                | 引用另一个工作流定义         |
| 11 | `Code`            | `CodeNodeConfig`                   | 代码执行（Python/JS/Bash）   |
| 12 | `HTTPRequest`     | `HttpRequestConfig`                | HTTP API 调用                |
| 13 | `DatabaseQuery`   | `DatabaseQueryConfig`              | 数据库查询                   |
| 14 | `FileOperation`   | `FileOperationConfig`              | 文件读写操作                 |
| 15 | `Email`           | `EmailConfig`                      | 邮件发送                     |
| 16 | `Notification`    | `NotificationConfig`               | 通知推送                     |
| 17 | `WebhookSend`     | `WebhookSendConfig`                | 主动 Webhook 发送            |
| 18 | `VectorRetrieve`  | `VectorRetrieveConfig`             | 向量检索                     |
| 19 | `DataTransformer` | `DataTransformerConfig`            | 数据转换/格式化              |
| 20 | `LLMClassifier`   | `LLMClassifierConfig`              | LLM 文本分类                 |
| 21 | `DocumentParser`  | `DocumentParserConfig`             | 文档解析                     |
| 22 | `Debate`          | `DebateNodeConfig` + `sub_graph`   | 多 Agent 辩论（含子图）      |
| 23 | `Swarm`           | `SwarmConfig` + `sub_graph`        | Agent 集群协作（含子图）     |
| 24 | `HumanInput`      | `HumanInputConfig`                 | 人工输入节点                 |
| 25 | `Timer`           | `TimerConfig`                      | 定时延迟                     |
| 26 | `Variable`        | `VariableConfig`                   | 变量定义/赋值                |
| 27 | `Comment`         | `CommentConfig`                    | 注释说明（无执行逻辑）       |
| 28 | `Subgraph`        | `SubgraphConfig`                   | 内联子图（匿名）             |
| 29 | `Start`           | `StartConfig`                      | 工作流入口                   |
| 30 | `End`             | `EndConfig`                        | 工作流出口                   |
| 31 | `RAG`             | `RAGNodeConfig`                    | RAG 检索增强生成             |
| 32 | `Connector`       | `ConnectorConfig`                  | 多入/多出连接器              |

#### 8.3.2 ToolNodeConfig 的约束

```rust
pub struct ToolNodeConfig {
    pub tool_name: String,                    // 工具名称字符串（精确匹配）
    pub input_mapping: HashMap<String, String>, // 输入参数映射
    pub output_var: String,                   // 输出变量的变量名
}
```

**自由度**：非常有限。`tool_name` 是纯字符串，必须与 `ToolRegistry` 中注册的工具名完全一致。没有版本约束、没有命名空间、没有 fallback。

**运行时绑定**：`ToolExecutor` 在运行时通过 `ExecutionState.tool_registry.execute_tool(tool_name, ...)` 查找。如果 tool_name 未注册，返回明确错误。支持通过 `callbacks.tool_handlers` 和 `callbacks.tool_fallback` 做两层回退。

**权限校验**：`ExecutionState.tool_permissions` 中的 `forbidden_tools` 和 `allowed_tools` 列表在工作流执行前注入，在 ToolExecutor 层面做前置拦截。

#### 8.3.3 AgentNodeConfig 的自由度

```rust
pub struct AgentNodeConfig {
    pub system_prompt: String,                          // 系统提示（支持 {{var}} 模板）
    pub tools: Vec<ToolDef>,                            // 可用工具定义列表
    pub exposed_tools: Vec<String>,                     // 实际暴露给 LLM 的工具子集
    pub context_sources: Vec<String>,                   // 上下文来源（工作流变量/全局状态）
    pub input_mapping: HashMap<String, String>,         // 输入映射
    pub output_var: String,                             // 输出变量名
    pub model: Option<String>,                          // 模型指定（None=使用默认）
    pub temperature: Option<f64>,                       // 温度参数
    pub max_tokens: Option<u32>,                        // 最大 Token 数
    pub agent_profile_id: Option<String>,               // 引用的 Agent 配置文件（DB 查找）
    pub agent_role_override: Option<AgentRole>,         // 角色覆盖
    pub consistency_check: Option<bool>,                // 一致性校验
    pub hallucination_guard: Option<bool>,              // 幻觉防护
    pub output_mode: OutputMode,                        // Text / Structured / Both
    pub retry: RetryConfig,                             // 重试策略
    pub timeout: Option<u32>,                           // 超时（秒）
    // ... 更多扩展字段
}
```

**自由度**：非常高。20+ 可配置字段，涵盖 LLM 参数、工具选择、角色绑定、输出模式、质量保障等。`tools` 字段本身就是完整的 `Vec<ToolDef>`，可以在工作流设计时静态指定任意工具组合。

**与 Skill 的关系**：Agent 节点的 `system_prompt` 字段可以引用 Skill 的内容。但工作流引擎本身**没有**内置的"将 Skill 转换为 Agent 节点"的快捷机制——Skill 需要通过 LLM 的 system prompt 注入方式间接参与。

#### 8.3.4 工作流能否调用 Skill？

**不能直接调用**。工作流节点类型中没有任何 `Skill` 节点。Skill 的典型消费路径是：

1. **Agent 节点的 system_prompt 注入**：将 Skill 的 Markdown 内容拼接到 Agent 的 system_prompt 中，让 LLM 按照技能指引执行
2. **SubWorkflow 节点引用**：如果 Skill 被编排为工作流模板，可通过 SubWorkflow 节点引用
3. **Tool 调用间接使用**：如果 Skill 指导使用特定工具，Agent 会通过常规工具调用路径执行

**设计哲学**：Skill 是"Agent 的知识/指令"，不是"工作流的原子节点"。工作流管执行拓扑，Skill 管执行方法。

#### 8.3.5 工作流能否嵌套其他工作流？

**可以，有两种方式**：

1. **`SubWorkflow` 节点**：引用外部工作流定义（通过 ID 或名称）——支持参数传递和结果回传
2. **容器节点 + `sub_graph`**：`Loop` / `Parallel` / `Debate` / `Swarm` / `Subgraph` 五种容器节点类型内置 `sub_graph: Vec<WorkflowNode>` 字段，直接在定义中嵌套子图

**嵌套约束**：子图继承父工作流的 `ExecutionState`，但可以有自己的变量作用域（通过 `context_sources` 控制可见性）。嵌套深度没有硬性限制，但受数据库字段大小和序列化性能制约。

### 8.4 编排器（Orchestrator）的 6 种策略

#### 8.4.1 OrchestrationStrategy 枚举（`orchestrator/src/types.rs`）

```rust
pub enum OrchestrationStrategy {
    Ordered,   // 串行：子任务按顺序执行，结果依次传递
    FanOut,    // 全并行：所有子任务同时启动，无依赖
    Pipeline,  // 流水线：分阶段执行，每阶段内并行、阶段间串行
    Race,      // 竞速：多方案并行，最先完成者胜出
    Debate,    // 辩论：多 Agent 并行分析，最终由裁判 Agent 综合
    Dynamic,   // 动态：由 LLM 实时决策任务分解和依赖关系
}
```

#### 8.4.2 编排器与工作流引擎的关系

**它们是独立但嵌套的**：编排器是工作流引擎的"上层建筑"。

```
OrchestratorExecutor.receive_mission("任务描述", strategy)
  │
  ├── decompose()           [规则驱动分解为 SubTask 列表]
  │     └── 关键词匹配：review → 3任务 / refactor → 4任务 / design → 3任务 / default → 3任务
  │
  ├── generate_subgraph()   [DynamicSubGraph 将 SubTask 转换为 WorkflowNode + WorkflowEdge]
  │     ├── 每个 SubTask → 一个 AgentNode（含系统提示、工具绑定、角色指定）
  │     └── 按策略构建边：
  │         Ordered/Pipeline → 串行链（隐式顺序边）
  │         FanOut/Race      → 无边（全并行）
  │         Debate            → 全部节点 → 裁判节点（Converge 边）
  │         Dynamic           → 保留 LLM 生成的依赖边（不添加隐式边）
  │
  ├── validate()            [Kahn's algorithm 检测环 + 孤立节点检测]
  │
  └── 产出 WorkflowGraph → 提交给 WorkEngine 执行
       │
       ├── monitor_and_maybe_replan()  [监听子任务完成/失败事件]
       │     ├── 全部成功 → Completed
       │     ├── 有失败 → replan() 重置失败任务状态，保留已完成任务
       │     │              → max_replans = 3（默认）
       │     └── 超限 → Aborted
       │
       └── emit OrchesterEvent 通知监听器
```

**关键设计**：

1. **编排器不直接执行**——它只负责分解和生成工作流图，实际执行交给 `WorkEngine`
2. **当前 decompose() 是规则驱动的**（关键词匹配），代码中明确标注了 "Future: LLM-driven decomposition"
3. **replan() 不会重复已完成的任务**——只重置失败任务状态，保留已完成任务的输出
4. **StructuredHandover** 定义了 Agent 间的交接协议：`completed_work` / `files_changed` / `next_steps` / `remaining_issues` / `dependencies_needed` / `validation_evidence`

#### 8.4.3 6 种策略的拓扑语义

| 策略       | 子图拓扑               | 最大并行            | 适用场景                                   |
| ---------- | ---------------------- | ------------------- | ------------------------------------------ |
| `Ordered`  | 串行链                 | 1                   | 严格顺序依赖的任务（分析→设计→实现）       |
| `FanOut`   | 无边图                 | phase_count         | 完全独立的任务（多文件分析、数据并行处理） |
| `Pipeline` | 阶段内并行、阶段间串行 | 每个阶段内 = 任务数 | 流水线作业（采集→清洗→分析→报告）          |
| `Race`     | 无边图                 | phase_count         | 多方案竞速（不同模型/不同思路同时尝试）    |
| `Debate`   | N→1 汇聚               | N                   | 需要多视角辩论后综合决策                   |
| `Dynamic`  | 由 LLM 决定            | 由 LLM 决定         | 复杂且不可预知结构的任务                   |

### 8.5 动态 UI 的触发和消费方式

#### 8.5.1 JSON Schema 由谁生成？

**三种生成来源**：

| 来源         | 场景              | 生成方式                                                                      |
| ------------ | ----------------- | ----------------------------------------------------------------------------- |
| **LLM**      | 对话式自然语言→UI | 用户说"帮我做一个XXX表单" → LLM 通过 `nl2ui.ts` 生成 UISchema                 |
| **手动定义** | 开发者设计 UI     | 通过 Tauri 命令 `create_dynamic_ui_schema` 写入 DB 的 `dynamic_ui_schemas` 表 |
| **插件声明** | 插件附带 UI       | `PluginManifest.dashboard_panels` 中的 `props` 字段包含 JSON Schema           |

#### 8.5.2 nl2ui.ts 的核心转换逻辑（`src/lib/dynamicUI/nl2ui.ts`）

自然语言→UI Schema 的转换流程：

1. **`FIELD_PATTERNS`** — 14 种关键词模式（姓名/邮箱/电话/地址/日期/性别/分类/金额/密码/URL/数量/评分/多行文本/图片）

2. **`detectFields()`** — 对自然语言描述做关键词匹配，检测表单字段类型：
   ```
   输入："收集姓名、邮箱和电话号码"
   → [{type: "input", label: "姓名"}, {type: "email", label: "邮箱"}, {type: "tel", label: "电话"}]
   ```

3. **`generateUIFromNaturalLanguage()`** — 组装 UISchema：
   ```
   Column → Text（标题说明）→ Form（含检测到的字段）
   ```
   匹配不到任何字段时 fallback 为"标题 + 内容输入框"

**核心约束**：

- 转换算法是**规则驱动**的（关键词匹配），不是 LLM 驱动的
- 14 种关键词模式覆盖常见表单场景，但不覆盖复杂 UI（如仪表盘、图表、树状视图）
- 复杂 UI 的 Schema 需要手动构建或通过 LLM 直接输出 UISchema JSON

#### 8.5.3 ComponentRegistry 组件注册表（`src/lib/dynamicUI/ComponentRegistry.ts`）

```typescript
class ComponentRegistry {
  private registry: Map<string, ComponentRegistryEntry>; // 支持 namespace:component 格式

  register(entry, namespace?); // 注册单个组件
  registerBatch(entries, namespace?); // 批量注册
  get(type: string); // 按名称获取（支持跨命名空间查找）
  resolve(type, namespace?); // 按命名空间精确解析
  getByCategory(category); // 按分类获取
  has(type); // 存在性检查
  unregister(type, namespace?); // 注销单个组件
  unregisterNamespace(namespace); // 注销整个命名空间
}
```

**命名空间隔离**：`PluginA:MyButton` 和 `PluginB:MyButton` 是两个独立组件。`get()` 在没有命名空间冲突时支持无命名空间查找，`resolve()` 是精确解析。

#### 8.5.4 registerBuiltins 内置组件（`src/lib/dynamicUI/registerBuiltins.ts`）

共注册 **27 个内置组件**，分 5 类：

| 分类                      | 组件                                                                                                                               |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Container**（布局容器） | `Container` / `Row` / `Column` / `Grid` / `Card` / `Tabs` / `Accordion`                                                            |
| **Data**（数据展示）      | `DataTable` / `ChartRenderer` / `TimelineView` / `TreeView` / `ListView` / `Dashboard`                                             |
| **Form**（表单组件）      | `FormRenderer` / `InputField` / `SelectField` / `DatePickerField` / `NumberField` / `CheckboxField` / `RadioField` / `SwitchField` |
| **Media**（媒体展示）     | `MarkdownView` / `CodeEditorView` / `FilePreviewView`                                                                              |
| **Misc**（杂项）          | `DynamicButton` / `DynamicText` / `DynamicImage` / `DynamicProgress` / `DynamicTag` / `DynamicDivider`                             |

#### 8.5.5 DynamicUIRenderer 渲染入口（`src/components/dynamicUI/DynamicUIRenderer.tsx`）

**核心渲染逻辑**（465 行）：

1. 接收 `UISchema`（递归树结构）
2. 从 `ComponentRegistry` 查找对应组件 → 未找到渲染错误提示
3. **数据绑定**：`subscribeDataSource` 订阅外部数据源，自动更新组件显示
4. **条件渲染**：`evaluateConditions` 按 `visibleWhen` / `hiddenWhen` 字段控制显隐
5. **事件处理**：`handleEvents` 处理 `onClick` / `onChange` / `onSubmit` 等事件，通过 `executeActions` 触发动作链
6. **Schema 热更新**：监听 `CustomEvent('schema-update')`，支持运行时动态替换/追加/移除 UI 节点（通过 `operation: "replace" | "append" | "remove"`）

**特殊处理**：`Tabs` / `Accordion` / `Form` 三种容器组件在渲染前做子节点预处理（`NEEDS_CHILD_PREPROCESSING`），因为它们需要将子节点重新组织到面板结构中。

#### 8.5.6 动态 UI 的运行时和设计时角色

**设计时**：

- 开发者通过 Tauri 命令 CRUD Schema（`list_dynamic_ui_schemas` / `create_dynamic_ui_schema` / `update_dynamic_ui_schema` / `delete_dynamic_ui_schema`）
- Schema 存储在数据库 `dynamic_ui_schemas` 表中（`id` / `title` / `schema_json` / `category` / `tags` / `is_builtin`）
- 表单数据通过 `dynamic_ui_form_data` 表持久化（支持 `instance_key` 多实例）

**运行时**：

- `DynamicUIRenderer` 根据 Schema JSON 递归渲染组件树
- 支持数据绑定（外部数据源 → 组件 props）、条件渲染、事件动作链
- 不支持 WebSocket 实时推送——状态更新依赖 React state + CustomEvent 机制

### 8.6 现有的配置文件和示例

#### 8.6.1 settings.json（项目根目录，284 行）

关键配置要点：

```json
{
  "models": {
    "DeepSeek-V3-0324": { "provider": "deepseek", "apiKey": "..." },
    "NVIDIA-Llama-3.1-Nemotron": { "provider": "nvidia", "apiKey": "...", "baseUrl": "..." }
  },
  "enabledPlugins": {
    "com.github.copilot": true
  },
  "agent": {
    "maxRequests": 500,
    "maxParallelTools": 16,
    "planMode": true,
    "thinkingTool": "extended"
  }
}
```

**实践样本特征**：

- 多 Provider 模型配置（DeepSeek + NVIDIA）
- GitHub Copilot 插件显式启用
- Agent 配置中 `maxRequests=500` / `maxParallelTools=16` 表明面向高并发长任务
- `planMode=true` 启用编排器的 Plan 模式
- 代码生成指令中指定了文件排除列表（node_modules / .git / dist）

#### 8.6.2 .codeartsdoer/ 目录结构

```
.codeartsdoer/
├── AGENTS.md                    # IDE 代理上下文声明
│   └── Language Context: TS / TS_Strict / TS_ESM
│   └── 技术栈: React + Vite + Antd
├── package.json                 # 依赖: @opencode-ai/plugin 1.3.17, @opencode-ai/sdk 1.1.3
├── agents/                      # 空目录（待扩展）
├── mcp/
│   └── mcp_settings.json        # 空 MCP 服务器配置 {}
├── rule/
│   └── metadata.properties      # projectExpert=[]（项目领域专家配置为空）
└── skills/
    └── ProjectSkillStatus.txt   # 空文件（技能状态占位）
```

**实践总结**：

1. **`AGENTS.md`** 是 IDE 上下文声明文件，定义了此项目的基础语言环境和架构栈信息，供 IDE 内的 Agent 加载使用
2. **`mcp_settings.json`** 为空对象，表明当前未配置任何业务级 MCP 服务器——MCP 能力通过插件系统中的 `PluginManifest.mcp_servers` 走插件通道
3. **`rule/metadata.properties`** 中 `projectExpert=[]` 表明未配置项目级领域专家规则
4. **`skills/`** 目录仅有一个占位文件，表明当前项目尚未创建自定义本地技能——技能主要通过插件系统和技能市场获取

#### 8.6.3 从配置样本看业务接入的典型模式

| 扩展需求            | 实践方式                                   | 配置位置                                     |
| ------------------- | ------------------------------------------ | -------------------------------------------- |
| 自定义 LLM Provider | 通过 `settings.json` 的 `models` 字段声明  | `settings.json`                              |
| 插件启用/禁用       | `enabledPlugins` 字段按 plugin_id 精确控制 | `settings.json`                              |
| MCP 服务器          | 通过插件 manifest 声明（非本地配置文件）   | `plugin.json` → `PluginManifest.mcp_servers` |
| Agent 行为调整      | `agent` 字段控制全局 Agent 参数            | `settings.json`                              |
| 项目上下文声明      | `AGENTS.md` 提供语言/架构栈信息            | `.codeartsdoer/AGENTS.md`                    |
| 业务规则            | `rule/metadata.properties`（当前为空）     | `.codeartsdoer/rule/`                        |
| 本地技能            | `skills/` 目录（当前为空）                 | `.codeartsdoer/skills/`                      |

#### 8.6.4 能力声明的完整矩阵

综合前六节分析，AxAgent 中每种扩展能力的目标对象、声明方式与生效路径：

| 能力            | 声明位置                                   | 声明格式                      | 生效触发器               | 查找方式                                   |
| --------------- | ------------------------------------------ | ----------------------------- | ------------------------ | ------------------------------------------ |
| Tool（SDK内联） | Plugin trait `tools()`                     | `Vec<PluginToolDefinition>`   | PluginManager 加载       | 按 `name` 精确匹配                         |
| Tool（子进程）  | PluginManifest `tools`                     | `Vec<PluginToolManifest>`     | PluginManager 加载       | 子进程 stdin/stdout 通信                   |
| Skill           | PluginManifest `skills`                    | `Vec<PluginSkillEntry>`       | SkillInstaller 部署      | 文件系统路径 + 关键词匹配                  |
| Agent           | PluginManifest `agents`                    | `Vec<PluginAgentDefInternal>` | PluginAgentRegistry 注册 | 按 `agent_type` 字符串查找                 |
| Dashboard       | PluginManifest `dashboard_panels`          | `Vec<PluginDashboardPanel>`   | DashboardRegistry 注册   | 按 `component_name` 查找 ComponentRegistry |
| MCP 服务器      | Plugin trait `mcp_servers()`               | `Vec<PluginMcpServerConfig>`  | McpLauncher 启动进程     | MCP 协议 tools/list 发现                   |
| Hook            | Plugin trait `hooks()`                     | `Vec<PluginHookDefinition>`   | HookRunner 分发          | 按事件类型 + 优先级排序                    |
| 工作流节点      | 编译期枚举                                 | `WorkflowNode::XXX`           | NodeDispatcher dispatch  | 按 `node_type` 字符串 HashMap              |
| Feature Flag    | RuntimeConfig `features`                   | `BTreeMap<String, bool>`      | 代码内运行时判断         | 按 key 精确匹配                            |
| 编排策略        | OrchestratorExecutor `receive_mission(..)` | `OrchestrationStrategy` 枚举  | orchestrator decompose   | 策略影响拓扑生成                           |
| 动态 UI Schema  | `create_dynamic_ui_schema` Tauri 命令      | JSON Schema（DB 持久化）      | DynamicUIRenderer        | ComponentRegistry 按 `type` 查找           |

---

## 7. 关键模块深度调研

> 基于代码级深入分析，涵盖五个关键方面。所有结论均有文件引用支撑。

### 7.1 Plugin Trait 与 PluginDefinition（plugins/src/core.rs + types.rs）

#### Plugin Trait 完整方法签名

`Plugin` trait 定义位于 `plugins/src/core.rs`，共 9 个方法：

| 方法            | 返回类型                                   | 说明                       |
| --------------- | ------------------------------------------ | -------------------------- |
| `metadata()`    | `&PluginMetadata`                          | 插件元信息                 |
| `hooks()`       | `Option<&'static [PluginHookDefinition]>`  | Hook 声明（`None`=不注册） |
| `lifecycle()`   | `PluginLifecycle`                          | 生命周期阶段标记           |
| `tools()`       | `Option<&'static [PluginToolDefinition]>`  | 此插件提供的工具定义列表   |
| `mcp_servers()` | `Option<&'static [PluginMcpServerConfig]>` | 此插件管理的 MCP 服务器    |
| `skills()`      | `Option<&'static [PluginSkillEntry]>`      | 此插件绑定的技能文件       |
| `validate()`    | `Result<(), PluginValidationError>`        | 插件自检                   |
| `initialize()`  | `Result<(), PluginError>`                  | 插件初始化回调             |
| `shutdown()`    | `Result<(), PluginError>`                  | 插件关闭回调               |

**关键发现**：`Plugin` trait **没有**直接的 `agents()` / `dashboards()` 方法。Agent 和 Dashboard 的注册走独立通道（见下文）。

#### PluginDefinition 枚举

```rust
pub enum PluginDefinition {
    Builtin(BuiltinPlugin),   // 编译期内联，无独立目录
    Bundled(BundledPlugin),   // 随应用分发的预装插件（{bundled_root} 目录）
    External(ExternalPlugin), // 用户安装或外部目录发现的插件
}
```

三个变体各自包含 `metadata`、`hooks`、`lifecycle`、`tools`、`mcp_servers`、`skills` 字段，结构一致但加载路径不同。

#### 能力声明后的下游消费路径

**路径 A — 工具（Tool）**：

1. `Plugin::tools()` 返回 `Option<&'static [PluginToolDefinition]>`，每个定义含 `name` / `description` / `input_schema`
2. `PluginRegistry::aggregated_tools()` 遍历所有已启用插件，收集 tools → 去重 → 校验重名冲突 → 返回 `Vec<PluginToolDefinition>`
3. 上游调用方（ToolRegistry / ToolExecutor）通过名称精确匹配

**路径 B — Skill**：

1. `Plugin::skills()` 返回 `Option<&'static [PluginSkillEntry]>`，每个条目含 `name` / `path`
2. `SkillInstaller` 在插件加载时扫描这些条目，将技能文件从插件目录**复制到**系统技能目录 `{config_home}/skills/`
3. 之后技能系统通过文件系统路径独立管理，与插件解耦

**路径 C — Agent（独立通道，不在 Plugin trait 上）**：

1. `PluginManifest.agents: Vec<PluginAgentDefInternal>`（定义在 `plugins/src/types.rs`，含 `agent_type` / `description` / `tools` / `disallowed_tools` / `model` 等字段）
2. 插件加载后，`PluginAgentRegistry`（`HashMap<String, PluginAgentDef> + RwLock`）通过全局单例 `GLOBAL_PLUGIN_AGENTS` 接收 `register_plugin_agents()`
3. Agent 注册与 Plugin trait 的生命周期方法**解耦**——Agent 注册发生在插件加载阶段（`PluginManager` 协调），不经过 `Plugin` trait

**路径 D — Dashboard 面板**：
`PluginManifest.dashboard_panels: Vec<PluginDashboardPanel>` → `DashboardRegistry` 注册，同样独立于 `Plugin` trait。

**路径 E — MCP 服务器**：
`Plugin::mcp_servers()` → `McpLauncher` 管理进程生命周期（启动/健康检查/关闭）。

---

### 7.2 工作流引擎：NodeExecutor + NodeDispatcher（rt-workflow/src/work_engine/）

#### NodeExecutorTrait 签名

定义于 `node_executor_trait.rs`：

```rust
pub trait NodeExecutorTrait: Send + Sync {
    fn node_type(&self) -> &'static str;
    fn execute(
        &self,
        input: Value,
        execution_state: &mut ExecutionState,
    ) -> Result<NodeOutput, NodeError>;
}
```

两个伴生结构体：

```rust
pub struct NodeOutput {
    pub content: Value,
    pub status: NodeStatus,          // Success / Failed / Skipped / Pending
    pub metadata: Option<Value>,
}

pub struct NodeError {
    pub error_code: &'static str,    // 20+ 预定义错误码常量
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
}
```

#### NodeDispatcher 调度逻辑

`dispatcher.rs`（436 行）核心结构：

```rust
pub struct NodeDispatcher {
    executors: HashMap<&'static str, Arc<dyn NodeExecutorTrait>>,
    // ...
}
```

**注册**：`register(executor)` / `register_arc(executor)` 将实现插入 HashMap，按 `node_type()` 返回值索引。

**调度**（`dispatch` 方法）流程：

1. 根据 `node_type_str` 从 HashMap 查找 executor → 未找到返回 `UnknownNodeType` 错误
2. **业务规则检查**（`check_business_rules`）：工作流级 domain constraints 前置校验
3. **表达式模板解析**：对 input 中的 `{{variable}}` 模板做解析替换
4. 调用 `executor.execute(input, execution_state)` 执行
5. 错误处理：捕获 `NodeError` → 按 `retryable` 判断是否重试 → 支持 `continue_on_fail` 语义

**调度机制**：纯名称字符串精确匹配，HashMap O(1) 查找。没有任何语义匹配、别名解析或模糊路由。

#### 节点如何引用 Tool / Agent

**ToolNode 绑定工具**（通过 `ToolNodeConfig`，位于 `harness/workflow_types.rs`）：

```rust
pub struct ToolNodeConfig {
    pub tool_name: String,       // 工具名称字符串
    pub input_mapping: HashMap<String, String>,
    pub output_var: String,
}
```

`ToolExecutor`（`work_engine/executors/tool_executor.rs`）三级调用优先级：

1. `ToolRegistry.execute_tool(tool_name, input, ctx)` — 中心化路径（含权限校验）
2. `callbacks.tool_handlers` HashMap 按 tool_name 精确匹配 — 多路注册
3. `callbacks.tool_fallback` — 旧版兼容

**AgentNode 绑定工具**（通过 `AgentNodeConfig`）：

```rust
pub struct AgentNodeConfig {
    // ...
    pub tools: Vec<ToolDef>,        // 完整 Tool 定义列表
    pub exposed_tools: Vec<String>,  // 暴露给 LLM 的工具子集（按名称过滤）
    // ...
}

pub struct ToolDef {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<JsonSchema>,
}
```

`AgentExecutor::execute()` 执行时：

1. 将 `tools` 列表转换为 LLM 的 function/tool schema 声明
2. LLM 调用工具时，`execute_tool()` 辅助函数按 tool_name **精确字符串**匹配查找
3. **连续 2 轮空 content 提前终止**（防幻觉）, 最多 5 轮工具调用

**结论**：工作流节点对 Tool/Agent 的引用是**静态名称硬编码**，在设计时确定，无运行时的注册表动态查找或语义匹配。

---

### 7.3 Agent Coordinator 的工具选择机制（agent/src/coordinator.rs）

#### AgentCoordinator 结构

`AgentCoordinator` 本身**不直接持有或选择工具列表**——它是一个协调器壳层：

```rust
pub struct AgentCoordinator<I: AgentImpl + Send + Sync> {
    inner: I,
    state: AgentState,
    tot_engine: Option<Box<dyn TotEngine>>,
    reasoning_router: Option<ReasoningRouter>,
    reasoning_engine: Option<Box<dyn ReasoningEngine>>,
}
```

构造方法：`new(impl)` / `with_tot_engine(...)` / `with_reasoning_router(...)` / `with_reasoning_engine(...)` — 全部是依赖注入模式。

#### 工具选择实际发生的位置

工具选择逻辑**不在 coordinator.rs**，而在两个下游执行层：

**A. 工作流层 — AgentExecutor（`rt-workflow/src/work_engine/engine/executors/agent_executor.rs`）**：

- 从 `AgentNodeConfig.tools: Vec<ToolDef>` 读取工具列表（工作流定义时静态绑定）
- 构建 OpenAIFunction 格式的 ChatTool 列表发给 LLM
- LLM 返回的 tool_call.name 通过 `execute_tool()` 按**精确名称字符串**匹配

**B. Core Agent 层 — AgentImpl 实现**：

- `WorkerDefinition` 结构体持有 `tools: Vec<String>`（工具名列表）
- Agent 通过 `ToolRegistry` 接口按名称查找：`registry.get(name)` / `registry.execute_tool(name, input, ctx)`
- 所有查找均为**精确名称匹配**，不存在语义匹配、相似度排序或自然语言→工具名的映射

#### 关键结论

Agent 工具选择是**纯名称精确匹配**。如果工具名与 LLM 返回的调用名不一致（如大小写差异、别名、简写），查找将失败。`parse_tool_name()` 辅助函数（`harness/src/tool.rs`）支持 `"server/tool"` 格式的命名空间拆分。

---

### 7.4 Trajectory：原子技能提取与技能分解

#### 7.4.1 AtomicSkillExecutor（trajectory/src/atomic_skill/executor.rs）

`AtomicSkillExecutor` 按 `EntryType` 分发执行：

```rust
pub enum EntryType {
    Builtin,  // 内置操作（string/math/json/list）
    Mcp,      // MCP 服务器工具
    Local,    // 本地文件系统技能
    Plugin,   // 插件提供的技能
}
```

三种调度方法：

- `execute_builtin(input)` — 直接执行内置操作（字符串处理/数学运算/JSON转换/列表操作）
- `execute_mcp(input, mcp_call_fn)` — 通过外部注入的 `mcp_call_fn` 闭包调用 MCP 工具
- `execute_local(input, local_execute_fn)` — 通过外部注入的 `local_execute_fn` 闭包执行本地技能
- `execute_plugin(input, plugin_call_fn)` — 通过外部注入的 `plugin_call_fn` 闭包调用插件能力

**核心设计**：`AtomicSkillExecutor` 自身是无状态的，所有外部能力通过依赖注入的闭包传入。这意味着原子技能的入库/注册路径不在 executor 层，而在更上层的调用方（Tauri 命令或 skill_manager）。

#### 7.4.2 SkillDecomposer（trajectory/src/skill_decomposition/decomposer.rs）

`CodeBlock` 结构体（核心分解单元）：

```rust
pub struct CodeBlock {
    pub language: String,
    pub code: String,
    pub dependencies: Vec<String>,     // 推断的依赖项
    pub entry_point: Option<String>,
}

impl CodeBlock {
    pub fn infer_dependencies(&self) -> Vec<String> {
        // 支持 Python（import / from ... import）、
        // JavaScript/TypeScript（require / import ... from）的依赖推断
    }
}
```

`SkillContentType` 枚举：

```rust
pub enum SkillContentType {
    Markdown,
    CodeScript,
    ConfigTemplate,
    WorkflowDefinition,
    Unknown,
}
```

#### 7.4.3 ToolResolver（skill_decomposition/tool_resolver.rs）

`ToolResolver::check_tool_dependencies(tool_names, available_tools)` 将每个依赖工具分类为：

| 状态                | 说明                                    |
| ------------------- | --------------------------------------- |
| `Satisfied`         | 已安装可用                              |
| `AutoInstallable`   | 可通过包管理器自动安装（如 npm/pip 包） |
| `ManualInstallable` | 需要手动安装（系统级工具或二进制）      |
| `NeedsGeneration`   | 不存在且无法安装，需要 LLM 生成         |

#### 7.4.4 SkillPackageParser（skill_decomposition/package_parser.rs）

```rust
pub struct SkillPackageParser;

impl SkillPackageParser {
    pub fn parse_files(paths: Vec<PathBuf>) -> Vec<ParsedFile>;
    pub fn extract_code_blocks(content: &str) -> Vec<CodeBlock>;
    pub fn extract_references(content: &str) -> Vec<SkillReference>;
}
```

#### 7.4.5 Skill 数据模型（trajectory/src/skill.rs）

```rust
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub platforms: Vec<String>,
    pub quality_score: f64,
    pub success_rate: f64,
    pub total_usages: u32,
    pub metadata: SkillMetadata,      // 含 HermesMetadata
}

pub struct HermesMetadata {
    pub tags: Vec<String>,
    pub category: String,
    pub fallback_for_toolsets: Vec<String>,  // 可替代的工具集
    pub requires_toolsets: Vec<String>,       // 依赖的工具集
    pub config: Vec<SkillConfig>,
    pub skill_dependencies: Option<Vec<SkillDependency>>,
    pub source_kind: Option<String>,   // 技能来源（plugin/hub/manual）
    pub source_ref: Option<String>,    // 来源引用
    pub commit: Option<String>,        // 版本提交
}
```

#### 7.4.6 原子技能如何入库

入库路径分两层：

1. **插件技能**：`PluginManifest.skills` → `SkillInstaller` → 部署到 `{config_home}/skills/{plugin_id}/` → 技能文件系统索引
2. **分解产生的技能**：`SkillDecomposer` 产生 `CodeBlock` + 依赖信息 → 上层命令（`skill_decomposition.rs` Tauri 命令）调用 `skill_manager::create_skill_from_params()` → 写入 `Skill` 结构体 → 持久化到数据库（通过 entities/dao）

**关键衔接点**：`AtomicSkillExecutor` 和 `SkillDecomposer` 都不直接操作数据库。入库逻辑在 Tauri 命令层（`src-tauri/src/commands/skill_decomposition.rs`），该命令组合调用 `SkillDecomposer` 产出分解结果，再调用 `SkillManager` 入库。

#### 7.4.7 SkillMatcher（trajectory/src/skill_matcher.rs）

用于根据用户输入匹配已有技能：

```rust
pub struct SkillMatch {
    pub skill: MatchedSkill,
    pub match_score: f64,
    pub match_reasons: Vec<String>,
    pub source: MatchSource,        // Installed / Marketplace / Custom
}

pub struct MatchingResult {
    pub matches: Vec<SkillMatch>,
    pub best_match: Option<SkillMatch>,
    pub needs_marketplace_search: bool,
    pub suggested_marketplace_skills: Vec<String>,
}
```

匹配引擎使用**关键词模式**（`KeywordPatterns`——约 25 个类别 × 多关键词的 HashMap），而非向量语义匹配。

---

### 7.5 RuntimeConfig：Feature Flag 与 Plugin 控制（runtime-core/src/config/types.rs）

#### RuntimeConfig 结构

```rust
pub struct RuntimeConfig {
    pub schema_version: String,            // "2"
    pub merged: Value,                     // 合并后的原始 JSON（用于调试）
    pub feature_config: RuntimeFeatureConfig,
}
```

#### RuntimeFeatureConfig

```rust
pub struct RuntimeFeatureConfig {
    pub hooks: RuntimeHookConfig,          // Hook 命令配置
    pub plugins: RuntimePluginConfig,      // 插件启用/禁用与路径配置
    pub mcp: McpConfigCollection,          // MCP 服务器集合
    pub oauth: OAuthConfig,                // OAuth 提供商配置
    pub model: ModelConfig,                // 模型默认配置
    pub aliases: HashMap<String, String>,  // 模型别名
    pub permission: PermissionConfig,      // 权限策略
    pub sandbox: SandboxConfig,            // 沙箱配置
    pub features: BTreeMap<String, bool>,  // Feature Flag（任意 key→bool）
    pub agent: AgentConfig,                // Agent 全局配置
    pub custom_commands: Vec<CustomCommandConfig>, // 自定义 slash 命令
    pub skill_directories: Vec<String>,    // 额外技能搜索路径
}
```

#### Feature Flag 机制

`features: BTreeMap<String, bool>` — 任意字符串 key 映射到布尔值。这是**通用 Feature Flag**，用于控制实验性功能的开关。使用方式为运行时通过 `config.feature_config.features.get("flag_name") == Some(&true)` 判断。

**与 Plugin 控制的关系**：Feature Flag 和 Plugin 启用/禁用是**两层独立机制**。Feature Flag 控制代码级功能开关（如"是否启用新的对话引擎"），Plugin 控制通过 `RuntimePluginConfig.enabled_plugins` 管理。

#### RuntimePluginConfig

```rust
pub struct RuntimePluginConfig {
    /// BTreeMap<plugin_id, bool> — 按插件 ID 精确启用/禁用
    pub enabled_plugins: BTreeMap<String, bool>,
    /// 外部插件搜索目录
    pub external_directories: Vec<String>,
    /// 插件安装根目录
    pub install_root: String,
    /// 已安装插件注册表路径
    pub registry_path: String,
    /// 内置/Bundled 插件目录
    pub bundled_root: String,
    /// 是否自动检查插件更新
    pub auto_update_check: bool,
}
```

**启用/禁用流程**：

1. `PluginManager` 启动时从 `RuntimePluginConfig` 构建 `PluginManagerConfig`
2. 发现所有插件（Builtin/Bundled/External）→ 加载 → 校验
3. **按 `enabled_plugins` 过滤**：只有显式设为 `true` 或在白名单中且未设为 `false` 的插件才会被"激活"
4. 已激活的插件参与 `aggregated_tools()` / `SkillInstaller` / `McpLauncher` / Agent 注册等下游流程

**注意**：由于采用 BTreeMap 而非 Vec，`enabled_plugins` 支持"全量默认启用 + 选择性禁用"和"全量默认禁用 + 选择性启用"两种策略，取决于 PluginManager 的默认行为（未出现在 map 中的插件走默认策略）。

#### RuntimeHookConfig

```rust
pub struct RuntimeHookConfig {
    pub pre_tool_use: Option<Vec<HookCommand>>,
    pub post_tool_use: Option<Vec<HookCommand>>,
    pub post_tool_use_failure: Option<Vec<HookCommand>>,
    pub subagent_start: Option<Vec<HookCommand>>,
    pub subagent_stop: Option<Vec<HookCommand>>,
    pub stop: Option<Vec<HookCommand>>,
    pub session_start: Option<Vec<HookCommand>>,
    pub session_end: Option<Vec<HookCommand>>,
    pub user_prompt_submit: Option<Vec<HookCommand>>,
    pub pre_compaction: Option<Vec<HookCommand>>,
    pub notification: Option<Vec<HookCommand>>,
    pub pre_workflow_execute: Option<Vec<HookCommand>>,
    pub post_workflow_complete: Option<Vec<HookCommand>>,
    pub checkpoint_create: Option<Vec<HookCommand>>,
    pub checkpoint_restore: Option<Vec<HookCommand>>,
}
```

14 种生命周期 Hook，每种可配置多个 Shell 命令串行执行。

#### 配置加载链路

```
ConfigLoader.discover()
  → 扫描 5 层配置文件（~/.claw.json → settings.json → .claw.json → .claw/settings.json → .claw/settings.local.json）
  → deep_merge_objects 深度合并
  → migrate_v1_to_v2 版本迁移（如有旧格式）
  → RuntimeConfig { schema_version: "2", merged, feature_config }
```
