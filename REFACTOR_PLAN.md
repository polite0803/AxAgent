# agent/mod.rs 模块解耦重构计划

**目标：** 将 `agent/mod.rs` 中 26 个跨领域函数拆分到对应独立模块，使每个模块只包含单一领域的功能实现，消除模块边界崩溃问题。

**架构：** 遵循项目现有模块组织模式（每个 `commands/xxx/` 目录独立声明 `pub mod`，在 `register_commands!` 中按 `commands::xxx::func` 路径注册），保持 Tauri 命令名不变（前端零改动），仅改变后端模块归属。

**技术栈：** Rust 2021 · Tauri 2 · Tokio · 项目现有 crate 体系

---

## 前置约束

1. **命令名不变**：前端通过 `invoke("command_name")` 调用，Tauri 按函数路径匹配，不按命令名字符串匹配。因此移动函数到新模块后，前端 `invoke` 调用无需修改。
2. **注册两步走**：`commands/mod.rs` 声明模块 + `register_commands!` 注册函数路径。
3. **可见性**：`skill_execution.rs` 中的 `pub(super)` 项需改为 `pub(crate)` 才能被新模块引用。
4. **Harness 架构**：跨模块调用必须通过 `AppState` 共享状态，不直接相互引用。

---

## 任务拆分

### 任务 1：将 7 个工作流函数迁移到 `commands/workflows/` 模块

**变更文件：**

- 新建：`src-tauri/src/commands/workflows/mod.rs`
- 修改：`src-tauri/src/commands/mod.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`
- 修改：`src-tauri/src/commands/agent/skill_execution.rs`（提升可见性）

**迁移函数：**

| 函数名                              | 用途                         | 依赖                                                                                       |
| ----------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------------ |
| `workflow_create`                   | 创建工作流                   | `AppState.work_engine`                                                                     |
| `workflow_execute`                  | 执行工作流（含 LLM 步骤）    | `AppState.work_engine`, `AppState.local_tool_registry`                                     |
| `workflow_get_status`               | 获取工作流状态               | `AppState.work_engine`                                                                     |
| `workflow_cancel`                   | 取消执行中的工作流           | `AppState.work_engine`                                                                     |
| `workflow_list`                     | 列出所有工作流               | `AppState.work_engine`                                                                     |
| `workflow_get_steps`                | 获取工作流步骤（DAG 可视化） | `AppState.work_engine`                                                                     |
| `get_conversation_workflow_preview` | 获取对话工作流预览           | `AppState.harness.db()`, `skill_execution::SkillStep`, `skill_execution::infer_agent_role` |

**附带迁移：**

| 附属项                                      | 当前文件             | 迁移到             |
| ------------------------------------------- | -------------------- | ------------------ |
| `WorkflowCreateRequest` struct              | `agent/mod.rs:L2670` | `workflows/mod.rs` |
| `WorkflowCreateResponse` struct             | `agent/mod.rs:L2678` | `workflows/mod.rs` |
| `ConversationWorkflowPreview` struct        | `agent/mod.rs:L2988` | `workflows/mod.rs` |
| `skill_steps_to_nodes_edges_with_offset` fn | `agent/mod.rs:L2993` | `workflows/mod.rs` |

**步骤 1.1：** 提升 `skill_execution.rs` 中依赖项的可见性

```rust
// skill_execution.rs 中修改：
// pub(super) struct SkillStep -> pub(crate) struct SkillStep
// pub(super) fn infer_agent_role -> pub(crate) fn infer_agent_role
```

**步骤 1.2：** 创建 `src-tauri/src/commands/workflows/mod.rs`

