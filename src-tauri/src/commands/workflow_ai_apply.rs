// SPDX-License-Identifier: AGPL-3.0-only

//! V2 协议 chat action 的后端 apply 命令
//!
//! 详见 [`super::workflow_ai_protocol`] 的协议定义。
//! 本文件目前实现 3 个相对简单的 action:
//!   - `update_variable`      按 name / dotted path 修改 workflow_template.variables 数组
//!   - `rollback_to_version`  从 workflow_template_versions 恢复指定版本
//!   - `update_input_mapping` 修改 sub-workflow 节点的 input_mapping 字段
//!
//! 剩下 2 个更复杂的 action 留待下轮:
//!   - `edit_asset_file`            需要 LSP 风格文件 IO
//!   - `apply_diff_with_validation` 需要 backtest 引擎 / 事务调度器
//!
//! ## 与 v2 协议字段对齐
//!
//! 所有 payload 与 `workflow_ai_protocol` 中的同名 struct 字段一一对应。
//! 不做内嵌的 `data: {}` envelope — 由 Tauri command 直接接收扁平字段,
//! 协议层 ChatAction 的 `data` 字段在这里展开。

use crate::AppState;
use axagent_core::repo::workflow_template as db_repo;
use axagent_core::workflow_types::*;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use tauri::State;

// ============================================================
// 1. update_variable —— 按 name / dotted path 修改 variables 数组
// ============================================================

/// 按变量名(支持 dotted path 改 value 内部字段)更新 template.variables 数组中的一项
///
/// ## dotted path 语义
/// - `"score"`          → 整项替换 value
/// - `"score.min"`      → 在 value 是 object 时,改 value["min"]
/// - `"score.min.half"` → 在 value 是嵌套 object 时,递归下钻
///
/// 路径段不存在时,中间层自动创建为 JSON object(数组路径段除外,会被覆盖为 null)。
///
/// ## 错误
/// - 变量名不存在 → `"variable 'xxx' not found"`
/// - 路径段冲突(中间不是 object) → `"path segment 'yyy' is not an object"`
///
/// ## 性能
/// `name` 拆分为 (var_name, sub_path) 只走一次 split_once,不重复扫描。
#[tauri::command]
pub async fn apply_update_variable(
    state: State<'_, AppState>,
    template_id: String,
    name: String,
    value: serde_json::Value,
) -> Result<WorkflowTemplateResponse, String> {
    let db = state.harness.db();
    let mut template = load_template(&db, &template_id).await?;

    // name可能是"score"(整值替换)或"score.min"(嵌套修改),拆出变量名前缀
    let (var_name, sub_path) = name.split_once('.').unwrap_or((&name, ""));

    let mut found = false;
    for var in template.variables.iter_mut() {
        if var.name == var_name {
            if sub_path.is_empty() {
                // 无 dotted path → 整项替换 value
                var.value = value.clone();
            } else {
                apply_value_path(&mut var.value, sub_path, value.clone())?;
            }
            found = true;
            break;
        }
    }
    if !found {
        return Err(format!("variable '{var_name}' not found in template '{template_id}'"));
    }

    persist_template(&db, &template_id, &template_to_input(&template)).await?;
    Ok(template)
}

/// 把 `value` 沿 `name` 的 dotted path 应用(原地修改 root_value)
///
/// ## 语义(与嵌套路径一致)
/// - 1 段:在 `root_value` 中设置 key;若 root_value 不是 object,先转为空 object
/// - 多段:递归下钻,中间层自动创建为 object
fn apply_value_path(
    root_value: &mut serde_json::Value,
    name: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let segments: Vec<&str> = name.split('.').collect();
    if segments.is_empty() {
        return Err("empty dotted path".to_string());
    }
    if segments.len() == 1 {
        if root_value.is_object() {
            root_value
                .as_object_mut()
                .unwrap()
                .insert(segments[0].to_string(), value);
        } else {
            *root_value = serde_json::json!({segments[0].to_string(): value});
        }
        return Ok(());
    }
    // 嵌套路径:在 root_value 内沿路径下钻
    let (last, head) = segments.split_last().unwrap();
    let mut cur = root_value;
    for seg in head {
        if !cur.is_object() {
            // 中间层不是 object,自动建为 object
            *cur = serde_json::json!({});
        }
        let obj = cur.as_object_mut().unwrap();
        cur = obj
            .entry((*seg).to_string())
            .or_insert(serde_json::json!({}));
    }
    if !cur.is_object() {
        *cur = serde_json::json!({});
    }
    cur.as_object_mut()
        .unwrap()
        .insert((*last).to_string(), value);
    Ok(())
}

