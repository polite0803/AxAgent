// SPDX-License-Identifier: AGPL-3.0-only

//! Paper Overview Engine 实体：论文/长文档结构化概览（LLM 生成，缓存）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "paper_overviews")]
pub struct Model {
    /// 唯一 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联 knowledge_documents.id
    pub document_id: String,
    /// 关联 knowledge_bases.id
    pub knowledge_base_id: String,
    /// 概览类型（paper / long_document / auto）
    pub overview_type: String,
    /// 论文摘要
    #[sea_orm(column_type = "Text", nullable)]
    pub abstract_text: Option<String>,
    /// 核心概念 JSON 数组字符串 ["concept1","concept2"]
    pub key_concepts: String,
    /// 方法论 JSON 数组字符串
    pub methods: String,
    /// 贡献 JSON 数组字符串
    pub contributions: String,
    /// 局限 JSON 数组字符串
    pub limitations: String,
    /// 一句话总结（TL;DR）
    #[sea_orm(column_type = "Text", nullable)]
    pub tl_dr: Option<String>,
    /// 章节结构 JSON 数组字符串 [{title, summary}]
    pub sections: String,
    /// 任意扩展元数据 JSON 字符串（authors/doi/arxiv_id/published_date 等）
    pub metadata_json: String,
    /// 生成模型标识
    pub generated_by: Option<String>,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 更新时间（Unix 毫秒）
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
