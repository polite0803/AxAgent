// SPDX-License-Identifier: AGPL-3.0-only
//! RAR 检索增强路由器 — 三层路由树第二层
//!
//! # 架构
//! ```text
//! 用户输入
//!     │
//!     ▼
//! Step 1: 向量检索 Top-K（基于 L1/L2 过滤）
//!     │
//!     ▼
//! Step 2: 候选过滤（visibility + 负向场景 + 自指熔断）
//!     │
//!     ▼
//! Step 3: 动态 Few-shot Prompt 注入（Top-5 候选）
//!     │
//!     ▼
//! Step 4: LLM 选择最终工作流
//! ```
//!
//! # 关键设计
//! - 固定 Top-5 候选 → Prompt 长度恒定（~300 Token），不随工作流总量增长
//! - 三重过滤闸门：visibility 硬性过滤 → 负向场景排除 → 自指熔断保护
//! - 物理隔离：RAR 检索 100% 不返回 SystemOnly 能力

use crate::capability::{CapabilityDomain, CapabilityKind, CapabilityPassportDto, Visibility};
use crate::capability_indexer::CapabilityIndexer;
use crate::capability_retriever::{CapabilityQuery, CapabilityRetriever};
use crate::rag_provider::EmbeddingProvider;
use crate::rar_recaller::{RarRecallResult, RarRecaller, build_rar_prompt};
use crate::routing_path::RoutingPath;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── 数据结构 ──────────────────────────────────────

/// RAR 检索候选 — 向量检索返回的工作流候选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RarCandidate {
    /// 工作流/能力 ID（对应 document_id）
    pub workflow_id: String,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 输入参数 Schema（JSON）
    pub input_schema: Option<serde_json::Value>,
    /// 标签列表
    pub tags: Vec<String>,
    /// 向量相似度得分（0-1）
    pub score: f64,
    /// 所属业务域
    pub domain: String,
    /// 所属集群
    pub cluster: Option<String>,
    /// 负向场景列表（用户明确排除的场景）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub negative_scenarios: Vec<String>,
    /// 能力承载载体类型（工作流/工具/技能/知识库/Agent，用于决策执行模式）
    #[serde(default)]
    pub kind: CapabilityKind,
    /// 能力可见性（熔断判定的权威依据）
    #[serde(default)]
    pub visibility: Visibility,
    /// 推荐执行专家（AgentProfile ID）。认知编排 Agent 执行路径据此自动选择专家。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile_id: Option<String>,
}

impl RarCandidate {
    /// 从能力护照 + 检索结果构建
    pub fn from_passport(passport: &CapabilityPassportDto, score: f64) -> Self {
        Self {
            workflow_id: passport.capability_id.clone(),
            name: passport.name.clone(),
            description: passport.description.clone(),
            input_schema: passport.input_schema.clone(),
            tags: passport.tags.clone(),
            score,
            domain: passport.domain.as_str().to_string(),
            cluster: if passport.sub_category.is_empty() {
                None
            } else {
                Some(passport.sub_category.clone())
            },
            negative_scenarios: passport.negative_scenarios.clone(),
            kind: passport.kind,
            visibility: passport.visibility,
            agent_profile_id: passport.agent_profile_id.clone(),
        }
    }

    /// 是否为系统能力（需要被过滤）— 依据权威 visibility 判定
    pub fn is_system_only(&self) -> bool {
        self.visibility.is_system_only()
    }

    /// 检查是否命中负向场景（用户明确表示不需要）
    ///
    /// # 匹配策略
    /// 负向场景通常是完整句子（如 "此工具不处理 PDF 文件"），直接与用户输入做
    /// 整句 contains 几乎不会命中，导致负向场景形同虚设。这里提取场景描述中的
    /// **核心排除关键词**（PDF、港股、2020 等），用户输入命中任一关键词即排除。
    pub fn matches_negative_scenario(&self, user_input: &str) -> bool {
        let user_lower = user_input.to_lowercase();
        self.negative_scenarios.iter().any(|scenario| {
            extract_negative_keywords(scenario)
                .iter()
                .any(|kw| kw.len() >= 2 && user_lower.contains(&kw.to_lowercase()))
        })
    }
}

/// RAR 检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RarSearchResult {
    /// 候选列表（已通过过滤）
    pub candidates: Vec<RarCandidate>,
    /// 原始候选数量（过滤前）
    pub raw_count: usize,
    /// 被过滤的原因列表
    pub filtered_reasons: Vec<FilteredReason>,
    /// L1 域路由结果
    pub domain: String,
    /// L2 集群路由结果（可选）
    pub cluster: Option<String>,
}

/// 候选被过滤的原因
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredReason {
    pub capability_id: String,
    pub reason: RarFilterReason,
}

/// 过滤原因枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RarFilterReason {
    /// visibility 为 SystemOnly，禁止返回
    SystemOnlyFiltered,
    /// visibility 为 Hidden，禁止返回
    HiddenFiltered,
    /// 命中负向场景（用户明确表示不需要）
    NegativeScenarioMatched,
    /// 自指熔断（包含禁止的 ID 模式或标签）
    SelfReferenceCircuitBreak,
    /// 域不匹配
    DomainMismatch,
    /// 集群不匹配
    ClusterMismatch,
}

