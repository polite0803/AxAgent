// SPDX-License-Identifier: AGPL-3.0-only
//! 认知路由器 — 三层路由树协调器（Phase 4 集成层）
//!
//! # 架构
//! ```text
//! 用户输入
//!     │
//!     ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    CognitiveRouter                           │
//! │                                                              │
//! │  Step 1: L1 域路由 → DomainRoutingResult                     │
//! │  Step 2: L2 簇路由 → ClusterRoutingResult                    │
//! │  Step 3: L3a RAR 检索 → RarSearchResult (Top-K 候选)         │
//! │  Step 4: L3b 图谱路由 → 路径选择                             │
//! │  Step 5: 熔断检查 → RarCircuitBreaker                       │
//! │  Step 6: 最终决策 → RoutingDecisionV2                       │
//! └─────────────────────────────────────────────────────────────┘
//!     │
//!     ▼
//! RoutingDecisionV2 {
//!     route_path: "finance/stock_analysis/tech",
//!     domain: "finance",
//!     cluster: "stock_analysis",
//!     capability_id: "wf_tech",
//!     confidence: 0.98,
//!     ...
//! }
//! ```
//!
//! # 设计原则
//! - 渐进式精化：从粗到细逐层缩小搜索空间
//! - 熔断保护：任何层级都可能触发熔断，防止自指路由
//! - 可观测性：每层决策都记录耗时和置信度
//! - 确定性输出：route_path 是确定性路径，而非自然语言

use crate::capability::{CapabilityDomain, CapabilityKind};
use crate::cluster_router::{ClusterRouter, ClusterRoutingResult};
use crate::domain_router::{DomainDecision, DomainRouter, DomainRoutingResult, LlmReasoner};
use crate::rar_router::{RarCircuitBreaker, RarRouter};
use crate::workflow_graph::{WorkflowGraph, WorkflowGraphRouter, WorkflowGraphSync};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

// ── 系统能力 ID ──────────────────────────────────

/// 系统能力：RAR 向量检索（L3 子工作流中的 `system_rar_retriever` 节点）
///
/// 输入（HashMap）：`user_input` / `l1_domain` / `l2_cluster`
/// 输出（Value）：`{ count, candidates: [{id,name,description,score,kind,domain,cluster}], raw_count }`
pub const SYSTEM_RAR_RETRIEVER_ID: &str = "system_rar_retriever";

/// 系统能力：工作流图谱路径规划（L3 子工作流中的 `system_workflow_graph_router` 节点）
///
/// 输入（HashMap）：`selected_capability` / `selected_score` / `selected_kind` / `l1_domain` / `l2_cluster` / `user_input`
/// 输出（Value）：`{ route_path, capability_id, confidence, execution_mode, circuit_broken, reason }`
///
/// `selected_kind`：选中候选的能力类型（workflow / agent / tool / knowledge_base / skill）。
/// 非 Workflow 能力即使高置信命中也不会直发工作流（见 `clamp_mode_for_kind`），
/// 而是降级委派 agent 执行，避免触发 WORKFLOW_NOT_FOUND。空串视为 Workflow。
pub const SYSTEM_WORKFLOW_GRAPH_ROUTER_ID: &str = "system_workflow_graph_router";

// ── 路由作用域枚举 ──────────────────────────────────

/// 路由作用域 — 用于物理隔离业务/系统注册表
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingScope {
    /// 业务作用域（用户可见的工作流、技能、工具）
    Business,
    /// 系统作用域（系统内部能力，对用户不可见）
    System,
}

impl RoutingScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoutingScope::Business => "business",
            RoutingScope::System => "system",
        }
    }
}

// ── 自指熔断配置 ──────────────────────────────────

/// 自指熔断保护器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfReferenceProtection {
    /// 熔断触发关键词（工作流 ID/标签包含这些词则触发熔断）
    pub protected_keywords: Vec<String>,
    /// 熔断触发标签（工作流标签包含这些标签则触发熔断）
    pub protected_tags: Vec<String>,
    /// 是否启用自指熔断
    pub enabled: bool,
}

impl Default for SelfReferenceProtection {
    fn default() -> Self {
        Self {
            protected_keywords: vec![
                "cognitive_router".to_string(),
                "orchestrator".to_string(),
                "meta_agent".to_string(),
                "self_improvement".to_string(),
                // 阶段二 T2.4：进化产物自指熔断。
                // 进化/补齐产物（passport ID `evolution:workflow:*`、路由 tag `route:/evolution/*`）
                // 属于"编排器动态生成"的能力，禁止被再次路由/执行，防止通过进化产物递归编排编排器。
                "/evolution/".to_string(),
                "evolution:workflow".to_string(),
                // 自指系统工具护照（`system:self_evolution:*`）亦不可被业务路由命中
                "self_evolution".to_string(),
            ],
            protected_tags: vec![
                "SYSTEM_ONLY".to_string(),
                "INTERNAL".to_string(),
                "META".to_string(),
                "ORCHESTRATOR".to_string(),
            ],
            enabled: true,
        }
    }
}

impl SelfReferenceProtection {
    /// 检查工作流是否触发自指熔断
    pub fn check(&self, workflow_id: &str, tags: &[String]) -> Option<String> {
        if !self.enabled {
            return None;
        }

        // 检查 ID 是否包含保护关键词
        for keyword in &self.protected_keywords {
            if workflow_id.contains(keyword) {
                return Some(format!(
                    "工作流 ID '{}' 包含保护关键词 '{}'，触发自指熔断",
                    workflow_id, keyword
                ));
            }
        }

        // 检查标签是否包含保护标签
        for tag in tags {
            if self.protected_tags.contains(tag) {
                return Some(format!("工作流标签 '{}' 触发自指熔断保护", tag));
            }
        }

        None
    }
}

// ── 用户意图提示枚举 ──────────────────────────────

/// 前端用户意图提示 — 用户显式覆盖执行模式时传入（Auto 表示完全自动决策）
///
/// 由认知编排器统一决策后，前端不再承担模式选择的路径开关职责；
/// 仅在用户显式指定覆盖时通过 `route_with_hint` 传入，路由决策优先尊重。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeHint {
    /// 完全自动决策（默认）
    #[default]
    Auto,
    /// 用户强制自由对话
    Ask,
    /// 用户强制先规划再执行
    Plan,
    /// 用户强制直接行动
    Act,
}

impl ModeHint {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModeHint::Auto => "auto",
            ModeHint::Ask => "ask",
            ModeHint::Plan => "plan",
            ModeHint::Act => "act",
        }
    }

    /// 从字符串解析（未知值回退为 Auto）
    pub fn parse_str(s: &str) -> Self {
        match s {
            "ask" => ModeHint::Ask,
            "plan" => ModeHint::Plan,
            "act" => ModeHint::Act,
            _ => ModeHint::Auto,
        }
    }
}

// ── 路由执行模式枚举 ──────────────────────────────

/// 路由决策的执行模式 — 决定路由结果如何落地执行
///
/// # 模式语义
/// - `Workflow`: 命中确定性工作流能力，由 WorkEngine 执行对应模板
/// - `Delegate`: 命中工具/技能/知识库/Agent 能力，委派给 agent 加载执行
/// - `ParameterExtract`: 精准命中工作流（置信度 > 0.90），跳过澄清直接抽参执行
/// - `Clarify`: 模糊命中（0.60 ~ 0.90），将 Top2 候选交用户澄清后二次路由
/// - `Ask` / `Plan` / `Act`: 未命中确定性能力，交给通用 agent 的三种执行模式
///   （替代原先的手动模式切换，由认知编排器自动决策；用户可经 ModeHint 覆盖）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// 自由对话（无确定性能力命中，agent 直接回答）
    #[default]
    Ask,
    /// 规划（域明确但无具体工作流，agent 先进 plan 模式拆解任务）
    Plan,
    /// 直接行动（域明确且任务直接，agent 直接执行工具）
    Act,
    /// 执行目标工作流（命中 workflow 能力，由 WorkEngine 执行）
    Workflow,
    /// 直接执行（置信度 0.75 ~ 0.90 的中高置信命中）：由 WorkEngine 直接执行目标能力，
    /// 与 `Workflow` 等价（图谱路由系统能力在主 DAG 的 Switch 分支决策中产生）
    Direct,
    /// 委派给 agent 执行指定能力（命中 tool/技能/知识库/Agent）
    Delegate,
    /// 精准命中（置信度 > 0.90）：跳过澄清，直接参数抽取后执行目标能力
    ParameterExtract,
    /// 模糊命中（置信度 0.60 ~ 0.90）：触发澄清分支，Top2 候选交用户选择
    Clarify,
}

