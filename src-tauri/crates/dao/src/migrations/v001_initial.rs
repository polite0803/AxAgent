//! v001 — 全量 DDL 初始化（从 ddl.rs 迁出）
//!
//! 这是 schema 的"初始快照"：所有 `CREATE TABLE IF NOT EXISTS`、
//! `CREATE VIRTUAL TABLE IF NOT EXISTS`、`CREATE TRIGGER IF NOT EXISTS`、
//! `CREATE INDEX IF NOT EXISTS`（v001 时期就有的那些）都集中在这里。
//!
//! 注意：v002/v003 引入的新增索引 / 死表清理 **不应** 写到这里；
//! 这条 migration 的语义是"项目第一次启动时的 schema"。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // ========================================================================
    // SECTION A: Core tables
    // ========================================================================

    for sql in &[
        // providers
        "CREATE TABLE IF NOT EXISTS providers (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, provider_type TEXT NOT NULL, \
            api_host TEXT NOT NULL, api_path TEXT, enabled INTEGER NOT NULL DEFAULT 1, \
            proxy_config TEXT, sort_order INTEGER NOT NULL DEFAULT 0, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, \
            custom_headers TEXT, icon TEXT, builtin_id TEXT)",
        // provider_keys
        "CREATE TABLE IF NOT EXISTS provider_keys (\
            id TEXT NOT NULL PRIMARY KEY, provider_id TEXT NOT NULL, \
            key_encrypted TEXT NOT NULL, key_prefix TEXT NOT NULL DEFAULT '', \
            enabled INTEGER NOT NULL DEFAULT 1, last_validated_at INTEGER, last_error TEXT, \
            rotation_index INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL, \
            FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE)",
        // models (composite PK)
        "CREATE TABLE IF NOT EXISTS models (\
            provider_id TEXT NOT NULL, model_id TEXT NOT NULL, name TEXT NOT NULL, \
            capabilities TEXT NOT NULL DEFAULT '[]', max_tokens INTEGER, \
            enabled INTEGER NOT NULL DEFAULT 1, param_overrides TEXT, \
            model_type TEXT NOT NULL DEFAULT 'chat', group_name TEXT, \
            input_price_per_mtok REAL, output_price_per_mtok REAL, \
            PRIMARY KEY (provider_id, model_id), \
            FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE)",
        // conversations
        "CREATE TABLE IF NOT EXISTS conversations (\
            id TEXT NOT NULL PRIMARY KEY, title TEXT NOT NULL, model_id TEXT NOT NULL, \
            provider_id TEXT NOT NULL, app_id TEXT, system_prompt TEXT, temperature REAL, \
            max_tokens INTEGER, top_p REAL, frequency_penalty REAL, \
            message_count INTEGER NOT NULL DEFAULT 0, is_pinned INTEGER NOT NULL DEFAULT 0, \
            is_archived INTEGER NOT NULL DEFAULT 0, \
            workspace_snapshot_json TEXT NOT NULL DEFAULT '{}', \
            active_branch_id TEXT, active_artifact_id TEXT, \
            research_mode INTEGER NOT NULL DEFAULT 0, search_enabled INTEGER NOT NULL DEFAULT 0, \
            search_provider_id TEXT, thinking_budget INTEGER, \
            enabled_mcp_server_ids TEXT NOT NULL DEFAULT '[]', \
            enabled_knowledge_base_ids TEXT NOT NULL DEFAULT '[]', \
            enabled_memory_namespace_ids TEXT NOT NULL DEFAULT '[]', \
            enabled_wiki_ids TEXT NOT NULL DEFAULT '[]', agent_profile_id TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, \
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
            content TEXT NOT NULL, provider_id TEXT, model_id TEXT, token_count INTEGER, \
            attachments TEXT NOT NULL DEFAULT '[]', thinking TEXT, parent_message_id TEXT, \
            version_index INTEGER NOT NULL DEFAULT 0, is_active INTEGER NOT NULL DEFAULT 1, \
            branch_id TEXT, tool_calls_json TEXT, tool_call_id TEXT, \
            created_at INTEGER NOT NULL, parts TEXT, prompt_tokens BIGINT, \
            completion_tokens BIGINT, status TEXT NOT NULL DEFAULT 'complete', \
            tokens_per_second REAL, first_token_latency_ms BIGINT, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    for sql in &[
        "ALTER TABLE messages ADD COLUMN cache_creation_tokens BIGINT",
        "ALTER TABLE messages ADD COLUMN cache_read_tokens BIGINT",
    ] {
        let _ = db.execute_unprepared(sql).await;
    }

    for sql in &[
        // categories — 死表，无代码引用（v003 会 DROP）
        // apps — 死表，无代码引用（v003 会 DROP）
        // context_packs — 死表，无代码引用（v003 会 DROP）
        // gateway_keys
        "CREATE TABLE IF NOT EXISTS gateway_keys (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            key_hash TEXT NOT NULL UNIQUE, key_prefix TEXT NOT NULL, encrypted_key TEXT, \
            enabled INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL, last_used_at INTEGER)",
        // gateway_usage
        "CREATE TABLE IF NOT EXISTS gateway_usage (\
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, key_id TEXT NOT NULL, \
            provider_id TEXT NOT NULL, model_id TEXT, \
            request_tokens INTEGER NOT NULL DEFAULT 0, response_tokens INTEGER NOT NULL DEFAULT 0, \
            created_at INTEGER NOT NULL, \
            FOREIGN KEY (key_id) REFERENCES gateway_keys(id) ON DELETE CASCADE)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    let _ = db
        .execute_unprepared(
            "ALTER TABLE gateway_usage ADD COLUMN cached_input_tokens BIGINT NOT NULL DEFAULT 0",
        )
        .await;

    for sql in &[
        // settings
        "CREATE TABLE IF NOT EXISTS settings (\
            key TEXT NOT NULL PRIMARY KEY, value TEXT NOT NULL)",
        // search_providers
        "CREATE TABLE IF NOT EXISTS search_providers (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            provider_type TEXT NOT NULL DEFAULT 'tavily', endpoint TEXT, api_key_ref TEXT, \
            enabled INTEGER NOT NULL DEFAULT 1, region TEXT, language TEXT, safe_search INTEGER, \
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
            error_message TEXT, duration_ms INTEGER, approval_status TEXT, \
            skill_steps_json TEXT, depends_on TEXT, \
            created_at TEXT NOT NULL DEFAULT (datetime('now')), \
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
            size_bytes INTEGER NOT NULL DEFAULT 0, \
            indexing_status TEXT NOT NULL DEFAULT 'pending', doc_type TEXT NOT NULL DEFAULT '', \
            index_error TEXT, source_conversation_id TEXT, \
            created_at BIGINT NOT NULL DEFAULT 0, updated_at BIGINT NOT NULL DEFAULT 0, \
            FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE)",
        // retrieval_hits
        "CREATE TABLE IF NOT EXISTS retrieval_hits (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, message_id TEXT NOT NULL, \
            knowledge_base_id TEXT NOT NULL, document_id TEXT NOT NULL, chunk_ref TEXT NOT NULL, \
            score REAL NOT NULL DEFAULT 0.0, preview TEXT NOT NULL, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE, \
            FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id) ON DELETE CASCADE)",
        // memory_namespaces
        "CREATE TABLE IF NOT EXISTS memory_namespaces (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            scope TEXT NOT NULL DEFAULT 'global', app_id TEXT, embedding_provider TEXT, \
            embedding_dimensions INTEGER, retrieval_threshold REAL, retrieval_top_k INTEGER, \
            icon_type TEXT, icon_value TEXT, sort_order INTEGER NOT NULL DEFAULT 0)",
        // memory_items
        "CREATE TABLE IF NOT EXISTS memory_items (\
            id TEXT NOT NULL PRIMARY KEY, namespace_id TEXT NOT NULL, title TEXT NOT NULL, \
            content TEXT NOT NULL, source TEXT NOT NULL DEFAULT 'manual', \
            index_status TEXT NOT NULL DEFAULT 'pending', index_error TEXT, \
            updated_at TEXT NOT NULL DEFAULT (datetime('now')), \
            FOREIGN KEY (namespace_id) REFERENCES memory_namespaces(id) ON DELETE CASCADE)",
        // artifacts
        "CREATE TABLE IF NOT EXISTS artifacts (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            kind TEXT NOT NULL DEFAULT 'draft', title TEXT NOT NULL, \
            content TEXT NOT NULL DEFAULT '', format TEXT NOT NULL DEFAULT 'markdown', \
            pinned INTEGER NOT NULL DEFAULT 0, \
            updated_at TEXT NOT NULL DEFAULT (datetime('now')), \
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
            created_at TEXT NOT NULL DEFAULT (datetime('now')), \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // backup_manifests
        "CREATE TABLE IF NOT EXISTS backup_manifests (\
            id TEXT NOT NULL PRIMARY KEY, version TEXT NOT NULL, \
            created_at TEXT NOT NULL DEFAULT (datetime('now')), \
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
            created_at TEXT NOT NULL DEFAULT (datetime('now')))",
        // program_policies
        "CREATE TABLE IF NOT EXISTS program_policies (\
            id TEXT NOT NULL PRIMARY KEY, program_name TEXT NOT NULL UNIQUE, \
            allowed_provider_ids_json TEXT NOT NULL DEFAULT '[]', \
            allowed_model_ids_json TEXT NOT NULL DEFAULT '[]', \
            default_provider_id TEXT, default_model_id TEXT, rate_limit_per_minute INTEGER)",
        // gateway_diagnostics
        "CREATE TABLE IF NOT EXISTS gateway_diagnostics (\
            id TEXT NOT NULL PRIMARY KEY, category TEXT NOT NULL, \
            status TEXT NOT NULL DEFAULT 'ok', message TEXT NOT NULL, \
            created_at TEXT NOT NULL DEFAULT (datetime('now')))",
        // desktop_state
        "CREATE TABLE IF NOT EXISTS desktop_state (\
            window_key TEXT NOT NULL PRIMARY KEY, width INTEGER NOT NULL DEFAULT 1200, \
            height INTEGER NOT NULL DEFAULT 800, x INTEGER, y INTEGER, \
            maximized INTEGER NOT NULL DEFAULT 0, visible INTEGER NOT NULL DEFAULT 1)",
        // stored_files
        "CREATE TABLE IF NOT EXISTS stored_files (\
            id TEXT NOT NULL PRIMARY KEY, hash TEXT NOT NULL, original_name TEXT NOT NULL, \
            mime_type TEXT NOT NULL DEFAULT 'application/octet-stream', \
            size_bytes INTEGER NOT NULL, storage_path TEXT NOT NULL, conversation_id TEXT, \
            created_at TEXT NOT NULL DEFAULT (datetime('now')), \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL)",
        // gateway_request_logs
        "CREATE TABLE IF NOT EXISTS gateway_request_logs (\
            id TEXT NOT NULL PRIMARY KEY, key_id TEXT NOT NULL, key_name TEXT NOT NULL, \
            method TEXT NOT NULL, path TEXT NOT NULL, model TEXT, provider_id TEXT, \
            status_code INTEGER NOT NULL, duration_ms INTEGER NOT NULL, \
            request_tokens INTEGER NOT NULL DEFAULT 0, response_tokens INTEGER NOT NULL DEFAULT 0, \
            error_message TEXT, created_at INTEGER NOT NULL)",
        // conversation_summaries
        "CREATE TABLE IF NOT EXISTS conversation_summaries (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            summary_text TEXT NOT NULL, compressed_until_message_id TEXT, \
            token_count BIGINT, model_used TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
        // conversation_categories
        "CREATE TABLE IF NOT EXISTS conversation_categories (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, \
            icon_type TEXT, icon_value TEXT, system_prompt TEXT, \
            default_provider_id TEXT, default_model_id TEXT, \
            default_temperature REAL, default_max_tokens BIGINT, \
            default_top_p REAL, default_frequency_penalty REAL, \
            sort_order INTEGER NOT NULL DEFAULT 0, is_collapsed INTEGER NOT NULL DEFAULT 0, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
        // skill_states
        "CREATE TABLE IF NOT EXISTS skill_states (\
            name TEXT NOT NULL PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 0, \
            updated_at INTEGER NOT NULL)",
        // agent_sessions
        "CREATE TABLE IF NOT EXISTS agent_sessions (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, cwd TEXT, \
            workspace_locked INTEGER NOT NULL DEFAULT 0, permission_mode TEXT NOT NULL, \
            runtime_status TEXT NOT NULL, sdk_context_json TEXT, \
            sdk_context_backup_json TEXT, total_tokens INTEGER NOT NULL DEFAULT 0, \
            total_cost_usd REAL NOT NULL DEFAULT 0.0, \
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL)",
        // wikis
        "CREATE TABLE IF NOT EXISTS wikis (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, root_path TEXT NOT NULL, \
            schema_version TEXT NOT NULL DEFAULT '1.0', description TEXT, \
            note_count INTEGER NOT NULL DEFAULT 0, source_count INTEGER NOT NULL DEFAULT 0, \
            embedding_provider TEXT, embedding_dimensions INTEGER, \
            retrieval_threshold REAL, retrieval_top_k INTEGER, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
        // wiki_sources
        "CREATE TABLE IF NOT EXISTS wiki_sources (\
            id TEXT NOT NULL PRIMARY KEY, wiki_id TEXT NOT NULL, source_type TEXT NOT NULL, \
            source_path TEXT NOT NULL, title TEXT NOT NULL, mime_type TEXT NOT NULL, \
            size_bytes BIGINT NOT NULL, content_hash TEXT NOT NULL, metadata_json TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // wiki_pages
        "CREATE TABLE IF NOT EXISTS wiki_pages (\
            id TEXT NOT NULL PRIMARY KEY, wiki_id TEXT NOT NULL, note_id TEXT NOT NULL, \
            page_type TEXT NOT NULL, title TEXT NOT NULL, source_ids TEXT, \
            quality_score REAL, last_linted_at INTEGER, last_compiled_at INTEGER, \
            compiled_source_hash TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // wiki_operations
        "CREATE TABLE IF NOT EXISTS wiki_operations (\
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, wiki_id TEXT NOT NULL, \
            operation_type TEXT NOT NULL, target_type TEXT NOT NULL, target_id TEXT NOT NULL, \
            status TEXT NOT NULL, details_json TEXT, error_message TEXT, \
            created_at INTEGER NOT NULL, completed_at INTEGER, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // wiki_sync_queue
        "CREATE TABLE IF NOT EXISTS wiki_sync_queue (\
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, wiki_id TEXT NOT NULL, \
            event_type TEXT NOT NULL, target_type TEXT NOT NULL, target_id TEXT NOT NULL, \
            payload TEXT, status TEXT NOT NULL DEFAULT 'pending', \
            retry_count INTEGER NOT NULL DEFAULT 0, error_message TEXT, \
            created_at INTEGER NOT NULL, processed_at INTEGER, \
            FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE)",
        // note_links
        "CREATE TABLE IF NOT EXISTS note_links (\
            id INTEGER NOT NULL PRIMARY KEY, vault_id TEXT NOT NULL, \
            source_note_id TEXT NOT NULL, target_note_id TEXT NOT NULL, link_text TEXT, \
            link_type TEXT NOT NULL, created_at INTEGER NOT NULL)",
        // note_backlinks
        "CREATE TABLE IF NOT EXISTS note_backlinks (\
            id INTEGER NOT NULL PRIMARY KEY, vault_id TEXT NOT NULL, \
            source_note_id TEXT NOT NULL, target_note_id TEXT NOT NULL, link_text TEXT, \
            link_type TEXT NOT NULL, created_at INTEGER NOT NULL)",
        // plans
        "CREATE TABLE IF NOT EXISTS plans (\
            id TEXT NOT NULL PRIMARY KEY, conversation_id TEXT NOT NULL, \
            user_message_id TEXT NOT NULL, title TEXT NOT NULL, \
            steps_json TEXT NOT NULL DEFAULT '[]', status TEXT NOT NULL DEFAULT 'draft', \
            is_active INTEGER NOT NULL DEFAULT 1, created_under_strategy TEXT, reason TEXT, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, \
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)",
        // agency_experts
        "CREATE TABLE IF NOT EXISTS agency_experts (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            category TEXT NOT NULL, system_prompt TEXT NOT NULL, color TEXT, \
            source_dir TEXT NOT NULL, is_enabled INTEGER NOT NULL DEFAULT 1, \
            imported_at INTEGER NOT NULL, recommended_workflows TEXT, recommended_tools TEXT)",
        // agent_profiles
        "CREATE TABLE IF NOT EXISTS agent_profiles (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            category TEXT NOT NULL DEFAULT 'general', icon TEXT NOT NULL DEFAULT '🤖', \
            agent_role TEXT, \
            source TEXT NOT NULL DEFAULT 'builtin', tags TEXT, \
            suggested_provider_id TEXT, suggested_model_id TEXT, \
            suggested_temperature REAL, suggested_max_tokens BIGINT, \
            search_enabled INTEGER, recommend_permission_mode TEXT, \
            recommended_tools TEXT, disallowed_tools TEXT, recommended_workflows TEXT, \
            sort_order INTEGER NOT NULL DEFAULT 0, is_enabled INTEGER NOT NULL DEFAULT 1, \
            expert_id TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // agent_roles
        "CREATE TABLE IF NOT EXISTS agent_roles (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            system_prompt TEXT NOT NULL DEFAULT '', default_tools TEXT, \
            max_concurrent INTEGER NOT NULL DEFAULT 3, \
            timeout_seconds BIGINT NOT NULL DEFAULT 600, \
            source TEXT NOT NULL DEFAULT 'builtin', sort_order INTEGER NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // semantic_cache
        "CREATE TABLE IF NOT EXISTS semantic_cache (\
            id TEXT NOT NULL PRIMARY KEY, prompt_hash TEXT NOT NULL, response TEXT NOT NULL, \
            model_id TEXT, token_count INTEGER NOT NULL DEFAULT 0, \
            task_type TEXT NOT NULL DEFAULT 'moderate', ttl_secs INTEGER NOT NULL, \
            created_at INTEGER NOT NULL, hit_count INTEGER NOT NULL DEFAULT 0)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // SECTION B: Workflow tables
    // ========================================================================

    for sql in &[
        "CREATE TABLE IF NOT EXISTS workflow_templates (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            icon TEXT NOT NULL DEFAULT '', tags TEXT, version INTEGER NOT NULL DEFAULT 1, \
            is_preset INTEGER NOT NULL DEFAULT 0, is_editable INTEGER NOT NULL DEFAULT 1, \
            is_public INTEGER NOT NULL DEFAULT 0, trigger_config TEXT, \
            nodes TEXT NOT NULL, edges TEXT NOT NULL, input_schema TEXT, output_schema TEXT, \
            variables TEXT, error_config TEXT, composite_source TEXT, tool_defs TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_template_versions (\
            id TEXT NOT NULL PRIMARY KEY, template_id TEXT NOT NULL, name TEXT NOT NULL, \
            description TEXT, icon TEXT NOT NULL DEFAULT '', tags TEXT, \
            version INTEGER NOT NULL, is_preset INTEGER NOT NULL DEFAULT 0, \
            is_editable INTEGER NOT NULL DEFAULT 1, is_public INTEGER NOT NULL DEFAULT 0, \
            trigger_config TEXT, nodes TEXT NOT NULL, edges TEXT NOT NULL, \
            input_schema TEXT, output_schema TEXT, variables TEXT, error_config TEXT, \
            created_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_executions (\
            id TEXT NOT NULL PRIMARY KEY, workflow_id TEXT NOT NULL, status TEXT NOT NULL, \
            input_params TEXT, output_result TEXT, node_executions TEXT, \
            total_time_ms INTEGER, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_marketplace (\
            id TEXT NOT NULL PRIMARY KEY, template_id TEXT NOT NULL, author_id TEXT NOT NULL, \
            name TEXT NOT NULL, description TEXT, category TEXT NOT NULL, \
            icon TEXT NOT NULL DEFAULT '', tags TEXT, downloads BIGINT NOT NULL DEFAULT 0, \
            rating_average REAL NOT NULL DEFAULT 0.0, rating_count INTEGER NOT NULL DEFAULT 0, \
            is_featured INTEGER NOT NULL DEFAULT 0, is_verified INTEGER NOT NULL DEFAULT 0, \
            is_public INTEGER NOT NULL DEFAULT 1, created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_marketplace_reviews (\
            id TEXT NOT NULL PRIMARY KEY, marketplace_id TEXT NOT NULL, user_id TEXT NOT NULL, \
            rating INTEGER NOT NULL, comment TEXT, is_hidden INTEGER NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS workflow_snapshots (\
            id TEXT NOT NULL PRIMARY KEY, workflow_id TEXT NOT NULL, \
            snapshot_json TEXT NOT NULL, created_at BIGINT NOT NULL, step_id TEXT)",
        // Loop 节点可恢复检查点。复合主键 (execution_id, node_id)：
        // 同一 execution 内多个 Loop 节点并存时互不干扰。payload_json
        // 序列化为 axagent_core::workflow_types::LoopCheckpoint。
        "CREATE TABLE IF NOT EXISTS loop_checkpoints (\
            execution_id TEXT NOT NULL, node_id TEXT NOT NULL, \
            payload_json TEXT NOT NULL, updated_at BIGINT NOT NULL, \
            PRIMARY KEY(execution_id, node_id))",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // SECTION C: Gateway / Tools tables
    // ========================================================================

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
        // scheduled_tasks — 死表，CronJobStore 纯内存不碰 SQLite
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // SECTION D: Knowledge extension tables
    // ========================================================================

    for sql in &[
        "CREATE TABLE IF NOT EXISTS notes (\
            id TEXT NOT NULL PRIMARY KEY, vault_id TEXT NOT NULL, title TEXT NOT NULL, \
            file_path TEXT NOT NULL, content TEXT NOT NULL, content_hash TEXT NOT NULL, \
            author TEXT NOT NULL, page_type TEXT, source_refs TEXT, related_pages TEXT, \
            quality_score REAL, last_linted_at BIGINT, last_compiled_at BIGINT, \
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
            is_required INTEGER NOT NULL DEFAULT 0, default_value TEXT, constraints TEXT, \
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
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // SECTION E: Prompt tables
    // ========================================================================

    for sql in &[
        "CREATE TABLE IF NOT EXISTS prompt_templates (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            content TEXT NOT NULL, variables_schema TEXT, version INTEGER NOT NULL DEFAULT 1, \
            is_active INTEGER NOT NULL DEFAULT 1, ab_test_enabled INTEGER NOT NULL DEFAULT 0, \
            ab_test_variant TEXT, \
            category TEXT, tags TEXT, author TEXT, source TEXT, source_type TEXT, \
            format TEXT DEFAULT 'plain', metadata_json TEXT, \
            usage_count INTEGER NOT NULL DEFAULT 0, is_favorite INTEGER NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS prompt_template_versions (\
            id TEXT NOT NULL PRIMARY KEY, template_id TEXT NOT NULL, version INTEGER NOT NULL, \
            name TEXT NOT NULL, description TEXT, content TEXT NOT NULL, \
            variables_schema TEXT, changelog TEXT, \
            category TEXT, tags TEXT, author TEXT, source TEXT, \
            created_at BIGINT NOT NULL)",
        // background_tasks
        "CREATE TABLE IF NOT EXISTS background_tasks (\
            id TEXT NOT NULL PRIMARY KEY, title TEXT NOT NULL, \
            description TEXT NOT NULL DEFAULT '', task_type TEXT NOT NULL, command TEXT, \
            prompt TEXT, status TEXT NOT NULL DEFAULT 'pending', \
            output TEXT NOT NULL DEFAULT '', exit_code INTEGER, conversation_id TEXT, \
            created_by TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            finished_at BIGINT)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // SECTION F: Wiki extension tables
    // ========================================================================

    for sql in &[
        "CREATE TABLE IF NOT EXISTS wiki_templates (\
            id TEXT NOT NULL PRIMARY KEY, wiki_id TEXT NOT NULL, name TEXT NOT NULL, \
            description TEXT, content TEXT NOT NULL, page_type TEXT, \
            is_builtin INTEGER NOT NULL DEFAULT 0, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS wiki_page_versions (\
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, wiki_id TEXT NOT NULL, \
            note_id TEXT NOT NULL, title TEXT NOT NULL, content TEXT NOT NULL, \
            content_hash TEXT NOT NULL, author TEXT NOT NULL, created_at INTEGER NOT NULL)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // SECTION G: Trajectory tables (raw SQL from original migration)
    // ========================================================================

    for sql in &[
        "CREATE TABLE IF NOT EXISTS trajectory_trajectories (\
            id TEXT PRIMARY KEY, session_id TEXT NOT NULL, user_id TEXT NOT NULL, \
            topic TEXT NOT NULL, summary TEXT NOT NULL, outcome TEXT NOT NULL, \
            duration_ms INTEGER NOT NULL, quality_overall REAL NOT NULL, \
            quality_task_completion REAL NOT NULL, quality_tool_efficiency REAL NOT NULL, \
            quality_reasoning_quality REAL NOT NULL, quality_user_satisfaction REAL NOT NULL, \
            value_score REAL NOT NULL, patterns TEXT NOT NULL, created_at TEXT NOT NULL, \
            replay_count INTEGER NOT NULL DEFAULT 0, last_replay_at TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_steps (\
            id INTEGER PRIMARY KEY AUTOINCREMENT, trajectory_id TEXT NOT NULL, \
            step_index INTEGER NOT NULL, timestamp_ms INTEGER NOT NULL, role TEXT NOT NULL, \
            content TEXT NOT NULL, reasoning TEXT, tool_calls TEXT, tool_results TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_rewards (\
            id TEXT PRIMARY KEY, trajectory_id TEXT NOT NULL, reward_type TEXT NOT NULL, \
            value REAL NOT NULL, created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_skills (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, \
            skill_type TEXT NOT NULL, content TEXT NOT NULL, category TEXT NOT NULL, \
            tags TEXT NOT NULL, scenarios TEXT NOT NULL DEFAULT '[]', \
            parameters TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, \
            usage_count INTEGER NOT NULL DEFAULT 0, success_rate REAL NOT NULL DEFAULT 0.0, \
            avg_execution_time_ms REAL NOT NULL DEFAULT 0.0)",
        "CREATE TABLE IF NOT EXISTS trajectory_skill_executions (\
            id TEXT PRIMARY KEY, skill_id TEXT NOT NULL, trajectory_id TEXT, \
            success INTEGER NOT NULL, execution_time_ms INTEGER NOT NULL, \
            created_at TEXT NOT NULL, input_args TEXT, output_result TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_patterns (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, \
            pattern_type TEXT NOT NULL, trajectory_ids TEXT NOT NULL, \
            frequency INTEGER NOT NULL, success_rate REAL NOT NULL, \
            average_quality REAL NOT NULL, average_value_score REAL NOT NULL, \
            reward_profile TEXT NOT NULL, created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_entities (\
            id TEXT PRIMARY KEY, name TEXT NOT NULL, entity_type TEXT NOT NULL, \
            properties TEXT NOT NULL DEFAULT '{}', aliases TEXT NOT NULL DEFAULT '[]', \
            first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL, \
            mention_count INTEGER NOT NULL DEFAULT 1, confidence REAL NOT NULL DEFAULT 0.5, \
            created_at TEXT, updated_at TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_relationships (\
            id TEXT PRIMARY KEY, source_id TEXT NOT NULL, target_id TEXT NOT NULL, \
            relation_type TEXT NOT NULL, properties TEXT NOT NULL DEFAULT '{}', \
            weight REAL NOT NULL DEFAULT 1.0, created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_sessions (\
            id TEXT PRIMARY KEY, title TEXT NOT NULL, \
            platform TEXT NOT NULL DEFAULT 'web', user_id TEXT NOT NULL DEFAULT 'default', \
            model TEXT NOT NULL DEFAULT 'unknown', system_prompt TEXT NOT NULL DEFAULT '', \
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL, parent_session_id TEXT, \
            token_input INTEGER NOT NULL DEFAULT 0, token_output INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS trajectory_messages (\
            id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL, \
            content TEXT NOT NULL, tool_calls TEXT, tool_results TEXT, usage TEXT, \
            created_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS trajectory_memories (\
            id TEXT PRIMARY KEY, memory_type TEXT NOT NULL, content TEXT NOT NULL, \
            updated_at TEXT NOT NULL, \
            tier TEXT NOT NULL DEFAULT 'working', importance REAL NOT NULL DEFAULT 0.5, \
            access_count INTEGER NOT NULL DEFAULT 0, last_accessed TEXT, \
            decay_rate REAL NOT NULL DEFAULT 0.01, created_at TEXT, expires_at TEXT, \
            source_conversation_id TEXT, source_message_id TEXT, \
            memory_nature TEXT NOT NULL DEFAULT 'semantic', tags TEXT NOT NULL DEFAULT '[]', \
            namespace_id TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_learned_patterns (\
            id TEXT PRIMARY KEY, pattern TEXT NOT NULL, pattern_type TEXT NOT NULL, \
            success INTEGER NOT NULL DEFAULT 0, failure INTEGER NOT NULL DEFAULT 0, \
            last_used TEXT NOT NULL, created_at TEXT NOT NULL, metadata TEXT)",
        "CREATE TABLE IF NOT EXISTS trajectory_preferences (\
            id TEXT PRIMARY KEY, key TEXT NOT NULL UNIQUE, value TEXT NOT NULL, \
            confidence REAL NOT NULL DEFAULT 0.0, updated_at TEXT NOT NULL)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // SECTION H: FTS5 Virtual Tables
    // ========================================================================

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

    // ========================================================================
    // SECTION I: FTS5 Triggers (messages_fts content-sync)
    // ========================================================================

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

    // ========================================================================
    // SECTION J: v001 时期就有的索引（v001_initial 不补新索引——那是 v002 的事）
    // ========================================================================

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

    Ok(())
}