impl std::fmt::Display for RarFilterReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RarFilterReason::SystemOnlyFiltered => write!(f, "系统能力禁止返回"),
            RarFilterReason::HiddenFiltered => write!(f, "隐藏能力禁止返回"),
            RarFilterReason::NegativeScenarioMatched => write!(f, "命中负向场景"),
            RarFilterReason::SelfReferenceCircuitBreak => write!(f, "自指熔断触发"),
            RarFilterReason::DomainMismatch => write!(f, "域不匹配"),
            RarFilterReason::ClusterMismatch => write!(f, "集群不匹配"),
        }
    }
}

/// RAR 检索错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RarError {
    /// Embedding 生成失败
    EmbeddingFailed { message: String },
    /// 向量检索失败
    VectorSearchFailed { message: String },
    /// 能力索引访问失败
    IndexAccessFailed { message: String },
    /// L1/L2 路由上下文缺失
    MissingRoutingContext { message: String },
    /// 过滤后无有效候选
    NoValidCandidates { domain: String, cluster: Option<String> },
}

impl std::fmt::Display for RarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RarError::EmbeddingFailed { message } => write!(f, "Embedding 生成失败: {}", message),
            RarError::VectorSearchFailed { message } => write!(f, "向量检索失败: {}", message),
            RarError::IndexAccessFailed { message } => write!(f, "能力索引访问失败: {}", message),
            RarError::MissingRoutingContext { message } => {
                write!(f, "路由上下文缺失: {}", message)
            },
            RarError::NoValidCandidates { domain, cluster } => {
                write!(f, "无有效候选: domain={}, cluster={:?}", domain, cluster)
            },
        }
    }
}

impl std::error::Error for RarError {}

// ── RAR 自指熔断保护器 ──────────────────────────────

/// RAR 自指熔断保护器 — 防止编排器/系统能力被自身路由
///
/// # 工作原理
/// 在最终决策前增加"死刑判决"逻辑，双重保险：
/// 1. ID 模式匹配：检查 capability_id 是否包含禁止模式
/// 2. 标签匹配：检查 tags 是否包含禁止标记
pub struct RarCircuitBreaker {
    /// 绝对禁止的能力 ID 前缀
    forbidden_id_patterns: Vec<String>,
    /// 绝对禁止的标签
    forbidden_tags: Vec<String>,
}

impl RarCircuitBreaker {
    /// 创建默认熔断保护器
    pub fn new() -> Self {
        Self {
            forbidden_id_patterns: vec![
                "router".to_string(),
                "orchestrator".to_string(),
                "cognitive_router".to_string(),
                "system_".to_string(),
            ],
            forbidden_tags: vec![
                "system".to_string(),
                "orchestrator".to_string(),
                "meta".to_string(),
                "circuit_breaker".to_string(),
            ],
        }
    }

    /// 自定义禁止模式
    pub fn with_forbidden_patterns(mut self, patterns: Vec<String>) -> Self {
        self.forbidden_id_patterns = patterns;
        self
    }

    /// 自定义禁止标签
    pub fn with_forbidden_tags(mut self, tags: Vec<String>) -> Self {
        self.forbidden_tags = tags;
        self
    }

    /// 熔断检查：返回 true 表示应该阻断
    pub fn should_block(&self, candidate: &RarCandidate) -> bool {
        // 规则0：权威 visibility 判定（P0-4）—— SystemOnly 直接阻断，
        // 优先于字符串模式匹配，避免黑名单遗漏导致自指路由。
        if candidate.is_system_only() {
            tracing::warn!(
                capability_id = %candidate.workflow_id,
                reason = "system_only_visibility",
                "🔒 RAR 自指熔断触发：系统能力（权威 visibility 判定）"
            );
            return true;
        }

        let id_lower = candidate.workflow_id.to_lowercase();

        // 规则1：ID 包含禁止模式
        for pattern in &self.forbidden_id_patterns {
            if id_lower.contains(pattern) {
                tracing::warn!(
                    capability_id = %candidate.workflow_id,
                    reason = "forbidden_id_pattern",
                    "🔒 RAR 自指熔断触发：ID匹配禁止模式"
                );
                return true;
            }
        }

        // 规则2：标签包含禁止标记
        for tag in &candidate.tags {
            if self.forbidden_tags.contains(&tag.to_lowercase()) {
                tracing::warn!(
                    capability_id = %candidate.workflow_id,
                    tag = %tag,
                    reason = "forbidden_tag",
                    "🔒 RAR 自指熔断触发：标签匹配禁止标记"
                );
                return true;
            }
        }

        false
    }

    /// 批量熔断检查
    pub fn filter_candidates(
        &self,
        candidates: Vec<RarCandidate>,
    ) -> (Vec<RarCandidate>, Vec<FilteredReason>) {
        let mut valid = Vec::new();
        let mut filtered = Vec::new();

        for candidate in candidates {
            if self.should_block(&candidate) {
                filtered.push(FilteredReason {
                    capability_id: candidate.workflow_id,
                    reason: RarFilterReason::SelfReferenceCircuitBreak,
                });
            } else {
                valid.push(candidate);
            }
        }

        (valid, filtered)
    }
}

impl Default for RarCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

// ── RAR 路由器接口 ──────────────────────────────────

