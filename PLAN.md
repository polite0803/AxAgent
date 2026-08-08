# 任务计划 — OPC 内容媒体行业功能增强（断裂修复 + 专属工作流落地）

> 由 WorkBuddy 生成，作为 Trae 编码的输入规范。Trae 严格按本计划的「任务清单」执行，按「验收标准」自检。
> 本计划基于 2026-08-08 代码审计：内容媒体行业（`content-media`）功能地基齐全（yaml 五件套 + 博客/落地页 CRUD + 3 工具 + `workflow-content-media` 模板），但存在 7 处前后端契约断裂导致真实 Tauri 环境核心功能不可用。

## 元信息

| 字段     | 值                     |
| -------- | ---------------------- |
| 项目路径 | D:\OneManager\AxInvest |
| 分支     | master                 |
| 生成时间 | 2026-08-08 01:30       |
| 计划版本 | v1.0                   |

## 1. 需求说明

让 OPC「内容媒体」行业在真实 Tauri 环境下完整跑通：修复行业 ID 归一化、前后端 action/workflow 契约错位、模板幽灵引用等 7 处断裂，补齐 3 个专属工作流模板（爆款内容生成 / 多平台适配 / IP 打造），打通「专属操作 → 对话/工作流执行」链路，KPI 落真实数据。

## 2. 约束与约定

- 技术栈：前端 React 19 + TS(strict) + AntD + i18next（11 种语言）；后端 Rust workspace（analysis-engine / commands / entities / tools）
- 遵循项目既有代码风格与目录结构（见 AGENTS.md）；**禁止新增第三方依赖**
- **前端 i18n 键修改必须同步所有含该 key 的 locale 文件**（默认/回退 zh-CN，其他语言缺失 key 可不动）
- 前端格式 dprint 通过（`npm run format`）、类型 `npm run typecheck` 通过；后端 `cargo fmt` + `cargo clippy -- -D warnings` 零警告
- **不得修改**：`src-tauri/crates/analysis-engine/src/opc/industry/mod.rs` 之外的 adapter 注册逻辑（T1 只动工厂入口归一化）；不得拆 crate、不动 harness/entities 分层
- T3/T4 涉及 `ChatPage.tsx` / `WorkflowPage.tsx` 核心页面：改动须最小化、向后兼容（无 query 参数时行为与现在完全一致）
- 新增 yaml 工作流模板遵循 `config/opc/industries/content_media/workflows/content_media.yaml` 既有 schema，seed 引擎会自动发现，**不要改 seed 引擎**
- 遇到阻塞：停止并报告，不擅自扩大改动范围

## 3. 任务清单（Trae 逐项执行，完成一项勾一项）

### T1 后端行业 ID 归一化（修复：真实环境 KPI/分析/工作流执行报「行业适配器不存在」）

- 背景：前端路由/参数用连字符 `content-media`，`IndustryAdapterFactory::create` 只匹配下划线 `content_media` → `opc_get_industry_dashboard` / `opc_get_industry_workflow_steps` / `opc_get_industry_automation_rules` / `opc_run_automation_rules` / `opc_execute_analysis` / `opc_execute_workflow` 全部失败（全行业共性问题，浏览器预览被 browserMock 掩盖）
- 做法：在 `IndustryAdapterFactory::create` 入口做归一化——`let id = industry_id.trim().replace('-', "_")`，用归一化后的 id 做 match；错误消息仍用原始入参保持可读。全项目 7 个调用点自动受益，**不要逐个改调用点**
- 涉及文件：`src-tauri/crates/analysis-engine/src/opc/industry/mod.rs`（约 288 行 `pub fn create`）
- 验证：`cargo check -p axagent-analysis-engine`；单测可选加 `create("content-media").is_some()` 断言

### T2 前后端 action 契约统一（修复：前端点「文章写作」报「操作不存在」）

