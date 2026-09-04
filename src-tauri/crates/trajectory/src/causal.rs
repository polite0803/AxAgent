// SPDX-License-Identifier: AGPL-3.0-only

//! 因果边层 — 在知识图谱上叠加带方向、强度、时间延迟与置信度的因果语义
//!
//! 复用 `knowledge_relations` 表（`relation_type = "causes"`），不新增表与列：
//! - `weight` 承载因果强度（成功/共现频率的无偏估计）
//! - `properties` JSON 承载统计量（观测数、置信度、延迟均值与累积 M2）
//!
//! 算法取自教科书级标准方法（无偏样本均值、Laplace 平滑、Welford 在线方差），
//! 独立实现，不引用第三方源码。
//!
//! 设计要点：
//! - 表上无 (source, target, relation_type) 唯一索引，因此采用 read-then-write，
//!   不使用 `on_conflict`（现有 `dao::upsert_relation` 即因此退化为纯 INSERT）。
//! - 延迟只取 step 之间的差值，不假设 `timestamp_ms` 的基准（相对/绝对皆可）。

use anyhow::Result;
use axagent_entities::knowledge_relations;
use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// 因果边在 `knowledge_relations.relation_type` 上的取值。
///
/// 权威定义在 harness 契约层（dao 检索方也引用同一常量），此处 re-export 维持路径稳定。
pub use axagent_harness::knowledge_graph::CAUSAL_RELATION_TYPE;

/// 因果边来源标记，供检索层与文档知识边区分
pub const CAUSAL_SOURCE_TYPE: &str = "causal_observation";

/// 置信度平滑系数：`confidence = n / (n + CONFIDENCE_SMOOTHING)`
const CONFIDENCE_SMOOTHING: f64 = 3.0;

/// 置信度上限 — 经验观测不产生确定性结论
const MAX_CONFIDENCE: f64 = 0.99;

/// 链预测默认最大深度
const DEFAULT_MAX_DEPTH: usize = 5;

/// 链预测深度硬上限，防止调用方传入病态值
const MAX_DEPTH_CAP: usize = 16;

/// 链预测默认最小累计强度，低于此值停止扩展
const DEFAULT_MIN_STRENGTH: f64 = 0.35;

/// 延迟提示的最低置信度门槛 — 预取时机对噪声敏感，门槛高于链预测
pub const DEFAULT_HINT_MIN_CONFIDENCE: f64 = 0.5;

/// 单条轨迹最多观测的边数，防止超长轨迹撑爆写入
const MAX_EDGES_PER_TRAJECTORY: usize = 32;

/// topic 规范化后的最大长度
const TOPIC_MAX_LEN: usize = 64;

/// 因果边统计量 — 存入 `knowledge_relations.properties`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalEdgeStats {
    /// 置信度 `min(0.99, n / (n + 3))`，低样本时自动抑制
    pub confidence: f64,
    /// 总观测次数
    pub observations: u64,
    /// 命中次数（工具未报错 / 结果匹配）
    pub positive: u64,
    /// 时间延迟均值（毫秒）
    pub delay_mean_ms: f64,
    /// Welford 累积 M2；样本方差 = `delay_m2_ms / (observations - 1)`
    pub delay_m2_ms: f64,
    /// 末次观测时间戳（epoch 秒）
    pub last_observed_at: i64,
}

impl Default for CausalEdgeStats {
    fn default() -> Self {
        Self {
            confidence: 0.0,
            observations: 0,
            positive: 0,
            delay_mean_ms: 0.0,
            delay_m2_ms: 0.0,
            last_observed_at: 0,
        }
    }
}

impl CausalEdgeStats {
    /// 因果强度 = 命中率。无观测时为 0.0
    pub fn strength(&self) -> f64 {
        if self.observations == 0 {
            return 0.0;
        }
        self.positive as f64 / self.observations as f64
    }

    /// 延迟样本标准差（毫秒）。观测数不足 2 时为 0.0
    pub fn delay_std_ms(&self) -> f64 {
        if self.observations < 2 {
            return 0.0;
        }
        let var = self.delay_m2_ms / (self.observations as f64 - 1.0);
        if var <= 0.0 { 0.0 } else { var.sqrt() }
    }