/// RAR 路由器 — 检索增强路由接口
#[async_trait]
pub trait RarRouter: Send + Sync {
    /// 在指定域+集群下检索 Top-K 工作流
    ///
    /// # 参数
    /// - `user_input`: 用户原始输入
    /// - `domain`: L1 域路由结果
    /// - `cluster`: L2 集群路由结果（可选）
    /// - `top_k`: 返回候选数量（推荐 3-5）
    async fn search_top_k(
        &self,
        user_input: &str,
        domain: &str,
        cluster: Option<&str>,
        top_k: usize,
    ) -> Result<RarSearchResult, RarError>;

    /// 构建 RAR 动态注入的 Few-shot Prompt
    fn build_few_shot_prompt(&self, candidates: &[RarCandidate], user_input: &str) -> String;
}

/// RAR 路由器默认实现
pub struct DefaultRarRouter {
    /// Embedding 提供者
    embedding_provider: Arc<dyn EmbeddingProvider>,
    /// 能力索引器
    capability_indexer: Arc<dyn CapabilityIndexer>,
    /// 真实向量检索器（可选，注入后 `search_top_k` 走真实向量检索；
    /// 未注入时降级为字符串/标签匹配）
    retriever: Option<Arc<dyn CapabilityRetriever>>,
    /// 自指熔断保护器
    circuit_breaker: RarCircuitBreaker,
    /// 检索集合名称
    collection_name: String,
}

impl DefaultRarRouter {
    /// 创建新的 RAR 路由器
    pub fn new(
        embedding_provider: Arc<dyn EmbeddingProvider>,
        capability_indexer: Arc<dyn CapabilityIndexer>,
    ) -> Self {
        Self {
            embedding_provider,
            capability_indexer,
            retriever: None,
            circuit_breaker: RarCircuitBreaker::new(),
            collection_name: "capabilities".to_string(),
        }
    }

    /// 注入真实向量检索器
    ///
    /// 注入后 `search_top_k` 复用 `CapabilityRetriever`（tools crate 实现）的
    /// 真实向量检索（VectorStore 语义搜索 + FTS + 标签 + 负面排除），
    /// 未注入时降级为基于字符串/标签的相关性评分。
    pub fn with_retriever(mut self, retriever: Arc<dyn CapabilityRetriever>) -> Self {
        self.retriever = Some(retriever);
        self
    }

    /// 自定义集合名称
    pub fn with_collection(mut self, name: impl Into<String>) -> Self {
        self.collection_name = name.into();
        self
    }

    /// 自定义熔断保护器
    pub fn with_circuit_breaker(mut self, breaker: RarCircuitBreaker) -> Self {
        self.circuit_breaker = breaker;
        self
    }

    /// 从能力索引获取指定域+集群下的文档 ID 列表
    async fn get_filtered_doc_ids(&self, domain: &str, cluster: Option<&str>) -> Vec<String> {
        let passports = self.capability_indexer.list_passports().await;

        passports
            .into_iter()
            .filter(|p| {
                // 过滤：仅保留指定域的能力
                if p.domain.as_str() != domain {
                    return false;
                }

                // 过滤：如果指定了集群，检查 sub_category
                if let Some(c) = cluster
                    && p.sub_category != c
                {
                    return false;
                }

                // 过滤：排除 SystemOnly/Hidden 能力（物理隔离第一层）
                if matches!(p.visibility, Visibility::SystemOnly | Visibility::Hidden) {
                    return false;
                }

                true
            })
            .map(|p| p.capability_id)
            .collect()
    }

    /// 全库检索 Top-K 候选（不做域/集群过滤）—— 供 `RarRecaller` 复用（P0-3）
    ///
    /// 与 `search_top_k` 共享同一套相关性评分与排序逻辑，仅去掉 L1/L2 过滤，
    /// 使得 harness 内两套 RAR 实现收敛为单一检索内核。
    async fn search_all(&self, user_input: &str, top_k: usize) -> Vec<RarCandidate> {
        // 预热 embedding（与实际检索路径一致）
        let _ = self.embedding_provider.embed(user_input).await;

        let all_passports = self.capability_indexer.list_passports().await;
        let user_lower = user_input.to_lowercase();

        let mut candidates: Vec<RarCandidate> = all_passports
            .into_iter()
            // 物理隔离：全库检索同样不返回 SystemOnly/Hidden 能力
            .filter(|p| !matches!(p.visibility, Visibility::SystemOnly | Visibility::Hidden))
            .map(|p| {
                let score = compute_relevance_score(&p, &user_lower);
                RarCandidate::from_passport(&p, score)
            })
            .collect();

        candidates
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        candidates.into_iter().take(top_k).collect()
    }
}

// ── RarRecaller 收敛实现（P0-3） ──────────────────

#[async_trait]
impl RarRecaller for DefaultRarRouter {
    async fn recall(&self, user_input: &str, top_k: usize) -> Result<RarRecallResult, String> {
        let candidates = self.search_all(user_input, top_k).await;

        let mut recalled_paths = Vec::new();
        let mut recalled_capabilities = Vec::new();
        let mut similarity_scores = Vec::new();

        for c in candidates {
            // 仅保留能解析出合法业务域的能力（System 域亦被物理隔离排除）
            if let Some(domain) = parse_capability_domain(&c.domain) {
                let cluster_segment = c.cluster.as_deref().unwrap_or("general");
                recalled_paths.push(RoutingPath::from_passport(
                    domain,
                    cluster_segment,
                    &c.workflow_id,
                ));
                recalled_capabilities.push(c.workflow_id);
                similarity_scores.push(c.score);
            }
        }

        let injected_prompt = build_rar_prompt(&recalled_paths, &similarity_scores);

        Ok(RarRecallResult {
            recalled_paths,
            recalled_capabilities,
            similarity_scores,
            injected_prompt,
        })
    }
}

