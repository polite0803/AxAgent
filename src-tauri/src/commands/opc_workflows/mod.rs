// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务工作流种子化（代码驱动 + 数据资产包）

use axagent_entities::workflow_template;
use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;

pub mod domain_workflows;
mod industry_pack;
mod seed_content_media;
mod seed_domain_academic;
mod seed_domain_design;
mod seed_domain_engineering;
mod seed_domain_finance;
mod seed_domain_gamedev;
mod seed_domain_gis;
mod seed_domain_helpers;
mod seed_domain_marketing;
mod seed_domain_paidmedia;
mod seed_domain_pm;
mod seed_domain_product;
mod seed_domain_sales;
mod seed_domain_security;
mod seed_domain_spatial;
mod seed_domain_specialized;
mod seed_domain_strategy;
mod seed_domain_support;
mod seed_domain_testing;
mod seed_domain_utils;
mod seed_industry_accounting;
mod seed_industry_ai_research;
mod seed_industry_content_media;
mod seed_industry_design;
mod seed_industry_ecommerce;
mod seed_industry_education;
mod seed_industry_finance_invest;
mod seed_industry_game_dev;
mod seed_industry_geospatial;
mod seed_industry_industry_consulting;
mod seed_industry_project_management;
mod seed_industry_sales_growth;
mod seed_industry_security;
mod seed_industry_software_dev;
mod seed_production;

pub use industry_pack::INDUSTRIES_DIR;
pub use industry_pack::IndustryManifest;
#[allow(unused_imports)]
pub use industry_pack::analysis_schema::{AnalysisDataSource, IndustryAnalysisConfig};
pub use industry_pack::export_industry_pack;
pub use industry_pack::import_industry_pack;
pub use industry_pack::{local_tool_defs, opc_tool_defs, stock_tool_defs};
pub use seed_content_media::seed_content_media_workflows;
pub use seed_industry_accounting::seed_industry_accounting_workflow_template;
pub use seed_industry_ai_research::seed_industry_ai_research_workflow_template;
pub use seed_industry_content_media::seed_industry_content_media_workflow_template;
pub use seed_industry_design::seed_industry_design_workflow_template;
pub use seed_industry_ecommerce::seed_industry_ecommerce_workflow_template;
pub use seed_industry_education::seed_industry_education_workflow_template;
pub use seed_industry_finance_invest::seed_industry_finance_invest_workflow_template;
pub use seed_industry_game_dev::seed_industry_game_dev_workflow_template;
pub use seed_industry_geospatial::seed_industry_geospatial_workflow_template;
pub use seed_industry_industry_consulting::seed_industry_industry_consulting_workflow_template;
pub use seed_industry_project_management::seed_industry_project_management_workflow_template;
pub use seed_industry_sales_growth::seed_industry_sales_growth_workflow_template;
pub use seed_industry_security::seed_industry_security_workflow_template;
pub use seed_industry_software_dev::seed_industry_software_dev_workflow_template;
pub use seed_production::seed_landing_page_workflow;
pub use seed_production::seed_startup_mvp_workflow;

/// 行业包根目录（相对仓库根，由调用方拼接）
pub fn industries_base_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(INDUSTRIES_DIR)
}

const OPC_TEMPLATE_VERSION: i32 = 3; // 升级到 2 以覆盖旧 YAML 版本

/// 主入口：全代码驱动种子化（行业 + 领域 + 生产 + 内容媒体）。
///
/// 与股票分析工作流一致：手动定义 WorkflowNode/Edge →
/// 种子化写入 workflow_template 表 → 运行时 DB 加载执行。
pub async fn ensure_opc_workflows_seeded(
    db: &DatabaseConnection,
    _app_dir: Option<&std::path::Path>,
) -> Result<(), String> {
    // 1) 行业工作流（14 行业，手动定义 WorkflowNode/Edge）
    seed_opc_industries_from_seed_files(db).await?;

    // 2) 领域工作流（17 领域 75 个工作流，手动定义 seed 文件）
    let domain_seeded = seed_domains_from_seed_files(db).await?;
    tracing::info!("[opc-workflows] Domains seeded from seed files: {domain_seeded}");

    // 3) 生产工作流（landing page / startup MVP）
    seed_landing_page_workflow(db).await?;
    seed_startup_mvp_workflow(db).await?;

    // 4) 内容媒体专属工作流（爆款内容 / 多平台 / IP 打造）
    let cm_seeded = seed_content_media_workflows(db).await?;
    tracing::info!("[opc-workflows] Content media workflows seeded: {cm_seeded}");

    // 5) 回填存量缺失 route_path（幂等，保证旧版本升级后预设模板可被路由定位）
    let backfilled = backfill_missing_route_paths(db).await?;
    tracing::info!("[opc-workflows] Backfilled route_path: {backfilled}");

    tracing::info!("[opc-workflows] All workflows seeded (code-driven)");
    Ok(())
}