    /// 融入一次新观测，返回新的统计量
    ///
    /// 强度采用无偏样本均值 `w' = (w*n + o) / (n+1)`，比固定学习率抗噪且收敛无偏。
    /// 延迟采用 Welford 在线算法增量更新均值与累积 M2，不保留历史样本。
    fn observe(&self, positive: bool, delay_ms: Option<i64>, now_ts: i64) -> Self {
        let observations = self.observations + 1;
        let nf = observations as f64;
        let positive_count = self.positive + u64::from(positive);

        let (delay_mean_ms, delay_m2_ms) = match delay_ms {
            None => (self.delay_mean_ms, self.delay_m2_ms),
            Some(ms) => {
                let x = ms as f64;
                let delta = x - self.delay_mean_ms;
                let mean_new = self.delay_mean_ms + delta / nf;
                let delta2 = x - mean_new;
                (mean_new, self.delay_m2_ms + delta * delta2)
            },
        };

        Self {
            confidence: (nf / (nf + CONFIDENCE_SMOOTHING)).min(MAX_CONFIDENCE),
            observations,
            positive: positive_count,
            delay_mean_ms,
            delay_m2_ms,
            last_observed_at: now_ts,
        }
    }
}

/// 一条因果链 — 由 [`predict_chain`] 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChain {
    /// 实体 ID 序列，长度 ≥ 2
    pub path: Vec<String>,
    /// 累计强度 = 路径上各边强度之积
    pub strength: f64,
    /// 累计延迟 = 路径上各边延迟均值之和（毫秒）
    pub total_delay_ms: i64,
}

/// 工具实体 ID
pub fn tool_entity(tool_name: &str) -> String {
    format!("tool:{tool_name}")
}

/// 结果实体 ID
pub fn outcome_entity(outcome: &str) -> String {
    format!("outcome:{outcome}")
}

/// 意图实体 ID
pub fn intent_entity(intent: &str) -> String {
    format!("intent:{intent}")
}

/// [`PredictedIntent`] → 因果实体 ID。
///
/// `Unknown` 返回空串，不会命中任何边或提示键。
pub fn prediction_intent_entity(intent: &crate::proactive_assistant::PredictedIntent) -> String {
    use crate::proactive_assistant::PredictedIntent;
    let name = match intent {
        PredictedIntent::CodeCompletion { .. } => "code_completion",
        PredictedIntent::Documentation { .. } => "documentation",
        PredictedIntent::Search { .. } => "search",
        PredictedIntent::Refactoring { .. } => "refactoring",
        PredictedIntent::Debug { .. } => "debug",
        PredictedIntent::TestGeneration { .. } => "test_generation",
        PredictedIntent::Unknown => return String::new(),
    };
    intent_entity(name)
}

/// 话题实体 ID — 规范化后截断
pub fn topic_entity(topic: &str) -> String {
    format!("topic:{}", normalize_topic(topic))
}

