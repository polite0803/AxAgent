// SPDX-License-Identifier: AGPL-3.0-only

//! 行业数据资产包（Industry Pack）引擎
//!
//! 行业 = 数据资产包，非代码。每个行业一个独立目录：
//! `config/opc/industries/{industry_id}/`
//!   ├── manifest.yaml     # id / name / icon / version / enabled
//!   ├── roles.yaml        # 行业角色映射（opc-cfo 等 → 专家/工具白名单）
//!   └── workflows/*.yaml  # 工作流模板（纯数据，节点/边/prompt）
//!
//! 启动扫描注册到 `opc_industries` 表，支持单独启用/禁用/导出/导入。
//! 行业级版本号取代全局 OPC_TEMPLATE_VERSION，行业间互不影响。

use axagent_harness::util_fns::now_ts;
use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 行业包根目录（相对仓库根）
pub const INDUSTRIES_DIR: &str = "config/opc/industries";

/// 领域包根目录（相对仓库根；与行业包同 schema，独立目录）
pub const DOMAINS_DIR: &str = "config/opc/domains";

// ── manifest.yaml schema ──────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct IndustryManifest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: i32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_icon() -> String {
    "🏢".into()
}
fn default_version() -> i32 {
    1
}
fn default_true() -> bool {
    true
}

// ── workflows/*.yaml schema ───────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct IndustryWorkflow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 绑定角色 profile_id（如 opc-cfo-cfo-financial-analyst）
    pub profile_id: String,
    /// 步骤（agent 节点链），按顺序串接
    pub steps: Vec<IndustryStep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndustryStep {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub prompt: String,
    /// 节点类型：agent（默认）| approval（人工审批）
    #[serde(default)]
    pub node_type: String,
    /// approval 节点配置
    #[serde(default)]
    pub approval: Option<IndustryApproval>,
    /// 上游输入映射：{ 输入变量名: 上游节点输出路径 }
    /// 例：{ "report": "a-report.result" }
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    /// 工具白名单：节点可调用的工具名（如 get_stock_quote / search_news）。
    /// 空 = 不暴露任何工具。匹配 astock-data stock_mcp_tools 工具名。
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndustryApproval {
    #[serde(default = "default_approval_message")]
    pub message: String,
    /// 审批人角色（如 manager）
    #[serde(default)]
    pub approver: String,
    /// 超时秒数（默认 86400）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 超时动作：auto_reject（默认）| auto_approve
    #[serde(default = "default_timeout_action")]
    pub timeout_action: String,
}

fn default_approval_message() -> String {
    "请审批。24小时超时自动拒绝。".into()
}
fn default_timeout() -> u64 {
    86400
}
fn default_timeout_action() -> String {
    "auto_reject".into()
}

// ── 包加载 ────────────────────────────────────────────────────────

/// 扫描行业包目录，返回所有 manifest（含是否启用）。
pub fn scan_industry_packs(base_dir: &Path) -> Vec<IndustryManifest> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(base_dir) else { return out };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let manifest_path = dir.join("manifest.yaml");
        let Ok(raw) = std::fs::read_to_string(&manifest_path) else { continue };
        match serde_yaml::from_str::<IndustryManifest>(&raw) {
            Ok(m) => {
                out.push(m);
            },
            Err(e) => {
                tracing::warn!("[industry-pack] {} manifest 解析失败: {e}", dir.display());
            },
        }
    }
    out
}

/// 读取某行业包目录下的全部工作流 yaml。
pub fn load_industry_workflows(industry_dir: &Path) -> Vec<IndustryWorkflow> {
    let mut out = Vec::new();
    let wf_dir = industry_dir.join("workflows");
    let Ok(entries) = std::fs::read_dir(&wf_dir) else { return out };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e != "yaml" && e != "yml").unwrap_or(true) {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else { continue };
        match serde_yaml::from_str::<IndustryWorkflow>(&raw) {
            Ok(w) => {
                out.push(w);
            },
            Err(e) => {
                tracing::warn!("[industry-pack] {} 解析失败: {e}", path.display());
            },
        }
    }
    out
}