/// 从手动定义的 seed 文件种子化 14 个行业工作流。
///
/// 每个 seed 文件手动定义 WorkflowNode/Edge，与股票分析工作流一致。
/// 替换了旧版通过 IndustryAdapterFactory 动态生成 DAG 的方式。
async fn seed_opc_industries_from_seed_files(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded_count = 0;

    seed_industry_accounting_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_ai_research_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_content_media_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_design_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_ecommerce_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_education_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_finance_invest_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_game_dev_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_geospatial_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_industry_consulting_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_project_management_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_sales_growth_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_security_workflow_template(db).await?;
    seeded_count += 1;
    seed_industry_software_dev_workflow_template(db).await?;
    seeded_count += 1;

    tracing::info!("[opc-workflows] Industries seeded from seed files: {seeded_count}");
    Ok(seeded_count)
}

/// 从手动定义的 seed 文件种子化 17 个领域工作流。
///
/// 每个 seed 文件手动定义 WorkflowNode/Edge，通过 check_template_version +
/// upsert_template 写入数据库。与行业工作流一致，每个领域有独立的 seed 文件。
/// 注：工程领域 wf-eng-refactor 暂时保留内联生成器调用，待后续完全手动转换。
async fn seed_domains_from_seed_files(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded_count = 0;

    seeded_count += seed_domain_academic::seed_domain_academic_workflows(db).await?;
    seeded_count += seed_domain_design::seed_domain_design_workflows(db).await?;
    seeded_count += seed_domain_engineering::seed_domain_engineering_workflows(db).await?;
    seeded_count += seed_domain_finance::seed_domain_finance_workflows(db).await?;
    seeded_count += seed_domain_gamedev::seed_domain_gamedev_workflows(db).await?;
    seeded_count += seed_domain_gis::seed_domain_gis_workflows(db).await?;
    seeded_count += seed_domain_marketing::seed_domain_marketing_workflows(db).await?;
    seeded_count += seed_domain_paidmedia::seed_domain_paidmedia_workflows(db).await?;
    seeded_count += seed_domain_pm::seed_domain_pm_workflows(db).await?;
    seeded_count += seed_domain_product::seed_domain_product_workflows(db).await?;
    seeded_count += seed_domain_sales::seed_domain_sales_workflows(db).await?;
    seeded_count += seed_domain_security::seed_domain_security_workflows(db).await?;
    seeded_count += seed_domain_spatial::seed_domain_spatial_workflows(db).await?;
    seeded_count += seed_domain_specialized::seed_domain_specialized_workflows(db).await?;
    seeded_count += seed_domain_strategy::seed_domain_strategy_workflows(db).await?;
    seeded_count += seed_domain_support::seed_domain_support_workflows(db).await?;
    seeded_count += seed_domain_testing::seed_domain_testing_workflows(db).await?;

    tracing::info!("[opc-workflows] Domains seeded from seed files: {seeded_count}");
    Ok(seeded_count)
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
/// 2. 当前工作目录下的 `src-tauri`（从仓库根启动时）
/// 3. 当前工作目录的上一级（从 src-tauri 目录启动时）
/// 4. 可执行文件所在目录的上三级（exe 位于 `src-tauri/target/{profile}/`）
pub fn find_repo_config_dir(rel: &str) -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(rel));
        candidates.push(cwd.join("src-tauri").join(rel));
        candidates.push(cwd.join("..").join(rel));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("..").join(rel));
            candidates.push(parent.join("../..").join(rel));
            candidates.push(parent.join("../../../").join(rel));
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

