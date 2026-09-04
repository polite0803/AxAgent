// SPDX-License-Identifier: AGPL-3.0-only

//! 测试替身支持
//!
//! 提供 `axagent-harness` 自身定义的 mock / empty 实现，使测试代码
//! 无需在 dev-dependencies 里引入具体的 provider / tool 实现 crate。
//!
//! 目的：测试也走 trait 抽象，与生产代码风格一致。
//!
//! 注意：本模块始终编译，但内部仅导出"返回 None / 空实现"的轻量辅助函数，
//! 不会引入运行时依赖，也不会进入生产热路径（仅测试代码使用）。

#![allow(clippy::disallowed_types)]

use std::sync::Arc;

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use serde_json::Map;
use serde_json::Value;

use crate::AccessDecision;
use crate::Agent;
use crate::AgentCapability;
use crate::AgentExecuteRequest;
use crate::AgentPlan;
use crate::AgentResult;
use crate::BenchmarkReport;
use crate::BenchmarkRunner;
use crate::BenchmarkTask;
use crate::BrowserController;
use crate::BrowserNavigateResult;
use crate::BrowserScreenshotResult;
use crate::ChunkProvider;
use crate::CircuitBreaker;
use crate::CircuitBreakerConfig;
use crate::CircuitBreakerSnapshot;
use crate::CircuitState;
use crate::CodeSample;
use crate::CodeStyleTemplate;
use crate::ConsolidationDataProvider;
use crate::ConsolidationSuggestion;
use crate::ContentFilter;
use crate::ContentType;
use crate::DevExperienceProvider;
use crate::DiscoveredMcpTool;
use crate::DistilledKnowledge;
use crate::DocumentChunk;
use crate::DocumentIndexer;
use crate::DocumentStyleProfile;
use crate::DreamConsolidationConfig;
use crate::DreamConsolidationResult;
use crate::DreamConsolidator;
use crate::EmbeddingProvider;
use crate::EntityExtractor;
use crate::EntityGraphProvider;
use crate::EnvironmentInfo;
use crate::ExperienceRecord;
use crate::ExtractedCodePatterns;
use crate::ExtractedElement;
use crate::ExtractedEntity;
use crate::ExtractedRelation;
use crate::FilterAction;
use crate::GatewayService;
use crate::GatewayStatus;
use crate::GraphEnhancedSearchInput;
use crate::GraphEnhancedSearchResult;
use crate::IndexConfig;
use crate::IndexJobStatus;
use crate::IntegrityResult;
use crate::KnowledgeType;
use crate::LlmExecutionService;
use crate::LogLevel;
use crate::McpClientService;
use crate::McpPrompt;
use crate::McpPromptResult;
use crate::McpResource;
use crate::McpResourceContent;
use crate::McpServerConfig;
use crate::McpServerStore;
use crate::McpToolCallResult;
use crate::MemoryActionResultDto;
use crate::MemoryAddRequest;
use crate::MemoryFeedbackRequest;
use crate::MemoryGroupedDto;
use crate::MemoryScanner;
use crate::MemorySearchItem;
use crate::MemorySearchRequest;
use crate::MemoryStore;
use crate::MemoryTreeItem;
use crate::MemoryUpdateRequest;
use crate::MessageSample;
use crate::ModelKnowledgeProvider;
use crate::NpmRegistryService;
use crate::ObservabilityProvider;
use crate::ObservabilitySpanType;
use crate::OutputSanitizer;
use crate::PlannerAdapter;
use crate::PlatformConnectionInfo;
use crate::PlatformManager;
use crate::ProfileUpdate;
use crate::PromptGuard;
use crate::PromptLang;
use crate::PromptProvider;
use crate::ProviderRequestContext;
use crate::RAGProvider;
use crate::RAGQuery;
use crate::RLEngine;
use crate::RLTrainer;
use crate::RateLimitResult;
use crate::RateLimitStatus;
use crate::RateLimiter;
use crate::RerankProvider;
use crate::RetrievalQuality;
use crate::RhaiEngineAdapter;
use crate::RhaiToolFn;
use crate::SanitizeContext;
use crate::ScanResult;
use crate::ScannerConfig;
use crate::SelfRagProvider;
use crate::SessionTracer;
use crate::SsrFConfig;
use crate::SsrFGuard;
use crate::StyleApplier;
use crate::StyleExtractor;
use crate::StyleVector;
use crate::StyleVectorizer;
use crate::TaskComplexity;
use crate::TaskResult;
use crate::ToolAccessControl;
use crate::ToolAccessRequest;
use crate::ToolMetricsCollector;
use crate::ToolMetricsSnapshot;
use crate::TrainingEpisode;
use crate::TrainingReport;
use crate::TrajectoryService;
use crate::UrlSafety;
use crate::UserProfile;
use crate::UserProfileService;
use crate::VectorQueryResult;
use crate::VectorStoreProvider;
use crate::WebhookSubscriptionInfo;
use crate::WebhookSubscriptionService;
use crate::core_error::{AxAgentError, Result};
use crate::llm_execution::{LlmCallConfig, LlmCallResult};
use crate::repo_dtos::{WorkflowExecutionData, WorkflowTemplateData};
use crate::repositories::{
    LoopCheckpointRepository, WorkflowExecutionRepository, WorkflowTemplateRepository,
};
use crate::tool_metrics::ToolCallRecord;
use crate::types::CreateKnowledgeEntityInput;
use crate::types::KnowledgeEntity;
use crate::types::KnowledgeRelation;
use crate::types::RagContextResult;
use crate::types::RagRetrievedItem;
use crate::workflow_types::LoopCheckpoint;

use crate::platform_adapter::{
    CryptoService, GatewayKeyRepository, GatewayRequestLogRepository, PlatformAdapter,
    ProviderRepository, SettingsRepository,
};
use crate::provider::ProviderAdapter;
use crate::types::{AppSettings, GatewayKey, ProviderConfig, ProviderKey};

/// 一个返回 `None` 的空 ProviderRegistry，测试 gateway / runtime 时使用。
pub struct EmptyProviderRegistry;

impl crate::registry::ProviderRegistry for EmptyProviderRegistry {
    fn get(&self, _provider_type: &str) -> Option<Arc<dyn ProviderAdapter>> {
        None
    }
}

/// 工厂：构造一个 `Arc<dyn ProviderRegistry>` 测试替身
pub fn empty_provider_registry() -> Arc<dyn crate::registry::ProviderRegistry> {
    Arc::new(EmptyProviderRegistry)
}

// ── PlatformAdapter 测试替身 ──

struct EmptyProviderRepository;
#[async_trait::async_trait]
impl ProviderRepository for EmptyProviderRepository {
    async fn list_providers(&self) -> Result<Vec<ProviderConfig>> {
        Ok(vec![])
    }
    async fn get_provider(&self, _id: &str) -> Result<ProviderConfig> {
        Err(AxAgentError::NotFound("test stub".into()))
    }
    async fn get_active_key(&self, _provider_id: &str) -> Result<ProviderKey> {
        Err(AxAgentError::NotFound("test stub".into()))
    }
    async fn resolve_model_for_node(
        &self,
        _node_model: Option<&str>,
        _session_model: Option<&str>,
        _session_provider_id: Option<&str>,
        _profile_suggested_provider: Option<&str>,
    ) -> Result<(ProviderConfig, ProviderKey, String)> {
        Err(AxAgentError::NotFound("test stub".into()))
    }
}

struct EmptySettingsRepository;
#[async_trait::async_trait]
impl SettingsRepository for EmptySettingsRepository {
    async fn get_settings(&self) -> Result<AppSettings> {
        Err(AxAgentError::NotFound("test stub".into()))
    }
}

