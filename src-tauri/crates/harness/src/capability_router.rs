// SPDX-License-Identifier: AGPL-3.0-only
//! 能力路由器 — 完整编排管线（用户输入 → 能力发现 → 最终输出）
//!
//! # 管线流程
//! 用户输入
//! → L1 多模态预处理器（抽文本、抽图像特征）  ← 在 agent 层实现
//! → L2 混合检索引擎（语义+标签，召回 Top20） ← CapabilityRetriever
//! → L3 8维过滤闸门（硬性剔除）             ← CapabilityFilter
//! → L4 智能排序器（动态加权）              ← CapabilityRanker
//! → L5 结果校验器（入参匹配检查）          ← Router 内置
//! → 主动式增强（补全 / 熔断）              ← Completer / CircuitBreaker

use crate::capability::{CapabilityPassportDto, DiscoveryWeights, SessionBudget};
use crate::capability_circuit::{CapabilityCircuitBreaker, CapabilityCompleter};
use crate::capability_clusters::derive_cluster_for_passport;
use crate::capability_filter::{CapabilityFilter, FilterContext};
use crate::capability_ranker::{CapabilityRanker, RankedCapability, RankingResult};
use crate::capability_retriever::{CapabilityQuery, CapabilityRetriever};
use crate::rar_recaller::{RarRecallResult, RarRecaller};
use crate::routing_path::RoutingPath;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── 编排请求 ──────────────────────────────────────

/// RAR 默认召回数量
fn default_rar_top_k() -> usize {
    5
}

/// 能力发现请求（从用户输入开始的完整管线）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDiscoveryRequest {
    /// 用户原始输入
    pub user_input: String,
    /// 过滤上下文
    pub filter_context: FilterContext,
    /// 检索配置
    pub query: CapabilityQuery,
    /// 用户偏好权重
    pub weights: DiscoveryWeights,
    /// 会话预算
    pub budget: SessionBudget,
    /// 是否启用补全建议
    pub enable_completion: bool,
    /// 是否启用熔断降级
    pub enable_circuit_breaker: bool,
    /// 是否启用 RAR(检索增强路由,软引导能力召回)
    #[serde(default)]
    pub enable_rar: bool,
    /// RAR 召回数量(默认 5)
    #[serde(default = "default_rar_top_k")]
    pub rar_top_k: usize,
    /// P0: 任务形态决策（原则三标尺输出，由 TaskShapeClassifier 在路由前产出）。
    /// `None` 表示未启用 UNITY_P0_TASK_SHAPE flag，走旧链路。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_shape: Option<crate::task_shape::TaskShapeDecision>,
}

impl Default for CapabilityDiscoveryRequest {
    fn default() -> Self {
        Self {
            user_input: String::default(),
            filter_context: FilterContext::default(),
            query: CapabilityQuery::default(),
            weights: DiscoveryWeights::default(),
            budget: SessionBudget::default(),
            enable_completion: false,
            enable_circuit_breaker: false,
            enable_rar: false,
            rar_top_k: default_rar_top_k(),
            task_shape: None,
        }
    }
}

// ── 编排结果 ──────────────────────────────────────

/// 能力发现最终结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDiscoveryResult {
    /// 命中的主能力（可能为 None，表示未匹配到）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_match: Option<RankedCapability>,
    /// 备选能力列表
    #[serde(default)]
    pub alternatives: Vec<RankedCapability>,
    /// 是否触发模糊发现
    pub ambiguous: bool,
    /// 模糊发现时的澄清问题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarification_prompt: Option<String>,
    /// 补全建议列表
    #[serde(default)]
    pub suggestions: Vec<crate::capability_circuit::CapabilitySuggestion>,
    /// 熔断降级信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circuit_info: Option<String>,
    /// 管线总耗时（毫秒）
    pub total_elapsed_ms: u64,
    /// 各阶段耗时明细
    #[serde(default)]
    pub phase_timings: Vec<PhaseTiming>,
    /// 命中主能力的路径地址(L3 输出,由护照元数据推导)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_path: Option<RoutingPath>,
    /// 备选能力的路径地址列表(与 alternatives 一一对应)
    #[serde(default)]
    pub alternative_paths: Vec<RoutingPath>,
    /// RAR 召回结果(若启用且 recaller 已注入)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rar_result: Option<RarRecallResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseTiming {
    pub phase: String,
    pub elapsed_ms: u64,
}