```rust
// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::agent::skill_execution::{self, SkillStep};
use crate::commands::agent::{agent_err, ErrorResponse};
use crate::commands::error_code::agent as agent_err;
use crate::commands::spawn_guard::SpawnGuard;
use axagent_harness::workflow_types;
use axagent_runtime::work_engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

// ── 类型定义 ──

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowCreateRequest {
    pub name: String,
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
}

#[derive(Debug, Serialize)]
pub struct WorkflowCreateResponse {
    pub workflow_id: String,
    pub name: String,
    pub step_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ConversationWorkflowPreview {
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
    pub skill_execution_order: Vec<String>,
    pub skill_count: usize,
}

// ── 命令函数 ──

#[tauri::command]
pub async fn workflow_create(
    app_state: State<'_, AppState>,
    request: WorkflowCreateRequest,
) -> Result<WorkflowCreateResponse, String> {
    // ... 从 agent/mod.rs 迁移的完整实现
}

#[tauri::command]
pub async fn workflow_execute(
    app: tauri::AppHandle,
    app_state: State<'_, AppState>,
    workflow_id: String,
    model_id: Option<String>,
    provider_id: Option<String>,
    variables: Option<Vec<workflow_types::Variable>>,
) -> Result<String, String> {
    // ... 从 agent/mod.rs 迁移的完整实现
}

#[tauri::command]
pub async fn workflow_get_status(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Value, String> {
    // ... 从 agent/mod.rs 迁移的完整实现
}

#[tauri::command]
pub async fn workflow_cancel(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Value, String> {
    // ... 从 agent/mod.rs 迁移的完整实现
}

#[tauri::command]
pub async fn workflow_list(app_state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    // ... 从 agent/mod.rs 迁移的完整实现
}

#[tauri::command]
pub async fn workflow_get_steps(
    app_state: State<'_, AppState>,
    workflow_id: String,
) -> Result<Vec<Value>, String> {
    // ... 从 agent/mod.rs 迁移的完整实现
}

#[tauri::command]
pub async fn get_conversation_workflow_preview(
    app_state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationWorkflowPreview, String> {
    // ... 从 agent/mod.rs 迁移的完整实现
}

// ── 辅助函数 ──

fn skill_steps_to_nodes_edges_with_offset(
    skill_steps: &[SkillStep],
    skill_id: &str,
    base_y: f64,
) -> (Vec<Value>, Vec<Value>) {
    // ... 从 agent/mod.rs 迁移的完整实现
}
```

**步骤 1.3：** 在 `commands/mod.rs` 添加模块声明

```rust
pub mod workflows;  // 新增
```

**步骤 1.4：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::workflow_create,
commands::agent::workflow_execute,
commands::agent::workflow_get_status,
commands::agent::workflow_cancel,
commands::agent::workflow_list,
commands::agent::get_conversation_workflow_preview,
commands::agent::workflow_get_steps,

// 修改后：
commands::workflows::workflow_create,
commands::workflows::workflow_execute,
commands::workflows::workflow_get_status,
commands::workflows::workflow_cancel,
commands::workflows::workflow_list,
commands::workflows::get_conversation_workflow_preview,
commands::workflows::workflow_get_steps,
```

**步骤 1.5：** 从 `agent/mod.rs` 中删除以上 7 个函数和相关类型定义。

**步骤 1.6：** 运行编译验证

```powershell
cd src-tauri && cargo check 2>&1
```

---

### 任务 2：将 4 个子代理函数迁移到 `commands/sub_agent/` 模块

**变更文件：**

- 新建：`src-tauri/src/commands/sub_agent/mod.rs`
- 修改：`src-tauri/src/commands/mod.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名                   | 用途               | 依赖                          |
| ------------------------ | ------------------ | ----------------------------- |
| `sub_agent_list`         | 列出所有子代理     | `AppState.sub_agent_registry` |
| `sub_agent_get`          | 获取指定子代理     | `AppState.sub_agent_registry` |
| `sub_agent_get_children` | 获取父代理的子代理 | `AppState.sub_agent_registry` |
| `sub_agent_get_messages` | 获取代理待处理消息 | `AppState.sub_agent_registry` |

**步骤 2.1：** 创建 `src-tauri/src/commands/sub_agent/mod.rs`