struct EmptyGatewayKeyRepository;
#[async_trait::async_trait]
impl GatewayKeyRepository for EmptyGatewayKeyRepository {
    async fn list_gateway_keys(&self) -> Result<Vec<GatewayKey>> {
        Ok(vec![])
    }
    async fn verify_key(&self, _token: &str) -> Result<Option<GatewayKey>> {
        Ok(None)
    }
    async fn get_by_id(&self, _key_id: &str) -> Result<Option<GatewayKey>> {
        Ok(None)
    }
    async fn update_last_used(&self, _key_id: &str) -> Result<()> {
        Ok(())
    }
    async fn record_usage(
        &self,
        _key_id: &str,
        _provider_id: &str,
        _model_id: Option<&str>,
        _request_tokens: u64,
        _response_tokens: u64,
        _cached_input_tokens: u64,
        _cost_usd: f64,
    ) -> Result<()> {
        Ok(())
    }
    async fn get_metrics(&self) -> Result<crate::types::GatewayMetrics> {
        Ok(crate::types::GatewayMetrics::default())
    }
}

struct EmptyGatewayRequestLogRepository;
#[async_trait::async_trait]
impl GatewayRequestLogRepository for EmptyGatewayRequestLogRepository {
    async fn record_request_log(
        &self,
        _key_id: &str,
        _key_name: &str,
        _method: &str,
        _path: &str,
        _model_id: Option<&str>,
        _provider_id: Option<&str>,
        _status_code: i32,
        _duration_ms: i64,
        _request_tokens: i64,
        _response_tokens: i64,
        _error_message: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }
}

struct EmptyCryptoService;
impl CryptoService for EmptyCryptoService {
    fn decrypt_key(&self, _encrypted: &str) -> Result<String> {
        Err(AxAgentError::Crypto("test stub".into()))
    }
    fn encrypt_key(&self, _plaintext: &str) -> Result<String> {
        Err(AxAgentError::Crypto("test stub".into()))
    }
    fn decrypt_key_with(&self, _encrypted: &str, _master_key: &[u8; 32]) -> Result<String> {
        Err(AxAgentError::Crypto("test stub".into()))
    }
    fn encrypt_key_with(&self, _plaintext: &str, _master_key: &[u8; 32]) -> Result<String> {
        Err(AxAgentError::Crypto("test stub".into()))
    }
    fn hmac_sha256(&self, _key: &[u8], _msg: &str) -> String {
        String::new()
    }
    fn sha256_hash(&self, _input: &str) -> String {
        String::new()
    }
    fn key_prefix(&self, _key: &str) -> String {
        String::new()
    }
    fn generate_gateway_key(&self) -> String {
        String::new()
    }
    fn generate_master_key(&self) -> [u8; 32] {
        [0u8; 32]
    }
    fn encrypt_backup_key(&self, _key_data: &[u8]) -> Result<Vec<u8>> {
        Err(AxAgentError::Crypto("test stub".into()))
    }
    fn decrypt_backup_key(&self, _enc_data: &[u8]) -> Result<Vec<u8>> {
        Err(AxAgentError::Crypto("test stub".into()))
    }
}

struct EmptyPlatformAdapter;
impl PlatformAdapter for EmptyPlatformAdapter {
    fn providers(&self) -> &dyn ProviderRepository {
        &EmptyProviderRepository
    }
    fn settings(&self) -> &dyn SettingsRepository {
        &EmptySettingsRepository
    }
    fn gateway_keys(&self) -> &dyn GatewayKeyRepository {
        &EmptyGatewayKeyRepository
    }
    fn request_log(&self) -> &dyn GatewayRequestLogRepository {
        &EmptyGatewayRequestLogRepository
    }
    fn crypto(&self) -> &dyn CryptoService {
        &EmptyCryptoService
    }
}

/// 工厂：构造一个 `Arc<dyn PlatformAdapter>` 测试替身（所有方法返回空 / 错误）
pub fn empty_platform_adapter() -> Arc<dyn PlatformAdapter> {
    Arc::new(EmptyPlatformAdapter)
}

// ── MarketplaceService 测试替身 ──

use crate::marketplace::{
    CreateReviewRequest, MarketplaceService, MarketplaceStats, ReviewResponse, UpdateReviewRequest,
};
use sea_orm::DatabaseConnection;

struct EmptyMarketplaceService;

#[async_trait::async_trait]
impl MarketplaceService for EmptyMarketplaceService {
    async fn create_review(
        &self,
        _db: &DatabaseConnection,
        _req: CreateReviewRequest,
    ) -> std::result::Result<ReviewResponse, String> {
        Err("test stub".into())
    }
    async fn get_reviews(
        &self,
        _db: &DatabaseConnection,
        _marketplace_id: &str,
    ) -> std::result::Result<Vec<ReviewResponse>, String> {
        Err("test stub".into())
    }
    async fn get_user_review(
        &self,
        _db: &DatabaseConnection,
        _marketplace_id: &str,
        _user_id: &str,
    ) -> std::result::Result<Option<ReviewResponse>, String> {
        Err("test stub".into())
    }
    async fn update_review(
        &self,
        _db: &DatabaseConnection,
        _review_id: &str,
        _req: UpdateReviewRequest,
    ) -> std::result::Result<ReviewResponse, String> {
        Err("test stub".into())
    }
    async fn delete_review(
        &self,
        _db: &DatabaseConnection,
        _review_id: &str,
    ) -> std::result::Result<(), String> {
        Err("test stub".into())
    }
    async fn get_stats(
        &self,
        _db: &DatabaseConnection,
        _marketplace_id: &str,
    ) -> std::result::Result<MarketplaceStats, String> {
        Err("test stub".into())
    }
    async fn get_marketplace_id_for_review(
        &self,
        _db: &DatabaseConnection,
        _review_id: &str,
    ) -> std::result::Result<String, String> {
        Err("test stub".into())
    }
}

/// 工厂：构造一个 `Arc<dyn MarketplaceService>` 测试替身（所有方法返回错误）
pub fn empty_marketplace_service() -> Arc<dyn MarketplaceService> {
    Arc::new(EmptyMarketplaceService)
}

// ── CredentialService 测试替身 ──

use crate::credential_service::{CredentialService, SmtpServiceConfig};

/// 凭证服务空实现 — 所有方法返回错误。
#[derive(Debug)]
pub struct EmptyCredentialService;

#[async_trait::async_trait]
impl CredentialService for EmptyCredentialService {
    async fn get_database_connection_string(
        &self,
        _credential_id: &str,
    ) -> std::result::Result<String, String> {
        Err("EmptyCredentialService: not implemented".into())
    }

    async fn get_smtp_config(
        &self,
        _credential_id: &str,
    ) -> std::result::Result<SmtpServiceConfig, String> {
        Err("EmptyCredentialService: not implemented".into())
    }

    async fn get_auth_headers(
        &self,
        _credential_id: &str,
    ) -> std::result::Result<Vec<(String, String)>, String> {
        Err("EmptyCredentialService: not implemented".into())
    }
}

/// 工厂：构造一个 `Arc<dyn CredentialService>` 测试替身（所有方法返回错误）
pub fn empty_credential_service() -> Arc<dyn CredentialService> {
    Arc::new(EmptyCredentialService)
}

// ═══════════════════════════════════════════════════════════════
// Noop test doubles — migrated from individual modules
// ═══════════════════════════════════════════════════════════════

// Each Noop struct below was originally defined alongside its trait.
// They are consolidated here so that test files can import them from
// a single location: use axagent_harness::test_support::NoopXxx;

// ── from agent.rs ──
#[derive(Debug)]
pub struct NoopAgent;

#[async_trait]
impl Agent for NoopAgent {
    fn name(&self) -> &str {
        "noop"
    }
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![]
    }
    async fn execute(&self, _req: AgentExecuteRequest) -> std::result::Result<AgentResult, String> {
        Err("NoopAgent cannot execute".to_string())
    }
    async fn plan(&self, _goal: &str) -> std::result::Result<AgentPlan, String> {
        Err("NoopAgent cannot plan".to_string())
    }
}

// ── from benchmark.rs ──
#[derive(Default)]
pub struct NoopBenchmarkRunner;
#[async_trait]
impl BenchmarkRunner for NoopBenchmarkRunner {
    async fn run_task(&self, _: &BenchmarkTask) -> std::result::Result<TaskResult, String> {
        Err("not configured".into())
    }
    async fn run_suite(&self, _: &[BenchmarkTask]) -> std::result::Result<BenchmarkReport, String> {
        Err("not configured".into())
    }
}

