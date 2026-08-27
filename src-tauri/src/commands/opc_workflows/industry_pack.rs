// SPDX-License-Identifier: AGPL-3.0-only
#![allow(dead_code)]

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

use axagent_harness::capability::Visibility;
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
    /// 分析配置文件（P0-4 四件套之一），缺省 "analysis.yaml"，None 表示无分析配置
    // P0-4：字段供 `load_industry_analysis` 读取，P1 数据接入层接入后移除 allow
    #[allow(dead_code)]
    #[serde(default = "default_analysis_file")]
    pub analysis: String,
    /// 学习配置文件（P0-4 四件套之一），缺省 "learning.yaml"；读取见
    /// `opc_industry_actions::industry_learning_config_path`
    #[allow(dead_code)]
    #[serde(default = "default_learning_file")]
    pub learning: String,
}

fn default_analysis_file() -> String {
    "analysis.yaml".into()
}
fn default_learning_file() -> String {
    "learning.yaml".into()
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
    /// 全局错误处理配置（映射 WorkflowTemplateData.error_config）
    #[serde(default)]
    pub error_handling: Option<IndustryErrorHandling>,
    /// 步骤（agent 节点链），按顺序串接
    pub steps: Vec<IndustryStep>,
}

/// 工作流级错误处理配置（yaml `error_handling:` 顶层键）。
#[derive(Debug, Clone, Deserialize)]
pub struct IndustryErrorHandling {
    #[serde(default)]
    pub retry: u32,
    #[serde(default)]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub on_failure: String,
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
    /// 步骤失败时的降级说明（追加到 prompt 尾部，指导 LLM 处理失败场景）
    #[serde(default)]
    pub on_error: Option<String>,
    /// 上游失败时是否容错继续（默认 false）
    #[serde(default)]
    pub continue_on_fail: Option<bool>,
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
    /// 通过按钮文案（附加到审批消息尾部，供前端展示）
    #[serde(default)]
    pub approve_label: Option<String>,
    /// 拒绝按钮文案（附加到审批消息尾部）
    #[serde(default)]
    pub reject_label: Option<String>,
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

// ── analysis.yaml schema（P0-4：行业分析配置，四件套之一） ──────

// P0-4 定义 schema；Phase 1 数据接入层已消费 data_sources/quality_precheck。
// strategies/risk 由 P2（分析策略维度）消费，bundle 字段由 P1 部分消费——接入后移除 allow。
#[allow(dead_code)]
pub mod analysis_schema {
    use serde::Deserialize;
    use std::path::{Path, PathBuf};

    use super::{IndustryManifest, IndustryWorkflow, load_industry_workflows};

    /// 行业分析配置（`analysis.yaml`，由 manifest.analysis 字段引用，缺省同名文件）。
    ///
    /// 供数据接入层（OpIndustryVendor 路由）、分析层（策略维度）与
    /// 质量预检（QualityPrecheck 源清单）消费。P0-4 先定义 schema 与加载，
    /// 执行逻辑在 P1/P2 接入。
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct IndustryAnalysisConfig {
        #[serde(default)]
        pub version: u32,
        #[serde(default)]
        pub industry_id: String,
        /// 数据源声明（vendor 链按优先级）
        #[serde(default)]
        pub data_sources: Vec<AnalysisDataSource>,
        /// 分析策略（行业专属分析维度）
        #[serde(default)]
        pub strategies: Vec<AnalysisStrategy>,
        /// 风控参数（对齐 position_limits 的行业版）
        #[serde(default)]
        pub risk: AnalysisRisk,
        /// 质量预检源清单（对齐 stock QualityPrecheck 的行业版）
        #[serde(default)]
        pub quality_precheck: Vec<String>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AnalysisDataSource {
        pub id: String,
        /// vendor 链（按优先级：db / cache / web / file / astock）
        #[serde(default)]
        pub chain: Vec<String>,
        /// 是否纳入质量预检
        #[serde(default)]
        pub quality_precheck: bool,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AnalysisStrategy {
        pub id: String,
        #[serde(default)]
        pub name: String,
        /// 分析维度（如 cash_flow_health / tax_risk）
        #[serde(default)]
        pub dimensions: Vec<String>,
    }

    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct AnalysisRisk {
        /// 超阈值告警线（0-1）
        #[serde(default)]
        pub max_kpi_warning_pct: f64,
        /// 关键 KPI 清单（越界触发风控拦截）
        #[serde(default)]
        pub critical_kpis: Vec<String>,
    }

    /// 读取行业包内分析配置（`{manifest.analysis}`，缺省 analysis.yaml）。
    /// 文件缺失返回 None（向后兼容：旧行业包无分析配置）。
    pub fn load_industry_analysis(
        industry_dir: &Path,
        manifest: &IndustryManifest,
    ) -> Option<IndustryAnalysisConfig> {
        let path = industry_dir.join(&manifest.analysis);
        let raw = std::fs::read_to_string(&path).ok()?;
        match serde_yaml::from_str(&raw) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                tracing::warn!("[industry-pack] {} analysis 解析失败: {e}", path.display());
                None
            },
        }
    }

