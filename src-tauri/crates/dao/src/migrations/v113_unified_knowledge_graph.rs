// SPDX-License-Identifier: AGPL-3.0-only
//! v113_unified_knowledge_graph: 扩展知识图谱表支持多源节点。
//!
//! ## 背景
//!
//! 原有知识图谱（knowledge_entities / knowledge_relations）仅支持 RAG 知识库实体。
//! 为实现统一图谱存储，需要扩展以支持：
//! - wiki note（Wiki 笔记节点）
//! - memory item（记忆条目节点）
//! - KB entity（知识库实体节点，原有）
//! - Obsidian note（Obsidian Vault 笔记节点）
//!
//! ## 新增字段
//!
//! knowledge_entities 表新增：
//! - `source_type`: 节点来源类型（'knowledge_base' / 'wiki' / 'memory' / 'obsidian_vault'）
//! - `source_id`: 源 ID（kb_id / wiki_id / namespace_id / vault_id）
//! - `node_type`: 节点类型（'entity' / 'note' / 'memory_item' / 'obsidian_note'）
//! - `external_id`: 外部系统 ID（如 wiki note_id / memory_id / obsidian note path）
//!
//! knowledge_relations 表新增：
//! - `source_type`: 关系来源类型
//! - `source_id`: 关系源 ID

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// knowledge_entities 新增列
const ENTITY_NEW_COLUMNS: &[(&str, &str)] = &[
    ("source_type", "TEXT NOT NULL DEFAULT 'knowledge_base'"),
    ("source_id", "TEXT NOT NULL DEFAULT ''"),
    ("node_type", "TEXT NOT NULL DEFAULT 'entity'"),
    ("external_id", "TEXT"),
];

/// knowledge_relations 新增列
const RELATION_NEW_COLUMNS: &[(&str, &str)] = &[
    ("source_type", "TEXT NOT NULL DEFAULT 'knowledge_base'"),
    ("source_id", "TEXT NOT NULL DEFAULT ''"),
];

/// 检查表是否存在
async fn table_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    is_pg: bool,
) -> Result<bool, DbErr> {
    let sql = if is_pg {
        format!(
            "SELECT COUNT(*) AS cnt FROM information_schema.tables WHERE table_name = '{}'",
            table
        )
    } else {
        format!("SELECT COUNT(*) AS cnt FROM sqlite_master WHERE type='table' AND name='{}'", table)
    };
    let row = db.query_one_raw(Statement::from_string(db.get_database_backend(), sql)).await?;
    match row {
        Some(r) => {
            let cnt: i64 = r.try_get_by("cnt").unwrap_or(0);
            Ok(cnt > 0)
        },
        None => Ok(false),
    }
}

