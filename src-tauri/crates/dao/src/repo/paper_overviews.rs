// SPDX-License-Identifier: AGPL-3.0-only

//! Paper Overview Engine DAO 实现。
//!
//! 提供 paper_overviews 表的 CRUD 与 upsert_by_document 操作。
//! Vec<String> / serde_json::Value 字段在 DB 中以 JSON 字符串存储；
//! 反序列化失败时降级为空数组 / 空对象，保证读取不致整条记录失败。

use sea_orm::*;

use axagent_entities::paper_overviews;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{
    CreatePaperOverviewInput, PaperOverview, PaperSection, UpdatePaperOverviewInput,
};
use axagent_harness::util_fns::gen_id;

/// 当前时间戳（Unix 毫秒）
fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 序列化 Vec<String> 为 JSON 字符串（失败兜底为 "[]"）
fn stringify_str_arr(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|e| {
        tracing::warn!("JSON 数组序列化失败: {e}");
        "[]".to_string()
    })
}

/// 反序列化 JSON 字符串为 Vec<String>（失败/空 → 空 Vec）
fn parse_str_arr(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    serde_json::from_str(raw).unwrap_or_else(|e| {
        tracing::warn!("JSON 数组反序列化失败: {e}");
        Vec::new()
    })
}

/// 序列化 sections 为 JSON 字符串
fn stringify_sections(sections: &[PaperSection]) -> String {
    serde_json::to_string(sections).unwrap_or_else(|e| {
        tracing::warn!("sections 序列化失败: {e}");
        "[]".to_string()
    })
}

/// 反序列化 sections
fn parse_sections(raw: &str) -> Vec<PaperSection> {
    if raw.is_empty() {
        return Vec::new();
    }
    serde_json::from_str(raw).unwrap_or_else(|e| {
        tracing::warn!("sections 反序列化失败: {e}");
        Vec::new()
    })
}

/// 序列化 metadata（None → "{}"）
fn stringify_metadata(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
        None => "{}".to_string(),
    }
}

/// 反序列化 metadata（失败 → 空对象）
fn parse_metadata(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::Object(serde_json::Map::new());
    }
    serde_json::from_str(raw).unwrap_or_else(|e| {
        tracing::warn!("metadata 反序列化失败: {e}");
        serde_json::Value::Object(serde_json::Map::new())
    })
}

