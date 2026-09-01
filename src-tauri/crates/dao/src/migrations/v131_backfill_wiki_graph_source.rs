// SPDX-License-Identifier: AGPL-3.0-only
//! v131_backfill_wiki_graph_source: 回填 Wiki 实体/关系的 v113 多源来源字段。
//!
//! ## 背景
//!
//! v113 为 knowledge_entities / knowledge_relations 增加了 source_type / source_id
//! 来源字段，但 Wiki 实体抽取路径（extract_entities_from_wiki）此前硬编码
//! `source_type='knowledge_base'`、`source_id=''`，导致 vault 中抽取出的实体
//! 与真实 KB 实体无法区分，统一图谱按来源过滤时会互相混入（R5）。
//!
//! ## 回填规则
//!
//! `knowledge_base_id` 命中 wikis 表的存量实体/关系是 Wiki 抽取产物
//! （Wiki 抽取以 vault_id 充当图谱分区键），回填为
//! `source_type='wiki'`、`source_id=knowledge_base_id`（即 wiki_id）。
//! 新写入路径（batch_upsert_entities_and_relations 的 source 参数）已同步修正。
//!
//! 判定条件附加 `source_type <> 'wiki'`，迁移幂等可重跑。

use sea_orm::{ConnectionTrait, DbErr};

/// 回填 SQL：SQLite / PG 子查询语法通用
const BACKFILL_SQLS: &[&str] = &[
    "UPDATE knowledge_entities SET source_type = 'wiki', source_id = knowledge_base_id \
     WHERE knowledge_base_id IN (SELECT id FROM wikis) AND source_type <> 'wiki'",
    "UPDATE knowledge_relations SET source_type = 'wiki', source_id = knowledge_base_id \
     WHERE knowledge_base_id IN (SELECT id FROM wikis) AND source_type <> 'wiki'",
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("[v131] 回填 Wiki 图谱实体/关系来源字段");

    for sql in BACKFILL_SQLS {
        match db.execute_unprepared(sql).await {
            Ok(_) => {},
            Err(e) => {
                // 极端场景（存量库缺 wikis 表等）：容忍失败不阻断启动，
                // 后续手动抽取/自动抽取的新写入不受影响
                tracing::warn!("[v131] 回填失败（容忍，不阻断启动）: {}", e);
            },
        }
    }

    tracing::info!("[v131] Wiki 图谱来源字段回填完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, DbBackend, Statement};

    #[tokio::test]
    async fn v131_backfills_wiki_entities_only() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        super::super::v113_unified_knowledge_graph::up(db.clone())
            .await
            .expect("测试：异步操作应成功");

        // 建 wikis 表 + 插入两类实体：wiki 产物（kb_id 命中 wikis.id）与真实 KB 实体
        // （wikis 表已由 v100_consolidated 建好，这里只插入数据；
        //   name / root_path / created_at / updated_at 为 NOT NULL 必填列）
        db.execute_unprepared(
            "INSERT INTO wikis (id, name, root_path, created_at, updated_at) \
             VALUES ('wiki-1', 'wiki-1', '/wiki-1', 0, 0)",
        )
        .await
        .expect("测试应成功");

        let insert = |id: &str, kb_id: &str, source_type: &str| -> String {
            format!(
                "INSERT INTO knowledge_entities (id, knowledge_base_id, name, entity_type, \
                     description, source_path, source_language, properties, lifecycle, behaviors, \
                     metadata, created_at, updated_at, aliases, mention_count, confidence, \
                     first_seen_at, last_seen_at, source_type, source_id, node_type, external_id) \
                     VALUES ('{}', '{}', 'n-{}', 'concept', '', '', '', '{{}}', '{{}}', '{{}}', '{{}}', \
                             1700000000, 1700000000, '', 1, 0.5, '', '', '{}', '', 'entity', '')",
                id, kb_id, id, source_type
            )
        };
        db.execute_unprepared(&insert("e1", "wiki-1", "knowledge_base")).await.expect("测试应成功");
        db.execute_unprepared(&insert("e2", "kb-1", "knowledge_base")).await.expect("测试应成功");

        up(db.clone()).await.expect("测试：异步操作应成功");

        let count = async |filter: &str| -> i64 {
            let sql = format!("SELECT COUNT(*) AS cnt FROM knowledge_entities WHERE {}", filter);
            let row = db
                .query_one_raw(Statement::from_string(DbBackend::Sqlite, sql))
                .await
                .expect("测试应成功")
                .expect("row should exist");
            row.try_get_by::<i64, &str>("cnt").expect("测试应成功")
        };

        // wiki 产物回填为 wiki 来源，且 source_id = wiki_id
        assert_eq!(count("id = 'e1' AND source_type = 'wiki' AND source_id = 'wiki-1'").await, 1);
        // 真实 KB 实体不受影响
        assert_eq!(count("id = 'e2' AND source_type = 'knowledge_base'").await, 1);
    }

    #[tokio::test]
    async fn v131_is_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        super::super::v113_unified_knowledge_graph::up(db.clone())
            .await
            .expect("测试：异步操作应成功");
        // wikis 表已由 v100_consolidated 建好，这里只插入数据
        db.execute_unprepared(
            "INSERT INTO wikis (id, name, root_path, created_at, updated_at) \
             VALUES ('wiki-1', 'wiki-1', '/wiki-1', 0, 0)",
        )
        .await
        .expect("测试应成功");

        up(db.clone()).await.expect("第一次执行应成功");
        up(db.clone()).await.expect("第二次执行（幂等）应成功");
    }
}