- 背景：前端 `INDUSTRY_CONFIGS["content-media"].actions` 用 `cm-article`(workflow)/`cm-seo`(conversation)/`cm-video`(workflow)/`cm-calendar`(workflow)；后端 `content_media_config()` 是 `cm-writing`/`cm-seo`/`cm-video`/`cm-calendar` 且**全部 Conversation**（有完整 system_prompt/user_prompt）→ 前端 `cm-article` 后端不存在，`cm-video`/`cm-calendar` type 错位
- 做法（前端对齐后端，后端不动）：
  - `src/pages/IndustryPage.tsx` `INDUSTRY_CONFIGS["content-media"].actions`：`cm-article` → `cm-writing`；`cm-video`、`cm-calendar` 的 `type` 改为 `"conversation"`；`cm-seo` 保持
  - i18n：所有含 `cm-article` key 的 locale 中 `opc.industry.actions.content-media.cm-article` → `cm-writing`（名称文本「文章写作」不变）
- 改后 4 个 action 全部走 conversation 分支（`opc_build_industry_prompt` → 创建会话 → 跳 `/chat?conversationId=&prompt=`，由 T3 消费）
- 涉及文件：`src/pages/IndustryPage.tsx`（234-246 行）、`src/i18n/locales/*.json`
- 验证：`npm run typecheck`；浏览器预览点「文章写作/视频脚本/内容日历」能进入对话（配合 T3）

### T3 ChatPage 支持 URL query 初始化（修复：`/chat?conversationId=&prompt=&workflow=` 参数被静默丢弃）

- 背景：`handleAction` 跳 `/chat?conversationId={id}&prompt={userPrompt}`、`handleUseWorkflow` 跳 `/chat?workflow={id}`，但 ChatPage 不读任何 URL query → 会话创建成功但 prompt/工作流上下文丢失，用户看到空会话
- 做法：`src/pages/ChatPage.tsx` 新增 `useSearchParams()` + 初始化 useEffect（仅在页面挂载时消费一次，消费后 `setSearchParams` 清理参数，避免刷新重复触发）：
  - `conversationId`：若该会话存在则 `setActiveConversation` 打开它
  - `prompt`：向该会话注入初始 user prompt（复用既有发送/编辑机制，参考 `conversationStoreSend` 或 chat 输入区 API）
  - `workflow=<template_id>`：按 template_id 加载工作流模板（复用既有模板加载/执行机制——搜索项目中模板执行入口，如 `run_workflow`/模板 API，禁止新造机制），创建会话并附加模板执行上下文
  - 三个参数可组合出现，需处理顺序（先打开会话，再注入内容）
- 涉及文件：`src/pages/ChatPage.tsx`
- 验证：浏览器预览下 `/chat?workflow=workflow-cm-viral-content` 能创建带该工作流上下文的会话；`conversationId+prompt` 能打开会话并注入 prompt；无参数时行为与现在完全一致

### T4 WorkflowPage 支持 template/industry query（修复：workflow 型 action 死链接）

- 背景：`/workflow/new?industry={id}&template={key}` 的 query 无人消费，WorkflowPage 只用内部 state → 点「文章写作」等卡片只打开市场列表
- 做法：`src/pages/WorkflowPage.tsx` 新增 `useSearchParams()`：
  - `template` 存在时初始化进入编辑器全屏模式并预选该模板（初始化 `editingTemplateId` 或等价 state）
  - `industry` 透传给模板创建上下文（`WorkflowEditor` 支持则使用，不支持则忽略、不得报错）
  - 无 query 时行为与现在完全一致
- 涉及文件：`src/pages/WorkflowPage.tsx`
- 验证：`/workflow/new?industry=content-media&template=workflow-cm-viral-content` 直接打开编辑器并加载该模板；无参数时打开市场列表

### T5 前端 workflows id 对齐真实模板（修复：专属工作流点击后指向无效模板）

