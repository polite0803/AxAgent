// SPDX-License-Identifier: AGPL-3.0-only

//! v107: Paper Overview Engine + Reading List & Queue
//!
//! 新增三张表：
//! 1. paper_overviews — 论文/长文档结构化概览（LLM 生成，缓存）
//! 2. reading_lists — 阅读列表（用户收藏的论文/文档集合）
//! 3. reading_list_items — 阅读列表条目（多对一）

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    // 1. paper_overviews 表
    // 字段说明：
    // - id: UUID 主键
    // - document_id: 关联 knowledge_documents.id
    // - knowledge_base_id: 关联 knowledge_bases.id
    // - overview_type: 概览类型（paper / long_document / auto）
    // - abstract_text: 论文摘要
    // - key_concepts: 核心概念 JSON 数组 ["concept1","concept2"]
    // - methods: 方法论 JSON 数组
    // - contributions: 贡献 JSON 数组
    // - limitations: 局限 JSON 数组
    // - tl_dr: 一句话总结
    // - sections: 章节结构 JSON [{title, summary}]
    // - metadata_json: 任意扩展元数据（authors/doi/arxiv_id/published_date 等）
    // - generated_by: 生成模型标识
    // - created_at / updated_at: unix millis
    let sql_paper = if backend == DbBackend::Postgres {
        r#"CREATE TABLE IF NOT EXISTS paper_overviews (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            knowledge_base_id TEXT NOT NULL,
            overview_type TEXT NOT NULL DEFAULT 'auto',
            abstract_text TEXT,
            key_concepts TEXT NOT NULL DEFAULT '[]',
            methods TEXT NOT NULL DEFAULT '[]',
            contributions TEXT NOT NULL DEFAULT '[]',
            limitations TEXT NOT NULL DEFAULT '[]',
            tl_dr TEXT,
            sections TEXT NOT NULL DEFAULT '[]',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            generated_by TEXT,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_paper_overviews_document ON paper_overviews(document_id);
        CREATE INDEX IF NOT EXISTS idx_paper_overviews_kb ON paper_overviews(knowledge_base_id);"#
    } else {
        r#"CREATE TABLE IF NOT EXISTS paper_overviews (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            knowledge_base_id TEXT NOT NULL,
            overview_type TEXT NOT NULL DEFAULT 'auto',
            abstract_text TEXT,
            key_concepts TEXT NOT NULL DEFAULT '[]',
            methods TEXT NOT NULL DEFAULT '[]',
            contributions TEXT NOT NULL DEFAULT '[]',
            limitations TEXT NOT NULL DEFAULT '[]',
            tl_dr TEXT,
            sections TEXT NOT NULL DEFAULT '[]',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            generated_by TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_paper_overviews_document ON paper_overviews(document_id);
        CREATE INDEX IF NOT EXISTS idx_paper_overviews_kb ON paper_overviews(knowledge_base_id);"#
    };
    for stmt in sql_paper.split(';').filter(|s| !s.trim().is_empty()) {
        db.execute_unprepared(stmt).await?;
    }

    // 2. reading_lists 表
    let sql_rl = if backend == DbBackend::Postgres {
        r#"CREATE TABLE IF NOT EXISTS reading_lists (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            owner_user_id TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_reading_lists_owner ON reading_lists(owner_user_id);
        CREATE INDEX IF NOT EXISTS idx_reading_lists_status ON reading_lists(status);"#
    } else {
        r#"CREATE TABLE IF NOT EXISTS reading_lists (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            owner_user_id TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_reading_lists_owner ON reading_lists(owner_user_id);
        CREATE INDEX IF NOT EXISTS idx_reading_lists_status ON reading_lists(status);"#
    };
    for stmt in sql_rl.split(';').filter(|s| !s.trim().is_empty()) {
        db.execute_unprepared(stmt).await?;
    }

    // 3. reading_list_items 表
    // 字段说明：
    // - id: UUID
    // - reading_list_id: 关联 reading_lists.id
    // - document_id: 关联 knowledge_documents.id（允许为空，方便外部链接）
    // - external_url: 外部链接（arxiv URL 等）
    // - title: 条目标题
    // - notes: 用户备注
    // - status: reading_status（unread / reading / read / skipped）
    // - priority: 优先级 0-100，默认 50
    // - position: 在列表中的位置（用于自定义排序）
    // - metadata_json: 任意元数据（authors/published_date 等）
    // - added_at / updated_at: unix millis
    let sql_rli = if backend == DbBackend::Postgres {
        r#"CREATE TABLE IF NOT EXISTS reading_list_items (
            id TEXT PRIMARY KEY,
            reading_list_id TEXT NOT NULL,
            document_id TEXT,
            external_url TEXT,
            title TEXT NOT NULL,
            notes TEXT,
            status TEXT NOT NULL DEFAULT 'unread',
            priority INTEGER NOT NULL DEFAULT 50,
            position INTEGER NOT NULL DEFAULT 0,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            added_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            FOREIGN KEY (reading_list_id) REFERENCES reading_lists(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_reading_list_items_list ON reading_list_items(reading_list_id);
        CREATE INDEX IF NOT EXISTS idx_reading_list_items_status ON reading_list_items(status);
        CREATE INDEX IF NOT EXISTS idx_reading_list_items_doc ON reading_list_items(document_id);"#
    } else {
        r#"CREATE TABLE IF NOT EXISTS reading_list_items (
            id TEXT PRIMARY KEY,
            reading_list_id TEXT NOT NULL,
            document_id TEXT,
            external_url TEXT,
            title TEXT NOT NULL,
            notes TEXT,
            status TEXT NOT NULL DEFAULT 'unread',
            priority INTEGER NOT NULL DEFAULT 50,
            position INTEGER NOT NULL DEFAULT 0,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            added_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (reading_list_id) REFERENCES reading_lists(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_reading_list_items_list ON reading_list_items(reading_list_id);
        CREATE INDEX IF NOT EXISTS idx_reading_list_items_status ON reading_list_items(status);
        CREATE INDEX IF NOT EXISTS idx_reading_list_items_doc ON reading_list_items(document_id);"#
    };
    for stmt in sql_rli.split(';').filter(|s| !s.trim().is_empty()) {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    /// v107 单独 idempotent：重复跑不报错（所有 CREATE 都用 IF NOT EXISTS）。
    #[tokio::test]
    async fn v107_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        up(db.clone()).await.unwrap();
        up(db).await.expect("v107 must be re-runnable in isolation");
    }

    /// v107 创建的表与索引必须真实存在。
    #[tokio::test]
    async fn v107_tables_and_indices_exist() {
        use sea_orm::Statement;
        let db = Database::connect("sqlite::memory:").await.unwrap();
        up(db.clone()).await.unwrap();

        // 表存在
        for table in &["paper_overviews", "reading_lists", "reading_list_items"] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                    [(*table).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "table {} should exist after v107", table);
        }

        // 索引存在
        for idx in &[
            "idx_paper_overviews_document",
            "idx_paper_overviews_kb",
            "idx_reading_lists_owner",
            "idx_reading_lists_status",
            "idx_reading_list_items_list",
            "idx_reading_list_items_status",
            "idx_reading_list_items_doc",
        ] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "index {} should exist after v107", idx);
        }
    }
}