// ── 工作流构建（yaml → WorkflowTemplateData） ────────────────────

/// 将 IndustryWorkflow 转为 WorkflowTemplateData（节点链 + 串接边）。
///
/// 节点类型：agent（默认）| approval。approval 产生条件分支：
/// 通过(true) → 下一节点，拒绝(false) → end。
pub fn build_workflow_from_pack(w: &IndustryWorkflow, version: i32) -> WorkflowTemplateData {
    let now = now_ts();
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    // trigger
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: make_base("trigger", "手动启动", "用户选择后启动工作流", 250.0, 0.0),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    }));

    // 步骤链（y 坐标按序递增 200）
    let step_ids: Vec<&str> = w.steps.iter().map(|s| s.id.as_str()).collect();
    let mut has_approval = false;
    for (i, step) in w.steps.iter().enumerate() {
        let y = 150.0 + (i as f64) * 200.0;
        let node = if step.node_type == "approval" {
            has_approval = true;
            let cfg = step.approval.clone().unwrap_or(IndustryApproval {
                message: default_approval_message(),
                approver: String::new(),
                timeout_secs: default_timeout(),
                timeout_action: default_timeout_action(),
            });
            WorkflowNode::Approval(ApprovalNode {
                base: make_base(&step.id, &step.title, "", 250.0, y),
                config: ApprovalNodeConfig {
                    message: cfg.message,
                    approver: if cfg.approver.is_empty() {
                        None
                    } else {
                        Some(cfg.approver)
                    },
                    timeout_secs: cfg.timeout_secs,
                    timeout_action: cfg.timeout_action,
                    output_var: format!("{}_result", step.id),
                },
            })
        } else {
            let mut input_mapping: HashMap<String, String> = HashMap::new();
            for (k, v) in &step.inputs {
                input_mapping.insert(k.clone(), v.clone());
            }
            // 工具白名单：step.tools 声明的工具名 → ToolDef
            // 优先匹配 stock_mcp_tools（金融），其次匹配 OPC 工具（一人公司业务）。
            let node_tools = if step.tools.is_empty() {
                vec![]
            } else {
                let mut defs = stock_tool_defs(&step.tools);
                defs.extend(opc_tool_defs(&step.tools));
                defs
            };
            WorkflowNode::Agent(AgentNode {
                base: make_base(&step.id, &step.title, "", 250.0, y),
                config: AgentNodeConfig {
                    system_prompt: step.prompt.clone(),
                    context_sources: vec![],
                    output_var: format!("{}_result", step.id),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                    tools: node_tools.clone(),
                    exposed_tools: node_tools.iter().map(|t| t.name.clone()).collect(),
                    output_mode: OutputMode::Json,
                    agent_profile_id: Some(w.profile_id.clone()),
                    max_tool_rounds: Some(10),
                    execution_mode: None,
                    rag_source_ids: vec![],
                    model_role: Some("opc-worker".to_string()),
                    consistency_check: None,
                    hallucination_guard: Some(
                        axagent_harness::hallucination_guard::HallucinationGuardConfig {
                            enabled: true,
                            match_threshold: 0.4,
                        },
                    ),
                    fallback_model: None,
                    task_scene: None,
                    stream_chunk_timeout_secs: None,
                    input_mapping,
                },
            })
        };
        nodes.push(node);
    }

    // end
    nodes.push(WorkflowNode::End(EndNode {
        base: make_base("end", "完成", "", 250.0, 150.0 + (w.steps.len() as f64) * 200.0),
        config: EndNodeConfig { output_var: None },
    }));

    // 串接边
    if step_ids.is_empty() {
        edges.push(edge("e-trigger-end", "trigger", "end"));
        return WorkflowTemplateData {
            id: w.id.clone(),
            name: w.name.clone(),
            description: Some(w.description.clone()),
            icon: if w.icon.is_empty() {
                "📄".into()
            } else {
                w.icon.clone()
            },
            tags: w.tags.clone(),
            version,
            is_preset: true,
            is_editable: true,
            is_public: false,
            trigger_config: Some(TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::json!({}),
            }),
            nodes,
            edges,
            input_schema: None,
            output_schema: None,
            variables: vec![],
            error_config: None,
            error_workflow_id: None,
            mission_hash: None,
            tool_defs: vec![],
            created_at: now,
            updated_at: now,
        };
    }

    if has_approval {
        // 有 approval：逐段串接，approval 通过(true)→下一节点，拒绝(false)→end
        // 修复：使用 pending_approval 追踪审批状态，确保审批后节点连 true 分支而非普通边
        let mut prev: &str = "trigger";
        let mut pending_approval: Option<&str> = None;
        for (i, sid) in step_ids.iter().enumerate() {
            let is_approval = w.steps[i].node_type == "approval";
            if is_approval {
                edges.push(cond_edge(&format!("e-{prev}-{sid}-true"), prev, sid, true));
                edges.push(cond_edge(&format!("e-{sid}-end-false"), sid, "end", false));
                pending_approval = Some(sid);
            } else {
                if let Some(approval_id) = pending_approval {
                    edges.push(cond_edge(
                        &format!("e-{approval_id}-{sid}-true"),
                        approval_id,
                        sid,
                        true,
                    ));
                    pending_approval = None;
                } else {
                    edges.push(edge(&format!("e-{prev}-{sid}"), prev, sid));
                }
            }
            prev = sid;
        }
        // 最后一步 → end（若最后一步是 approval，其 false 分支已连 end，true 分支连 end）
        let last = step_ids.last().unwrap();
        let last_is_approval = w.steps.last().map(|s| s.node_type == "approval").unwrap_or(false);
        if !last_is_approval {
            edges.push(edge(&format!("e-{last}-end"), last, "end"));
        } else if !edges.iter().any(|e| e.target == "end" && e.source == *last) {
            edges.push(cond_edge(&format!("e-{last}-end-true"), last, "end", true));
        }
    } else {
        // 纯链式：trigger → s0 → s1 → ... → end
        edges.push(edge("e-trigger-first", "trigger", step_ids[0]));
        for i in 0..step_ids.len().saturating_sub(1) {
            edges.push(edge(
                &format!("e-{}-{}", step_ids[i], step_ids[i + 1]),
                step_ids[i],
                step_ids[i + 1],
            ));
        }
        if let Some(last) = step_ids.last() {
            edges.push(edge(&format!("e-{last}-end"), last, "end"));
        }
    }

    WorkflowTemplateData {
        id: w.id.clone(),
        name: w.name.clone(),
        description: Some(w.description.clone()),
        icon: if w.icon.is_empty() {
            "📄".into()
        } else {
            w.icon.clone()
        },
        tags: w.tags.clone(),
        version,
        is_preset: true,
        is_editable: true,
        is_public: false,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema: None,
        output_schema: None,
        variables: vec![],
        error_config: None,
        error_workflow_id: None,
        mission_hash: None,
        tool_defs: vec![],
        created_at: now,
        updated_at: now,
    }
}