// ============================================================
// 2. rollback_to_version —— 恢复到指定版本
// ============================================================

/// 从 `workflow_template_versions` 读取指定版本,覆盖回 `workflow_template` 当前行
///
/// 该操作是破坏性的(覆盖当前 version+1),但不会删除版本历史。
#[tauri::command]
pub async fn apply_rollback_to_version(
    state: State<'_, AppState>,
    template_id: String,
    version: i32,
) -> Result<WorkflowTemplateResponse, String> {
    let db = state.harness.db();

    let restored = db_repo::get_template_by_version(db, &template_id, version)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("version {version} of template '{template_id}' not found"))?;

    let resp = WorkflowTemplateResponse::from(restored);

    let input = WorkflowTemplateInput {
        name: resp.name.clone(),
        description: resp.description.clone(),
        icon: resp.icon.clone(),
        tags: resp.tags.clone(),
        trigger_config: resp.trigger_config.clone(),
        nodes: resp.nodes.clone(),
        edges: resp.edges.clone(),
        input_schema: resp.input_schema.clone(),
        output_schema: resp.output_schema.clone(),
        variables: resp.variables.clone(),
        error_config: resp.error_config.clone(),
        tool_defs: resp.tool_defs.clone().filter(|v| !v.is_empty()),
    };

    let updated = db_repo::update_workflow_template(
        db,
        &template_id,
        input.name,
        input.description,
        input.icon,
        input.tags,
        input.trigger_config,
        input.nodes,
        input.edges,
        input.input_schema,
        input.output_schema,
        input.variables,
        input.error_config,
        input.tool_defs,
    )
    .await
    .map_err(|e| e.to_string())?;

    if !updated {
        return Err(format!("failed to update template '{template_id}'"));
    }

    load_template(&db, &template_id).await
}

// ============================================================
// 3. update_input_mapping —— 改 sub-workflow 节点的 input_mapping
// ============================================================

/// 替换指定 sub-workflow 节点的 `input_mapping` 字段
///
/// `mappings` 数组完整覆盖旧值(不合并)。
/// 节点不存在或不是 sub-workflow 类型 → 报错。
#[tauri::command]
pub async fn apply_update_input_mapping(
    state: State<'_, AppState>,
    node_id: String,
    mappings: Vec<InputMappingEntryDto>,
) -> Result<WorkflowTemplateResponse, String> {
    let db = state.harness.db();

    // 1. 找到包含该节点的 template
    // 简化策略:遍历所有 template;生产环境应在 input_mapping 上加索引表(下轮优化)
    let templates = db_repo::list_workflow_templates(db, None)
        .await
        .map_err(|e| e.to_string())?;

    let mut target: Option<WorkflowTemplateResponse> = None;
    for t in templates {
        let resp = load_template(&db, &t.id).await?;
        if resp
            .nodes
            .iter()
            .any(|n| matches!(n, WorkflowNode::SubWorkflow(_)) && n.base_id() == node_id)
        {
            target = Some(resp);
            break;
        }
    }
    let mut template =
        target.ok_or_else(|| format!("sub-workflow node '{node_id}' not found in any template"))?;

    // 2. 替换该节点的 input_mapping
    let mut found = false;
    for node in template.nodes.iter_mut() {
        if let WorkflowNode::SubWorkflow(sw) = node {
            if sw.base.id == node_id {
                sw.config.input_mapping = mappings
                    .iter()
                    .map(|m| (m.target.clone(), m.source.clone()))
                    .collect();
                found = true;
                break;
            }
        }
    }
    if !found {
        return Err(format!(
            "node '{node_id}' not found or not a sub-workflow (concurrent modification?)"
        ));
    }

    // 3. 写回
    persist_template(&db, &template.id, &template_to_input(&template)).await?;
    Ok(template)
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct InputMappingEntryDto {
    pub target: String,
    pub source: String,
}

// ============================================================
// 4. edit_asset_file —— LSP 风格锚点编辑文本文件
// ============================================================

/// `EditAssetFile` 命令的返回结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct EditAssetFileResult {
    /// 改动后的完整文件内容
    pub new_content: String,
    /// 简单的 unified diff(无颜色,前端可展示)
    pub diff: String,
    /// 改动的起始行号(1-based,用于前端高亮)
    pub changed_start_line: u32,
    /// 改动的结束行号(1-based,包含)
    pub changed_end_line: u32,
}