// ── from browser.rs ──
#[derive(Debug, Default)]
pub struct NoopBrowserController;

#[async_trait]
impl BrowserController for NoopBrowserController {
    async fn navigate(&self, _url: &str) -> std::result::Result<BrowserNavigateResult, String> {
        Err("browser not configured".to_string())
    }
    async fn screenshot(&self) -> std::result::Result<BrowserScreenshotResult, String> {
        Err("browser not configured".to_string())
    }
    async fn extract_elements(
        &self,
        _selector: &str,
    ) -> std::result::Result<Vec<ExtractedElement>, String> {
        Ok(Vec::new())
    }
    async fn click(&self, _selector: &str) -> std::result::Result<(), String> {
        Err("browser not configured".to_string())
    }
    async fn type_text(&self, _selector: &str, _text: &str) -> std::result::Result<(), String> {
        Err("browser not configured".to_string())
    }
    async fn close(&self) -> std::result::Result<(), String> {
        Ok(())
    }
}

// ── from circuit_breaker.rs ──
#[derive(Default)]
pub struct NoopCircuitBreaker {
    config: CircuitBreakerConfig,
}
impl CircuitBreaker for NoopCircuitBreaker {
    fn is_allowed(&self) -> bool {
        true
    }
    fn record_success(&self) {}
    fn record_failure(&self) {}
    fn reset(&self) {}
    fn snapshot(&self) -> CircuitBreakerSnapshot {
        CircuitBreakerSnapshot {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure_secs_ago: None,
            total_success: 0,
            total_failure: 0,
        }
    }
    fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }
}

// ── from content_filter.rs ──
#[derive(Default)]
pub struct NoopContentFilter;
#[async_trait]
impl ContentFilter for NoopContentFilter {
    async fn filter(&self, _: &str, _: ContentType) -> std::result::Result<FilterAction, String> {
        Ok(FilterAction::Allow)
    }
    async fn is_safe(&self, _: &str, _: ContentType) -> std::result::Result<bool, String> {
        Ok(true)
    }
}

// ── from dev_experience.rs ──
#[derive(Default)]
pub struct NoopDevExperienceProvider;
#[async_trait]
impl DevExperienceProvider for NoopDevExperienceProvider {
    async fn get_env_info(&self) -> std::result::Result<EnvironmentInfo, String> {
        Ok(EnvironmentInfo {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            hostname: "unknown".into(),
            rust_version: None,
            node_version: None,
            ide: None,
            workspace_path: None,
        })
    }
    async fn set_log_level(&self, _: LogLevel) {}
    async fn get_log_level(&self) -> std::result::Result<LogLevel, String> {
        Ok(LogLevel::Info)
    }
    fn version(&self) -> &'static str {
        "0.0.0"
    }
    async fn check_update(&self) -> std::result::Result<Option<String>, String> {
        Ok(None)
    }
}