fn edge(id: &str, src: &str, tgt: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.into(),
        source: src.into(),
        source_handle: None,
        target: tgt.into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    }
}

fn cond_edge(id: &str, src: &str, tgt: &str, is_true: bool) -> WorkflowEdge {
    WorkflowEdge {
        id: id.into(),
        source: src.into(),
        source_handle: Some(if is_true {
            "true".into()
        } else {
            "false".into()
        }),
        target: tgt.into(),
        target_handle: None,
        edge_type: if is_true {
            EdgeType::ConditionTrue
        } else {
            EdgeType::ConditionFalse
        },
        label: None,
    }
}

fn make_base(id: &str, title: &str, desc: &str, x: f64, y: f64) -> WorkflowNodeBase {
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

// ── 注册与 seed ───────────────────────────────────────────────────

/// 将行业包注册进 opc_industries 表（存在则按 version 判断是否升级）。
pub async fn upsert_industry_registry(
    db: &DatabaseConnection,
    m: &IndustryManifest,
) -> Result<(), String> {
    use axagent_opc_entities::opc_industries;
    use sea_orm::*;

    let now = now_ts();
    let am = opc_industries::ActiveModel {
        id: Set(m.id.clone()),
        name: Set(m.name.clone()),
        icon: Set(m.icon.clone()),
        description: Set(m.description.clone()),
        version: Set(m.version),
        enabled: Set(m.enabled as i32),
        pack_path: Set(format!("{INDUSTRIES_DIR}/{}", m.id)),
        installed_at: Set(now),
        updated_at: Set(now),
    };
    opc_industries::Entity::insert(am)
        .on_conflict(
            sea_query::OnConflict::column(opc_industries::Column::Id)
                .update_columns([
                    opc_industries::Column::Name,
                    opc_industries::Column::Icon,
                    opc_industries::Column::Description,
                    opc_industries::Column::Version,
                    opc_industries::Column::Enabled,
                    opc_industries::Column::PackPath,
                    opc_industries::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await
        .map_err(|e| format!("upsert industry: {e}"))?;
    Ok(())
}

/// 从 opc_industries 读取启用的行业（按 version 过滤需要 seed 的）。
/// P2 export/install 命令使用，当前尚未接线。
#[allow(dead_code)]
pub async fn enabled_industries(
    db: &DatabaseConnection,
) -> Result<Vec<axagent_opc_entities::opc_industries::Model>, String> {
    use axagent_opc_entities::opc_industries;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    opc_industries::Entity::find()
        .filter(opc_industries::Column::Enabled.eq(1))
        .all(db)
        .await
        .map_err(|e| format!("list enabled industries: {e}"))
}

/// 行业包完整 seed：扫描目录 → 注册表 → 逐行业 seed 工作流。
/// 返回 seed 的行业 id 列表。
pub async fn ensure_opc_industries_seeded(
    db: &DatabaseConnection,
    base_dir: &Path,
) -> Result<Vec<String>, String> {
    use axagent_opc_entities::opc_industries;
    use sea_orm::EntityTrait;

    let manifests = scan_industry_packs(base_dir);
    let mut seeded = Vec::new();

    for m in manifests {
        // 版本判断：读 DB 现有记录（seed 前，避免 registry upsert 自引用）
        let existing = opc_industries::Entity::find_by_id(&m.id).one(db).await.ok().flatten();
        let already_seeded =
            existing.as_ref().map(|e| e.version >= m.version && e.enabled == 1).unwrap_or(false);

        // 注册表 upsert（记录当前包状态）
        upsert_industry_registry(db, &m).await?;

        if already_seeded {
            seeded.push(m.id.clone());
            continue;
        }

        if !m.enabled {
            tracing::info!("[industry-pack] {} 已禁用，跳过 seed", m.id);
            continue;
        }

        let industry_dir = base_dir.join(&m.id);
        let workflows = load_industry_workflows(&industry_dir);
        for wf in &workflows {
            let data = build_workflow_from_pack(wf, m.version);
            super::upsert_template(db, data).await?;
        }
        tracing::info!("[industry-pack] {} seed 完成（{} 个工作流）", m.id, workflows.len());
        seeded.push(m.id.clone());
    }
    Ok(seeded)
}

/// 供测试/工具使用：给定行业 id 的包目录路径。
pub fn industry_pack_dir(base_dir: &Path, id: &str) -> PathBuf {
    base_dir.join(id)
}

// ── .opcip 导出/导入 ─────────────────────────────────────────────
//
// .opcip = Industry Pack 的 zip 归档（manifest.yaml + workflows/*.yaml）。
// 导出：打包行业目录 → zip 文件；导入：解包 → 注册 → seed。

/// 导出行业包为 .opcip 归档。
/// 返回生成的文件路径。
pub async fn export_industry_pack(
    base_dir: &Path,
    id: &str,
    out_dir: &Path,
) -> Result<String, String> {
    let src = industry_pack_dir(base_dir, id);
    if !src.is_dir() {
        return Err(format!("行业包不存在: {}", src.display()));
    }

    let file_path = out_dir.join(format!("{id}.opcip"));
    let file = std::fs::File::create(&file_path).map_err(|e| format!("创建归档失败: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 递归打包目录（zip 内部用正斜杠相对路径）
    fn add_dir(
        zip: &mut zip::ZipWriter<std::fs::File>,
        opts: &zip::write::SimpleFileOptions,
        _base: &Path,
        dir: &Path,
        prefix: &str,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("读取目录失败: {e}"))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let zip_name = format!("{prefix}{name}");
            if path.is_dir() {
                add_dir(zip, opts, _base, &path, &format!("{zip_name}/"))?;
            } else {
                let content = std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
                zip.start_file(zip_name, *opts).map_err(|e| format!("写入归档失败: {e}"))?;
                zip.write_all(&content).map_err(|e| format!("写入归档失败: {e}"))?;
            }
        }
        Ok(())
    }

    // 打包：zip 内路径以 {id}/ 为前缀（如 "finance_invest/manifest.yaml"），
    // 保证导入时能识别单一顶层行业目录。
    add_dir(&mut zip, &opts, &src, &src, &format!("{id}/"))
        .map_err(|e| format!("打包失败: {e}"))?;
    zip.finish().map_err(|e| format!("归档完成失败: {e}"))?;
    tracing::info!("[industry-pack] 导出 {id} → {}", file_path.display());
    Ok(file_path.to_string_lossy().to_string())
}

/// 导入 .opcip 行业包：解包到 app_dir/config/opc/industries/{id}/ 并注册 seed。
/// 返回导入的行业 id。
pub async fn import_industry_pack(
    db: &DatabaseConnection,
    app_dir: &Path,
    archive_path: &Path,
) -> Result<String, String> {
    let file = std::fs::File::open(archive_path).map_err(|e| format!("打开归档失败: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("解析归档失败: {e}"))?;

    // 目标目录：app_dir/config/opc/industries/{id}
    let target_root = app_dir.join(INDUSTRIES_DIR);
    std::fs::create_dir_all(&target_root).map_err(|e| format!("创建目录失败: {e}"))?;

    // 解包所有条目，记录顶层目录（行业 id，通常只有一个）
    let mut top_dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut has_manifest = false;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("读取条目失败: {e}"))?;
        let entry_name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }
        // 顶层目录 = 行业 id（zip_name 形如 "finance_invest/manifest.yaml"）
        let top = entry_name.split('/').next().unwrap_or("").to_string();
        if top.is_empty() {
            continue;
        }
        top_dirs.insert(top.clone());
        if entry_name.ends_with("manifest.yaml") {
            has_manifest = true;
        }
        let out_path = target_root.join(&entry_name);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
        let mut out = std::fs::File::create(&out_path).map_err(|e| format!("创建文件失败: {e}"))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("解包失败: {e}"))?;
    }

    if !has_manifest {
        return Err("归档内未找到 manifest.yaml，不是有效的 .opcip 行业包".to_string());
    }
    if top_dirs.len() != 1 {
        return Err(format!("归档应只含一个行业包目录，实际 {} 个: {top_dirs:?}", top_dirs.len()));
    }
    let id = top_dirs.into_iter().next().unwrap();
    tracing::info!("[industry-pack] 导入 {id} → {}", target_root.display());

    // 注册 + seed
    let seeded = ensure_opc_industries_seeded(db, &target_root).await?;
    if !seeded.contains(&id) {
        // 包已存在且版本一致（已 seed），视为导入成功
        tracing::info!("[industry-pack] {id} 已存在，跳过 seed");
    }
    Ok(id)
}

// ── 领域包 seed（Self-Built 通用领域工作流）─────────────────────
//
// 与行业包同 schema（manifest.yaml + workflows/*.yaml），独立目录
// config/opc/domains/{domain}/。不建注册表——领域包启用/禁用由
// manifest.enabled 控制，版本由 manifest.version 驱动 upsert 幂等。

/// 扫描并 seed 全部启用的领域包。返回 seed 的领域 id 列表。
pub async fn ensure_opc_domains_seeded(
    db: &DatabaseConnection,
    base_dir: &Path,
) -> Result<Vec<String>, String> {
    let manifests = scan_industry_packs(base_dir);
    let mut seeded = Vec::new();

    for m in manifests {
        if !m.enabled {
            tracing::info!("[domain-pack] {} 已禁用，跳过 seed", m.id);
            continue;
        }
        let domain_dir = base_dir.join(&m.id);
        let workflows = load_industry_workflows(&domain_dir);
        for wf in &workflows {
            let data = build_workflow_from_pack(wf, m.version);
            super::upsert_template(db, data).await?;
        }
        tracing::info!(
            "[domain-pack] {} seed 完成（{} 个工作流，v{}）",
            m.id,
            workflows.len(),
            m.version
        );
        seeded.push(m.id.clone());
    }
    Ok(seeded)
}

// ── 股票工具白名单（P4-2：金融行业吃 astock-data 工具链）────────

/// 从 astock-data stock_mcp_tools 匹配工具名 → ToolDef 列表。
/// 工具已由 init/services.rs ToolResolver 接通执行路径（execute_mcp_tool），
/// 工作流 AgentNode 只要 exposed_tools 含工具名即可调用。
pub fn stock_tool_defs(names: &[String]) -> Vec<axagent_harness::workflow_types::ToolDef> {
    let mut out = Vec::new();
    for tool in axagent_astock_data::mcp_tools::stock_mcp_tools() {
        let Some(name) = tool.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        if !names.iter().any(|n| n == name) {
            continue;
        }
        let description = tool.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
        // parameters：把 inputSchema json 转 ToolDef.parameters（JsonSchema）
        let parameters =
            tool.get("inputSchema").and_then(|v| serde_json::from_value(v.clone()).ok());
        out.push(axagent_harness::workflow_types::ToolDef {
            name: name.to_string(),
            description,
            parameters,
        });
    }
    out
}

// ── OPC 工具白名单（一人公司业务：内容营销/电商等行业吃 Opc 工具链）────

/// 从 tools crate 内置 OPC 工具匹配工具名 → ToolDef 列表。
/// 工具已注册进本地工具注册表（UnifiedToolRegistry），
/// init/services.rs ToolResolver 的 `known` 分支即可接通执行路径，
/// 工作流 AgentNode 只要 exposed_tools 含工具名即可调用。
pub fn opc_tool_defs(names: &[String]) -> Vec<axagent_harness::workflow_types::ToolDef> {
    use axagent_tools::Tool;
    let candidates: Vec<Arc<dyn Tool>> = vec![
        std::sync::Arc::new(axagent_tools::tools::opc::OpcListInvoicesTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcCreateInvoiceTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcTransitionInvoiceTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcListCustomersTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcCreateCustomerTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcListProjectsTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcCreateProjectTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcAddMilestoneTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcGetDashboardTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcListLandingPagesTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcListBlogPostsTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcCreateLandingPageTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcCreateBlogPostTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcListContactsTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcSendNotificationTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcRecordKpiTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcListKpisTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcSearchWikiTool),
        std::sync::Arc::new(axagent_tools::tools::opc::OpcGetFinancialReportTool),
    ];
    let mut out = Vec::new();
    for tool in candidates {
        if !names.iter().any(|n| n == tool.name()) {
            continue;
        }
        // parameters：把 input_schema()（serde_json::Value）转 ToolDef.parameters（JsonSchema）
        let parameters = serde_json::from_value(tool.input_schema()).ok();
        out.push(axagent_harness::workflow_types::ToolDef {
            name: tool.name().to_string(),
            description: Some(tool.description().to_string()),
            parameters,
        });
    }
    out
}