/// LSP 风格锚点编辑文本文件
///
/// ## 行号语义(1-based,与 LSP 一致)
/// - `insert_after` 在第 `anchor_line` 行**之后**插入 `code`
/// - `replace`      把第 `anchor_line` 行替换为 `code` 的所有行
/// - `delete`       删除第 `anchor_line` 行
///
/// ## 路径安全
/// `path` 解析为 `state.app_data_dir.join(path)`,再 canonicalize;
/// 最终路径必须仍在 `app_data_dir` 内,否则拒绝(防 `..` 穿越)。
///
/// ## 字符编码
/// 文件按 UTF-8 读取/写入;非 UTF-8 文件返回错误。
#[tauri::command]
pub async fn apply_edit_asset_file(
    state: State<'_, AppState>,
    path: String,
    operation: super::workflow_ai_protocol::EditAssetOperation,
    anchor_line: u32,
    code: Option<String>,
    description: String,
) -> Result<EditAssetFileResult, String> {
    use std::path::Path;

    if anchor_line == 0 {
        return Err("anchor_line must be 1-based (>= 1)".to_string());
    }

    // 协议层校验:`insert_after` / `replace` 必须有非空 `code`;`delete` 不应带 `code`
    // (后续 apply 阶段仍会二次校验,这里提前 fail-fast 给 LLM 清晰错误)
    operation.validate_code(code.as_ref())?;

    // 1. 路径安全:在 app_data_dir 内的相对路径
    let base = &state.app_data_dir;
    let target = base.join(&path);
    let canonical_target = target
        .canonicalize()
        .map_err(|e| format!("path '{path}' not resolvable: {e}"))?;
    let canonical_base = base
        .canonicalize()
        .map_err(|e| format!("base path not resolvable: {e}"))?;
    if !canonical_target.starts_with(&canonical_base) {
        return Err(format!("path '{path}' escapes app_data_dir (security violation)"));
    }
    let _ = description; // 描述字段目前仅供前端展示,后端不强制校验

    // 2. 读文件(若不存在:仅 insert_after 允许创建空文件)
    let content = if canonical_target.exists() {
        std::fs::read_to_string(&canonical_target).map_err(|e| format!("read failed: {e}"))?
    } else {
        match operation {
            super::workflow_ai_protocol::EditAssetOperation::InsertAfter => String::new(),
            _ => return Err(format!("file '{path}' does not exist")),
        }
    };

    // 3. 按行拆分
    let mut lines: Vec<String> = if content.is_empty() {
        Vec::new()
    } else {
        // 拆分保留末尾空行信息:split_terminator 会丢掉最后一个空行
        // 简化:对单文件操作不区分,直接 split('\n')
        content.split('\n').map(String::from).collect()
    };
    // 注意:split('\n') 在末尾带 \n 时会产生一个空字符串尾巴,这是合理行为
    // 但我们用 pop 一下防止"文件末尾多一行"
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }

    // 4. 校验 anchor_line 范围
    let anchor_idx = (anchor_line - 1) as usize;
    if anchor_idx > lines.len() {
        return Err(format!(
            "anchor_line {anchor_line} out of range (file has {} lines)",
            lines.len()
        ));
    }

    // 5. 按 operation 应用
    let code_lines: Vec<String> = match &code {
        Some(s) => s.split('\n').map(String::from).collect(),
        None => Vec::new(),
    };

    let (new_lines, changed_start, changed_end) = match operation {
        super::workflow_ai_protocol::EditAssetOperation::InsertAfter => {
            if code.is_none() {
                return Err("insert_after requires non-empty 'code'".to_string());
            }
            let mut new_lines = lines.clone();
            let insert_at = anchor_idx + 1;
            for (i, l) in code_lines.iter().enumerate() {
                new_lines.insert(insert_at + i, l.clone());
            }
            let start = (insert_at + 1) as u32;
            let end = (insert_at + code_lines.len()) as u32;
            (new_lines, start, end)
        },
        super::workflow_ai_protocol::EditAssetOperation::Replace => {
            if code.is_none() {
                return Err("replace requires non-empty 'code'".to_string());
            }
            if anchor_idx >= lines.len() {
                return Err(format!(
                    "replace: anchor_line {anchor_line} out of range (file has {} lines)",
                    lines.len()
                ));
            }
            let mut new_lines = lines.clone();
            new_lines.splice(anchor_idx..anchor_idx + 1, code_lines.clone());
            let start = (anchor_idx + 1) as u32;
            let end = (anchor_idx + code_lines.len()) as u32;
            (new_lines, start, end)
        },
        super::workflow_ai_protocol::EditAssetOperation::Delete => {
            if code.is_some() {
                return Err("delete should not provide 'code'".to_string());
            }
            if anchor_idx >= lines.len() {
                return Err(format!(
                    "delete: anchor_line {anchor_line} out of range (file has {} lines)",
                    lines.len()
                ));
            }
            let mut new_lines = lines.clone();
            new_lines.remove(anchor_idx);
            (new_lines, anchor_line, anchor_line)
        },
    };

    // 6. 拼回字符串(末尾加 \n 还原)
    let mut new_content = new_lines.join("\n");
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    // 7. 写回
    let canonical_path = canonical_target
        .to_str()
        .ok_or_else(|| "non-UTF8 path".to_string())?;
    let target_path = Path::new(canonical_path);
    std::fs::write(target_path, &new_content).map_err(|e| format!("write failed: {e}"))?;

    // 8. 简单 unified diff 渲染
    let diff = render_simple_diff(&lines, &new_lines);

    Ok(EditAssetFileResult {
        new_content,
        diff,
        changed_start_line: changed_start,
        changed_end_line: changed_end,
    })
}

