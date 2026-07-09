// SPDX-License-Identifier: AGPL-3.0-only

//! Deserializer for WorkflowNode — decoupled from the DTO definition.
//!
//! The custom `Deserialize` impl for WorkflowNode is a ~90-line dispatcher
//! over 31 variants. This module isolates that logic so that
//! `workflow_types.rs` remains a pure data definition.

use serde::Deserialize;

use crate::workflow_types::*;

pub fn deserialize<'de, D>(deserializer: D) -> Result<WorkflowNode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let type_str = value
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::missing_field("type"))?;

    macro_rules! try_from_value {
        ($variant:ident, $inner:ty) => {
            WorkflowNode::$variant(
                serde_json::from_value::<$inner>(value).map_err(serde::de::Error::custom)?,
            )
        };
    }

    match type_str {
        "trigger" => Ok(try_from_value!(Trigger, TriggerNode)),
        "agent" => Ok(try_from_value!(Agent, AgentNode)),
        "llm" => Ok(try_from_value!(Llm, LLMNode)),
        "condition" => Ok(try_from_value!(Condition, ConditionNode)),
        "parallel" => Ok(try_from_value!(Parallel, ParallelNode)),
        "loop" => Ok(try_from_value!(Loop, LoopNode)),
        "merge" => Ok(try_from_value!(Merge, MergeNode)),
        "delay" => Ok(try_from_value!(Delay, DelayNode)),
        "validation" => Ok(try_from_value!(Validation, ValidationNode)),
        "subWorkflow" => Ok(try_from_value!(SubWorkflow, SubWorkflowNode)),
        "documentParser" => Ok(try_from_value!(DocumentParser, DocumentParserNode)),
        "vectorRetrieve" => Ok(try_from_value!(VectorRetrieve, VectorRetrieveNode)),
        "httpRequest" => Ok(try_from_value!(HttpRequest, HttpRequestNode)),
        "switch" => Ok(try_from_value!(Switch, SwitchNode)),
        "databaseQuery" => Ok(try_from_value!(DatabaseQuery, DatabaseQueryNode)),
        "notification" => Ok(try_from_value!(Notification, NotificationNode)),
        "approval" => Ok(try_from_value!(Approval, ApprovalNode)),
        "fileOperation" => Ok(try_from_value!(FileOperation, FileOperationNode)),
        "dataTransformer" => Ok(try_from_value!(DataTransformer, DataTransformerNode)),
        "webhookSend" => Ok(try_from_value!(WebhookSend, WebhookSendNode)),
        "logging" => Ok(try_from_value!(Logging, LoggingNode)),
        "llmClassifier" => Ok(try_from_value!(LlmClassifier, LlmClassifierNode)),
        "aggregator" => Ok(try_from_value!(Aggregator, AggregatorNode)),
        "email" => Ok(try_from_value!(Email, EmailNode)),
        "debate" => Ok(try_from_value!(Debate, DebateNode)),
        "swarm" => Ok(try_from_value!(Swarm, SwarmNode)),
        "storage" => Ok(try_from_value!(Storage, StorageNode)),
        "workflowRef" => Ok(try_from_value!(WorkflowRef, WorkflowRefNode)),
        "end" => Ok(try_from_value!(End, EndNode)),
        "tool" => Ok(try_from_value!(Tool, ToolNode)),
        "code" => Ok(try_from_value!(Code, CodeNode)),
        other => Err(serde::de::Error::unknown_variant(
            other,
            &[
                "trigger",
                "agent",
                "llm",
                "condition",
                "parallel",
                "loop",
                "merge",
                "delay",
                "validation",
                "subWorkflow",
                "documentParser",
                "vectorRetrieve",
                "httpRequest",
                "switch",
                "databaseQuery",
                "notification",
                "approval",
                "fileOperation",
                "dataTransformer",
                "webhookSend",
                "logging",
                "llmClassifier",
                "aggregator",
                "email",
                "debate",
                "swarm",
                "storage",
                "workflowRef",
                "end",
                "tool",
                "code",
            ],
        )),
    }
}
