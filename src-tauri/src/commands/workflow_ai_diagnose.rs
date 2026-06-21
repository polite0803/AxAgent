// SPDX-License-Identifier: AGPL-3.0-only

//! Diagnose 路径:调用 LLM 生成 `DiagnosticReportV2`,并支持批量应用 fixes。
//!
//! V2 协议细节见 [`super::workflow_ai_protocol`]。本模块消费其中的
//! `DiagnosticReportV2` / `DiagnosticFix` / `InjectContextMarker`,并把
//! 工具函数 `validate_report` / `validate_issue` / `dedup_fixes` 串到
//! 真实命令路径上。

use crate::AppState;
use axagent_core::crypto::decrypt_key;
use axagent_core::entity::provider_keys;
use axagent_core::repo::provider;
use axagent_core::workflow_types::WorkflowNode;
use axagent_rt_workflow::work_engine::node_executor_trait::node_type_name;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::workflow_ai_protocol::{
    ChatAction, DiagnosticFix, DiagnosticReportV2, InjectContextMarker, InputMappingEntry,
    validate_report,
};

/// V2 协议 diagnose 上游扩展 prompt,附加在 `base_prompt` 之后。
///
/// 告诉 diagnose LLM 9 类业务无关 category + 4 档 severity + fix 必填规则 +
/// top-level fixes 数组 + auto_apply 含义。
///
/// 提取为 `pub const` 同 [`super::workflow_ai::UPSTREAM_EXTENSION_FOR_CHAT`],
/// 供启动期 `assert_v2_prompts_well_formed` 校验关键 token 防回归。
pub const UPSTREAM_EXTENSION_FOR_DIAGNOSE: &str = r#"
=== Diagnostic Output Schema (v2.0, business-agnostic) ===

You diagnose workflow abstractions. You do NOT know the business
domain. You DO know: nodes, edges, variables, files, versions.

Each issue you emit MUST follow this shape:
{
  "severity":"critical"|"high"|"medium"|"low",
  "category":"<see category table below>",
  "node_id":"<id or null>",
  "title":"<one-line>",
  "detail":"<analysis>",
  "suggestion":"<natural language fix>",
  "fix":{                        // required for critical / high
    "action_type":"<see action_type table below>",
    "data":{ ... }               // must match the chat protocol
  }
}

=== category table (generic, no business meaning) ===
- prompt_quality:          prompt is ambiguous, missing variable refs, no schema
- missing_validation:      no hard guard, no required-field check, no input validation
- variable_misconfig:      variable type/value/required misconfigured
- hardcoded_asset_drift:   text file out of sync with the template state
- workflow_template_version: template behind latest saved version
- backtest_regression:     any caller-defined metric regressed
- tool_missing:            referenced tool not registered (or alias missing)
- edge_misroute:           condition/parallel edge points to wrong target
- semantic_conflict:       two nodes produce conflicting state for the same key

=== action_type table (subset; the same as in workflow_ai_chat) ===
set_node_field, delete_node, delete_edge, enable_retry, set_timeout,
remove_debater_step,
update_variable, update_input_mapping, edit_asset_file,
rollback_to_version

The `data` object MUST match the schema used by the corresponding
chat action. (Same protocol — one source of truth.)

