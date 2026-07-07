// SPDX-License-Identifier: AGPL-3.0-only

//! 触发器执行器 —— 解析触发配置并激活 Schedule/Webhook/Event 触发器。
//!
//! - Manual: 直通返回配置（与旧版行为一致）
//! - Schedule: 调用 TriggerManager 注册 cron 定时任务
//! - Webhook:  调用 TriggerManager 注册 HTTP 路由
//! - Event:    调用 TriggerManager 注册事件订阅

use async_trait::async_trait;
use axagent_harness::workflow_types::{
    EventTriggerConfig, ScheduleTriggerConfig, TriggerType, WebhookTriggerConfig, WorkflowNode,
};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct TriggerExecutor;

impl TriggerExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TriggerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for TriggerExecutor {
    fn node_type(&self) -> &'static str {
        "trigger"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Trigger(trigger_node) = node else {
            return Err(NodeError::type_mismatch(
                "trigger".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let trigger_type = trigger_node.config.trigger_type.clone();

        // 获取 TriggerManager（若未注入则降级为旧版直通行为）
        let tm = context.callbacks.as_ref().and_then(|cb| cb.trigger_manager.as_ref());

        match trigger_type {
            TriggerType::Manual => {
                // Manual 类型保持直通，不做任何调度注册
                Ok(NodeOutput {
                    output: build_output("manual", &trigger_node.config.config),
                    output_var: None,
                })
            },
            TriggerType::Schedule => {
                if let Ok(cfg) = serde_json::from_value::<ScheduleTriggerConfig>(
                    trigger_node.config.config.clone(),
                ) && let Some(tm) = tm
                {
                    tm.register_schedule(
                        trigger_node.base.id.as_str(),
                        &cfg.cron,
                        &cfg.timezone,
                        cfg.input_params.clone(),
                    )
                    .await
                    .map_err(|e| {
                        NodeError::exec_failed(
                            crate::work_engine::node_executor_trait::error_code::VALIDATION_FAILED,
                            format!("定时触发器注册失败: {e}"),
                        )
                    })?;
                }
                Ok(NodeOutput {
                    output: build_output("schedule", &trigger_node.config.config),
                    output_var: None,
                })
            },
            TriggerType::Webhook => {
                if let Ok(cfg) = serde_json::from_value::<WebhookTriggerConfig>(
                    trigger_node.config.config.clone(),
                ) && let Some(tm) = tm
                {
                    let mode = cfg.response_mode.as_deref().unwrap_or("async");
                    tm.register_webhook(
                        trigger_node.base.id.as_str(),
                        &cfg.path,
                        &cfg.method,
                        mode,
                    )
                    .await;
                }
                Ok(NodeOutput {
                    output: build_output("webhook", &trigger_node.config.config),
                    output_var: None,
                })
            },
            TriggerType::Event => {
                if let Ok(cfg) =
                    serde_json::from_value::<EventTriggerConfig>(trigger_node.config.config.clone())
                    && let Some(tm) = tm
                {
                    tm.register_event(trigger_node.base.id.as_str(), &cfg.event_type).await;
                }
                Ok(NodeOutput {
                    output: build_output("event", &trigger_node.config.config),
                    output_var: None,
                })
            },
        }
    }
}

fn build_output(trigger_type: &str, config: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "status": "triggered",
        "trigger_type": trigger_type,
        "config": config,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    })
}