/// OPC 行业 industry_id → CapabilityDomain L1 映射
/// （对齐 `src/lib/domainMeta.ts` 的 `NAV_ITEM_DOMAIN_MAP` 业务本质归域）。
const OPC_INDUSTRY_DOMAIN: &[(&str, &str)] = &[
    ("accounting", "finance"),
    ("ai_research", "data_analysis"),
    ("content_media", "content_creation"),
    ("design", "content_creation"),
    ("ecommerce", "automation"),
    ("education", "content_creation"),
    ("finance_invest", "finance"),
    ("game_dev", "ai_media"),
    ("geospatial", "data_analysis"),
    ("industry_consulting", "automation"),
    ("project_management", "automation"),
    ("sales_growth", "automation"),
    ("security", "devops"),
    ("software_dev", "devops"),
];

/// OPC 领域模板 wf 第二段（wf-{seg}-{slug}）→ CapabilityDomain L1 映射
const OPC_WF_SEGMENT_DOMAIN: &[(&str, &str)] = &[
    ("acd", "general"),          // academic
    ("des", "content_creation"), // design
    ("eng", "devops"),           // engineering
    ("fin", "finance"),
    ("gd", "ai_media"), // gamedev
    ("gis", "data_analysis"),
    ("mkt", "communication"), // marketing
    ("pm", "automation"),     // project management
    ("prod", "automation"),   // product
    ("sal", "automation"),    // sales
    ("sec", "devops"),        // security
    ("spatial", "data_analysis"),
    ("spc", "automation"),   // specialized
    ("strat", "automation"), // strategy
    ("sup", "automation"),   // support
    ("tst", "devops"),       // testing
];

/// stock 域预设模板 → 权威 route_path(与各 seed 文件 `Set(Some(...))` 显式值一致,
/// 供 backfill 覆盖存量 NULL,避免前端按 L1 归域落"未分类")。
const STOCK_EXPLICIT_ROUTE: &[(&str, &str)] = &[
    ("stock-analysis", "/finance/equity/multi-dim-analysis"),
    ("stock-pipeline", "/finance/pipeline/stock-pipeline"),
    ("stock-reflection", "/finance/equity/reflection"),
    ("serenity-screening", "/finance/trend/serenity"),
    ("news-to-cross-market-analysis", "/finance/cross-market/news"),
    ("daily-market-events", "/finance/market/mainlines"),
    ("screenshot-portfolio-diagnosis", "/finance/portfolio/diagnosis"),
    ("auto-position-plan", "/finance/trading/position-plan"),
    ("auto-stop-loss-review", "/finance/trading/stop-loss-review"),
    ("opc-demand-discovery", "/automation/opc/demand-discovery"),
];

fn industry_to_domain(industry_id: &str) -> Option<&'static str> {
    OPC_INDUSTRY_DOMAIN.iter().find(|(k, _)| *k == industry_id).map(|(_, v)| *v)
}

fn wf_segment_to_domain(seg: &str) -> Option<&'static str> {
    OPC_WF_SEGMENT_DOMAIN.iter().find(|(k, _)| *k == seg).map(|(_, v)| *v)
}