=== Top-level "fixes" array ===
In addition to per-issue `fix`, emit a top-level `"fixes":[...]` array
deduplicating all fixes. The system uses this array to apply fixes
in one batch (with the caller's validation hook).

=== auto_apply flag ===
"auto_apply":<bool>     // default false; true means system may apply
                         // fixes without user confirmation (caller-gated)

=== Business rules (caller-supplied, NOT in this prompt) ===
The caller injects business-specific diagnostic rules into the user
message, e.g.:
  "If X is missing AND Y is missing → critical"
  "If template is older than 2 versions → medium"
You apply these rules; you do not invent them.

=== Hard rules ===
1. critical / high issues MUST include a `fix`. Empty fix → rejected.
2. `fix.data` schema must match the corresponding chat action exactly.
3. `auto_apply=true` requires the caller to have an apply-with-validation
   hook configured; otherwise the system downgrades to false.
"#;

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmDiagnoseRequest {
    pub nodes: Vec<WorkflowNode>,
    pub workflow_name: String,
    pub workflow_description: Option<String>,
}

/// LLM 增强诊断:返回 V2 协议 `DiagnosticReportV2`
///
/// - `summary` / `issues` / `suggestions` 为诊断内容
/// - `fixes` 顶层数组 — 由系统通过 `dedup_fixes` 去重后用于批应用
/// - `auto_apply` 标志 — true 时要求调用方配置 apply-with-validation hook
#[tauri::command]
pub async fn llm_diagnose_workflow(
    state: State<'_, AppState>,
    request: LlmDiagnoseRequest,
) -> Result<DiagnosticReportV2, String> {
    // 构造诊断 prompt
    let mut node_summaries = Vec::new();
    for node in &request.nodes {
        let summary = match node {
            WorkflowNode::Agent(n) => format!(
                "- Agent '{}': system_prompt='{}' ({} chars), tools={:?}
",
                n.base.id,
                n.config.system_prompt.chars().take(100).collect::<String>(),
                n.config.system_prompt.len(),
                n.config.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
            ),
            WorkflowNode::Llm(n) => format!(
                "- LLM '{}': prompt='{}' ({} chars), model={:?}
",
                n.base.id,
                n.config.prompt.chars().take(100).collect::<String>(),
                n.config.prompt.len(),
                n.config.model
            ),
            WorkflowNode::Condition(n) => format!(
                "- Condition '{}': {} conditions, logical_op={:?}, judge_by_llm={:?}
",
                n.base.id,
                n.config.conditions.len(),
                n.config.logical_op,
                n.config.judge_by_llm
            ),
            WorkflowNode::HttpRequest(n) => format!(
                "- HttpRequest '{}': {} {} (timeout={}s, retry={:?})
",
                n.base.id, n.config.method, n.config.url, n.config.timeout_secs, n.base.retry
            ),
            _ => format!(
                "- {} '{}'
",
                node_type_name(node),
                node.base_id()
            ),
        };
        node_summaries.push(summary);
    }

    let base_prompt = format!(
        "You are a workflow diagnostic expert. Analyze this workflow and identify issues across 5 dimensions.

Workflow name: {name}
Description: {desc}

Nodes:
{nodes}

Please analyze and provide a JSON response with:
1. summary: Overall workflow health summary (1-2 sentences)
2. issues: Array of issues found, each with:
   - severity: error | warning | info
   - category: prompt_quality | performance | cost | security | best_practice
   - node_id: Node ID if applicable, or null
   - title: Short issue title
   - detail: Detailed description
   - suggestion: How to fix it
3. suggestions: Array of general improvement suggestions (3-5 items)

Diagnostic dimensions:
1. prompt_quality — agent/llm system_prompt 是否清晰、有角色定义、输出约束、错误处理
2. performance — 长链同步串行、缺少 parallel 加速、循环无 max_iterations、httpRequest/dbQuery 无超时、documentParser 缺 parser_type
3. cost — LLM/Agent 节点无 max_tokens、max_tool_rounds 未设置、温度未调优（默认 0.7 适合生成但不适合分类）、vectorRetrieve top_k 过大
4. security — httpRequest/webhookSend/notification URL 走 http 非 https、approval 无 approver、email 凭据硬编码、approval timeout 过长、vectorRetrieve 缺 similarity_threshold、SubWorkflow 无沙箱提示
5. best_practice — 变量命名、错误处理、链路完整性（有无 start/end）、condition 后是否双出口、loop 是否有终止条件、validation 覆盖关键节点
6. structure — debate 容器是否有至少 2 个辩手子节点、debater_steps 是否引用了存在的子节点、容器子节点是否正确设置 parentId

Respond ONLY with valid JSON.",
        name = request.workflow_name,
        desc = request.workflow_description.as_deref().unwrap_or(""),
        nodes = node_summaries.join(""),
    );

    let prompt = format!("{base_prompt}{UPSTREAM_EXTENSION_FOR_DIAGNOSE}");

    // 查找默认 provider 调用 LLM
    let db = state.harness.db();
    let providers = provider::list_providers(db)
        .await
        .map_err(|e| e.to_string())?;
    let default_prov = providers
        .iter()
        .find(|p| p.enabled)
        .ok_or("No enabled provider found")?;
    let key = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(&default_prov.id))
        .filter(provider_keys::Column::Enabled.eq(1))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No enabled API key found")?;
    let api_key =
        decrypt_key(&key.key_encrypted, state.harness.master_key()).map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": "deepseek-chat",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.1,
        "max_tokens": 2000,
        "response_format": { "type": "json_object" }
    });

    let resp = client
        .post(format!("{}/chat/completions", default_prov.api_host.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM call failed: {e}"))?;

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {e}"))?;
    let content = result["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("{}");
    let parsed: DiagnosticReportV2 =
        serde_json::from_str(content).map_err(|e| format!("LLM response parse failed: {e}"))?;

    // 协议层校验:critical / high 必有 fix;auto_apply + 空 fixes 拒绝
    validate_report(&parsed)?;

    Ok(parsed)
}

// ============================================================
// apply_diagnostic_fixes —— 把 LLM 输出的 fixes 批量落地
// ============================================================

/// `apply_diagnostic_fixes` 命令的入参
#[derive(Debug, Clone, Deserialize)]
pub struct ApplyDiagnosticFixesRequest {
    /// 从 LLM 报告 `fixes[]` 字段传过来的 fix 列表
    pub fixes: Vec<DiagnosticFix>,
    /// `auto_apply=true` 时表示调用方已配置 validation hook,允许后端走
    /// `apply_diff_with_validation` 调度器跑校验 + 可选回滚
    #[serde(default)]
    pub auto_apply: bool,
    /// 上下文注入 marker(透传到结果,供未来 chat 路径消费)。
    /// 已知 key:`version_history` / `diagnostic`;未知 key 走 `Custom` 透传。
    /// 当前 diagnose 路径不直接消费 marker 内容,仅做 round-trip 验证序列化。
    #[serde(default)]
    pub inject_context_marker: Option<InjectContextMarker>,
}

/// `apply_diagnostic_fixes` 命令的返回
#[derive(Debug, Clone, Serialize)]
pub struct ApplyDiagnosticFixesResult {
    /// 接收的 fix 总数(去重前)
    pub received: usize,
    /// 去重后的 fix 数(实际进入调度器的)
    pub deduped: usize,
    /// 调度结果(走 `apply_diff_with_validation` 时的摘要)
    /// 当 `auto_apply=false` 时,`validation_passed` 一律为 true(仅登记不执行)
    pub validation_passed: bool,
    pub applied: Vec<String>,
    pub rolled_back: bool,
    pub error: Option<String>,
    /// 透传回请求中的 marker(供前端读取)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject_context_marker: Option<InjectContextMarker>,
}

/// 批量应用 `DiagnosticFix` 列表
///
/// ## 流程
/// 1. `dedup_fixes` 去重
/// 2. 拆分两类:
///    - **新 4 种(基础设施类)**:可映射到 `ChatAction`,由后端
///      `apply_diff_with_validation` 调度器跑(可走 backtest hook)
///    - **原 6 种(节点级 UI)**:不通过本命令落地,由前端 `applyDiagnoseFix`
///      走 store 路径处理(返回 `client_fix_count` 计数,前端用此触发)
/// 3. 校验协议层 `validate_report` 约束(critical / high 必带 fix 已由 issue 侧保证)
///
/// ## 限制
/// - 原 6 种(set_node_field / delete_node / delete_edge / enable_retry /
///   set_timeout / remove_debater_step)不走后端 apply,仅记录在结果里
///   (`client_fix_count`),前端看到 > 0 时主动走本地 store 路径
#[tauri::command]
pub async fn apply_diagnostic_fixes(
    state: State<'_, AppState>,
    request: ApplyDiagnosticFixesRequest,
) -> Result<ApplyDiagnosticFixesResult, String> {
    use super::workflow_ai_protocol::dedup_fixes;

    let received = request.fixes.len();
    let deduped = dedup_fixes(&request.fixes);

    // 拆分:原 6 种客户端处理;新 4 种走调度器
    let mut server_actions: Vec<ChatAction> = Vec::new();
    let mut client_fix_count = 0;
    for fix in &deduped {
        if let Some(action) = fix_to_chat_action(fix) {
            server_actions.push(action);
        } else {
            client_fix_count += 1;
        }
    }

    // 没有可调度的 action → 直接返回客户端计数
    if server_actions.is_empty() {
        return Ok(ApplyDiagnosticFixesResult {
            received,
            deduped: deduped.len(),
            validation_passed: true,
            applied: Vec::new(),
            rolled_back: false,
            error: None,
            inject_context_marker: request.inject_context_marker,
        });
    }

    // 调 apply_diff_with_validation 调度器
    // 该函数已支持 5 种 ChatAction 调度,无 validation hook 时 no-op pass
    let scheduler_result = super::workflow_ai_apply::apply_diff_with_validation(
        state.clone(),
        server_actions.clone(),
        super::workflow_ai_protocol::ValidationSpec {
            r#type: "diagnostic".to_string(),
            params: serde_json::json!({
                "client_fix_count": client_fix_count,
                "auto_apply": request.auto_apply,
            }),
        },
        Some(request.auto_apply),
    )
    .await?;

    Ok(ApplyDiagnosticFixesResult {
        received,
        deduped: deduped.len(),
        validation_passed: scheduler_result.validation_passed,
        applied: scheduler_result.applied,
        rolled_back: scheduler_result.rolled_back,
        error: scheduler_result.error,
        inject_context_marker: request.inject_context_marker,
    })
}

/// 把 `DiagnosticFix` 中"可由后端 apply_* 命令消费"的新 4 种映射成 `ChatAction`。
/// 原 6 种(节点级 UI 操作)返回 `None`,由前端 store 路径处理。
fn fix_to_chat_action(fix: &DiagnosticFix) -> Option<ChatAction> {
    use super::workflow_ai_protocol::EditAssetFilePayload;
    match fix {
        DiagnosticFix::UpdateVariable {
            template_id,
            name,
            value,
        } => Some(ChatAction::UpdateVariable {
            data: super::workflow_ai_protocol::UpdateVariablePayload {
                template_id: template_id.clone(),
                name: name.clone(),
                value: value.clone(),
            },
        }),
        DiagnosticFix::UpdateInputMapping { node_id, mappings } => {
            Some(ChatAction::UpdateInputMapping {
                data: super::workflow_ai_protocol::UpdateInputMappingPayload {
                    node_id: node_id.clone(),
                    mappings: mappings
                        .iter()
                        .map(|m| InputMappingEntry {
                            target: m.target.clone(),
                            source: m.source.clone(),
                        })
                        .collect(),
                },
            })
        },
        DiagnosticFix::EditAssetFile {
            path,
            operation,
            anchor_line,
            code,
            description,
        } => Some(ChatAction::EditAssetFile {
            data: EditAssetFilePayload {
                path: path.clone(),
                operation: *operation,
                anchor_line: *anchor_line,
                code: code.clone(),
                description: description.clone(),
            },
        }),
        DiagnosticFix::RollbackToVersion {
            template_id,
            version,
        } => Some(ChatAction::RollbackToVersion {
            data: super::workflow_ai_protocol::RollbackToVersionPayload {
                template_id: template_id.clone(),
                version: *version,
            },
        }),
        // 原 6 种(节点级 UI)由前端 store 路径处理
        DiagnosticFix::SetNodeField { .. }
        | DiagnosticFix::DeleteNode { .. }
        | DiagnosticFix::DeleteEdge { .. }
        | DiagnosticFix::EnableRetry { .. }
        | DiagnosticFix::SetTimeout { .. }
        | DiagnosticFix::RemoveDebaterStep { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── fix_to_chat_action: 新 4 种映射 ──

    #[test]
    fn fix_to_chat_action_update_variable() {
        let fix = DiagnosticFix::UpdateVariable {
            template_id: "t1".to_string(),
            name: "score".to_string(),
            value: json!(0.5),
        };
        let action = fix_to_chat_action(&fix).expect("UpdateVariable should map to ChatAction");
        assert!(matches!(action, ChatAction::UpdateVariable { .. }));
    }

    #[test]
    fn fix_to_chat_action_update_input_mapping() {
        let fix = DiagnosticFix::UpdateInputMapping {
            node_id: "n1".to_string(),
            mappings: vec![InputMappingEntry {
                target: "a".to_string(),
                source: "b".to_string(),
            }],
        };
        let action = fix_to_chat_action(&fix).expect("UpdateInputMapping should map to ChatAction");
        assert!(matches!(action, ChatAction::UpdateInputMapping { .. }));
    }

    #[test]
    fn fix_to_chat_action_edit_asset_file() {
        use super::super::workflow_ai_protocol::EditAssetOperation;
        let fix = DiagnosticFix::EditAssetFile {
            path: "x.rhai".to_string(),
            operation: EditAssetOperation::InsertAfter,
            anchor_line: 5,
            code: Some("print(1);".to_string()),
            description: "insert".to_string(),
        };
        let action = fix_to_chat_action(&fix).expect("EditAssetFile should map to ChatAction");
        assert!(matches!(action, ChatAction::EditAssetFile { .. }));
    }

    #[test]
    fn fix_to_chat_action_rollback_to_version() {
        let fix = DiagnosticFix::RollbackToVersion {
            template_id: "t1".to_string(),
            version: 3,
        };
        let action = fix_to_chat_action(&fix).expect("RollbackToVersion should map to ChatAction");
        assert!(matches!(action, ChatAction::RollbackToVersion { .. }));
    }

    // ── fix_to_chat_action: 原 6 种 client-side ──

    #[test]
    fn fix_to_chat_action_set_node_field_returns_none() {
        let fix = DiagnosticFix::SetNodeField {
            node_id: "n".to_string(),
            field: "x".to_string(),
            value: json!(1),
        };
        assert!(fix_to_chat_action(&fix).is_none());
    }

    #[test]
    fn fix_to_chat_action_delete_node_returns_none() {
        let fix = DiagnosticFix::DeleteNode {
            node_id: "n".to_string(),
        };
        assert!(fix_to_chat_action(&fix).is_none());
    }

    #[test]
    fn fix_to_chat_action_delete_edge_returns_none() {
        let fix = DiagnosticFix::DeleteEdge {
            edge_id: "e".to_string(),
        };
        assert!(fix_to_chat_action(&fix).is_none());
    }

    #[test]
    fn fix_to_chat_action_enable_retry_returns_none() {
        let fix = DiagnosticFix::EnableRetry {
            node_id: "n".to_string(),
            max_retries: 3,
        };
        assert!(fix_to_chat_action(&fix).is_none());
    }

    #[test]
    fn fix_to_chat_action_set_timeout_returns_none() {
        let fix = DiagnosticFix::SetTimeout {
            node_id: "n".to_string(),
            timeout_ms: 1000,
        };
        assert!(fix_to_chat_action(&fix).is_none());
    }

    #[test]
    fn fix_to_chat_action_remove_debater_step_returns_none() {
        let fix = DiagnosticFix::RemoveDebaterStep {
            node_id: "n".to_string(),
            step_id: "s".to_string(),
        };
        assert!(fix_to_chat_action(&fix).is_none());
    }
}
