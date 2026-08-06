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

// ── 配置目录同步（CWD 无关）────────────────────────────────

/// OPC 配置根目录常量（相对仓库根）
pub const OPC_CONFIG_DIR: &str = "config/opc";

/// 递归拷贝目录（仅文件与子目录，保持结构）
pub fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// 递归增量拷贝：仅补目标**缺失**的文件，已存在（含用户编辑）一律保留不覆盖。
///
/// v1.1 行业独立版：行业包目录已存在于 app_dir 时，把包内新增资产
/// （如 learning.yaml、新增 workflows）补进生产目录。返回拷贝文件数。
pub fn copy_dir_incremental(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<u32> {
    let mut copied = 0u32;
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copied += copy_dir_incremental(&from, &to)?;
        } else if !to.exists() {
            std::fs::copy(&from, &to)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// 探测仓库根下的 `rel` 相对目录（不依赖 CWD）。
///
/// 依次尝试：
/// 1. 当前工作目录（dev：仓库根）
/// 2. 当前工作目录下的 `src-tauri`（从 src-tauri 目录启动时）
/// 3. 可执行文件所在目录的上两级（exe 位于 `src-tauri/target/{profile}/`）
pub fn find_repo_config_dir(rel: &str) -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(rel));
        candidates.push(cwd.join("src-tauri").join(rel));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("..").join(rel));
            candidates.push(parent.join("../..").join(rel));
        }
    }
    candidates.into_iter().find(|p| p.is_dir())
}

/// 启动时确保 `config/opc`（行业包 + 领域包）同步到 `app_dir/config/opc`。
///
/// 生产/服务模式下进程 CWD 不是仓库根，`resolve_industries_dir` /
/// `resolve_domains_dir` 的仓库根 fallback 必然失败；将仓库根的资产
/// 同步一份到用户数据目录，使 app_dir 分支始终可用。
///
/// P2-4 修复：原实现"目标已含任一 manifest 则整体跳过"，新增行业包永远
/// 推不到生产目录。改为**增量同步**——只补目标缺失的行业/领域包与散文件，
/// 已存在（含用户导入/编辑的包）一律保留不覆盖。
pub fn ensure_opc_config_synced(app_dir: &std::path::Path) {
    let Some(src) = find_repo_config_dir(OPC_CONFIG_DIR) else {
        tracing::warn!("[opc-workflows] 仓库根 OPC 配置目录未找到，跳过同步: {}", OPC_CONFIG_DIR);
        return;
    };
    let target_dir = app_dir.join(OPC_CONFIG_DIR);

    let mut copied = 0u32;
    if let Ok(entries) = std::fs::read_dir(&src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let name = entry.file_name();
            let dst_path = target_dir.join(&name);
            if src_path.is_dir() {
                if !dst_path.is_dir() {
                    if copy_dir_recursive(&src_path, &dst_path).is_ok() {
                        copied += 1;
                    }
                } else {
                    // 已存在目录：内部文件级增量（industries/{id}、domains/{id} 缺失文件，
                    // 如新增 learning.yaml / workflows；已存在文件保留不覆盖）
                    if let Ok(inner) = std::fs::read_dir(&src_path) {
                        for sub in inner.flatten() {
                            let sub_name = sub.file_name();
                            let sub_dst = dst_path.join(&sub_name);
                            let n = if sub.path().is_dir() {
                                if sub_dst.is_dir() {
                                    // 子目录已存在：递归补缺失文件
                                    copy_dir_incremental(&sub.path(), &sub_dst).unwrap_or(0)
                                } else if copy_dir_recursive(&sub.path(), &sub_dst).is_ok() {
                                    1
                                } else {
                                    0
                                }
                            } else if !sub_dst.exists()
                                && std::fs::copy(sub.path(), &sub_dst).is_ok()
                            {
                                1
                            } else {
                                0
                            };
                            copied += n;
                        }
                    }
                }
            } else if !dst_path.exists() && std::fs::copy(&src_path, &dst_path).is_ok() {
                copied += 1;
            }
        }
    }

    if copied > 0 {
        tracing::info!(
            "[opc-workflows] OPC 配置增量同步 {} 项: {} → {}",
            copied,
            src.display(),
            target_dir.display()
        );
    }
}

// ── 行业适配器配置加载（P0-1-A：行业包驱动，消灭 Rust 硬编码） ────

