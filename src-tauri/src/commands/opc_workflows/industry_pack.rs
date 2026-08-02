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
use std::path::{Path, PathBuf};

/// 行业包根目录（相对仓库根）
pub const INDUSTRIES_DIR: &str = "config/opc/industries";

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

fn default_icon() -> String { "🏢".into() }
fn default_version() -> i32 { 1 }
fn default_true() -> bool { true }

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

fn default_approval_message() -> String { "请审批。24小时超时自动拒绝。".into() }
fn default_timeout() -> u64 { 86400 }
fn default_timeout_action() -> String { "auto_reject".into() }

// ── 包加载 ────────────────────────────────────────────────────────

/// 扫描行业包目录，返回所有 manifest（含是否启用）。
pub fn scan_industry_packs(base_dir: &Path) -> Vec<IndustryManifest> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(base_dir) else { return out };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() { continue; }
        let manifest_path = dir.join("manifest.yaml");
        let Ok(raw) = std::fs::read_to_string(&manifest_path) else { continue };
        match serde_yaml::from_str::<IndustryManifest>(&raw) {
            Ok(m) => { out.push(m); }
            Err(e) => { tracing::warn!("[industry-pack] {} manifest 解析失败: {e}", dir.display()); }
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
        if path.extension().map(|e| e != "yaml" && e != "yml").unwrap_or(true) { continue; }
        let Ok(raw) = std::fs::read_to_string(&path) else { continue };
        match serde_yaml::from_str::<IndustryWorkflow>(&raw) {
            Ok(w) => { out.push(w); }
            Err(e) => { tracing::warn!("[industry-pack] {} 解析失败: {e}", path.display()); }
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
                    approver: if cfg.approver.is_empty() { None } else { Some(cfg.approver) },
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
            WorkflowNode::Agent(AgentNode {
                base: make_base(&step.id, &step.title, "", 250.0, y),
                config: AgentNodeConfig {
                    system_prompt: step.prompt.clone(),
                    context_sources: vec![],
                    output_var: format!("{}_result", step.id),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                    tools: vec![],
                    exposed_tools: vec![],
                    output_mode: OutputMode::Json,
                    agent_profile_id: Some(w.profile_id.clone()),
                    max_tool_rounds: Some(10),
                    execution_mode: None,
                    rag_source_ids: vec![],
                    model_role: Some("opc-worker".to_string()),
                    consistency_check: None,
                    hallucination_guard: Some(axagent_harness::hallucination_guard::HallucinationGuardConfig {
                        enabled: true, match_threshold: 0.4,
                    }),
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
            icon: if w.icon.is_empty() { "📄".into() } else { w.icon.clone() },
            tags: w.tags.clone(),
            version,
            is_preset: true,
            is_editable: true,
            is_public: false,
            trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
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
        let mut prev: &str = "trigger";
        for (i, sid) in step_ids.iter().enumerate() {
            let is_approval = w.steps[i].node_type == "approval";
            if is_approval {
                edges.push(cond_edge(&format!("e-{prev}-{sid}-true"), prev, sid, true));
                edges.push(cond_edge(&format!("e-{sid}-end-false"), sid, "end", false));
            } else {
                edges.push(edge(&format!("e-{prev}-{sid}"), prev, sid));
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
        icon: if w.icon.is_empty() { "📄".into() } else { w.icon.clone() },
        tags: w.tags.clone(),
        version,
        is_preset: true,
        is_editable: true,
        is_public: false,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
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
        id: id.into(), source: src.into(), source_handle: None,
        target: tgt.into(), target_handle: None,
        edge_type: EdgeType::Direct, label: None,
    }
}

fn cond_edge(id: &str, src: &str, tgt: &str, is_true: bool) -> WorkflowEdge {
    WorkflowEdge {
        id: id.into(), source: src.into(),
        source_handle: Some(if is_true { "true".into() } else { "false".into() }),
        target: tgt.into(), target_handle: None,
        edge_type: if is_true { EdgeType::ConditionTrue } else { EdgeType::ConditionFalse },
        label: None,
    }
}

fn make_base(id: &str, title: &str, desc: &str, x: f64, y: f64) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.into(), title: title.into(), description: Some(desc.into()),
        position: Position { x, y },
        retry: RetryConfig::default(), timeout: Some(300), enabled: true,
        parent_id: None, compensation: None, continue_on_fail: false,
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
pub async fn enabled_industries(db: &DatabaseConnection) -> Result<Vec<axagent_opc_entities::opc_industries::Model>, String> {
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
        let already_seeded = existing
            .as_ref()
            .map(|e| e.version >= m.version && e.enabled == 1)
            .unwrap_or(false);

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