// ── 路由器 trait ──────────────────────────────────

/// 能力路由器 — 编排完整的发现管线
#[async_trait]
pub trait CapabilityRouter: Send + Sync {
    /// 执行完整的能力发现管线
    async fn discover(
        &self,
        request: &CapabilityDiscoveryRequest,
    ) -> Result<CapabilityDiscoveryResult, String>;

    /// 从 Top-N 候选中选取最终命中（含降级逻辑）
    async fn resolve_final_match(
        &self,
        ranked: &RankingResult,
        request: &CapabilityDiscoveryRequest,
    ) -> Option<RankedCapability>;

    /// 入参校验器（L5）
    ///
    /// 检查 Top1 的入参是否完全匹配当前提取的实体，
    /// 若不匹配则自动降级到 Top2
    fn validate_input_match(&self, _capability: &CapabilityPassportDto, _user_input: &str) -> bool {
        // 实际实现在 tools crate 中完成
        true
    }
}

// ── 默认管线实现 ──────────────────────────────────

/// 默认能力路由器（组合 Retriever + Filter + Ranker + Completer + CircuitBreaker + RarRecaller）
pub struct DefaultCapabilityRouter {
    pub retriever: Arc<dyn CapabilityRetriever>,
    pub filter: Arc<dyn CapabilityFilter>,
    pub ranker: Arc<dyn CapabilityRanker>,
    pub completer: Option<Arc<dyn CapabilityCompleter>>,
    pub circuit_breaker: Option<Arc<dyn CapabilityCircuitBreaker>>,
    /// RAR 召回器(可选,启用后 discover 在 enable_rar=true 时调用)
    pub rar_recaller: Option<Arc<dyn RarRecaller>>,
}

impl DefaultCapabilityRouter {
    pub fn new(
        retriever: Arc<dyn CapabilityRetriever>,
        filter: Arc<dyn CapabilityFilter>,
        ranker: Arc<dyn CapabilityRanker>,
    ) -> Self {
        Self {
            retriever,
            filter,
            ranker,
            completer: None,
            circuit_breaker: None,
            rar_recaller: None,
        }
    }

    pub fn with_completer(mut self, completer: Arc<dyn CapabilityCompleter>) -> Self {
        self.completer = Some(completer);
        self
    }

    pub fn with_circuit_breaker(mut self, cb: Arc<dyn CapabilityCircuitBreaker>) -> Self {
        self.circuit_breaker = Some(cb);
        self
    }

    /// 注入 RAR 召回器(启用后,discover 在 request.enable_rar=true 时调用)
    pub fn with_rar_recaller(mut self, recaller: Arc<dyn RarRecaller>) -> Self {
        self.rar_recaller = Some(recaller);
        self
    }
}

/// 从能力护照推导路径地址(L3 输出)
///
/// 推导规则:
/// 1. 取护照的 `domain`
/// 2. 通过 `derive_cluster_for_passport` 按 tags/name 关键词匹配该 domain 下的集群
/// 3. 用集群的 path_segment + 简化后的 capability_id 构造 `RoutingPath`
fn derive_routing_path_from_passport(passport: &CapabilityPassportDto) -> RoutingPath {
    let cluster = derive_cluster_for_passport(passport.domain, &passport.tags, &passport.name);
    RoutingPath::from_passport(passport.domain, cluster.path_segment, &passport.capability_id)
}

