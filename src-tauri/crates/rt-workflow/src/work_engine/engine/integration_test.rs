// SPDX-License-Identifier: AGPL-3.0-only

//! 三层架构集成测试
//!
//! 验证"刚性协议 + 柔性节点"架构的三层协作：
//! 1. 协议层：SchemaValidator 校验节点输入/输出
//! 2. 执行层：FsmExecutor 驱动业务状态机转移
//! 3. 观测层：TraceRecorder 记录执行轨迹

#[cfg(test)]
mod tests {
    use axagent_harness::business_state_machine::{BusinessStateMachine, FsmContext};
    use axagent_harness::execution_trace::{NodeErrorType, SchemaDiffReport};
    use axagent_harness::schema::{NodeContract, SchemaValidationResult};
    use axagent_harness::workflow_types::{
        Position, RetryConfig, ToolNode, ToolNodeConfig, WorkflowNode, WorkflowNodeBase,
    };

    use crate::work_engine::engine::fsm_executor::FsmExecutor;
    use crate::work_engine::engine::schema_validator::SchemaValidator;
    use crate::work_engine::engine::trace_recorder::TraceRecorder;

    fn create_test_tool_node(id: &str) -> WorkflowNode {
        WorkflowNode::Tool(ToolNode {
            base: WorkflowNodeBase {
                id: id.to_string(),
                title: format!("Test Tool {id}"),
                description: None,
                position: Position::default(),
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: ToolNodeConfig {
                tool_name: "test_tool".to_string(),
                input_mapping: std::collections::HashMap::new(),
                output_var: "result".to_string(),
            },
        })
    }

    /// 测试 1：Schema 校验器与 FSM 执行器协作
    ///
    /// 场景：审批流程中，节点输出不符合契约时应被拦截
    #[tokio::test]
    async fn test_schema_validation_with_fsm() {
        // 创建 FSM
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        // 创建 Schema 校验器并注册默认契约
        let mut validator = SchemaValidator::new();
        validator.register_default_contracts();

        // 创建测试节点
        let node = create_test_tool_node("test-node");

        // 有效状态转移
        let result = executor.transition_to("under_review", None).await;
        assert!(result.is_ok());

        // 验证当前状态
        let state = executor.current_state().await;
        assert_eq!(state.current_state_id, "under_review");

        // 测试 Schema 校验
        // 有效输入（tool 节点契约：接受任意 JSON 对象）
        let valid_input = serde_json::json!({
            "query": "分析销售数据",
            "max_results": 10
        });
        let validation = validator.validate_input(&node, &valid_input);
        assert!(validation.is_valid() || validation.is_skipped());

        // 测试无效输出（不符合预设契约的输出）
        let invalid_output = serde_json::json!(42); // tool 输出应为 object
        let validation = validator.validate_output(&node, &invalid_output);
        // 无效输出应该被检测到（tool 节点期望 object 类型）
        assert!(!matches!(validation, SchemaValidationResult::Valid));
    }

    /// 测试 2：轨迹记录器完整生命周期
    ///
    /// 场景：模拟一个完整的工作流执行，验证轨迹记录
    #[test]
    fn test_trace_recorder_lifecycle() {
        let recorder = TraceRecorder::new("exec-001", "wf-001");

        // 开始记录第一个节点
        recorder.start_node(
            "node-1",
            "agent",
            Some("分析需求".to_string()),
            Some(serde_json::json!({
                "query": "分析销售数据"
            })),
        );

        // 记录工具调用（添加到 node-1）
        recorder.add_tool_call(
            "node-1",
            "search",
            Some(serde_json::json!({
                "query": "销售数据"
            })),
        );

        // 完成第一个节点
        recorder.complete_node(
            "node-1",
            serde_json::json!({
                "analysis": "销售数据呈现上升趋势"
            }),
        );

        // 开始记录第二个节点
        recorder.start_node(
            "node-2",
            "tool",
            Some("执行操作".to_string()),
            Some(serde_json::json!({
                "action": "update_stock"
            })),
        );

        // 第二个节点失败
        recorder.fail_node("node-2", NodeErrorType::Timeout, "节点执行超时");

        // 完成整个执行
        recorder.complete_trace(Some(serde_json::json!({
            "status": "partial_success"
        })));

        // 验证轨迹
        let trace = recorder.get_trace();
        assert_eq!(trace.execution_id, "exec-001");
        assert_eq!(trace.workflow_id, "wf-001");
        assert_eq!(trace.node_traces.len(), 2);

        // 验证统计信息
        let stats = recorder.compute_statistics();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.success_nodes, 1);
        assert_eq!(stats.failed_nodes, 1);

        // 验证时间线
        let timeline = recorder.get_timeline();
        assert!(timeline.len() >= 2);
    }