```rust
// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error_code::agent as agent_err;
use crate::commands::error::ErrorResponse;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn sub_agent_list(app_state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let agents = registry.list_all();
    Ok(agents.iter().filter_map(|a| serde_json::to_value(a).ok()).collect())
}

#[tauri::command]
pub async fn sub_agent_get(
    app_state: State<'_, AppState>,
    agent_id: String,
) -> Result<Value, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let agent = registry.get(&agent_id).ok_or_else(|| ErrorResponse::err(agent_err::NOT_FOUND))?;
    serde_json::to_value(agent).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sub_agent_get_children(
    app_state: State<'_, AppState>,
    parent_id: String,
) -> Result<Vec<Value>, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let children = registry.get_children(&parent_id);
    Ok(children.iter().filter_map(|c| serde_json::to_value(c).ok()).collect())
}

#[tauri::command]
pub async fn sub_agent_get_messages(
    app_state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<Value>, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let messages = registry.message_bus().peek_all(&agent_id);
    Ok(messages.iter().filter_map(|m| serde_json::to_value(m).ok()).collect())
}
```

**步骤 2.2：** 在 `commands/mod.rs` 添加 `pub mod sub_agent;`

**步骤 2.3：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::sub_agent_list,
commands::agent::sub_agent_get,
commands::agent::sub_agent_get_children,
commands::agent::sub_agent_get_messages,

// 修改后：
commands::sub_agent::sub_agent_list,
commands::sub_agent::sub_agent_get,
commands::sub_agent::sub_agent_get_children,
commands::sub_agent::sub_agent_get_messages,
```

**步骤 2.4：** 从 `agent/mod.rs` 删除以上 4 个函数。

**步骤 2.5：** 运行 `cargo check` 验证。

---

### 任务 3：将 4 个共享记忆函数迁移到 `commands/memory/` 模块（合并到现有 `commands/memory.rs`）

**变更文件：**

- 修改：`src-tauri/src/commands/memory.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名                | 用途                     | 依赖                      |
| --------------------- | ------------------------ | ------------------------- |
| `shared_memory_list`  | 列出命名空间中的共享记忆 | `AppState.shared_memory`  |
| `shared_memory_get`   | 获取指定共享记忆条目     | `AppState.shared_memory`  |
| `shared_memory_stats` | 获取共享记忆统计         | `AppState.shared_memory`  |
| `memory_flush`        | 手动刷新记忆（前端触发） | `AppState.memory_service` |

**步骤 3.1：** 在现有 `commands/memory.rs` 末尾追加 4 个函数实现。

**步骤 3.2：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::shared_memory_list,
commands::agent::shared_memory_get,
commands::agent::shared_memory_stats,
commands::agent::memory_flush,

// 修改后：
commands::memory::shared_memory_list,
commands::memory::shared_memory_get,
commands::memory::shared_memory_stats,
commands::memory::memory_flush,
```

**步骤 3.3：** 从 `agent/mod.rs` 删除以上 4 个函数。

**步骤 3.4：** 运行 `cargo check` 验证。

---

### 任务 4：将 4 个学习/进化函数迁移到 `commands/evolution.rs` 模块（合并到现有文件）

**变更文件：**

- 修改：`src-tauri/src/commands/evolution.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名                   | 用途                        | 依赖                                                             |
| ------------------------ | --------------------------- | ---------------------------------------------------------------- |
| `pattern_list`           | 获取学习模式（高价值/失败） | `AppState.pattern_learner`, `AppState.trajectory_storage`        |
| `cross_session_insights` | 获取跨会话洞察              | `AppState.cross_session_learner`                                 |
| `skill_evolution_start`  | 启动技能进化                | `AppState.trajectory_storage`, `AppState.skill_evolution_engine` |
| `skill_evolution_status` | 获取进化状态                | `AppState.skill_evolution_engine`                                |

**步骤 4.1：** 在现有 `commands/evolution.rs` 末尾追加 4 个函数实现。

**步骤 4.2：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::pattern_list,
commands::agent::cross_session_insights,
commands::agent::skill_evolution_start,
commands::agent::skill_evolution_status,