/// 从行业包目录加载全部行业适配器（`learning.yaml` 的 `adapter:` 段驱动）。
///
/// P0-1-A：替代 orchestrator `create_all_adapters()` 的 Rust 硬编码 9 行业配置；
/// 动态扫描 `config/opc/industries/*/`，新增行业无需改代码。
/// `adapter` 段缺失（旧包）→ 默认适配器（向后兼容）；解析失败仅告警跳过该行业。
pub fn load_industry_adapters_from_packs(
    app_dir: Option<&std::path::Path>,
) -> Vec<std::sync::Arc<dyn axagent_orchestrator::IndustryAdapter>> {
    use axagent_orchestrator::industry_adapters::BaseIndustryAdapter;

    let base = resolve_industries_dir(app_dir);
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&base) else { return out };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(bundle) = industry_pack::analysis_schema::load_industry_pack(&dir) else {
            continue;
        };
        let m = &bundle.manifest;
        // 行业 ID 双轨归一：manifest.id 是下划线（software_dev），orchestrator
        // 学习/编排侧约定连字符（software-dev）——与 learning hook 的
        // `identify_industry_from_template` 转换一致（P4-4）。
        let industry_id = m.id.replace('_', "-");
        let learning_path = dir.join(&m.learning);
        let adapter_cfg = std::fs::read_to_string(&learning_path)
            .ok()
            .and_then(|c| serde_yaml::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| v.get("adapter").cloned())
            .unwrap_or(serde_json::Value::Null);
        match BaseIndustryAdapter::from_config_json(&industry_id, &m.name, &adapter_cfg) {
            Ok(a) => {
                out.push(std::sync::Arc::new(a)
                    as std::sync::Arc<dyn axagent_orchestrator::IndustryAdapter>);
            },
            Err(e) => tracing::warn!("[opc-adapter] 行业 {} 适配器配置解析失败: {e}", m.id),
        }
    }
    out
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