/// entity → DTO
fn model_to_overview(m: paper_overviews::Model) -> PaperOverview {
    PaperOverview {
        id: m.id,
        document_id: m.document_id,
        knowledge_base_id: m.knowledge_base_id,
        overview_type: m.overview_type,
        abstract_text: m.abstract_text,
        key_concepts: parse_str_arr(&m.key_concepts),
        methods: parse_str_arr(&m.methods),
        contributions: parse_str_arr(&m.contributions),
        limitations: parse_str_arr(&m.limitations),
        tl_dr: m.tl_dr,
        sections: parse_sections(&m.sections),
        metadata: parse_metadata(&m.metadata_json),
        generated_by: m.generated_by,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

/// 按 KB 列出全部概览
pub async fn list_by_kb(db: &DatabaseConnection, kb_id: &str) -> Result<Vec<PaperOverview>> {
    let rows = paper_overviews::Entity::find()
        .filter(paper_overviews::Column::KnowledgeBaseId.eq(kb_id))
        .order_by_desc(paper_overviews::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(model_to_overview).collect())
}

/// 按文档列出（理论上每个 document 最多一个 overview，仍返回 Vec 兜底）
pub async fn list_by_document(
    db: &DatabaseConnection,
    document_id: &str,
) -> Result<Vec<PaperOverview>> {
    let rows = paper_overviews::Entity::find()
        .filter(paper_overviews::Column::DocumentId.eq(document_id))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(model_to_overview).collect())
}

/// 按 ID 获取
pub async fn get(db: &DatabaseConnection, id: &str) -> Result<PaperOverview> {
    let m = paper_overviews::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PaperOverview {}", id)))?;
    Ok(model_to_overview(m))
}

/// 按 document_id 获取（每个 document 默认最多一个 overview）
pub async fn get_by_document(
    db: &DatabaseConnection,
    document_id: &str,
) -> Result<Option<PaperOverview>> {
    let m = paper_overviews::Entity::find()
        .filter(paper_overviews::Column::DocumentId.eq(document_id))
        .one(db)
        .await?;
    Ok(m.map(model_to_overview))
}

/// 创建
pub async fn create(
    db: &DatabaseConnection,
    input: CreatePaperOverviewInput,
) -> Result<PaperOverview> {
    let id = gen_id();
    let now = now_millis();
    let overview_type = input.overview_type.unwrap_or_else(|| "auto".to_string());

    let am = paper_overviews::ActiveModel {
        id: Set(id.clone()),
        document_id: Set(input.document_id),
        knowledge_base_id: Set(input.knowledge_base_id),
        overview_type: Set(overview_type),
        abstract_text: Set(input.abstract_text),
        key_concepts: Set(stringify_str_arr(&input.key_concepts)),
        methods: Set(stringify_str_arr(&input.methods)),
        contributions: Set(stringify_str_arr(&input.contributions)),
        limitations: Set(stringify_str_arr(&input.limitations)),
        tl_dr: Set(input.tl_dr),
        sections: Set(stringify_sections(&input.sections)),
        metadata_json: Set(stringify_metadata(&input.metadata)),
        generated_by: Set(input.generated_by),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    get(db, &id).await
}

/// 更新
pub async fn update(
    db: &DatabaseConnection,
    id: &str,
    input: UpdatePaperOverviewInput,
) -> Result<PaperOverview> {
    let m = paper_overviews::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PaperOverview {}", id)))?;

    let mut am: paper_overviews::ActiveModel = m.into();
    am.updated_at = Set(now_millis());

    if let Some(v) = input.abstract_text {
        am.abstract_text = Set(v);
    }
    if let Some(v) = input.key_concepts {
        am.key_concepts = Set(stringify_str_arr(&v));
    }
    if let Some(v) = input.methods {
        am.methods = Set(stringify_str_arr(&v));
    }
    if let Some(v) = input.contributions {
        am.contributions = Set(stringify_str_arr(&v));
    }
    if let Some(v) = input.limitations {
        am.limitations = Set(stringify_str_arr(&v));
    }
    if let Some(v) = input.tl_dr {
        am.tl_dr = Set(v);
    }
    if let Some(v) = input.sections {
        am.sections = Set(stringify_sections(&v));
    }
    if let Some(v) = input.metadata {
        am.metadata_json = Set(stringify_metadata(&Some(v)));
    }

    am.update(db).await?;
    get(db, id).await
}

/// 按 document_id upsert（存在则更新，不存在则创建）
pub async fn upsert_by_document(
    db: &DatabaseConnection,
    input: CreatePaperOverviewInput,
) -> Result<PaperOverview> {
    let existing = paper_overviews::Entity::find()
        .filter(paper_overviews::Column::DocumentId.eq(input.document_id.clone()))
        .one(db)
        .await?;

    if let Some(m) = existing {
        // 已存在 → 更新
        let id = m.id.clone();
        let update_input = UpdatePaperOverviewInput {
            abstract_text: Some(input.abstract_text),
            key_concepts: Some(input.key_concepts),
            methods: Some(input.methods),
            contributions: Some(input.contributions),
            limitations: Some(input.limitations),
            tl_dr: Some(input.tl_dr),
            sections: Some(input.sections),
            metadata: Some(
                input.metadata.unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            ),
        };
        // 同时更新 overview_type / generated_by（update 不支持，单独处理）
        let mut am: paper_overviews::ActiveModel = m.into();
        am.updated_at = Set(now_millis());
        if let Some(t) = input.overview_type {
            am.overview_type = Set(t);
        }
        am.generated_by = Set(input.generated_by);
        am.update(db).await?;
        return update(db, &id, update_input).await;
    }

    // 不存在 → 创建
    create(db, input).await
}

/// 删除
pub async fn delete(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = paper_overviews::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("PaperOverview {}", id)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input(doc_id: &str, kb_id: &str) -> CreatePaperOverviewInput {
        CreatePaperOverviewInput {
            document_id: doc_id.to_string(),
            knowledge_base_id: kb_id.to_string(),
            overview_type: Some("paper".to_string()),
            abstract_text: Some("这是摘要".to_string()),
            key_concepts: vec!["概念A".to_string(), "概念B".to_string()],
            methods: vec!["方法1".to_string()],
            contributions: vec!["贡献1".to_string()],
            limitations: vec!["局限1".to_string()],
            tl_dr: Some("一句话总结".to_string()),
            sections: vec![PaperSection {
                title: "引言".to_string(),
                summary: "背景介绍".to_string(),
            }],
            metadata: Some(serde_json::json!({"authors": ["张三"]})),
            generated_by: Some("gpt-4o".to_string()),
        }
    }

    #[tokio::test]
    async fn crud_round_trip() {
        use crate::migrations::v107_paper_reading_list as v107;
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        v107::up(db.clone()).await.unwrap();

        // create
        let created = create(&db, sample_input("doc1", "kb1")).await.unwrap();
        assert_eq!(created.document_id, "doc1");
        assert_eq!(created.key_concepts.len(), 2);
        assert_eq!(created.sections.len(), 1);
        assert_eq!(created.metadata["authors"][0], "张三");

        // get
        let fetched = get(&db, &created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);

        // get_by_document
        let by_doc = get_by_document(&db, "doc1").await.unwrap().unwrap();
        assert_eq!(by_doc.id, created.id);

        // list_by_kb
        let list = list_by_kb(&db, "kb1").await.unwrap();
        assert_eq!(list.len(), 1);

        // update
        let updated = update(
            &db,
            &created.id,
            UpdatePaperOverviewInput {
                abstract_text: Some(Some("更新后的摘要".to_string())),
                key_concepts: Some(vec!["新概念".to_string()]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.abstract_text.as_deref(), Some("更新后的摘要"));
        assert_eq!(updated.key_concepts, vec!["新概念".to_string()]);
        // 未更新字段保持
        assert_eq!(updated.sections.len(), 1);

        // delete
        delete(&db, &created.id).await.unwrap();
        assert!(get(&db, &created.id).await.is_err());
    }

    #[tokio::test]
    async fn upsert_by_document_creates_then_updates() {
        use crate::migrations::v107_paper_reading_list as v107;
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        v107::up(db.clone()).await.unwrap();

        // 首次 upsert → 创建
        let v1 = upsert_by_document(&db, sample_input("doc2", "kb2")).await.unwrap();
        assert_eq!(v1.document_id, "doc2");

        // 第二次 upsert 同 document → 更新
        let mut input2 = sample_input("doc2", "kb2");
        input2.abstract_text = Some("更新后的摘要".to_string());
        input2.overview_type = Some("long_document".to_string());
        let v2 = upsert_by_document(&db, input2).await.unwrap();
        assert_eq!(v2.id, v1.id, "upsert 应保持同一 ID");
        assert_eq!(v2.abstract_text.as_deref(), Some("更新后的摘要"));
        assert_eq!(v2.overview_type, "long_document");

        // 仍只有一条记录
        let list = list_by_kb(&db, "kb2").await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_not_found() {
        use crate::migrations::v107_paper_reading_list as v107;
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        v107::up(db.clone()).await.unwrap();

        let err = delete(&db, "nonexistent").await.unwrap_err();
        match err {
            AxAgentError::NotFound(_) => {},
            other => panic!("expected NotFound, got {other:?}"),
        }
    }
}