// ── from dream.rs ──
#[derive(Default)]
pub struct NoopConsolidationDataProvider;
#[async_trait]
impl ConsolidationDataProvider for NoopConsolidationDataProvider {
    async fn fetch_recent_experiences(
        &self,
        _limit: usize,
    ) -> std::result::Result<Vec<ExperienceRecord>, String> {
        Ok(Vec::new())
    }
    async fn fetch_experience_by_topic(
        &self,
        _topic: &str,
    ) -> std::result::Result<Vec<ExperienceRecord>, String> {
        Ok(Vec::new())
    }
    async fn store_distilled_knowledge(
        &self,
        _knowledge: &DistilledKnowledge,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn store_suggestion(
        &self,
        _suggestion: &ConsolidationSuggestion,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn fetch_existing_knowledge(
        &self,
        _knowledge_type: &KnowledgeType,
    ) -> std::result::Result<Vec<DistilledKnowledge>, String> {
        Ok(Vec::new())
    }
}

// ── from dream.rs ──
#[derive(Default)]
pub struct NoopDreamConsolidator;
#[async_trait]
impl DreamConsolidator for NoopDreamConsolidator {
    async fn consolidate(&self) -> std::result::Result<DreamConsolidationResult, String> {
        Err("dream consolidator not configured".to_string())
    }
    async fn should_consolidate(&self) -> std::result::Result<bool, String> {
        Ok(false)
    }
    async fn config(&self) -> DreamConsolidationConfig {
        DreamConsolidationConfig::default()
    }
}

// ── from gateway_service.rs ──
#[derive(Default)]
pub struct NoopGatewayService;
#[async_trait]
impl GatewayService for NoopGatewayService {
    async fn start(&self) -> std::result::Result<(), String> {
        Err("not configured".into())
    }
    async fn stop(&self) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn status(&self) -> std::result::Result<GatewayStatus, String> {
        Ok(GatewayStatus::Stopped)
    }
}

// ── from indexer.rs ──
#[derive(Default)]
pub struct NoopChunkProvider;
#[async_trait]
impl ChunkProvider for NoopChunkProvider {
    async fn chunk(
        &self,
        _: &str,
        _: &IndexConfig,
    ) -> std::result::Result<Vec<DocumentChunk>, String> {
        Ok(Vec::new())
    }
    async fn chunk_batch(
        &self,
        _: &[(String, String)],
        _: &IndexConfig,
    ) -> std::result::Result<Vec<DocumentChunk>, String> {
        Ok(Vec::new())
    }
}

// ── from indexer.rs ──
#[derive(Default)]
pub struct NoopDocumentIndexer;
#[async_trait]
impl DocumentIndexer for NoopDocumentIndexer {
    async fn index_document(
        &self,
        _: &str,
        _: &str,
        _: &IndexConfig,
    ) -> std::result::Result<IndexJobStatus, String> {
        Err("not configured".into())
    }
    async fn index_batch(
        &self,
        _: &[(String, String)],
        _: &IndexConfig,
    ) -> std::result::Result<IndexJobStatus, String> {
        Err("not configured".into())
    }
    async fn delete_index(&self, _: &str) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn get_stats(&self, _: &str) -> std::result::Result<serde_json::Value, String> {
        Ok(serde_json::json!({"status":"not configured"}))
    }
}

// ── from knowledge_graph.rs ──
#[derive(Default)]
pub struct NoopEntityGraphProvider;
#[async_trait]
impl EntityGraphProvider for NoopEntityGraphProvider {
    async fn get_entities(&self, _: &str) -> std::result::Result<Vec<KnowledgeEntity>, String> {
        Ok(Vec::new())
    }
    async fn search_entities(
        &self,
        _: &str,
        _: &str,
    ) -> std::result::Result<Vec<KnowledgeEntity>, String> {
        Ok(Vec::new())
    }
    async fn create_entity(
        &self,
        _: &str,
        _: CreateKnowledgeEntityInput,
    ) -> std::result::Result<KnowledgeEntity, String> {
        Err("not configured".into())
    }
    async fn delete_entity(&self, _: &str) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn get_relations(&self, _: &str) -> std::result::Result<Vec<KnowledgeRelation>, String> {
        Ok(Vec::new())
    }
    async fn create_relation(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> std::result::Result<KnowledgeRelation, String> {
        Err("not configured".into())
    }
    async fn delete_relation(&self, _: &str) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn graph_enhanced_search(
        &self,
        _: GraphEnhancedSearchInput,
    ) -> std::result::Result<GraphEnhancedSearchResult, String> {
        Err("not configured".into())
    }
}

// ── from knowledge_graph.rs ──
#[derive(Default)]
pub struct NoopEntityExtractor;
#[async_trait]
impl EntityExtractor for NoopEntityExtractor {
    async fn extract_entities(&self, _: &str) -> std::result::Result<Vec<ExtractedEntity>, String> {
        Ok(Vec::new())
    }
    async fn extract_relations(
        &self,
        _: &str,
        _: &[ExtractedEntity],
    ) -> std::result::Result<Vec<ExtractedRelation>, String> {
        Ok(Vec::new())
    }
}

// ── from llm_execution.rs ──
pub struct NoopLlmExecutionService;

#[async_trait]
impl LlmExecutionService for NoopLlmExecutionService {
    async fn execute(
        &self,
        _adapter: &(dyn ProviderAdapter + '_),
        _ctx: &ProviderRequestContext,
        _messages: serde_json::Value,
        _config: &LlmCallConfig,
    ) -> std::result::Result<LlmCallResult, String> {
        Ok(LlmCallResult { content: String::new() })
    }
}

// ── from mcp_service.rs ──
#[derive(Debug, Default)]
pub struct NoopMcpServerStore;

#[async_trait]
impl McpServerStore for NoopMcpServerStore {
    async fn list_enabled(&self) -> std::result::Result<Vec<McpServerConfig>, String> {
        Ok(Vec::new())
    }
    async fn get_by_id(&self, _id: &str) -> std::result::Result<Option<McpServerConfig>, String> {
        Ok(None)
    }
}

// ── from mcp_service.rs ──
#[derive(Debug, Default)]
pub struct NoopMcpClientService;

#[async_trait]
impl McpClientService for NoopMcpClientService {
    async fn discover_tools(
        &self,
        _server: &McpServerConfig,
    ) -> std::result::Result<Vec<DiscoveredMcpTool>, String> {
        Ok(Vec::new())
    }
    async fn call_tool(
        &self,
        _server: &McpServerConfig,
        _tool_name: &str,
        _args: serde_json::Value,
    ) -> std::result::Result<McpToolCallResult, String> {
        Ok(McpToolCallResult { success: false, content: serde_json::Value::Null })
    }

    async fn list_resources(
        &self,
        _server: &McpServerConfig,
    ) -> std::result::Result<Vec<McpResource>, String> {
        Ok(Vec::new())
    }

    async fn read_resource(
        &self,
        _server: &McpServerConfig,
        _uri: &str,
    ) -> std::result::Result<Vec<McpResourceContent>, String> {
        Ok(Vec::new())
    }

    async fn list_prompts(
        &self,
        _server: &McpServerConfig,
    ) -> std::result::Result<Vec<McpPrompt>, String> {
        Ok(Vec::new())
    }

    async fn get_prompt(
        &self,
        _server: &McpServerConfig,
        _name: &str,
        _args: serde_json::Value,
    ) -> std::result::Result<McpPromptResult, String> {
        Ok(McpPromptResult { description: None, messages: serde_json::Value::Null })
    }
}

// ── from memory.rs ──
pub struct NoopMemoryStore;

#[async_trait]
impl MemoryStore for NoopMemoryStore {
    async fn add_memory(
        &self,
        _req: MemoryAddRequest,
    ) -> std::result::Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
    async fn search_memories(
        &self,
        _req: MemorySearchRequest,
    ) -> std::result::Result<Vec<MemorySearchItem>, String> {
        Ok(vec![])
    }
    async fn get_memory_tree(&self) -> std::result::Result<Vec<MemoryTreeItem>, String> {
        Ok(vec![])
    }
    async fn get_working_memory(&self) -> std::result::Result<Option<String>, String> {
        Ok(None)
    }
    async fn get_grouped_memories(&self) -> std::result::Result<Vec<MemoryGroupedDto>, String> {
        Ok(vec![])
    }
    async fn submit_feedback(
        &self,
        _req: MemoryFeedbackRequest,
    ) -> std::result::Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
    async fn delete_memory(&self, _id: &str) -> std::result::Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
    async fn update_memory(
        &self,
        _req: MemoryUpdateRequest,
    ) -> std::result::Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
}

// ── from model_knowledge.rs ──
pub struct NoopModelKnowledgeProvider;

impl ModelKnowledgeProvider for NoopModelKnowledgeProvider {
    fn get_model_context_window(&self, _model_id: &str) -> Option<u32> {
        None
    }
}

// ── from npm_registry.rs ──
#[derive(Debug)]
pub struct NoopNpmRegistryService;

#[async_trait]
impl NpmRegistryService for NoopNpmRegistryService {
    async fn download_package(
        &self,
        _name: &str,
        _version: Option<&str>,
        _dest: &Path,
    ) -> std::result::Result<(), String> {
        Err("npm registry service is not configured".to_string())
    }
}

// ── from observability.rs ──
#[derive(Default)]
pub struct NoopObservabilityProvider;
#[async_trait]
impl ObservabilityProvider for NoopObservabilityProvider {
    async fn start_span(&self, _: &str, _: ObservabilitySpanType, _: Map<String, Value>) {}
    async fn end_span(&self, _: Map<String, Value>) {}
    async fn record_event(&self, _: &str, _: Map<String, Value>) {}
    async fn record_metric(&self, _: &str, _: f64, _: Map<String, Value>) {}
    async fn record_error(&self, _: &str, _: Map<String, Value>) {}
    async fn export_traces(&self) -> std::result::Result<String, String> {
        Ok("[]".into())
    }
    async fn export_metrics(&self) -> std::result::Result<String, String> {
        Ok("{}".into())
    }
}

// ── from output_sanitizer.rs ──
#[derive(Debug, Clone)]
pub struct NoopOutputSanitizer;

impl OutputSanitizer for NoopOutputSanitizer {
    fn sanitize(&self, output: &str, _ctx: &SanitizeContext) -> String {
        output.to_string()
    }
}

// ── from planner.rs ──
#[derive(Debug)]
pub struct NoopPlannerAdapter;

impl PlannerAdapter for NoopPlannerAdapter {
    fn create_plan(
        &mut self,
        _goal: &str,
        _phases_json: &[serde_json::Value],
    ) -> std::result::Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn start_execution(&mut self) -> std::result::Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn current_plan(&self) -> Option<serde_json::Value> {
        None
    }

    fn request_replan(
        &mut self,
        _reason: &str,
        _actions_json: &[serde_json::Value],
    ) -> std::result::Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn is_completed(&self) -> bool {
        false
    }

    fn mark_task_completed(
        &mut self,
        _phase_index: usize,
        _task_index: usize,
        _result: serde_json::Value,
    ) {
    }

    fn mark_phase_completed(&mut self, _phase_index: usize) -> std::result::Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn get_failed_steps(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_pending_steps(&self) -> Vec<String> {
        Vec::new()
    }
}

// ── from platform_manager.rs ──
#[derive(Default)]
pub struct NoopPlatformManager;
#[async_trait]
impl PlatformManager for NoopPlatformManager {
    async fn start_all(&self) -> std::result::Result<(), String> {
        Err("not configured".into())
    }
    async fn stop_all(&self) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn start_platform(&self, _: &str) -> std::result::Result<(), String> {
        Err("not configured".into())
    }
    async fn stop_platform(&self, _: &str) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn get_connections(&self) -> std::result::Result<Vec<PlatformConnectionInfo>, String> {
        Ok(Vec::new())
    }
    async fn send_message(&self, _: &str, _: &str, _: &str) -> std::result::Result<(), String> {
        Err("not configured".into())
    }
}

// ── from profile.rs ──
#[derive(Default)]
pub struct NoopUserProfileService;
#[async_trait]
impl UserProfileService for NoopUserProfileService {
    async fn get_profile(&self) -> std::result::Result<UserProfile, String> {
        Err("user profile not configured".to_string())
    }
    async fn update_profile(&self, _update: ProfileUpdate) -> std::result::Result<(), String> {
        Err("user profile not configured".to_string())
    }
    async fn reset_profile(&self) -> std::result::Result<(), String> {
        Err("user profile not configured".to_string())
    }
}

// ── from prompt_guard.rs ──
#[derive(Debug)]
pub struct NoopPromptGuard;

impl PromptGuard for NoopPromptGuard {
    fn process_user_input(&self, input: &str) -> std::result::Result<String, String> {
        Ok(input.to_string())
    }

    fn process_external_data(
        &self,
        content: &str,
        _source_label: &str,
        _source_id: &str,
    ) -> String {
        content.to_string()
    }
}

// ── from prompt_provider.rs ──
pub struct NoopPromptProvider;

impl PromptProvider for NoopPromptProvider {
    fn get(&self, _key: &str, _lang: PromptLang) -> &'static str {
        ""
    }

    fn get_all_languages(&self, _key: &str) -> HashMap<String, &'static str> {
        HashMap::new()
    }
}

// ── from rag_provider.rs ──
#[derive(Default)]
pub struct NoopEmbeddingProvider;
#[async_trait]
impl EmbeddingProvider for NoopEmbeddingProvider {
    async fn embed(&self, _: &str) -> std::result::Result<Vec<f32>, String> {
        Err("not configured".into())
    }
    async fn embed_batch(&self, _: &[String]) -> std::result::Result<Vec<Vec<f32>>, String> {
        Err("not configured".into())
    }
    fn dimension(&self) -> usize {
        0
    }
}

// ── from rag_provider.rs ──
#[derive(Default)]
pub struct NoopVectorStoreProvider;
#[async_trait]
impl VectorStoreProvider for NoopVectorStoreProvider {
    async fn search(
        &self,
        _: &str,
        _: &[f32],
        _: usize,
    ) -> std::result::Result<Vec<VectorQueryResult>, String> {
        Ok(Vec::new())
    }
    async fn upsert(
        &self,
        _: &str,
        _: &str,
        _: &[f32],
        _: &str,
    ) -> std::result::Result<(), String> {
        Err("not configured".into())
    }
    async fn delete(&self, _: &str, _: &str) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn clear_collection(&self, _: &str) -> std::result::Result<(), String> {
        Ok(())
    }
}

// ── from rag_provider.rs ──
#[derive(Default)]
pub struct NoopRerankProvider;
#[async_trait]
impl RerankProvider for NoopRerankProvider {
    async fn rerank(
        &self,
        _: &str,
        items: &[RagRetrievedItem],
        _: usize,
    ) -> std::result::Result<Vec<RagRetrievedItem>, String> {
        Ok(items.to_vec())
    }
}

// ── from rag_provider.rs ──
#[derive(Default)]
pub struct NoopSelfRagProvider;
#[async_trait]
impl SelfRagProvider for NoopSelfRagProvider {
    async fn judge_chunks(
        &self,
        _: &str,
        _: &[String],
    ) -> std::result::Result<RetrievalQuality, String> {
        Ok(RetrievalQuality::Good)
    }
    async fn refine_query(&self, query: &str, _: &str) -> std::result::Result<String, String> {
        Ok(query.to_string())
    }
}

// ── from rag_provider.rs ──
#[derive(Default)]
pub struct NoopRAGProvider;
#[async_trait]
impl RAGProvider for NoopRAGProvider {
    async fn retrieve(&self, _: &RAGQuery) -> std::result::Result<RagContextResult, String> {
        Err("not configured".into())
    }
    async fn hybrid_search(
        &self,
        _: &RAGQuery,
    ) -> std::result::Result<Vec<RagRetrievedItem>, String> {
        Ok(Vec::new())
    }
    fn available_collections(&self) -> Vec<String> {
        Vec::new()
    }
}

// ── from rate_limiter.rs ──
#[derive(Default)]
pub struct NoopRateLimiter;
#[async_trait]
impl RateLimiter for NoopRateLimiter {
    async fn check(&self, _: &str) -> RateLimitResult {
        RateLimitResult::Allowed
    }
    async fn record(&self, _: &str) {}
    async fn reset(&self, _: &str) {}
    async fn status(&self, _: &str) -> std::result::Result<RateLimitStatus, String> {
        Ok(RateLimitStatus {
            current_count: 0,
            max_requests: 0,
            window_secs: 0,
            remaining: 0,
            reset_after_secs: 0,
        })
    }
}

// ── from rhai_engine.rs ──
#[derive(Debug)]
pub struct NoopRhaiEngineAdapter;

impl RhaiEngineAdapter for NoopRhaiEngineAdapter {
    fn register_scripts(&self, _scripts: &[serde_json::Value]) {}

    fn execute_script(
        &self,
        _script_name: &str,
        _args: serde_json::Value,
        _tool_fns: &HashMap<String, RhaiToolFn>,
    ) -> std::result::Result<serde_json::Value, String> {
        Err("Rhai engine is not configured".to_string())
    }
}

// ── from rl.rs ──
#[derive(Default)]
pub struct NoopRLEngine;
#[async_trait]
impl RLEngine for NoopRLEngine {
    async fn compute_rewards(
        &self,
        _episodes: &[TrainingEpisode],
    ) -> std::result::Result<Vec<f64>, String> {
        Ok(vec![0.0])
    }
    async fn compute_advantages(&self, _rewards: &[f64]) -> Vec<f64> {
        vec![0.0]
    }
    async fn reset(&self) {}
}

// ── from rl.rs ──
#[derive(Default)]
pub struct NoopRLTrainer;
#[async_trait]
impl RLTrainer for NoopRLTrainer {
    async fn train_episode(
        &self,
        _episode: TrainingEpisode,
    ) -> std::result::Result<TrainingReport, String> {
        Err("RL trainer not configured".to_string())
    }
    async fn get_progress(&self) -> std::result::Result<TrainingReport, String> {
        Ok(TrainingReport {
            episodes_trained: 0,
            avg_reward: 0.0,
            max_reward: 0.0,
            total_steps: 0,
            duration_secs: 0.0,
        })
    }
}

// ── from scanner.rs ──
#[derive(Debug, Default)]
pub struct NoopMemoryScanner;

#[async_trait]
impl MemoryScanner for NoopMemoryScanner {
    async fn scan(&self, _config: &ScannerConfig) -> std::result::Result<ScanResult, String> {
        Ok(ScanResult::default())
    }
}

// ── from session_tracer.rs ──
#[derive(Debug)]
pub struct NoopSessionTracer;

impl SessionTracer for NoopSessionTracer {
    fn record(&self, _name: &str, _attributes: Map<String, Value>) {}
}

// ── Telemetry test mocks ──
//
// 专为 consumer crate（agent / runtime-core / gateway 等）单元测试设计的轻量
// telemetry mock。避免 consumer 在 dev-dependencies 中引入实现层 axagent-telemetry
// crate（违反 AGENTS.md 铁律 5：consumer crate 测试只能通过
// `axagent_harness::test_support::*` mock）。
//
// 仅保留测试断言所需的 `SessionTrace` 变体与 `name` 字段，其余字段（session_id、
// sequence、timestamp_ms、attributes 等）省略。若测试需要更丰富的事件结构，应
// 扩展本 mock 而非退回依赖 axagent-telemetry。

/// 简化的 SessionTrace 记录，仅保留测试断言所需的 `name` 字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTraceRecord {
    pub name: String,
}

/// 简化的 telemetry 事件枚举，仅含 `SessionTrace` 变体（满足测试断言）。
///
/// 标记为 `#[non_exhaustive]` 以便未来扩展（如需断言 HTTP/Analytics 事件）
/// 时不必破坏现有 match；同时使跨 crate match 中的 `_` 模式不算 unreachable。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TelemetryEvent {
    SessionTrace(SessionTraceRecord),
}

/// 内存 telemetry sink，收集事件供测试断言。
///
/// 等价于 `axagent_telemetry::MemoryTelemetrySink`，但不依赖实现层 crate。
/// 所有锁操作均通过 `PoisonError::into_inner` 恢复，避免测试中 panic。
#[derive(Default, Debug)]
pub struct MemoryTelemetrySink {
    events: parking_lot::Mutex<Vec<TelemetryEvent>>,
}

impl MemoryTelemetrySink {
    #[must_use]
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events.lock().clone()
    }