- 背景：前端 `INDUSTRY_CONFIGS["content-media"].workflows` 的 id 是 `wf-cm-1/2/3`，后端 `content_media_config()` 的 template_id 是 `workflow-cm-viral-content`/`workflow-cm-multi-platform`/`workflow-cm-ip-building`（T6 将补齐这些模板）→ 前端跳 `/chat?workflow=wf-cm-1` 找不到模板
- 做法：
  - `src/pages/IndustryPage.tsx` `INDUSTRY_CONFIGS["content-media"].workflows`：`wf-cm-1` → `workflow-cm-viral-content`、`wf-cm-2` → `workflow-cm-multi-platform`、`wf-cm-3` → `workflow-cm-ip-building`（顺序对应「爆款内容生成/多平台适配/IP 打造方案」）
  - i18n：所有含 `wf-cm-1/2/3` key 的 locale 中 `opc.industry.workflows.content-media.wf-cm-*` 键名改为新 id（名称文本不变）
  - `handleUseWorkflow` 跳转 URL 自动变为 `/chat?workflow={真实模板id}`（配合 T3 生效）
- 涉及文件：`src/pages/IndustryPage.tsx`（241-245 行）、`src/i18n/locales/*.json`
- 验证：`npm run typecheck`；点「爆款内容生成」卡片跳 `/chat?workflow=workflow-cm-viral-content` 且 T3 生效后能加载模板

### T6 补齐 3 个专属工作流模板（Rust 种子化到数据库，与股票业务一致）

- 背景：后端声明 `workflow-cm-viral-content`/`workflow-cm-multi-platform`/`workflow-cm-ip-building` 但 config 下无对应定义 → 永远无法执行。之前用 yaml 文件方案**已废弃**。
- **强制要求：与股票业务（`stock_analysis_setup/seed_stock_analysis.rs`）完全一致的工作流模式**——Rust 代码内嵌定义 → 启动时 seed 到 `workflow_template` 表 → 用户可在工作流编辑器编辑：
  1. **删除 yaml 方案**：删除 `config/opc/industries/content_media/workflows/` 下的 `workflow-cm-viral-content.yaml`、`workflow-cm-multi-platform.yaml`、`workflow-cm-ip-building.yaml`（内容媒体行业工作流不再走 yaml 扫描）
  2. **新建 Rust seed 函数**：参考 `src-tauri/src/commands/stock_analysis_setup/seed_stock_analysis.rs` 的模式，新建 `src-tauri/src/commands/content_media_setup/`（或复用现有 opc 相关 seed 文件）：
     - 用 `axagent_harness::workflow_types` 的 `WorkflowNode`/`WorkflowEdge`/`Variable` 等结构体构建 4 个模板：`workflow-cm-viral-content`（爆款内容生成：选题策划→内容创作[工具 OpcCreateBlogPost]→优化打磨）、`workflow-cm-multi-platform`（多平台适配：内容创作→平台适配→分发策略）、`workflow-cm-ip-building`（IP 打造方案：人设定位→内容规划→粉丝运营），以及基础模板 `workflow-content-media`（替代 yaml 的 content_media.yaml）
     - 工具注入：`OpcCreateBlogPost` / `OpcListBlogPosts` / `OpcCreateLandingPage` / `OpcListLandingPages`（必须是 tools crate 已注册名）
     - **版本管理**：`TEMPLATE_VERSION` + 版本快照（跳过覆盖用户修改），`is_preset=true, is_editable=true`（用户可在编辑器编辑）
  3. **注册 seed 入口**：在启动初始化链（参考 `ensure_stock_analysis_experts_seeded` 的调用方式，见 `src-tauri/src/lib.rs`）注册新 seed 函数
  4. 清理：yaml 扫描引擎若只为 content_media 服务可保留（其他行业可能仍用），**但 content_media 的 4 个模板不得再来自 yaml**
- 涉及文件：新增 Rust seed 文件（如 `src-tauri/src/commands/content_media_setup/seed_content_media_workflows.rs` + `mod.rs`）、`src-tauri/src/lib.rs`（注册入口）、删除 3 个 yaml
- 验证：`cargo check`；启动后 DB `workflow_template` 表含 `workflow-content-media` + 3 个新模板（`is_preset=true, is_editable=true`）；前端工作流编辑器（`/workflow`）能看到并打开编辑这 4 个模板

### T7 CMO 角色工具白名单补 OpcCreateBlogPost（修复：角色直呼工具创建博客失败）

