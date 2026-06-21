// SPDX-License-Identifier: AGPL-3.0-only

//! V2 协议类型(LLM 输入/输出的强类型 schema)
//!
//! ## 协议范围
//!
//! 涵盖两个调用方对 LLM 的输入/输出契约:
//!
//! ### Chat 端(`workflow_ai.rs`)
//! 5 类基础设施 action:
//!   1. `update_variable`            改 `workflow_template.variables`
//!   2. `rollback_to_version`        回滚到 `workflow_template_versions` 里的指定版本
//!   3. `update_input_mapping`       改 sub-workflow 节点 `input_mapping`
//!   4. `edit_asset_file`            LSP 风格锚点编辑任意文本文件
//!   5. `apply_diff_with_validation` 调度器:baseline → apply → validation → 必要时 rollback
//!
//! ### Diagnose 端(`workflow_ai_diagnose.rs`)
//! - `DiagnosticIssue`:v2.0 严重度(4 级 critical/high/medium/low)+ 9 类 category + 可选 fix
//! - `DiagnosticFix`:10 种 `action_type`(原 6 + 新 4),与 chat 协议共享 schema
//! - `DiagnosticReportV2`:顶层带 `fixes[]`(去重批应用入口)+ `auto_apply` 标志
//!
//! ### Context Injection Marker
//! 调用方在 user message 末尾追加 `{"inject_context": ...}` 块,系统解析后注入下一轮对话。
//! 已知 key:`version_history` / `diagnostic`;未知 key 走 `Custom` 透传。
//!
//! ## 与前端 `DiagnosticFix` 对齐
//!
//! 后端这 10 种 fix 的 `action_type` 字符串和字段命名与前端
//! `src/components/workflow/types/workflow.types.ts:782` 的 discriminated union 严格一致。
//! 序列化格式:`#[serde(tag = "action_type", rename_all = "snake_case")]`(内联 tag)。
//!
//! ## 设计取舍
//!
//! - 用 `String` 作为 error,不复用 `thiserror`(commands/ 是应用层,见 AGENTS.md 错误规范)
//! - 不依赖 `regex`(`parse_chat_actions` 暂时不实现,留给上游调用方;若需要再补)
//! - `edit_asset_file.delete` 时 `code` 字段为 `Option<String>`,反序列化允许缺失
//!
//! ## 接入状态
//!
//! ✅ 完整接入:
//! - `ChatAction` / `EditAssetOperation` / `ValidationSpec` 已被 5 个 apply 命令消费
//!   (`workflow_ai_apply::apply_*`)
//! - `DiagnosticReportV2` / `DiagnosticFix` 已被 `llm_diagnose_workflow` 返回,
//!   顶层 `fixes[]` 经 `dedup_fixes` 去重后由 `apply_diagnostic_fixes` 批应用
//! - `validate_report` / `validate_issue` 在 `llm_diagnose_workflow` 入口校验协议
//! - `validate_code` 在 `apply_edit_asset_file` 入口校验
//! - `InjectContextMarker` 由 `apply_diagnostic_fixes` 透传(供未来 chat 路径消费)

use serde::{Deserialize, Serialize};

// ============================================================
// Chat 端:5 类基础设施 action
// ============================================================