impl ExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Ask => "ask",
            ExecutionMode::Plan => "plan",
            ExecutionMode::Act => "act",
            ExecutionMode::Workflow => "workflow",
            ExecutionMode::Direct => "direct",
            ExecutionMode::Delegate => "delegate",
            ExecutionMode::ParameterExtract => "parameter_extract",
            ExecutionMode::Clarify => "clarify",
        }
    }
}

// ── 路由层级枚举 ──────────────────────────────────

/// 路由执行层级（用于追踪和调试）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStage {
    /// L1 域路由
    L1Domain,
    /// L2 簇路由
    L2Cluster,
    /// L3a RAR 检索
    L3RarRetrieval,
    /// L3b 图谱路由
    L3GraphRouting,
    /// 熔断检查
    CircuitBreak,
}

impl RouteStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            RouteStage::L1Domain => "L1_domain",
            RouteStage::L2Cluster => "L2_cluster",
            RouteStage::L3RarRetrieval => "L3_rar_retrieval",
            RouteStage::L3GraphRouting => "L3_graph_routing",
            RouteStage::CircuitBreak => "circuit_break",
        }
    }
}

// ── 候选能力摘要 ──────────────────────────────────

/// 候选能力摘要 — 澄清分支（Clarify）Top2 展示用
///
/// 从 RarCandidate 精简提取（不含大体积的 input_schema / negative_scenarios），
/// 供前端直接渲染候选名称/描述/置信度，无需按 ID 二次查询。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateSummary {
    /// 工作流/能力 ID
    pub capability_id: String,
    /// 名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 向量相似度得分（0-1）
    pub score: f64,
    /// 能力承载载体类型（工作流/工具/技能/知识库/Agent）
    pub kind: CapabilityKind,
    /// 所属业务域
    pub domain: String,
    /// 所属集群
    pub cluster: Option<String>,
    /// 推荐执行专家（AgentProfile ID）。认知编排 Agent 执行路径据此自动选择专家。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile_id: Option<String>,
}

// ── 路由阶段记录 ──────────────────────────────────

/// 单个路由阶段的执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStageRecord {
    /// 阶段标识
    pub stage: RouteStage,
    /// 是否成功
    pub success: bool,
    /// 置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 耗时（毫秒）
    pub elapsed_ms: u64,
    /// 阶段摘要信息
    pub summary: String,
}

// ── 路由决策输出结构 ──────────────────────────────

/// V2 路由决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingDecisionV2 {
    /// 三层路由地址（确定性路径），如 "finance/stock_analysis/tech"
    pub route_path: String,
    /// 业务域
    pub domain: String,
    /// 功能集群
    pub cluster: String,
    /// 具体能力/工作流 ID
    pub capability_id: String,
    /// 路由置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 是否通过 LLM 兜底
    pub is_llm_fallback: bool,
    /// 各阶段执行记录
    pub stage_records: Vec<RouteStageRecord>,
    /// 总耗时（毫秒）
    pub total_elapsed_ms: u64,
    /// 是否触发熔断
    pub circuit_broken: bool,
    /// 熔断原因（如果触发）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_break_reason: Option<String>,
    /// 备选路径（主路径失败时使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_path: Option<String>,
    /// 候选列表（Top-K，仅 ID）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    /// 候选摘要（Top-K，含名称/描述/置信度，Clarify 分支展示用）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub candidate_details: Vec<CandidateSummary>,
    /// 路由决策的执行模式（如何落地执行，由 route 决策）
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    /// P0: 任务形态决策（原则三标尺输出，Step 0 产出，随路由决策留痕）。
    /// `None` 表示未启用 UNITY_P0_TASK_SHAPE flag。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_shape: Option<crate::task_shape::TaskShapeDecision>,
}

impl RoutingDecisionV2 {
    /// 创建空决策
    pub fn empty() -> Self {
        Self {
            route_path: String::new(),
            domain: String::new(),
            cluster: String::new(),
            capability_id: String::new(),
            confidence: 0.0,
            is_llm_fallback: false,
            stage_records: Vec::new(),
            total_elapsed_ms: 0,
            circuit_broken: false,
            circuit_break_reason: None,
            fallback_path: None,
            candidates: Vec::new(),
            candidate_details: Vec::new(),
            execution_mode: ExecutionMode::Ask,
            task_shape: None,
        }
    }

    /// 判断是否为有效决策
    pub fn is_valid(&self) -> bool {
        !self.route_path.is_empty() && !self.circuit_broken
    }

    /// 添加阶段记录
    pub fn add_record(&mut self, record: RouteStageRecord) {
        self.stage_records.push(record);
    }

    /// 标记熔断
    pub fn mark_circuit_broken(&mut self, reason: impl Into<String>) {
        self.circuit_broken = true;
        self.circuit_break_reason = Some(reason.into());
        self.confidence = 0.0;
    }

    /// 从各阶段结果构建路径
    pub fn build_path(domain: &str, cluster: &str, capability_id: &str) -> String {
        format!("{}/{}/{}", domain, cluster, capability_id)
    }
}

// ── 认知路由器配置 ────────────────────────────────

/// 认知路由器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveRouterConfig {
    /// 是否启用 L1 域路由规则（关闭则直接用 LLM）
    pub enable_l1_rules: bool,
    /// 是否启用 L2 簇路由规则
    pub enable_l2_rules: bool,
    /// RAR 检索 Top-K 数量
    pub rar_top_k: usize,
    /// 是否启用图谱路由
    pub enable_graph_routing: bool,
    /// 熔断保护配置
    pub enable_circuit_breaker: bool,
    /// 最大总耗时（毫秒），超过则降级
    pub max_total_ms: u64,
    /// 默认路由作用域（Business 用于业务请求，System 用于系统内部请求）
    pub default_scope: RoutingScope,
    /// 自指熔断保护配置
    pub self_reference_protection: SelfReferenceProtection,
    /// 是否启用作用域隔离（启用后业务请求不会路由到系统能力）
    pub enable_scope_isolation: bool,
}

impl Default for CognitiveRouterConfig {
    fn default() -> Self {
        Self {
            enable_l1_rules: true,
            enable_l2_rules: true,
            rar_top_k: 5,
            enable_graph_routing: true,
            enable_circuit_breaker: true,
            max_total_ms: 5000,
            default_scope: RoutingScope::Business,
            self_reference_protection: SelfReferenceProtection::default(),
            enable_scope_isolation: true,
        }
    }
}

// ── 认知路由器接口 ────────────────────────────────

/// 认知路由器 — 三层路由树协调器
///
/// # 职责
/// 将 L1 域路由、L2 簇路由、L3 RAR+图谱路由串联为完整的路由决策流程。
///
/// # 实现
/// 实际实现由 `DefaultCognitiveRouter` 完成，本 trait 定义接口契约。
#[async_trait]
pub trait CognitiveRouter: Send + Sync {
    /// 执行完整的三层路由决策（默认完全自动模式）
    async fn route(&self, user_input: &str) -> RoutingDecisionV2 {
        self.route_with_hint(user_input, ModeHint::Auto).await
    }

    /// 执行完整的三层路由决策（携带用户意图提示，用户显式覆盖时优先尊重）
    async fn route_with_hint(&self, user_input: &str, mode_hint: ModeHint) -> RoutingDecisionV2;

    /// 仅执行 L1 域路由（快速路径）
    async fn route_l1(&self, user_input: &str) -> DomainRoutingResult;

    /// 仅执行 L1 域路由的纯规则匹配（不触发 LLM 兜底）。
    ///
    /// 用于生成 L2 动态分类目录：规则命中返回业务域，未命中返回 `None`。
    /// 与 [`Self::route_l1`] 的区别在于绝不调用 LLM，避免主 DAG 内部
    /// L1 子工作流重复触发 LLM 兜底（规则未命中时）造成的重复调用。
    /// 默认实现返回 `None`；`DefaultCognitiveRouter` 覆盖实现为对
    /// `DomainRouter::list_rules` 的规则做纯关键词/正则匹配。
    async fn route_l1_rules_only(&self, _user_input: &str) -> Option<CapabilityDomain> {
        None
    }