    fn push(&self, event: TelemetryEvent) {
        self.events.lock().push(event);
    }
}

/// 测试用 `SessionTracer` 实现，将 `record` 调用转换为 `TelemetryEvent::SessionTrace`
/// 存入关联的 `MemoryTelemetrySink`。
///
/// 用于替代 `axagent_telemetry::SessionTracer::new(session_id, sink)` 在测试中的
/// 构造场景。本实现忽略 `attributes`（测试断言通常只关心 `name`）。
#[derive(Debug)]
pub struct MemorySessionTracer {
    sink: Arc<MemoryTelemetrySink>,
}

impl MemorySessionTracer {
    #[must_use]
    pub fn new(sink: Arc<MemoryTelemetrySink>) -> Self {
        Self { sink }
    }
}

impl SessionTracer for MemorySessionTracer {
    fn record(&self, name: &str, _attributes: Map<String, Value>) {
        self.sink.push(TelemetryEvent::SessionTrace(SessionTraceRecord { name: name.to_string() }));
    }
}

// ── from ssrf_guard.rs ──
#[allow(deprecated)]
#[derive(Default)]
pub struct NoopSsrFGuard {
    #[allow(deprecated)]
    config: SsrFConfig,
}
#[async_trait]
#[allow(deprecated)]
impl SsrFGuard for NoopSsrFGuard {
    async fn check_url(&self, _: &str) -> UrlSafety {
        tracing::warn!("NoopSsrFGuard is active - SSRF protection disabled");
        UrlSafety::Safe
    }
    fn config(&self) -> &SsrFConfig {
        &self.config
    }
    async fn safe_client(&self) -> std::result::Result<reqwest::Client, String> {
        tracing::warn!("NoopSsrFGuard is active - SSRF protection disabled");
        reqwest::Client::builder().build().map_err(|e| e.to_string())
    }
}

