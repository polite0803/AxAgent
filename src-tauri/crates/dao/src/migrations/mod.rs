// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned schema migration framework.
//!
//! ## 当前状态
//!
//! 本项目采用「上游基线 + 本地增量」的双层迁移架构：
//! - [v100_consolidated]：上游所有 DDL（表/索引/触发器/种子数据）的单一基线。
//! - [v200_axinvest_stock_tables]：AxInvest fork 独有的股票业务表 + 上游
//!   CHECK 约束扩展。本地独有迁移从 v200 起单调递增，预留 v101–v199 给
//!   上游未来扩展（详见 project_memory.md）。
//!
//! ## 历史
//!
//! - 旧版采用 v001–v011 + v101–v103 + v200 多版本迁移，导致：
//!   - v100 后续追加的 PHASE 在已应用 v100 的旧库上永远不会跑
//!   - v200 INT4→INT8 ALTER 通道与 v100 pg_ddl() 类型转换重复
//!   - 多次数据转换逻辑相互覆盖，难以维护
//! - 现已合并为 v100 单一基线：
//!   - v101_business_roles / v102_mission_hash / v103_workflow_reflections
//!     的字段和表全部合并到 v100 PHASE 2/8/9/10
//!   - v200_pg_int4_to_int8_axinvest 删除（pg_ddl 在 CREATE TABLE 时一次性
//!     产出正确类型，无需二次 ALTER）
//!   - 所有 ALTER TABLE 补字段通道删除（字段直接在 CREATE TABLE 中建好）
//! - AxInvest 独有表（stock_analyses / stock_reflections /
//!   stock_pipeline_runs / strategy_performance）原散落各处，现统一收纳
//!   到 v200_axinvest_stock_tables。上游 v100 保持原貌，合并上游无冲突。
//!
//! ## 约定
//!
//! - 上游表/字段变更：直接修改 v100_consolidated.rs（与上游保持同步）
//! - AxInvest 独有表/字段：在 v200_axinvest_stock_tables.rs 中追加，或
//!   新建 v201/v202/... 递增版本
//! - 新增索引：跟随所属表的迁移文件
//! - 上游表需扩展（如 CHECK 约束加值）：在 v200+ 迁移中用 ALTER 语句扩展

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub mod pg_ddl;
pub mod v100_consolidated;
pub mod v101_consolidate_knowledge_memory;
pub mod v102_create_fleets;
pub mod v103_wiki_graph_perf;
pub mod v104_notes_fts;
pub mod v200_axinvest_stock_tables;
pub mod v201_lesson_application_tracking;
pub mod v202_stock_analyses_parent_version;
pub mod v203_price_alerts_align_monitor;
pub mod v204_paper_portfolio;
pub mod v205_market_mainline;
pub mod v206_screenshot_diagnosis;
pub mod v207_chat_run;

/// 当前 schema 版本号。每次新增 migration 时必须累加此常量。
pub const CURRENT_VERSION: i32 = 207;

/// 迁移函数签名：所有 `up()` 都遵循这个接口。
///
/// `DatabaseConnection` 是 `Arc<DbConnection>` 的 newtype，clone
/// 是引用计数 +1，零拷贝。所以 `up` 接收 owned 是 trivial 的：
/// 调用方在每次 invoke 时 clone 一份即可。
///
/// 用 owned 而非 `&DatabaseConnection` 是为了让 boxed future 不带
/// 借用——`Pin<Box<dyn Future + 'static>>` 可以装进 `const MIGRATIONS`
/// 数组（fn pointer 自身要求 'static）。
///
/// `Send` 是为了让 `run_migrations` 能在 multi-threaded runtime 中
/// 被调用（生产环境 `tokio::main` 默认是 multi_thread）。不需要
/// `Sync`：future 只在 await 期间被一个 task 持有，不存在共享。
pub type MigrationFn =
    fn(
        sea_orm::DatabaseConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), DbErr>> + Send>>;