/// 将域字符串解析为 `CapabilityDomain`（兼容历史旧值 core/invest/opc）
fn parse_capability_domain(s: &str) -> Option<CapabilityDomain> {
    s.parse().ok()
}

#[async_trait]
impl RarRouter for DefaultRarRouter {
    async fn search_top_k(
        &self,
        user_input: &str,
        domain: &str,
        cluster: Option<&str>,
        top_k: usize,
    ) -> Result<RarSearchResult, RarError> {
        // 0. 若注入了真实向量检索器，走向量检索（复用 CapabilityRetriever 混合引擎）
        if let Some(retriever) = &self.retriever {
            return self.search_top_k_vector(retriever, user_input, domain, cluster, top_k).await;
        }

        // 1. 生成用户输入的 embedding（预热接口，实际检索在 implementor 层完成）
        let _input_embedding = self
            .embedding_provider
            .embed(user_input)
            .await
            .map_err(|e| RarError::EmbeddingFailed { message: e })?;

        // 2. 获取过滤条件下的文档 ID 列表（域+集群过滤）
        let doc_ids = self.get_filtered_doc_ids(domain, cluster).await;

        if doc_ids.is_empty() {
            tracing::warn!(domain, cluster, "RAR 检索：指定域+集群下无可用能力文档");
            return Ok(RarSearchResult {
                candidates: Vec::new(),
                raw_count: 0,
                filtered_reasons: Vec::new(),
                domain: domain.to_string(),
                cluster: cluster.map(|s| s.to_string()),
            });
        }

        // 3. 获取所有能力护照进行相关性评分
        let all_passports = self.capability_indexer.list_passports().await;

        // 4. 基于标签和描述相关性进行初步筛选与评分
        let user_lower = user_input.to_lowercase();
        let mut candidates: Vec<RarCandidate> = all_passports
            .into_iter()
            .filter(|p| doc_ids.contains(&p.capability_id))
            .map(|p| {
                let score = compute_relevance_score(&p, &user_lower);
                RarCandidate::from_passport(&p, score)
            })
            .collect();

        // 5. 按得分排序
        candidates
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let raw_count = candidates.len();

        // 6. 三重过滤闸门
        let mut filtered_reasons = Vec::new();
        let mut filtered_candidates = Vec::new();

        for candidate in candidates {
            let mut keep = true;

            // 闸门1：visibility 硬性过滤（已在 get_filtered_doc_ids 处理）

            // 闸门2：负向场景检查（用户明确表示不需要的场景）
            if candidate.matches_negative_scenario(user_input) {
                filtered_reasons.push(FilteredReason {
                    capability_id: candidate.workflow_id.clone(),
                    reason: RarFilterReason::NegativeScenarioMatched,
                });
                keep = false;
                tracing::debug!(
                    capability_id = %candidate.workflow_id,
                    "负向场景匹配：用户输入命中负向场景，过滤该候选"
                );
            }

            // 闸门3：自指熔断检查
            if keep && self.circuit_breaker.should_block(&candidate) {
                filtered_reasons.push(FilteredReason {
                    capability_id: candidate.workflow_id.clone(),
                    reason: RarFilterReason::SelfReferenceCircuitBreak,
                });
                keep = false;
            }

            if keep {
                filtered_candidates.push(candidate);
            }
        }

        // 7. 截取 Top-K
        let final_candidates: Vec<RarCandidate> =
            filtered_candidates.into_iter().take(top_k).collect();

        tracing::info!(
            domain,
            cluster,
            top_k,
            raw_count,
            final_count = final_candidates.len(),
            "RAR 检索完成"
        );

        Ok(RarSearchResult {
            candidates: final_candidates,
            raw_count,
            filtered_reasons,
            domain: domain.to_string(),
            cluster: cluster.map(|s| s.to_string()),
        })
    }

    fn build_few_shot_prompt(&self, candidates: &[RarCandidate], user_input: &str) -> String {
        build_rar_few_shot_prompt(candidates, user_input)
    }
}

// ── 真实向量检索分支（inherent impl） ──────────────