    /// 执行 L1 + L2 路由
    async fn route_l1_l2(&self, user_input: &str) -> (DomainRoutingResult, ClusterRoutingResult);

    /// 执行系统能力（L3 子工作流中的 `system_*` 节点回调）
    ///
    /// # 支持的系统能力
    /// - [`SYSTEM_RAR_RETRIEVER_ID`]（`system_rar_retriever`）：RAR 向量检索，
    ///   输入 `user_input` / `l1_domain` / `l2_cluster`，输出
    ///   `{ count, candidates: [{id,name,description,score,kind,domain,cluster}], raw_count }`
    /// - [`SYSTEM_WORKFLOW_GRAPH_ROUTER_ID`]（`system_workflow_graph_router`）：图谱路径规划，
    ///   输入 `selected_capability` / `l1_domain` / `l2_cluster` / `user_input`，输出
    ///   `{ route_path, capability_id, confidence, execution_mode, circuit_broken, reason }`
    ///
    /// 未识别的系统能力返回 `Err`，由调用方兜底。
    async fn execute_system_capability(
        &self,
        capability_id: &str,
        input: HashMap<String, Value>,
    ) -> Result<Value, String>;

    /// 列出 L1 动态分类目录（业务域集合，供主 DAG 的 L1 LlmClassifier 动态注入口
    /// `__l1_categories` 使用）。
    ///
    /// 由实现方从路由规则实时提取，保证分类目录与规则联动（新增域规则无需改 DAG）。
    /// 默认实现返回空列表；`DefaultCognitiveRouter` 覆盖实现为
    /// `DomainRouter::list_rules` 的规则目标域 ∪ 全量业务域枚举。
    async fn list_l1_categories(&self) -> Vec<String> {
        Vec::new()
    }

    /// 列出指定业务域下的 L2 动态分类目录（能力簇集合，供主 DAG 的 L2 LlmClassifier
    /// 动态注入口 `__l2_categories` 使用）。
    ///
    /// `domain` 为 L1 路由输出的域字符串。默认实现返回空列表；
    /// `DefaultCognitiveRouter` 覆盖实现为 `ClusterRouter::list_rules` 的规则目标簇
    /// ∪ 静态集群定义（`capability_clusters::clusters_by_domain`）。
    async fn list_l2_categories(&self, _domain: &str) -> Vec<String> {
        Vec::new()
    }

    /// 将进化/补齐产物同步进工作流图谱（L3 `system_workflow_graph_router` 可见）。
    ///
    /// 自进化闭环（通道一/二）在用户同意补齐或进化后调用：把产物工作流登记为
    /// L3 图谱节点，使下一次用户输入的路由决策可命中该产物。默认实现为空操作；
    /// `DefaultCognitiveRouter` 覆写为写入其内部工作流图谱。
    ///
    /// # 参数
    /// - `domain`: 业务域（进化产物统一为 `general`）
    /// - `cluster`: L2 集群（进化产物统一为 `auto_generated`）
    /// - `workflow_id`: 产物工作流 ID（即护照 `capability_id`）
    /// - `display_name`: 图谱节点显示名称
    async fn sync_evolved_workflow(
        &self,
        _domain: &str,
        _cluster: &str,
        _workflow_id: &str,
        _display_name: &str,
    ) {
    }
}

// ── 默认认知路由器实现 ────────────────────────────

/// 默认认知路由器实现
pub struct DefaultCognitiveRouter {
    /// L1 域路由器
    domain_router: Arc<dyn DomainRouter>,
    /// L2 簇路由器
    cluster_router: Arc<dyn ClusterRouter>,
    /// RAR 检索路由器
    rar_router: Arc<dyn RarRouter>,
    /// 工作流图谱（读写锁：路由只读，自进化/能力补齐写入同步）
    workflow_graph: Arc<RwLock<WorkflowGraph>>,
    /// 熔断保护器
    circuit_breaker: RarCircuitBreaker,
    /// 配置
    config: CognitiveRouterConfig,
    /// L2 模型兜底推理器（双层决策三段闭环）。可选项；None 时 L1 退化为纯规则路由。
    llm_reasoner: Option<Arc<LlmReasoner>>,
}

impl DefaultCognitiveRouter {
    /// 创建新的认知路由器
    pub fn new(
        domain_router: Arc<dyn DomainRouter>,
        cluster_router: Arc<dyn ClusterRouter>,
        rar_router: Arc<dyn RarRouter>,
        workflow_graph: Arc<RwLock<WorkflowGraph>>,
    ) -> Self {
        Self {
            domain_router,
            cluster_router,
            rar_router,
            workflow_graph,
            circuit_breaker: RarCircuitBreaker::new(),
            config: CognitiveRouterConfig::default(),
            llm_reasoner: None,
        }
    }

    /// 注入 L2 模型兜底推理器，启用"规则优先→模型兜底→规则复查"双层决策闭环。
    pub fn with_llm_reasoner(mut self, reasoner: Arc<LlmReasoner>) -> Self {
        self.llm_reasoner = Some(reasoner);
        self
    }

    /// L1 域路由（双层决策三段闭环）。
    ///
    /// 注入 `llm_reasoner` 时走 `DomainRouter::decide`，未注入时回退纯规则 `route`，
    /// 保证认知路由始终可用、不强制依赖外部 LLM。
    async fn l1_route(&self, user_input: &str) -> DomainRoutingResult {
        let start = Instant::now();
        match &self.llm_reasoner {
            Some(reasoner) => match self.domain_router.decide(user_input, Some(reasoner)).await {
                DomainDecision::Rule(rules) => {
                    let rule = rules.first().cloned();
                    match rule {
                        Some(r) => DomainRoutingResult::rule_hit(
                            r.target_domain,
                            r,
                            start.elapsed().as_millis() as u64,
                        ),
                        None => DomainRoutingResult::unknown(start.elapsed().as_millis() as u64),
                    }
                },
                DomainDecision::Llm { domain, confidence } => DomainRoutingResult::llm_hit(
                    domain,
                    confidence,
                    start.elapsed().as_millis() as u64,
                ),
                DomainDecision::General => {
                    DomainRoutingResult::unknown(start.elapsed().as_millis() as u64)
                },
            },
            None => self.domain_router.route(user_input).await,
        }
    }

    /// 自定义配置
    pub fn with_config(mut self, config: CognitiveRouterConfig) -> Self {
        self.config = config;
        self
    }

    /// 自定义熔断保护器
    pub fn with_circuit_breaker(mut self, breaker: RarCircuitBreaker) -> Self {
        self.circuit_breaker = breaker;
        self
    }

    /// 设置工作流图谱
    pub fn with_workflow_graph(mut self, graph: Arc<RwLock<WorkflowGraph>>) -> Self {
        self.workflow_graph = graph;
        self
    }

    /// 将进化/补齐产物同步进工作流图谱（L3 `system_workflow_graph_router` 可见）。
    ///
    /// 自进化闭环（通道一/二）用户同意补齐/进化后调用：登记 L3 图谱节点，
    /// 使下一次用户输入的路由决策可命中该产物。
    pub async fn sync_evolved_workflow(
        &self,
        domain: &str,
        cluster: &str,
        workflow_id: &str,
        display_name: &str,
    ) {
        let mut graph = self.workflow_graph.write().await;
        WorkflowGraphSync::sync_workflow(&mut graph, domain, cluster, workflow_id, display_name);
        tracing::info!(
            path = format!("{}/{}/{}", domain, cluster, workflow_id),
            "🗺️ 自进化产物已同步进工作流图谱"
        );
    }

    // ── 物理隔离检查方法 ──────────────────────────

    /// 检查工作流是否在指定作用域内可路由
    pub fn is_workflow_routable_in_scope(
        &self,
        workflow_id: &str,
        tags: &[String],
        scope: RoutingScope,
    ) -> bool {
        // 1. 自指熔断检查
        if let Some(reason) = self.config.self_reference_protection.check(workflow_id, tags) {
            tracing::warn!(workflow_id, reason, "🛡️ 自指熔断触发");
            return false;
        }

        // 2. 作用域隔离检查
        if self.config.enable_scope_isolation {
            match scope {
                RoutingScope::Business => {
                    // 业务作用域：排除系统内部能力
                    let has_system_tag = tags.iter().any(|t| {
                        matches!(t.as_str(), "SYSTEM_ONLY" | "INTERNAL" | "META" | "ORCHESTRATOR")
                    });
                    if has_system_tag {
                        tracing::debug!(workflow_id, "🔒 系统能力，业务作用域不可见");
                        return false;
                    }
                },
                RoutingScope::System => {
                    // 系统作用域：仅路由系统内部能力
                    // 系统作用域可以访问所有能力，但优先系统能力
                },
            }
        }

        true
    }