    /// 测试 3：FSM 状态转移与轨迹记录协作
    ///
    /// 场景：审批流程的完整状态转移过程被轨迹记录
    #[tokio::test]
    async fn test_fsm_with_trace_recording() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");
        let recorder = TraceRecorder::new("exec-fsm", "wf-approval");

        // 记录初始状态
        recorder.start_node("state_init", "fsm_state", None, None);
        recorder.complete_node(
            "state_init",
            serde_json::json!({
                "state": "submitted"
            }),
        );

        // 执行状态转移
        executor.transition_to("under_review", Some(FsmContext::new())).await.unwrap();

        // 记录状态转移
        recorder.start_node("transition_1", "fsm_transition", None, None);
        recorder.complete_node(
            "transition_1",
            serde_json::json!({
                "from": "submitted",
                "to": "under_review"
            }),
        );

        // 转移到最终状态
        executor.transition_to("approved", Some(FsmContext::new())).await.unwrap();

        // 记录最终状态
        recorder.start_node("transition_2", "fsm_transition", None, None);
        recorder.complete_node(
            "transition_2",
            serde_json::json!({
                "from": "under_review",
                "to": "approved"
            }),
        );

        recorder.complete_trace(None);

        // 验证 FSM 状态
        let state = executor.current_state().await;
        assert_eq!(state.current_state_id, "approved");
        assert!(state.is_completed);

        // 验证轨迹（state_init + transition_1 + transition_2 = 3 个节点）
        let trace = recorder.get_trace();
        assert_eq!(trace.node_traces.len(), 3);