impl DefaultRarRouter {
    /// 真实向量检索分支 — 复用 `CapabilityRetriever` 混合检索引擎
    ///
    /// 与字符串匹配分支的区别：用 `CapabilityRetriever.retrieve` 做语义向量检索
    /// （VectorStore 相似度 + FTS + 标签硬匹配 + 负面排除），替代 compute_relevance_score
    /// 的简化字符串/标签匹配。域过滤走 `domain_filter`，集群过滤在候选层按 sub_category 完成。
    async fn search_top_k_vector(
        &self,
        retriever: &Arc<dyn CapabilityRetriever>,
        user_input: &str,
        domain: &str,
        cluster: Option<&str>,
        top_k: usize,
    ) -> Result<RarSearchResult, RarError> {
        // 考虑集群过滤带来的损耗，检索量取 max(top_k, 20) 以确保最终候选充足
        let retrieve_k = top_k.max(20);

        // 域过滤：仅检索业务域（System 域不参与用户 RAR）
        let domain_filter = match parse_capability_domain(domain) {
            Some(d) if !d.is_system() => Some(vec![d]),
            _ => None,
        };

        let query = CapabilityQuery {
            user_input: user_input.to_string(),
            top_k: retrieve_k,
            domain_filter,
            ..Default::default()
        };

        let result = retriever
            .retrieve(&query)
            .await
            .map_err(|e| RarError::VectorSearchFailed { message: e })?;

        // 映射为 RAR 候选 + 集群过滤（sub_category 与 L2 集群对齐）
        let mut candidates: Vec<RarCandidate> = result
            .candidates
            .into_iter()
            .filter(|c| {
                if let Some(cl) = cluster {
                    c.passport.sub_category == cl
                } else {
                    true
                }
            })
            .map(|c| RarCandidate::from_passport(&c.passport, c.retrieval_score))
            .collect();

        let raw_count = candidates.len();

        // 三重过滤闸门：负向场景排除 + 自指熔断（visibility 权威判定已在熔断器内）
        let mut filtered_reasons = Vec::new();
        let mut filtered_candidates = Vec::new();

        for candidate in candidates.drain(..) {
            let mut keep = true;

            // 闸门2：负向场景检查
            if candidate.matches_negative_scenario(user_input) {
                filtered_reasons.push(FilteredReason {
                    capability_id: candidate.workflow_id.clone(),
                    reason: RarFilterReason::NegativeScenarioMatched,
                });
                keep = false;
            }

            // 闸门3：自指熔断检查
            if keep && self.circuit_breaker.should_block(&candidate) {
                filtered_reasons.push(FilteredReason {
                    capability_id: candidate.workflow_id.clone(),
                    reason: RarFilterReason::SelfReferenceCircuitBreak,
                });
                keep = false;
            }

            if keep {
                filtered_candidates.push(candidate);
            }
        }

        // 按综合检索分降序排序并截取 Top-K
        filtered_candidates
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let final_candidates: Vec<RarCandidate> =
            filtered_candidates.into_iter().take(top_k).collect();

        tracing::info!(
            domain,
            cluster,
            top_k,
            raw_count,
            final_count = final_candidates.len(),
            "RAR 向量检索完成"
        );

        Ok(RarSearchResult {
            candidates: final_candidates,
            raw_count,
            filtered_reasons,
            domain: domain.to_string(),
            cluster: cluster.map(|s| s.to_string()),
        })
    }
}

// ── 辅助函数 ──────────────────────────────────────

/// 从负向场景描述中提取核心排除关键词（P1-7）
///
/// 负向场景通常是完整句子（如 "此工具不处理 PDF 文件"），直接整句做 contains
/// 几乎不会命中，导致负向场景形同虚设。这里剥离否定词/停用词后提取实词：
/// - 英文/数字按连续字母数字块提取（如 `PDF`、`2020`）
/// - 中文按停用词切分剥离泛化词（如 "不"、"工具"、"处理"），保留剩余实词（如 "港股"、"文件"）
fn extract_negative_keywords(scenario: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "此",
        "该",
        "本",
        "的",
        "了",
        "和",
        "与",
        "及",
        "或",
        "是",
        "在",
        "对",
        "有",
        "会",
        "可以",
        "能够",
        "提供",
        "支持",
        "处理",
        "针对",
        "用于",
        "用来",
        "进行",
        "作为",
        "不",
        "无",
        "未",
        "没",
        "非",
        "不能",
        "无法",
        "不会",
        "不支持",
        "不处理",
        "不要",
        "用户",
        "工具",
        "能力",
        "功能",
        "工作流",
        "这个",
        "那个",
        "一些",
        "所有",
        "任务",
        "场景",
        "情况",
        "适用",
        "除外",
        "排除",
    ];

    /// 将中文片段按停用词切分，剥离否定词/泛化词，保留实词子串
    fn split_on_stopwords(seg: &str) -> Vec<String> {
        let chars: Vec<char> = seg.chars().collect();
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut i = 0;
        while i < chars.len() {
            // 贪心匹配最长停用词（避免 "不支持" 只匹配到 "不"）
            let mut matched_len: usize = 0;
            for sw in STOPWORDS {
                let sw_chars: Vec<char> = sw.chars().collect();
                if i + sw_chars.len() <= chars.len()
                    && chars[i..i + sw_chars.len()] == sw_chars[..]
                    && sw_chars.len() > matched_len
                {
                    matched_len = sw_chars.len();
                }
            }
            if matched_len > 0 {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
                i += matched_len;
            } else {
                current.push(chars[i]);
                i += 1;
            }
        }
        if !current.is_empty() {
            parts.push(current);
        }
        parts
    }

    scenario
        // 按分隔符切成片段（中英文混合）
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    ',' | '，'
                        | '。'
                        | '、'
                        | ';'
                        | '；'
                        | '('
                        | ')'
                        | '（'
                        | '）'
                        | '/'
                        | '-'
                        | ':'
                        | '！'
                        | '？'
                )
        })
        .flat_map(|seg| {
            if seg.is_empty() {
                return Vec::new();
            }
            // 分离连续 ASCII 词（英文/数字）与中文片段
            let mut parts = Vec::new();
            let mut ascii = String::new();
            let mut cjk = String::new();
            for c in seg.chars() {
                if c.is_ascii() {
                    if !cjk.is_empty() {
                        parts.push(std::mem::take(&mut cjk));
                    }
                    if c.is_ascii_alphanumeric() {
                        ascii.push(c);
                    } else if !ascii.is_empty() {
                        parts.push(std::mem::take(&mut ascii));
                    }
                } else if !ascii.is_empty() {
                    parts.push(std::mem::take(&mut ascii));
                    cjk.push(c);
                } else {
                    cjk.push(c);
                }
            }
            if !ascii.is_empty() {
                parts.push(ascii);
            }
            if !cjk.is_empty() {
                parts.push(cjk);
            }
            // 中文片段再按停用词切分
            let mut words = Vec::new();
            for part in parts {
                if part.is_ascii() {
                    words.push(part);
                } else {
                    words.extend(split_on_stopwords(&part));
                }
            }
            words
        })
        .map(|kw| kw.trim().to_lowercase())
        .filter(|kw| kw.len() >= 2)
        .filter(|kw| !STOPWORDS.iter().any(|s| kw == *s))
        .collect()
}