    /// 过滤可路由的工作流列表
    pub fn filter_routable_workflows(
        &self,
        candidates: &[String],
        scope: RoutingScope,
    ) -> Vec<String> {
        candidates
            .iter()
            .filter(|id| {
                let tags = self.get_workflow_tags(id);
                self.is_workflow_routable_in_scope(id, &tags, scope)
            })
            .cloned()
            .collect()
    }

    /// 获取工作流标签（占位实现，实际从注册表获取）
    fn get_workflow_tags(&self, _workflow_id: &str) -> Vec<String> {
        // TODO: 从工作流注册表获取标签
        // 目前返回空列表，实际使用时需要注入注册表
        Vec::new()
    }

    // ── 系统能力实现 ──────────────────────────────

    /// 系统能力 `system_rar_retriever`：RAR 向量检索
    ///
    /// 输入：`user_input` / `l1_domain` / `l2_cluster`
    /// 输出：`{ count, candidates: [{id,name,description,score,kind,domain,cluster}], raw_count }`
    async fn execute_system_rar_retriever(
        &self,
        input: HashMap<String, Value>,
    ) -> Result<Value, String> {
        let user_input = get_input_str(&input, "user_input");
        let l1_domain = get_input_str(&input, "l1_domain");
        let l2_cluster = get_input_opt_str(&input, "l2_cluster");

        let result = self
            .rar_router
            .search_top_k(&user_input, &l1_domain, l2_cluster.as_deref(), self.config.rar_top_k)
            .await
            // BE-I1 修复：RAR 检索失败返回携带错误码的结构化错误，前端可 i18n。
            .map_err(|e| {
                crate::error_codes::error_json(
                    crate::error_codes::cognitive::RAR_RETRIEVE_FAILED,
                    format!("RAR 检索失败: {e}"),
                )
            })?;

        // 候选对象使用 id 字段（L3 LlmClassifier 动态目录优先取 id）
        let candidates: Vec<Value> = result
            .candidates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.workflow_id,
                    "name": c.name,
                    "description": c.description,
                    "score": c.score,
                    "kind": c.kind.as_str(),
                    "domain": c.domain,
                    "cluster": c.cluster,
                    "agent_profile_id": c.agent_profile_id,
                })
            })
            .collect();

        tracing::info!(
            capability_id = SYSTEM_RAR_RETRIEVER_ID,
            domain = %l1_domain,
            cluster = %l2_cluster.as_deref().unwrap_or(""),
            count = candidates.len(),
            "🧭 系统能力 RAR 检索完成"
        );

        Ok(serde_json::json!({
            "count": candidates.len(),
            "candidates": candidates,
            "raw_count": result.raw_count,
        }))
    }

    /// 统一执行模式决策：按置信度分级（单一权威阈值表）
    ///
    /// 供两处共用，保证行为一致：
    /// - L3 图谱路由（`system_workflow_graph_router`，主 DAG 实际运行路径）
    /// - 程序化路由 `route_with_hint`（非 DAG 调用方）
    ///
    /// 置信度档位与 `cognitive_query` 分支执行消费一致（认知编排器主 DAG 已不设
    /// execution_mode SwitchNode，模式由 L3 图谱路由统一决策、经 EndNode 原样透传）：
    /// - > 0.90 → `Workflow`（精准命中，直发工作流）
    /// - ≥ 0.75 → `Direct`（高置信，直接执行）
    /// - ≥ 0.60 → `Clarify`（模糊命中，Top2 候选交用户澄清）
    /// - ≥ 0.40 → `Plan`（域明确但无高置信工作流，先规划再执行）
    /// - ≥ 0.20 → `Act`（行动模式，交给 agent 执行）
    /// - 其余   → `Delegate`（委派给 agent 处理）
    fn execution_mode_from_confidence(confidence: f64) -> ExecutionMode {
        if confidence > 0.90 {
            ExecutionMode::Workflow
        } else if confidence >= 0.75 {
            ExecutionMode::Direct
        } else if confidence >= 0.60 {
            ExecutionMode::Clarify
        } else if confidence >= 0.40 {
            ExecutionMode::Plan
        } else if confidence >= 0.20 {
            ExecutionMode::Act
        } else {
            ExecutionMode::Delegate
        }
    }

    /// 非工作流能力高置信度保护（P5）：即使置信度达到 Workflow/Direct 阈值，
    /// Agent / Tool / 知识库 / 技能等非 Workflow 能力也必须委派给 agent 执行，
    /// 防止被误当工作流模板直发 `workflow_execute`（触发 WORKFLOW_NOT_FOUND）。
    ///
    /// `kind` 缺失或为空串（如簇级兜底路径无选中候选）时视为 Workflow，保持原行为。
    fn clamp_mode_for_kind(mode: ExecutionMode, kind: &str) -> ExecutionMode {
        if kind.is_empty() || kind == "workflow" {
            return mode;
        }
        match mode {
            ExecutionMode::Workflow | ExecutionMode::Direct | ExecutionMode::ParameterExtract => {
                ExecutionMode::Delegate
            },
            other => other,
        }
    }

    /// 系统能力 `system_workflow_graph_router`：图谱路径规划
    ///
    /// 输入：`selected_capability` / `l1_domain` / `l2_cluster` / `user_input`
    /// 输出：`{ route_path, capability_id, confidence, execution_mode, circuit_broken, reason }`
    ///
    /// 自指熔断：选中能力命中编排器关键字（cognitive_router / orchestrator / system_ 前缀）
    /// 时立即返回 circuit_broken=true，防止路由回系统内部。
    async fn execute_system_workflow_graph_router(
        &self,
        input: HashMap<String, Value>,
    ) -> Result<Value, String> {
        let selected_capability = get_input_str(&input, "selected_capability");
        let l1_domain = get_input_str(&input, "l1_domain");
        let l2_cluster = get_input_str(&input, "l2_cluster");
        let user_input = get_input_str(&input, "user_input");
        // RAR 选中候选的置信度（图谱降级时保留真实分数，替代硬编码 0.5；缺失为 0 表示未提供）
        let selected_score = get_input_f64(&input, "selected_score");
        // RAR 选中候选的能力类型（P5 非 Workflow 能力高置信度保护；空串视为 Workflow）
        let selected_kind = get_input_str(&input, "selected_kind");

        // 1. 自指熔断检查（系统能力层第一道防线）
        let self_ref = selected_capability.contains("cognitive_router")
            || selected_capability.contains("orchestrator")
            || selected_capability.starts_with("system_")
            // 阶段二 T2.4：自指护照用冒号（system:self_evolution:*），进化产物含 /evolution/ / evolution:
            || selected_capability.starts_with("system:")
            || selected_capability.contains("/evolution/")
            || selected_capability.contains("evolution:workflow")
            || selected_capability.contains("self_evolution");
        if self_ref {
            tracing::warn!(
                capability_id = %selected_capability,
                "🛡️ 图谱路由自指熔断：选中能力命中编排器关键字"
            );
            return Ok(serde_json::json!({
                "route_path": "",
                "capability_id": selected_capability,
                "confidence": 0.0,
                "execution_mode": "ask",
                "circuit_broken": true,
                "reason": "self_reference",
            }));
        }

        // 2. 构建当前节点路径（L2 为空时降级为 L1）
        let current_path = if l2_cluster.is_empty() {
            l1_domain.clone()
        } else {
            format!("{}/{}", l1_domain, l2_cluster)
        };

        let candidate_ids = vec![selected_capability.clone()];

        // 3. 图谱路由：优先按关键词匹配，其次按候选 ID 精确匹配
        //    （图谱读写锁：此处只读；guard 在块内释放，结果 owned 传出）
        let graph_result = {
            let graph = self.workflow_graph.read().await;
            if self.config.enable_graph_routing {
                WorkflowGraphRouter::select_best_path(
                    &graph,
                    &current_path,
                    &user_input,
                    &candidate_ids,
                )
                .or_else(|| {
                    WorkflowGraphRouter::select_from_candidates(
                        &graph,
                        &current_path,
                        &candidate_ids,
                    )
                })
            } else {
                None
            }
        };

        // 4. 组装路由结果（fallback_path 提前声明，供输出对象透传观测字段）
        let mut fallback_path = String::new();
        let (route_path, capability_id, confidence) = if let Some(result) = graph_result {
            let cap_id =
                result.selected_path.rsplit('/').next().unwrap_or(&selected_capability).to_string();
            (result.selected_path, cap_id, result.confidence)
        } else {
            // 图谱无匹配：降级为直接构造确定性路径。置信度优先保留 RAR 选中候选的真实
            // 分数（此前硬编码 0.5 丢失候选 score，导致高置信命中被降级为 Plan/Act）；
            // 未提供分数（无候选兜底路径）时回退 0.5。
            fallback_path =
                RoutingDecisionV2::build_path(&l1_domain, &l2_cluster, &selected_capability);
            let fb_conf = if selected_score > 0.0 {
                selected_score
            } else {
                0.5
            };
            (fallback_path.clone(), selected_capability.clone(), fb_conf)
        };

        // 5. 执行模式决策：无能力实体（selected_capability 为空）视为自由问答 → Ask；
        //    否则按置信度分级（统一走公共决策函数，与主 DAG Switch cases 对齐）。
        //    P5 保护：非 Workflow 能力（Agent/Tool/知识库/技能）即使高置信命中也不直发
        //    工作流，强制降级委派 agent 执行，防止触发 WORKFLOW_NOT_FOUND。
        let execution_mode = if selected_capability.trim().is_empty() {
            "ask"
        } else {
            let mode = Self::execution_mode_from_confidence(confidence);
            Self::clamp_mode_for_kind(mode, &selected_kind).as_str()
        };

        tracing::info!(
            capability_id = SYSTEM_WORKFLOW_GRAPH_ROUTER_ID,
            route_path = %route_path,
            selected = %selected_capability,
            confidence,
            "🧭 系统能力图谱路由完成"
        );

        // L3 阶段观测记录（供主 DAG EndNode 透传到 cognitive_query 的 stage_records）
        let stage_records = serde_json::json!([
            {
                "stage": "L3Graph",
                "success": true,
                "confidence": confidence,
                "elapsed_ms": 0,
                "summary": format!("图谱路由: {route_path} (置信度 {confidence:.2})"),
            }
        ]);

        Ok(serde_json::json!({
            "route_path": route_path,
            "capability_id": capability_id,
            "confidence": confidence,
            "execution_mode": execution_mode,
            "circuit_broken": false,
            "reason": "",
            "is_llm_fallback": false,
            "fallback_path": fallback_path,
            "stage_records": stage_records,
        }))
    }
}