// ── from style.rs ──
#[derive(Default)]
pub struct NoopStyleExtractor;
#[async_trait]
impl StyleExtractor for NoopStyleExtractor {
    async fn extract_from_code(
        &self,
        _: &[CodeSample],
    ) -> std::result::Result<ExtractedCodePatterns, String> {
        Err("not configured".into())
    }
    async fn extract_from_messages(
        &self,
        _: &[MessageSample],
    ) -> std::result::Result<DocumentStyleProfile, String> {
        Err("not configured".into())
    }
}

// ── from style.rs ──
#[derive(Default)]
pub struct NoopStyleApplier;
#[async_trait]
impl StyleApplier for NoopStyleApplier {
    async fn apply_style(
        &self,
        code: &str,
        _: &CodeStyleTemplate,
    ) -> std::result::Result<String, String> {
        Ok(code.to_string())
    }
    fn active_template(&self) -> Option<CodeStyleTemplate> {
        None
    }
}

// ── from style.rs ──
#[derive(Default)]
pub struct NoopStyleVectorizer;
#[async_trait]
impl StyleVectorizer for NoopStyleVectorizer {
    async fn vectorize_code(&self, _: &CodeSample) -> std::result::Result<StyleVector, String> {
        Err("not configured".into())
    }
    async fn vectorize_message(
        &self,
        _: &MessageSample,
    ) -> std::result::Result<StyleVector, String> {
        Err("not configured".into())
    }
}

// ── from task_shape.rs ──

use crate::task_shape::{TaskShapeDecision, TaskShapeLlmClassifier};

/// 返回 `Ok(None)` 的 TaskShapeLlmClassifier 空实现（回退规则结论）。
#[derive(Debug)]
pub struct NoopTaskShapeLlmClassifier;

#[async_trait]
impl TaskShapeLlmClassifier for NoopTaskShapeLlmClassifier {
    async fn refine(
        &self,
        _user_input: &str,
        _rule_result: &TaskShapeDecision,
    ) -> std::result::Result<Option<TaskShapeDecision>, String> {
        Ok(None)
    }
}

// ── from tool_access.rs ──
#[derive(Default)]
pub struct NoopToolAccessControl;
#[async_trait]
impl ToolAccessControl for NoopToolAccessControl {
    async fn check_access(&self, _: &ToolAccessRequest) -> AccessDecision {
        AccessDecision::Allow
    }
    async fn record_result(&self, _: &ToolAccessRequest, _: bool, _: Option<&str>) {}
}

// ── from tool_metrics.rs ──
#[derive(Default)]
pub struct NoopToolMetricsCollector;
#[async_trait]
impl ToolMetricsCollector for NoopToolMetricsCollector {
    async fn record_call(&self, _: ToolCallRecord) {}
    async fn snapshot(&self) -> ToolMetricsSnapshot {
        ToolMetricsSnapshot {
            total_calls: 0,
            success_count: 0,
            error_count: 0,
            avg_duration_ms: 0.0,
            p99_duration_ms: 0.0,
            calls_by_tool: Vec::new(),
        }
    }
    async fn tool_stats(&self, _: &str) -> std::result::Result<ToolMetricsSnapshot, String> {
        Ok(ToolMetricsSnapshot {
            total_calls: 0,
            success_count: 0,
            error_count: 0,
            avg_duration_ms: 0.0,
            p99_duration_ms: 0.0,
            calls_by_tool: Vec::new(),
        })
    }
    async fn reset(&self) {}
}

// ── from trajectory_service.rs ──
#[derive(Debug)]
pub struct NoopTrajectoryService;

impl TrajectoryService for NoopTrajectoryService {
    fn extract_entities(&self, _messages: &[serde_json::Value]) -> Vec<String> {
        Vec::new()
    }

    fn verify_compression_integrity(
        &self,
        _original: &[serde_json::Value],
        _compressed: &[serde_json::Value],
        _key_entities: &[String],
    ) -> IntegrityResult {
        IntegrityResult { is_valid: true, checks: Vec::new() }
    }

    fn estimate_complexity(&self, _input: &str) -> TaskComplexity {
        TaskComplexity::Medium
    }
}

// ── from webhook_subscription.rs ──
#[derive(Debug)]
pub struct NoopWebhookSubscriptionService;

#[async_trait::async_trait]
impl WebhookSubscriptionService for NoopWebhookSubscriptionService {
    async fn subscribe(
        &self,
        _url: String,
        _event: &str,
        _secret: Option<String>,
    ) -> std::result::Result<WebhookSubscriptionInfo, String> {
        Err("Webhook subscription service is not configured".to_string())
    }

    async fn get_subscriptions_for_event(&self, _event: &str) -> Vec<WebhookSubscriptionInfo> {
        Vec::new()
    }

    async fn unsubscribe(&self, _subscription_id: &str) -> std::result::Result<(), String> {
        Ok(())
    }

    async fn reset_failures(&self, _subscription_id: &str) {}

    async fn increment_failure(&self, _subscription_id: &str) {}

    async fn update_last_triggered(&self, _subscription_id: &str) {}

    async fn list_subscriptions(&self) -> Vec<WebhookSubscriptionInfo> {
        Vec::new()
    }
}

// ── Wiki / Note repo test doubles ──────────────────────────

use crate::note_dtos::{CreateNoteInput, Note, UpdateNoteInput};
use crate::types::Wiki;
use crate::wiki_dtos::{
    InsertWikiSourceInput, NoteRepository, NoteVersion, WikiOperation, WikiOperationRepository,
    WikiPage, WikiPageRepository, WikiRepository, WikiSource, WikiSourceRepository,
};

struct EmptyNoteRepository;
#[async_trait]
impl NoteRepository for EmptyNoteRepository {
    async fn find_by_id(&self, _id: &str) -> std::result::Result<Option<Note>, String> {
        Ok(None)
    }
    async fn find_by_vault(
        &self,
        _vault_id: &str,
        _include_deleted: bool,
    ) -> std::result::Result<Vec<Note>, String> {
        Ok(Vec::new())
    }
    async fn find_by_vault_and_title(
        &self,
        _vault_id: &str,
        _title: &str,
        _include_deleted: bool,
    ) -> std::result::Result<Vec<Note>, String> {
        Ok(Vec::new())
    }
    async fn create_note(&self, _input: CreateNoteInput) -> std::result::Result<Note, String> {
        Err("not implemented".into())
    }
    async fn update_note(
        &self,
        _note_id: &str,
        _input: UpdateNoteInput,
    ) -> std::result::Result<Note, String> {
        Err("not implemented".into())
    }