/// 规范化话题名：小写化，非字母数字字符折叠为下划线并合并，截断 64 字符
pub fn normalize_topic(topic: &str) -> String {
    let mut out = String::with_capacity(topic.len().min(TOPIC_MAX_LEN));
    let mut pending_sep = false;

    for ch in topic.chars() {
        if out.len() >= TOPIC_MAX_LEN {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            if pending_sep && !out.is_empty() {
                out.push('_');
            }
            // 无论是否产出分隔符都要清除，否则前导分隔符会污染下一个字符
            pending_sep = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_sep = true;
        }
    }

    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

fn stats_to_json(stats: &CausalEdgeStats) -> serde_json::Value {
    serde_json::to_value(stats).unwrap_or_else(|_| serde_json::Value::Object(Default::default()))
}

/// 从 `properties` 解析统计量。字段缺失或类型不符时按零值兜底，
/// 保证旧数据与手写数据不会导致解析失败。
fn json_to_stats(raw: Option<&serde_json::Value>) -> CausalEdgeStats {
    let Some(value) = raw else {
        return CausalEdgeStats::default();
    };
    serde_json::from_value(value.clone()).unwrap_or_default()
}

/// 查找单条因果边（不含统计量解析）
async fn find_edge_model(
    db: &DatabaseConnection,
    cause: &str,
    effect: &str,
) -> Result<Option<knowledge_relations::Model>> {
    Ok(knowledge_relations::Entity::find()
        .filter(knowledge_relations::Column::SourceEntityId.eq(cause))
        .filter(knowledge_relations::Column::TargetEntityId.eq(effect))
        .filter(knowledge_relations::Column::RelationType.eq(CAUSAL_RELATION_TYPE))
        .one(db)
        .await?)
}

/// 读取一条因果边的统计量。不存在时返回 `None`
pub async fn get_edge(
    db: &DatabaseConnection,
    cause: &str,
    effect: &str,
) -> Result<Option<CausalEdgeStats>> {
    Ok(find_edge_model(db, cause, effect).await?.map(|m| json_to_stats(m.properties.as_ref())))
}

/// 列出全部因果边的统计量（供校准器周期采样：predicted=confidence, actual=strength）。
///
/// 只返回 `observations > 0` 的边——零观测边的 confidence/strength 无意义。
pub async fn list_causal_edge_stats(
    db: &DatabaseConnection,
    kb_id: &str,
) -> Result<Vec<CausalEdgeStats>> {
    let models = knowledge_relations::Entity::find()
        .filter(knowledge_relations::Column::KnowledgeBaseId.eq(kb_id))
        .filter(knowledge_relations::Column::RelationType.eq(CAUSAL_RELATION_TYPE))
        .all(db)
        .await?;
    Ok(models
        .into_iter()
        .map(|m| json_to_stats(m.properties.as_ref()))
        .filter(|s| s.observations > 0)
        .collect())
}

/// 观测一次因果事件并落库
///
/// 采用 read-then-write：表上无 (source, target, relation_type) 唯一索引，
/// `on_conflict` 无法在此维度生效（现有 `dao::upsert_relation` 即因此退化为纯 INSERT）。
pub async fn observe_edge(
    db: &DatabaseConnection,
    cause: &str,
    effect: &str,
    positive: bool,
    delay_ms: Option<i64>,
    trajectory_id: &str,
) -> Result<CausalEdgeStats> {
    let now_ts = Utc::now().timestamp();

    let existing = find_edge_model(db, cause, effect).await?;
    let base = existing.as_ref().map(|m| json_to_stats(m.properties.as_ref())).unwrap_or_default();
    let next = base.observe(positive, delay_ms, now_ts);
    let weight = next.strength();
    let properties = stats_to_json(&next);

    match existing {
        Some(model) => {
            knowledge_relations::Entity::update(knowledge_relations::ActiveModel {
                id: Set(model.id),
                weight: Set(weight),
                properties: Set(Some(properties)),
                updated_at: Set(now_ts),
                source_id: Set(trajectory_id.to_string()),
                ..Default::default()
            })
            .exec(db)
            .await?;
        },
        None => {
            knowledge_relations::Entity::insert(knowledge_relations::ActiveModel {
                id: Set(format!("rel_causal_{}", uuid::Uuid::new_v4())),
                knowledge_base_id: Set(String::new()),
                source_entity_id: Set(cause.to_string()),
                target_entity_id: Set(effect.to_string()),
                relation_type: Set(CAUSAL_RELATION_TYPE.to_string()),
                description: Set(None),
                properties: Set(Some(properties)),
                metadata: Set(None),
                created_at: Set(now_ts),
                updated_at: Set(now_ts),
                weight: Set(weight),
                source_type: Set(CAUSAL_SOURCE_TYPE.to_string()),
                source_id: Set(trajectory_id.to_string()),
            })
            .exec(db)
            .await?;
        },
    }

    Ok(next)
}

/// 从某实体出发的全部因果出边
async fn outgoing_edges(
    db: &DatabaseConnection,
    from: &str,
) -> Result<Vec<(String, CausalEdgeStats)>> {
    let rows = knowledge_relations::Entity::find()
        .filter(knowledge_relations::Column::SourceEntityId.eq(from))
        .filter(knowledge_relations::Column::RelationType.eq(CAUSAL_RELATION_TYPE))
        .all(db)
        .await?;

    Ok(rows
        .into_iter()
        .map(|m| {
            let stats = json_to_stats(m.properties.as_ref());
            (m.target_entity_id, stats)
        })
        .collect())
}

/// 预测从 `from` 出发的因果链
///
/// BFS 遍历，累计强度按路径乘积、累计延迟按路径求和衰减；
/// 累计强度低于 `min_strength` 时停止扩展，路径内已出现的节点不再重复进入。
pub async fn predict_chain(
    db: &DatabaseConnection,
    from: &str,
    max_depth: usize,
    min_strength: f64,
) -> Result<Vec<CausalChain>> {
    let depth = max_depth.clamp(1, MAX_DEPTH_CAP);
    let mut out: Vec<CausalChain> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(from.to_string());

    let mut queue: VecDeque<(Vec<String>, f64, i64)> = VecDeque::new();
    queue.push_back((vec![from.to_string()], 1.0, 0));

    while let Some((path, strength, delay)) = queue.pop_front() {
        let Some(tail) = path.last() else {
            continue;
        };

        for (to, stats) in outgoing_edges(db, tail).await? {
            if visited.contains(&to) || path.contains(&to) {
                continue;
            }
            let next_strength = strength * stats.strength();
            if next_strength < min_strength {
                continue;
            }

            let next_delay = delay + stats.delay_mean_ms.max(0.0).round() as i64;
            let mut next_path = path.clone();
            next_path.push(to.clone());

            out.push(CausalChain {
                path: next_path.clone(),
                strength: next_strength,
                total_delay_ms: next_delay,
            });

            if next_path.len() < depth {
                queue.push_back((next_path, next_strength, next_delay));
            }
        }
    }

    out.sort_by(|a, b| b.strength.total_cmp(&a.strength));
    Ok(out)
}

/// 按默认参数预测因果链 — 深度 5、最小累计强度 0.35
///
/// 调用方无需各自硬编码阈值。需要精细控制时用 [`predict_chain`]。
pub async fn predict_chain_with_defaults(
    db: &DatabaseConnection,
    from: &str,
) -> Result<Vec<CausalChain>> {
    predict_chain(db, from, DEFAULT_MAX_DEPTH, DEFAULT_MIN_STRENGTH).await
}

/// 构建「后继实体 → 观测延迟(ms)」提示表
///
/// 供预取器替代硬编码的准备耗时估算。只收录置信度达标的边——
/// 预取时机对噪声敏感，小样本会把提前量带偏。
pub async fn build_delay_hints(
    db: &DatabaseConnection,
    from: &str,
    min_confidence: f64,
) -> Result<HashMap<String, i64>> {
    let mut hints = HashMap::new();
    for (to, stats) in outgoing_edges(db, from).await? {
        if stats.confidence < min_confidence {
            continue;
        }
        hints.insert(to, stats.delay_mean_ms.max(0.0).round() as i64);
    }
    Ok(hints)
}

/// 因果建议的存活时长（分钟）。证据随统计演化，建议不宜长命
const CAUSAL_SUGGESTION_TTL_MINUTES: i64 = 5;

/// 实体 ID 的展示名：去掉 `tool:` / `intent:` 等前缀
fn entity_display_name(entity_id: &str) -> &str {
    entity_id.split_once(':').map(|(_, name)| name).unwrap_or(entity_id)
}

/// 依据因果边为当前预测意图生成可解释建议
///
/// 只取意图实体（`intent:<name>`）的高置信度出边，按强度降序截断。
/// 低样本边自动被排除：`confidence = n/(n+3)`，观测 < 3 次时必然低于
/// [`DEFAULT_HINT_MIN_CONFIDENCE`]。查询失败只记日志，返回空表——
/// 建议是增强信息，不能因它失败。
pub async fn causal_suggestions_for_intent(
    db: &DatabaseConnection,
    prediction: &crate::proactive_assistant::ContextPrediction,
    max: usize,
) -> Vec<crate::proactive_assistant::ProactiveSuggestion> {
    let from = prediction_intent_entity(&prediction.predicted_intent);
    if from.is_empty() || max == 0 {
        return Vec::new();
    }

    let suggestions = build_causal_suggestions(db, &from, max).await;
    match suggestions {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("causal: suggestion query failed for {from}: {e:#}");
            Vec::new()
        },
    }
}