/// P2-2：清理行业/领域包升级后残留的旧模板。
///
/// 包内 yaml 删除或改 id 后，DB 中旧的 preset 模板不会消失（upsert 只更新存在项）。
/// 按 `prefix` 匹配该包历史 seed 的模板，删除不在 `keep` 集合中的残留。
pub(crate) async fn cleanup_stale_pack_templates(
    db: &DatabaseConnection,
    prefix: &str,
    keep: &[String],
) -> Result<u32, String> {
    use sea_orm::*;

    let stale = workflow_template::Entity::find()
        .filter(workflow_template::Column::Id.like(format!("{prefix}%")))
        .filter(workflow_template::Column::IsPreset.eq(true))
        .all(db)
        .await
        .map_err(|e| format!("查询旧模板失败: {e}"))?;

    let mut removed = 0u32;
    for t in stale {
        if keep.iter().any(|k| k == &t.id) {
            continue;
        }
        workflow_template::Entity::delete_by_id(&t.id)
            .exec(db)
            .await
            .map_err(|e| format!("删除旧模板 {} 失败: {e}", t.id))?;
        tracing::info!("[opc-workflows] 清理旧模板 {}", t.id);
        removed += 1;
    }
    Ok(removed)
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

    /// 最终验收：9 行业 seed 产物端到端断言——工具注入/审批边/error_config/variables/profile 全部就绪。
    #[tokio::test]
    async fn industry_packs_end_to_end_verification() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/industries");
        ensure_opc_industries_seeded(db, &base).await.expect("seed 9 行业");

        use axagent_entities::workflow_template;
        use sea_orm::EntityTrait;

        // 1. 9 行业 10 个工作流模板全部存在
        for id in [
            "workflow-accounting",
            "workflow-ai-research",
            "workflow-content-media",
            "workflow-ecommerce",
            "workflow-education",
            "workflow-finance-invest",
            "workflow-finance-stock-research",
            "workflow-industry-consulting",
            "workflow-sales-growth",
            "workflow-software-dev",
        ] {
            let t = workflow_template::Entity::find_by_id(id).one(db).await.unwrap();
            assert!(t.is_some(), "{id} 应被 seed");
        }

        // 2. 工具注入：通用工具（P1-1 local_tool_defs）/ OPC 工具 / astock 工具
        let sdev = workflow_template::Entity::find_by_id("workflow-software-dev")
            .one(db)
            .await
            .unwrap()
            .expect("software_dev 存在");
        assert!(
            sdev.nodes.contains("FileRead")
                && sdev.nodes.contains("Bash")
                && sdev.nodes.contains("Grep"),
            "software_dev 应注入通用工具（FileRead/Bash/Grep）: {}",
            sdev.nodes
        );
        let cm = workflow_template::Entity::find_by_id("workflow-content-media")
            .one(db)
            .await
            .unwrap()
            .expect("content_media 存在");
        assert!(cm.nodes.contains("OpcCreateBlogPost"), "content_media 应注入 OPC 工具");
        let fin = workflow_template::Entity::find_by_id("workflow-finance-stock-research")
            .one(db)
            .await
            .unwrap()
            .expect("finance_stock 存在");
        assert!(
            fin.nodes.contains("get_stock_quote") && fin.nodes.contains("search_stock"),
            "finance_stock 应注入 astock 工具"
        );
        let acc = workflow_template::Entity::find_by_id("workflow-accounting")
            .one(db)
            .await
            .unwrap()
            .expect("accounting 存在");
        assert!(acc.nodes.contains("OpcCreateInvoice"), "accounting 应注入 OPC 工具");

        // 3. 审批边：普通节点→approval 为 Direct（P0-1），approval→下一节点为 true 条件边
        assert!(
            !acc.edges.contains("e-a-create-approval-true"),
            "不得存在 ConditionTrue 边 e-a-create-approval-true"
        );
        assert!(acc.edges.contains("e-a-create-approval"), "应有 Direct 边 e-a-create-approval");
        assert!(acc.edges.contains("e-approval-a-notify-true"), "应有审批通过 true 条件边");

        // 4. error_config：finance-invest 声明了 error_handling → 模板 error_config 非空（P1-9）
        let fi = workflow_template::Entity::find_by_id("workflow-finance-invest")
            .one(db)
            .await
            .unwrap()
            .expect("finance-invest 存在");
        assert!(
            fi.error_config.is_some(),
            "finance-invest 应有 error_config（error_handling 生效）"
        );

        // 5. variables：{keyword} 引用收集为模板变量（P1-10）
        assert!(
            fin.variables.unwrap_or_default().contains("keyword"),
            "finance_stock 应声明 keyword 输入变量"
        );

        // 6. profile 引用：节点绑定真实存在的 OPC profile（P1-11 全名）
        assert!(acc.nodes.contains("opc-cfo-cfo-financial-analyst"), "accounting 应绑 CFO profile");
        assert!(sdev.nodes.contains("opc-cto-cto-ai-engineer"), "software_dev 应绑 CTO profile");

        // 7. 幂等：二次 seed 不报错、不产生重复（P2-2 清理不误删 keep 集）
        ensure_opc_industries_seeded(db, &base).await.expect("二次 seed 应成功");
        let count = workflow_template::Entity::find()
            .filter(workflow_template::Column::Id.like("workflow-%"))
            .count(db)
            .await
            .unwrap();
        assert_eq!(count, 10, "9 行业共 10 个工作流，二次 seed 后不应残留/重复，实际 {count}");
    }

    #[tokio::test]
    async fn approval_edges_build_correctly() {
        // P0-1 回归：普通节点 → approval 必须用 Direct 边（此前误用 ConditionTrue 导致审批断链）
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/industries");
        let wfs = super::industry_pack::load_industry_workflows(&base.join("accounting"));
        let wf =
            wfs.iter().find(|w| w.id == "workflow-accounting").expect("accounting workflow 存在");
        let data = super::industry_pack::build_workflow_from_pack(wf, 1);
        let edges: Vec<serde_json::Value> =
            serde_json::from_str(&serde_json::to_string(&data.edges).unwrap()).unwrap();

        // a-create（普通 agent）→ approval：Direct 边，无条件 handle
        let direct = edges
            .iter()
            .find(|e| e["id"] == "e-a-create-approval")
            .expect("应有 Direct 边 e-a-create-approval");
        assert!(direct["sourceHandle"].is_null(), "普通节点→approval 不得带条件 handle: {direct}");
        assert!(
            !edges.iter().any(|e| e["id"] == "e-a-create-approval-true"),
            "不得生成 e-a-create-approval-true 条件边"
        );

        // approval → 后续节点：ConditionTrue；approval → end：ConditionFalse
        let next = edges
            .iter()
            .find(|e| e["id"] == "e-approval-a-notify-true")
            .expect("approval→下一节点应有 true 条件边");
        assert_eq!(next["sourceHandle"], "true");
        let reject = edges
            .iter()
            .find(|e| e["id"] == "e-approval-end-false")
            .expect("approval→end 应有 false 条件边");
        assert_eq!(reject["sourceHandle"], "false");
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

    #[tokio::test]
    async fn industry_pack_four_assets_loaded() {
        // P0-4 回归：行业包四件套（manifest + workflows + analysis + learning）一次读全，
        // manifest.analysis/learning 字段缺省默认值，analysis.yaml 全部可解析
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config/opc/industries");

        let mut count = 0;
        for entry in std::fs::read_dir(&base).unwrap().flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let bundle = super::industry_pack::analysis_schema::load_industry_pack(&dir)
                .expect("行业包应完整加载（manifest 可解析）");
            // manifest 扩展字段：缺省默认 analysis.yaml / learning.yaml
            assert_eq!(
                bundle.manifest.analysis, "analysis.yaml",
                "{} analysis 缺省",
                bundle.manifest.id
            );
            assert_eq!(
                bundle.manifest.learning, "learning.yaml",
                "{} learning 缺省",
                bundle.manifest.id
            );
            // analysis.yaml 四件套之一：必须存在且可解析
            assert!(
                bundle.analysis.is_some(),
                "{} 缺 analysis.yaml（P0-4 四件套要求）",
                bundle.manifest.id
            );
            let analysis = bundle.analysis.unwrap();
            assert!(!analysis.data_sources.is_empty(), "{} data_sources 非空", bundle.manifest.id);
            assert!(
                analysis
                    .quality_precheck
                    .iter()
                    .all(|s| analysis.data_sources.iter().any(|ds| ds.id == *s)),
                "{} quality_precheck 源必须存在于 data_sources",
                bundle.manifest.id
            );
            // learning.yaml 四件套之一：P4-3 已迁入行业包
            assert!(
                dir.join("learning.yaml").is_file(),
                "{} 缺 learning.yaml（P4-3 要求）",
                bundle.manifest.id
            );
            count += 1;
        }
        assert_eq!(count, 9, "应扫描到 9 个行业包，实际 {count}");
    }

    #[test]
    fn industry_adapters_loaded_from_packs() {
        // P0-1-A 回归：行业适配器由行业包 learning.yaml 的 adapter 段驱动
        //（替代 orchestrator create_all_adapters Rust 硬编码）。
        // 用 accounting 已知配置对账：3 checkpoints + 3 AC + min/max 2/15 + protected compliance_check。
        // 测试 CWD=src-tauri，相对路径落空 → 显式传仓库根（模拟 app_dir 命中分支）。
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let adapters = super::load_industry_adapters_from_packs(Some(repo_root));
        assert_eq!(adapters.len(), 9, "应加载 9 个行业适配器: {}", adapters.len());

        let accounting = adapters
            .iter()
            .find(|a| a.industry_id() == "accounting")
            .expect("accounting 适配器应存在");
        assert_eq!(accounting.industry_name(), "会计财务流程");

        let rt = accounting.reflection_template();
        assert_eq!(rt.id, "accounting-default", "reflection_template.id 应对账 yaml");
        assert_eq!(rt.checkpoints.len(), 3, "accounting 应有 3 个检查点");
        assert!(rt.checkpoints.iter().any(|c| c.id == "accuracy" && (c.weight - 0.5).abs() < 1e-9));

        let ec = accounting.evolution_constraints();
        assert_eq!(ec.min_steps, 2, "min_steps 应对账 yaml");
        assert_eq!(ec.max_steps, 15, "max_steps 应对账 yaml");
        assert!(ec.protected_steps.iter().any(|p| p.step_id == "compliance_check"));
        assert!(
            ec.forbidden_optimizations.iter().any(|f| f.optimization_type == "skip_compliance")
        );
        assert!((ec.quality_thresholds.min_accuracy - 0.95).abs() < 1e-9);

        let ac = accounting.acceptance_criteria();
        assert_eq!(ac.len(), 3, "accounting 应有 3 条验收标准");
        assert!(ac.iter().any(|c| c.id == "ac-accuracy" && c.is_critical));

        // software-dev：唯一带 protected/deps/forbidden + must_follow_order 的行业
        let sd = adapters
            .iter()
            .find(|a| a.industry_id() == "software-dev")
            .expect("software-dev 适配器应存在");
        assert!(sd.evolution_constraints().must_follow_order, "software-dev 应 must_follow_order");
        assert_eq!(sd.evolution_constraints().protected_steps.len(), 3);
        assert_eq!(sd.acceptance_criteria().len(), 4);

        // 新增行业零代码：临时目录建 manifest + learning.yaml(adapter 段) → 动态出现
        let tmp = std::env::temp_dir().join(format!("opc-adapter-test-{}", std::process::id()));
        let pkg = tmp.join("config/opc/industries/mock_industry");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("manifest.yaml"),
            "id: mock-industry\nname: 模拟行业\nversion: 1\nenabled: true\n",
        )
        .unwrap();
        std::fs::write(
            pkg.join("learning.yaml"),
            "version: 1\nindustry_id: mock-industry\nadapter:\n  reflection_template:\n    id: mock\n    name: Mock 模板\n    checkpoints:\n      - id: c1\n        name: C1\n        dimension: d\n        description: desc\n        weight: 0.5\n",
        )
        .unwrap();
        let adapters2 = super::load_industry_adapters_from_packs(Some(&tmp));
        let mock = adapters2
            .iter()
            .find(|a| a.industry_id() == "mock-industry")
            .expect("新增行业应自动加载（零代码）");
        assert_eq!(mock.reflection_template().id, "mock");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