/// 中英文标签映射表 — 支持中文用户输入与英文标签的匹配
fn get_chinese_tag_mapping(tag: &str) -> Option<&'static str> {
    let mappings: &[(&str, &str)] = &[
        ("stock", "股票"),
        ("tech", "技术"),
        ("fund", "基金"),
        ("market", "市场"),
        ("trade", "交易"),
        ("invest", "投资"),
        ("analysis", "分析"),
        ("data", "数据"),
        ("chart", "图表"),
        ("report", "报表"),
        ("deploy", "部署"),
        ("monitor", "监控"),
        ("image", "图像"),
        ("video", "视频"),
        ("audio", "音频"),
        ("email", "邮件"),
        ("message", "消息"),
        ("system", "系统"),
        ("file", "文件"),
        ("config", "配置"),
        ("search", "搜索"),
        ("summary", "摘要"),
        ("translate", "翻译"),
        ("write", "写作"),
        ("design", "设计"),
        ("automation", "自动化"),
        ("schedule", "定时"),
        ("customer", "客户"),
        ("order", "订单"),
        ("product", "产品"),
        ("inventory", "库存"),
        ("risk", "风险"),
        ("portfolio", "组合"),
        ("news", "新闻"),
        ("sentiment", "情绪"),
        ("backtest", "回测"),
        ("strategy", "策略"),
        ("forecast", "预测"),
        ("crypto", "加密货币"),
        ("bond", "债券"),
        ("currency", "汇率"),
        ("login", "登录"),
        ("auth", "认证"),
        ("permission", "权限"),
        ("security", "安全"),
        ("log", "日志"),
        ("database", "数据库"),
        ("query", "查询"),
        ("pipeline", "流水线"),
        ("sync", "同步"),
        ("backup", "备份"),
        ("network", "网络"),
        ("cloud", "云"),
        ("test", "测试"),
        ("debug", "调试"),
        ("refactor", "重构"),
        ("document", "文档"),
        ("meeting", "会议"),
        ("calendar", "日历"),
        ("task", "任务"),
        ("project", "项目"),
    ];
    mappings.iter().find(|(en, _)| *en == tag).map(|(_, zh)| *zh)
}

/// 中文标签 → 英文关键词反向映射（P1-6）
///
/// 当护照标签为中文（如 "股票"）而用户输入为英文（如 "stock"）时，
/// 需要反向映射才能命中。返回所有等价的英文关键词。
fn get_english_tag_mapping(zh: &str) -> Option<&'static [&'static str]> {
    let mappings: &[(&str, &[&str])] = &[
        ("股票", &["stock", "equity", "share"]),
        ("技术", &["tech", "technical"]),
        ("基金", &["fund"]),
        ("市场", &["market"]),
        ("交易", &["trade", "trading"]),
        ("投资", &["invest", "investment"]),
        ("分析", &["analysis", "analyze"]),
        ("数据", &["data"]),
        ("图表", &["chart", "graph"]),
        ("报表", &["report"]),
        ("部署", &["deploy", "deployment"]),
        ("监控", &["monitor", "monitoring"]),
        ("图像", &["image", "picture"]),
        ("视频", &["video"]),
        ("音频", &["audio", "voice"]),
        ("邮件", &["email", "mail"]),
        ("消息", &["message", "msg"]),
        ("系统", &["system"]),
        ("文件", &["file"]),
        ("配置", &["config", "configuration"]),
        ("搜索", &["search"]),
        ("摘要", &["summary", "summarize"]),
        ("翻译", &["translate", "translation"]),
        ("写作", &["write", "writing"]),
        ("设计", &["design"]),
        ("自动化", &["automation", "auto"]),
        ("定时", &["schedule", "scheduled"]),
        ("客户", &["customer", "client"]),
        ("订单", &["order"]),
        ("产品", &["product"]),
        ("库存", &["inventory", "stock_level"]),
        ("风险", &["risk"]),
        ("组合", &["portfolio"]),
        ("新闻", &["news"]),
        ("情绪", &["sentiment", "emotion"]),
        ("回测", &["backtest", "back_test"]),
        ("策略", &["strategy"]),
        ("预测", &["forecast", "predict"]),
        ("加密货币", &["crypto", "cryptocurrency", "bitcoin", "btc"]),
        ("债券", &["bond"]),
        ("汇率", &["currency", "exchange_rate", "forex"]),
        ("登录", &["login", "signin"]),
        ("认证", &["auth", "authentication"]),
        ("权限", &["permission", "acl"]),
        ("安全", &["security", "secure"]),
        ("日志", &["log", "logging"]),
        ("数据库", &["database", "db"]),
        ("查询", &["query", "search"]),
        ("流水线", &["pipeline"]),
        ("同步", &["sync", "synchronize"]),
        ("备份", &["backup"]),
        ("网络", &["network", "net"]),
        ("云", &["cloud"]),
        ("测试", &["test", "testing"]),
        ("调试", &["debug"]),
        ("重构", &["refactor", "refactoring"]),
        ("文档", &["document", "doc"]),
        ("会议", &["meeting"]),
        ("日历", &["calendar"]),
        ("任务", &["task", "todo"]),
        ("项目", &["project"]),
    ];
    mappings.iter().find(|(zh_tag, _)| *zh_tag == zh).map(|(_, en)| *en)
}