#[async_trait]
impl CognitiveRouter for DefaultCognitiveRouter {
    async fn route_with_hint(&self, user_input: &str, mode_hint: ModeHint) -> RoutingDecisionV2 {
        let total_start = Instant::now();
        let mut decision = RoutingDecisionV2::empty();

        // Step 1: L1 域路由（双层决策三段闭环：规则优先→模型兜底→规则复查）
        let l1_start = Instant::now();
        let l1_result = self.l1_route(user_input).await;
        let l1_elapsed = l1_start.elapsed().as_millis() as u64;

        let domain_str = l1_result.domain.as_str().to_string();
        decision.add_record(RouteStageRecord {
            stage: RouteStage::L1Domain,
            success: true,
            confidence: l1_result.confidence,
            elapsed_ms: l1_elapsed,
            summary: format!(
                "域={}, 规则命中={}, LLM兜底={}",
                domain_str,
                l1_result.matched_rule.is_some(),
                l1_result.is_llm_fallback
            ),
        });

        tracing::info!(
            user_input,
            domain = %domain_str,
            l1_confidence = l1_result.confidence,
            "📍 L1 域路由完成"
        );

        // Step 2: L2 簇路由
        let l2_start = Instant::now();
        let l2_result = self.cluster_router.route(user_input, &l1_result).await;
        let l2_elapsed = l2_start.elapsed().as_millis() as u64;

        // 从 ClusterRoutingResult 获取集群 ID
        let cluster_str = l2_result.cluster.map(|c| c.cluster_id.to_string()).unwrap_or_default();

        decision.add_record(RouteStageRecord {
            stage: RouteStage::L2Cluster,
            success: true,
            confidence: l2_result.confidence,
            elapsed_ms: l2_elapsed,
            summary: format!(
                "集群={}, 规则命中={}, 置信度={:.2}",
                cluster_str,
                l2_result.matched_rule.is_some(),
                l2_result.confidence
            ),
        });

        tracing::info!(
            domain = %domain_str,
            cluster = %cluster_str,
            l2_confidence = l2_result.confidence,
            "📂 L2 簇路由完成"
        );

        // Step 3: L3a RAR 检索
        let l3a_start = Instant::now();
        let rar_result = self
            .rar_router
            .search_top_k(user_input, &domain_str, Some(&cluster_str), self.config.rar_top_k)
            .await;
        let l3a_elapsed = l3a_start.elapsed().as_millis() as u64;

        // 处理 RAR 检索结果
        let mut rar_search_result = match rar_result {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(
                    domain = %domain_str,
                    error = %e,
                    "⚠️ RAR 检索失败，降级返回"
                );
                decision.add_record(RouteStageRecord {
                    stage: RouteStage::L3RarRetrieval,
                    success: false,
                    confidence: 0.0,
                    elapsed_ms: l3a_elapsed,
                    summary: format!("RAR 检索失败: {}", e),
                });
                decision.total_elapsed_ms = total_start.elapsed().as_millis() as u64;
                return decision;
            },
        };

        // 记录 L3a RAR 检索阶段
        decision.add_record(RouteStageRecord {
            stage: RouteStage::L3RarRetrieval,
            success: !rar_search_result.candidates.is_empty(),
            confidence: if rar_search_result.candidates.is_empty() {
                0.0
            } else {
                rar_search_result.candidates.first().map(|c| c.score).unwrap_or(0.0)
            },
            elapsed_ms: l3a_elapsed,
            summary: format!(
                "候选数={}, 过滤原因={:?}",
                rar_search_result.candidates.len(),
                rar_search_result
                    .filtered_reasons
                    .iter()
                    .map(|r| format!("{}:{}", r.capability_id, r.reason))
                    .collect::<Vec<_>>()
            ),
        });

        if rar_search_result.candidates.is_empty() {
            // 无候选：降级返回（不属于熔断，不标记 circuit_broken）
            tracing::warn!(
                domain = %domain_str,
                cluster = %cluster_str,
                "⚠️ RAR 检索无候选，降级返回"
            );
            decision.total_elapsed_ms = total_start.elapsed().as_millis() as u64;
            return decision;
        }

        // Step 4: 熔断检查（三重保护）—— P0-1：剔除被阻断候选而非整体 return
        //
        // 修复前：任一候选被阻断即 `mark_circuit_broken + return`，丢弃了其余
        // 合法候选的降级路径。修复后：用 `filter_candidates` 剔除被阻断候选，
        // 仅当全部候选被阻断（无可用降级路径）时才判定熔断失败。
        if self.config.enable_circuit_breaker {
            let (valid_candidates, filtered) = self
                .circuit_breaker
                .filter_candidates(std::mem::take(&mut rar_search_result.candidates));

            if !filtered.is_empty() {
                tracing::warn!(
                    filtered_count = filtered.len(),
                    "🔒 熔断保护器剔除 {} 个候选（保留合法降级路径）",
                    filtered.len()
                );
                rar_search_result.filtered_reasons.extend(filtered);
            }

            if valid_candidates.is_empty() {
                tracing::warn!("🔒 熔断保护器拦截全部候选，无可用路径");
                decision.mark_circuit_broken("全部候选被熔断保护器拦截，无可用路径");
                decision.total_elapsed_ms = total_start.elapsed().as_millis() as u64;
                return decision;
            }

            rar_search_result.candidates = valid_candidates;
        }

        // 获取候选 ID 列表（熔断剔除后重新计算）
        let candidate_ids: Vec<String> =
            rar_search_result.candidates.iter().map(|c| c.workflow_id.clone()).collect();

        // 为图谱路由保留克隆
        let candidate_ids_for_graph = candidate_ids.clone();