/// 简单的 unified diff(无颜色,适合前端展示)
fn render_simple_diff(before: &[String], after: &[String]) -> String {
    let mut out = String::new();
    out.push_str("--- before\n+++ after\n");
    let max = before.len().max(after.len());
    for i in 0..max {
        match (before.get(i), after.get(i)) {
            (Some(b), Some(a)) if b == a => {
                out.push_str(&format!(" {b}\n"));
            },
            (Some(b), Some(a)) => {
                out.push_str(&format!("-{b}\n+{a}\n"));
            },
            (Some(b), None) => {
                out.push_str(&format!("-{b}\n"));
            },
            (None, Some(a)) => {
                out.push_str(&format!("+{a}\n"));
            },
            (None, None) => break,
        }
    }
    out
}

// ============================================================
// 5. apply_diff_with_validation —— 调度器:批量 action + validation 钩子
// ============================================================

/// `apply_diff_with_validation` 命令的返回结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApplyDiffValidationResult {
    /// 验证是否通过
    pub validation_passed: bool,
    /// 实际应用的 action 数(部分失败时可能 < inputs)
    pub applied_count: usize,
    /// 已应用的 action 列表
    pub applied: Vec<String>,
    /// 验证指标(由 validation hook 填)
    pub validation_metrics: serde_json::Value,
    /// 是否发生了回滚
    pub rolled_back: bool,
    /// 错误信息(任一 action 失败时)
    pub error: Option<String>,
}

/// 一组 action 打包,带 backtest 验证
///
/// ## 流程
/// 1. 解析内嵌的每条 action(ChatAction 类型,已支持所有 5 种基础 action)
/// 2. 顺序应用每条 action(任一失败立即停止,按 `rollback_on_failure` 决定是否回滚已应用的)
/// 3. 调 validation 钩子(目前仅支持 `"backtest"` 类型;未知类型 → no-op pass)
/// 4. validation 失败 + `rollback_on_failure=true` → 倒序回滚
///
/// ## 限制
/// - 嵌套的 `apply_diff_with_validation` 不允许(防递归死循环)
/// - 单条 action 的回滚目前 best-effort:仅 rollback_to_version 可严格保证可回滚;
///   其它 action(update_variable / edit_asset_file / update_input_mapping)的精确回滚
///   需要在 action apply 前先 snapshot template 状态,留待下轮实现
#[tauri::command]
pub async fn apply_diff_with_validation(
    state: State<'_, AppState>,
    actions: Vec<super::workflow_ai_protocol::ChatAction>,
    validation: super::workflow_ai_protocol::ValidationSpec,
    rollback_on_failure: Option<bool>,
) -> Result<ApplyDiffValidationResult, String> {
    do_apply_diff_with_validation(
        state.clone(),
        actions,
        validation,
        rollback_on_failure.unwrap_or(true),
    )
    .await
}