/// 权威三层路由地址（`/{domain}/{cluster}/{capability}`）：
/// seed 时按模板 ID 推导行业/能力路径，**L1 段必须是 CapabilityDomain
/// 合法值**（general/finance/automation/devops/data_analysis/content_creation/
/// ai_media/communication），以匹配前端 `TemplateList` 的 `getTemplateDomain`
/// 严格按 routePath 归域逻辑（`src/components/workflow/Templates/TemplateList.tsx`）。
///
/// 推导优先级：
/// 1. stock 域模板显式路径（`STOCK_EXPLICIT_ROUTE`，如 `/finance/equity/multi-dim-analysis`）；
/// 2. content_media 专属特例（既有契约，L1=`content_creation`）；
/// 3. `{industry}_harness_workflow` → `/{industry_domain}/{industry}/harness`
///    （14 行业，按业务本质归 CapabilityDomain）；
/// 4. `wf-{seg}-{slug}` → `/{seg_domain}/{seg}/{slug}`（17 领域 75 模板）；
/// 5. `prod-{slug}` → `/automation/production/{slug}`（OPC 自动化运营）；
/// 6. 兜底 `/general/{template_id}`（确定性路径，不产生未分类）。
pub(crate) fn authoritative_route_path(template_id: &str) -> String {
    if let Some((_, path)) = STOCK_EXPLICIT_ROUTE.iter().find(|(k, _)| *k == template_id) {
        return path.to_string();
    }
    if let Some(rest) = template_id.strip_prefix("workflow-cm-") {
        let cluster_cap = match rest {
            "literary-creation" => "writing/literary",
            "viral-content" => "media/viral",
            "multi-platform" => "media/multi-platform",
            "ip-building" => "media/ip-building",
            _ => "media/main",
        };
        return format!("/content_creation/{cluster_cap}");
    }
    if let Some(industry) = template_id.strip_suffix("_harness_workflow") {
        let domain = industry_to_domain(industry).unwrap_or("general");
        return format!("/{domain}/{industry}/harness");
    }
    if let Some(rest) = template_id.strip_prefix("wf-") {
        let (seg, slug) = rest.split_once('-').unwrap_or((rest, "main"));
        let domain = wf_segment_to_domain(seg).unwrap_or("general");
        return format!("/{domain}/{seg}/{slug}");
    }
    if let Some(slug) = template_id.strip_prefix("prod-") {
        return format!("/automation/production/{slug}");
    }
    format!("/general/{template_id}")
}