    /// 行业包完整资产（P0-4：一次读全四件套 = manifest + workflows + analysis + learning）
    #[derive(Debug, Clone)]
    pub struct IndustryPackBundle {
        pub manifest: IndustryManifest,
        pub workflows: Vec<IndustryWorkflow>,
        pub analysis: Option<IndustryAnalysisConfig>,
        /// 学习配置在 `{industry_dir}/{manifest.learning}`（P4-3 已迁入行业包），
        /// 此处不重复解析，读取走 `opc_industry_actions::industry_learning_config_path`
        pub pack_dir: PathBuf,
    }

    /// 加载单个行业包目录的完整资产（manifest 解析失败返回 None）。
    pub fn load_industry_pack(dir: &Path) -> Option<IndustryPackBundle> {
        let manifest_path = dir.join("manifest.yaml");
        let raw = std::fs::read_to_string(&manifest_path).ok()?;
        let manifest: IndustryManifest = serde_yaml::from_str(&raw).ok()?;
        let workflows = load_industry_workflows(dir);
        let analysis = load_industry_analysis(dir, &manifest);
        Some(IndustryPackBundle { manifest, workflows, analysis, pack_dir: dir.to_path_buf() })
    }
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

    // P1-10：收集 step.inputs 中 `{var}` 引用为工作流变量（模板级声明，前端执行时可注入）
    let mut variables: Vec<Variable> = Vec::new();
    for step in &w.steps {
        for v in step.inputs.values() {
            if let Some(name) = v.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                if !variables.iter().any(|x| x.name == name) {
                    variables.push(Variable {
                        name: name.to_string(),
                        var_type: "string".to_string(),
                        value: serde_json::Value::String(String::new()),
                        description: Some(format!("工作流输入变量 {name}")),
                        is_secret: false,
                    });
                }
            }
        }
    }

