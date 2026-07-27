// SPDX-License-Identifier: AGPL-3.0-only
//! v100_consolidated: 合并 v001–v011 所有历史迁移，统一修正列类型
//!
//! ## 背景
//!
//! 历史问题：v001–v009 的 DDL 中大量时间戳列和数字列使用 `INTEGER` (INT4)，
//! 但 SeaORM entity 声明为 `i64`/`Option<i64>` → PG 下强类型检查报错：
//!
//!   `Rust type core::option::Option<i64> (as SQL type INT8)
//!    is not compatible with SQL type INT4`
//!
//! v010/v011 只修补了部分旧表，仍有 67+ 对列遗漏。
//!
//! ## 策略
//!
//! 本 migration 一劳永逸：
//!   - DDL 直接写 PostgreSQL 语法（BIGINT/DOUBLE PRECISION/BOOLEAN/BIGSERIAL），
//!     PG 下 CREATE TABLE 直接产出正确类型。
//!   - SQLite 侧通过 [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 仅做 3 条确定性
//!     替换（BIGSERIAL→INTEGER AUTOINCREMENT, SERIAL→INTEGER, to_char→datetime），
//!     其他类型 SQLite 动态亲和性自动兼容。
//!   - 新实例：CREATE TABLE 直接产出正确类型。
//!   - 旧实例：CREATE TABLE IF NOT EXISTS 是 no-op，类型由原库保留。
//!   - SQLite：全部 no-op（动态类型无此问题）。
//!
//! ## 替代
//!
//! 本 migration 取代 v001–v011。历史文件保留仅作参考。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub use super::pg_ddl::exec_ddl;

// ============================================================================
// 缺失字段目标列表：v100 之前的旧库（v001–v011）已存在 agent_roles /
// agency_experts 表，而 `CREATE TABLE IF NOT EXISTS` 对已有表是 no-op，
// **不会补后加字段**。entity 中这些字段为 `Option<String>`（TEXT，可空），
// 旧库缺失会导致 SeaORM 报「字段不存在」。
//
// 两种后端都需 ALTER ADD COLUMN：
//   - PostgreSQL：使用 `ADD COLUMN IF NOT EXISTS`，幂等且不会在已存在的列上报错。
//   - SQLite：普通 `ADD COLUMN`，忽略「重复列」错误（let _ = ...）。
//
// 注意：PHASE 2 已用 CREATE TABLE IF NOT EXISTS 确保表存在，故此处 ALTER 时
// 表一定存在；新库在 PHASE 2 已带上这些列，本阶段对它们为 no-op。
// ============================================================================