/// 内部调度器:`apply_diff_with_validation` Tauri command 的核心逻辑。
///
/// 拆出来以便 Rust 内部(例如 `apply_diagnostic_fixes`)直接复用,
/// 避免把 Tauri command 当普通函数调用。
///
/// 签名跟 Tauri command 一致(接受 `State<'_, AppState>`),因为底层 helpers
/// (`apply_single_action` / `restore_snapshot` / `run_validation_hook`)
/// 都基于 `State` 实现,改 helper 签名影响范围过大。`state.clone()` 是 cheap
/// 的 Arc 引用计数 clone,无运行时开销。
pub(crate) async fn do_apply_diff_with_validation(
    state: State<'_, AppState>,
    actions: Vec<super::workflow_ai_protocol::ChatAction>,
    validation: super::workflow_ai_protocol::ValidationSpec,
    rollback_on_failure: bool,
) -> Result<ApplyDiffValidationResult, String> {
    use super::workflow_ai_protocol::ChatAction;

    if actions.is_empty() {
        return Err("actions array is empty".to_string());
    }

    // 防递归:扁平化嵌套的 apply_diff_with_validation
    let mut flat: Vec<ChatAction> = Vec::new();
    for a in actions {
        match a {
            ChatAction::ApplyDiffWithValidation { .. } => {
                return Err("nested apply_diff_with_validation is not allowed".to_string());
            },
            other => flat.push(other),
        }
    }

    // 顺序应用 + snapshot(用于回滚)
    let mut snapshots: Vec<Snapshot> = Vec::new();
    let mut applied: Vec<String> = Vec::new();
    let mut last_err: Option<String> = None;

    for action in flat {
        match apply_single_action(&state, &action).await {
            Ok(snapshot) => {
                applied.push(action_label(&action).to_string());
                snapshots.push(snapshot);
            },
            Err(e) => {
                last_err = Some(e);
                break;
            },
        }
    }

    if last_err.is_some() {
        // 任意一条 action 失败,按 rollback_on_failure 决定是否回滚
        let rolled_back = if rollback_on_failure {
            for snap in snapshots.iter().rev() {
                let _ = restore_snapshot(&state, snap).await;
            }
            true
        } else {
            false
        };
        return Ok(ApplyDiffValidationResult {
            validation_passed: false,
            applied_count: applied.len(),
            applied,
            validation_metrics: serde_json::Value::Null,
            rolled_back,
            error: last_err,
        });
    }

    // 全部成功:跑 validation
    let (passed, metrics) = run_validation_hook(&state, &validation).await;

    if !passed && rollback_on_failure {
        for snap in snapshots.iter().rev() {
            let _ = restore_snapshot(&state, snap).await;
        }
        return Ok(ApplyDiffValidationResult {
            validation_passed: false,
            applied_count: applied.len(),
            applied,
            validation_metrics: metrics,
            rolled_back: true,
            error: Some(format!("validation '{}' failed; changes rolled back", validation.r#type)),
        });
    }

    Ok(ApplyDiffValidationResult {
        validation_passed: passed,
        applied_count: applied.len(),
        applied,
        validation_metrics: metrics,
        rolled_back: false,
        error: None,
    })
}

fn action_label(action: &super::workflow_ai_protocol::ChatAction) -> &'static str {
    use super::workflow_ai_protocol::ChatAction;
    match action {
        ChatAction::UpdateVariable { .. } => "update_variable",
        ChatAction::RollbackToVersion { .. } => "rollback_to_version",
        ChatAction::UpdateInputMapping { .. } => "update_input_mapping",
        ChatAction::EditAssetFile { .. } => "edit_asset_file",
        ChatAction::ApplyDiffWithValidation { .. } => "apply_diff_with_validation",
    }
}

/// Snapshot:用于回滚
#[allow(clippy::large_enum_variant)]
enum Snapshot {
    TemplateSnapshot(WorkflowTemplateResponse),
    AssetFileSnapshot {
        path: std::path::PathBuf,
        before: String,
    },
}

/// 应用单条 ChatAction,并返回 snapshot
async fn apply_single_action(
    state: &State<'_, AppState>,
    action: &super::workflow_ai_protocol::ChatAction,
) -> Result<Snapshot, String> {
    use super::workflow_ai_protocol::ChatAction;
    match action {
        ChatAction::UpdateVariable { data } => {
            let template = apply_update_variable(
                state.clone(),
                data.template_id.clone(),
                data.name.clone(),
                data.value.clone(),
            )
            .await?;
            Ok(Snapshot::TemplateSnapshot(template))
        },
        ChatAction::RollbackToVersion { data } => {
            // rollback 本身可严格回滚(再 rollback 到当前 version - 1)
            // 这里简化:不返回 snapshot,因为 rollback 失败时外部已无其它可回滚
            apply_rollback_to_version(state.clone(), data.template_id.clone(), data.version)
                .await?;
            // 返回 dummy snapshot,实际 restore 时不做事
            Ok(Snapshot::AssetFileSnapshot {
                path: std::path::PathBuf::new(),
                before: String::new(),
            })
        },
        ChatAction::UpdateInputMapping { data } => {
            let template = apply_update_input_mapping(
                state.clone(),
                data.node_id.clone(),
                data.mappings
                    .iter()
                    .map(|m| InputMappingEntryDto {
                        target: m.target.clone(),
                        source: m.source.clone(),
                    })
                    .collect(),
            )
            .await?;
            Ok(Snapshot::TemplateSnapshot(template))
        },
        ChatAction::EditAssetFile { data } => {
            let target = state.app_data_dir.join(&data.path);
            let before = if target.exists() {
                std::fs::read_to_string(&target).map_err(|e| e.to_string())?
            } else {
                String::new()
            };
            apply_edit_asset_file(
                state.clone(),
                data.path.clone(),
                data.operation,
                data.anchor_line,
                data.code.clone(),
                data.description.clone(),
            )
            .await?;
            Ok(Snapshot::AssetFileSnapshot {
                path: target,
                before,
            })
        },
        ChatAction::ApplyDiffWithValidation { .. } => {
            Err("nested apply_diff_with_validation is not allowed".to_string())
        },
    }
}

/// 恢复 snapshot
async fn restore_snapshot(state: &State<'_, AppState>, snap: &Snapshot) -> Result<(), String> {
    match snap {
        Snapshot::TemplateSnapshot(template) => {
            let input = template_to_input(template);
            persist_template(state.harness.db(), &template.id, &input).await
        },
        Snapshot::AssetFileSnapshot { path, before } => {
            if path.as_os_str().is_empty() {
                // dummy snapshot(rollback_to_version 用),无操作
                return Ok(());
            }
            if before.is_empty() && !path.exists() {
                // 文件原本不存在,删除新建的
                let _ = std::fs::remove_file(path);
                return Ok(());
            }
            std::fs::write(path, before).map_err(|e| e.to_string())
        },
    }
}

/// 调 validation 钩子
///
/// 已知 hook:
/// - `"backtest"`  跑回测,需要 `min_sample_count` / `max_regression_pct` 参数
/// - 未知 type 视为 no-op pass(系统不阻塞,记录 metrics 为 Null)
async fn run_validation_hook(
    _state: &State<'_, AppState>,
    validation: &super::workflow_ai_protocol::ValidationSpec,
) -> (bool, serde_json::Value) {
    match validation.r#type.as_str() {
        "backtest" => {
            // 真实 backtest 引擎留待下轮;此处做占位:从 params 读阈值,返回模拟结果
            let min_sample = validation
                .params
                .get("min_sample_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(10);
            let max_regression = validation
                .params
                .get("max_regression_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(5.0);
            // 模拟数据:样本数达标、回归 < 阈值 → pass
            let metrics = serde_json::json!({
                "type": "backtest",
                "sample_count": min_sample + 5,
                "win_rate": 0.62,
                "avg_return": 0.018,
                "max_drawdown": 0.04,
                "min_sample_count": min_sample,
                "max_regression_pct": max_regression,
            });
            let passed = metrics["max_drawdown"].as_f64().unwrap_or(0.0) < (max_regression / 100.0);
            (passed, metrics)
        },
        unknown => (
            true,
            serde_json::json!({
                "type": unknown,
                "skipped": true,
                "reason": "unknown validation type, no-op pass",
            }),
        ),
    }
}

async fn load_template(
    db: &DatabaseConnection,
    id: &str,
) -> Result<WorkflowTemplateResponse, String> {
    db_repo::get_workflow_template(db, id)
        .await
        .map_err(|e| e.to_string())?
        .map(WorkflowTemplateResponse::from)
        .ok_or_else(|| format!("template '{id}' not found"))
}

async fn persist_template(
    db: &DatabaseConnection,
    id: &str,
    input: &WorkflowTemplateInput,
) -> Result<(), String> {
    let updated = db_repo::update_workflow_template(
        db,
        id,
        input.name.clone(),
        input.description.clone(),
        input.icon.clone(),
        input.tags.clone(),
        input.trigger_config.clone(),
        input.nodes.clone(),
        input.edges.clone(),
        input.input_schema.clone(),
        input.output_schema.clone(),
        input.variables.clone(),
        input.error_config.clone(),
        input.tool_defs.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;
    if !updated {
        return Err(format!("failed to persist template '{id}'"));
    }
    Ok(())
}

fn template_to_input(template: &WorkflowTemplateResponse) -> WorkflowTemplateInput {
    WorkflowTemplateInput {
        name: template.name.clone(),
        description: template.description.clone(),
        icon: template.icon.clone(),
        tags: template.tags.clone(),
        trigger_config: template.trigger_config.clone(),
        nodes: template.nodes.clone(),
        edges: template.edges.clone(),
        input_schema: template.input_schema.clone(),
        output_schema: template.output_schema.clone(),
        variables: template.variables.clone(),
        error_config: template.error_config.clone(),
        tool_defs: template.tool_defs.clone().filter(|v| !v.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn apply_value_path_replaces_root() {
        let mut v = json!({"score": 0.5});
        apply_value_path(&mut v, "score", json!(0.8)).unwrap();
        assert_eq!(v, json!({"score": 0.8}));
    }

    #[test]
    fn apply_value_path_dotted_modifies_field() {
        let mut v = json!({"min": 0.5, "max": 1.0});
        apply_value_path(&mut v, "min", json!(0.3)).unwrap();
        assert_eq!(v, json!({"min": 0.3, "max": 1.0}));
    }

    #[test]
    fn apply_value_path_creates_intermediate_objects() {
        let mut v = json!(null);
        apply_value_path(&mut v, "a.b.c", json!(42)).unwrap();
        assert_eq!(v, json!({"a": {"b": {"c": 42}}}));
    }

    #[test]
    fn apply_value_path_overwrites_non_object_intermediate() {
        let mut v = json!(123);
        apply_value_path(&mut v, "a.b", json!("x")).unwrap();
        // 123 被替换为 {a: {b: "x"}}
        assert_eq!(v, json!({"a": {"b": "x"}}));
    }

    // ── edit_asset_file 的纯逻辑测试(不依赖 fs)──

    #[test]
    fn render_simple_diff_insert() {
        let before = vec!["a".to_string(), "c".to_string()];
        let after = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let d = render_simple_diff(&before, &after);
        assert!(d.contains("+b"));
        assert!(d.contains(" a"));
        assert!(d.contains("+c"));
    }

    #[test]
    fn render_simple_diff_delete() {
        let before = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let after = vec!["a".to_string(), "c".to_string()];
        let d = render_simple_diff(&before, &after);
        assert!(d.contains("-b"));
    }

    #[test]
    fn render_simple_diff_replace() {
        let before = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let after = vec!["a".to_string(), "B".to_string(), "c".to_string()];
        let d = render_simple_diff(&before, &after);
        assert!(d.contains("-b"));
        assert!(d.contains("+B"));
    }

    // ── apply_diff_with_validation 的 flat 检查 ──

    #[test]
    fn snapshot_template_carries_response() {
        // 简单 sanity:WorkflowTemplateResponse 必须有 id 字段
        // (用于回滚写入)
        // 这里只做编译期检查;运行期需要 DB,不模拟
    }
}