/// 获取标签的全部中英文别名（用于双向匹配，P1-6）
fn get_tag_aliases(tag: &str) -> Vec<&'static str> {
    let mut aliases: Vec<&'static str> = Vec::new();
    // 英文标签 → 中文别名
    if let Some(zh) = get_chinese_tag_mapping(tag) {
        aliases.push(zh);
    }
    // 中文标签 → 英文别名
    if let Some(en_list) = get_english_tag_mapping(tag) {
        aliases.extend_from_slice(en_list);
    }
    aliases
}

/// 计算能力与用户输入的相关性得分（简化版）
///
/// 实际生产中应使用向量余弦相似度，此处提供基于标签匹配的简化实现。
pub fn compute_relevance_score(passport: &CapabilityPassportDto, user_lower: &str) -> f64 {
    let mut score = 0.0;

    // 标签匹配（高权重）—— 双向中英映射（P1-6）
    for tag in &passport.tags {
        let tag_lower = tag.to_lowercase();

        // 直接匹配：用户输入包含标签
        if user_lower.contains(&tag_lower) {
            score += 0.3;
            continue;
        }

        // 双向映射匹配：通过别名表查找标签的中英文等价词
        for alias in get_tag_aliases(&tag_lower) {
            if user_lower.contains(&alias.to_lowercase()) {
                score += 0.25; // 映射匹配权重略低
                break;
            }
        }
    }

    // 描述关键词匹配
    let desc_lower = passport.description.to_lowercase();
    let words: Vec<&str> = user_lower.split_whitespace().collect();
    let word_match_count = words.iter().filter(|w| desc_lower.contains(**w)).count();
    score += (word_match_count as f64 / words.len().max(1) as f64) * 0.4;

    // 名称匹配（支持子串匹配）
    let name_lower = passport.name.to_lowercase();
    if user_lower.contains(&name_lower) || name_lower.contains(user_lower) {
        score += 0.3;
    }

    // 归一化到 0-1
    score.min(1.0)
}

/// 构建 RAR 动态注入的 Few-shot Prompt（5选1 选择题格式）
///
/// # 格式
/// ```text
/// 你是一个工作流路由器。从以下候选项中选择最合适的一个。
///
/// ## 候选项（必须且只能选择一个）
/// 1. wf_stock_tech — 技术面分析（K线、均线、MACD）
///    输入参数：stock_code, period
/// ...
///
/// ## 用户输入
/// {user_input}
///
/// 要求：必须且只能从上述候选项中选择一个最匹配的工作流。输出 JSON 格式。
/// ```
pub fn build_rar_few_shot_prompt(candidates: &[RarCandidate], user_input: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str("你是一个工作流路由器。从以下候选项中选择最合适的一个。\n\n");
    prompt.push_str("## 候选项（必须且只能选择一个）\n");

    for (i, c) in candidates.iter().enumerate() {
        let idx = i + 1;
        prompt.push_str(&format!(
            "{}. {} — {}（相似得分 {:.2}）\n",
            idx, c.name, c.description, c.score
        ));

        // 提取输入参数
        if let Some(schema) = &c.input_schema
            && let Some(params) = extract_input_params(schema)
        {
            prompt.push_str(&format!("   输入参数：{}\n", params.join(", ")));
        }
    }

    prompt.push_str("\n## 用户输入\n");
    prompt.push_str(user_input);
    prompt.push_str("\n\n要求：必须且只能从上述候选项中选择一个最匹配的工作流。输出 JSON 格式。");

    prompt
}

/// 从 JSON Schema 提取输入参数名列表（按字母顺序排序确保稳定性）
fn extract_input_params(schema: &serde_json::Value) -> Option<Vec<String>> {
    schema.get("properties").and_then(|p| p.as_object()).map(|m| {
        let mut params: Vec<String> = m.keys().map(|k| k.as_str().to_string()).collect();
        params.sort(); // 排序确保输出顺序稳定
        params
    })
}