    // P1-9：error_handling.timeout_seconds → 节点级超时（默认 300s）
    let node_timeout = w.error_handling.as_ref().map(|eh| eh.timeout_seconds).filter(|&t| t > 0);

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
                approve_label: None,
                reject_label: None,
            });
            // P1-9：approve_label/reject_label 附加到审批消息（ApprovalNodeConfig 无独立字段，
            // 前端审批面板展示 message + 固定通过/拒绝按钮，按钮文案语义通过消息传达）。
            let mut message = cfg.message.clone();
            if let Some(label) = &cfg.approve_label {
                message.push_str(&format!("\n[通过] {label}"));
            }
            if let Some(label) = &cfg.reject_label {
                message.push_str(&format!("\n[拒绝] {label}"));
            }
            let mut approval_base = make_base(&step.id, &step.title, "", 250.0, y);
            approval_base.continue_on_fail = step.continue_on_fail.unwrap_or(false);
            if let Some(t) = node_timeout {
                approval_base.timeout = Some(t);
            }
            WorkflowNode::Approval(ApprovalNode {
                base: approval_base,
                config: ApprovalNodeConfig {
                    message,
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
            // 优先匹配 stock_mcp_tools（金融），其次 OPC 工具（一人公司业务），
            // 最后通用本机工具（FileRead/Bash/Grep 等，P1-1 修复 software_dev 等工具落空）。
            let node_tools = if step.tools.is_empty() {
                vec![]
            } else {
                let mut defs = stock_tool_defs(&step.tools);
                defs.extend(opc_tool_defs(&step.tools));
                defs.extend(local_tool_defs(&step.tools));
                defs
            };
            // P1-9：on_error 降级说明追加到 prompt 尾部
            let mut system_prompt = step.prompt.clone();
            if let Some(on_error) = &step.on_error {
                system_prompt.push_str(&format!("\n\n[失败降级] {on_error}"));
            }
            let mut agent_base = make_base(&step.id, &step.title, "", 250.0, y);
            agent_base.continue_on_fail = step.continue_on_fail.unwrap_or(false);
            if let Some(t) = node_timeout {
                agent_base.timeout = Some(t);
            }
            WorkflowNode::Agent(AgentNode {
                base: agent_base,
                config: AgentNodeConfig {
                    system_prompt,
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
            cluster_id: None,
            route_path: None,
            tags: w.tags.clone(),
            version,
            is_preset: true,
            is_editable: true,
            is_public: false,
            visibility: Visibility::Public,
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
        // 修复 P0-1：只有"上一个节点是审批"（pending_approval 未消费）时，前一步→审批
        // 才用 ConditionTrue 条件边；否则（trigger/普通 agent 节点）用 Direct 边——
        // 普通节点输出是 JSON 字符串，dag_store 条件边判定 `results[src]["result"].as_bool()`
        // 恒 false，ConditionTrue 边永不激活导致审批断链。
        let mut prev: &str = "trigger";
        let mut pending_approval: Option<&str> = None;
        for (i, sid) in step_ids.iter().enumerate() {
            let is_approval = w.steps[i].node_type == "approval";
            if is_approval {
                if pending_approval.is_some() {
                    edges.push(cond_edge(&format!("e-{prev}-{sid}-true"), prev, sid, true));
                } else {
                    edges.push(edge(&format!("e-{prev}-{sid}"), prev, sid));
                }
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
        cluster_id: None,
        route_path: None,
        tags: w.tags.clone(),
        version,
        is_preset: true,
        is_editable: true,
        is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema: None,
        output_schema: None,
        variables: variables.clone(),
        error_config: pack_error_config(w),
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

/// P1-9：行业包顶层 `error_handling` → WorkflowTemplateData.error_config。
fn pack_error_config(w: &IndustryWorkflow) -> Option<ErrorConfig> {
    w.error_handling.as_ref().map(|eh| ErrorConfig {
        retry_policy: if eh.retry > 0 {
            Some(WorkflowRetryPolicy {
                max_retries: eh.retry,
                base_delay_ms: 1000,
                max_delay_ms: 30000,
            })
        } else {
            None
        },
        on_failure: if eh.on_failure.contains("continue") {
            OnFailureAction::ContinueWithDefault
        } else if eh.on_failure.contains("branch") {
            OnFailureAction::RunErrorBranch
        } else {
            OnFailureAction::RetryThenAbort
        },
        error_branch: None,
        compensation_steps: None,
    })
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
    use axagent_entities::opc_industries;
    use sea_orm::*;

    let now = now_ts();
    // P1-5：保留用户手动禁用状态——DB 已有记录时以 DB enabled 为准，
    // manifest.enabled 仅首次插入生效（否则重启会把用户禁用的行业自动重新启用）。
    let existing = opc_industries::Entity::find_by_id(&m.id).one(db).await.ok().flatten();
    let effective_enabled = existing.map(|e| e.enabled != 0).unwrap_or(m.enabled);
    let am = opc_industries::ActiveModel {
        id: Set(m.id.clone()),
        name: Set(m.name.clone()),
        icon: Set(m.icon.clone()),
        description: Set(m.description.clone()),
        version: Set(m.version),
        enabled: Set(effective_enabled as i32),
        pack_path: Set(format!("{INDUSTRIES_DIR}/{}", m.id)),
        installed_at: Set(now),
        updated_at: Set(now),
    };
    opc_industries::Entity::insert(am)
        .on_conflict(
            sea_query::OnConflict::column(opc_industries::Column::Id)
                .update_column(opc_industries::Column::Name)
                .update_column(opc_industries::Column::Icon)
                .update_column(opc_industries::Column::Description)
                .update_column(opc_industries::Column::Version)
                .update_column(opc_industries::Column::Enabled)
                .update_column(opc_industries::Column::PackPath)
                .update_column(opc_industries::Column::UpdatedAt)
                .to_owned(),
        )
        .exec_without_returning(db)
        .await
        .map_err(|e| format!("upsert industry: {e}"))?;
    Ok(())
}

/// 从 opc_industries 读取启用的行业（按 version 过滤需要 seed 的）。
/// P2 export/install 命令使用，当前尚未接线。
#[allow(dead_code)]
pub async fn enabled_industries(
    db: &DatabaseConnection,
) -> Result<Vec<axagent_entities::opc_industries::Model>, String> {
    use axagent_entities::opc_industries;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    opc_industries::Entity::find()
        .filter(opc_industries::Column::Enabled.eq(1))
        .all(db)
        .await
        .map_err(|e| format!("list enabled industries: {e}"))
}

/// 行业包完整 seed：扫描目录 → 注册表（opc_industries）。
///
/// ⚠️ 架构变更：行业工作流已迁移至手动定义的 seed 文件（见 mod.rs `seed_opc_industries_from_seed_files`），
/// 本函数仅负责 manifest 注册（opc_industries 表），不再从 YAML 加载工作流。
///
/// 返回 seed 的行业 id 列表。
pub async fn ensure_opc_industries_seeded(
    db: &DatabaseConnection,
    base_dir: &Path,
) -> Result<Vec<String>, String> {
    use axagent_entities::opc_industries;
    use sea_orm::EntityTrait;

    let manifests = scan_industry_packs(base_dir);
    let mut seeded = Vec::new();

    for m in manifests {
        // 版本判断：读 DB 现有记录（seed 前，避免 registry upsert 自引用）
        let existing = opc_industries::Entity::find_by_id(&m.id).one(db).await.ok().flatten();
        // P1-5：生效 enabled 以 DB 为准（用户手动禁用优先于 manifest），manifest 仅首装生效
        let effective_enabled = existing.as_ref().map(|e| e.enabled != 0).unwrap_or(m.enabled);
        let already_seeded = existing.as_ref().map(|e| e.version >= m.version).unwrap_or(false);

        // 注册表 upsert（记录当前包状态，enabled 保留 DB 用户状态）
        upsert_industry_registry(db, &m).await?;

        if already_seeded {
            seeded.push(m.id.clone());
            continue;
        }

        if !effective_enabled {
            tracing::info!("[industry-pack] {} 已禁用，跳过注册", m.id);
            continue;
        }

        // 行业工作流已由手动定义的 seed 文件生成（seed_opc_industries_from_seed_files），
        // 此处仅注册 manifest 到 opc_industries 表。
        tracing::info!(
            "[industry-pack] {} manifest 注册完成（v{}，工作流由手动 seed 文件提供）",
            m.id,
            m.version
        );
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
    // P1-12：兼容目录导入（市场页把行业目录路径当归档传）。
    // 目录内应含 manifest.yaml（或其子目录含），直接拷贝到 industries/ 并 seed。
    if archive_path.is_dir() {
        let Some(id) = archive_path.file_name().map(|s| s.to_string_lossy().to_string()) else {
            return Err("无法从目录名确定行业 id".to_string());
        };
        let industries_root = app_dir.join(INDUSTRIES_DIR);
        let target = industries_root.join(&id);
        // 源目录可能是 {id}/（含 manifest）或 {id}/workflows 的父目录，先探测 manifest 位置
        let manifest_candidate = if archive_path.join("manifest.yaml").is_file() {
            archive_path.to_path_buf()
        } else if archive_path.parent().map(|p| p.join("manifest.yaml").is_file()).unwrap_or(false)
        {
            archive_path.parent().unwrap().to_path_buf()
        } else {
            return Err(format!(
                "{} 目录内未找到 manifest.yaml，不是有效的行业包目录",
                archive_path.display()
            ));
        };
        super::copy_dir_recursive(&manifest_candidate, &target)
            .map_err(|e| format!("拷贝行业包目录失败: {e}"))?;
        tracing::info!("[industry-pack] 目录导入 {id} → {}", target.display());
        let seeded = ensure_opc_industries_seeded(db, &industries_root).await?;
        if !seeded.contains(&id) {
            tracing::info!("[industry-pack] {id} 已存在（版本一致），视为导入成功");
        }
        return Ok(id);
    }

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
        // P2-3：zip-slip 防护——拒绝绝对路径与 `..` 穿越（恶意 .opcip 可写任意目录）
        let normalized = entry_name.replace('\\', "/");
        if std::path::Path::new(&normalized).is_absolute()
            || normalized.split('/').any(|c| c == "..")
        {
            return Err(format!("归档内存在非法路径，已拒绝解包: {entry_name}"));
        }
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

    // 注册 + seed（行业工作流现已由 Rust 代码生成，仅注册 manifest）
    let seeded = ensure_opc_industries_seeded(db, &target_root).await?;
    if !seeded.contains(&id) {
        tracing::info!("[industry-pack] {id} 已存在，跳过 seed");
    }
    Ok(id)
}

// ── JSON 工作流模板导出/导入（资源交换机制） ─────────────────────
//
// 行业/领域工作流 → JSON 文件（WorkflowTemplateResponse 数组），
// 通过工作流编辑器的导出/导入功能实现资源交换。
// 替代旧 .opcip zip 格式（YAML 工作流），统一使用 JSON 工作流模板。

/// 从数据库查询指定行业的所有工作流模板（按 tags 过滤），
/// 导出为 JSON bundle 文件（manifest + templates[]）。
pub async fn export_industry_workflows_json(
    db: &DatabaseConnection,
    industry_id: &str,
    out_path: &Path,
) -> Result<String, String> {
    use axagent_entities::workflow_template;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let templates = workflow_template::Entity::find()
        .filter(workflow_template::Column::Tags.like(format!("%\"{industry_id}\"%")))
        .all(db)
        .await
        .map_err(|e| format!("查询工作流失败: {e}"))?;

    if templates.is_empty() {
        return Err(format!("行业 {industry_id} 无工作流模板"));
    }

    let mut template_json_list = Vec::new();
    for t in &templates {
        let tags: Vec<String> =
            t.tags.as_ref().and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default();

        let nodes: Vec<WorkflowNode> = serde_json::from_str(&t.nodes).unwrap_or_default();
        let edges: Vec<WorkflowEdge> = serde_json::from_str(&t.edges).unwrap_or_default();

        let template_data = WorkflowTemplateData {
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            icon: t.icon.clone(),
            cluster_id: None,
            route_path: None,
            tags,
            version: t.version,
            is_preset: t.is_preset,
            is_editable: t.is_editable,
            is_public: t.is_public,
            visibility: Visibility::Public,
            trigger_config: t.trigger_config.as_ref().and_then(|j| serde_json::from_str(j).ok()),
            nodes,
            edges,
            input_schema: t.input_schema.as_ref().and_then(|j| serde_json::from_str(j).ok()),
            output_schema: t.output_schema.as_ref().and_then(|j| serde_json::from_str(j).ok()),
            variables: t
                .variables
                .as_ref()
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_default(),
            error_config: t.error_config.as_ref().and_then(|j| serde_json::from_str(j).ok()),
            tool_defs: t
                .tool_defs
                .as_ref()
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_default(),
            error_workflow_id: None,
            mission_hash: t.mission_hash.clone(),
            created_at: t.created_at,
            updated_at: t.updated_at,
        };

        template_json_list.push(serde_json::to_value(&template_data).unwrap_or_default());
    }

    let bundle = serde_json::json!({
        "format": "axagent-workflow-bundle",
        "version": 1,
        "industry_id": industry_id,
        "exported_at": now_ts(),
        "templates": template_json_list,
    });

    let json_str =
        serde_json::to_string_pretty(&bundle).map_err(|e| format!("序列化 bundle 失败: {e}"))?;

    std::fs::write(out_path, json_str).map_err(|e| format!("写入文件失败: {e}"))?;

    tracing::info!(
        "[industry-pack] JSON bundle 导出 {industry_id} → {} ({} 个工作流)",
        out_path.display(),
        template_json_list.len()
    );

    Ok(out_path.to_string_lossy().to_string())
}

/// 从 JSON bundle 文件导入工作流模板，upsert 到数据库。
pub async fn import_industry_workflows_json(
    db: &DatabaseConnection,
    bundle_path: &Path,
) -> Result<(String, usize), String> {
    let json_str =
        std::fs::read_to_string(bundle_path).map_err(|e| format!("读取文件失败: {e}"))?;

    let bundle: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("解析 JSON 失败: {e}"))?;

    let format = bundle.get("format").and_then(|v| v.as_str()).unwrap_or("");
    if format != "axagent-workflow-bundle" {
        return Err("不是有效的 axagent 工作流 bundle 格式".to_string());
    }

    let industry_id =
        bundle.get("industry_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();

    let templates = bundle
        .get("templates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "bundle 中无 templates 数组".to_string())?;

    let mut imported_count = 0;
    for template_val in templates {
        let template_data: WorkflowTemplateData = serde_json::from_value(template_val.clone())
            .map_err(|e| format!("解析模板失败: {e}"))?;
        super::upsert_template(db, template_data).await?;
        imported_count += 1;
    }

    tracing::info!("[industry-pack] JSON bundle 导入 {industry_id} ({} 个工作流)", imported_count);

    Ok((industry_id, imported_count))
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
        let keep_ids: Vec<String> = workflows.iter().map(|w| w.id.clone()).collect();

        // P2-5：版本判断——首个 workflow 已存在且 version >= manifest.version 则跳过
        // （此前每次启动无条件 upsert 覆盖，用户编辑的领域工作流会被重置）
        if let Some(first) = workflows.first() {
            use axagent_entities::workflow_template;
            use sea_orm::EntityTrait;
            if let Ok(Some(existing)) =
                workflow_template::Entity::find_by_id(&first.id).one(db).await
                && existing.version >= m.version
            {
                seeded.push(m.id.clone());
                continue;
            }
        }

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
        // 领域包无统一 id 前缀，清理跳过（结构稳定）；仅行业包执行 cleanup
        let _ = keep_ids;
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

// ── OPC 工具白名单（一人公司业务：内容营销/电商等行业吃 Opc 工具链）────/// 从 tools crate 内置 OPC 工具匹配工具名 → ToolDef 列表。
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

// ── 通用本机工具白名单（P1-1：software_dev 等行业声明 FileRead/Bash/Grep 等）──

/// 从 tools crate 内置通用工具匹配工具名 → ToolDef 列表。
/// 与 stock_tool_defs / opc_tool_defs 并列，构成完整工具注入白名单。
pub fn local_tool_defs(names: &[String]) -> Vec<axagent_harness::workflow_types::ToolDef> {
    use axagent_tools::Tool;
    let candidates: Vec<Arc<dyn Tool>> = vec![
        std::sync::Arc::new(axagent_tools::tools::file_read::FileReadTool),
        std::sync::Arc::new(axagent_tools::tools::file_write::FileWriteTool),
        std::sync::Arc::new(axagent_tools::tools::file_edit::FileEditTool),
        std::sync::Arc::new(axagent_tools::tools::bash::BashTool),
        std::sync::Arc::new(axagent_tools::tools::grep::GrepTool),
        std::sync::Arc::new(axagent_tools::tools::glob::GlobTool),
        std::sync::Arc::new(axagent_tools::tools::file_system::ListDirectoryTool),
        std::sync::Arc::new(axagent_tools::tools::web_search::WebSearchTool),
    ];
    let mut out = Vec::new();
    for tool in candidates {
        if !names.iter().any(|n| n == tool.name()) {
            continue;
        }
        let parameters = serde_json::from_value(tool.input_schema()).ok();
        out.push(axagent_harness::workflow_types::ToolDef {
            name: tool.name().to_string(),
            description: Some(tool.description().to_string()),
            parameters,
        });
    }
    out
}
