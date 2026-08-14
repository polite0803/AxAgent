// SPDX-License-Identifier: AGPL-3.0-only
//! 能力排序器实现 — 加权公式计算最终适配分
//!
//! 复用 harness 层 CapabilityRanker trait 的默认实现，
//! 提供可注入的结构体实例。
//!
//! # 最终分公式
//! final_score = α × semantic + β × history − γ × (1 − speed) − δ × (1 − cost)
//!                + personalization_boost + exploration_boost

use axagent_harness::{
    CapabilityPassportDto, CapabilityRanker, DiscoveryWeights, FilterContext, RankedCapability,
    RankingResult,
};

/// 能力排序器实现
///
/// 可通过配置调整权重和阈值。
#[derive(Debug, Clone)]
pub struct CapabilityRankerImpl {
    /// 模糊检测阈值（Top1 vs Top2 分差低于此值时触发）
    pub ambiguity_threshold: f64,
}

impl Default for CapabilityRankerImpl {
    fn default() -> Self {
        Self { ambiguity_threshold: 0.1 }
    }
}

impl CapabilityRanker for CapabilityRankerImpl {
    fn rank(
        &self,
        candidates: Vec<CapabilityPassportDto>,
        query_text: &str,
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
        let (ambiguous, suggestion) = self.check_ambiguity(&ranked, self.ambiguity_threshold);

        let _ = query_text; // 保留参数供后续使用

        RankingResult { ranked, ambiguous, clarification_suggestion: suggestion }
    }

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