struct Migration {
    version: i32,
    description: &'static str,
    up: MigrationFn,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 100,
        description: "v100_consolidated: 合并 v001–v011 + v101–v104 的全部 DDL（表/索引/触发器/种子数据），统一用正确类型建表；不再保留旧库类型修复 ALTER 通道",
        up: |db| Box::pin(v100_consolidated::up(db)),
    },
    Migration {
        version: 101,
        description: "v101_consolidate_knowledge_memory: 合并轨迹实体/关系到知识图谱知识实体/关系表，合并轨迹记忆到记忆条目表，删除 trajectory_entities/relationships/memories 旧表",
        up: |db| Box::pin(v101_consolidate_knowledge_memory::up(db)),
    },
    Migration {
        version: 102,
        description: "v102_create_fleets: 创建 fleets / fleet_members 表与索引，承载多办公室 AI 团队的持久化（AgentFleet 集成）",
        up: |db| Box::pin(v102_create_fleets::up(db)),
    },
    Migration {
        version: 103,
        description: "v103_wiki_graph_perf: 给 notes/note_links/note_backlinks 加复合索引（10 万节点查询优化），新增 wiki_graph_cache 表缓存 GraphData+LouvainResult",
        up: |db| Box::pin(v103_wiki_graph_perf::up(db)),
    },
    Migration {
        version: 104,
        description: "v104_notes_fts: 为 notes 表添加全文检索索引（SQLite FTS5 + PostgreSQL tsvector+GIN），解决 wiki_notes_search_keyword 内存 BM25 在 10 万节点下的性能问题",
        up: |db| Box::pin(v104_notes_fts::up(db)),
    },
    Migration {
        version: 200,
        description: "v200_axinvest_stock_tables: AxInvest 独有股票业务表（stock_analyses / stock_reflections / stock_pipeline_runs / strategy_performance）+ agency_experts/agent_profiles 的 category CHECK 约束扩展（加入 stock-analysis）",
        up: |db| Box::pin(v200_axinvest_stock_tables::up(db)),
    },
    Migration {
        version: 201,
        description: "v201_lesson_application_tracking: P2-F15 切入点 3 —— lesson_applications 关联表（记录决策分析引用了哪些 lesson + T+N 验证后回写 outcome，用于精确计算 times_applied/success_count）",
        up: |db| Box::pin(v201_lesson_application_tracking::up(db)),
    },
    Migration {
        version: 202,
        description: "v202_stock_analyses_parent_version: stock_analyses 表新增 parent_analysis_id 字段，支持版本化分析（重跑分析保留历史版本而非覆盖）",
        up: |db| Box::pin(v202_stock_analyses_parent_version::up(db)),
    },
    Migration {
        version: 203,
        description: "v203_price_alerts_align_monitor: price_alerts 表新增 alert_type/condition_type/threshold 列并与 RealtimeMonitor 6 类告警对齐，回填老 above/below 数据",
        up: |db| Box::pin(v203_price_alerts_align_monitor::up(db)),
    },
    Migration {
        version: 204,
        description: "v204_paper_portfolio: G2 模拟观察组合 —— paper_portfolios 主表 + paper_positions 持仓表，承接 DojoAgents 场景 1/2/3 的研究观察列表 / 模拟建仓实体",
        up: |db| Box::pin(v204_paper_portfolio::up(db)),
    },
    Migration {
        version: 205,
        description: "v205_market_mainline: G4 市场主线自动提炼 —— market_mainlines 表，承接 daily-market-events 工作流每日自动提炼的市场主题 + 叙述 + 代表标的 + 强度评分 + 持续性",
        up: |db| Box::pin(v205_market_mainline::up(db)),
    },
    Migration {
        version: 206,
        description: "v206_screenshot_diagnosis: G6 截图持仓诊断完整闭环 —— screenshot_diagnoses 表，承接 screenshot-portfolio-diagnosis 工作流，含 OCR 文本 / 结构化持仓 / 风险诊断 schema / 建议动作",
        up: |db| Box::pin(v206_screenshot_diagnosis::up(db)),
    },
    Migration {
        version: 207,
        description: "v207_chat_run: G8 /api/chat/runs 后台 Run Lifecycle 持久化归档 —— chat_runs 主表 + chat_run_events 事件流表，支持多实例网关共享和历史 run 回放",
        up: |db| Box::pin(v207_chat_run::up(db)),
    },
];