/// 根据域和集群获取默认 Top-K 数量
pub fn default_top_k_for_cluster(_domain: &str, has_cluster: bool) -> usize {
    if has_cluster { 5 } else { 8 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rar_circuit_breaker() {
        let breaker = RarCircuitBreaker::new();

        // 应该被阻断的候选
        let system_candidate = RarCandidate {
            workflow_id: "system_cognitive_router".to_string(),
            name: "认知路由编排器".to_string(),
            description: "系统内部能力".to_string(),
            input_schema: None,
            tags: vec!["system".to_string(), "orchestrator".to_string()],
            score: 0.95,
            domain: "system".to_string(),
            cluster: None,
            negative_scenarios: vec![],
            kind: CapabilityKind::Tool,
            visibility: Visibility::Public,
            agent_profile_id: None,
        };

        assert!(breaker.should_block(&system_candidate), "system_ 前缀应该被熔断");

        let orchestrator_candidate = RarCandidate {
            workflow_id: "workflow_orchestrator".to_string(),
            name: "工作流编排器".to_string(),
            description: "系统内部编排器".to_string(),
            input_schema: None,
            tags: vec!["workflow".to_string()],
            score: 0.9,
            domain: "system".to_string(),
            cluster: None,
            negative_scenarios: vec![],
            kind: CapabilityKind::Workflow,
            visibility: Visibility::Public,
            agent_profile_id: None,
        };

        assert!(breaker.should_block(&orchestrator_candidate), "orchestrator 标签应该被熔断");

        // 正常业务能力不应该被阻断
        let normal_candidate = RarCandidate {
            workflow_id: "wf_stock_tech".to_string(),
            name: "股票技术分析".to_string(),
            description: "分析股票技术面".to_string(),
            input_schema: None,
            tags: vec!["finance".to_string(), "stock".to_string()],
            score: 0.92,
            domain: "finance".to_string(),
            cluster: Some("stock_analysis".to_string()),
            negative_scenarios: vec![],
            kind: CapabilityKind::Tool,
            visibility: Visibility::Public,
            agent_profile_id: None,
        };

        assert!(!breaker.should_block(&normal_candidate), "正常业务能力不应该被熔断");
    }

    #[test]
    fn test_build_rar_few_shot_prompt() {
        let candidates = vec![RarCandidate {
            workflow_id: "wf_tech".to_string(),
            name: "技术面分析".to_string(),
            description: "K线、均线、MACD".to_string(),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "stock_code": {"type": "string"},
                    "period": {"type": "string"}
                }
            })),
            tags: vec!["tech".to_string()],
            score: 0.95,
            domain: "finance".to_string(),
            cluster: None,
            negative_scenarios: vec![],
            kind: CapabilityKind::Workflow,
            visibility: Visibility::Public,
            agent_profile_id: None,
        }];

        let prompt = build_rar_few_shot_prompt(&candidates, "分析301302");

        assert!(prompt.contains("技术面分析"));
        assert!(prompt.contains("K线、均线、MACD"));
        // 参数按字母顺序排序：period 在前，stock_code 在后
        assert!(prompt.contains("period, stock_code"));
        assert!(prompt.contains("分析301302"));
    }

    #[test]
    fn test_extract_input_params() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "stock_code": {"type": "string"},
                "period": {"type": "string"}
            }
        });

        let params = extract_input_params(&schema).unwrap();
        assert!(params.contains(&"stock_code".to_string()));
        assert!(params.contains(&"period".to_string()));
    }

    #[test]
    fn test_filter_candidates() {
        let breaker = RarCircuitBreaker::new();

        let candidates = vec![
            RarCandidate {
                workflow_id: "wf_normal".to_string(),
                name: "正常工作流".to_string(),
                description: "测试".to_string(),
                input_schema: None,
                tags: vec!["normal".to_string()],
                score: 0.9,
                domain: "test".to_string(),
                cluster: None,
                negative_scenarios: vec![],
                kind: CapabilityKind::Workflow,
                visibility: Visibility::Public,
                agent_profile_id: None,
            },
            RarCandidate {
                workflow_id: "system_router".to_string(),
                name: "系统路由器".to_string(),
                description: "测试".to_string(),
                input_schema: None,
                tags: vec!["system".to_string()],
                score: 0.95,
                domain: "system".to_string(),
                cluster: None,
                negative_scenarios: vec![],
                kind: CapabilityKind::Tool,
                visibility: Visibility::Public,
                agent_profile_id: None,
            },
        ];

        let (valid, filtered) = breaker.filter_candidates(candidates);

        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].workflow_id, "wf_normal");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].capability_id, "system_router");
    }

    #[test]
    fn test_compute_relevance_score() {
        let passport = CapabilityPassportDto {
            capability_id: "test".to_string(),
            name: "股票技术分析".to_string(),
            description: "分析股票的技术面走势".to_string(),
            kind: crate::capability::CapabilityKind::Tool,
            domain: crate::capability::CapabilityDomain::Finance,
            sub_category: "stock_analysis".to_string(),
            visibility: Visibility::Public,
            caller_permissions: crate::capability::CallerPermissions::new(),
            input_schema: None,
            tags: vec!["stock".to_string(), "tech".to_string()],
            negative_scenarios: vec![],
            security_level: crate::capability::SecurityLevel::Public,
            modality_support: crate::capability::ModalitySupport::default(),
            output_capabilities: crate::capability::OutputCapabilities::default(),
            estimated_cost_usd: None,
            avg_duration_seconds: None,
            planning_complexity: crate::capability::PlanningComplexity::Simple,
            model_iq_requirement: 0,
            experiment_group: None,
            agent_profile_id: None,
            stats: crate::capability::CapabilityStats::default(),
            level: crate::capability::CapabilityLevel::L1,
            enabled: true,
        };

        // 完全匹配
        let score = compute_relevance_score(&passport, "分析股票");
        assert!(score > 0.5, "完全匹配得分应该较高");

        // 不匹配
        let score = compute_relevance_score(&passport, "预订酒店机票");
        assert!(score < 0.3, "不匹配得分应该较低");
    }
}