// 修改后：
commands::evolution::pattern_list,
commands::evolution::cross_session_insights,
commands::evolution::skill_evolution_start,
commands::evolution::skill_evolution_status,
```

**步骤 4.3：** 从 `agent/mod.rs` 删除以上 4 个函数。

**步骤 4.4：** 运行 `cargo check` 验证。

---

### 任务 5：将 4 个用户配置函数迁移到 `commands/user_profile.rs` 模块（合并到现有文件）

**变更文件：**

- 修改：`src-tauri/src/commands/user_profile.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名                        | 用途             | 依赖                    |
| ----------------------------- | ---------------- | ----------------------- |
| `user_profile_get`            | 获取当前用户配置 | `AppState.user_profile` |
| `user_profile_set_preference` | 更新用户偏好     | `AppState.user_profile` |
| `user_profile_set_expertise`  | 设置领域专业水平 | `AppState.user_profile` |
| `user_profile_export_md`      | 导出为 USER.md   | `AppState.user_profile` |

**步骤 5.1：** 在现有 `commands/user_profile.rs` 末尾追加 4 个函数实现。

**步骤 5.2：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::user_profile_get,
commands::agent::user_profile_set_preference,
commands::agent::user_profile_set_expertise,
commands::agent::user_profile_export_md,

// 修改后：
commands::user_profile::user_profile_get,
commands::user_profile::user_profile_set_preference,
commands::user_profile::user_profile_set_expertise,
commands::user_profile::user_profile_export_md,
```

**步骤 5.3：** 从 `agent/mod.rs` 删除以上 4 个函数。

**步骤 5.4：** 运行 `cargo check` 验证。

---

### 任务 6：将 `record_feedback` 迁移到 `commands/agent_analytics/` 模块

**变更文件：**

- 修改：`src-tauri/src/commands/agent_analytics/mod.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名            | 用途                     | 依赖                         |
| ----------------- | ------------------------ | ---------------------------- |
| `record_feedback` | 记录反馈信号用于实时学习 | `AppState.realtime_learning` |

**步骤 6.1：** 在现有 `commands/agent_analytics/` 末尾追加 `record_feedback` 函数。

**步骤 6.2：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::record_feedback,

// 修改后：
commands::agent_analytics::record_feedback,
```

**步骤 6.3：** 从 `agent/mod.rs` 删除 `record_feedback` 函数。

**步骤 6.4：** 运行 `cargo check` 验证。

---

### 任务 7：将 `adaptation_status` 迁移到 `commands/evolution.rs` 模块

**变更文件：**

- 修改：`src-tauri/src/commands/evolution.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名              | 用途               | 依赖                         |
| ------------------- | ------------------ | ---------------------------- |
| `adaptation_status` | 获取当前自适应状态 | `AppState.realtime_learning` |

**步骤 7.1：** 在 `commands/evolution.rs` 末尾追加 `adaptation_status` 函数。

**步骤 7.2：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::adaptation_status,

// 修改后：
commands::evolution::adaptation_status,
```

**步骤 7.3：** 从 `agent/mod.rs` 删除 `adaptation_status` 函数。

**步骤 7.4：** 运行 `cargo check` 验证。

---

### 任务 8：将 `classify_route` 迁移到 `commands/smart_router.rs` 模块

**变更文件：**

- 新建：`src-tauri/src/commands/smart_router.rs`
- 修改：`src-tauri/src/commands/mod.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名           | 用途                           | 依赖                                      |
| ---------------- | ------------------------------ | ----------------------------------------- |
| `classify_route` | 分类用户提示并返回模型路由建议 | `crate::smart_router::classify_and_route` |

**附带迁移：**

| 附属项                        | 当前文件             | 迁移到            |
| ----------------------------- | -------------------- | ----------------- |
| `ClassifyRouteRequest` struct | `agent/mod.rs:L3390` | `smart_router.rs` |

**步骤 8.1：** 创建 `src-tauri/src/commands/smart_router.rs`

```rust
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassifyRouteRequest {
    pub prompt: String,
}

#[tauri::command]
pub fn classify_route(request: ClassifyRouteRequest) -> crate::smart_router::RouteDecision {
    crate::smart_router::classify_and_route(&request.prompt)
}
```

**步骤 8.2：** 在 `commands/mod.rs` 添加 `pub mod smart_router;`

**步骤 8.3：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::classify_route,

// 修改后：
commands::smart_router::classify_route,
```

**步骤 8.4：** 从 `agent/mod.rs` 删除 `classify_route` 和 `ClassifyRouteRequest`。

**步骤 8.5：** 运行 `cargo check` 验证。

---

### 任务 9：将 `agent_estimate_complexity` 迁移到 `commands/agent_advanced/` 模块

**变更文件：**

- 修改：`src-tauri/src/commands/agent_advanced/mod.rs`
- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/register_commands.rs`

