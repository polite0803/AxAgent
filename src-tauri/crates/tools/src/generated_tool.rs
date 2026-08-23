// SPDX-License-Identifier: AGPL-3.0-only

//! 进化生成工具适配器 —— 将 `trajectory_types::GeneratedTool`（代码型工具）包装为 `Tool` trait 实现。
//!
//! 该适配器打通"进化产物 → 运行时工具注册"的链路：
//! - 注册后对 Agent 可见（`get_chat_tools()` 可发现、可被 LLM 选中调用）；
//! - 卸载后立即不可见（`unregister_runtime_tool`）。
//!
//! 分层执行（阶段四）：
//! - `RhaiScript`（计算型产物）：`call()` 将 `tool.code` 编译为 Rhai AST 并由
//!   Rhai 引擎真正执行（纯计算 / 数据处理），复用 `crate::rhai_engine`，不新造执行器。
//! - `WorkflowDag`（编排型产物）：映射为 `WorkflowGenome` 由 rt-workflow 引擎执行（T4.3 接入）。

use crate::rhai_engine::{compile_script, create_rhai_engine, execute_rhai_ast};
use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_harness::trajectory_types::EvolutionArtifactKind;
use axagent_harness::workflow_evolution::{
    EvolutionArtifactValidator, ExecutionFeedbackSink, WorkflowDagExecutor,
    workflow_genome_from_generated,
};
use serde_json::Value;
use std::sync::Arc;

/// 进化生成工具的 Tool trait 适配器
pub struct GeneratedToolAdapter {
    tool: axagent_harness::trajectory_types::GeneratedTool,
    input_schema: Value,
    /// 编排型产物执行器(wiring 层注入,包装 rt-workflow 引擎)。
    /// `None` 时 `WorkflowDag` 产物退回返回定义(不执行)。
    workflow_executor: Option<Arc<dyn WorkflowDagExecutor>>,
    /// 计算型产物沙箱验证器(wiring 层注入,包装自指熔断 + 危险模式检测)。
    /// `None` 时 `RhaiScript` 产物跳过沙箱验证(仅用于纯 tools 层测试)。
    sandbox_validator: Option<Arc<dyn EvolutionArtifactValidator>>,
    /// 进化产物执行反馈回传(wiring 层注入,累计真实成败到贝叶斯证据)。
    /// `None` 时跳过反馈上报(仅用于纯 tools 层测试)。
    feedback_sink: Option<Arc<dyn ExecutionFeedbackSink>>,
}

impl GeneratedToolAdapter {
    /// 从进化引擎生成的工具构造适配器。
    ///
    /// 输入 schema 使用宽松对象 schema（不限制属性），使 LLM 可自由传入
    /// 任务相关参数，同时保留在 `call` 中回显。
    pub fn new(tool: axagent_harness::trajectory_types::GeneratedTool) -> Self {
        let input_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "传入该工具执行的具体任务输入"
                }
            }
        });
        Self {
            tool,
            input_schema,
            workflow_executor: None,
            sandbox_validator: None,
            feedback_sink: None,
        }
    }

    /// 注入编排型产物执行器(wiring 层包装 rt-workflow 引擎)。
    ///
    /// 未注入时 `WorkflowDag` 产物仅返回定义,不真正执行。
    pub fn with_workflow_executor(mut self, executor: Arc<dyn WorkflowDagExecutor>) -> Self {
        self.workflow_executor = Some(executor);
        self
    }

    /// 注入计算型产物沙箱验证器(wiring 层包装自指熔断 + 危险模式检测)。
    ///
    /// 未注入时 `RhaiScript` 产物跳过沙箱验证(仅用于纯 tools 层测试)。
    pub fn with_sandbox_validator(
        mut self,
        validator: Arc<dyn EvolutionArtifactValidator>,
    ) -> Self {
        self.sandbox_validator = Some(validator);
        self
    }

    /// 注入进化产物执行反馈回传(wiring 层包装贝叶斯证据累计)。
    ///
    /// 未注入时跳过反馈上报(仅用于纯 tools 层测试)。
    pub fn with_feedback_sink(mut self, sink: Arc<dyn ExecutionFeedbackSink>) -> Self {
        self.feedback_sink = Some(sink);
        self
    }
}

#[async_trait]
impl Tool for GeneratedToolAdapter {
    fn name(&self) -> &str {
        &self.tool.name
    }

