// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务工作流种子化（行业数据资产包驱动）

use axagent_entities::workflow_template;
use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;

mod industry_pack;
mod seed_industries;
mod seed_production;
mod seed_extended;

pub use industry_pack::ensure_opc_industries_seeded;
pub use industry_pack::INDUSTRIES_DIR;
pub use seed_extended::seed_all_workflows;
pub use seed_production::seed_landing_page_workflow;
pub use seed_production::seed_startup_mvp_workflow;

/// 行业包根目录（相对仓库根，由调用方拼接）
pub fn industries_base_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(INDUSTRIES_DIR)
}

const OPC_TEMPLATE_VERSION: i32 = 1;

/// 主入口：行业包 seed + 领域/生产工作流。
///
/// 行业包路径解析：优先 `app_dir/config/opc/industries`（生产，
/// 随配置拷贝到用户数据目录），不存在则 fallback 仓库根
/// `config/opc/industries`（开发/测试）。`app_dir` 传 None 时直接用
/// 仓库根（测试场景）。
pub async fn ensure_opc_workflows_seeded(
    db: &DatabaseConnection,
    app_dir: Option<&std::path::Path>,
) -> Result<(), String> {
    // 1) 行业包驱动（9 大行业，各自独立启用/禁用/版本）
    let base = resolve_industries_dir(app_dir);
    tracing::info!("[opc-workflows] Industry pack dir: {}", base.display());
    let seeded = ensure_opc_industries_seeded(db, &base).await?;
    tracing::info!("[opc-workflows] Industry packs seeded: {seeded:?}");

    // 2) 领域工作流（seed_extended，18 领域通用工作流）
    seed_all_workflows(db).await?;

    // 3) 生产工作流（landing page / startup MVP）
    seed_landing_page_workflow(db).await?;
    seed_startup_mvp_workflow(db).await?;

    tracing::info!("[opc-workflows] All workflows seeded");
    Ok(())
}

/// 行业包目录解析：app_dir/config/opc/industries → 仓库根 fallback。
pub fn resolve_industries_dir(app_dir: Option<&std::path::Path>) -> std::path::PathBuf {
    if let Some(dir) = app_dir {
        let candidate = dir.join(INDUSTRIES_DIR);
        if candidate.is_dir() {
            return candidate;
        }
    }
    industries_base_dir()
}

// ── OPC Tool 定义 ────────────────────────────────────────────────

pub(crate) fn opc_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef { name: "OpcListInvoices".into(), description: Some("查询发票列表，按状态/客户/日期过滤".into()), parameters: None },
        ToolDef { name: "OpcCreateInvoice".into(), description: Some("创建发票，需客户ID和行项目".into()), parameters: None },
        ToolDef { name: "OpcTransitionInvoice".into(), description: Some("变更发票状态: draft→sent→paid→refunded".into()), parameters: None },
        ToolDef { name: "OpcListCustomers".into(), description: Some("查询客户列表".into()), parameters: None },
        ToolDef { name: "OpcCreateCustomer".into(), description: Some("创建客户记录".into()), parameters: None },
        ToolDef { name: "OpcListProjects".into(), description: Some("查询项目列表".into()), parameters: None },
        ToolDef { name: "OpcCreateProject".into(), description: Some("创建项目".into()), parameters: None },
        ToolDef { name: "OpcAddMilestone".into(), description: Some("为项目添加里程碑".into()), parameters: None },
        ToolDef { name: "OpcGetDashboard".into(), description: Some("运营概览: 收入/发票/项目/客户".into()), parameters: None },
        ToolDef { name: "OpcSendNotification".into(), description: Some("发送消息通知(Telegram/钉钉等)".into()), parameters: None },
        ToolDef { name: "OpcRecordKpi".into(), description: Some("记录 KPI 指标".into()), parameters: None },
        ToolDef { name: "OpcListKpis".into(), description: Some("查询 KPI 记录".into()), parameters: None },
        ToolDef { name: "OpcGetFinancialReport".into(), description: Some("财务报表(收入/利润/投资建议)".into()), parameters: None },
        ToolDef { name: "OpcSearchWiki".into(), description: Some("搜索 OPC Wiki 知识文档".into()), parameters: None },
    ]
}

// ── 节点构建辅助 ─────────────────────────────────────────────────

pub(crate) fn make_base(id: &str, title: &str, desc: &str, x: f64, y: f64) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.into(), title: title.into(), description: Some(desc.into()),
        position: Position { x, y },
        retry: RetryConfig::default(), timeout: Some(300), enabled: true,
        parent_id: None, compensation: None, continue_on_fail: false,
    }
}

pub(crate) fn default_agent_config(system_prompt: &str, profile_id: &str, tool_set: &[&str], tools: &[ToolDef]) -> AgentNodeConfig {
    let profile_tools: Vec<ToolDef> = tools.iter()
        .filter(|t| tool_set.contains(&t.name.as_str()))
        .cloned().collect();
    AgentNodeConfig {
        system_prompt: system_prompt.into(), context_sources: vec![],
        output_var: format!("{profile_id}_result"),
        model: None, temperature: None, max_tokens: None,
        tools: profile_tools,
        exposed_tools: tool_set.iter().map(|s| s.to_string()).collect(),
        output_mode: OutputMode::Json,
        agent_profile_id: Some(format!("opc-{profile_id}")),
        max_tool_rounds: Some(10), execution_mode: None,
        rag_source_ids: vec![], model_role: Some("opc-worker".to_string()),
        consistency_check: None,
        hallucination_guard: Some(axagent_harness::hallucination_guard::HallucinationGuardConfig {
            enabled: true, match_threshold: 0.4,
        }),
            fallback_model: None,
            task_scene: None,
            stream_chunk_timeout_secs: None,
        input_mapping: std::collections::HashMap::new(),
    }
}