/// 检查列是否存在
async fn column_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    column: &str,
    is_pg: bool,
) -> Result<bool, DbErr> {
    let sql = if is_pg {
        format!(
            "SELECT COUNT(*) AS cnt FROM information_schema.columns \
             WHERE table_name = '{}' AND column_name = '{}'",
            table, column
        )
    } else {
        format!(
            "SELECT COUNT(*) AS cnt FROM pragma_table_info('{}') WHERE name = '{}'",
            table, column
        )
    };
    let row = db.query_one_raw(Statement::from_string(db.get_database_backend(), sql)).await?;
    match row {
        Some(r) => {
            let cnt: i64 = r.try_get_by("cnt").unwrap_or(0);
            Ok(cnt > 0)
        },
        None => Ok(false),
    }
}

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    tracing::info!("[v113] 开始扩展知识图谱支持多源节点 (is_pg={})", is_pg);

    // ── 扩展 knowledge_entities 表 ──
    let entities_table = "knowledge_entities";
    if !table_exists(&db, entities_table, is_pg).await? {
        tracing::warn!("[v113] 表 {} 不存在，跳过", entities_table);
    } else {
        for (col, def) in ENTITY_NEW_COLUMNS {
            if column_exists(&db, entities_table, col, is_pg).await? {
                tracing::debug!("[v113] 列 {}.{} 已存在，跳过", entities_table, col);
                continue;
            }
            let sql = if is_pg {
                format!("ALTER TABLE {} ADD COLUMN IF NOT EXISTS {} {}", entities_table, col, def)
            } else {
                format!("ALTER TABLE {} ADD COLUMN {} {}", entities_table, col, def)
            };
            tracing::info!("[v113] 执行: {}", sql);
            match db.execute_unprepared(&sql).await {
                Ok(_) => {},
                Err(e) => {
                    tracing::warn!("[v113] 添加列 {}.{} 失败: {}", entities_table, col, e);
                },
            }
        }

        // 为新字段添加索引
        let index_sqls = [
            "CREATE INDEX IF NOT EXISTS idx_entities_source_type ON knowledge_entities(source_type)",
            "CREATE INDEX IF NOT EXISTS idx_entities_source_id ON knowledge_entities(source_id)",
            "CREATE INDEX IF NOT EXISTS idx_entities_node_type ON knowledge_entities(node_type)",
            "CREATE INDEX IF NOT EXISTS idx_entities_external_id ON knowledge_entities(external_id)",
        ];
        for sql in &index_sqls {
            if let Err(e) = db.execute_unprepared(sql).await {
                tracing::warn!("[v113] 建索引失败（不阻塞）: {} — {}", sql, e);
            }
        }
    }

    // ── 扩展 knowledge_relations 表 ──
    let relations_table = "knowledge_relations";
    if !table_exists(&db, relations_table, is_pg).await? {
        tracing::warn!("[v113] 表 {} 不存在，跳过", relations_table);
    } else {
        for (col, def) in RELATION_NEW_COLUMNS {
            if column_exists(&db, relations_table, col, is_pg).await? {
                tracing::debug!("[v113] 列 {}.{} 已存在，跳过", relations_table, col);
                continue;
            }
            let sql = if is_pg {
                format!("ALTER TABLE {} ADD COLUMN IF NOT EXISTS {} {}", relations_table, col, def)
            } else {
                format!("ALTER TABLE {} ADD COLUMN {} {}", relations_table, col, def)
            };
            tracing::info!("[v113] 执行: {}", sql);
            match db.execute_unprepared(&sql).await {
                Ok(_) => {},
                Err(e) => {
                    tracing::warn!("[v113] 添加列 {}.{} 失败: {}", relations_table, col, e);
                },
            }
        }

        // 为新字段添加索引
        let index_sqls = [
            "CREATE INDEX IF NOT EXISTS idx_relations_source_type ON knowledge_relations(source_type)",
            "CREATE INDEX IF NOT EXISTS idx_relations_source_id ON knowledge_relations(source_id)",
        ];
        for sql in &index_sqls {
            if let Err(e) = db.execute_unprepared(sql).await {
                tracing::warn!("[v113] 建索引失败（不阻塞）: {} — {}", sql, e);
            }
        }
    }

    tracing::info!("[v113] 知识图谱多源扩展完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn v113_adds_new_columns_to_entities() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        // 先跑 v100 建表
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        // 验证新列存在
        for col in &["source_type", "source_id", "node_type", "external_id"] {
            let result = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT {} FROM knowledge_entities LIMIT 0", col),
                ))
                .await;
            assert!(result.is_ok(), "column {} should exist in knowledge_entities", col);
        }
    }

    #[tokio::test]
    async fn v113_adds_new_columns_to_relations() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        for col in &["source_type", "source_id"] {
            let result = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT {} FROM knowledge_relations LIMIT 0", col),
                ))
                .await;
            assert!(result.is_ok(), "column {} should exist in knowledge_relations", col);
        }
    }

    #[tokio::test]
    async fn v113_is_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        // 第二次跑：所有列已存在，应跳过不报错
        up(db.clone()).await.expect("v113 must be re-runnable without error");
    }

    #[tokio::test]
    async fn v113_can_insert_multi_source_entities() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        // 插入不同来源的实体
        let entities = vec![
            // RAG KB entity（原有类型）
            ("ent1", "kb1", "knowledge_base", "entity", "RAG Entity", None::<String>),
            // Wiki note
            ("ent2", "wiki1", "wiki", "note", "Wiki Note", Some("wiki_note_001".to_string())),
            // Memory item
            ("ent3", "ns1", "memory", "memory_item", "Memory Item", Some("mem_001".to_string())),
            // Obsidian note
            (
                "ent4",
                "vault1",
                "obsidian_vault",
                "obsidian_note",
                "Obsidian Note",
                Some("notes/concepts.md".to_string()),
            ),
        ];

        for (id, kb_id, source_type, node_type, name, ext_id) in &entities {
            let ext_id_str = ext_id.as_deref().unwrap_or("");
            db.execute_unprepared(&format!(
                "INSERT INTO knowledge_entities (id, knowledge_base_id, name, entity_type, \
                 description, source_path, source_language, properties, lifecycle, behaviors, \
                 metadata, created_at, updated_at, aliases, mention_count, confidence, \
                 first_seen_at, last_seen_at, source_type, source_id, node_type, external_id) \
                 VALUES ('{}', '{}', '{}', 'concept', '', '', '', '{{}}', '{{}}', '{{}}', '{{}}', \
                         1700000000, 1700000000, '', 0, 1.0, '', '', '{}', '{}', '{}', '{}')",
                id, kb_id, name, source_type, ext_id_str, node_type, ext_id_str
            ))
            .await
            .expect("测试应成功");
        }

        // 验证可按 source_type 查询
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM knowledge_entities WHERE source_type = 'wiki'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let cnt: i64 = row.try_get_by("cnt").expect("测试应成功");
        assert_eq!(cnt, 1, "should find 1 wiki entity");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM knowledge_entities WHERE node_type = 'memory_item'"
                    .to_string(),
            ))
            .await
            .expect("测试应成功")
            .expect("row should exist");
        let cnt: i64 = row.try_get_by("cnt").expect("测试应成功");
        assert_eq!(cnt, 1, "should find 1 memory item entity");
    }
}