    fn description(&self) -> &str {
        &self.tool.description
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    /// 进化生成工具是"返回定义"的模板工具，不产生外部副作用，标记为只读。
    /// 保证在 ReadOnly 权限模式下也能被 `get_chat_tools_filtered` 发现。
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        self.call_inner(input, ctx).await
    }
}

impl GeneratedToolAdapter {
    /// 上报一次真实执行结果到进化证据（D4：仅真正执行的分支调用，
    /// 「未执行/返回定义」的兜底分支不上报，避免假成功污染贝叶斯证据）。
    /// D2：随执行上下文携带会话 id，统计按会话隔离。
    /// 注意：tool_id 用 `tool.name`（进化引擎以 name 为唯一业务键索引
    /// `created_tools`），而非随机 `tool.id`（UUID 每次重建都会变化，无法稳定累计）。
    fn report(&self, ctx: &ToolContext, success: bool) {
        if let Some(sink) = &self.feedback_sink {
            sink.record(ctx.conversation_id.as_deref(), &self.tool.name, success);
        }
    }

    /// `call` 的真实执行逻辑（各真正执行分支在末尾上报真实成败）。
    async fn call_inner(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        match self.tool.artifact_kind {
            EvolutionArtifactKind::RhaiScript => {
                // T4.4：执行前先过沙箱验证（自指熔断关键词 / 危险模式 / 长度限制）。
                // 验证不过 → 拒绝执行（不落地）。未注入验证器时跳过（纯 tools 层测试）。
                if let Some(validator) = &self.sandbox_validator {
                    let violations = validator.validate_code(&self.tool.code);
                    if !violations.is_empty() {
                        self.report(ctx, false);
                        return Err(ToolError::execution_failed_for(
                            &self.tool.name,
                            format!("进化工具沙箱验证未通过: {}", violations.join("; ")),
                        ));
                    }
                }
                // 计算型产物：用 Rhai 引擎真正执行脚本
                let engine = create_rhai_engine();
                let ast = compile_script(&engine, &self.tool.code).map_err(|e| {
                    self.report(ctx, false);
                    ToolError::execution_failed_for(
                        &self.tool.name,
                        format!("进化工具编译失败: {e}"),
                    )
                })?;
                // 统一把输入包装为 input 变量，保证脚本可访问
                //（execute_rhai_ast 对 object 参数按字段平铺进 scope）
                let wrapped = serde_json::json!({ "input": input });
                let result = execute_rhai_ast(&ast, wrapped, None).map_err(|e| {
                    self.report(ctx, false);
                    ToolError::execution_failed_for(
                        &self.tool.name,
                        format!("进化工具执行失败: {e}"),
                    )
                })?;
                // 字符串结果直接取值（避免 JSON 引号）；数字/对象按其 Display 输出
                let content = match result {
                    serde_json::Value::String(s) => s,
                    other => {
                        if other.is_null() {
                            "OK".to_string()
                        } else {
                            other.to_string()
                        }
                    },
                };
                self.report(ctx, true);
                Ok(ToolResult::success(content))
            },
            EvolutionArtifactKind::WorkflowDag => {
                // 编排型产物：映射为 WorkflowGenome，交由注入的 rt-workflow 执行器执行（T4.3）。
                match &self.workflow_executor {
                    Some(executor) => {
                        let genome = workflow_genome_from_generated(&self.tool).map_err(|e| {
                            self.report(ctx, false);
                            ToolError::execution_failed_for(
                                &self.tool.name,
                                format!("编排型产物映射 WorkflowGenome 失败: {e}"),
                            )
                        })?;
                        let result = executor.execute(&genome, &input).await.map_err(|e| {
                            self.report(ctx, false);
                            ToolError::execution_failed_for(
                                &self.tool.name,
                                format!("编排型产物工作流执行失败: {e}"),
                            )
                        })?;
                        let content = if result.is_null() {
                            "OK".to_string()
                        } else {
                            serde_json::to_string_pretty(&result)
                                .unwrap_or_else(|_| result.to_string())
                        };
                        self.report(ctx, true);
                        Ok(ToolResult::success(content))
                    },
                    None => {
                        // 未注入执行器（如纯 tools 层测试）：返回定义供 Agent 参照。
                        // D4：产物并未真正执行，不上报真实成败（避免假成功污染贝叶斯证据）。
                        let definition = serde_json::json!({
                            "toolName": self.tool.name,
                            "description": self.tool.description,
                            "artifactKind": self.tool.artifact_kind.as_str(),
                            "code": self.tool.code,
                            "inputSchema": self.input_schema,
                            "requestInput": input,
                            "note": "编排型进化产物：未注入执行器，仅返回定义（wiring 层注入 rt-workflow 引擎后真正执行）。",
                        });
                        let content = serde_json::to_string_pretty(&definition)
                            .unwrap_or_else(|_| self.tool.code.clone());
                        Ok(ToolResult::success(content))
                    },
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::trajectory_types::{EvolutionArtifactKind, GeneratedTool};

    /// D4 测试桩：收集上报的真实执行反馈（会话 id / tool_id / 成败）。
    #[derive(Default)]
    #[allow(clippy::disallowed_types)]
    // SAFETY: 测试桩使用 parking_lot::Mutex 保护内部数据，仅在同步测试场景中使用，无跨 await 风险。
    struct MockSink(parking_lot::Mutex<Vec<(Option<String>, String, bool)>>);

    impl ExecutionFeedbackSink for MockSink {
        fn record(&self, conversation_id: Option<&str>, tool_id: &str, success: bool) {
            self.0.lock().push((
                conversation_id.map(|s| s.to_string()),
                tool_id.to_string(),
                success,
            ));
        }
    }

    impl MockSink {
        fn snapshot(&self) -> Vec<(Option<String>, String, bool)> {
            self.0.lock().clone()
        }
    }

    #[tokio::test]
    async fn test_rhai_script_artifact_really_executes() {
        // 计算型产物：Rhai 引擎真正执行脚本，而非返回定义
        let tool = GeneratedTool::with_artifact_kind(
            "calc_double",
            "let result = input * 2;\nresult",
            "翻倍计算工具",
            EvolutionArtifactKind::RhaiScript,
        );
        let adapter = GeneratedToolAdapter::new(tool);
        let result = adapter.call(serde_json::json!(21), &ToolContext::new(".")).await;
        assert!(result.is_ok(), "测试：计算型产物应执行成功");
        let tr = result.unwrap();
        assert!(!tr.is_error);
        assert_eq!(tr.content, "42");
    }

    #[tokio::test]
    async fn test_rhai_script_artifact_string_input() {
        let tool = GeneratedTool::with_artifact_kind(
            "echo_upper",
            r#"let s = input.to_upper();
s"#,
            "大写转换工具",
            EvolutionArtifactKind::RhaiScript,
        );
        let adapter = GeneratedToolAdapter::new(tool);
        let result = adapter.call(serde_json::json!("hello"), &ToolContext::new(".")).await;
        assert!(result.is_ok(), "测试：字符串输入执行应成功");
        assert_eq!(result.unwrap().content, "HELLO");
    }

    #[tokio::test]
    async fn test_rhai_script_artifact_invalid_code_fails() {
        let tool = GeneratedTool::with_artifact_kind(
            "broken",
            "this is not valid rhai @@@",
            "非法脚本工具",
            EvolutionArtifactKind::RhaiScript,
        );
        let adapter = GeneratedToolAdapter::new(tool);
        let result = adapter.call(serde_json::json!(1), &ToolContext::new(".")).await;
        assert!(result.is_err(), "测试：非法脚本应编译失败");
        assert!(result.unwrap_err().message.contains("编译失败"));
    }

    #[tokio::test]
    async fn test_rhai_script_artifact_rejected_by_sandbox() {
        // T4.4：沙箱验证不通过 → 拒绝执行（不落地），不编译不执行
        use axagent_harness::workflow_evolution::EvolutionArtifactValidator;
        use std::sync::Arc;

        struct RejectValidator;
        impl EvolutionArtifactValidator for RejectValidator {
            fn validate_code(&self, _code: &str) -> Vec<String> {
                vec!["命中自指熔断关键词 /evolution/".to_string()]
            }
        }

        let tool = GeneratedTool::with_artifact_kind(
            "bad_tool",
            "let result = input * 2;\nresult",
            "被沙箱拒绝的工具",
            EvolutionArtifactKind::RhaiScript,
        );
        let adapter =
            GeneratedToolAdapter::new(tool).with_sandbox_validator(Arc::new(RejectValidator));
        let result = adapter.call(serde_json::json!(21), &ToolContext::new(".")).await;
        assert!(result.is_err(), "测试：沙箱拒绝应导致执行失败");
        assert!(result.unwrap_err().message.contains("沙箱验证未通过"));
    }

    #[tokio::test]
    async fn test_rhai_script_artifact_passes_sandbox_then_executes() {
        // T4.4：沙箱通过 → 正常执行
        use axagent_harness::workflow_evolution::EvolutionArtifactValidator;
        use std::sync::Arc;

        struct PassValidator;
        impl EvolutionArtifactValidator for PassValidator {
            fn validate_code(&self, _code: &str) -> Vec<String> {
                Vec::new()
            }
        }

        let tool = GeneratedTool::with_artifact_kind(
            "ok_tool",
            "let result = input * 3;\nresult",
            "通过沙箱的工具",
            EvolutionArtifactKind::RhaiScript,
        );
        let adapter =
            GeneratedToolAdapter::new(tool).with_sandbox_validator(Arc::new(PassValidator));
        let result = adapter.call(serde_json::json!(7), &ToolContext::new(".")).await;
        assert!(result.is_ok(), "测试：沙箱通过应正常执行");
        assert_eq!(result.unwrap().content, "21");
    }

    #[tokio::test]
    async fn test_workflow_dag_artifact_without_executor_returns_definition() {
        // 编排型产物：未注入执行器 → 返回定义（不执行）
        let tool = GeneratedTool::with_artifact_kind(
            "wf_demo",
            r#"{ "nodes": ["a", "b"], "edges": [["a", "b"]] }"#,
            "编排型工具",
            EvolutionArtifactKind::WorkflowDag,
        );
        let adapter = GeneratedToolAdapter::new(tool);
        let result = adapter.call(serde_json::json!({}), &ToolContext::new(".")).await;
        assert!(result.is_ok(), "测试：未注入执行器应返回定义");
        let tr = result.unwrap();
        assert!(tr.content.contains("workflow_dag"));
        assert!(tr.content.contains("未注入执行器"));
    }

    #[tokio::test]
    async fn test_workflow_dag_artifact_with_executor_really_executes() {
        // 编排型产物：注入执行器 → 真正执行并返回结果
        use axagent_harness::workflow_evolution::WorkflowDagExecutor;
        use axagent_harness::workflow_evolution::WorkflowGenome;
        use std::sync::Arc;

        struct MockExecutor;
        #[async_trait::async_trait]
        impl WorkflowDagExecutor for MockExecutor {
            async fn execute(
                &self,
                genome: &WorkflowGenome,
                input: &serde_json::Value,
            ) -> Result<serde_json::Value, String> {
                Ok(serde_json::json!({
                    "template": genome.name,
                    "input": input,
                    "executed": true,
                }))
            }
        }

        let tool = GeneratedTool::with_artifact_kind(
            "wf_real",
            &serde_json::json!({
                "template_id": "wf-real",
                "name": "wf_real",
                "nodes": [{
                    "type": "end",
                    "id": "end",
                    "title": "end",
                    "position": {"x": 0, "y": 0},
                    "retry": {"enabled": false, "max_retries": 1, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                    "enabled": true,
                    "config": {"output_var": null}
                }],
                "edges": [],
                "variables": [],
                "fitness": 0.0,
                "generation": 0,
                "changed_node_ids": []
            })
            .to_string(),
            "编排型工具（真执行）",
            EvolutionArtifactKind::WorkflowDag,
        );
        let adapter =
            GeneratedToolAdapter::new(tool).with_workflow_executor(Arc::new(MockExecutor));
        let result = adapter.call(serde_json::json!({"q": 1}), &ToolContext::new(".")).await;
        assert!(result.is_ok(), "测试：注入执行器应执行成功");
        let tr = result.unwrap();
        assert!(!tr.is_error);
        assert!(tr.content.contains("wf_real"));
        assert!(tr.content.contains("\"executed\": true"));
    }

    // ── D4：真实执行反馈上报（仅真正执行的分支上报，兜底分支不产生假成功）──

    #[tokio::test]
    async fn test_workflow_dag_without_executor_does_not_report_feedback() {
        // D4 核心断言：WorkflowDag 未注入执行器 → 返回定义（未真正执行）→ 不上报反馈，
        // 避免「假成功」污染贝叶斯证据。
        let tool = GeneratedTool::with_artifact_kind(
            "wf_demo",
            r#"{ "nodes": ["a", "b"], "edges": [["a", "b"]] }"#,
            "编排型工具",
            EvolutionArtifactKind::WorkflowDag,
        );
        let sink = Arc::new(MockSink::default());
        let adapter = GeneratedToolAdapter::new(tool).with_feedback_sink(sink.clone());
        let result = adapter.call(serde_json::json!({}), &ToolContext::new(".")).await;
        assert!(result.is_ok(), "测试：未注入执行器应返回定义");
        assert!(sink.snapshot().is_empty(), "测试：未执行不应上报任何反馈");
    }

    #[tokio::test]
    async fn test_workflow_dag_with_executor_reports_success() {
        // D4：注入执行器 → 真正执行成功 → 上报一次成功反馈（携带会话 id）
        use axagent_harness::workflow_evolution::WorkflowDagExecutor;
        use axagent_harness::workflow_evolution::WorkflowGenome;

        struct MockExecutor;
        #[async_trait::async_trait]
        impl WorkflowDagExecutor for MockExecutor {
            async fn execute(
                &self,
                genome: &WorkflowGenome,
                input: &serde_json::Value,
            ) -> Result<serde_json::Value, String> {
                Ok(serde_json::json!({ "template": genome.name, "input": input }))
            }
        }

        let tool = GeneratedTool::with_artifact_kind(
            "wf_real",
            &serde_json::json!({
                "template_id": "wf-real",
                "name": "wf_real",
                "nodes": [{
                    "type": "end",
                    "id": "end",
                    "title": "end",
                    "position": {"x": 0, "y": 0},
                    "retry": {"enabled": false, "max_retries": 1, "backoff_type": "Exponential", "base_delay_ms": 1000, "max_delay_ms": 30000},
                    "enabled": true,
                    "config": {"output_var": null}
                }],
                "edges": [],
                "variables": [],
                "fitness": 0.0,
                "generation": 0,
                "changed_node_ids": []
            })
            .to_string(),
            "编排型工具（真执行）",
            EvolutionArtifactKind::WorkflowDag,
        );
        let sink = Arc::new(MockSink::default());
        let adapter = GeneratedToolAdapter::new(tool)
            .with_workflow_executor(Arc::new(MockExecutor))
            .with_feedback_sink(sink.clone());
        let ctx = ToolContext::new(".").with_conversation("conv-a");
        let result = adapter.call(serde_json::json!({"q": 1}), &ctx).await;
        assert!(result.is_ok(), "测试：注入执行器应执行成功");
        let records = sink.snapshot();
        assert_eq!(records.len(), 1, "测试：真执行应上报一次反馈");
        assert_eq!(records[0], (Some("conv-a".to_string()), "wf_real".to_string(), true));
    }

    #[tokio::test]
    async fn test_rhai_failure_reports_feedback() {
        // D4：Rhai 脚本真实执行失败 → 上报一次失败反馈（非假成功）
        let tool = GeneratedTool::with_artifact_kind(
            "broken",
            "this is not valid rhai @@@",
            "非法脚本工具",
            EvolutionArtifactKind::RhaiScript,
        );
        let sink = Arc::new(MockSink::default());
        let adapter = GeneratedToolAdapter::new(tool).with_feedback_sink(sink.clone());
        let ctx = ToolContext::new(".").with_conversation("conv-b");
        let result = adapter.call(serde_json::json!(1), &ctx).await;
        assert!(result.is_err(), "测试：非法脚本应编译失败");
        let records = sink.snapshot();
        assert_eq!(records.len(), 1, "测试：真实失败应上报一次反馈");
        assert_eq!(records[0], (Some("conv-b".to_string()), "broken".to_string(), false));
    }

    #[tokio::test]
    async fn test_rhai_success_reports_feedback() {
        // D4：Rhai 脚本真实执行成功 → 上报一次成功反馈
        let tool = GeneratedTool::with_artifact_kind(
            "calc_double",
            "let result = input * 2;\nresult",
            "翻倍计算工具",
            EvolutionArtifactKind::RhaiScript,
        );
        let sink = Arc::new(MockSink::default());
        let adapter = GeneratedToolAdapter::new(tool).with_feedback_sink(sink.clone());
        let ctx = ToolContext::new(".").with_conversation("conv-c");
        let result = adapter.call(serde_json::json!(21), &ctx).await;
        assert!(result.is_ok(), "测试：计算型产物应执行成功");
        let records = sink.snapshot();
        assert_eq!(records.len(), 1, "测试：真执行应上报一次反馈");
        assert_eq!(records[0], (Some("conv-c".to_string()), "calc_double".to_string(), true));
    }
}