/// 执行所有尚未应用的 schema 迁移。
///
/// 启动时调用；幂等，多次调用结果相同。
///
/// 第一步（建 version tracking 表、读 MAX(version)）使用 `&impl
/// ConnectionTrait`——这是 ConnectionTrait 的稳定接口，ddl.rs shim
/// 可以直接转发。第二步（实际跑 up()）需要 `&DatabaseConnection`，
/// 所以顶层 API 接收 `&DatabaseConnection`；ddl.rs shim 已经更新
/// 成强类型。
pub async fn run_migrations(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    // 1) 确保 version tracking 表存在（ANSI DDL，SQLite/PG 通用）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS axagent_schema_version (\
         version INTEGER NOT NULL PRIMARY KEY, \
         applied_at INTEGER NOT NULL, \
         description TEXT)",
    )
    .await?;

    // 2) 读已应用的最大版本号（首次启动 = 0）
    let applied_max: i32 = read_max_version(db).await?;

    // 3) 按顺序补跑未应用 migration
    for m in MIGRATIONS {
        if m.version <= applied_max {
            continue;
        }
        // db.clone() 是 Arc +1，up() 内部 await 时持有一个 owned 副本。
        (m.up)(db.clone()).await?;
        record_version(db, backend, m.version, m.description).await?;
    }

    Ok(())
}

async fn read_max_version(db: &sea_orm::DatabaseConnection) -> Result<i32, DbErr> {
    let row = db
        .query_one_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT COALESCE(MAX(version), 0) AS v FROM axagent_schema_version",
        ))
        .await?;
    match row {
        None => Ok(0),
        Some(r) => {
            // COALESCE 在空表返回 0，因此总能解析为 i32
            let v: i32 = r.try_get_by("v").unwrap_or(0);
            Ok(v)
        },
    }
}

