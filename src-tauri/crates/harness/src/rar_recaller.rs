// SPDX-License-Identifier: AGPL-3.0-only
//! RAR(检索增强路由)召回器契约 — 软引导能力发现的向量检索层
//!
//! # RAR 流程
//! 用户输入 → 嵌入 → 向量检索召回 Top-K → 软引导 Prompt 注入
//!
//! # 软引导措辞
//! RAR 不强制约束 LLM 选择,而是通过 Prompt 软引导:
//! "以下是推荐能力(按相似度排序),优先从中选择,若无匹配可自行判断"。
//!
//! # 实现位置
//! trait 定义在 harness(foundation 层),具体实现(`RarRecaller` 的 impl)
//! 在 `tools` crate(hybrid 层)完成,依赖向量检索基础设施。
//! 本 PR 仅定义 trait + DTO,不提供实现。

use crate::routing_path::RoutingPath;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── RAR 召回结果 DTO ──────────────────────────────

/// RAR 召回结果 — 软引导能力推荐的完整输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RarRecallResult {
    /// Top-K 命中的路径地址(按相似度降序)
    #[serde(default)]
    pub recalled_paths: Vec<RoutingPath>,
    /// Top-K 命中的完整 capability_id(与 paths 一一对应)
    #[serde(default)]
    pub recalled_capabilities: Vec<String>,
    /// 各候选相似度得分(0.0-1.0,与 paths 一一对应)
    #[serde(default)]
    pub similarity_scores: Vec<f64>,
    /// 已组装的软引导 Prompt(可直接拼接到 System Prompt)
    #[serde(default)]
    pub injected_prompt: String,
}

impl RarRecallResult {
    /// 构造空结果(无召回时使用)
    pub fn empty() -> Self {
        RarRecallResult {
            recalled_paths: Vec::new(),
            recalled_capabilities: Vec::new(),
            similarity_scores: Vec::new(),
            injected_prompt: String::new(),
        }
    }
}

impl Default for RarRecallResult {
    fn default() -> Self {
        Self::empty()
    }
}

// ── 软引导 Prompt 构造器 ──────────────────────────

/// 构造 RAR 软引导 Prompt
///
/// # 模板
/// ```text
/// 以下是推荐能力(按相似度排序),优先从中选择,若无匹配可自行判断:
/// 1. /core/file_ops/read_file (相似度 0.92)
/// 2. /invest/market_data/get_quote (相似度 0.85)
/// ...
/// ```
///
/// 若候选为空,返回空字符串(调用方可据此跳过注入)。
pub fn build_rar_prompt(paths: &[RoutingPath], scores: &[f64]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let mut prompt = String::from("以下是推荐能力(按相似度排序),优先从中选择,若无匹配可自行判断:");
    for (i, path) in paths.iter().enumerate() {
        let score = scores.get(i).copied().unwrap_or(0.0);
        prompt.push_str(&format!("\n{}. {} (相似度 {:.2})", i + 1, path.to_path_string(), score));
    }
    prompt
}

// ── RAR 召回器 trait ──────────────────────────────

/// RAR 召回器 — 检索增强路由的向量召回接口
///
/// # 职责
/// 1. 将用户输入嵌入为向量
/// 2. 在能力向量索引中召回 Top-K 相似能力
/// 3. 将结果转为 `RoutingPath` 并生成软引导 Prompt
///
/// # 实现方
/// `tools` crate(依赖向量检索 + capability 索引)
#[async_trait]
pub trait RarRecaller: Send + Sync {
    /// 召回 Top-K 相似能力并生成软引导 Prompt
    ///
    /// # 参数
    /// - `user_input`: 用户原始输入文本
    /// - `top_k`: 召回数量(默认由调用方传入,通常为 5)
    ///
    /// # 返回
    /// `RarRecallResult`,包含路径列表、capability_id 列表、相似度分数、已组装 Prompt。
    /// 若召回失败或无命中,返回空结果(`RarRecallResult::empty()`)。
    async fn recall(&self, user_input: &str, top_k: usize) -> Result<RarRecallResult, String>;
}