    async fn find_link_target_ids(
        &self,
        _note_id: &str,
    ) -> std::result::Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

struct EmptyWikiRepository;
#[async_trait]
impl WikiRepository for EmptyWikiRepository {
    async fn find_by_id(&self, _id: &str) -> std::result::Result<Option<Wiki>, String> {
        Ok(None)
    }
    async fn create_version(
        &self,
        _wiki_id: &str,
        _note_id: &str,
        _title: &str,
        _content: &str,
        _author: &str,
    ) -> std::result::Result<NoteVersion, String> {
        Err("not implemented".into())
    }
    async fn increment_note_count(&self, _wiki_id: &str) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn update_schema_version(
        &self,
        _wiki_id: &str,
        _version: &str,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
}

struct EmptyWikiPageRepository;
#[async_trait]
impl WikiPageRepository for EmptyWikiPageRepository {
    async fn find_by_note_id(
        &self,
        _note_id: &str,
    ) -> std::result::Result<Option<WikiPage>, String> {
        Ok(None)
    }
    async fn find_by_wiki_id(&self, _wiki_id: &str) -> std::result::Result<Vec<WikiPage>, String> {
        Ok(Vec::new())
    }
    async fn upsert(&self, _page: WikiPage) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn update_lint_result(
        &self,
        _note_id: &str,
        _quality_score: Option<f64>,
        _last_linted_at: Option<i64>,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
}

struct EmptyWikiSourceRepository;
#[async_trait]
impl WikiSourceRepository for EmptyWikiSourceRepository {
    async fn find_by_id(
        &self,
        _source_id: &str,
    ) -> std::result::Result<Option<WikiSource>, String> {
        Ok(None)
    }
    async fn find_by_wiki_id(
        &self,
        _wiki_id: &str,
    ) -> std::result::Result<Vec<WikiSource>, String> {
        Ok(Vec::new())
    }
    async fn count_by_wiki_id(&self, _wiki_id: &str) -> std::result::Result<usize, String> {
        Ok(0)
    }
    async fn insert(
        &self,
        _input: InsertWikiSourceInput,
    ) -> std::result::Result<WikiSource, String> {
        Err("not implemented".into())
    }
    async fn delete(&self, _source_id: &str) -> std::result::Result<bool, String> {
        Ok(false)
    }
}

struct EmptyWikiOperationRepository;
#[async_trait]
impl WikiOperationRepository for EmptyWikiOperationRepository {
    async fn log(&self, _op: WikiOperation) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub fn empty_note_repo() -> Arc<dyn NoteRepository> {
    Arc::new(EmptyNoteRepository)
}
pub fn empty_wiki_repo() -> Arc<dyn WikiRepository> {
    Arc::new(EmptyWikiRepository)
}
pub fn empty_wiki_page_repo() -> Arc<dyn WikiPageRepository> {
    Arc::new(EmptyWikiPageRepository)
}
pub fn empty_wiki_source_repo() -> Arc<dyn WikiSourceRepository> {
    Arc::new(EmptyWikiSourceRepository)
}
pub fn empty_wiki_operation_repo() -> Arc<dyn WikiOperationRepository> {
    Arc::new(EmptyWikiOperationRepository)
}

struct EmptyWorkflowExecutionRepository;
#[async_trait]
impl WorkflowExecutionRepository for EmptyWorkflowExecutionRepository {
    async fn create_workflow_execution(
        &self,
        _id: &str,
        _workflow_id: &str,
        _input_params: Option<&str>,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn update_workflow_execution_status(
        &self,
        _id: &str,
        _status: &str,
        _output_result: Option<&str>,
        _node_executions: Option<&str>,
        _total_time_ms: Option<i32>,
    ) -> std::result::Result<bool, String> {
        Ok(true)
    }
    async fn list_workflow_executions(
        &self,
        _workflow_id: &str,
    ) -> std::result::Result<Vec<WorkflowExecutionData>, String> {
        Ok(vec![])
    }
    async fn save_execution_state(
        &self,
        _id: &str,
        _status: &str,
        _execution_state_json: &str,
    ) -> std::result::Result<bool, String> {
        Ok(true)
    }
    async fn clear_execution_state(
        &self,
        _id: &str,
        _status: &str,
    ) -> std::result::Result<bool, String> {
        Ok(true)
    }
    async fn list_paused_executions(
        &self,
    ) -> std::result::Result<Vec<WorkflowExecutionData>, String> {
        Ok(vec![])
    }
}

pub fn empty_workflow_execution_repo() -> Arc<dyn WorkflowExecutionRepository> {
    Arc::new(EmptyWorkflowExecutionRepository)
}

struct EmptyLoopCheckpointRepository;
#[async_trait]
impl LoopCheckpointRepository for EmptyLoopCheckpointRepository {
    async fn save_loop_checkpoint(&self, _cp: &LoopCheckpoint) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn load_loop_checkpoint(
        &self,
        _execution_id: &str,
        _node_id: &str,
    ) -> std::result::Result<Option<LoopCheckpoint>, String> {
        Ok(None)
    }
    async fn delete_loop_checkpoint(
        &self,
        _execution_id: &str,
        _node_id: &str,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
    async fn delete_loop_checkpoints_for_execution(
        &self,
        _execution_id: &str,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub fn empty_loop_checkpoint_repo() -> Arc<dyn LoopCheckpointRepository> {
    Arc::new(EmptyLoopCheckpointRepository)
}

struct EmptyWorkflowTemplateRepository;
#[async_trait]
impl WorkflowTemplateRepository for EmptyWorkflowTemplateRepository {
    async fn get_workflow_template(
        &self,
        _id: &str,
    ) -> std::result::Result<Option<WorkflowTemplateData>, String> {
        Ok(None)
    }

    async fn create_workflow_template(
        &self,
        template: WorkflowTemplateData,
    ) -> std::result::Result<String, String> {
        Ok(template.id)
    }

    async fn update_workflow_template(
        &self,
        _template: WorkflowTemplateData,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub fn empty_workflow_template_repo() -> Arc<dyn WorkflowTemplateRepository> {
    Arc::new(EmptyWorkflowTemplateRepository)
}

// ── NoteBacklinkRepository 测试替身 ──────────────────────────
//
// 为 consumer crate（agent 等）单元测试提供 NoteBacklinkRepository 的空实现，
// 避免引入实现层 axagent-dao crate（违反 AGENTS.md 铁律 5）。

use crate::wiki_dtos::{NoteBacklink, NoteBacklinkRepository};

struct EmptyNoteBacklinkRepository;
#[async_trait]
impl NoteBacklinkRepository for EmptyNoteBacklinkRepository {
    async fn count_by_target_note_id(&self, _note_id: &str) -> std::result::Result<usize, String> {
        Ok(0)
    }
    async fn batch_count_by_target_note_ids(
        &self,
        _note_ids: &[String],
    ) -> std::result::Result<HashMap<String, i64>, String> {
        Ok(HashMap::new())
    }
    async fn find_by_target_note_id(
        &self,
        _note_id: &str,
    ) -> std::result::Result<Vec<NoteBacklink>, String> {
        Ok(Vec::new())
    }
}

/// 工厂：构造一个 `Arc<dyn NoteBacklinkRepository>` 测试替身（所有方法返回空值）
pub fn empty_note_backlink_repo() -> Arc<dyn NoteBacklinkRepository> {
    Arc::new(EmptyNoteBacklinkRepository)
}

// ── AgentSessionRepository 测试替身 ──────────────────────────
//
// 为 consumer crate（agent 等）单元测试提供 AgentSessionRepository 的空实现，
// 避免引入实现层 axagent-dao crate（违反 AGENTS.md 铁律 5）。
// 注意：upsert_agent_session 返回一个最小化的 AgentSession，字段均为默认值，
// 仅供"不依赖具体会话数据"的测试使用。需要真实 DB 行为的测试应下沉到 wiring 层。

use crate::agent_session_repo::AgentSessionRepository;
use crate::types::AgentSession;

struct EmptyAgentSessionRepository;
#[async_trait]
impl AgentSessionRepository for EmptyAgentSessionRepository {
    async fn upsert_agent_session(
        &self,
        conversation_id: &str,
        _cwd: Option<&str>,
        permission_mode: Option<&str>,
    ) -> Result<AgentSession> {
        Ok(AgentSession {
            id: String::new(),
            conversation_id: conversation_id.to_string(),
            cwd: None,
            workspace_locked: 0,
            permission_mode: permission_mode.unwrap_or("default").to_string(),
            runtime_status: String::new(),
            sdk_context_json: None,
            sdk_context_backup_json: None,
            total_tokens: 0,
            total_cost_usd: 0.0,
            created_at: 0,
            updated_at: 0,
        })
    }
    async fn update_agent_session_status(&self, _id: &str, _runtime_status: &str) -> Result<()> {
        Ok(())
    }
    async fn update_agent_session_after_query(
        &self,
        _id: &str,
        _runtime_status: &str,
        _sdk_context_json: Option<&str>,
        _tokens_delta: i64,
        _cost_delta: f64,
    ) -> Result<()> {
        Ok(())
    }
    async fn clear_sdk_context_by_conversation_id(&self, _conversation_id: &str) -> Result<()> {
        Ok(())
    }
    async fn get_by_id(&self, _id: &str) -> Result<Option<AgentSession>> {
        Ok(None)
    }
    async fn get_by_conversation_id(&self, _conversation_id: &str) -> Result<Option<AgentSession>> {
        Ok(None)
    }
    async fn list_all(&self) -> Result<Vec<AgentSession>> {
        Ok(Vec::new())
    }
}

/// 工厂：构造一个 `Arc<dyn AgentSessionRepository>` 测试替身（所有写方法返回 Ok(())，
/// upsert 返回最小化 AgentSession）
pub fn empty_agent_session_repo() -> Arc<dyn AgentSessionRepository> {
    Arc::new(EmptyAgentSessionRepository)
}

// ── ToolRegistry 测试替身 ────────────────────────────────────
//
// 为 consumer crate（agent 等）单元测试提供 ToolRegistry 的空实现，
// 避免引入实现层 axagent-tools crate（违反 AGENTS.md 铁律 5）。
// get / find 返回 None，list 返回空 Vec，适合测试"工具不存在"的报错路径。

use crate::registry::ToolRegistry as ToolRegistryTrait;
use crate::tool::{Tool, ToolCategory, ToolInfo};

/// 空实现的 ToolRegistry 测试替身。
///
/// - `get` / `find` 返回 `None`
/// - `list` / `list_by_category` 返回空 Vec
/// - `is_disabled` 返回 `false`
/// - `execute_tool` 走 trait 默认实现（因 `find` 返回 None，会得到 `ToolError::not_found`）
#[derive(Debug, Default)]
pub struct NoopToolRegistry;

#[async_trait::async_trait]
impl ToolRegistryTrait for NoopToolRegistry {
    fn get(&self, _name: &str) -> Option<Arc<dyn Tool>> {
        None
    }
    fn find(&self, _name: &str) -> Option<Arc<dyn Tool>> {
        None
    }
    fn list(&self) -> Vec<ToolInfo> {
        Vec::new()
    }
    fn list_by_category(&self, _category: ToolCategory) -> Vec<ToolInfo> {
        Vec::new()
    }
    fn is_disabled(&self, _name: &str) -> bool {
        false
    }
}

/// 工厂：构造一个 `Arc<dyn ToolRegistry>` 测试替身
pub fn noop_tool_registry() -> Arc<dyn ToolRegistryTrait> {
    Arc::new(NoopToolRegistry)
}

// ── AgentRoleRepository / AgencyExpertRepository 空实现（用于 workflow_ai 测试）──

use crate::repo_dtos::{AgencyExpertDto, AgentRoleDto};
use crate::repositories::{AgencyExpertRepository, AgentRoleRepository};

struct NoopAgentRoleRepository;
#[async_trait]
impl AgentRoleRepository for NoopAgentRoleRepository {
    async fn get_agent_role(&self, _id: &str) -> std::result::Result<Option<AgentRoleDto>, String> {
        Ok(None)
    }
    async fn list_agent_roles(&self) -> std::result::Result<Vec<AgentRoleDto>, String> {
        Ok(vec![])
    }
}

struct NoopAgencyExpertRepository;
#[async_trait]
impl AgencyExpertRepository for NoopAgencyExpertRepository {
    async fn get_agency_expert(
        &self,
        _id: &str,
    ) -> std::result::Result<Option<AgencyExpertDto>, String> {
        Ok(None)
    }
    async fn list_agency_experts(&self) -> std::result::Result<Vec<AgencyExpertDto>, String> {
        Ok(vec![])
    }
}

/// 注册 Noop 的角色与专家仓储，使 `build_roles_and_experts_brief()` 在测试中不崩溃。
pub fn register_noop_role_and_expert_repos() {
    use crate::repositories::{set_agency_expert_repository, set_agent_role_repository};
    set_agent_role_repository(Arc::new(NoopAgentRoleRepository));
    set_agency_expert_repository(Arc::new(NoopAgencyExpertRepository));
}

// ── 会话状态存储测试替身 ──────────────────────────────

/// 内存版 `SessionStateStore` —— 让不碰数据库的测试也能构造完整依赖。
///
/// TTL 在读取侧生效（与生产实现同口径）：过期条目读为 `None`，
/// 但不会自行从 map 里移除，直到 `purge_expired` 被调用。
#[derive(Debug, Default)]
pub struct MemorySessionStateStore {
    entries: std::sync::RwLock<HashMap<String, (String, Option<i64>)>>,
}

impl MemorySessionStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl crate::SessionStateStore for MemorySessionStateStore {
    async fn set(
        &self,
        key: &str,
        value: &str,
        ttl_ms: Option<i64>,
    ) -> std::result::Result<(), String> {
        let expires_at_ms = ttl_ms.map(|ttl| crate::util_fns::now_ms().saturating_add(ttl));
        self.entries
            .write()
            .map_err(|e| format!("会话状态写锁中毒: {e}"))?
            .insert(key.to_string(), (value.to_string(), expires_at_ms));
        Ok(())
    }

    async fn get(&self, key: &str) -> std::result::Result<Option<String>, String> {
        let guard = self.entries.read().map_err(|e| format!("会话状态读锁中毒: {e}"))?;
        let Some((value, expires_at_ms)) = guard.get(key) else {
            return Ok(None);
        };
        let now = crate::util_fns::now_ms();
        if expires_at_ms.is_some_and(|exp| exp <= now) {
            return Ok(None);
        }
        Ok(Some(value.clone()))
    }

    async fn delete(&self, key: &str) -> std::result::Result<(), String> {
        self.entries.write().map_err(|e| format!("会话状态写锁中毒: {e}"))?.remove(key);
        Ok(())
    }

    async fn list_by_prefix(
        &self,
        prefix: &str,
    ) -> std::result::Result<Vec<crate::SessionStateEntry>, String> {
        let now = crate::util_fns::now_ms();
        let guard = self.entries.read().map_err(|e| format!("会话状态读锁中毒: {e}"))?;
        Ok(guard
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .filter(|(_, (_, exp))| !exp.is_some_and(|e| e <= now))
            .map(|(k, (v, exp))| crate::SessionStateEntry {
                key: k.clone(),
                value: v.clone(),
                scope: k.split(':').next().unwrap_or("temp").to_string(),
                conversation_id: None,
                agent_id: None,
                updated_at_ms: now,
                expires_at_ms: *exp,
            })
            .collect())
    }

    async fn purge_expired(&self) -> std::result::Result<usize, String> {
        let now = crate::util_fns::now_ms();
        let mut guard = self.entries.write().map_err(|e| format!("会话状态写锁中毒: {e}"))?;
        let before = guard.len();
        guard.retain(|_, (_, exp)| !exp.is_some_and(|e| e <= now));
        Ok(before - guard.len())
    }
}
