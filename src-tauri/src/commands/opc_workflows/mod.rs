// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务工作流种子化（行业数据资产包驱动）

use axagent_entities::workflow_template;
use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;

mod industry_pack;
mod seed_production;

pub use industry_pack::IndustryManifest;
pub use industry_pack::ensure_opc_domains_seeded;
pub use industry_pack::ensure_opc_industries_seeded;
pub use industry_pack::export_industry_pack;
pub use industry_pack::import_industry_pack;
pub use industry_pack::{DOMAINS_DIR, INDUSTRIES_DIR};
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

    // 2) 领域包驱动（17 领域 75 工作流，数据资产包）
    let domains_base = resolve_domains_dir(app_dir);
    let domains = ensure_opc_domains_seeded(db, &domains_base).await?;
    tracing::info!("[opc-workflows] Domain packs seeded: {domains:?}");

    // 3) 生产工作流（landing page / startup MVP）
    seed_landing_page_workflow(db).await?;
    seed_startup_mvp_workflow(db).await?;

    tracing::info!("[opc-workflows] All workflows seeded");
    Ok(())
}

/// 领域包目录解析：app_dir/config/opc/domains → 仓库根 fallback。
pub fn resolve_domains_dir(app_dir: Option<&std::path::Path>) -> std::path::PathBuf {
    if let Some(dir) = app_dir {
        let candidate = dir.join(DOMAINS_DIR);
        if candidate.is_dir() {
            return candidate;
        }
    }
    std::path::PathBuf::from(DOMAINS_DIR)
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

// ── 节点构建辅助 ─────────────────────────────────────────────────

pub(crate) fn make_base(id: &str, title: &str, desc: &str, x: f64, y: f64) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.into(),
        title: title.into(),
        description: Some(desc.into()),
        position: Position { x, y },
        retry: RetryConfig::default(),
        timeout: Some(300),
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    }
}

