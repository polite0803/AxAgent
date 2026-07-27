// SPDX-License-Identifier: AGPL-3.0-only
//! Wiki 图谱缓存：持久化 GraphData + LouvainResult 到 `wiki_graph_cache` 表。
//!
//! ## 设计
//!
//! 10 万节点规模下，每次 `get_wiki_graph` 三次 DB 扫描 + 内存聚合，
//! `wiki_graph_communities` 还要跑 Louvain 算法，单次数秒到数十秒。
//! 缓存到独立表后，前端读取直接命中缓存（< 10ms）。
//!
//! ## 失效策略
//!
//! `notes` 表有写入/更新/删除时，调用方应调用 `invalidate_cache(vault_id)`
//! 清除对应 vault 的缓存。`updated_at` 字段用于手动判断缓存新鲜度。

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

use axagent_harness::graph_dtos::GraphData;
use axagent_harness::louvain_dtos::LouvainResult;

/// 缓存命中的图谱数据 + 社区检测结果（社区可能为 None 表示未计算）。
pub struct GraphCacheEntry {
    pub graph_data: GraphData,
    pub communities: Option<LouvainResult>,
    pub computed_at: i64,
}

/// 读取缓存的图谱数据。未命中返回 Ok(None)。
pub async fn get_cached_graph(
    db: &DatabaseConnection,
    vault_id: &str,
) -> Result<Option<GraphCacheEntry>, DbErr> {
    let backend = db.get_database_backend();
    let stmt = if backend == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT graph_data_json, communities_json, computed_at FROM wiki_graph_cache WHERE vault_id = $1",
            [vault_id.into()],
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT graph_data_json, communities_json, computed_at FROM wiki_graph_cache WHERE vault_id = ?",
            [vault_id.into()],
        )
    };

    let row = match db.query_one_raw(stmt).await? {
        Some(r) => r,
        None => return Ok(None),
    };

    let graph_data_json: String = row.try_get_by("graph_data_json")?;
    let communities_json: Option<String> = row.try_get_by("communities_json").ok().flatten();
    let computed_at: i64 = row.try_get_by("computed_at").unwrap_or(0);

    let graph_data: GraphData = serde_json::from_str(&graph_data_json)
        .map_err(|e| DbErr::Custom(format!("反序列化 graph_data 失败: {e}")))?;

    let communities = match communities_json {
        Some(json) if !json.is_empty() => serde_json::from_str::<LouvainResult>(&json).ok(),
        _ => None,
    };

    Ok(Some(GraphCacheEntry { graph_data, communities, computed_at }))
}

/// 写入/更新缓存。如果 communities 为 None，保留原有 communities（若存在）。
pub async fn save_cached_graph(
    db: &DatabaseConnection,
    vault_id: &str,
    graph_data: &GraphData,
    communities: Option<&LouvainResult>,
) -> Result<(), DbErr> {
    let now = chrono::Utc::now().timestamp();
    let graph_data_json = serde_json::to_string(graph_data)
        .map_err(|e| DbErr::Custom(format!("序列化 graph_data 失败: {e}")))?;
    let communities_json = match communities {
        Some(c) => serde_json::to_string(c).ok(),
        None => None,
    };
    let node_count = graph_data.nodes.len() as i32;
    let edge_count = graph_data.edges.len() as i32;

    let backend = db.get_database_backend();
    // UPSERT：PG 用 ON CONFLICT，SQLite 用 INSERT OR REPLACE
    if backend == DbBackend::Postgres {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO wiki_graph_cache (vault_id, graph_data_json, communities_json, node_count, edge_count, computed_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $6) \
             ON CONFLICT (vault_id) DO UPDATE SET \
             graph_data_json = EXCLUDED.graph_data_json, \
             communities_json = COALESCE(EXCLUDED.communities_json, wiki_graph_cache.communities_json), \
             node_count = EXCLUDED.node_count, \
             edge_count = EXCLUDED.edge_count, \
             computed_at = EXCLUDED.computed_at, \
             updated_at = EXCLUDED.updated_at",
            [
                vault_id.into(),
                graph_data_json.into(),
                communities_json.into(),
                node_count.into(),
                edge_count.into(),
                now.into(),
            ],
        );
        db.execute_raw(stmt).await?;
    } else {
        // SQLite：INSERT OR REPLACE 会覆盖 communities_json（即使传 NULL），
        // 所以分两步：先尝试 UPDATE，无行 affected 再 INSERT
        let update_stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "UPDATE wiki_graph_cache SET graph_data_json = ?, communities_json = COALESCE(?, communities_json), node_count = ?, edge_count = ?, computed_at = ?, updated_at = ? WHERE vault_id = ?",
            [
                graph_data_json.clone().into(),
                communities_json.clone().into(),
                node_count.into(),
                edge_count.into(),
                now.into(),
                now.into(),
                vault_id.into(),
            ],
        );
        let result = db.execute_raw(update_stmt).await?;
        if result.rows_affected() == 0 {
            let insert_stmt = Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO wiki_graph_cache (vault_id, graph_data_json, communities_json, node_count, edge_count, computed_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                [
                    vault_id.into(),
                    graph_data_json.into(),
                    communities_json.into(),
                    node_count.into(),
                    edge_count.into(),
                    now.into(),
                    now.into(),
                ],
            );
            db.execute_raw(insert_stmt).await?;
        }
    }

    Ok(())
}

/// 仅更新社区检测结果（图谱数据已缓存，只补算社区）。
pub async fn save_cached_communities(
    db: &DatabaseConnection,
    vault_id: &str,
    communities: &LouvainResult,
) -> Result<(), DbErr> {
    let now = chrono::Utc::now().timestamp();
    let communities_json = serde_json::to_string(communities)
        .map_err(|e| DbErr::Custom(format!("序列化 communities 失败: {e}")))?;

    let backend = db.get_database_backend();
    let stmt = if backend == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE wiki_graph_cache SET communities_json = $1, updated_at = $2 WHERE vault_id = $3",
            [communities_json.into(), now.into(), vault_id.into()],
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "UPDATE wiki_graph_cache SET communities_json = ?, updated_at = ? WHERE vault_id = ?",
            [communities_json.into(), now.into(), vault_id.into()],
        )
    };
    db.execute_raw(stmt).await?;
    Ok(())
}

/// 失效缓存：notes 表有写入/更新/删除时调用。
pub async fn invalidate_cache(db: &DatabaseConnection, vault_id: &str) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let stmt = if backend == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM wiki_graph_cache WHERE vault_id = $1",
            [vault_id.into()],
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "DELETE FROM wiki_graph_cache WHERE vault_id = ?",
            [vault_id.into()],
        )
    };
    db.execute_raw(stmt).await?;
    Ok(())
}
