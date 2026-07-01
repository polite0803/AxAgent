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