const MISSING_COLUMN_TARGETS: &[(&str, &str, &str)] = &[
    ("agent_roles", "active_domains", "TEXT"),
    ("agency_experts", "recommended_workflows", "TEXT"),
    ("agency_experts", "recommended_tools", "TEXT"),
    ("agency_experts", "active_domains", "TEXT"),
    ("wiki_sync_queue", "created_at", "BIGINT"),
    ("wiki_sync_queue", "processed_at", "BIGINT"),
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: Drop dead tables（来自 v003）
    // ========================================================================

    for sql in &[
        "DROP TABLE IF EXISTS categories",
        "DROP TABLE IF EXISTS apps",
        "DROP TABLE IF EXISTS context_packs",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // PHASE 2: 创建全部表（合并 v001 + v004–v009）
    //   DDL 直接写 PG 语法，exec_ddl 在 SQLite 下自动转换 BIGSERIAL/to_char。
    //   SQLite 动态类型亲和性接受 BIGINT/DOUBLE PRECISION/BOOLEAN/TEXT。
    // ========================================================================

    // --- Section A: Core tables（来自 v001） ---

    for sql in &[
        // providers
        "CREATE TABLE IF NOT EXISTS providers (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, provider_type TEXT NOT NULL, \
            api_host TEXT NOT NULL, api_path TEXT, enabled INTEGER NOT NULL DEFAULT 1, \
            proxy_config TEXT, sort_order INTEGER NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            custom_headers TEXT, icon TEXT, builtin_id TEXT)",
        // provider_keys
        "CREATE TABLE IF NOT EXISTS provider_keys (\
            id TEXT NOT NULL PRIMARY KEY, provider_id TEXT NOT NULL, \
            key_encrypted TEXT NOT NULL, key_prefix TEXT NOT NULL DEFAULT '', \
            enabled INTEGER NOT NULL DEFAULT 1, last_validated_at BIGINT, last_error TEXT, \
            rotation_index INTEGER NOT NULL DEFAULT 0, created_at BIGINT NOT NULL, \
            FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE)",
        // models (composite PK)
        "CREATE TABLE IF NOT EXISTS models (\
            provider_id TEXT NOT NULL, model_id TEXT NOT NULL, name TEXT NOT NULL, \
            capabilities TEXT NOT NULL DEFAULT '[]', max_tokens BIGINT, \
            enabled INTEGER NOT NULL DEFAULT 1, param_overrides TEXT, \
            model_type TEXT NOT NULL DEFAULT 'chat', group_name TEXT, \
            input_price_per_mtok DOUBLE PRECISION, output_price_per_mtok DOUBLE PRECISION, \
            PRIMARY KEY (provider_id, model_id), \
            FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE)",
        // conversations
        "CREATE TABLE IF NOT EXISTS conversations (\
            id TEXT NOT NULL PRIMARY KEY, title TEXT NOT NULL, model_id TEXT NOT NULL, \
            provider_id TEXT NOT NULL, system_prompt TEXT, temperature DOUBLE PRECISION, \
            max_tokens BIGINT, top_p DOUBLE PRECISION, frequency_penalty DOUBLE PRECISION, \
            message_count INTEGER NOT NULL DEFAULT 0, is_pinned INTEGER NOT NULL DEFAULT 0, \
            is_archived INTEGER NOT NULL DEFAULT 0, \
            workspace_snapshot_json TEXT NOT NULL DEFAULT '{}', \
            active_branch_id TEXT, active_artifact_id TEXT, \
            research_mode INTEGER NOT NULL DEFAULT 0, search_enabled INTEGER NOT NULL DEFAULT 0, \
            search_provider_id TEXT, thinking_budget BIGINT, \
            enabled_mcp_server_ids TEXT NOT NULL DEFAULT '[]', \
            enabled_knowledge_base_ids TEXT NOT NULL DEFAULT '[]', \
            enabled_memory_namespace_ids TEXT NOT NULL DEFAULT '[]', \
            enabled_wiki_ids TEXT NOT NULL DEFAULT '[]', agent_profile_id TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            context_compression INTEGER NOT NULL DEFAULT 0, category_id TEXT, \
            parent_conversation_id TEXT, mode TEXT NOT NULL DEFAULT 'chat', \
            work_strategy TEXT, scenario TEXT, \
            enabled_skill_ids TEXT NOT NULL DEFAULT '[]', \
            workflow_template_id TEXT, session_type TEXT NOT NULL DEFAULT 'conversation', \
            workflow_status TEXT, \
            memory_status TEXT NOT NULL DEFAULT 'none', last_memory_extracted_at TEXT)",
        // messages
        "CREATE TABLE IF NOT EXISTS messages (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, role TEXT NOT NULL, \
            content TEXT NOT NULL, provider_id TEXT, model_id TEXT, token_count BIGINT, \
            attachments TEXT NOT NULL DEFAULT '[]', thinking TEXT, parent_message_id TEXT, \
            version_index INTEGER NOT NULL DEFAULT 0, is_active INTEGER NOT NULL DEFAULT 1, \
            branch_id TEXT, tool_calls_json TEXT, tool_call_id TEXT, \
            created_at BIGINT NOT NULL, parts TEXT, prompt_tokens BIGINT, \
            completion_tokens BIGINT, status TEXT NOT NULL DEFAULT 'complete', \
            tokens_per_second DOUBLE PRECISION, first_token_latency_ms BIGINT, \
            quoted_message_id TEXT, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // messages cache token 列（ALTER ADD COLUMN 兼容已有表）
    for sql in &[
        "ALTER TABLE messages ADD COLUMN cache_creation_tokens BIGINT",
        "ALTER TABLE messages ADD COLUMN cache_read_tokens BIGINT",
        // 引用回复字段：v101 新增，旧库需 ALTER 补列；新库 CREATE TABLE 已包含此列
        "ALTER TABLE messages ADD COLUMN quoted_message_id TEXT",
    ] {
        let _ = db.execute_unprepared(sql).await;
    }

    // gateway_keys / gateway_usage
    for sql in &[
        "CREATE TABLE IF NOT EXISTS gateway_keys (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            key_hash TEXT NOT NULL UNIQUE, key_prefix TEXT NOT NULL, encrypted_key TEXT, \
            enabled INTEGER NOT NULL DEFAULT 1, created_at BIGINT NOT NULL, last_used_at BIGINT)",
        "CREATE TABLE IF NOT EXISTS gateway_usage (\
            id BIGSERIAL PRIMARY KEY, key_id TEXT NOT NULL, \
            provider_id TEXT NOT NULL, model_id TEXT, \
            request_tokens BIGINT NOT NULL DEFAULT 0, response_tokens BIGINT NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, \
            FOREIGN KEY (key_id) REFERENCES gateway_keys(id) ON DELETE CASCADE)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }
    let _ = db
        .execute_unprepared(
            "ALTER TABLE gateway_usage ADD COLUMN cached_input_tokens BIGINT NOT NULL DEFAULT 0",
        )
        .await;

    // 网关用量成本估算列：record_usage 时根据 ModelPricing 换算的美元成本。
    // SQLite REAL 等价于 f64；历史行回填 0.0，新行由 dao 写入实际估算值。
    let _ = db
        .execute_unprepared("ALTER TABLE gateway_usage ADD COLUMN cost REAL NOT NULL DEFAULT 0.0")
        .await;

    for sql in &[
        // settings
        "CREATE TABLE IF NOT EXISTS settings (\
            key TEXT NOT NULL PRIMARY KEY, value TEXT NOT NULL)",
        // search_providers
        "CREATE TABLE IF NOT EXISTS search_providers (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            provider_type TEXT NOT NULL DEFAULT 'tavily', endpoint TEXT, api_key_ref TEXT, \
            enabled INTEGER NOT NULL DEFAULT 1, region TEXT, language TEXT, safe_search BIGINT, \
            result_limit INTEGER NOT NULL DEFAULT 10, timeout_ms INTEGER NOT NULL DEFAULT 5000)",
        // search_citations
        "CREATE TABLE IF NOT EXISTS search_citations (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            message_id TEXT NOT NULL, title TEXT NOT NULL, url TEXT NOT NULL, snippet TEXT, \
            provider_id TEXT NOT NULL, rank INTEGER NOT NULL DEFAULT 0, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // mcp_servers
        "CREATE TABLE IF NOT EXISTS mcp_servers (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, alias TEXT, description TEXT, \
            transport TEXT NOT NULL DEFAULT 'stdio', command TEXT, args_json TEXT, endpoint TEXT, \
            env_json TEXT, enabled INTEGER NOT NULL DEFAULT 1, \
            permission_policy TEXT NOT NULL DEFAULT 'ask', source TEXT NOT NULL DEFAULT 'custom', \
            discover_timeout_secs INTEGER, execute_timeout_secs INTEGER, headers_json TEXT, \
            icon_type TEXT, icon_value TEXT)",
        // tool_descriptors
        "CREATE TABLE IF NOT EXISTS tool_descriptors (\
            id TEXT NOT NULL PRIMARY KEY, server_id TEXT NOT NULL, name TEXT NOT NULL, \
            description TEXT, input_schema_json TEXT, \
            FOREIGN KEY (server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE)",
        // tool_executions
        "CREATE TABLE IF NOT EXISTS tool_executions (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, message_id TEXT, \
            server_id TEXT NOT NULL, tool_name TEXT NOT NULL, \
            status TEXT NOT NULL DEFAULT 'pending', input_preview TEXT, output_preview TEXT, \
            error_message TEXT, duration_ms BIGINT, approval_status TEXT, \
            skill_steps_json TEXT, depends_on TEXT, \
            created_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')), \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // knowledge_bases
        "CREATE TABLE IF NOT EXISTS knowledge_bases (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            embedding_provider TEXT, enabled INTEGER NOT NULL DEFAULT 1, \
            icon_type TEXT, icon_value TEXT, sort_order INTEGER NOT NULL DEFAULT 0, \
            embedding_dimensions INTEGER, retrieval_threshold REAL, retrieval_top_k INTEGER, \
            chunk_size INTEGER, chunk_overlap INTEGER, separator TEXT)",
        // knowledge_documents
        "CREATE TABLE IF NOT EXISTS knowledge_documents (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, title TEXT NOT NULL, \
            source_path TEXT NOT NULL, mime_type TEXT NOT NULL, \
            size_bytes BIGINT NOT NULL DEFAULT 0, \
            indexing_status TEXT NOT NULL DEFAULT 'pending', doc_type TEXT NOT NULL DEFAULT '', \
            index_error TEXT, source_conversation_id TEXT, \
            created_at BIGINT NOT NULL DEFAULT 0, updated_at BIGINT NOT NULL DEFAULT 0, \
            FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE)",
        // retrieval_hits
        "CREATE TABLE IF NOT EXISTS retrieval_hits (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, message_id TEXT NOT NULL, \
            knowledge_base_id TEXT NOT NULL, document_id TEXT NOT NULL, chunk_ref TEXT NOT NULL, \
            score DOUBLE PRECISION NOT NULL DEFAULT 0.0, preview TEXT NOT NULL, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE, \
            FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE)",
        // memory_namespaces
        "CREATE TABLE IF NOT EXISTS memory_namespaces (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            scope TEXT NOT NULL DEFAULT 'global', embedding_provider TEXT, \
            embedding_dimensions INTEGER, retrieval_threshold REAL, retrieval_top_k INTEGER, \
            icon_type TEXT, icon_value TEXT, sort_order INTEGER NOT NULL DEFAULT 0)",
        // memory_items
        "CREATE TABLE IF NOT EXISTS memory_items (\
            id TEXT NOT NULL PRIMARY KEY, namespace_id TEXT NOT NULL, title TEXT NOT NULL, \
            content TEXT NOT NULL, source TEXT NOT NULL DEFAULT 'manual', \
            index_status TEXT NOT NULL DEFAULT 'pending', index_error TEXT, \
            updated_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')), \
            FOREIGN KEY (namespace_id) REFERENCES memory_namespaces(id) ON DELETE CASCADE)",
        // artifacts
        "CREATE TABLE IF NOT EXISTS artifacts (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            kind TEXT NOT NULL DEFAULT 'draft', title TEXT NOT NULL, \
            content TEXT NOT NULL DEFAULT '', format TEXT NOT NULL DEFAULT 'markdown', \
            pinned INTEGER NOT NULL DEFAULT 0, \
            updated_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')), \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // context_sources
        "CREATE TABLE IF NOT EXISTS context_sources (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, message_id TEXT, \
            source_type TEXT NOT NULL, ref_id TEXT NOT NULL, title TEXT NOT NULL, \
            enabled INTEGER NOT NULL DEFAULT 1, summary TEXT, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // conversation_branches
        "CREATE TABLE IF NOT EXISTS conversation_branches (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            parent_message_id TEXT NOT NULL, branch_label TEXT NOT NULL, \
            branch_index INTEGER NOT NULL DEFAULT 0, compared_message_ids_json TEXT, \
            created_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')), \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // backup_manifests
        "CREATE TABLE IF NOT EXISTS backup_manifests (\
            id TEXT NOT NULL PRIMARY KEY, version TEXT NOT NULL, \
            created_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')), \
            encrypted INTEGER NOT NULL DEFAULT 0, checksum TEXT NOT NULL, \
            object_counts_json TEXT NOT NULL DEFAULT '{}', source_app_version TEXT NOT NULL, \
            file_path TEXT, file_size BIGINT NOT NULL DEFAULT 0)",
        // backup_targets
        "CREATE TABLE IF NOT EXISTS backup_targets (\
            id TEXT NOT NULL PRIMARY KEY, kind TEXT NOT NULL DEFAULT 'local', \
            config_json TEXT NOT NULL DEFAULT '{}')",
        // import_jobs
        "CREATE TABLE IF NOT EXISTS import_jobs (\
            id TEXT NOT NULL PRIMARY KEY, source_type TEXT NOT NULL, \
            status TEXT NOT NULL DEFAULT 'scanning', summary_json TEXT, \
            conflict_count INTEGER NOT NULL DEFAULT 0, \
            created_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')))",
        // program_policies
        "CREATE TABLE IF NOT EXISTS program_policies (\
            id TEXT NOT NULL PRIMARY KEY, program_name TEXT NOT NULL UNIQUE, \
            allowed_provider_ids_json TEXT NOT NULL DEFAULT '[]', \
            allowed_model_ids_json TEXT NOT NULL DEFAULT '[]', \
            default_provider_id TEXT, default_model_id TEXT, rate_limit_per_minute BIGINT)",
        // gateway_diagnostics
        "CREATE TABLE IF NOT EXISTS gateway_diagnostics (\
            id TEXT NOT NULL PRIMARY KEY, category TEXT NOT NULL, \
            status TEXT NOT NULL DEFAULT 'ok', message TEXT NOT NULL, \
            created_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')))",
        // desktop_state
        "CREATE TABLE IF NOT EXISTS desktop_state (\
            window_key TEXT NOT NULL PRIMARY KEY, width INTEGER NOT NULL DEFAULT 1200, \
            height INTEGER NOT NULL DEFAULT 800, x BIGINT, y BIGINT, \
            maximized INTEGER NOT NULL DEFAULT 0, visible INTEGER NOT NULL DEFAULT 1)",
        // stored_files
        "CREATE TABLE IF NOT EXISTS stored_files (\
            id TEXT NOT NULL PRIMARY KEY, hash TEXT NOT NULL, original_name TEXT NOT NULL, \
            mime_type TEXT NOT NULL DEFAULT 'application/octet-stream', \
            size_bytes BIGINT NOT NULL, storage_path TEXT NOT NULL, conversation_id TEXT, \
            created_at TEXT NOT NULL DEFAULT (to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')), \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL)",
        // gateway_request_logs
        "CREATE TABLE IF NOT EXISTS gateway_request_logs (\
            id TEXT NOT NULL PRIMARY KEY, key_id TEXT NOT NULL, key_name TEXT NOT NULL, \
            method TEXT NOT NULL, path TEXT NOT NULL, model TEXT, provider_id TEXT, \
            status_code INTEGER NOT NULL, duration_ms BIGINT NOT NULL, \
            request_tokens BIGINT NOT NULL DEFAULT 0, response_tokens BIGINT NOT NULL DEFAULT 0, \
            error_message TEXT, created_at BIGINT NOT NULL)",
        // conversation_summaries
        "CREATE TABLE IF NOT EXISTS conversation_summaries (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            summary_text TEXT NOT NULL, compressed_until_message_id TEXT, \
            token_count BIGINT, model_used TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // conversation_categories
        "CREATE TABLE IF NOT EXISTS conversation_categories (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            icon_type TEXT, icon_value TEXT, system_prompt TEXT, \
            default_provider_id TEXT, default_model_id TEXT, \
            default_temperature DOUBLE PRECISION, default_max_tokens BIGINT, \
            default_top_p DOUBLE PRECISION, default_frequency_penalty DOUBLE PRECISION, \
            sort_order INTEGER NOT NULL DEFAULT 0, is_collapsed INTEGER NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // skill_states
        "CREATE TABLE IF NOT EXISTS skill_states (\
            name TEXT NOT NULL PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 0, \
            updated_at BIGINT NOT NULL)",
        // agent_sessions
        "CREATE TABLE IF NOT EXISTS agent_sessions (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, cwd TEXT, \
            workspace_locked INTEGER NOT NULL DEFAULT 0, permission_mode TEXT NOT NULL, \
            runtime_status TEXT NOT NULL, sdk_context_json TEXT, \
            sdk_context_backup_json TEXT, total_tokens BIGINT NOT NULL DEFAULT 0, \
            total_cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0.0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // wikis
        "CREATE TABLE IF NOT EXISTS wikis (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, root_path TEXT NOT NULL, \
            schema_version TEXT NOT NULL DEFAULT '1.0', description TEXT, \
            note_count INTEGER NOT NULL DEFAULT 0, source_count INTEGER NOT NULL DEFAULT 0, \
            embedding_provider TEXT, embedding_dimensions INTEGER, \
            retrieval_threshold REAL, retrieval_top_k INTEGER, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // wiki_sources
        "CREATE TABLE IF NOT EXISTS wiki_sources (\
            id TEXT NOT NULL PRIMARY KEY, wiki_id TEXT NOT NULL, source_type TEXT NOT NULL, \
            source_path TEXT NOT NULL, title TEXT NOT NULL, mime_type TEXT NOT NULL, \
            size_bytes BIGINT NOT NULL, content_hash TEXT NOT NULL, metadata_json TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // wiki_pages
        "CREATE TABLE IF NOT EXISTS wiki_pages (\
            id TEXT NOT NULL PRIMARY KEY, wiki_id TEXT NOT NULL, note_id TEXT NOT NULL, \
            page_type TEXT NOT NULL, title TEXT NOT NULL, source_ids TEXT, \
            quality_score DOUBLE PRECISION, last_linted_at BIGINT, last_compiled_at BIGINT NOT NULL, \
            compiled_source_hash TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // wiki_operations
        "CREATE TABLE IF NOT EXISTS wiki_operations (\
            id BIGSERIAL PRIMARY KEY, wiki_id TEXT NOT NULL, \
            operation_type TEXT NOT NULL, target_type TEXT NOT NULL, target_id TEXT NOT NULL, \
            status TEXT NOT NULL, details_json TEXT, error_message TEXT, \
            created_at BIGINT NOT NULL, completed_at BIGINT, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // wiki_sync_queue
        "CREATE TABLE IF NOT EXISTS wiki_sync_queue (\
            id BIGSERIAL PRIMARY KEY, wiki_id TEXT NOT NULL, \
            event_type TEXT NOT NULL, target_type TEXT NOT NULL, target_id TEXT NOT NULL, \
            payload TEXT, status TEXT NOT NULL DEFAULT 'pending', \
            retry_count INTEGER NOT NULL DEFAULT 0, error_message TEXT, \
            created_at BIGINT NOT NULL, processed_at BIGINT, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // note_links
        "CREATE TABLE IF NOT EXISTS note_links (\
            id BIGSERIAL PRIMARY KEY, vault_id TEXT NOT NULL, \
            source_note_id TEXT NOT NULL, target_note_id TEXT NOT NULL, link_text TEXT, \
            link_type TEXT NOT NULL, created_at BIGINT NOT NULL)",
        // note_backlinks
        "CREATE TABLE IF NOT EXISTS note_backlinks (\
            id BIGSERIAL PRIMARY KEY, vault_id TEXT NOT NULL, \
            source_note_id TEXT NOT NULL, target_note_id TEXT NOT NULL, link_text TEXT, \
            link_type TEXT NOT NULL, created_at BIGINT NOT NULL)",
        // plans
        "CREATE TABLE IF NOT EXISTS plans (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            user_message_id TEXT NOT NULL, title TEXT NOT NULL, \
            steps_json TEXT NOT NULL DEFAULT '[]', status TEXT NOT NULL DEFAULT 'draft', \
            is_active INTEGER NOT NULL DEFAULT 1, created_under_strategy TEXT, reason TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // agency_experts
        // P2-8: category 加 CHECK 约束（新部署生效；存量库靠应用层 validate_category 兜底）
        "CREATE TABLE IF NOT EXISTS agency_experts (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            category TEXT NOT NULL CHECK (category IN ('general','development','security','data','finance','devops','design','writing','business')), \
            system_prompt TEXT NOT NULL, color TEXT, \
            source_dir TEXT NOT NULL, is_enabled INTEGER NOT NULL DEFAULT 1, \
            imported_at BIGINT NOT NULL, recommended_workflows TEXT, recommended_tools TEXT, \
            active_domains TEXT)",
        // agent_profiles
        // P2-8: category 加 CHECK 约束
        // P1-5: expert_id 加 FK → agency_experts(id) ON DELETE SET NULL（专家被删除时 profile 保留）
        "CREATE TABLE IF NOT EXISTS agent_profiles (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            category TEXT NOT NULL DEFAULT 'general' CHECK (category IN ('general','development','security','data','finance','devops','design','writing','business')), \
            icon TEXT NOT NULL DEFAULT '🤖', \
            agent_role TEXT, \
            source TEXT NOT NULL DEFAULT 'builtin', tags TEXT, \
            suggested_provider_id TEXT, suggested_model_id TEXT, \
            suggested_temperature DOUBLE PRECISION, suggested_max_tokens BIGINT, \
            search_enabled BOOLEAN, recommend_permission_mode TEXT, \
            recommended_tools TEXT, disallowed_tools TEXT, recommended_workflows TEXT, \
            sort_order INTEGER NOT NULL DEFAULT 0, is_enabled INTEGER NOT NULL DEFAULT 1, \
            expert_id TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (expert_id) REFERENCES agency_experts(id) ON DELETE SET NULL)",
        // agent_roles
        "CREATE TABLE IF NOT EXISTS agent_roles (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            system_prompt TEXT NOT NULL DEFAULT '', default_tools TEXT, \
            max_concurrent INTEGER NOT NULL DEFAULT 3, \
            timeout_seconds BIGINT NOT NULL DEFAULT 600, \
            source TEXT NOT NULL DEFAULT 'builtin', sort_order INTEGER NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            active_domains TEXT)",
        // semantic_cache
        "CREATE TABLE IF NOT EXISTS semantic_cache (\
            id TEXT NOT NULL PRIMARY KEY, prompt_hash TEXT NOT NULL, response TEXT NOT NULL, \
            model_id TEXT, token_count INTEGER NOT NULL DEFAULT 0, \
            task_type TEXT NOT NULL DEFAULT 'moderate', ttl_secs INTEGER NOT NULL, \
            created_at BIGINT NOT NULL, hit_count INTEGER NOT NULL DEFAULT 0)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // --- Section B: Workflow tables（来自 v001） ---

    for sql in &[
        "CREATE TABLE IF NOT EXISTS workflow_templates (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            icon TEXT NOT NULL DEFAULT '', tags TEXT, version INTEGER NOT NULL DEFAULT 1, \
            is_preset BOOLEAN NOT NULL DEFAULT FALSE, is_editable BOOLEAN NOT NULL DEFAULT TRUE, \
            is_public BOOLEAN NOT NULL DEFAULT FALSE, trigger_config TEXT, \
            nodes TEXT NOT NULL, edges TEXT NOT NULL, input_schema TEXT, output_schema TEXT, \
            variables TEXT, error_config TEXT, composite_source TEXT, tool_defs TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_template_versions (\
            id TEXT NOT NULL PRIMARY KEY, template_id TEXT NOT NULL, name TEXT NOT NULL, \
            description TEXT, icon TEXT NOT NULL DEFAULT '', tags TEXT, \
            version INTEGER NOT NULL, is_preset BOOLEAN NOT NULL DEFAULT FALSE, \
            is_editable BOOLEAN NOT NULL DEFAULT TRUE, is_public BOOLEAN NOT NULL DEFAULT FALSE, \
            trigger_config TEXT, nodes TEXT NOT NULL, edges TEXT NOT NULL, \
            input_schema TEXT, output_schema TEXT, variables TEXT, error_config TEXT, \
            created_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_executions (\
            id TEXT NOT NULL PRIMARY KEY, workflow_id TEXT NOT NULL, status TEXT NOT NULL, \
            input_params TEXT, output_result TEXT, node_executions TEXT, \
            total_time_ms BIGINT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_marketplace (\
            id TEXT NOT NULL PRIMARY KEY, template_id TEXT NOT NULL, author_id TEXT NOT NULL, \
            name TEXT NOT NULL, description TEXT, category TEXT NOT NULL, \
            icon TEXT NOT NULL DEFAULT '', tags TEXT, downloads BIGINT NOT NULL DEFAULT 0, rating_average DOUBLE PRECISION NOT NULL DEFAULT 0.0, rating_count INTEGER NOT NULL DEFAULT 0, \
            is_featured BOOLEAN NOT NULL DEFAULT FALSE, is_verified BOOLEAN NOT NULL DEFAULT FALSE, \
            is_public BOOLEAN NOT NULL DEFAULT TRUE, created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_marketplace_reviews (\
            id TEXT NOT NULL PRIMARY KEY, marketplace_id TEXT NOT NULL, user_id TEXT NOT NULL, \
            rating INTEGER NOT NULL, comment TEXT, is_hidden BOOLEAN NOT NULL DEFAULT FALSE, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_snapshots (\
            id TEXT NOT NULL PRIMARY KEY, workflow_id TEXT NOT NULL, \
            snapshot_json TEXT NOT NULL, created_at BIGINT NOT NULL, step_id TEXT)",
        "CREATE TABLE IF NOT EXISTS loop_checkpoints (\
            execution_id TEXT NOT NULL, node_id TEXT NOT NULL, \
            payload_json TEXT NOT NULL, updated_at BIGINT NOT NULL, \
            PRIMARY KEY(execution_id, node_id))",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // --- Section C: Gateway / Tools tables（来自 v001） ---

    for sql in &[
        "CREATE TABLE IF NOT EXISTS gateway_links (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, link_type TEXT NOT NULL, \
            endpoint TEXT NOT NULL, api_key_id TEXT, enabled INTEGER NOT NULL DEFAULT 1, \
            status TEXT NOT NULL DEFAULT 'disconnected', error_message TEXT, \
            auto_sync_models INTEGER NOT NULL DEFAULT 1, \
            auto_sync_skills INTEGER NOT NULL DEFAULT 1, last_sync_at BIGINT, \
            latency_ms BIGINT, version TEXT, created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS gateway_link_policies (\
            id TEXT NOT NULL PRIMARY KEY, link_id TEXT NOT NULL, route_strategy TEXT NOT NULL, \
            model_fallback_enabled INTEGER NOT NULL DEFAULT 1, global_rpm BIGINT, \
            per_model_rpm BIGINT, token_limit_per_minute BIGINT, \
            key_rotation_strategy TEXT NOT NULL DEFAULT 'round_robin', \
            key_failover_enabled INTEGER NOT NULL DEFAULT 1)",
        "CREATE TABLE IF NOT EXISTS gateway_link_activities (\
            id TEXT NOT NULL PRIMARY KEY, link_id TEXT NOT NULL, activity_type TEXT NOT NULL, \
            description TEXT, created_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS generated_tools (\
            id TEXT NOT NULL PRIMARY KEY, tool_name TEXT NOT NULL, original_name TEXT NOT NULL, \
            original_description TEXT NOT NULL, input_schema TEXT NOT NULL, \
            output_schema TEXT NOT NULL, implementation TEXT NOT NULL, \
            source_info TEXT NOT NULL, created_at BIGINT NOT NULL)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // --- Section D: Knowledge extension tables（来自 v001） ---

    for sql in &[
        "CREATE TABLE IF NOT EXISTS notes (\
            id TEXT NOT NULL PRIMARY KEY, vault_id TEXT NOT NULL, title TEXT NOT NULL, \
            file_path TEXT NOT NULL, content TEXT NOT NULL, content_hash TEXT NOT NULL, \
            author TEXT NOT NULL, page_type TEXT, source_refs TEXT, related_pages TEXT, \
            quality_score DOUBLE PRECISION, last_linted_at BIGINT, last_compiled_at BIGINT, \
            compiled_source_hash TEXT, user_edited INTEGER NOT NULL DEFAULT 0, \
            user_edited_at BIGINT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            is_deleted INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS knowledge_entities (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            entity_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            source_language TEXT, properties TEXT NOT NULL, lifecycle TEXT, behaviors TEXT, \
            metadata TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS knowledge_attributes (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, \
            entity_id TEXT NOT NULL, name TEXT NOT NULL, attribute_type TEXT NOT NULL, \
            data_type TEXT NOT NULL, description TEXT, \
            is_required BOOLEAN NOT NULL DEFAULT FALSE, default_value TEXT, constraints TEXT, \
            validation_rules TEXT, metadata TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS knowledge_relations (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, \
            source_entity_id TEXT NOT NULL, target_entity_id TEXT NOT NULL, \
            relation_type TEXT NOT NULL, description TEXT, properties TEXT, metadata TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS knowledge_flows (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            flow_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            steps TEXT NOT NULL, decision_points TEXT, error_handling TEXT, \
            preconditions TEXT, postconditions TEXT, metadata TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS knowledge_interfaces (\
            id TEXT NOT NULL PRIMARY KEY, knowledge_base_id TEXT NOT NULL, name TEXT NOT NULL, \
            interface_type TEXT NOT NULL, description TEXT, source_path TEXT NOT NULL, \
            input_schema TEXT NOT NULL, output_schema TEXT NOT NULL, error_codes TEXT, \
            communication_pattern TEXT, version TEXT, metadata TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // --- Section E: Prompt tables + background_tasks（来自 v001） ---

    for sql in &[
        "CREATE TABLE IF NOT EXISTS prompt_templates (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            content TEXT NOT NULL, variables_schema TEXT, version INTEGER NOT NULL DEFAULT 1, \
            is_active BOOLEAN NOT NULL DEFAULT TRUE, ab_test_enabled BOOLEAN NOT NULL DEFAULT FALSE, \
            ab_test_variant TEXT, \
            category TEXT, tags TEXT, author TEXT, source TEXT, source_type TEXT, \
            format TEXT DEFAULT 'plain', metadata_json TEXT, \
            usage_count INTEGER NOT NULL DEFAULT 0, is_favorite BOOLEAN NOT NULL DEFAULT FALSE, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS prompt_template_versions (\
            id TEXT NOT NULL PRIMARY KEY, template_id TEXT NOT NULL, version INTEGER NOT NULL, \
            name TEXT NOT NULL, description TEXT, content TEXT NOT NULL, \
            variables_schema TEXT, changelog TEXT, \
            category TEXT, tags TEXT, author TEXT, source TEXT, \
            created_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS background_tasks (\
            id TEXT NOT NULL PRIMARY KEY, title TEXT NOT NULL, \
            description TEXT NOT NULL DEFAULT '', task_type TEXT NOT NULL, command TEXT, \
            prompt TEXT, status TEXT NOT NULL DEFAULT 'pending', \
            output TEXT NOT NULL DEFAULT '', exit_code INTEGER, conversation_id TEXT, \
            created_by TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            finished_at BIGINT)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // --- Section F: Wiki extension tables（来自 v001） ---

    for sql in &[
        "CREATE TABLE IF NOT EXISTS wiki_templates (\
            id TEXT NOT NULL PRIMARY KEY, wiki_id TEXT NOT NULL, name TEXT NOT NULL, \
            description TEXT, content TEXT NOT NULL, page_type TEXT, \
            is_builtin BOOLEAN NOT NULL DEFAULT FALSE, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS wiki_page_versions (\
            id BIGSERIAL PRIMARY KEY, wiki_id TEXT NOT NULL, \
            note_id TEXT NOT NULL, title TEXT NOT NULL, content TEXT NOT NULL, \
            content_hash TEXT NOT NULL, author TEXT NOT NULL, created_at BIGINT NOT NULL)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // --- Section G: Trajectory tables（来自 v001） ---

    for sql in &[
        "CREATE TABLE IF NOT EXISTS trajectory_trajectories (\
            id TEXT PRIMARY KEY, session_id TEXT NOT NULL, user_id TEXT NOT NULL, \
            topic TEXT NOT NULL, summary TEXT NOT NULL, outcome TEXT NOT NULL, \
            duration_ms BIGINT NOT NULL, quality_overall DOUBLE PRECISION NOT NULL, \
            quality_task_completion DOUBLE PRECISION NOT NULL, quality_tool_efficiency DOUBLE PRECISION NOT NULL, \
            quality_reasoning_quality DOUBLE PRECISION NOT NULL, quality_user_satisfaction DOUBLE PRECISION NOT NULL, \
            value_score DOUBLE PRECISION NOT NULL, patterns TEXT NOT NULL, created_at TEXT NOT NULL, \
            replay_count INTEGER NOT NULL DEFAULT 0, last_replay_at TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_steps (\
            id BIGSERIAL PRIMARY KEY, trajectory_id TEXT NOT NULL, \
            step_index INTEGER NOT NULL, timestamp_ms BIGINT NOT NULL, role TEXT NOT NULL, \
            content TEXT NOT NULL, reasoning TEXT, tool_calls TEXT, tool_results TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_rewards (\
            id TEXT PRIMARY KEY, trajectory_id TEXT NOT NULL, reward_type TEXT NOT NULL, \
            step_index INTEGER NOT NULL DEFAULT 0, value DOUBLE PRECISION NOT NULL, created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_skills (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, \
            skill_type TEXT NOT NULL, content TEXT NOT NULL, category TEXT NOT NULL, \
            tags TEXT NOT NULL, scenarios TEXT NOT NULL DEFAULT '[]', \
            parameters TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, \
            usage_count INTEGER NOT NULL DEFAULT 0, success_rate DOUBLE PRECISION NOT NULL DEFAULT 0.0, \
            avg_execution_time_ms BIGINT NOT NULL DEFAULT 0, \
            consecutive_failures INTEGER NOT NULL DEFAULT 0, last_failure_at TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_skill_executions (\
            id TEXT PRIMARY KEY, skill_id TEXT NOT NULL, trajectory_id TEXT, \
            success INTEGER NOT NULL, execution_time_ms BIGINT NOT NULL, \
            created_at TEXT NOT NULL, input_args TEXT, output_result TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_patterns (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, \
            pattern_type TEXT NOT NULL, trajectory_ids TEXT NOT NULL, \
            frequency INTEGER NOT NULL, success_rate DOUBLE PRECISION NOT NULL, \
            average_quality DOUBLE PRECISION NOT NULL, average_value_score DOUBLE PRECISION NOT NULL, \
            reward_profile TEXT NOT NULL, created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_entities (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL, entity_type TEXT NOT NULL, \
            properties TEXT NOT NULL DEFAULT '{}', aliases TEXT NOT NULL DEFAULT '[]', \
            first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL, \
            mention_count INTEGER NOT NULL DEFAULT 1, confidence DOUBLE PRECISION NOT NULL DEFAULT 0.5, \
            created_at TEXT, updated_at TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_relationships (\
            id TEXT PRIMARY KEY, source_id TEXT NOT NULL, target_id TEXT NOT NULL, \
            relation_type TEXT NOT NULL, properties TEXT NOT NULL DEFAULT '{}', \
            weight DOUBLE PRECISION NOT NULL DEFAULT 1.0, created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_sessions (\
            id TEXT PRIMARY KEY, title TEXT NOT NULL, \
            platform TEXT NOT NULL DEFAULT 'web', user_id TEXT NOT NULL DEFAULT 'default', \
            model TEXT NOT NULL DEFAULT 'unknown', system_prompt TEXT NOT NULL DEFAULT '', \
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL, parent_session_id TEXT, \
            token_input BIGINT NOT NULL DEFAULT 0, token_output BIGINT NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS trajectory_messages (\
            id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL, \
            content TEXT NOT NULL, tool_calls TEXT, tool_results TEXT, usage TEXT, \
            created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_memories (\
            id TEXT PRIMARY KEY, memory_type TEXT NOT NULL, content TEXT NOT NULL, \
            updated_at BIGINT NOT NULL, \
            tier TEXT NOT NULL DEFAULT 'working', importance DOUBLE PRECISION NOT NULL DEFAULT 0.5, \
            access_count INTEGER NOT NULL DEFAULT 0, last_accessed BIGINT, \
            decay_rate DOUBLE PRECISION NOT NULL DEFAULT 0.01, created_at BIGINT, expires_at BIGINT, \
            source_conversation_id TEXT, source_message_id TEXT, \
            memory_nature TEXT NOT NULL DEFAULT 'semantic', tags TEXT NOT NULL DEFAULT '[]', \
            namespace_id TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_learned_patterns (\
            id TEXT PRIMARY KEY, pattern TEXT NOT NULL, pattern_type TEXT NOT NULL, \
            success INTEGER NOT NULL DEFAULT 0, failure INTEGER NOT NULL DEFAULT 0, \
            last_used TEXT NOT NULL, created_at TEXT NOT NULL, metadata TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_preferences (\
            id TEXT PRIMARY KEY, key TEXT NOT NULL UNIQUE, value TEXT NOT NULL, \
            confidence DOUBLE PRECISION NOT NULL DEFAULT 0.0, updated_at TEXT NOT NULL)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // --- v101: Smart Router 路由历史持久化表（route_history） ---

    {
        let rh = "\
            CREATE TABLE IF NOT EXISTS route_history (\
                id TEXT NOT NULL PRIMARY KEY, prompt_hash TEXT NOT NULL, \
                prompt_preview TEXT NOT NULL, heuristic_tier TEXT NOT NULL, \
                selected_tier TEXT NOT NULL, outcome_success BOOLEAN, \
                outcome_quality_score DOUBLE PRECISION, outcome_user_override BOOLEAN, \
                outcome_user_tier TEXT, outcome_latency_ms BIGINT, \
                outcome_tokens_used BIGINT, outcome_cost_usd DOUBLE PRECISION, \
                timestamp BIGINT NOT NULL, features_json TEXT)";
        exec_ddl(&db, is_pg, rh).await?;
        exec_ddl(&db, is_pg, "CREATE INDEX IF NOT EXISTS idx_route_history_prompt_hash ON route_history(prompt_hash)").await?;
        exec_ddl(
            &db,
            is_pg,
            "CREATE INDEX IF NOT EXISTS idx_route_history_timestamp ON route_history(timestamp)",
        )
        .await?;
    }

    // --- v004: Dynamic UI schemas ---

    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS dynamic_ui_schemas (\
         id TEXT NOT NULL PRIMARY KEY, \
         title TEXT NOT NULL, \
         description TEXT NOT NULL DEFAULT '', \
         schema_json TEXT NOT NULL, \
         category TEXT NOT NULL DEFAULT 'custom', \
         tags TEXT NOT NULL DEFAULT '[]', \
         is_builtin INTEGER NOT NULL DEFAULT 0, \
         created_at TEXT NOT NULL, \
         updated_at TEXT NOT NULL)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS dynamic_ui_form_data (\
         id TEXT NOT NULL PRIMARY KEY, \
         schema_id TEXT NOT NULL, \
         form_data_json TEXT NOT NULL, \
         instance_key TEXT NOT NULL DEFAULT 'default', \
         updated_at TEXT NOT NULL)",
    )
    .await?;

    // --- v004b: Dynamic UI pins (导航钉入配置，后端持久化) ---

    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS dynamic_ui_pins (\
         schema_id TEXT NOT NULL PRIMARY KEY, \
         title TEXT NOT NULL, \
         group_name TEXT NOT NULL DEFAULT 'other', \
         position INTEGER NOT NULL DEFAULT 0, \
         created_at TEXT NOT NULL, \
         updated_at TEXT NOT NULL)",
    )
    .await?;

    // --- v008: Credentials & RL policies ---

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS credentials (\
         id TEXT NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL, \
         credential_type TEXT NOT NULL, \
         data_encrypted TEXT NOT NULL, \
         created_at BIGINT NOT NULL, \
         updated_at BIGINT NOT NULL)",
    )
    .await?;

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS rl_policies (\
         id TEXT NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL, \
         policy_type TEXT NOT NULL, \
         model_id TEXT NOT NULL, \
         reward_signals_json TEXT NOT NULL, \
         experiences_json TEXT NOT NULL, \
         total_experiences INTEGER NOT NULL, \
         episodes_completed INTEGER NOT NULL, \
         avg_reward REAL NOT NULL, \
         last_update TEXT NOT NULL, \
         created_at TEXT NOT NULL)",
    )
    .await?;

    // --- v005: Index jobs ---

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS index_jobs (\
         id TEXT NOT NULL PRIMARY KEY, \
         job_type TEXT NOT NULL, \
         container_type TEXT NOT NULL, \
         container_id TEXT NOT NULL, \
         item_id TEXT NOT NULL, \
         status TEXT NOT NULL DEFAULT 'pending', \
         current_stage TEXT, \
         progress INTEGER NOT NULL DEFAULT 0, \
         error_message TEXT, \
         retry_count INTEGER NOT NULL DEFAULT 0, \
         max_retries INTEGER NOT NULL DEFAULT 3, \
         priority INTEGER NOT NULL DEFAULT 0, \
         created_at BIGINT NOT NULL, \
         started_at BIGINT, \
         completed_at BIGINT, \
         metadata TEXT)",
    )
    .await?;

    // --- v006: Vec collections ---

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS vec_collections (\
         collection_id TEXT NOT NULL PRIMARY KEY, \
         dimensions INTEGER NOT NULL, \
         embedding_model TEXT, \
         index_type TEXT NOT NULL DEFAULT 'flat', \
         hnsw_ef_construction INTEGER, \
         hnsw_m INTEGER, \
         hnsw_ef_search INTEGER, \
         vector_count BIGINT NOT NULL DEFAULT 0, \
         created_at BIGINT NOT NULL, \
         updated_at BIGINT NOT NULL, \
         last_indexed_at BIGINT, \
         metadata TEXT)",
    )
    .await?;

    // --- v007: Dynamic UI schema versions ---

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS dynamic_ui_schema_versions (\
         id BIGSERIAL PRIMARY KEY, \
         schema_id TEXT NOT NULL, \
         version TEXT NOT NULL, \
         title TEXT NOT NULL, \
         description TEXT NOT NULL DEFAULT '', \
         schema_json TEXT NOT NULL, \
         category TEXT NOT NULL DEFAULT 'custom', \
         tags TEXT NOT NULL DEFAULT '[]', \
         change_log TEXT NOT NULL DEFAULT '', \
         created_at BIGINT NOT NULL)",
    )
    .await?;

    // --- v007: Add version column to dynamic_ui_schemas ---

    let _ = db
        .execute_unprepared(
            "ALTER TABLE dynamic_ui_schemas ADD COLUMN version TEXT NOT NULL DEFAULT '1.0.0'",
        )
        .await;

    // --- v009: Tool adaptation columns on providers ---

    let _ = db.execute_unprepared("ALTER TABLE providers ADD COLUMN tool_adaptation TEXT").await;
    let _ = db
        .execute_unprepared("ALTER TABLE providers ADD COLUMN tool_adaptation_marker_prefix TEXT")
        .await;

    // ========================================================================
    // PHASE 3.7: 补充旧库缺失字段
    //   v100 之前的库（v001–v011）已存在 agent_roles / agency_experts 表，
    //   PHASE 2 的 `CREATE TABLE IF NOT EXISTS` 对已有表是 no-op，不会补上
    //   后续新增的 active_domains / recommended_workflows / recommended_tools
    //   字段，导致 entity 访问时报「字段不存在」。
    //
    //   幂等策略：
    //     - PG: 先查 information_schema 确认缺失再用 ADD COLUMN（也可直接
    //       ADD COLUMN IF NOT EXISTS，这里保留计数日志以便观测）。
    //     - SQLite: 普通 ADD COLUMN，忽略重复列错误（let _ = ...）。
    // ========================================================================

    if is_pg {
        let mut added = 0usize;
        let mut already = 0usize;

        for (table, column, col_type) in MISSING_COLUMN_TARGETS {
            let row = db
                .query_one_raw(sea_orm::Statement::from_string(
                    DbBackend::Postgres,
                    format!(
                        "SELECT 1 AS exists_flag FROM information_schema.columns \
                         WHERE table_schema = current_schema() \
                           AND table_name = '{table}' AND column_name = '{column}'"
                    ),
                ))
                .await?;

            if row.is_some() {
                already += 1;
                continue;
            }

            let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {col_type}");
            db.execute_unprepared(&sql).await?;
            added += 1;
        }

        tracing::info!(
            "[v100] PHASE 3.7 missing columns: {} added, {} already present",
            added,
            already
        );
    } else {
        // SQLite: ADD COLUMN 已存在的列会报错，忽略之（PHASE 2 已建表的新库
        // 此处列已存在，报错被忽略；旧库缺失列则被补上）。
        for (table, column, col_type) in MISSING_COLUMN_TARGETS {
            let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {col_type}");
            let _ = db.execute_unprepared(&sql).await;
        }
        tracing::info!("[v100] SQLite: PHASE 3.7 missing columns ADD COLUMN (errors ignored)");
    }

    // ========================================================================
    // PHASE 4: 全部索引
    // ========================================================================

    // --- v001 时期索引 ---

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_conversations_memory_status ON conversations(memory_status)",
        "CREATE INDEX IF NOT EXISTS idx_search_providers_enabled ON search_providers(enabled)",
        "CREATE INDEX IF NOT EXISTS idx_search_citations_conv ON search_citations(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_search_citations_msg ON search_citations(message_id)",
        "CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled)",
        "CREATE INDEX IF NOT EXISTS idx_tool_descriptors_server ON tool_descriptors(server_id)",
        "CREATE INDEX IF NOT EXISTS idx_tool_executions_conv ON tool_executions(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_tool_executions_msg ON tool_executions(message_id)",
        "CREATE INDEX IF NOT EXISTS idx_tool_executions_server ON tool_executions(server_id)",
        "CREATE INDEX IF NOT EXISTS idx_knowledge_bases_enabled ON knowledge_bases(enabled)",
        "CREATE INDEX IF NOT EXISTS idx_knowledge_documents_kb ON knowledge_documents(knowledge_base_id)",
        "CREATE INDEX IF NOT EXISTS idx_retrieval_hits_conv ON retrieval_hits(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_retrieval_hits_msg ON retrieval_hits(message_id)",
        "CREATE INDEX IF NOT EXISTS idx_retrieval_hits_kb ON retrieval_hits(knowledge_base_id)",
        "CREATE INDEX IF NOT EXISTS idx_memory_namespaces_scope ON memory_namespaces(scope)",
        "CREATE INDEX IF NOT EXISTS idx_memory_items_ns ON memory_items(namespace_id)",
        "CREATE INDEX IF NOT EXISTS idx_artifacts_conv ON artifacts(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_artifacts_pinned ON artifacts(pinned)",
        "CREATE INDEX IF NOT EXISTS idx_context_sources_conv ON context_sources(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_context_sources_msg ON context_sources(message_id)",
        "CREATE INDEX IF NOT EXISTS idx_conv_branches_parent ON conversation_branches(parent_message_id)",
        "CREATE INDEX IF NOT EXISTS idx_backup_targets_kind ON backup_targets(kind)",
        "CREATE INDEX IF NOT EXISTS idx_import_jobs_status ON import_jobs(status)",
        "CREATE INDEX IF NOT EXISTS idx_import_jobs_created ON import_jobs(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_program_policies_name ON program_policies(program_name)",
        "CREATE INDEX IF NOT EXISTS idx_gateway_diagnostics_cat ON gateway_diagnostics(category)",
        "CREATE INDEX IF NOT EXISTS idx_gateway_diagnostics_created ON gateway_diagnostics(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_stored_files_hash ON stored_files(hash)",
        "CREATE INDEX IF NOT EXISTS idx_stored_files_conversation ON stored_files(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_conversation_summaries_conversation ON conversation_summaries(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_sessions_conversation ON agent_sessions(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_semantic_cache_hash ON semantic_cache(prompt_hash)",
        "CREATE INDEX IF NOT EXISTS idx_semantic_cache_created ON semantic_cache(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_wiki_templates_wiki_id ON wiki_templates(wiki_id)",
        "CREATE INDEX IF NOT EXISTS idx_wiki_page_versions_note_id ON wiki_page_versions(note_id)",
        "CREATE INDEX IF NOT EXISTS idx_wiki_page_versions_wiki_id ON wiki_page_versions(wiki_id)",
        "CREATE INDEX IF NOT EXISTS idx_traj_trajectories_session ON trajectory_trajectories(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_traj_trajectories_user ON trajectory_trajectories(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_traj_trajectories_created ON trajectory_trajectories(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_traj_steps_traj ON trajectory_steps(trajectory_id, step_index)",
        "CREATE INDEX IF NOT EXISTS idx_traj_skill_exec ON trajectory_skill_executions(skill_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_traj_patterns_type ON trajectory_patterns(pattern_type)",
        "CREATE INDEX IF NOT EXISTS idx_traj_entities_type ON trajectory_entities(entity_type)",
        "CREATE INDEX IF NOT EXISTS idx_traj_entities_name ON trajectory_entities(name)",
        "CREATE INDEX IF NOT EXISTS idx_traj_rel_source ON trajectory_relationships(source_id)",
        "CREATE INDEX IF NOT EXISTS idx_traj_rel_target ON trajectory_relationships(target_id)",
        "CREATE INDEX IF NOT EXISTS idx_traj_sessions_updated ON trajectory_sessions(updated_at)",
        "CREATE INDEX IF NOT EXISTS idx_traj_messages_session ON trajectory_messages(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_traj_memories_type ON trajectory_memories(memory_type)",
        "CREATE INDEX IF NOT EXISTS idx_traj_memories_tier ON trajectory_memories(tier)",
        "CREATE INDEX IF NOT EXISTS idx_traj_memories_importance ON trajectory_memories(importance)",
        "CREATE INDEX IF NOT EXISTS idx_traj_memories_expires ON trajectory_memories(expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_traj_memories_namespace ON trajectory_memories(namespace_id)",
        "CREATE INDEX IF NOT EXISTS idx_traj_learned_type ON trajectory_learned_patterns(pattern_type)",
        "CREATE INDEX IF NOT EXISTS idx_traj_prefs_key ON trajectory_preferences(key)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // --- v002 索引 ---

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_messages_conv_created \
         ON messages(conversation_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_conversations_updated \
         ON conversations(updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_provider_keys_provider \
         ON provider_keys(provider_id)",
        "CREATE INDEX IF NOT EXISTS idx_gateway_usage_key \
         ON gateway_usage(key_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_user \
         ON agent_sessions(conversation_id, total_tokens DESC)",
        "CREATE INDEX IF NOT EXISTS idx_messages_branch \
         ON messages(branch_id) WHERE branch_id IS NOT NULL",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // --- v004 索引 ---

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_dynamic_ui_schemas_category \
         ON dynamic_ui_schemas (category)",
        "CREATE INDEX IF NOT EXISTS idx_dynamic_ui_schemas_updated \
         ON dynamic_ui_schemas (updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_dynamic_ui_form_data_schema \
         ON dynamic_ui_form_data (schema_id)",
        "CREATE INDEX IF NOT EXISTS idx_dynamic_ui_pins_group \
         ON dynamic_ui_pins (group_name, position)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // --- v005 索引 ---

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_index_jobs_status \
         ON index_jobs (status, priority DESC, created_at ASC)",
        "CREATE INDEX IF NOT EXISTS idx_index_jobs_container \
         ON index_jobs (container_type, container_id)",
        "CREATE INDEX IF NOT EXISTS idx_index_jobs_item \
         ON index_jobs (container_type, item_id)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // --- v006 索引 ---

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_vec_collections_model \
         ON vec_collections (embedding_model)",
        "CREATE INDEX IF NOT EXISTS idx_vec_collections_updated \
         ON vec_collections (updated_at DESC)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // --- v007 索引 ---

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_dyn_ui_schema_versions_schema \
         ON dynamic_ui_schema_versions (schema_id)",
        "CREATE INDEX IF NOT EXISTS idx_dyn_ui_schema_versions_created \
         ON dynamic_ui_schema_versions (schema_id, created_at DESC)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // --- v008 索引 ---

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_credentials_name ON credentials (name)",
        "CREATE INDEX IF NOT EXISTS idx_credentials_type ON credentials (credential_type)",
        "CREATE INDEX IF NOT EXISTS idx_rl_policies_model ON rl_policies (model_id)",
        "CREATE INDEX IF NOT EXISTS idx_rl_policies_type ON rl_policies (policy_type)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // PHASE 5: 全文检索
    //   SQLite: FTS5 虚拟表
    //   PostgreSQL: tsvector 生成列 + GIN 索引
    // ========================================================================

    if !is_pg {
        for sql in &[
            "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(\
                content, content=messages, content_rowid=rowid, tokenize='unicode61')",
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectories_fts USING fts5(\
                id UNINDEXED, session_id UNINDEXED, topic, summary, content, \
                outcome UNINDEXED, quality_score UNINDEXED, created_at UNINDEXED, \
                tokenize='porter unicode61')",
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_memories_fts USING fts5(\
                id UNINDEXED, memory_type UNINDEXED, content, entities, \
                created_at UNINDEXED, tokenize='porter unicode61')",
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_skills_fts USING fts5(\
                id UNINDEXED, name, description, content, category UNINDEXED, \
                tags, created_at UNINDEXED, tokenize='porter unicode61')",
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_messages_fts USING fts5(\
                id UNINDEXED, session_id UNINDEXED, role UNINDEXED, content, \
                created_at UNINDEXED, tokenize='porter unicode61')",
        ] {
            db.execute_unprepared(sql).await?;
        }
    } else {
        for sql in &[
            "ALTER TABLE messages ADD COLUMN IF NOT EXISTS content_tsv tsvector \
             GENERATED ALWAYS AS (to_tsvector('simple', COALESCE(content, ''))) STORED",
            "CREATE INDEX IF NOT EXISTS idx_messages_content_tsv ON messages USING GIN (content_tsv)",
            "ALTER TABLE trajectory_trajectories ADD COLUMN IF NOT EXISTS tsv tsvector \
             GENERATED ALWAYS AS (to_tsvector('simple', \
               COALESCE(topic,'')||' '||COALESCE(summary,'')||' '||COALESCE(outcome,'')||' '||COALESCE(patterns,''))) STORED",
            "CREATE INDEX IF NOT EXISTS idx_traj_trajectories_tsv ON trajectory_trajectories USING GIN (tsv)",
            "ALTER TABLE trajectory_memories ADD COLUMN IF NOT EXISTS tsv tsvector \
             GENERATED ALWAYS AS (to_tsvector('simple', COALESCE(content,'')||' '||COALESCE(tags,''))) STORED",
            "CREATE INDEX IF NOT EXISTS idx_traj_memories_tsv ON trajectory_memories USING GIN (tsv)",
            "ALTER TABLE trajectory_skills ADD COLUMN IF NOT EXISTS tsv tsvector \
             GENERATED ALWAYS AS (to_tsvector('simple', \
               COALESCE(name,'')||' '||COALESCE(description,'')||' '||COALESCE(content,'')||' '||COALESCE(category,'')||' '||COALESCE(tags,''))) STORED",
            "CREATE INDEX IF NOT EXISTS idx_traj_skills_tsv ON trajectory_skills USING GIN (tsv)",
            "ALTER TABLE trajectory_messages ADD COLUMN IF NOT EXISTS tsv tsvector \
             GENERATED ALWAYS AS (to_tsvector('simple', COALESCE(content,'')||' '||COALESCE(role,''))) STORED",
            "CREATE INDEX IF NOT EXISTS idx_traj_messages_tsv ON trajectory_messages USING GIN (tsv)",
        ] {
            db.execute_unprepared(sql).await?;
        }
    }

    // ========================================================================
    // PHASE 6: FTS5 Triggers（仅 SQLite）
    // ========================================================================

    if !is_pg {
        for sql in &[
            "CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN \
             INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content); END",
            "CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN \
             INSERT INTO messages_fts(messages_fts, rowid, content) \
             VALUES('delete', old.rowid, old.content); END",
            "CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE OF content ON messages BEGIN \
             INSERT INTO messages_fts(messages_fts, rowid, content) \
             VALUES('delete', old.rowid, old.content); \
             INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content); END",
        ] {
            db.execute_unprepared(sql).await?;
        }
    }

    // ========================================================================
    // PHASE 7: workflow_approvals 表 — ApprovalNode HITL 审批持久化
    //   DDL 写 PG 语法（BIGINT），SQLite 通过 exec_ddl 自动接受。
    // ========================================================================

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS workflow_approvals (\
            id TEXT NOT NULL PRIMARY KEY, \
            execution_id TEXT NOT NULL, \
            node_id TEXT NOT NULL, \
            status TEXT NOT NULL DEFAULT 'pending', \
            title TEXT NOT NULL DEFAULT '', \
            message TEXT NOT NULL DEFAULT '', \
            approver TEXT, \
            channels TEXT, \
            payload TEXT, \
            decision TEXT, \
            approver_actual TEXT, \
            comment TEXT, \
            timeout_secs BIGINT NOT NULL DEFAULT 86400, \
            expires_at BIGINT NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, \
            resolved_at BIGINT)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_wf_approvals_exec ON workflow_approvals(execution_id)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_wf_approvals_status ON workflow_approvals(status)",
    )
    .await?;

    // ========================================================================
    // PHASE 8: business_roles + workflow_execution_stats + 字段扩展 + 种子数据
    //   来自 v101_business_roles：业务岗位表 / 工作流执行统计表 /
    //   agency_experts 人才属性扩展 / agent_profiles.business_role_id 外键 /
    //   6 个内置业务岗位种子数据。
    // ========================================================================

    let backend = db.get_database_backend();

    // --- 8.1: 创建 business_roles 表（业务岗位）
    //   DDL 写 PG 语法（BIGINT + FK），SQLite 通过 exec_ddl 自动接受。
    //   SQLite 支持 CREATE TABLE 内的 FK 声明（需 PRAGMA foreign_keys=ON 生效）。

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS business_roles (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            responsibilities TEXT, decision_authority TEXT, reports_to TEXT, \
            managed_expert_ids TEXT, required_certifications TEXT, active_domains TEXT, \
            system_prompt TEXT NOT NULL DEFAULT '', \
            icon TEXT, color TEXT, \
            source TEXT NOT NULL DEFAULT 'builtin', \
            sort_order INTEGER NOT NULL DEFAULT 0, is_enabled INTEGER NOT NULL DEFAULT 1, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (reports_to) REFERENCES business_roles(id) ON DELETE SET NULL)",
    )
    .await?;

    // --- 8.2: 创建 workflow_execution_stats 表（工作流执行统计）
    //   DDL 写 PG 语法（BIGINT + DOUBLE PRECISION），SQLite 自动接受。

    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS workflow_execution_stats (\
            id TEXT NOT NULL PRIMARY KEY, mission_hash TEXT, template_id TEXT, \
            execution_id TEXT, status TEXT NOT NULL, \
            total_time_ms BIGINT NOT NULL DEFAULT 0, \
            input_tokens BIGINT NOT NULL DEFAULT 0, \
            output_tokens BIGINT NOT NULL DEFAULT 0, \
            error_message TEXT, user_rating DOUBLE PRECISION, \
            created_at BIGINT NOT NULL)",
    )
    .await?;

    // 索引：按 mission_hash 聚合查询（PG/SQLite 语法一致）
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_exec_stats_mission \
         ON workflow_execution_stats(mission_hash)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_exec_stats_template \
         ON workflow_execution_stats(template_id)",
    )
    .await?;

    // --- 8.3: 扩展 agency_experts 表（人才属性） ---

    let agency_experts_columns: &[(&str, &str)] = &[
        ("seniority", "TEXT"),
        ("specialties", "TEXT"),
        ("parent_role_id", "TEXT"),
        ("success_rate", "DOUBLE PRECISION"),
        ("avg_latency_ms", "BIGINT"),
        ("avg_token_cost", "BIGINT"),
    ];

    for (col, ty) in agency_experts_columns {
        if is_pg {
            let sql = format!("ALTER TABLE agency_experts ADD COLUMN IF NOT EXISTS {} {}", col, ty);
            db.execute_unprepared(&sql).await?;
        } else {
            // SQLite: ADD COLUMN 不支持 IF NOT EXISTS，重复列错误吞掉实现幂等
            let sql = format!("ALTER TABLE agency_experts ADD COLUMN {} {}", col, ty);
            let _ = db.execute_raw(Statement::from_string(backend, sql)).await;
        }
    }

    // SQLite 的 agency_experts.parent_role_id 无法加 FK（SQLite 限制），靠应用层校验。
    // PostgreSQL 的 FK 也跳过（ALTER ADD CONSTRAINT IF NOT EXISTS 在 PG < 9.4 不支持，
    // 且存量库可能存在数据不一致），改由应用层 validate_parent_role_id 校验。

    // --- 8.4: 扩展 agent_profiles 表（business_role_id 外键） ---

    if is_pg {
        db.execute_unprepared(
            "ALTER TABLE agent_profiles ADD COLUMN IF NOT EXISTS business_role_id TEXT",
        )
        .await?;
    } else {
        let _ = db
            .execute_raw(Statement::from_string(
                backend,
                "ALTER TABLE agent_profiles ADD COLUMN business_role_id TEXT",
            ))
            .await;
    }

    // --- 8.5: 内置业务岗位种子数据（仅首次创建时插入） ---

    let now = axagent_harness::util_fns::now_ts();
    let builtin_roles = builtin_business_roles(now);

    for role in builtin_roles {
        let stmt = if is_pg {
            Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO business_roles \
                 (id, name, description, responsibilities, decision_authority, reports_to, \
                  managed_expert_ids, required_certifications, active_domains, system_prompt, \
                  icon, color, source, sort_order, is_enabled, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17) \
                 ON CONFLICT (id) DO NOTHING",
                [
                    role.id.into(),
                    role.name.into(),
                    role.description.into(),
                    role.responsibilities.into(),
                    role.decision_authority.into(),
                    role.reports_to.into(),
                    role.managed_expert_ids.into(),
                    role.required_certifications.into(),
                    role.active_domains.into(),
                    role.system_prompt.into(),
                    role.icon.into(),
                    role.color.into(),
                    role.source.into(),
                    role.sort_order.into(),
                    1i32.into(),
                    now.into(),
                    now.into(),
                ],
            )
        } else {
            Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT OR IGNORE INTO business_roles \
                 (id, name, description, responsibilities, decision_authority, reports_to, \
                  managed_expert_ids, required_certifications, active_domains, system_prompt, \
                  icon, color, source, sort_order, is_enabled, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    role.id.into(),
                    role.name.into(),
                    role.description.into(),
                    role.responsibilities.into(),
                    role.decision_authority.into(),
                    role.reports_to.into(),
                    role.managed_expert_ids.into(),
                    role.required_certifications.into(),
                    role.active_domains.into(),
                    role.system_prompt.into(),
                    role.icon.into(),
                    role.color.into(),
                    role.source.into(),
                    role.sort_order.into(),
                    1i32.into(),
                    now.into(),
                    now.into(),
                ],
            )
        };
        db.execute_raw(stmt).await?;
    }

    // ========================================================================
    // PHASE 9: workflow_templates.mission_hash 列 + 部分索引
    //   来自 v102_mission_hash：支持 compile_mission_to_template 去重缓存。
    // ========================================================================

    // 添加 mission_hash 列（用于 compile_mission_to_template 去重缓存）
    if is_pg {
        db.execute_unprepared(
            "ALTER TABLE workflow_templates ADD COLUMN IF NOT EXISTS mission_hash TEXT",
        )
        .await?;
    } else {
        // SQLite 不支持 IF NOT EXISTS，吞掉重复列错误实现幂等
        let _ = db
            .execute_unprepared("ALTER TABLE workflow_templates ADD COLUMN mission_hash TEXT")
            .await;
    }

    // 为 mission_hash 创建索引（仅在非 NULL 时索引，加速查重）
    let _ = db
        .execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_workflow_templates_mission_hash \
             ON workflow_templates(mission_hash) WHERE mission_hash IS NOT NULL",
        )
        .await;

    // ========================================================================
    // PHASE 10: trajectory_workflow_reflections 表 + 索引
    //   来自 v103_workflow_reflections：工作流反思历史持久化，
    //   支持跨会话反思查询 / 模式聚合 / 进化决策。
    // ========================================================================

    // 主表：trajectory_workflow_reflections
    //
    // 字段类型选择：
    // - quality_score: INTEGER（u8 转 i32，PG/SQLite 一致）
    // - timestamp / created_at: TEXT（RFC3339 字符串，避免 PG/SQLite 时间类型差异）
    // - error_patterns_json / reusable_patterns_json / metadata_json: TEXT（JSON 字符串）
    //
    // 注：所有字段均为 TEXT/INTEGER，PG 与 SQLite 语法一致，无需分支。
    // 不使用 PG 的 JSONB 是因为反思数据以读为主，无需 GIN 索引；保持与
    // trajectories.patterns 等已有 JSON 字段一致的 TEXT 存储方式。
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS trajectory_workflow_reflections (\
            id TEXT NOT NULL PRIMARY KEY, \
            workflow_id TEXT NOT NULL, \
            execution_id TEXT NOT NULL, \
            template_id TEXT, \
            quality_score INTEGER NOT NULL, \
            summary TEXT NOT NULL DEFAULT '', \
            error_patterns_json TEXT NOT NULL DEFAULT '[]', \
            reusable_patterns_json TEXT NOT NULL DEFAULT '[]', \
            metadata_json TEXT NOT NULL DEFAULT '{}', \
            timestamp TEXT NOT NULL, \
            created_at TEXT NOT NULL)",
    )
    .await?;

    // 索引 1：按 workflow_id 查询历史反思（聚合进化用）
    // 索引 2：按 timestamp 倒序查询（最近反思列表 / 分页用）
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_reflections_workflow \
         ON trajectory_workflow_reflections(workflow_id)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_reflections_timestamp \
         ON trajectory_workflow_reflections(timestamp)",
    )
    .await?;

    Ok(())
}

