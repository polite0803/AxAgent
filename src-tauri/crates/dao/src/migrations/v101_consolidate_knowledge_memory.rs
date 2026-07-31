// SPDX-License-Identifier: AGPL-3.0-only
//! v101: Consolidate knowledge & memory — merge trajectory entities/relationships/memories
//! into the canonical knowledge_entities/knowledge_relations/memory_items tables.
//!
//! ## Background
//!
//! Before v101, there were two parallel knowledge graph systems:
//!   - `knowledge_entities` / `knowledge_relations` (document-level, KB-scoped)
//!   - `trajectory_entities` / `trajectory_relationships` (conversation-level, global)
//!
//! ...and two parallel memory systems:
//!   - `memory_items` (flat, RAG-retrievable)
//!   - `trajectory_memories` (4-tier hierarchical, decay/promotion)
//!
//! This migration merges them: trajectory data is migrated into the canonical tables,
//! and the trajectory-specific tables are dropped.
//!
//! ## Strategy
//!
//! - A system knowledge base `__sys_trajectory__` is created to host trajectory-derived entities.
//! - New columns are added to `knowledge_entities` / `knowledge_relations` / `memory_items`.
//! - Old data is copied, then old tables and indexes are dropped.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub use super::pg_ddl::exec_ddl;