async fn build_causal_suggestions(
    db: &DatabaseConnection,
    from: &str,
    max: usize,
) -> Result<Vec<crate::proactive_assistant::ProactiveSuggestion>> {
    use crate::proactive_assistant::{
        Priority, ProactiveSuggestion, SuggestionAction, SuggestionType,
    };

    let mut edges: Vec<(String, CausalEdgeStats)> = outgoing_edges(db, from)
        .await?
        .into_iter()
        .filter(|(_, s)| s.confidence >= DEFAULT_HINT_MIN_CONFIDENCE)
        .collect();
    edges.sort_by(|a, b| b.1.strength().total_cmp(&a.1.strength()));
    edges.truncate(max);

    Ok(edges
        .into_iter()
        .map(|(to, s)| {
            let delay_secs = s.delay_mean_ms / 1000.0;
            ProactiveSuggestion::new(
                SuggestionType::CausalInsight,
                format!("Next: {}", entity_display_name(&to)),
                format!(
                    "{} → {} 置信度 {:.2}（观测 {} 次），平均间隔 {delay_secs:.1} 秒",
                    entity_display_name(from),
                    entity_display_name(&to),
                    s.confidence,
                    s.observations
                ),
                SuggestionAction::CausalInsight { from_entity: from.to_string(), to_entity: to },
                Priority::Medium,
                CAUSAL_SUGGESTION_TTL_MINUTES,
            )
        })
        .collect())
}

