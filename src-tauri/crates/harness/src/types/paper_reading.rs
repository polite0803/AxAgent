// SPDX-License-Identifier: AGPL-3.0-only

//! Paper Overview Engine + Reading List DTO
//!
//! 纯数据 DTO，不依赖重型实现模块。

use serde::{Deserialize, Serialize};

/// 论文章节结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaperSection {
    pub title: String,
    pub summary: String,
}

/// 论文/长文档结构化概览
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperOverview {
    pub id: String,
    pub document_id: String,
    pub knowledge_base_id: String,
    pub overview_type: String,
    pub abstract_text: Option<String>,
    pub key_concepts: Vec<String>,
    pub methods: Vec<String>,
    pub contributions: Vec<String>,
    pub limitations: Vec<String>,
    pub tl_dr: Option<String>,
    pub sections: Vec<PaperSection>,
    pub metadata: serde_json::Value,
    pub generated_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaperOverviewInput {
    pub document_id: String,
    pub knowledge_base_id: String,
    pub overview_type: Option<String>,
    pub abstract_text: Option<String>,
    pub key_concepts: Vec<String>,
    pub methods: Vec<String>,
    pub contributions: Vec<String>,
    pub limitations: Vec<String>,
    pub tl_dr: Option<String>,
    pub sections: Vec<PaperSection>,
    pub metadata: Option<serde_json::Value>,
    pub generated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaperOverviewInput {
    pub abstract_text: Option<Option<String>>,
    pub key_concepts: Option<Vec<String>>,
    pub methods: Option<Vec<String>>,
    pub contributions: Option<Vec<String>>,
    pub limitations: Option<Vec<String>>,
    pub tl_dr: Option<Option<String>>,
    pub sections: Option<Vec<PaperSection>>,
    pub metadata: Option<serde_json::Value>,
}

/// 阅读列表
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingList {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReadingListInput {
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReadingListInput {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    pub sort_order: Option<i32>,
}

/// 阅读列表条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingListItem {
    pub id: String,
    pub reading_list_id: String,
    pub document_id: Option<String>,
    pub external_url: Option<String>,
    pub title: String,
    pub notes: Option<String>,
    pub status: String,
    pub priority: i32,
    pub position: i32,
    pub metadata: serde_json::Value,
    pub added_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReadingListItemInput {
    pub reading_list_id: String,
    pub document_id: Option<String>,
    pub external_url: Option<String>,
    pub title: String,
    pub notes: Option<String>,
    pub priority: Option<i32>,
    pub position: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReadingListItemInput {
    pub title: Option<String>,
    pub notes: Option<Option<String>>,
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub position: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}