async fn record_version(
    db: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    version: i32,
    description: &str,
) -> Result<(), DbErr> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);

    // 参数化查询：避免 format! 拼接 SQL 带来的注入风险与转义负担。
    // SQLite 用 `INSERT OR IGNORE`；PostgreSQL 用 `ON CONFLICT DO NOTHING`
    // （二者语义等价：版本号冲突时静默跳过，保证幂等）。
    let stmt = if backend == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO axagent_schema_version (version, applied_at, description) \
             VALUES ($1, $2, $3) ON CONFLICT (version) DO NOTHING",
            [version.into(), now.into(), description.into()],
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT OR IGNORE INTO axagent_schema_version (version, applied_at, description) VALUES (?, ?, ?)",
            [version.into(), now.into(), description.into()],
        )
    };
    db.execute_raw(stmt).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn migrations_apply_cleanly_on_fresh_db() {
        let db = Database::connect("sqlite::memory:").await.expect("in-memory db");
        run_migrations(&db).await.expect("v1-v3 should apply on fresh db");

        // 验证关键表存在
        for table in &[
            "messages",
            "conversations",
            "providers",
            "provider_keys",
            "gateway_keys",
            "gateway_usage",
            "axagent_schema_version",
        ] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                    [(*table).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "table {} should exist", table);
        }

        // 死表应已被 v003 删除
        for dead in &["categories", "apps", "context_packs"] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                    [(*dead).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_none(), "dead table {} should have been dropped", dead);
        }
    }

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("in-memory db");
        run_migrations(&db).await.unwrap();
        // 第二次跑：所有 migration 都在 `applied_max >= m.version` 路径被 skip
        run_migrations(&db).await.expect("second run should be a no-op, not an error");

        let max: i32 = read_max_version(&db).await.unwrap();
        assert_eq!(max, CURRENT_VERSION, "version should be {}", CURRENT_VERSION);

        // schema_version 表应有 13 行（v100 + v101 + v102 + v103 + v104 + v200~v207）
        let count_row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM axagent_schema_version",
            ))
            .await
            .unwrap()
            .expect("count row");
        let cnt: i32 = count_row.try_get_by("cnt").unwrap();
        assert_eq!(
            cnt, 13,
            "schema_version should have 13 rows (v100 + v101 + v102 + v103 + v104 + v200~v207)"
        );
    }

    /// 防回归：v002 引入的索引必须真实存在。
    /// partial index (`idx_messages_branch`) 在 messages.branch_id IS NOT NULL
    /// 命中时使用。
    /// 注：v002 已被合并到 v100_consolidated，索引由 PHASE 4 创建。
    #[tokio::test]
    async fn v002_critical_indices_exist() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        for idx in &[
            "idx_messages_conv_created",
            "idx_conversations_updated",
            "idx_provider_keys_provider",
            "idx_gateway_usage_key",
            "idx_messages_branch",
        ] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "index {} should exist", idx);
        }
    }

    /// v100 consolidated 的 `up` 也应单独 idempotent：单独跑
    /// 一次，重复跑不报错（所有 CREATE 都用 IF NOT EXISTS）。
    #[tokio::test]
    async fn v100_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 不走 run_migrations，直接跑 v100
        v100_consolidated::up(db.clone()).await.unwrap();
        v100_consolidated::up(db).await.expect("v100 must be re-runnable in isolation");
    }

    /// 防回归：v102 创建的 fleets / fleet_members 表与索引必须真实存在。
    ///
    /// 此测试在 SQLite 内存库上验证迁移效果。PostgreSQL 侧由 DDL 的
    /// PG 语法原生支持（BIGINT/TEXT/REFERENCES ON DELETE CASCADE），
    /// CI 集成测试环境会覆盖 PG 路径。
    #[tokio::test]
    async fn v102_fleets_tables_and_indices_exist() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        // 表存在
        for table in &["fleets", "fleet_members"] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                    [(*table).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "table {} should exist after v102", table);
        }

        // 索引存在
        for idx in &[
            "idx_fleet_members_fleet_id",
            "idx_fleet_members_agent_slug",
            "idx_fleet_members_status",
            "idx_fleets_status",
        ] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "index {} should exist after v102", idx);
        }
    }

    /// v102 单独 idempotent：重复跑不报错（所有 CREATE 都用 IF NOT EXISTS）。
    #[tokio::test]
    async fn v102_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        v102_create_fleets::up(db.clone()).await.unwrap();
        v102_create_fleets::up(db).await.expect("v102 must be re-runnable in isolation");
    }

    /// 防回归：v103 创建的索引和 wiki_graph_cache 表必须真实存在。
    #[tokio::test]
    async fn v103_wiki_graph_perf_indices_and_cache_exist() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        // 索引存在
        for idx in &[
            "idx_notes_vault_deleted",
            "idx_note_links_vault_source",
            "idx_note_links_vault_target",
            "idx_note_backlinks_vault_source",
            "idx_note_backlinks_vault_target",
        ] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "index {} should exist after v103", idx);
        }

        // wiki_graph_cache 表存在
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                ["wiki_graph_cache".into()],
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "table wiki_graph_cache should exist after v103");
    }

    /// v103 单独 idempotent：先建表（v100）再重复跑 v103 两次，验证幂等。
    /// v103 依赖 notes/note_links/note_backlinks 表存在，必须先跑 v100。
    #[tokio::test]
    async fn v103_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v100 建 notes/note_links/note_backlinks
        v100_consolidated::up(db.clone()).await.unwrap();
        v103_wiki_graph_perf::up(db.clone()).await.unwrap();
        v103_wiki_graph_perf::up(db).await.expect("v103 must be re-runnable in isolation");
    }

    /// 防回归：v104 创建的 FTS5 虚拟表/触发器必须真实存在（SQLite 路径）。
    #[tokio::test]
    async fn v104_notes_fts_objects_exist() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        // notes_fts 虚拟表存在
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                ["notes_fts".into()],
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "virtual table notes_fts should exist after v104");

        // 触发器存在
        for trig in &["notes_fts_ai", "notes_fts_ad", "notes_fts_au"] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='trigger' AND name=?",
                    [(*trig).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "trigger {} should exist after v104", trig);
        }
    }

    /// v104 单独 idempotent：先建 notes 表（v100）再重复跑 v104 两次，验证幂等。
    /// v104 的 FTS5 触发器依赖 notes 表存在。
    #[tokio::test]
    async fn v104_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        v100_consolidated::up(db.clone()).await.unwrap();
        v104_notes_fts::up(db.clone()).await.unwrap();
        v104_notes_fts::up(db).await.expect("v104 must be re-runnable in isolation");
    }
}