        // 保存候选到决策
        decision.candidates = candidate_ids;

        // 保存候选摘要（Clarify 分支 Top2 展示用；过滤后 RarCandidate 列表仍可用）
        decision.candidate_details = rar_search_result
            .candidates
            .iter()
            .map(|c| CandidateSummary {
                capability_id: c.workflow_id.clone(),
                name: c.name.clone(),
                description: c.description.clone(),
                score: c.score,
                kind: c.kind,
                domain: c.domain.clone(),
                cluster: c.cluster.clone(),
                agent_profile_id: c.agent_profile_id.clone(),
            })
            .collect();

        decision.add_record(RouteStageRecord {
            stage: RouteStage::CircuitBreak,
            success: true,
            confidence: 1.0,
            elapsed_ms: 0,
            summary: format!(
                "检查 {} 个候选，剔除 {} 个",
                candidate_ids_for_graph.len() + rar_search_result.filtered_reasons.len(),
                rar_search_result.filtered_reasons.len()
            ),
        });

        // Step 5: L3b 图谱路由（生成路径）—— P0-2：enable_graph_routing 真正生效
        let l3b_start = Instant::now();

        // 构建当前节点路径
        let current_path = if cluster_str.is_empty() {
            domain_str.clone()
        } else {
            format!("{}/{}", domain_str, cluster_str)
        };

        // 尝试使用图谱路由选择最优路径（仅当配置启用；关闭时降级为第一个候选）
        //    （图谱读写锁：此处只读；guard 在块内释放，结果 owned 传出）
        let graph_result = {
            let graph = self.workflow_graph.read().await;
            if self.config.enable_graph_routing {
                WorkflowGraphRouter::select_best_path(
                    &graph,
                    &current_path,
                    user_input,
                    &candidate_ids_for_graph,
                )
            } else {
                None
            }
        };

        let (final_path, capability_id, confidence) = if let Some(graph_decision) = graph_result {
            // 使用图谱路由结果
            let cap_id = graph_decision.selected_path.rsplit('/').next().unwrap_or("").to_string();
            (graph_decision.selected_path.clone(), cap_id, graph_decision.confidence)
        } else {
            // 降级：直接使用第一个候选
            let selected_candidate = rar_search_result.candidates.first();
            if let Some(candidate) = selected_candidate {
                let path = RoutingDecisionV2::build_path(
                    &domain_str,
                    &cluster_str,
                    &candidate.workflow_id,
                );
                (path, candidate.workflow_id.clone(), candidate.score)
            } else {
                (String::new(), String::new(), 0.0)
            }
        };

        let l3b_elapsed = l3b_start.elapsed().as_millis() as u64;

        // 验证路径合法性
        if !final_path.is_empty()
            && let Err(reason) = WorkflowGraphRouter::validate_path(&final_path)
        {
            tracing::warn!(path = %final_path, %reason, "❌ 路径校验失败");
            decision.mark_circuit_broken(reason);
            decision.total_elapsed_ms = total_start.elapsed().as_millis() as u64;
            return decision;
        }

        decision.add_record(RouteStageRecord {
            stage: RouteStage::L3GraphRouting,
            success: !final_path.is_empty(),
            confidence,
            elapsed_ms: l3b_elapsed,
            summary: format!("路径={}, 能力ID={}", final_path, capability_id),
        });

        // 组装最终决策
        decision.route_path = final_path;
        decision.domain = domain_str;
        decision.cluster = cluster_str;
        decision.capability_id = capability_id;
        decision.confidence = confidence;
        decision.is_llm_fallback = l1_result.is_llm_fallback;
        // 决策执行模式：统一走执行模式决策（mode_hint 显式覆盖优先；
        // 无能力实体时视为自由问答 → Ask，否则按置信度分级）
        decision.execution_mode = match mode_hint {
            ModeHint::Ask => ExecutionMode::Ask,
            ModeHint::Plan => ExecutionMode::Plan,
            ModeHint::Act => ExecutionMode::Act,
            ModeHint::Auto => {
                if decision.capability_id.is_empty() {
                    ExecutionMode::Ask
                } else {
                    // P5 保护：按候选能力类型降级（与 L3 图谱路由一致，保证两条路径行为统一）
                    let kind =
                        rar_search_result.candidates.first().map(|c| c.kind.as_str()).unwrap_or("");
                    Self::clamp_mode_for_kind(
                        Self::execution_mode_from_confidence(decision.confidence),
                        kind,
                    )
                }
            },
        };

        // P0-2：max_total_ms 真正生效——超时则降低置信度并标记为降级结果
        let total_elapsed = total_start.elapsed().as_millis() as u64;
        if self.config.max_total_ms > 0 && total_elapsed > self.config.max_total_ms {
            tracing::warn!(
                total_ms = total_elapsed,
                max_total_ms = self.config.max_total_ms,
                "⚠️ 路由总耗时超限，降低置信度"
            );
            // 保持路径但降低置信度（标记结果不可靠，调用方可据此选择重试/降级）
            decision.confidence = decision.confidence.min(0.5);
        }
        decision.total_elapsed_ms = total_elapsed;

        tracing::info!(
            route_path = %decision.route_path,
            confidence = decision.confidence,
            total_ms = decision.total_elapsed_ms,
            "✅ 认知路由完成"
        );

        decision
    }

    async fn route_l1(&self, user_input: &str) -> DomainRoutingResult {
        self.domain_router.route(user_input).await
    }

    // 纯规则 L1 匹配：遍历启用规则做关键词/正则匹配，绝不触发 LLM 兜底。
    // 供主 DAG 外预路由生成 L2 分类目录使用，避免与主 DAG 内 L1 子工作流重复调用 LLM。
    async fn route_l1_rules_only(&self, user_input: &str) -> Option<CapabilityDomain> {
        for rule in self.domain_router.list_rules().await {
            if rule.enabled && rule.matches(user_input) {
                return Some(rule.target_domain);
            }
        }
        None
    }

    // ── 动态分类目录（L1/L2 注入口数据源）──────────────────

    /// 列出 L1 动态分类目录：`DomainRouter` 规则目标域 ∪ 全量业务域枚举。
    ///
    /// 保证新增/调整域路由规则后分类目录实时联动，主 DAG 无需改模板。
    async fn list_l1_categories(&self) -> Vec<String> {
        let mut domains: Vec<String> = Vec::new();
        for rule in self.domain_router.list_rules().await {
            let d = rule.target_domain.as_str().to_string();
            if !domains.contains(&d) {
                domains.push(d);
            }
        }
        // 全量业务域枚举兜底（排除 System 域，保证 LLM 分类候选完整）
        const BUSINESS_DOMAINS: [CapabilityDomain; 8] = [
            CapabilityDomain::General,
            CapabilityDomain::Devops,
            CapabilityDomain::AiMedia,
            CapabilityDomain::DataAnalysis,
            CapabilityDomain::ContentCreation,
            CapabilityDomain::Communication,
            CapabilityDomain::Finance,
            CapabilityDomain::Automation,
        ];
        for d in BUSINESS_DOMAINS {
            let s = d.as_str().to_string();
            if !domains.contains(&s) {
                domains.push(s);
            }
        }
        domains
    }

    /// 列出指定业务域的 L2 动态分类目录：`ClusterRouter` 规则目标簇
    /// ∪ 静态集群定义（`capability_clusters::clusters_by_domain`）。
    ///
    /// `domain` 为 L1 路由输出的域字符串；无法解析时返回空列表。
    async fn list_l2_categories(&self, domain: &str) -> Vec<String> {
        let mut clusters: Vec<String> = Vec::new();
        if let Some(d) = crate::routing_path::parse_domain(domain) {
            for rule in self.cluster_router.list_rules(d).await {
                if !clusters.contains(&rule.target_cluster_id) {
                    clusters.push(rule.target_cluster_id.clone());
                }
            }
            for c in crate::capability_clusters::clusters_by_domain(d) {
                let id = c.cluster_id.to_string();
                if !clusters.contains(&id) {
                    clusters.push(id);
                }
            }
        }
        clusters
    }

    async fn route_l1_l2(&self, user_input: &str) -> (DomainRoutingResult, ClusterRoutingResult) {
        let l1 = self.domain_router.route(user_input).await;
        let l2 = self.cluster_router.route(user_input, &l1).await;
        (l1, l2)
    }

    /// 将进化/补齐产物同步进工作流图谱（L3 `system_workflow_graph_router` 可见）。
    async fn sync_evolved_workflow(
        &self,
        domain: &str,
        cluster: &str,
        workflow_id: &str,
        display_name: &str,
    ) {
        DefaultCognitiveRouter::sync_evolved_workflow(
            self,
            domain,
            cluster,
            workflow_id,
            display_name,
        )
        .await;
    }

    async fn execute_system_capability(
        &self,
        capability_id: &str,
        input: HashMap<String, Value>,
    ) -> Result<Value, String> {
        match capability_id {
            SYSTEM_RAR_RETRIEVER_ID => self.execute_system_rar_retriever(input).await,
            SYSTEM_WORKFLOW_GRAPH_ROUTER_ID => {
                self.execute_system_workflow_graph_router(input).await
            },
            other => Err(format!("未知系统能力: {}", other)),
        }
    }
}