/// 回填存量缺失 route_path：seed 后对 `route_path IS NULL` 的预设模板
/// 按权威映射补全（幂等，非空行不动）。
///
/// 覆盖 OPC(行业 harness / wf / prod / workflow-cm) + stock 域模板
/// （stock- / serenity- / news- / daily- / screenshot- / auto- / opc- 前缀），
/// 排除 `cognitive_*` 系统模板（前端业务列表已过滤，归系统域）。
pub(crate) async fn backfill_missing_route_paths(db: &DatabaseConnection) -> Result<usize, String> {
    use sea_orm::*;
    let rows = workflow_template::Entity::find()
        .filter(workflow_template::Column::RoutePath.is_null())
        .filter(workflow_template::Column::IsPreset.eq(true))
        .all(db)
        .await
        .map_err(|e| format!("查询缺失 route_path 的模板: {e}"))?;

    let mut updated = 0;
    for row in rows {
        let is_opc = row.id.ends_with("_harness_workflow")
            || row.id.starts_with("wf-")
            || row.id.starts_with("prod-")
            || row.id.starts_with("workflow-cm-")
            || row.id.starts_with("stock-")
            || row.id.starts_with("serenity-")
            || row.id.starts_with("news-")
            || row.id.starts_with("daily-")
            || row.id.starts_with("screenshot-")
            || row.id.starts_with("auto-")
            || row.id.starts_with("opc-");
        if !is_opc {
            continue;
        }
        let path = authoritative_route_path(&row.id);
        workflow_template::Entity::update_many()
            .col_expr(workflow_template::Column::RoutePath, sea_query::Expr::value(path))
            .filter(workflow_template::Column::Id.eq(&row.id))
            .exec(db)
            .await
            .map_err(|e| format!("回填 route_path {} 失败: {e}", row.id))?;
        updated += 1;
    }
    if updated > 0 {
        tracing::info!("[opc-workflows] 回填 {} 个模板缺失的 route_path", updated);
    }
    Ok(updated)
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
        cluster_id: Set(None),
        // 显式 route_path 优先（如既有特例模板），否则走权威行业/能力映射
        route_path: Set(data
            .route_path
            .clone()
            .or_else(|| Some(authoritative_route_path(&data.id)))),
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
                .update_column(workflow_template::Column::Name)
                .update_column(workflow_template::Column::Description)
                .update_column(workflow_template::Column::Icon)
                .update_column(workflow_template::Column::Tags)
                .update_column(workflow_template::Column::Version)
                .update_column(workflow_template::Column::Nodes)
                .update_column(workflow_template::Column::Edges)
                .update_column(workflow_template::Column::InputSchema)
                .update_column(workflow_template::Column::OutputSchema)
                .update_column(workflow_template::Column::Variables)
                .update_column(workflow_template::Column::ErrorConfig)
                .update_column(workflow_template::Column::ToolDefs)
                .update_column(workflow_template::Column::RoutePath)
                .update_column(workflow_template::Column::UpdatedAt)
                .to_owned(),
        )
        .exec_without_returning(db)
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
    use sea_orm::{ConnectionTrait, EntityTrait, PaginatorTrait};

    #[test]
    fn authoritative_route_path_mapping() {
        // 行业 harness → /{CapabilityDomain}/{industry}/harness
        assert_eq!(
            super::authoritative_route_path("accounting_harness_workflow"),
            "/finance/accounting/harness"
        );
        assert_eq!(
            super::authoritative_route_path("ai_research_harness_workflow"),
            "/data_analysis/ai_research/harness"
        );
        assert_eq!(
            super::authoritative_route_path("content_media_harness_workflow"),
            "/content_creation/content_media/harness"
        );
        assert_eq!(
            super::authoritative_route_path("finance_invest_harness_workflow"),
            "/finance/finance_invest/harness"
        );
        assert_eq!(
            super::authoritative_route_path("game_dev_harness_workflow"),
            "/ai_media/game_dev/harness"
        );
        assert_eq!(
            super::authoritative_route_path("industry_consulting_harness_workflow"),
            "/automation/industry_consulting/harness"
        );
        assert_eq!(
            super::authoritative_route_path("security_harness_workflow"),
            "/devops/security/harness"
        );
        // 领域 wf-{seg}-{slug} → /{seg_domain}/{seg}/{slug}
        assert_eq!(super::authoritative_route_path("wf-fin-budget"), "/finance/fin/budget");
        assert_eq!(super::authoritative_route_path("wf-eng-refactor"), "/devops/eng/refactor");
        assert_eq!(super::authoritative_route_path("wf-gis-mapping"), "/data_analysis/gis/mapping");
        assert_eq!(
            super::authoritative_route_path("wf-mkt-analytics"),
            "/communication/mkt/analytics"
        );
        // 生产 prod-{slug} → /automation/production/{slug}
        assert_eq!(
            super::authoritative_route_path("prod-landing-page"),
            "/automation/production/landing-page"
        );
        // content_media 专属特例（既有契约，L1=content_creation）
        assert_eq!(
            super::authoritative_route_path("workflow-cm-literary-creation"),
            "/content_creation/writing/literary"
        );
        assert_eq!(
            super::authoritative_route_path("workflow-cm-viral-content"),
            "/content_creation/media/viral"
        );
        // 兜底：/general/{template_id}，确定性路径
        assert_eq!(super::authoritative_route_path("custom_workflow"), "/general/custom_workflow");
        // stock 域模板显式路径（与 seed 文件一致，覆盖存量 NULL）
        assert_eq!(
            super::authoritative_route_path("stock-analysis"),
            "/finance/equity/multi-dim-analysis"
        );
        assert_eq!(
            super::authoritative_route_path("serenity-screening"),
            "/finance/trend/serenity"
        );
        assert_eq!(
            super::authoritative_route_path("opc-demand-discovery"),
            "/automation/opc/demand-discovery"
        );
        // 战略咨询领域 → automation
        assert_eq!(
            super::authoritative_route_path("wf-strat-biz-plan"),
            "/automation/strat/biz-plan"
        );
    }

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

        // 行业包 manifest 注册（仅注册，不 seed 工作流）
        let seeded =
            super::industry_pack::ensure_opc_industries_seeded(db, &base).await.expect("注册成功");
        assert_eq!(seeded.len(), 14, "应注册 14 行业: {seeded:?}");

        use axagent_entities::opc_industries;
        let count = opc_industries::Entity::find().count(db).await.unwrap();
        assert_eq!(count, 14, "opc_industries 应有 14 行");

        // 工作流由 Rust 种子文件
        let wf_seeded = seed_opc_industries_from_seed_files(db).await.expect("seed 文件成功");
        assert_eq!(wf_seeded, 14, "应 seed 14 行业工作流");

        use axagent_entities::workflow_template;
        let fi = workflow_template::Entity::find_by_id("finance_invest_harness_workflow")
            .one(db)
            .await
            .unwrap()
            .expect("finance_invest_harness_workflow 应存在");
        assert!(!fi.nodes.is_empty(), "金融投资工作流节点不应为空");
    }

    #[tokio::test]
    async fn industry_pack_disabled_industry_not_seeded() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let tmp = std::env::temp_dir().join(format!("opc-test-{}", std::process::id()));
        let dir = tmp.join("disabled_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.yaml"),
            "id: disabled_test\nname: 禁用测试\nversion: 1\nenabled: false\n",
        )
        .unwrap();

        let seeded = super::industry_pack::ensure_opc_industries_seeded(db, &tmp).await.unwrap();
        assert!(!seeded.contains(&"disabled_test".to_string()));
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

        // 导出 finance_invest → .opcip（含 manifest + analysis + learning）
        let out = export_industry_pack(&base, "finance_invest", &tmp).await.expect("导出成功");
        assert!(std::path::Path::new(&out).exists(), "归档应生成");
        assert!(out.ends_with("finance_invest.opcip"), "归档名应含行业 id");

        // 导入到独立 app_dir → 注册 manifest
        let app_dir = tmp.join("app");
        let imported =
            import_industry_pack(db, &app_dir, std::path::Path::new(&out)).await.expect("导入成功");
        assert_eq!(imported, "finance_invest");

        // 解包的 manifest 应存在
        let manifest = app_dir.join("config/opc/industries/finance_invest/manifest.yaml");
        assert!(manifest.exists(), "解包后 manifest 应存在");

        // 工作流由 Rust 适配器提供，导入不包含 YAML 工作流
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn domain_pack_seed_all_workflows() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;

        let seeded = seed_domains_from_seed_files(db).await.expect("领域 seed 文件成功");
        assert!(seeded > 65, "应 seed 至少 65 个领域工作流，实际 {seeded}");

        use axagent_entities::workflow_template;
        let wf = workflow_template::Entity::find_by_id("wf-eng-code-review")
            .one(db)
            .await
            .unwrap()
            .expect("wf-eng-code-review 应存在");
        assert!(!wf.nodes.is_empty(), "节点不应为空");

        seed_domains_from_seed_files(db).await.expect("二次 seed 应成功");
    }

    #[tokio::test]
    async fn finance_pack_injects_astock_tools() {
        // 金融投资行业工作流种子化验证：验证行业适配器生成正确的工作流结构
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;

        // 行业工作流由 Rust 种子文件
        seed_opc_industries_from_seed_files(db).await.expect("seed 文件成功");

        // 金融投资行业工作流应存在（由 Rust adapter 生成）
        use axagent_entities::workflow_template;
        let wf = workflow_template::Entity::find_by_id("finance_invest_harness_workflow")
            .one(db)
            .await
            .unwrap()
            .expect("finance_invest_harness_workflow 应存在");

        // 节点字符串应包含核心节点类型
        assert!(!wf.nodes.is_empty(), "工作流节点不应为空");
        assert!(wf.nodes.contains("trigger"), "应含触发节点");
        assert!(wf.nodes.contains("step_finance_invest"), "应含业务步骤节点");
        assert!(wf.nodes.contains("end"), "应含结束节点");

        // 边字符串应包含至少若干条连接
        assert!(!wf.edges.is_empty(), "边不应为空");

        // stock_tool_defs 工具函数可正确匹配 astock 工具
        let defs = super::industry_pack::stock_tool_defs(&[
            "get_stock_quote".to_string(),
            "search_stock".to_string(),
        ]);
        assert_eq!(defs.len(), 2, "应匹配 2 个 astock 工具");
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

    /// 最终验收：9 行业 seed 产物端到端断言——工作流结构完整、幂等。
    #[tokio::test]
    async fn industry_packs_end_to_end_verification() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;

        // 行业工作流由 Rust 种子文件
        seed_opc_industries_from_seed_files(db).await.expect("seed 14 行业");

        use axagent_entities::workflow_template;
        use sea_orm::EntityTrait;

        // 1. 14 行业工作流模板全部存在
        let expected_ids = [
            "accounting_harness_workflow",
            "ai_research_harness_workflow",
            "content_media_harness_workflow",
            "ecommerce_harness_workflow",
            "education_harness_workflow",
            "finance_invest_harness_workflow",
            "industry_consulting_harness_workflow",
            "sales_growth_harness_workflow",
            "software_dev_harness_workflow",
            "design_harness_workflow",
            "project_management_harness_workflow",
            "security_harness_workflow",
            "geospatial_harness_workflow",
            "game_dev_harness_workflow",
        ];
        for id in &expected_ids {
            let t = workflow_template::Entity::find_by_id(*id).one(db).await.unwrap();
            assert!(t.is_some(), "{id} 应被 seed");
        }

        // 2. 每个工作流都应有非空的节点和边
        for id in &expected_ids {
            let wf = workflow_template::Entity::find_by_id(*id)
                .one(db)
                .await
                .unwrap()
                .expect("{id} 应存在");
            assert!(!wf.nodes.is_empty(), "{id} 节点不应为空");
            assert!(!wf.edges.is_empty(), "{id} 边不应为空");
        }

        // 3. 会计行业（requires_approval=true）应包含审批节点（v4 起节点 id 为 ap-accounting）
        let acc = workflow_template::Entity::find_by_id("accounting_harness_workflow")
            .one(db)
            .await
            .unwrap()
            .expect("accounting 存在");
        assert!(acc.nodes.contains("ap-accounting"), "accounting 应包含审批节点");

        // 4. 软件开发生意业务应有步骤节点
        let sdev = workflow_template::Entity::find_by_id("software_dev_harness_workflow")
            .one(db)
            .await
            .unwrap()
            .expect("software_dev 存在");
        assert!(sdev.nodes.contains("step_software_dev"), "software_dev 应含步骤节点");

        // 5. 幂等：二次 seed 不报错、不产生重复
        seed_opc_industries_from_seed_files(db).await.expect("二次 seed 应成功");
        let count = workflow_template::Entity::find().count(db).await.unwrap();
        assert_eq!(count, 14, "14 行业共 14 个工作流，二次 seed 后不应残留/重复，实际 {count}");
    }

    #[tokio::test]
    async fn approval_edges_build_correctly() {
        // P0-1 回归：会计行业需审批流程，验证审批节点存在且有正确的边连接
        // v4 升级：丰富拓扑（LLM 条件门 + 修正分支 + 汇合），审批节点 id 为 ap-accounting
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;

        seed_opc_industries_from_seed_files(db).await.expect("seed 文件成功");

        use axagent_entities::workflow_template;
        let wf = workflow_template::Entity::find_by_id("accounting_harness_workflow")
            .one(db)
            .await
            .unwrap()
            .expect("accounting_harness_workflow 应存在");

        // 会计行业 requires_approval()=true，应包含审批节点（id 为 ap-accounting）
        assert!(wf.nodes.contains("ap-accounting"), "应包含审批节点 ap-accounting");

        // 审批节点应有至少一条入边和一条出边（step4_accounting → ap-accounting → end）
        assert!(wf.edges.contains("ap-accounting"), "edges 应包含审批节点的引用");

        // 审批节点的入+出边都是 Direct 类型（v4 的 conditionTrue/False 属于 LLM 质量门，与审批无关）
        let direct_count = wf.edges.matches("\"edge_type\":\"direct\"").count();
        assert!(direct_count >= 2, "审批节点入+出至少 2 条 direct 边，实际 {direct_count}");

        // 验证审批节点的两条具体边：step4_accounting → ap-accounting → end
        assert!(wf.edges.contains("e-step4_accounting-approval"), "应存在 step4→approval 边");
        assert!(wf.edges.contains("e-ap-accounting-end"), "应存在 approval→end 边");
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
        assert_eq!(manifests.len(), 14, "内置 14 个行业包");

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
        assert_eq!(count, 14, "应扫描到 14 个行业包，实际 {count}");
    }

    #[test]
    fn industry_adapters_loaded_from_packs() {
        // P0-1-A 回归：行业适配器由行业包 learning.yaml 的 adapter 段驱动
        //（替代 orchestrator create_all_adapters Rust 硬编码）。
        // 用 accounting 已知配置对账：3 checkpoints + 3 AC + min/max 2/15 + protected compliance_check。
        // 测试 CWD=src-tauri，相对路径落空 → 显式传仓库根（模拟 app_dir 命中分支）。
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let adapters = super::load_industry_adapters_from_packs(Some(repo_root));
        assert_eq!(adapters.len(), 14, "应加载 14 个行业适配器: {}", adapters.len());

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