/// 将 WorkflowTemplateData 转为 ActiveModel 并写入
pub(crate) async fn upsert_template(
    db: &DatabaseConnection, data: WorkflowTemplateData,
) -> Result<(), String> {
    use axagent_entities::workflow_template;
    use sea_orm::*;

    let tags_json = serde_json::to_string(&data.tags).unwrap_or_default();
    let nodes_json = serde_json::to_string(&data.nodes).map_err(|e| format!("nodes json: {e}"))?;
    let edges_json = serde_json::to_string(&data.edges).map_err(|e| format!("edges json: {e}"))?;
    let vars_json = serde_json::to_string(&data.variables).unwrap_or_default();
    let tools_json = serde_json::to_string(&data.tool_defs).unwrap_or_default();
    let trigger_json = data.trigger_config.as_ref().and_then(|t| serde_json::to_string(t).ok());
    let input_json = data.input_schema.as_ref().and_then(|s| serde_json::to_string(s).ok());
    let output_json = data.output_schema.as_ref().and_then(|s| serde_json::to_string(s).ok());
    let error_json = data.error_config.as_ref().and_then(|e| serde_json::to_string(e).ok());

    let am = workflow_template::ActiveModel {
        id: Set(data.id.clone()),
        name: Set(data.name),
        description: Set(data.description),
        icon: Set(data.icon),
        tags: Set(Some(tags_json)),
        version: Set(data.version),
        is_preset: Set(data.is_preset),
        is_editable: Set(data.is_editable),
        is_public: Set(data.is_public),
        trigger_config: Set(trigger_json),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(input_json),
        output_schema: Set(output_json),
        variables: Set(Some(vars_json)),
        error_config: Set(error_json),
        composite_source: Set(None),
        mission_hash: Set(data.mission_hash.clone()),
        tool_defs: Set(Some(tools_json)),
        created_at: Set(data.created_at),
        updated_at: Set(data.updated_at),
    };

    workflow_template::Entity::insert(am)
        .on_conflict(
            sea_query::OnConflict::column(workflow_template::Column::Id)
                .update_columns([
                    workflow_template::Column::Name,
                    workflow_template::Column::Description,
                    workflow_template::Column::Icon,
                    workflow_template::Column::Tags,
                    workflow_template::Column::Version,
                    workflow_template::Column::Nodes,
                    workflow_template::Column::Edges,
                    workflow_template::Column::InputSchema,
                    workflow_template::Column::OutputSchema,
                    workflow_template::Column::Variables,
                    workflow_template::Column::ErrorConfig,
                    workflow_template::Column::ToolDefs,
                    workflow_template::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map_err(|e| format!("upsert template: {e}"))?;

    Ok(())
}

pub(crate) async fn check_template_version(db: &DatabaseConnection, id: &str, version: i32) -> Result<bool, String> {
    use sea_orm::EntityTrait;
    if let Ok(Some(existing)) = workflow_template::Entity::find_by_id(id).one(db).await {
        if existing.version >= version { return Ok(false); }
        tracing::info!("[opc-workflows] {} v{} → v{}", id, existing.version, version);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::industry_pack::scan_industry_packs;
    use sea_orm::{ConnectionTrait, EntityTrait, PaginatorTrait};

    #[tokio::test]
    async fn industry_pack_migration_creates_registry() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='opc_industries'",
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "opc_industries 表应存在（v211 迁移）");
        assert!(axagent_dao::migrations::CURRENT_VERSION >= 211);
    }

    #[tokio::test]
    async fn industry_pack_seed_registers_industries() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/industries");

        let manifests = scan_industry_packs(&base);
        assert!(!manifests.is_empty(), "scan_industry_packs 不应为空");

        let seeded = ensure_opc_industries_seeded(db, &base).await.expect("seed 成功");
        assert_eq!(seeded.len(), 9, "应 seed 9 行业: {seeded:?}");

        use axagent_opc_entities::opc_industries;
        let count = opc_industries::Entity::find().count(db).await.unwrap();
        assert_eq!(count, 9, "opc_industries 应有 9 行");

        use axagent_entities::workflow_template;
        let fi = workflow_template::Entity::find_by_id("workflow-finance-invest")
            .one(db)
            .await
            .unwrap()
            .expect("workflow-finance-invest 应存在");
        assert!(fi.nodes.contains("a-advice"), "节点应含投资建议");
    }

    #[tokio::test]
    async fn industry_pack_disabled_industry_not_seeded() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let tmp = std::env::temp_dir().join(format!("opc-test-{}", std::process::id()));
        let dir = tmp.join("disabled_test");
        let wf_dir = dir.join("workflows");
        std::fs::create_dir_all(&wf_dir).unwrap();
        std::fs::write(
            dir.join("manifest.yaml"),
            "id: disabled_test\nname: 禁用测试\nversion: 1\nenabled: false\n",
        )
        .unwrap();
        std::fs::write(
            wf_dir.join("d.yaml"),
            "id: workflow-disabled-test\nname: 禁用工作流\ndescription: ''\nicon: ''\ntags: []\nprofile_id: opc-ceo-ceo-business-strategist\nsteps:\n  - id: a1\n    title: 步骤1\n    prompt: 测试\n    inputs: {}\n",
        )
        .unwrap();

        let seeded = ensure_opc_industries_seeded(db, &tmp).await.unwrap();
        assert!(!seeded.contains(&"disabled_test".to_string()));

        use axagent_entities::workflow_template;
        let wf = workflow_template::Entity::find_by_id("workflow-disabled-test")
            .one(db)
            .await
            .unwrap();
        assert!(wf.is_none(), "禁用行业的工作流不应写入");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