// ── 便捷函数 ──────────────────────────────────────

/// 构建三层路由地址
///
/// # 格式
/// ```text
/// /{domain}/{cluster}/{capability_id}
/// ```
pub fn build_route_path(
    domain: &str,
    cluster: Option<&str>,
    capability_id: Option<&str>,
) -> String {
    match (cluster, capability_id) {
        (Some(c), Some(cap)) => format!("{}/{}/{}", domain, c, cap),
        (Some(c), None) => format!("{}/{}", domain, c),
        (None, _) => domain.to_string(),
    }
}

/// 从路由地址解析各层级
pub fn parse_route_path(path: &str) -> (String, Option<String>, Option<String>) {
    let parts: Vec<&str> = path.split('/').collect();

    let domain = parts.first().unwrap_or(&"").to_string();
    let cluster = parts.get(1).map(|s| s.to_string());
    let capability_id = parts.get(2).map(|s| s.to_string());

    (domain, cluster, capability_id)
}

// ── 系统能力输入提取 ──────────────────────────────

/// 从系统能力输入中提取字符串值（缺失/非字符串时回退为空串）
fn get_input_str(input: &HashMap<String, Value>, key: &str) -> String {
    input.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default()
}

/// 从系统能力输入中提取 f64 数值（缺失/非数值时回退为 0.0）
fn get_input_f64(input: &HashMap<String, Value>, key: &str) -> f64 {
    input.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// 从系统能力输入中提取可选字符串值（缺失/非字符串/空串时返回 None）
fn get_input_opt_str(input: &HashMap<String, Value>, key: &str) -> Option<String> {
    let s = get_input_str(input, key);
    if s.is_empty() { None } else { Some(s) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::CapabilityDomain;
    use crate::capability_clusters::CapabilityCluster;
    use crate::cluster_router::ClusterRoutingRule;
    use crate::domain_router::{DomainRoutingRule, DomainRuleType};
    use crate::rar_router::{RarCandidate, RarError};

    // ── Mock 实现 ──────────────────────────────────

    /// Mock 域路由器
    struct MockDomainRouter {
        result: DomainRoutingResult,
    }

    #[async_trait]
    impl DomainRouter for MockDomainRouter {
        async fn route(&self, _query: &str) -> DomainRoutingResult {
            self.result.clone()
        }

        async fn list_rules(&self) -> Vec<DomainRoutingRule> {
            Vec::new()
        }

        async fn add_rule(&self, _rule: DomainRoutingRule) -> Result<(), String> {
            Ok(())
        }

        async fn update_rule(&self, _rule: DomainRoutingRule) -> Result<(), String> {
            Ok(())
        }

        async fn remove_rule(&self, _rule_id: &str) -> Result<(), String> {
            Ok(())
        }

        async fn get_rule(&self, _rule_id: &str) -> Option<DomainRoutingRule> {
            None
        }

        async fn reorder_rules(&self, _rule_ids: Vec<String>) -> Result<(), String> {
            Ok(())
        }
    }

    /// Mock 簇路由器
    struct MockClusterRouter {
        result: ClusterRoutingResult,
    }

    #[async_trait]
    impl ClusterRouter for MockClusterRouter {
        async fn route(
            &self,
            _query: &str,
            _l1_result: &DomainRoutingResult,
        ) -> ClusterRoutingResult {
            self.result.clone()
        }

        async fn route_from_request(
            &self,
            _request: &crate::capability_router::CapabilityDiscoveryRequest,
            _l1_result: &DomainRoutingResult,
        ) -> ClusterRoutingResult {
            self.result.clone()
        }

        async fn list_rules(&self, _domain: CapabilityDomain) -> Vec<ClusterRoutingRule> {
            Vec::new()
        }

        async fn add_rule(&self, _rule: ClusterRoutingRule) -> Result<(), String> {
            Ok(())
        }

        async fn update_rule(&self, _rule: ClusterRoutingRule) -> Result<(), String> {
            Ok(())
        }

        async fn remove_rule(&self, _rule_id: &str) -> Result<(), String> {
            Ok(())
        }

        async fn get_default_cluster(
            &self,
            _domain: CapabilityDomain,
        ) -> Option<CapabilityCluster> {
            None
        }

        async fn set_default_cluster(
            &self,
            _domain: CapabilityDomain,
            _cluster_id: &str,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    /// Mock RAR 路由器
    struct MockRarRouter {
        candidates: Vec<RarCandidate>,
    }

    #[async_trait]
    impl RarRouter for MockRarRouter {
        async fn search_top_k(
            &self,
            _user_input: &str,
            _domain: &str,
            _cluster: Option<&str>,
            _top_k: usize,
        ) -> Result<crate::rar_router::RarSearchResult, RarError> {
            Ok(crate::rar_router::RarSearchResult {
                candidates: self.candidates.clone(),
                raw_count: self.candidates.len(),
                filtered_reasons: Vec::new(),
                domain: "finance".to_string(),
                cluster: Some("stock_analysis".to_string()),
            })
        }

        fn build_few_shot_prompt(&self, _candidates: &[RarCandidate], _user_input: &str) -> String {
            String::new()
        }
    }

    /// 创建测试用 DomainRoutingResult
    fn create_test_l1_result() -> DomainRoutingResult {
        DomainRoutingResult::rule_hit(
            CapabilityDomain::Finance,
            DomainRoutingRule::new(
                "test",
                "测试",
                CapabilityDomain::Finance,
                DomainRuleType::Keyword,
                ["股票"],
            ),
            10,
        )
    }

    /// 创建测试用 ClusterRoutingResult
    fn create_test_l2_result() -> ClusterRoutingResult {
        // 使用 clusters_by_domain 获取 Finance 域的第一个集群
        let cluster = crate::capability_clusters::clusters_by_domain(CapabilityDomain::Finance)
            .first()
            .copied()
            .unwrap_or_else(|| crate::capability_clusters::all_clusters()[0]);

        ClusterRoutingResult {
            domain: CapabilityDomain::Finance,
            cluster: Some(cluster),
            matched_rule: None,
            is_keyword_derived: false,
            is_fallback: false,
            confidence: 0.95,
            elapsed_ms: 5,
        }
    }

    fn create_test_candidates() -> Vec<RarCandidate> {
        vec![RarCandidate {
            workflow_id: "wf_tech".to_string(),
            name: "技术面分析".to_string(),
            description: "K线、均线".to_string(),
            input_schema: None,
            tags: vec!["stock".to_string()],
            score: 0.92,
            domain: "finance".to_string(),
            cluster: Some("stock_analysis".to_string()),
            negative_scenarios: vec![],
            kind: crate::capability::CapabilityKind::Workflow,
            visibility: crate::capability::Visibility::Public,
            agent_profile_id: None,
        }]
    }

    fn create_test_decision() -> RoutingDecisionV2 {
        RoutingDecisionV2 {
            route_path: "finance/stock_analysis/tech".to_string(),
            domain: "finance".to_string(),
            cluster: "stock_analysis".to_string(),
            capability_id: "wf_tech".to_string(),
            confidence: 0.95,
            is_llm_fallback: false,
            stage_records: Vec::new(),
            total_elapsed_ms: 123,
            circuit_broken: false,
            circuit_break_reason: None,
            fallback_path: None,
            candidates: vec!["wf_tech".to_string(), "wf_fundamental".to_string()],
            candidate_details: vec![CandidateSummary {
                capability_id: "wf_tech".to_string(),
                name: "技术面分析".to_string(),
                description: "K线、均线".to_string(),
                score: 0.95,
                kind: crate::capability::CapabilityKind::Workflow,
                domain: "finance".to_string(),
                cluster: Some("stock_analysis".to_string()),
                agent_profile_id: None,
            }],
            execution_mode: ExecutionMode::Workflow,
            task_shape: None,
        }
    }

    #[test]
    fn test_build_route_path() {
        assert_eq!(build_route_path("finance", Some("stock"), Some("tech")), "finance/stock/tech");
        assert_eq!(build_route_path("finance", Some("stock"), None), "finance/stock");
        assert_eq!(build_route_path("finance", None, None), "finance");
    }

    #[test]
    fn test_parse_route_path() {
        let (domain, cluster, capability) = parse_route_path("finance/stock/tech");
        assert_eq!(domain, "finance");
        assert_eq!(cluster, Some("stock".to_string()));
        assert_eq!(capability, Some("tech".to_string()));

        let (domain, cluster, capability) = parse_route_path("finance/stock");
        assert_eq!(domain, "finance");
        assert_eq!(cluster, Some("stock".to_string()));
        assert_eq!(capability, None);

        let (domain, cluster, capability) = parse_route_path("finance");
        assert_eq!(domain, "finance");
        assert_eq!(cluster, None);
        assert_eq!(capability, None);
    }

    #[test]
    fn test_routing_decision_valid() {
        let decision = create_test_decision();
        assert!(decision.is_valid());
        assert_eq!(decision.route_path, "finance/stock_analysis/tech");
        assert_eq!(decision.domain, "finance");
        assert_eq!(decision.cluster, "stock_analysis");
        assert_eq!(decision.capability_id, "wf_tech");
        assert!(decision.confidence > 0.9);
    }

    #[test]
    fn test_routing_decision_empty() {
        let decision = RoutingDecisionV2::empty();
        assert!(!decision.is_valid());
        assert!(decision.route_path.is_empty());
        assert_eq!(decision.confidence, 0.0);
    }

    #[test]
    fn test_circuit_breaker_mark() {
        let mut decision = create_test_decision();
        assert!(!decision.circuit_broken);

        decision.mark_circuit_broken("系统能力被拦截");
        assert!(decision.circuit_broken);
        assert_eq!(decision.circuit_break_reason, Some("系统能力被拦截".to_string()));
        assert_eq!(decision.confidence, 0.0);
        assert!(!decision.is_valid());
    }

    #[test]
    fn test_stage_record() {
        let record = RouteStageRecord {
            stage: RouteStage::L1Domain,
            success: true,
            confidence: 1.0,
            elapsed_ms: 10,
            summary: "域=finance, 规则命中".to_string(),
        };

        assert_eq!(record.stage.as_str(), "L1_domain");
        assert!(record.success);
        assert_eq!(record.elapsed_ms, 10);
    }

    #[test]
    fn test_cognitive_router_creation() {
        let domain_router = Arc::new(MockDomainRouter { result: create_test_l1_result() });

        let cluster_router = Arc::new(MockClusterRouter { result: create_test_l2_result() });

        let rar_router = Arc::new(MockRarRouter { candidates: create_test_candidates() });

        let workflow_graph = Arc::new(RwLock::new(WorkflowGraph::new()));

        let router =
            DefaultCognitiveRouter::new(domain_router, cluster_router, rar_router, workflow_graph);

        assert!(router.config.enable_l1_rules);
        assert_eq!(router.config.rar_top_k, 5);
    }

    #[tokio::test]
    async fn test_cognitive_router_route() {
        let domain_router = Arc::new(MockDomainRouter { result: create_test_l1_result() });

        let cluster_router = Arc::new(MockClusterRouter { result: create_test_l2_result() });

        let rar_router = Arc::new(MockRarRouter { candidates: create_test_candidates() });

        let workflow_graph = Arc::new(RwLock::new(WorkflowGraph::new()));

        let router =
            DefaultCognitiveRouter::new(domain_router, cluster_router, rar_router, workflow_graph);

        let decision = router.route("分析301302股票").await;

        assert!(decision.is_valid());
        assert_eq!(decision.capability_id, "wf_tech");
        assert!(decision.confidence > 0.9);
        assert!(!decision.circuit_broken);
        assert!(!decision.stage_records.is_empty());

        // 检查各阶段记录
        assert!(decision.stage_records.iter().any(|r| r.stage == RouteStage::L1Domain));
        assert!(decision.stage_records.iter().any(|r| r.stage == RouteStage::L2Cluster));
        assert!(decision.stage_records.iter().any(|r| r.stage == RouteStage::L3RarRetrieval));
        assert!(decision.stage_records.iter().any(|r| r.stage == RouteStage::CircuitBreak));
    }

    #[tokio::test]
    async fn test_cognitive_router_circuit_breaker() {
        let domain_router = Arc::new(MockDomainRouter { result: create_test_l1_result() });

        let cluster_router = Arc::new(MockClusterRouter { result: create_test_l2_result() });

        // 返回系统能力候选（应该被熔断）
        let rar_router = Arc::new(MockRarRouter {
            candidates: vec![RarCandidate {
                workflow_id: "system_cognitive_router".to_string(),
                name: "认知路由器".to_string(),
                description: "系统内部能力".to_string(),
                input_schema: None,
                tags: vec!["system".to_string(), "orchestrator".to_string()],
                score: 0.99,
                domain: "system".to_string(),
                cluster: None,
                negative_scenarios: vec![],
                kind: crate::capability::CapabilityKind::Tool,
                visibility: crate::capability::Visibility::SystemOnly,
                agent_profile_id: None,
            }],
        });

        let workflow_graph = Arc::new(RwLock::new(WorkflowGraph::new()));

        let router =
            DefaultCognitiveRouter::new(domain_router, cluster_router, rar_router, workflow_graph);

        let decision = router.route("测试查询").await;

        // 应该被熔断
        assert!(decision.circuit_broken);
        assert!(!decision.is_valid());
        assert!(decision.circuit_break_reason.is_some());
    }

    #[test]
    fn test_route_stage_as_str() {
        assert_eq!(RouteStage::L1Domain.as_str(), "L1_domain");
        assert_eq!(RouteStage::L2Cluster.as_str(), "L2_cluster");
        assert_eq!(RouteStage::L3RarRetrieval.as_str(), "L3_rar_retrieval");
        assert_eq!(RouteStage::L3GraphRouting.as_str(), "L3_graph_routing");
        assert_eq!(RouteStage::CircuitBreak.as_str(), "circuit_break");
    }

    #[test]
    fn test_cognitive_router_config() {
        let config = CognitiveRouterConfig::default();
        assert!(config.enable_l1_rules);
        assert!(config.enable_l2_rules);
        assert!(config.enable_graph_routing);
        assert!(config.enable_circuit_breaker);
        assert_eq!(config.rar_top_k, 5);
        assert_eq!(config.max_total_ms, 5000);
    }

    #[test]
    fn test_cognitive_router_config_custom() {
        let config = CognitiveRouterConfig {
            enable_l1_rules: false,
            enable_l2_rules: false,
            rar_top_k: 10,
            enable_graph_routing: false,
            enable_circuit_breaker: true,
            max_total_ms: 3000,
            default_scope: RoutingScope::Business,
            self_reference_protection: SelfReferenceProtection::default(),
            enable_scope_isolation: true,
        };

        assert!(!config.enable_l1_rules);
        assert!(!config.enable_l2_rules);
        assert!(!config.enable_graph_routing);
        assert_eq!(config.rar_top_k, 10);
        assert_eq!(config.max_total_ms, 3000);
        assert_eq!(config.default_scope, RoutingScope::Business);
        assert!(config.enable_scope_isolation);
    }

    #[test]
    fn test_decision_add_record() {
        let mut decision = RoutingDecisionV2::empty();
        assert!(decision.stage_records.is_empty());

        decision.add_record(RouteStageRecord {
            stage: RouteStage::L1Domain,
            success: true,
            confidence: 1.0,
            elapsed_ms: 5,
            summary: "test".to_string(),
        });

        assert_eq!(decision.stage_records.len(), 1);
        assert_eq!(decision.stage_records[0].stage, RouteStage::L1Domain);
    }

    #[test]
    fn test_decision_build_path() {
        let path = RoutingDecisionV2::build_path("finance", "stock", "tech");
        assert_eq!(path, "finance/stock/tech");
    }
}