**迁移函数：**

| 函数名                      | 用途           | 依赖                                             |
| --------------------------- | -------------- | ------------------------------------------------ |
| `agent_estimate_complexity` | 估算任务复杂度 | `axagent_trajectory::estimate_complexity_public` |

**步骤 9.1：** 在 `commands/agent_advanced/mod.rs` 末尾追加 `agent_estimate_complexity` 函数。

**步骤 9.2：** 在 `register_commands.rs` 修改注册路径

```rust
// 修改前：
commands::agent::agent_estimate_complexity,

// 修改后：
commands::agent_advanced::agent_estimate_complexity,
```

**步骤 9.3：** 从 `agent/mod.rs` 删除 `agent_estimate_complexity` 函数。

**步骤 9.4：** 运行 `cargo check` 验证。

---

### 任务 10：清理 `agent/mod.rs` 残留及最终验证

**变更文件：**

- 修改：`src-tauri/src/commands/agent/mod.rs`
- 修改：`src-tauri/src/commands/agent/skill_execution.rs`

**步骤 10.1：** 清理 `agent/mod.rs` 中已无用的 import 语句。

删除以下不再需要的 import：

```rust
use axagent_harness::workflow_types; // 仅工作流函数使用
// 检查其他仅被迁移函数使用的 import
```

**步骤 10.2：** 清理 `agent/mod.rs` 中残留的注释/分隔线标记（如 `// P3: Multi-agent`, `// P5: Pattern Learning` 等）。

**步骤 10.3：** `agent/mod.rs` 最终保留的 17 个函数（Agent 核心领域）：

```
agent_query, agent_approve, agent_respond_ask, agent_cancel,
agent_is_running, agent_pause, agent_resume, agent_is_paused,
agent_runtime_stats, agent_resolve_model, agent_update_session,
agent_get_session, agent_ensure_workspace,
agent_backup_and_clear_sdk_context, agent_restore_sdk_context_from_backup,
agent_steer, agent_estimate_complexity
```

**步骤 10.4：** 全局编译验证

```powershell
cd src-tauri
cargo check 2>&1
cargo fmt
cargo clippy -- -D warnings 2>&1
```

**步骤 10.5：** 前端类型检查

```powershell
npm run typecheck
```

---

## 执行顺序

按依赖关系递增排列，每个任务完成后立即验证：

```
任务 1 (工作流) ──→ 任务 2 (子代理) ──→ 任务 3 (共享记忆)
    ↓
任务 4 (学习/进化) + 任务 7 (自适应) ──→ 同一文件合并
    ↓
任务 5 (用户配置) ──→ 任务 6 (反馈) ──→ 任务 8 (路由) ──→ 任务 9 (复杂度)
    ↓
任务 10 (清理 + 最终验证)
```

---

## 风险评估

| 风险                         | 等级 | 缓解措施                                                    |
| ---------------------------- | ---- | ----------------------------------------------------------- |
| 编译错误（缺失 import）      | 低   | 每步执行 `cargo check`，及时修复                            |
| `pub(super)` 可见性不足      | 中   | 任务 1 先提升 `skill_execution.rs` 的可见性                 |
| `commands/mod.rs` 模块名冲突 | 低   | `smart_router` 需确认与 `lib.rs` 中声明不冲突               |
| 前端 `invoke` 调用失败       | 无   | Tauri 按函数路径匹配，不按字符串名，无需前端改动            |
| 循环依赖                     | 低   | 所有迁移函数仅依赖 `AppState`，不交叉引用其他 commands 模块 |

---

## 完成标准

1. `agent/mod.rs` 仅包含 17 个 Agent 核心领域函数
2. 26 个跨领域函数全部迁移到正确的目标模块
3. `cargo check` 零错误
4. `cargo clippy -- -D warnings` 零警告
5. `cargo fmt` 通过
6. 前端 `npm run typecheck` 通过
7. 所有 Tauri 命令名保持不变