/// 将 WorkflowTemplateData 转为 ActiveModel 并写入
pub(crate) async fn upsert_template(
    db: &DatabaseConnection,
    data: WorkflowTemplateData,
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

pub(crate) async fn check_template_version(
    db: &DatabaseConnection,
    id: &str,
    version: i32,
) -> Result<bool, String> {
    use sea_orm::EntityTrait;
    if let Ok(Some(existing)) = workflow_template::Entity::find_by_id(id).one(db).await {
        if existing.version >= version {
            return Ok(false);
        }
        tracing::info!("[opc-workflows] {} v{} → v{}", id, existing.version, version);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::industry_pack::scan_industry_packs;
    use super::*;
    use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};

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
        // 编译期常量断言（clippy: assertions-on-constants）
        const {
            assert!(axagent_dao::migrations::CURRENT_VERSION >= 211);
        }
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
        let wf =
            workflow_template::Entity::find_by_id("workflow-disabled-test").one(db).await.unwrap();
        assert!(wf.is_none(), "禁用行业的工作流不应写入");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn industry_pack_export_import_roundtrip() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/industries");
        let tmp = std::env::temp_dir().join(format!("opc-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        // 导出 finance_invest → .opcip
        let out = export_industry_pack(&base, "finance_invest", &tmp).await.expect("导出成功");
        assert!(std::path::Path::new(&out).exists(), "归档应生成");
        assert!(out.ends_with("finance_invest.opcip"), "归档名应含行业 id");

        // 导入到独立 app_dir → 注册 + seed
        let app_dir = tmp.join("app");
        let imported =
            import_industry_pack(db, &app_dir, std::path::Path::new(&out)).await.expect("导入成功");
        assert_eq!(imported, "finance_invest");

        // 解包的文件应存在
        let manifest = app_dir.join("config/opc/industries/finance_invest/manifest.yaml");
        assert!(manifest.exists(), "解包后 manifest 应存在");

        // 工作流已 seed 进 DB
        use axagent_entities::workflow_template;
        let fi =
            workflow_template::Entity::find_by_id("workflow-finance-invest").one(db).await.unwrap();
        assert!(fi.is_some(), "导入后工作流应 seed");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn domain_pack_seed_all_workflows() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/domains");

        let diag_wfs = super::industry_pack::load_industry_workflows(&base.join("engineering"));
        println!("[diag] engineering 领域包解析: {} 个工作流", diag_wfs.len());

        let seeded = ensure_opc_domains_seeded(db, &base).await.expect("领域包 seed 成功");
        assert_eq!(seeded.len(), 17, "应 seed 17 个领域: {seeded:?}");
        assert!(seeded.contains(&"engineering".to_string()));
        assert!(seeded.contains(&"finance".to_string()));

        // 抽查：engineering 的 12 个工作流已写入
        use axagent_entities::workflow_template;
        let wf = workflow_template::Entity::find_by_id("wf-eng-code-review")
            .one(db)
            .await
            .unwrap()
            .expect("wf-eng-code-review 应存在");
        assert!(wf.nodes.contains("a-review"), "节点应含 AI 审查");
        assert!(wf.nodes.contains("trigger"), "应有 trigger 节点");

        // 全部 wf- 前缀模板数 = 75
        let count = workflow_template::Entity::find()
            .filter(workflow_template::Column::Id.like("wf-%"))
            .count(db)
            .await
            .unwrap();
        assert_eq!(count, 76, "应 seed 76 个领域工作流，实际 {count}");

        // 幂等：二次 seed 不报错
        ensure_opc_domains_seeded(db, &base).await.expect("二次 seed 应成功");
    }

    #[tokio::test]
    async fn domain_pack_disabled_skipped() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let tmp = std::env::temp_dir().join(format!("opc-domain-test-{}", std::process::id()));
        let dir = tmp.join("disabled_domain");
        let wf_dir = dir.join("workflows");
        std::fs::create_dir_all(&wf_dir).unwrap();
        std::fs::write(
            dir.join("manifest.yaml"),
            "id: disabled_domain\nname: 禁用领域\nversion: 1\nenabled: false\n",
        )
        .unwrap();
        std::fs::write(
            wf_dir.join("d.yaml"),
            "id: wf-disabled-test\nname: 禁用工作流\ndescription: ''\nicon: ''\ntags: []\nprofile_id: cto\nsteps:\n  - id: a1\n    title: 步骤1\n    prompt: 测试\n    inputs: {}\n",
        )
        .unwrap();

        let seeded = ensure_opc_domains_seeded(db, &tmp).await.unwrap();
        assert!(!seeded.contains(&"disabled_domain".to_string()));

        use axagent_entities::workflow_template;
        let wf = workflow_template::Entity::find_by_id("wf-disabled-test").one(db).await.unwrap();
        assert!(wf.is_none(), "禁用领域的工作流不应写入");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn finance_pack_injects_astock_tools() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/industries");

        ensure_opc_industries_seeded(db, &base).await.expect("seed 成功");

        // 个股投资研究工作流应存在
        use axagent_entities::workflow_template;
        let wf = workflow_template::Entity::find_by_id("workflow-finance-stock-research")
            .one(db)
            .await
            .unwrap()
            .expect("workflow-finance-stock-research 应存在");

        // 节点序列化后应含工具白名单（search_stock / get_stock_quote 等）
        assert!(wf.nodes.contains("a-search"), "应含标的确认节点");
        assert!(wf.nodes.contains("get_stock_quote"), "行情节点应注入 get_stock_quote 工具");
        assert!(wf.nodes.contains("search_stock"), "标的节点应注入 search_stock 工具");
        assert!(wf.nodes.contains("get_stock_financials"), "财务节点应注入财报工具");
        assert!(wf.nodes.contains("get_fundamentals_report_markdown"), "应注入基本面报告工具");

        // 依赖连线：stock_code 从 a-search 传到 a-quote
        assert!(wf.edges.contains("e-a-search-a-quote"), "应有关键连线");
    }

    #[tokio::test]
    async fn stock_tool_defs_match_astock() {
        // stock_tool_defs 从 astock-data 匹配工具名
        let defs = super::industry_pack::stock_tool_defs(&[
            "get_stock_quote".to_string(),
            "get_stock_financials".to_string(),
        ]);
        assert_eq!(defs.len(), 2, "应匹配 2 个工具: {defs:?}");
        assert_eq!(defs[0].name, "get_stock_quote");
        assert!(defs[0].description.is_some(), "工具应有描述");
        assert!(defs[0].parameters.is_some(), "工具应有参数 schema");

        // 不存在的工具名 → 空
        let none = super::industry_pack::stock_tool_defs(&["not_a_real_tool".to_string()]);
        assert!(none.is_empty());
    }

    #[tokio::test]
    async fn talent_library_import_and_idempotent() {
        // 模拟 opc_import_talent_library：扫描目录 → 填充 opc_talent_templates → 幂等
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let org = axagent_company_runtime::OrgService::new(db);

        // 构造临时人才库
        let tmp = std::env::temp_dir().join(format!("opc-talent-test-{}", std::process::id()));
        let eng = tmp.join("engineering");
        let fin = tmp.join("finance");
        std::fs::create_dir_all(&eng).unwrap();
        std::fs::create_dir_all(&fin).unwrap();
        std::fs::write(
            eng.join("ai-engineer.md"),
            "---\nname: AI 工程师\ndescription: AI/LLM 应用开发\n---\n专家内容",
        )
        .unwrap();
        std::fs::write(
            fin.join("financial-analyst.md"),
            "---\nname: 金融分析师\ndescription: 财务报表分析\n---\n专家内容",
        )
        .unwrap();

        // 模拟导入（与 opc.rs opc_import_talent_library 相同逻辑）
        let mut imported = 0;
        for entry in std::fs::read_dir(&tmp).unwrap() {
            let dir = entry.unwrap().path();
            if !dir.is_dir() {
                continue;
            }
            let dir_name = dir.file_name().unwrap().to_string_lossy().to_string();
            for md in std::fs::read_dir(&dir).unwrap() {
                let md_path = md.unwrap().path();
                let stem = md_path.file_stem().unwrap().to_string_lossy().to_string();
                let tid = format!("tt-{dir_name}-{stem}");
                org.add_talent_template(axagent_company_runtime::org::NewTalentTemplate {
                    id: tid.clone(),
                    category: dir_name.clone(),
                    name: stem.clone(),
                    description: "导入的专家".to_string(),
                    source_repo: "agency-agents-src".to_string(),
                    prompt_refs: Some(vec![format!("{dir_name}/{stem}.md")]),
                    skill_refs: None,
                    tags: Some(vec![dir_name.clone()]),
                })
                .await
                .unwrap();
                imported += 1;
            }
        }
        assert_eq!(imported, 2);

        // 验证：2 条模板 + 按分类查
        let all = org.list_talent_templates(None).await.unwrap();
        assert_eq!(all.len(), 2);
        let eng_templates = org.list_talent_templates(Some("engineering")).await.unwrap();
        assert_eq!(eng_templates.len(), 1);
        assert!(eng_templates[0].prompt_refs.is_some(), "应记录提示词引用");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn market_list_scans_builtin_packs() {
        // 模拟 opc_market_list：扫描内置行业包目录
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/industries");
        let manifests = super::industry_pack::scan_industry_packs(&base);
        assert_eq!(manifests.len(), 9, "内置 9 个行业包");

        // 每个包有 manifest 关键字段
        for m in &manifests {
            assert!(!m.id.is_empty());
            assert!(!m.name.is_empty());
            assert!(m.version >= 1);
        }
    }
}
