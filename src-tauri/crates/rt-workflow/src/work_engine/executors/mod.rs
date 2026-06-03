mod agent_executor;
pub mod aggregator_executor;
pub mod approval_executor;
mod code_executor;
mod condition_executor;
pub mod data_transformer_executor;
pub mod database_query_executor;
mod debate_executor;
mod delay_executor;
mod document_parser_executor;
pub mod email_executor;
mod end_executor;
mod fallback_executor;
pub mod file_operation_executor;
pub mod http_request_executor;
pub mod llm_classifier_executor;
mod llm_executor;
pub mod logging_executor;
mod loop_executor;
mod merge_executor;
pub mod notification_executor;
mod parallel_executor;
mod subworkflow_executor;
pub mod switch_executor;
mod tool_executor;
mod trigger_executor;
mod validation_executor;
mod vector_retrieve_executor;
pub mod webhook_send_executor;
pub use aggregator_executor::AggregatorExecutor;
pub use approval_executor::ApprovalExecutor;
pub use data_transformer_executor::DataTransformerExecutor;
pub use database_query_executor::DatabaseQueryExecutor;
pub use email_executor::EmailExecutor;
pub use file_operation_executor::FileOperationExecutor;
pub use http_request_executor::HttpRequestExecutor;
pub use llm_classifier_executor::LlmClassifierExecutor;
pub use logging_executor::LoggingExecutor;
pub use notification_executor::NotificationExecutor;
pub use switch_executor::SwitchExecutor;
pub use webhook_send_executor::WebhookSendExecutor;

pub use agent_executor::{
    AgentExecutor, PlanApprovalCallback, PlanApprovalRequest, PlanCallbacks, PlanPhaseSummary,
    PlanStepCallback, PlanStepEvent, RagCallback,
};
pub(crate) use agent_executor::{ProfileCache, ProviderCache};
pub use code_executor::CodeExecutor;
pub use condition_executor::ConditionExecutor;
pub use debate_executor::DebateExecutor;
pub use delay_executor::DelayExecutor;
pub use document_parser_executor::DocumentParserExecutor;
pub use end_executor::EndExecutor;
pub use fallback_executor::FallbackExecutor;
pub use llm_executor::LlmExecutor;
pub use loop_executor::LoopExecutor;
pub use merge_executor::MergeExecutor;
pub use parallel_executor::ParallelExecutor;
pub use subworkflow_executor::{SubWorkflowCallback, SubWorkflowExecutor};
pub use tool_executor::{ToolCallback, ToolExecutor};
pub use trigger_executor::TriggerExecutor;
pub use validation_executor::ValidationExecutor;
pub use vector_retrieve_executor::{VectorRetrieveCallback, VectorRetrieveExecutor};

/// 获取节点类型名称（从 node_executor_trait 导入，供执行器使用）。
pub use crate::work_engine::node_executor_trait::node_type_name;

/// 将 ProviderType 转为 ProviderRegistry 的 key 字符串
pub(crate) fn provider_type_to_registry_key(
    pt: &axagent_core::types::ProviderType,
) -> &'static str {
    match pt {
        axagent_core::types::ProviderType::OpenAI => "openai",
        axagent_core::types::ProviderType::OpenAIResponses => "openai_responses",
        axagent_core::types::ProviderType::Anthropic => "anthropic",
        axagent_core::types::ProviderType::Gemini => "gemini",
        axagent_core::types::ProviderType::OpenClaw => "openclaw",
        axagent_core::types::ProviderType::Hermes => "hermes",
        axagent_core::types::ProviderType::Ollama => "ollama",
    }
}