#[async_trait]
impl CapabilityRouter for DefaultCapabilityRouter {
    async fn discover(
        &self,
        request: &CapabilityDiscoveryRequest,
    ) -> Result<CapabilityDiscoveryResult, String> {
        let start = std::time::Instant::now();
        let mut timings = Vec::new();

        // L2: 检索
        let t0 = std::time::Instant::now();
        let retrieval = self.retriever.retrieve(&request.query).await?;
        timings.push(PhaseTiming {
            phase: "retrieval".into(),
            elapsed_ms: t0.elapsed().as_millis() as u64,
        });

        // 将候选项转为 PassportDto
        let passports: Vec<CapabilityPassportDto> =
            retrieval.candidates.iter().map(|c| c.passport.clone()).collect();

        // L3: 过滤
        let t0 = std::time::Instant::now();
        let filtered = self.filter.filter_candidates(&passports, &request.filter_context).await;
        timings.push(PhaseTiming {
            phase: "filter".into(),
            elapsed_ms: t0.elapsed().as_millis() as u64,
        });

        // L4: 排序
        let t0 = std::time::Instant::now();
        let retrieval_scores: Vec<f64> =
            retrieval.candidates.iter().map(|c| c.retrieval_score).collect();
        let ranking = self.ranker.rank(
            filtered.passed.clone(),
            &request.user_input,
            retrieval_scores,
            &request.filter_context,
            &request.weights,
        );
        timings.push(PhaseTiming {
            phase: "rank".into(),
            elapsed_ms: t0.elapsed().as_millis() as u64,
        });

        // 选择主命中
        let primary = self.resolve_final_match(&ranking, request).await;
        let alternatives = if ranking.ranked.len() > 1 {
            ranking.ranked[1..].to_vec()
        } else {
            Vec::new()
        };

        // 主动式增强
        let mut suggestions = Vec::new();
        if let (Some(p), Some(completer)) = (&primary, &self.completer)
            && request.enable_completion
        {
            let ctx = crate::capability_circuit::UserContextSnapshot::default();
            suggestions = completer.suggest_completions(&p.passport, &ctx).await;
        }

        // 熔断信息
        let circuit_info = if request.enable_circuit_breaker {
            if let Some(ref cb) = self.circuit_breaker {
                if let Some(ref p) = primary {
                    let state = cb.get_state(&p.passport.capability_id).await;
                    Some(format!("主能力熔断状态: {:?}", state))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // 推导路径地址(L3 输出,由护照 domain + tags/name 推导集群)
        let primary_path = primary.as_ref().map(|p| derive_routing_path_from_passport(&p.passport));
        let alternative_paths: Vec<RoutingPath> =
            alternatives.iter().map(|a| derive_routing_path_from_passport(&a.passport)).collect();

        // RAR 召回(若启用且 recaller 已注入,失败不阻塞主管线)
        let mut rar_result = None;
        if request.enable_rar
            && let Some(ref recaller) = self.rar_recaller
        {
            let t0 = std::time::Instant::now();
            // 兼容 Default 之外被手动设为 0 的场景
            let top_k = if request.rar_top_k == 0 {
                default_rar_top_k()
            } else {
                request.rar_top_k
            };
            match recaller.recall(&request.user_input, top_k).await {
                Ok(result) => rar_result = Some(result),
                Err(_) => {
                    // RAR 失败不阻塞主管线,记录空结果
                    rar_result = Some(RarRecallResult::empty());
                },
            }
            timings.push(PhaseTiming {
                phase: "rar".into(),
                elapsed_ms: t0.elapsed().as_millis() as u64,
            });
        }

        Ok(CapabilityDiscoveryResult {
            primary_match: primary,
            alternatives,
            ambiguous: ranking.ambiguous,
            clarification_prompt: ranking.clarification_suggestion,
            suggestions,
            circuit_info,
            total_elapsed_ms: start.elapsed().as_millis() as u64,
            phase_timings: timings,
            primary_path,
            alternative_paths,
            rar_result,
        })
    }

    async fn resolve_final_match(
        &self,
        ranked: &RankingResult,
        request: &CapabilityDiscoveryRequest,
    ) -> Option<RankedCapability> {
        if ranked.ranked.is_empty() {
            return None;
        }

        // 熔断检查
        if request.enable_circuit_breaker
            && let Some(ref cb) = self.circuit_breaker
        {
            let primary_id = &ranked.ranked[0].passport.capability_id;
            if let Some(available) = cb.resolve_available(primary_id, &[]).await
                && available != *primary_id
            {
                // 降级到替代能力
                return ranked
                    .ranked
                    .iter()
                    .find(|r| r.passport.capability_id == available)
                    .cloned();
            }
        }

        // 入参校验
        let top = &ranked.ranked[0];
        if self.validate_input_match(&top.passport, &request.user_input) {
            Some(top.clone())
        } else if ranked.ranked.len() > 1 {
            // 降级到 Top2
            Some(ranked.ranked[1].clone())
        } else {
            None
        }
    }
}