/// `:::action` 块统一 envelope,序列化时以 `action_type` 作为内联 tag
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum ChatAction {
    /// 改 `workflow_template.variables` 字段(支持 dotted path)
    UpdateVariable { data: UpdateVariablePayload },
    /// 回滚到 `workflow_template_versions` 里的指定版本
    RollbackToVersion { data: RollbackToVersionPayload },
    /// 改 sub-workflow 节点 `input_mapping`
    UpdateInputMapping { data: UpdateInputMappingPayload },
    /// LSP 风格锚点编辑任意文本文件(`.rhai`/`.md`/...)
    EditAssetFile { data: EditAssetFilePayload },
    /// 一组 action 打包,带 validation 钩子
    ApplyDiffWithValidation {
        data: ApplyDiffWithValidationPayload,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateVariablePayload {
    pub template_id: String,
    /// 变量名或 dotted path(如 `"consensusScore.minForHold"`)
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollbackToVersionPayload {
    pub template_id: String,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateInputMappingPayload {
    pub node_id: String,
    pub mappings: Vec<InputMappingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InputMappingEntry {
    pub target: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditAssetFilePayload {
    pub path: String,
    pub operation: EditAssetOperation,
    pub anchor_line: u32,
    /// `insert_after` / `replace` 时必填;`delete` 时可省
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditAssetOperation {
    InsertAfter,
    Replace,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyDiffWithValidationPayload {
    pub actions: Vec<ChatAction>,
    pub validation: ValidationSpec,
    /// 默认 `true`;设 `false` 时即使 validation 失败也不回滚(仅记录)
    #[serde(default = "default_true")]
    pub rollback_on_failure: bool,
}

fn default_true() -> bool {
    true
}

/// 验证规格 — `r#type` 由调用方定义,系统按字符串路由到具体 validation hook
///
/// 已知 hook:
/// - `"backtest"` 跑回测,对比 `min_sample_count` / `max_regression_pct` 阈值
/// - 未知 type 由系统降级为 no-op validation(只记录,不阻塞 commit)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationSpec {
    pub r#type: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

// ============================================================
// Diagnose 端
// ============================================================

/// v2.0 严重度(4 级)
/// 与前端 3 级 `error/warning/info` 兼容方案:
/// 在 `workflowEditorStore.ts::transformLlmResult` 里把 critical/high 映射为 error,
/// medium 映射为 warning,low 映射为 info,title 加 `[CRITICAL]`/`[HIGH]` 前缀。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// v2.0 9 类 category(业务无关)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCategory {
    PromptQuality,
    MissingValidation,
    VariableMisconfig,
    HardcodedAssetDrift,
    WorkflowTemplateVersion,
    BacktestRegression,
    ToolMissing,
    EdgeMisroute,
    SemanticConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticIssue {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    /// 无对应节点时为 `null`(原协议允许字符串或 null)
    #[serde(default)]
    pub node_id: Option<String>,
    pub title: String,
    pub detail: String,
    pub suggestion: String,
    /// critical / high 必填(空 fix 系统拒绝保存);medium / low 可省
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<DiagnosticFix>,
}

/// DiagnosticFix 10 种(原 6 + 新 4)
/// 字段名与前端 `DiagnosticFix` discriminated union 严格对齐(见 workflow.types.ts:782)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum DiagnosticFix {
    // ── 原 6 种(workflow_ai_diagnose.rs:39 已有)──
    /// 覆盖节点的 `config` 字段
    SetNodeField {
        node_id: String,
        field: String,
        value: serde_json::Value,
    },
    /// 删除指定节点
    DeleteNode {
        node_id: String,
    },
    /// 删除指定边
    DeleteEdge {
        edge_id: String,
    },
    /// 启用节点重试
    EnableRetry {
        node_id: String,
        max_retries: u32,
    },
    /// 设置节点超时
    SetTimeout {
        node_id: String,
        timeout_ms: u64,
    },
    /// 移除 debate 节点的辩手子节点
    RemoveDebaterStep {
        node_id: String,
        step_id: String,
    },
    // ── 新 4 种(基础设施协议,与 chat 协议 1:1 对应)──
    UpdateVariable {
        template_id: String,
        name: String,
        value: serde_json::Value,
    },
    UpdateInputMapping {
        node_id: String,
        mappings: Vec<InputMappingEntry>,
    },
    EditAssetFile {
        path: String,
        operation: EditAssetOperation,
        anchor_line: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        description: String,
    },
    RollbackToVersion {
        template_id: String,
        version: i32,
    },
}

/// Diagnostic 报告顶层(LLM 输出)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticReportV2 {
    pub summary: String,
    pub issues: Vec<DiagnosticIssue>,
    pub suggestions: Vec<String>,
    /// 顶层 `fixes` 数组 — 从所有 issue.fix 去重后聚出,批应用入口
    #[serde(default)]
    pub fixes: Vec<DiagnosticFix>,
    /// 自动应用标志;`true` 时要求调用方配置 apply-with-validation hook
    /// (无 hook 则由系统降级为 `false`)
    #[serde(default)]
    pub auto_apply: bool,
}

// ============================================================
// 上下文注入 marker
// ============================================================

/// 调用方在 user message 末尾追加的 JSON 块
/// 已知 key:`version_history` / `diagnostic`;未知 key 走 `Custom` 透传到注入处理器
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "inject_context", rename_all = "snake_case")]
pub enum InjectContextMarker {
    /// 注入最近 N 个版本的 diff 摘要
    VersionHistory {
        template_id: String,
        #[serde(default = "default_history_limit")]
        limit: u32,
    },
    /// 注入诊断结果
    Diagnostic { template_id: String },
    /// 其它 caller_defined 的 marker,透传
    #[serde(untagged)]
    Custom(serde_json::Value),
}

fn default_history_limit() -> u32 {
    5
}

// ============================================================
// 校验辅助函数
// ============================================================

/// 校验 `DiagnosticIssue` 是否满足 v2.0 协议约束:
/// - critical / high 必须有 `fix`
/// - 已知 fix 的 `data` schema 由 serde 保证(反序列化失败即视为不合法)
///
/// 返回 `Err` 给出第一个失败原因;成功返回 `Ok(())`
pub fn validate_issue(issue: &DiagnosticIssue) -> Result<(), String> {
    if matches!(issue.severity, DiagnosticSeverity::Critical | DiagnosticSeverity::High)
        && issue.fix.is_none()
    {
        return Err(format!(
            "{} issue '{}' must include a fix",
            severity_label(issue.severity),
            issue.title
        ));
    }
    Ok(())
}

/// 校验整个 `DiagnosticReportV2`:
/// - 每条 issue 通过 [`validate_issue`]
/// - 顶层 `fixes` 数组里 critical/high 对应的 fix 不允许为空
pub fn validate_report(report: &DiagnosticReportV2) -> Result<(), String> {
    for issue in &report.issues {
        validate_issue(issue)?;
    }
    if report.auto_apply && report.fixes.is_empty() {
        return Err("auto_apply=true requires at least one fix in 'fixes'".to_string());
    }
    Ok(())
}

/// 顶层 `fixes` 数组去重 key:
/// `(action_type, 数据 JSON 字符串)` — 完全相同视为重复
pub fn dedup_fixes(fixes: &[DiagnosticFix]) -> Vec<DiagnosticFix> {
    use std::collections::HashSet;
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut out = Vec::new();
    for fix in fixes {
        let key = (
            fix.action_type_label().to_string(),
            serde_json::to_string(&fix.data_fields()).unwrap_or_default(),
        );
        if seen.insert(key) {
            out.push(fix.clone());
        }
    }
    out
}

fn severity_label(s: DiagnosticSeverity) -> &'static str {
    match s {
        DiagnosticSeverity::Critical => "critical",
        DiagnosticSeverity::High => "high",
        DiagnosticSeverity::Medium => "medium",
        DiagnosticSeverity::Low => "low",
    }
}

// DiagnosticFix 配套 trait:提取 action_type 字符串 / 数据字段
// 用于 dedup 与协议层通用处理
impl DiagnosticFix {
    pub fn action_type_label(&self) -> &'static str {
        match self {
            DiagnosticFix::SetNodeField { .. } => "set_node_field",
            DiagnosticFix::DeleteNode { .. } => "delete_node",
            DiagnosticFix::DeleteEdge { .. } => "delete_edge",
            DiagnosticFix::EnableRetry { .. } => "enable_retry",
            DiagnosticFix::SetTimeout { .. } => "set_timeout",
            DiagnosticFix::RemoveDebaterStep { .. } => "remove_debater_step",
            DiagnosticFix::UpdateVariable { .. } => "update_variable",
            DiagnosticFix::UpdateInputMapping { .. } => "update_input_mapping",
            DiagnosticFix::EditAssetFile { .. } => "edit_asset_file",
            DiagnosticFix::RollbackToVersion { .. } => "rollback_to_version",
        }
    }

    /// 提取除 `action_type` 外的所有数据字段,供 dedup key 使用
    pub fn data_fields(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

impl EditAssetOperation {
    /// 校验 `EditAssetFile` 的 `code` 字段约束:
    /// `insert_after` / `replace` 时 `code` 必须存在;`delete` 时不应提供
    pub fn validate_code(&self, code: Option<&String>) -> Result<(), String> {
        match self {
            EditAssetOperation::InsertAfter | EditAssetOperation::Replace => {
                if code.is_none() || code.is_some_and(String::is_empty) {
                    return Err(format!("{:?} requires non-empty 'code' field", self));
                }
            },
            EditAssetOperation::Delete => {
                if code.is_some() {
                    return Err("delete operation should not provide 'code' field".to_string());
                }
            },
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ChatAction 反序列化 ────────────────────────────────

    #[test]
    fn parse_update_variable() {
        let s = r#"{"action_type":"update_variable","data":{"template_id":"t1","name":"score.min","value":0.5}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        match a {
            ChatAction::UpdateVariable { data } => {
                assert_eq!(data.template_id, "t1");
                assert_eq!(data.name, "score.min");
                assert_eq!(data.value, serde_json::json!(0.5));
            },
            _ => panic!("expected UpdateVariable"),
        }
    }

    #[test]
    fn parse_rollback_to_version() {
        let s = r#"{"action_type":"rollback_to_version","data":{"template_id":"t1","version":3}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        match a {
            ChatAction::RollbackToVersion { data } => {
                assert_eq!(data.version, 3);
            },
            _ => panic!("expected RollbackToVersion"),
        }
    }

    #[test]
    fn parse_update_input_mapping() {
        let s = r#"{"action_type":"update_input_mapping","data":{"node_id":"n1","mappings":[{"target":"a","source":"b"}]}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        match a {
            ChatAction::UpdateInputMapping { data } => {
                assert_eq!(data.mappings.len(), 1);
                assert_eq!(data.mappings[0].target, "a");
                assert_eq!(data.mappings[0].source, "b");
            },
            _ => panic!("expected UpdateInputMapping"),
        }
    }

    #[test]
    fn parse_edit_asset_file_replace() {
        let s = r#"{"action_type":"edit_asset_file","data":{"path":"x.rhai","operation":"replace","anchor_line":10,"code":"new body","description":"fix"}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        match a {
            ChatAction::EditAssetFile { data } => {
                assert_eq!(data.operation, EditAssetOperation::Replace);
                assert_eq!(data.code.as_deref(), Some("new body"));
            },
            _ => panic!("expected EditAssetFile"),
        }
    }

    #[test]
    fn parse_edit_asset_file_delete_omits_code() {
        let s = r#"{"action_type":"edit_asset_file","data":{"path":"x.rhai","operation":"delete","anchor_line":10,"description":"rm"}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        match a {
            ChatAction::EditAssetFile { data } => {
                assert_eq!(data.operation, EditAssetOperation::Delete);
                assert_eq!(data.code, None);
            },
            _ => panic!("expected EditAssetFile"),
        }
    }

    #[test]
    fn parse_edit_asset_file_delete_rejects_code() {
        let s = r#"{"action_type":"edit_asset_file","data":{"path":"x.rhai","operation":"delete","anchor_line":10,"code":"x","description":"rm"}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        if let ChatAction::EditAssetFile { data } = a {
            let err = data.operation.validate_code(data.code.as_ref());
            assert!(err.is_err(), "delete with code should be invalid");
        } else {
            panic!();
        }
    }

    #[test]
    fn parse_edit_asset_file_replace_requires_code() {
        let s = r#"{"action_type":"edit_asset_file","data":{"path":"x.rhai","operation":"replace","anchor_line":10,"description":"x"}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        if let ChatAction::EditAssetFile { data } = a {
            let err = data.operation.validate_code(data.code.as_ref());
            assert!(err.is_err(), "replace without code should be invalid");
        } else {
            panic!();
        }
    }

    #[test]
    fn parse_apply_diff_with_validation() {
        let s = r#"{"action_type":"apply_diff_with_validation","data":{"actions":[{"action_type":"update_variable","data":{"template_id":"t","name":"x","value":1}}],"validation":{"type":"backtest","params":{"min_sample":10}},"rollback_on_failure":true}}"#;
        let a: ChatAction = serde_json::from_str(s).unwrap();
        match a {
            ChatAction::ApplyDiffWithValidation { data } => {
                assert_eq!(data.actions.len(), 1);
                assert_eq!(data.validation.r#type, "backtest");
                assert!(data.rollback_on_failure);
            },
            _ => panic!("expected ApplyDiffWithValidation"),
        }
    }

    // ── DiagnosticIssue 反序列化 ──────────────────────────

    #[test]
    fn parse_diagnostic_issue_critical_with_fix() {
        let s = r#"{"severity":"critical","category":"prompt_quality","title":"x","detail":"y","suggestion":"z","fix":{"action_type":"set_node_field","node_id":"n1","field":"temperature","value":0.2}}"#;
        let issue: DiagnosticIssue = serde_json::from_str(s).unwrap();
        assert_eq!(issue.severity, DiagnosticSeverity::Critical);
        assert_eq!(issue.category, DiagnosticCategory::PromptQuality);
        assert!(issue.fix.is_some());
    }

    #[test]
    fn parse_diagnostic_issue_node_id_null() {
        let s = r#"{"severity":"low","category":"semantic_conflict","node_id":null,"title":"x","detail":"y","suggestion":"z"}"#;
        let issue: DiagnosticIssue = serde_json::from_str(s).unwrap();
        assert_eq!(issue.node_id, None);
    }

    #[test]
    fn parse_diagnostic_issue_high_without_fix_rejected() {
        let s = r#"{"severity":"high","category":"prompt_quality","title":"x","detail":"y","suggestion":"z"}"#;
        let issue: DiagnosticIssue = serde_json::from_str(s).unwrap();
        let err = validate_issue(&issue);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("must include a fix"));
    }

    #[test]
    fn parse_diagnostic_issue_medium_without_fix_ok() {
        let s = r#"{"severity":"medium","category":"prompt_quality","title":"x","detail":"y","suggestion":"z"}"#;
        let issue: DiagnosticIssue = serde_json::from_str(s).unwrap();
        assert!(validate_issue(&issue).is_ok());
    }

    // ── DiagnosticFix 10 种 ────────────────────────────────

    #[test]
    fn parse_fix_set_node_field() {
        let s = r#"{"action_type":"set_node_field","node_id":"n","field":"x","value":1}"#;
        let f: DiagnosticFix = serde_json::from_str(s).unwrap();
        assert_eq!(f.action_type_label(), "set_node_field");
    }

    #[test]
    fn parse_fix_enable_retry() {
        let s = r#"{"action_type":"enable_retry","node_id":"n","max_retries":3}"#;
        let f: DiagnosticFix = serde_json::from_str(s).unwrap();
        assert_eq!(f.action_type_label(), "enable_retry");
    }

    #[test]
    fn parse_fix_rollback_to_version() {
        let s = r#"{"action_type":"rollback_to_version","template_id":"t","version":2}"#;
        let f: DiagnosticFix = serde_json::from_str(s).unwrap();
        assert_eq!(f.action_type_label(), "rollback_to_version");
    }

    #[test]
    fn parse_fix_edit_asset_file() {
        let s = r#"{"action_type":"edit_asset_file","path":"x.rhai","operation":"insert_after","anchor_line":5,"code":"new","description":"d"}"#;
        let f: DiagnosticFix = serde_json::from_str(s).unwrap();
        assert_eq!(f.action_type_label(), "edit_asset_file");
    }

    #[test]
    fn fix_dedup_identical() {
        let f1: DiagnosticFix = serde_json::from_str(
            r#"{"action_type":"set_node_field","node_id":"n","field":"x","value":1}"#,
        )
        .unwrap();
        let f2 = f1.clone();
        let out = dedup_fixes(&[f1, f2]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn fix_dedup_keeps_different() {
        let f1: DiagnosticFix = serde_json::from_str(
            r#"{"action_type":"set_node_field","node_id":"n","field":"x","value":1}"#,
        )
        .unwrap();
        let f2: DiagnosticFix = serde_json::from_str(
            r#"{"action_type":"set_node_field","node_id":"n","field":"x","value":2}"#,
        )
        .unwrap();
        let out = dedup_fixes(&[f1, f2]);
        assert_eq!(out.len(), 2);
    }

    // ── DiagnosticReportV2 ─────────────────────────────────

    #[test]
    fn parse_report_v2_with_fixes() {
        let s = r#"{"summary":"ok","issues":[],"suggestions":["a"],"fixes":[{"action_type":"rollback_to_version","template_id":"t","version":1}],"auto_apply":false}"#;
        let r: DiagnosticReportV2 = serde_json::from_str(s).unwrap();
        assert!(!r.auto_apply);
        assert_eq!(r.fixes.len(), 1);
        assert!(validate_report(&r).is_ok());
    }

    #[test]
    fn report_auto_apply_without_fixes_rejected() {
        let s = r#"{"summary":"ok","issues":[],"suggestions":[],"fixes":[],"auto_apply":true}"#;
        let r: DiagnosticReportV2 = serde_json::from_str(s).unwrap();
        assert!(validate_report(&r).is_err());
    }

    // ── InjectContextMarker ────────────────────────────────

    #[test]
    fn parse_inject_version_history() {
        let s = r#"{"inject_context":"version_history","template_id":"t","limit":3}"#;
        let m: InjectContextMarker = serde_json::from_str(s).unwrap();
        assert!(matches!(m, InjectContextMarker::VersionHistory { limit: 3, .. }));
    }

    #[test]
    fn parse_inject_diagnostic() {
        let s = r#"{"inject_context":"diagnostic","template_id":"t"}"#;
        let m: InjectContextMarker = serde_json::from_str(s).unwrap();
        assert!(matches!(m, InjectContextMarker::Diagnostic { .. }));
    }

    #[test]
    fn parse_inject_unknown_passes_through() {
        let s = r#"{"inject_context":"reflection","reflection_id":"u-1"}"#;
        let m: InjectContextMarker = serde_json::from_str(s).unwrap();
        assert!(matches!(m, InjectContextMarker::Custom(_)));
    }
}