        let stats = recorder.compute_statistics();
        assert_eq!(stats.success_nodes, 3);
        assert_eq!(stats.failed_nodes, 0);
    }

    /// 测试 4：Schema 校验失败时的差异报告
    ///
    /// 场景：节点输出不符合 Schema，生成差异报告
    #[test]
    fn test_schema_diff_report() {
        let error = axagent_harness::schema::SchemaValidationError {
            path: "/result".to_string(),
            message: "类型不匹配".to_string(),
            expected_type: Some("string".to_string()),
            actual_value: Some(serde_json::json!(42)),
        };

        let report = SchemaDiffReport::from_validation_error(&error);
        assert_eq!(report.expected_path, "/result");
        assert!(report.suggestion.is_some());
    }

    /// 测试 5：工具调用记录
    ///
    /// 场景：工具调用从添加到节点的完整轨迹
    #[test]
    fn test_tool_call_recording() {
        let recorder = TraceRecorder::new("exec-tools", "wf-tools");

        // 创建节点
        recorder.start_node("node-tool", "tool", Some("调用搜索工具".to_string()), None);

        // 添加工具调用
        recorder.add_tool_call(
            "node-tool",
            "web_search",
            Some(serde_json::json!({
                "query": "测试查询"
            })),
        );

        // 完成节点（包含工具调用的结果）
        recorder.complete_node(
            "node-tool",
            serde_json::json!({
                "results": [
                    {"title": "文档1", "url": "https://example.com/1"},
                    {"title": "文档2", "url": "https://example.com/2"}
                ]
            }),
        );

        recorder.complete_trace(None);

        // 验证节点轨迹
        let node_trace = recorder.get_node_trace("node-tool");
        assert!(node_trace.is_some());

        let node = node_trace.unwrap();
        assert_eq!(node.tool_calls.len(), 1);
        assert_eq!(node.tool_calls[0].tool_name, "web_search");

        // 验证统计
        let stats = recorder.compute_statistics();
        assert_eq!(stats.total_tool_calls, 1);
    }

    /// 测试 6：FSM 刚性转移规则验证
    ///
    /// 场景：尝试非法转移应被拒绝
    #[tokio::test]
    async fn test_fsm_rigid_transition_rules() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        // 初始状态是 submitted
        let state = executor.current_state().await;
        assert_eq!(state.current_state_id, "submitted");

        // 尝试非法转移（submitted → approved 应该需要先经过 under_review）
        let result = executor.transition_to("approved", None).await;
        assert!(result.is_err());

        // 合法转移
        let result = executor.transition_to("under_review", None).await;
        assert!(result.is_ok());

        // 当前状态应为 under_review
        let state = executor.current_state().await;
        assert_eq!(state.current_state_id, "under_review");

        // 尝试回退转移（under_review → submitted 应该不允许）
        let result = executor.transition_to("submitted", None).await;
        assert!(result.is_err());
    }

    /// 测试 7：Schema 预设契约验证
    ///
    /// 场景：验证所有预设节点类型的契约都能正常工作
    #[test]
    fn test_preset_contracts_validation() {
        // 验证 agent 节点契约（Agent 接受任意输入，可能没有强制 Schema）
        let agent_contract = NodeContract::agent_default();
        assert!(!agent_contract.description.is_none());

        // 验证 tool 节点契约
        let tool_contract = NodeContract::tool_default();
        assert!(tool_contract.has_input());
        assert!(tool_contract.has_output());

        // 验证 llm 节点契约
        let llm_contract = NodeContract::llm_default();
        assert!(llm_contract.has_input());
        assert!(llm_contract.has_output());

        // 验证预设契约能被注册
        let mut validator = SchemaValidator::new();
        validator.register_default_contracts();

        assert!(validator.get_contract("agent").is_some());
        assert!(validator.get_contract("tool").is_some());
        assert!(validator.get_contract("llm").is_some());
        assert!(validator.get_contract("condition").is_some());
    }

    /// 测试 8：错误汇总与根因定位
    ///
    /// 场景：多节点执行失败后，通过轨迹快速定位根因
    #[test]
    fn test_error_root_cause_analysis() {
        let recorder = TraceRecorder::new("exec-rc", "wf-rc");

        // 第一个节点成功
        recorder.start_node("node-1", "agent", None, None);
        recorder.complete_node("node-1", serde_json::json!({"result": "ok"}));

        // 第二个节点 Schema 校验失败
        recorder.start_node("node-2", "tool", None, None);

        // 记录输出校验失败
        let schema_error = axagent_harness::schema::SchemaValidationError {
            path: "/result".to_string(),
            message: "类型不匹配".to_string(),
            expected_type: Some("string".to_string()),
            actual_value: Some(serde_json::json!(42)),
        };
        let invalid_result = SchemaValidationResult::invalid(vec![schema_error]);
        recorder.record_output_validation("node-2", invalid_result);

        recorder.fail_node("node-2", NodeErrorType::SchemaValidation, "输出不符合 Schema");

        // 第三个节点超时
        recorder.start_node("node-3", "agent", None, None);
        recorder.fail_node("node-3", NodeErrorType::Timeout, "执行超时");

        recorder.complete_trace(None);

        // 获取失败节点
        let failed_nodes = recorder.get_failed_nodes();
        assert_eq!(failed_nodes.len(), 2);

        // 验证失败节点的 ID
        let failed_ids: Vec<&str> = failed_nodes.iter().map(|n| n.node_id.as_str()).collect();
        assert!(failed_ids.contains(&"node-2"));
        assert!(failed_ids.contains(&"node-3"));

        // 获取 Schema 差异报告
        let diff_reports = recorder.get_schema_diff_reports();
        assert_eq!(diff_reports.len(), 1);
        assert_eq!(diff_reports[0].expected_path, "/result");

        // 验证统计
        let stats = recorder.compute_statistics();
        assert_eq!(stats.schema_errors, 1);
        assert_eq!(stats.failed_nodes, 2);
    }
}
