// SPDX-License-Identifier: AGPL-3.0-only
//! 能力检索层 — 混合检索（向量 + BM25 + 标签硬匹配 + 负面排除）
//!
//! 底层复用 RAGProvider::hybrid_search 接口，
//! 向上提供能力专用的过滤参数和负面排除逻辑。

use crate::capability::{CapabilityDomain, CapabilityKind, InputModality};
use crate::capability_indexer::CapabilityIndexStats;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 检索请求 ──────────────────────────────────────

/// 能力检索请求
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityQuery {
    /// 用户原始输入文本
    pub user_input: String,
    /// 检索数量
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// 限制能力类型（None = 不限）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_filter: Option<Vec<CapabilityKind>>,
    /// 限制业务域（None = 不限）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_filter: Option<Vec<CapabilityDomain>>,
    /// 要求的输入模态支持（None = 不限）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_modalities: Option<Vec<InputModality>>,
    /// 额外标签硬匹配（AND 逻辑）
    #[serde(default)]
    pub required_tags: Vec<String>,
    /// 排除的能力 ID 列表
    #[serde(default)]
    pub exclude_ids: Vec<String>,
}

fn default_top_k() -> usize {
    20
}

// ── 检索结果 ──────────────────────────────────────

/// 能力检索层级（P0 分层检索降级）—— 规范的分层架构在检索层的落地。
///
/// 检索自上而下逐层尝试：`App`（应用层：工作流）→ `Task`（任务层：技能/模板/工具链）
/// → `Atomic`（原子层：工具/Agent/知识库）。高层命中（该层 top1 综合分达标）即不降级，
/// 返回该层候选；高层未命中才落到下一层，原子层无条件兜底。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityLayer {
    /// 应用层：Workflow（稳定编排路径，命中即直发，认知层不拆解）
    App,
    /// 任务层：Skill / Template / Toolchain（完整执行单元）
    Task,
    /// 原子层：Tool / Agent / KnowledgeBase（底层兜底）
    #[default]
    Atomic,
}

impl CapabilityLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityLayer::App => "app",
            CapabilityLayer::Task => "task",
            CapabilityLayer::Atomic => "atomic",
        }
    }

    /// 由能力类型推导所属层级
    pub fn from_kind(kind: CapabilityKind) -> Self {
        match kind {
            CapabilityKind::Workflow => CapabilityLayer::App,
            CapabilityKind::Skill | CapabilityKind::Template | CapabilityKind::Toolchain => {
                CapabilityLayer::Task
            },
            CapabilityKind::Tool | CapabilityKind::Agent | CapabilityKind::KnowledgeBase => {
                CapabilityLayer::Atomic
            },
        }
    }
}

/// 命中的候选能力
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityCandidate {
    pub capability_id: String,
    pub name: String,
    pub kind: CapabilityKind,
    pub domain: CapabilityDomain,
    /// 检索层级（App/Task/Atomic，由 kind 推导）
    #[serde(default)]
    pub layer: CapabilityLayer,
    /// 语义相似度得分（0.0-1.0）
    pub semantic_score: f64,
    /// BM25/关键词匹配得分（0.0-1.0）
    pub keyword_score: f64,
    /// 标签硬匹配得分（完全匹配 = 1.0，部分匹配 = 0.5，无匹配 = 0.0）
    pub tag_score: f64,
    /// 综合分（semantic * 0.6 + keyword * 0.2 + tag * 0.2）
    pub retrieval_score: f64,
    /// 命中的标签
    #[serde(default)]
    pub matched_tags: Vec<String>,
    /// 是否命中负面场景（true = 应被排除）
    #[serde(default)]
    pub negative_hit: bool,
    /// 护照完整 DTO（供下游过滤/排序使用）
    pub passport: crate::capability::CapabilityPassportDto,
}

/// 检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRetrievalResult {
    /// 候选列表（已按 retrieval_score 降序）
    pub candidates: Vec<CapabilityCandidate>,
    /// 实际召回数量
    pub total_recalled: usize,
    /// 检索耗时（毫秒）
    pub elapsed_ms: u64,
}

// ── 检索器 trait ──────────────────────────────────

/// 能力检索器 — 混合检索引擎
#[async_trait]
pub trait CapabilityRetriever: Send + Sync {
    /// 按查询条件召回 Top-K 候选能力
    async fn retrieve(&self, query: &CapabilityQuery) -> Result<CapabilityRetrievalResult, String>;

    /// 从候选中剔除命中负面场景的能力
    ///
    /// # 实现逻辑
    /// 1. 将用户输入嵌入向量
    /// 2. 在 negative collection 中搜索相似向量
    /// 3. 命中的 capability_id 标记 negative_hit = true
    /// 4. 从候选列表中过滤掉 negative_hit 的能力
    async fn filter_negative(
        &self,
        candidates: Vec<CapabilityCandidate>,
        user_input: &str,
    ) -> Vec<CapabilityCandidate>;

    /// 重新加载/刷新索引（用于热更新场景）
    async fn refresh_index(&self) -> Result<CapabilityIndexStats, String>;
}