// ============================================================================
// 内置业务岗位种子数据（来自 v101_business_roles）
// ============================================================================

/// 内置业务岗位种子数据
struct BuiltinRole {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    responsibilities: &'static str,
    decision_authority: &'static str,
    reports_to: Option<&'static str>,
    managed_expert_ids: &'static str,
    required_certifications: &'static str,
    active_domains: &'static str,
    system_prompt: &'static str,
    icon: &'static str,
    color: &'static str,
    source: &'static str,
    sort_order: i32,
}

fn builtin_business_roles(_now: i64) -> Vec<BuiltinRole> {
    vec![
        BuiltinRole {
            id: "ceo",
            name: "CEO 首席执行官",
            description: "负责公司整体战略与决策",
            responsibilities: "[\"制定公司战略\",\"重大决策审批\",\"资源分配\"]",
            decision_authority: "{\"max_budget\": 10000000, \"scopes\": [\"all\"]}",
            reports_to: None,
            managed_expert_ids: "[]",
            required_certifications: "[\"10 年管理经验\"]",
            active_domains: "[\"business\",\"strategy\"]",
            system_prompt: "你是 CEO 首席执行官。你负责公司整体战略方向，对重大决策有最终审批权。在分析问题时，从全局视角出发，平衡短期收益与长期价值。",
            icon: "👑",
            color: "#FFD700",
            source: "builtin",
            sort_order: 0,
        },
        BuiltinRole {
            id: "cto",
            name: "CTO 首席技术官",
            description: "负责技术战略与研发管理",
            responsibilities: "[\"技术战略制定\",\"技术选型决策\",\"技术团队管理\",\"技术风险评估\"]",
            decision_authority: "{\"max_budget\": 1000000, \"scopes\": [\"tech\",\"architecture\",\"security\"]}",
            reports_to: Some("ceo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"8 年技术管理经验\"]",
            active_domains: "[\"development\",\"security\",\"devops\",\"data\"]",
            system_prompt: "你是 CTO 首席技术官。你负责技术战略、架构选型与团队管理。在决策时权衡技术先进性、团队能力与交付风险，优先考虑长期可维护性。",
            icon: "💻",
            color: "#4169E1",
            source: "builtin",
            sort_order: 1,
        },
        BuiltinRole {
            id: "cfo",
            name: "CFO 首席财务官",
            description: "负责财务管理与风险控制",
            responsibilities: "[\"财务规划\",\"预算审批\",\"财务风险评估\",\"投资决策\"]",
            decision_authority: "{\"max_budget\": 5000000, \"scopes\": [\"finance\",\"budget\"]}",
            reports_to: Some("ceo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"CPA 或同等资质\",\"8 年财务管理经验\"]",
            active_domains: "[\"finance\",\"business\"]",
            system_prompt: "你是 CFO 首席财务官。你负责财务规划、预算控制与风险评估。在决策时严格把控财务纪律，对投入产出比与现金流敏感。",
            icon: "💰",
            color: "#2E8B57",
            source: "builtin",
            sort_order: 2,
        },
        BuiltinRole {
            id: "cpo",
            name: "CPO 首席产品官",
            description: "负责产品战略与规划",
            responsibilities: "[\"产品战略\",\"需求优先级\",\"用户体验\",\"产品路线图\"]",
            decision_authority: "{\"max_budget\": 500000, \"scopes\": [\"product\",\"design\"]}",
            reports_to: Some("ceo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"8 年产品管理经验\"]",
            active_domains: "[\"business\",\"design\",\"writing\"]",
            system_prompt: "你是 CPO 首席产品官。你负责产品战略、需求优先级与用户体验。在决策时以用户价值为核心，平衡商业目标与技术成本。",
            icon: "🎯",
            color: "#FF6347",
            source: "builtin",
            sort_order: 3,
        },
        BuiltinRole {
            id: "pm",
            name: "产品经理",
            description: "负责产品需求与项目执行",
            responsibilities: "[\"需求分析\",\"产品文档\",\"项目跟进\",\"跨部门协调\"]",
            decision_authority: "{\"max_budget\": 100000, \"scopes\": [\"product\",\"project\"]}",
            reports_to: Some("cpo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"3 年产品经验\"]",
            active_domains: "[\"business\",\"design\",\"writing\"]",
            system_prompt: "你是产品经理。你负责需求分析、产品文档与项目跟进。在执行时关注用户痛点与商业目标，善用数据驱动决策。",
            icon: "📋",
            color: "#9370DB",
            source: "builtin",
            sort_order: 4,
        },
        BuiltinRole {
            id: "tech_lead",
            name: "技术负责人",
            description: "负责技术架构与研发执行",
            responsibilities: "[\"架构设计\",\"技术方案评审\",\"代码审查\",\"技术难点攻坚\"]",
            decision_authority: "{\"max_budget\": 100000, \"scopes\": [\"tech\",\"architecture\"]}",
            reports_to: Some("cto"),
            managed_expert_ids: "[]",
            required_certifications: "[\"5 年研发经验\",\"架构设计能力\"]",
            active_domains: "[\"development\",\"security\",\"devops\"]",
            system_prompt: "你是技术负责人。你负责架构设计、技术评审与代码质量。在执行时关注可维护性、可扩展性与工程效率。",
            icon: "🔧",
            color: "#1E90FF",
            source: "builtin",
            sort_order: 5,
        },
    ]
}