/// Sentinel KB ID for trajectory-derived entities.
pub const TRAJECTORY_KB_ID: &str = "__sys_trajectory__";
/// Sentinel namespace ID for trajectory-derived memories.
pub const TRAJECTORY_MEM_NS_ID: &str = "__sys_trajectory_memory__";
/// Sentinel namespace name for trajectory-derived memories.
pub const TRAJECTORY_MEM_NS_NAME: &str = "System Memory (Trajectory)";

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: Create the sentinel knowledge base (for trajectory entities)
    // ========================================================================
    // We need this row to exist before we can INSERT into knowledge_entities
    // with this knowledge_base_id (FK constraint).
    // 用于 PHASE 6 数据迁移时缺失时间戳的兜底值（毫秒）。
    let now = chrono::Utc::now().timestamp_millis();

    // 注意：knowledge_bases 表（v100 DDL 与 entity 权威定义）不包含 created_at/updated_at 列，
    // 因此 sentinel 行不能引用这两列，否则会报 "no such column: created_at"。
    if is_pg {
        db.execute_unprepared(&format!(
            "INSERT INTO knowledge_bases (id, name, description, enabled) \
             VALUES ('{}', 'System Trajectory Entities', 'Auto-extracted entities from conversation trajectories', \
             FALSE) \
             ON CONFLICT (id) DO NOTHING",
            TRAJECTORY_KB_ID
        )).await?;
    } else {
        db.execute_unprepared(&format!(
            "INSERT OR IGNORE INTO knowledge_bases (id, name, description, enabled) \
             VALUES ('{}', 'System Trajectory Entities', 'Auto-extracted entities from conversation trajectories', \
             0)",
            TRAJECTORY_KB_ID
        )).await?;
    }

    // ========================================================================
    // PHASE 2: Add trajectory-specific columns to knowledge_entities
    // ========================================================================
    // Fields being added: aliases, mention_count, confidence, first_seen_at, last_seen_at

    if is_pg {
        for col in &[
            ("aliases", "TEXT NOT NULL DEFAULT '[]'"),
            ("mention_count", "INTEGER NOT NULL DEFAULT 1"),
            ("confidence", "DOUBLE PRECISION NOT NULL DEFAULT 0.5"),
            ("first_seen_at", "TEXT"),
            ("last_seen_at", "TEXT"),
        ] {
            db.execute_unprepared(&format!(
                "ALTER TABLE knowledge_entities ADD COLUMN IF NOT EXISTS {} {}",
                col.0, col.1
            ))
            .await?;
        }
    } else {
        // SQLite: ALTER TABLE 已由 v100 PHASE 3.9 全表合规检查统一保障，
        // 此处保留仅为向后兼容（纯文档性注释，实际由 v100 提前补列）。

        // 即使 v100 提前补了列，此处仍执行 ALTER TABLE 确保幂等——
        // v100 的 CREATE TABLE IF NOT EXISTS 对新库已包含这些列，
        // 对存量库 v100 PHASE 3.9 已兜底，此处 ADD COLUMN 遇到重复列
        // 报错被 `let _` 忽略。
        for col in &[
            "ALTER TABLE knowledge_entities ADD COLUMN aliases TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE knowledge_entities ADD COLUMN mention_count INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE knowledge_entities ADD COLUMN confidence REAL NOT NULL DEFAULT 0.5",
            "ALTER TABLE knowledge_entities ADD COLUMN first_seen_at TEXT",
            "ALTER TABLE knowledge_entities ADD COLUMN last_seen_at TEXT",
        ] {
            let _ = db.execute_unprepared(col).await;
        }
    }

    // ========================================================================
    // PHASE 3: Add `weight` column to knowledge_relations
    // ========================================================================

    if is_pg {
        db.execute_unprepared(
            "ALTER TABLE knowledge_relations ADD COLUMN IF NOT EXISTS weight DOUBLE PRECISION NOT NULL DEFAULT 1.0"
        ).await?;
    } else {
        // weight 列已由 v100 PHASE 3.9 统一保障，此处保留 ADD COLUMN 仅用于
        // 历史数据库幂等（重复列错误由 `let _` 忽略）。
        let _ = db
            .execute_unprepared(
                "ALTER TABLE knowledge_relations ADD COLUMN weight REAL NOT NULL DEFAULT 1.0",
            )
            .await;
    }

    // ========================================================================
    // PHASE 4: Add trajectory memory fields to memory_items
    // ========================================================================
    // Fields being added: tier, importance, access_count, last_accessed,
    // decay_rate, expires_at, source_conversation_id, source_message_id,
    // memory_nature, tags

    let mem_cols: &[(&str, &str)] = &[
        ("tier", "TEXT NOT NULL DEFAULT 'working'"),
        ("importance", "DOUBLE PRECISION NOT NULL DEFAULT 0.5"),
        ("access_count", "INTEGER NOT NULL DEFAULT 0"),
        ("last_accessed", "BIGINT"),
        ("decay_rate", "DOUBLE PRECISION NOT NULL DEFAULT 0.01"),
        ("expires_at", "BIGINT"),
        ("source_conversation_id", "TEXT"),
        ("source_message_id", "TEXT"),
        ("memory_nature", "TEXT NOT NULL DEFAULT 'semantic'"),
        ("tags", "TEXT NOT NULL DEFAULT '[]'"),
    ];

    if is_pg {
        for col in mem_cols {
            db.execute_unprepared(&format!(
                "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS {} {}",
                col.0, col.1
            ))
            .await?;
        }
    } else {
        for (name, _dtype) in mem_cols {
            // 所有 10 列已由 v100 PHASE 3.9 统一保障，此处保留 ALTER TABLE
            // 仅用于历史数据库幂等（重复列错误由 `let _` 忽略）。
            let sql = match *name {
                "tier" => {
                    "ALTER TABLE memory_items ADD COLUMN tier TEXT NOT NULL DEFAULT 'working'"
                },
                "importance" => {
                    "ALTER TABLE memory_items ADD COLUMN importance REAL NOT NULL DEFAULT 0.5"
                },
                "access_count" => {
                    "ALTER TABLE memory_items ADD COLUMN access_count INTEGER NOT NULL DEFAULT 0"
                },
                "last_accessed" => "ALTER TABLE memory_items ADD COLUMN last_accessed BIGINT",
                "decay_rate" => {
                    "ALTER TABLE memory_items ADD COLUMN decay_rate REAL NOT NULL DEFAULT 0.01"
                },
                "expires_at" => "ALTER TABLE memory_items ADD COLUMN expires_at BIGINT",
                "source_conversation_id" => {
                    "ALTER TABLE memory_items ADD COLUMN source_conversation_id TEXT"
                },
                "source_message_id" => "ALTER TABLE memory_items ADD COLUMN source_message_id TEXT",
                "memory_nature" => {
                    "ALTER TABLE memory_items ADD COLUMN memory_nature TEXT NOT NULL DEFAULT 'semantic'"
                },
                "tags" => "ALTER TABLE memory_items ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'",
                _ => unreachable!(),
            };
            let _ = db.execute_unprepared(sql).await;
        }
    }

    // ========================================================================
    // PHASE 5: Create sentinel memory namespace for trajectory memories
    // ========================================================================

    if is_pg {
        db.execute_unprepared(&format!(
            "INSERT INTO memory_namespaces (id, name, scope, sort_order) \
             VALUES ('{}', '{}', 'system', 0) \
             ON CONFLICT (id) DO NOTHING",
            TRAJECTORY_MEM_NS_ID, TRAJECTORY_MEM_NS_NAME
        ))
        .await?;
    } else {
        db.execute_unprepared(&format!(
            "INSERT OR IGNORE INTO memory_namespaces (id, name, scope, sort_order) \
             VALUES ('{}', '{}', 'system', 0)",
            TRAJECTORY_MEM_NS_ID, TRAJECTORY_MEM_NS_NAME
        ))
        .await?;
    }

    // ========================================================================
    // PHASE 6: Migrate data from trajectory_entities → knowledge_entities
    // ========================================================================
    // Only run migration if the source table exists and has rows.
    let has_traj_entities = table_has_rows(&db, "trajectory_entities", is_pg).await?;
    if has_traj_entities {
        if is_pg {
            db.execute_unprepared(&format!(
                "INSERT INTO knowledge_entities \
                 (id, knowledge_base_id, name, entity_type, description, source_path, \
                  source_language, properties, lifecycle, behaviors, metadata, \
                  created_at, updated_at, \
                  aliases, mention_count, confidence, first_seen_at, last_seen_at) \
                 SELECT \
                  te.id, '{}', te.name, te.entity_type, \
                  NULL AS description, '' AS source_path, \
                  NULL AS source_language, \
                  COALESCE(te.properties, '{{}}'::text) AS properties, \
                  NULL AS lifecycle, NULL AS behaviors, NULL AS metadata, \
                  COALESCE(EXTRACT(EPOCH FROM te.created_at::timestamp)::bigint * 1000, {}) AS created_at, \
                  COALESCE(EXTRACT(EPOCH FROM te.last_seen_at::timestamp)::bigint * 1000, {}) AS updated_at, \
                  te.aliases, te.mention_count, te.confidence, \
                  te.first_seen_at, te.last_seen_at \
                 FROM trajectory_entities te \
                 ON CONFLICT (id) DO NOTHING",
                TRAJECTORY_KB_ID, now, now
            )).await?;
        } else {
            db.execute_unprepared(&format!(
                "INSERT OR IGNORE INTO knowledge_entities \
                 (id, knowledge_base_id, name, entity_type, description, source_path, \
                  source_language, properties, lifecycle, behaviors, metadata, \
                  created_at, updated_at, \
                  aliases, mention_count, confidence, first_seen_at, last_seen_at) \
                 SELECT \
                  te.id, '{}', te.name, te.entity_type, \
                  NULL, '', \
                  NULL, \
                  COALESCE(te.properties, '{{}}'), \
                  NULL, NULL, NULL, \
                  {}, {}, \
                  te.aliases, te.mention_count, te.confidence, \
                  te.first_seen_at, te.last_seen_at \
                 FROM trajectory_entities te",
                TRAJECTORY_KB_ID, now, now
            ))
            .await?;
        }
    }

    // ========================================================================
    // PHASE 7: Migrate data from trajectory_relationships → knowledge_relations
    // ========================================================================

    let has_traj_rels = table_has_rows(&db, "trajectory_relationships", is_pg).await?;
    if has_traj_rels {
        if is_pg {
            db.execute_unprepared(&format!(
                "INSERT INTO knowledge_relations \
                 (id, knowledge_base_id, source_entity_id, target_entity_id, \
                  relation_type, description, properties, metadata, \
                  created_at, updated_at, weight) \
                 SELECT \
                  tr.id, '{}', tr.source_id, tr.target_id, \
                  tr.relation_type, NULL AS description, \
                  COALESCE(tr.properties, '{{}}'::text)::jsonb AS properties, \
                  NULL AS metadata, \
                  COALESCE(EXTRACT(EPOCH FROM tr.created_at::timestamp)::bigint * 1000, {}) AS created_at, \
                  COALESCE(EXTRACT(EPOCH FROM tr.created_at::timestamp)::bigint * 1000, {}) AS updated_at, \
                  tr.weight \
                 FROM trajectory_relationships tr \
                 ON CONFLICT (id) DO NOTHING",
                TRAJECTORY_KB_ID, now, now
            )).await?;
        } else {
            db.execute_unprepared(&format!(
                "INSERT OR IGNORE INTO knowledge_relations \
                 (id, knowledge_base_id, source_entity_id, target_entity_id, \
                  relation_type, description, properties, metadata, \
                  created_at, updated_at, weight) \
                 SELECT \
                  tr.id, '{}', tr.source_id, tr.target_id, \
                  tr.relation_type, NULL, \
                  tr.properties, NULL, \
                  {}, {}, \
                  tr.weight \
                 FROM trajectory_relationships tr",
                TRAJECTORY_KB_ID, now, now
            ))
            .await?;
        }
    }

    // ========================================================================
    // PHASE 8: Migrate data from trajectory_memories → memory_items
    // ========================================================================

    let has_traj_mems = table_has_rows(&db, "trajectory_memories", is_pg).await?;
    if has_traj_mems {
        if is_pg {
            db.execute_unprepared(&format!(
                "INSERT INTO memory_items \
                 (id, namespace_id, title, content, source, index_status, updated_at, \
                  tier, importance, access_count, last_accessed, decay_rate, expires_at, \
                  source_conversation_id, source_message_id, memory_nature, tags) \
                 SELECT \
                  tm.id, '{}', \
                  COALESCE(tm.memory_type, 'memory') AS title, \
                  tm.content, \
                  COALESCE(tm.source_conversation_id, 'trajectory') AS source, \
                  'ready' AS index_status, \
                  to_char(to_timestamp(tm.updated_at / 1000) AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS updated_at, \
                  tm.tier, tm.importance, tm.access_count, tm.last_accessed, \
                  tm.decay_rate, tm.expires_at, \
                  tm.source_conversation_id, tm.source_message_id, \
                  tm.memory_nature, tm.tags \
                 FROM trajectory_memories tm \
                 ON CONFLICT (id) DO NOTHING",
                TRAJECTORY_MEM_NS_ID
            )).await?;
        } else {
            db.execute_unprepared(&format!(
                "INSERT OR IGNORE INTO memory_items \
                 (id, namespace_id, title, content, source, index_status, updated_at, \
                  tier, importance, access_count, last_accessed, decay_rate, expires_at, \
                  source_conversation_id, source_message_id, memory_nature, tags) \
                 SELECT \
                  tm.id, '{}', \
                  COALESCE(tm.memory_type, 'memory'), \
                  tm.content, \
                  COALESCE(tm.source_conversation_id, 'trajectory'), \
                  'ready', \
                  tm.updated_at, \
                  tm.tier, tm.importance, tm.access_count, tm.last_accessed, \
                  tm.decay_rate, tm.expires_at, \
                  tm.source_conversation_id, tm.source_message_id, \
                  tm.memory_nature, tm.tags \
                 FROM trajectory_memories tm",
                TRAJECTORY_MEM_NS_ID
            ))
            .await?;
        }
    }

    // ========================================================================
    // PHASE 9: Drop old trajectory tables (entities, relationships, memories)
    // ========================================================================

    // Drop FTS virtual tables first (SQLite only)
    if !is_pg {
        for ftstable in &["trajectory_memories_fts"] {
            let _ = db.execute_unprepared(&format!("DROP TABLE IF EXISTS {}", ftstable)).await;
        }
    }

    // Drop PG-specific objects
    if is_pg {
        for obj in &[
            "DROP INDEX IF EXISTS idx_traj_entities_type",
            "DROP INDEX IF EXISTS idx_traj_entities_name",
            "DROP INDEX IF EXISTS idx_traj_rel_source",
            "DROP INDEX IF EXISTS idx_traj_rel_target",
            "DROP INDEX IF EXISTS idx_traj_memories_type",
            "DROP INDEX IF EXISTS idx_traj_memories_tier",
            "DROP INDEX IF EXISTS idx_traj_memories_importance",
            "DROP INDEX IF EXISTS idx_traj_memories_expires",
            "DROP INDEX IF EXISTS idx_traj_memories_namespace",
            "DROP INDEX IF EXISTS idx_traj_memories_tsv",
        ] {
            db.execute_unprepared(obj).await?;
        }
    }

    // Drop tables (works on both SQLite and PG)
    for table in &["trajectory_entities", "trajectory_relationships", "trajectory_memories"] {
        db.execute_unprepared(&format!("DROP TABLE IF EXISTS {}", table)).await?;
    }

    // ========================================================================
    // PHASE 10: Create replacement indexes on canonical tables
    // ========================================================================

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_knowledge_entities_name ON knowledge_entities(name)",
        "CREATE INDEX IF NOT EXISTS idx_knowledge_entities_type ON knowledge_entities(entity_type)",
        "CREATE INDEX IF NOT EXISTS idx_knowledge_relations_source ON knowledge_relations(source_entity_id)",
        "CREATE INDEX IF NOT EXISTS idx_knowledge_relations_target ON knowledge_relations(target_entity_id)",
        "CREATE INDEX IF NOT EXISTS idx_memory_items_tier ON memory_items(tier)",
        "CREATE INDEX IF NOT EXISTS idx_memory_items_importance ON memory_items(importance)",
        "CREATE INDEX IF NOT EXISTS idx_memory_items_namespace ON memory_items(namespace_id)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}

/// Check if a table exists and has at least one row.
async fn table_has_rows(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    is_pg: bool,
) -> Result<bool, DbErr> {
    let sql = if is_pg {
        format!(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '{}') AS has",
            table
        )
    } else {
        format!("SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='{}'", table)
    };
    let exists = db.query_one_raw(Statement::from_string(db.get_database_backend(), sql)).await?;
    if exists.is_none() {
        return Ok(false);
    }
    // Check rows
    let row = db
        .query_one_raw(Statement::from_string(
            db.get_database_backend(),
            format!("SELECT COUNT(*) AS cnt FROM {}", table),
        ))
        .await;
    match row {
        Ok(Some(r)) => {
            let cnt: i64 = r.try_get_by("cnt").unwrap_or(0);
            Ok(cnt > 0)
        },
        _ => Ok(false),
    }
}
