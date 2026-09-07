// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流模板级生命周期钩子协议。
//!
//! ## 设计原则（业务中立）
//!
//! 通用层（harness / rt-workflow）只认协议与运行时注册表，**不感知任何业务名**：
//! - 钩子实现由业务侧（下游 fork）编写并通过
//!   `WorkEngine::register_lifecycle_hook` 在运行时注入（OnceLock 槽位模式，
//!   先例：`axagent-tools` 的 `run_workflow::set_workflow_executor`）。
//! - 模板通过 `hooks_config`（workflow_templates 表 JSON 列）声明使用哪些钩子：
//!   `{"pre_exec": ["hook-a"], "post_exec": ["hook-b"]}`。
//! - 引擎按声明查注册表：未注册的钩子名 → `warn` 跳过不阻断
//!   （允许钩子实现滞后于模板声明）。
//!
//! ## 语义
//!
//! - `pre_exec`：DAG 主循环启动前调用，可增强/替换执行变量（如注入业务上下文、
//!   历史教训）；返回 `Err` 则阻断本次执行（工作流状态置为 failed）。
//! - `post_exec`：DAG 到达终态并构建 output 后调用，用于观测/持久化；
//!   失败仅 `warn` 不阻断（结果已产生，不可回滚）。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::workflow_types::Variable;

/// 模板级生命周期钩子。业务侧实现 + 注册，WorkEngine 按模板声明调用。
///
/// 实现必须 `Send + Sync`（引擎多任务共享注册表）。
#[async_trait]
pub trait WorkflowLifecycleHook: Send + Sync {
    /// 钩子名。模板 `hooks_config` 中按此名引用。
    fn name(&self) -> &str;

    /// DAG 主循环启动前调用：可增强变量（如注入业务上下文）。
    ///
    /// 返回的 `Vec<Variable>` 会覆盖写回执行上下文（同名变量以返回值为准）；
    /// 返回 `Err` 则阻断本次执行。
    async fn pre_exec(&self, ctx: HookExecContext) -> Result<Vec<Variable>, String>;

    /// DAG 到达终态后调用：观测/持久化。
    ///
    /// 失败仅 warn 不阻断（结果已产生，不可回滚）。
    async fn post_exec(&self, ctx: HookExecContext, outcome: &HookOutcome) -> Result<(), String>;
}

/// 钩子执行上下文。
#[derive(Debug, Clone)]
pub struct HookExecContext {
    /// 工作流模板 ID。
    pub template_id: String,
    /// 本次执行 ID。
    pub execution_id: String,
    /// 调用方传入的工作流输入（未经 schema 校验的原值）。
    pub input: Option<serde_json::Value>,
    /// pre_exec 入参：模板级变量列表（来自 RunOptions.variables）。
    /// post_exec 阶段为空列表。
    pub variables: Vec<Variable>,
}

/// 工作流终态快照（post_exec 用）。
#[derive(Debug, Clone)]
pub struct HookOutcome {
    /// 终态：`"completed"` / `"partially_completed"` / `"failed"` / `"cancelled"`。
    pub status: String,
    /// 工作流最终输出（经 output_schema 过滤或 End 节点聚合后的结果）。
    pub output: Option<serde_json::Value>,
    /// 节点结果 map（node_id / output_var → 输出值），post_exec 提取业务决策用。
    pub results: serde_json::Value,
}

/// 模板声明使用的生命周期钩子列表（对应 workflow_templates.hooks_config JSON 列）。
///
/// JSON 形如 `{"pre_exec": ["hook-a"], "post_exec": ["hook-b"]}`；
/// 两端均为空等价于 NULL（无钩子，与旧模板行为一致）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct WorkflowHooksConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_exec: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_exec: Vec<String>,
}

impl WorkflowHooksConfig {
    /// 两端钩子声明均为空 → 视为无钩子。
    pub fn is_empty(&self) -> bool {
        self.pre_exec.is_empty() && self.post_exec.is_empty()
    }
}