- 背景：`opc_setup` 中 `cmo-content-strategist` 角色白名单有 `OpcListBlogPosts` 但缺 `OpcCreateBlogPost`，而内容工作流注入该工具 → 角色直呼时失败
- 做法：`src-tauri/src/commands/opc_setup/mod.rs`（约 119-131 行）`cmo-content-strategist` 的工具白名单数组补 `"OpcCreateBlogPost"`
- 涉及文件：`src-tauri/src/commands/opc_setup/mod.rs`
- 验证：`cargo check`；角色 seed 后工具列表含该工具

### T8 KPI 落真实数据（修复：仪表盘 KPI 恒为 0）

- 背景：`runtime.yaml` 的 kpi_sources 全 `constant: 0`；内建 adapter `content_media.rs` 的 `compute_kpis` 实际被 dashboard 命令消费
- 做法：`src-tauri/crates/analysis-engine/src/opc/industry/content_media.rs` 的 `compute_kpis`：
  - `content_count` = `opc_blog_posts` 表 COUNT(*)
  - `page_views` = `opc_blog_posts` 表 SUM(view_count)（表已有 `view_count: u32` 字段）
  - `conversion_rate` / `content_engagement` 无对应数据字段 → 保持 0 或中性值，注释说明
  - 查询复用 data_service（`DefaultDataService`）既有接口或注入 DAO 查询 `opc_blog_posts`，遵循既有代码模式；不得直接拼 SQL 字符串
- 涉及文件：`src-tauri/crates/analysis-engine/src/opc/industry/content_media.rs`（`compute_kpis`，约 143-171 行）
- 验证：`cargo test -p axagent-analysis-engine`（若有相关测试）；Tauri 环境创建 1 篇博客后 `opc_get_industry_dashboard(content-media)` 的 content_count ≥ 1

### T9 自测与验证（贯穿全流程）

- 后端：`cargo fmt`、`cargo clippy -- -D warnings`（src-tauri 下）、`cargo check`
- 前端：`npm run format`、`npm run typecheck`、`npm run lint`（如配置）
- 浏览器预览回归：OpcPage 8 tab、IndustryPage 9 行业、SitesTab 博客/落地页 CRUD 不受影响
- 汇报：每个 T 完成后更新本清单勾选状态，说明验证结果

## 4. 验收标准

- [ ] **功能**：Tauri 真实环境 `opc_get_industry_dashboard(content-media)` / `opc_execute_workflow(content-media)` / `opc_execute_analysis(content-media)` 不再报「行业适配器不存在」
- [ ] **功能**：内容媒体 4 个专属操作（文章写作/SEO 优化/视频脚本/内容日历）点击后进入对话并注入正确 prompt，不再报「操作不存在」
- [ ] **功能**：3 个专属工作流模板在 DB 中真实存在，从 IndustryPage 点击「爆款内容生成/多平台适配/IP 打造方案」能创建带模板上下文的会话
- [ ] **功能**：`/workflow/new?industry=content-media&template=workflow-cm-viral-content` 直接打开编辑器并预选模板
- [ ] **功能**：CMO 角色工具白名单含 `OpcCreateBlogPost`；创建 1 篇博客后 KPI content_count 变化
- [ ] **代码**：前端 typecheck/dprint 通过；后端 clippy 零警告、rustfmt 通过
- [ ] **测试**：T1 归一化可加工厂单测；T8 若有现有测试则保持通过
- [ ] **不破坏**：无 query 参数时 ChatPage/WorkflowPage 行为与改动前一致；browserMock 预览（已确认无 cm-* key 依赖）不受影响；9 行业页面均可打开

## 5. 交付说明

- 完成每个 T 任务后：更新本清单勾选状态（`- [x]`）、运行对应验证命令，在 Trae 中汇报结果
- 遇到阻塞时：停止并报告，不擅自扩大改动范围；涉及新增文件/改动核心页面的决策先说明理由
- 全部完成后：运行 T9 全量验证，输出最终自测结果供 WorkBuddy 审核
