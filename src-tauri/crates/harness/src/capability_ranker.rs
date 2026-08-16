// SPDX-License-Identifier: AGPL-3.0-only
//! 加权排序器 — 基于 6 个加权分量计算最终适配分
//!
//! 在过滤闸门（[`crate::capability_filter::FilterDimension`]，维度零~维度八共 9 维硬过滤）之后，
//! 对通过的能力按以下 6 个分量加权排序：
//! 1. semantic 语义相似度
//! 2. history 历史成功率
//! 3. speed 耗时（越短越高）
//! 4. cost 成本（越低越高）
//! 5. personalization_boost 个性化提权
//! 6. exploration_boost 冷启动探索提权
//!
//! # 最终分公式
//! final_score = α × semantic + β × history − γ × (1 − speed) − δ × (1 − cost)
//!                + personalization_boost + exploration_boost

use crate::capability::{CapabilityPassportDto, DiscoveryWeights};
use crate::capability_filter::FilterContext;
use serde::{Deserialize, Serialize};

// ── 排序结果 ──────────────────────────────────────

/// 排序后的能力条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedCapability {
    pub passport: CapabilityPassportDto,
    /// 语义相似度分（0.0-1.0）
    pub semantic_score: f64,
    /// 历史成功率分（0.0-1.0）
    pub history_score: f64,
    /// 耗时分（越短越高，0.0-1.0）
    pub speed_score: f64,
    /// 成本分（越低越高，0.0-1.0）
    pub cost_score: f64,
    /// 个性化提权（命中历史使用）
    pub personalization_boost: f64,
    /// 冷启动探索提权
    pub exploration_boost: f64,
    /// 最终加权分
    pub final_score: f64,
    /// 命中原因（供前端展示）
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// 排序结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RankingResult {
    /// 排好序的候选列表
    pub ranked: Vec<RankedCapability>,
    /// 是否触发模糊发现（Top1 vs Top2 分差 < 阈值）
    pub ambiguous: bool,
    /// 模糊发现时的澄清建议
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarification_suggestion: Option<String>,
}

// ── 排序器 trait ──────────────────────────────────

/// 能力排序器 — 基于加权公式计算最终适配分
pub trait CapabilityRanker: Send + Sync {
    /// 对通过闸门的候选进行排序
    fn rank(
        &self,
        candidates: Vec<CapabilityPassportDto>,
        _query_text: &str,
        retrieval_scores: Vec<f64>,
        ctx: &FilterContext,
        weights: &DiscoveryWeights,
    ) -> RankingResult {
        let mut ranked: Vec<RankedCapability> = candidates
            .into_iter()
            .enumerate()
            .map(|(i, passport)| {
                let semantic = retrieval_scores.get(i).copied().unwrap_or(0.0);
                self.compute_score(&passport, semantic, ctx, weights)
            })
            .collect();

        // 按最终分降序
        ranked.sort_by(|a, b| {
            b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        // 模糊发现检测
        let (ambiguous, suggestion) = self.check_ambiguity(&ranked, 0.1);

        RankingResult { ranked, ambiguous, clarification_suggestion: suggestion }
    }

    /// 计算单个能力的最终分
    fn compute_score(
        &self,
        passport: &CapabilityPassportDto,
        semantic_score: f64,
        ctx: &FilterContext,
        weights: &DiscoveryWeights,
    ) -> RankedCapability {
        let stats = &passport.stats;

        // 语义分
        let semantic = semantic_score.clamp(0.0, 1.0);

        // 历史成功率分
        let history = stats.recent_success_rate.clamp(0.0, 1.0);

        // 耗时分（归一化：30秒内 = 1.0，60秒 = 0.5，>120秒 = 0.0）
        let speed = if let Some(dur) = passport.avg_duration_seconds {
            (1.0 - (dur / 120.0)).clamp(0.0, 1.0)
        } else {
            0.5 // 未知耗时给中性分
        };

        // 成本分（归一化：$0.01 内 = 1.0，$0.05 = 0.5，>$0.10 = 0.0）
        let cost = if let Some(c) = passport.estimated_cost_usd {
            (1.0 - (c / 0.10)).clamp(0.0, 1.0)
        } else {
            0.5 // 未知成本给中性分
        };

        // 个性化提权
        let personalization = if ctx.user_history_ids.contains(&passport.capability_id) {
            weights.personalization_boost
        } else {
            0.0
        };

        // 冷启动探索提权（仅对真正的新能力：总调用次数 < 10 才提权，且快速衰减）
        // 阈值从 100 降到 10，避免大量"未调用但非新"的能力获得提权扭曲排序
        let exploration = if stats.total_calls < 10 {
            weights.exploration_boost * (1.0 - stats.total_calls as f64 / 10.0)
        } else {
            0.0
        };

        // 最终分
        let final_score = weights.alpha * semantic + weights.beta * history
            - weights.gamma * (1.0 - speed)
            - weights.delta * (1.0 - cost)
            + personalization
            + exploration;

        let mut reasons = Vec::new();
        if personalization > 0.0 {
            reasons.push("你历史上使用过此能力".to_string());
        }
        if exploration > 0.0 {
            reasons.push("新上线能力，探索提权中".to_string());
        }
        if semantic > 0.8 {
            reasons.push("高度匹配语义".to_string());
        }

        RankedCapability {
            passport: passport.clone(),
            semantic_score: semantic,
            history_score: history,
            speed_score: speed,
            cost_score: cost,
            personalization_boost: personalization,
            exploration_boost: exploration,
            final_score,
            reasons,
        }
    }

    /// 检查是否触发模糊发现（维度一：置信度）
    fn check_ambiguity(
        &self,
        ranked: &[RankedCapability],
        threshold: f64,
    ) -> (bool, Option<String>) {
        if ranked.len() < 2 {
            return (false, None);
        }
        let gap = ranked[0].final_score - ranked[1].final_score;
        if gap < threshold {
            let suggestion = format!(
                "检测到 {} 和 {} 分差仅 {:.3}，建议进一步澄清需求",
                ranked[0].passport.name, ranked[1].passport.name, gap
            );
            (true, Some(suggestion))
        } else {
            (false, None)
        }
    }
}