/// 从一条轨迹中抽取因果观测
///
/// 三类边：
/// - **T**：相邻 step 的工具序列 `tool:A → tool:B`（供预取时机使用）
/// - **O**：工具 → 轨迹结果 `tool:A → outcome:X`，命中 = 工具表现与最终结果一致
/// - **P**：话题 → 轨迹结果 `topic:X → outcome:X`，命中 = 结果为正向
///
/// 延迟一律取 step 之间的差值，不假设 `timestamp_ms` 是相对还是绝对基准。
pub async fn observe_from_trajectory(
    db: &DatabaseConnection,
    t: &crate::trajectory::Trajectory,
) -> Result<usize> {
    let outcome_id = outcome_entity(&outcome_slug(t.outcome));
    let last_ts = t.steps.last().map(|s| s.timestamp_ms).unwrap_or(0);
    let mut observed = 0usize;

    // T 边与 O 边：按 step 顺序扫描
    let mut prev_tools: Option<(u64, Vec<String>)> = None;

    for step in &t.steps {
        let called: Vec<String> = step
            .tool_calls
            .as_ref()
            .map(|calls| calls.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();

        // T 边：上一步工具 → 本步工具
        if let Some((prev_ts, prev_names)) = &prev_tools {
            let delay = step.timestamp_ms.saturating_sub(*prev_ts);
            for from in prev_names {
                for to in &called {
                    if from == to {
                        continue;
                    }
                    if !push_observation(
                        db,
                        &tool_entity(from),
                        &tool_entity(to),
                        true,
                        Some(delay as i64),
                        &t.id,
                        &mut observed,
                    )
                    .await?
                    {
                        return Ok(observed);
                    }
                }
            }
        }

        // O 边：工具 → 轨迹结果
        //
        // 命中判据是「工具表现与最终结果是否一致」，而非单纯的「工具没报错」。
        // 若只看工具是否报错，则 tool:A→success 与 tool:A→failure 的强度会同时
        // 收敛到工具可靠性，无法区分「用 A 导致成功」还是「用 A 导致失败」，
        // 因果边也就退化成了工具可靠性统计。
        if let Some(results) = &step.tool_results {
            let delay = last_ts.saturating_sub(step.timestamp_ms);
            let outcome_ok = outcome_is_positive(t.outcome);
            for r in results {
                let positive = !r.is_error == outcome_ok;
                if !push_observation(
                    db,
                    &tool_entity(&r.tool_name),
                    &outcome_id,
                    positive,
                    Some(delay as i64),
                    &t.id,
                    &mut observed,
                )
                .await?
                {
                    return Ok(observed);
                }
            }
        }

        if called.is_empty() {
            continue;
        }
        prev_tools = Some((step.timestamp_ms, called));
    }

    // P 边：话题 → 轨迹结果。命中判据为「结果是正向的」
    let positive = outcome_is_positive(t.outcome);
    push_observation(
        db,
        &topic_entity(&t.topic),
        &outcome_id,
        positive,
        None,
        &t.id,
        &mut observed,
    )
    .await?;

    Ok(observed)
}

/// 写入一条观测并累加计数。达到 `MAX_EDGES_PER_TRAJECTORY` 后返回 `false` 要求中止
#[allow(clippy::too_many_arguments)]
async fn push_observation(
    db: &DatabaseConnection,
    cause: &str,
    effect: &str,
    positive: bool,
    delay_ms: Option<i64>,
    trajectory_id: &str,
    observed: &mut usize,
) -> Result<bool> {
    if *observed >= MAX_EDGES_PER_TRAJECTORY {
        return Ok(false);
    }
    observe_edge(db, cause, effect, positive, delay_ms, trajectory_id).await?;
    *observed += 1;
    Ok(true)
}

/// 轨迹结果是否为正向（成功或部分成功）
fn outcome_is_positive(outcome: crate::trajectory::TrajectoryOutcome) -> bool {
    use crate::trajectory::TrajectoryOutcome;
    matches!(outcome, TrajectoryOutcome::Success | TrajectoryOutcome::Partial)
}

/// `TrajectoryOutcome` → 实体 ID 段
fn outcome_slug(outcome: crate::trajectory::TrajectoryOutcome) -> String {
    use crate::trajectory::TrajectoryOutcome;
    match outcome {
        TrajectoryOutcome::Success => "success".to_string(),
        TrajectoryOutcome::Failure => "failure".to_string(),
        TrajectoryOutcome::Partial => "partial".to_string(),
        TrajectoryOutcome::Abandoned => "abandoned".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000;

    #[test]
    fn first_observation_creates_stats() {
        let next = CausalEdgeStats::default().observe(true, Some(100), T0);
        assert_eq!(next.observations, 1);
        assert_eq!(next.positive, 1);
        assert!((next.strength() - 1.0).abs() < f64::EPSILON);
        // n=1 → 1/(1+3) = 0.25
        assert!((next.confidence - 0.25).abs() < 1e-9);
        assert_eq!(next.last_observed_at, T0);
    }

    #[test]
    fn repeated_observation_converges_to_rate() {
        let mut s = CausalEdgeStats::default();
        for _ in 0..8 {
            s = s.observe(true, None, T0);
        }
        for _ in 0..2 {
            s = s.observe(false, None, T0);
        }
        assert_eq!(s.observations, 10);
        assert_eq!(s.positive, 8);
        assert!((s.strength() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn confidence_grows_and_is_capped() {
        let mut s = CausalEdgeStats::default();
        let mut prev = 0.0;
        // n/(n+3) 需 n ≥ 297 才触顶，取 500 保证封顶分支被覆盖
        for _ in 0..500 {
            s = s.observe(true, None, T0);
            assert!(s.confidence >= prev, "confidence 必须单调不减");
            prev = s.confidence;
        }
        assert!(s.confidence <= MAX_CONFIDENCE);
        assert!((s.confidence - MAX_CONFIDENCE).abs() < 1e-9);
    }

    #[test]
    fn confidence_stays_below_cap_before_saturation() {
        // 200 次观测时 n/(n+3) ≈ 0.985，尚未触顶——验证未饱和区的行为
        let mut s = CausalEdgeStats::default();
        for _ in 0..200 {
            s = s.observe(true, None, T0);
        }
        assert!(s.confidence < MAX_CONFIDENCE);
        assert!((s.confidence - 200.0f64 / 203.0).abs() < 1e-9);
    }

    #[test]
    fn welford_variance_matches_hand_computed() {
        let mut s = CausalEdgeStats::default();
        for ms in [10_i64, 20, 30] {
            s = s.observe(true, Some(ms), T0);
        }
        assert!((s.delay_mean_ms - 20.0).abs() < 1e-9);
        // 样本方差 = ((10-20)^2 + 0 + (30-20)^2) / 2 = 100
        assert!((s.delay_std_ms() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn delay_absent_preserves_previous_stats() {
        let first = CausalEdgeStats::default().observe(true, Some(500), T0);
        let second = first.observe(true, None, T0 + 10);
        assert!((second.delay_mean_ms - first.delay_mean_ms).abs() < f64::EPSILON);
        assert!((second.delay_m2_ms - first.delay_m2_ms).abs() < f64::EPSILON);
        assert_eq!(second.observations, 2);
    }

    #[test]
    fn std_dev_zero_when_insufficient_samples() {
        let s = CausalEdgeStats::default().observe(true, Some(100), T0);
        assert!((s.delay_std_ms() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_stats_have_zero_strength() {
        let s = CausalEdgeStats::default();
        assert!((s.strength() - 0.0).abs() < f64::EPSILON);
        assert_eq!(s.observations, 0);
    }

    #[test]
    fn normalize_topic_collapses_and_lowercases() {
        assert_eq!(normalize_topic("Refactor Auth/Login!!"), "refactor_auth_login");
        assert_eq!(normalize_topic("  Fix   Bug  "), "fix_bug");
        assert_eq!(normalize_topic("RPC-42"), "rpc_42");
    }

    #[test]
    fn normalize_topic_handles_empty_and_overflow() {
        assert_eq!(normalize_topic(""), "unknown");
        assert_eq!(normalize_topic("!!!"), "unknown");
        let long = normalize_topic(&"a".repeat(200));
        assert_eq!(long.len(), TOPIC_MAX_LEN);
    }

    #[test]
    fn entity_id_encodings() {
        assert_eq!(tool_entity("read_file"), "tool:read_file");
        assert_eq!(outcome_entity("success"), "outcome:success");
        assert_eq!(topic_entity("Fix Bug"), "topic:fix_bug");
    }

    #[test]
    fn stats_json_roundtrip() {
        let s = CausalEdgeStats::default().observe(true, Some(42), T0);
        let json = stats_to_json(&s);
        let back = json_to_stats(Some(&json));
        assert_eq!(back, s);
    }

    #[test]
    fn missing_properties_fall_back_to_default() {
        assert_eq!(json_to_stats(None), CausalEdgeStats::default());
        assert_eq!(
            json_to_stats(Some(&serde_json::Value::Object(Default::default()))),
            CausalEdgeStats::default()
        );
        assert_eq!(
            json_to_stats(Some(&serde_json::json!({"observations": "not_a_number"}))),
            CausalEdgeStats::default()
        );
    }

    #[test]
    fn outcome_slug_covers_all_variants() {
        use crate::trajectory::TrajectoryOutcome;
        assert_eq!(outcome_slug(TrajectoryOutcome::Success), "success");
        assert_eq!(outcome_slug(TrajectoryOutcome::Failure), "failure");
        assert_eq!(outcome_slug(TrajectoryOutcome::Partial), "partial");
        assert_eq!(outcome_slug(TrajectoryOutcome::Abandoned), "abandoned");
    }

    #[test]
    fn outcome_positive_classification() {
        use crate::trajectory::TrajectoryOutcome;
        assert!(outcome_is_positive(TrajectoryOutcome::Success));
        assert!(outcome_is_positive(TrajectoryOutcome::Partial));
        assert!(!outcome_is_positive(TrajectoryOutcome::Failure));
        assert!(!outcome_is_positive(TrajectoryOutcome::Abandoned));
    }

    /// O 边命中的四种组合：工具表现与最终结果一致才算命中
    #[test]
    fn tool_outcome_agreement_matrix() {
        // (工具是否报错, 结果是否正向) → 是否命中
        let cases = [
            (false, true, true),   // 工具正常 + 成功：一致
            (true, false, true),   // 工具报错 + 失败：一致
            (false, false, false), // 工具正常 + 失败：不一致
            (true, true, false),   // 工具报错 + 成功：不一致
        ];
        for (is_error, outcome_ok, expected) in cases {
            assert_eq!(!is_error == outcome_ok, expected, "case {is_error}/{outcome_ok}");
        }
    }
}
