// SPDX-License-Identifier: AGPL-3.0-only
//! SaveAsWorkflow — 把当前会话中已加载的能力组装为工作流模板并持久化。
//!
//! # 触发时机
//! Agent 在会话中通过 CapabilityLoad 加载了若干能力后，可主动调用本工具
//! 把这条能力序列沉淀为可复用的 WorkflowTemplate，存进 `workflow_templates` 表。
//!
//! # 数据流
//! ```text
//! SessionStateStore.list_by_prefix(NS_SKILL_LOADED)
//!   → 拿到已加载的 capability_id 列表
//!   → CapabilityIndexer.get_passport() 逐个取护照
//!   → expand_passports_recursive() 仅展开 Toolchain（Skill 保留原样）
//!       Toolchain → 遍历 steps → 逐 capability_id 查护照 → 递归
//!       Template → 跳过
//!       Skill    → 原样保留（由 AssemblyBuilder 映射为 AgentNode，保留 LLM 推理）
//!       其他（Tool/Workflow/KnowledgeBase/Agent）→ 原样保留
//!   → DefaultAssemblyBuilder.assemble_linear(passports) → AssemblyResult
//!       Skill → AgentNode（system_prompt + tools + rag_source_ids 从 skill_steps 收集）
//!       Toolchain 已在上层展开为 Tool/KB/Agent 等普通护照
//!   → nodes/edges 序列化为 JSON String
//!   → WorkflowTemplateRepository.create_workflow_template()
//! ```
//!
//! # 分层合规
//! 依赖链遵循 harness 铁律：
//! - `axagent_harness::AssemblyBuilder` — 纯 DTO 转换，foundation 层
//! - `axagent_harness::WorkflowTemplateRepository` — trait 契约
//! - `axagent_harness::CapabilityIndexer` — trait 契约
//! - `axagent_harness::SessionStateStore` — trait 契约
//! - 实际实现由 wiring 层（init/services.rs）通过 OnceLock 注入

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolErrorKind, ToolResult};
use async_trait::async_trait;
use axagent_harness::SessionStateStore;
use axagent_harness::assembly_builder::{AssemblyBuilder, DefaultAssemblyBuilder};
use axagent_harness::session_state::{NS_SKILL_LOADED, StateScope, namespace_prefix};
use serde_json::{Value, json};
use std::sync::Arc;

use super::capability_shared::capability_indexer;

static SESSION_STATE: std::sync::OnceLock<Arc<dyn SessionStateStore>> = std::sync::OnceLock::new();

pub fn set_session_state_store(store: Arc<dyn SessionStateStore>) {
    let _ = SESSION_STATE.set(store);
}

/// 展开 Toolchain 为其子能力护照列表（Skill 保留原样，由 AssemblyBuilder 映射为 AgentNode）。
///
/// # 规则
/// - Toolchain：遍历 `steps`（固定顺序工具串，无 prompt，展开为确定性 Tool/KB/Agent 序列）
/// - Skill：**原样保留不展开**（Skill 有 SKILL.md prompt，AssemblyBuilder 会把它映射为
///   AgentNode，保留 LLM ReAct 推理能力；skill_steps 里的子能力会自动收集到 AgentNode.tools）
/// - Template：跳过（占位符不生成节点也不展开）
/// - 其他（Tool / Workflow / KnowledgeBase / Agent）：原样保留
/// - 递归深度 ≤ 3 层（防止 Toolchain → Toolchain 循环引用）
/// - 同层去重：同一 capability_id 在同一展开层内只保留一次
///
/// # 为什么 Skill 不展开
/// Skill 的 SKILL.md 正文 prompt 本质是"给 LLM 的多步任务指令"，与 AgentNode 的
/// ReAct 执行模式天然对齐。如果把 Skill 展开为 ToolNode 序列（方式 A），
/// 就丢失了 LLM 中间决策能力，变成僵化的确定性 DAG。
/// 方式 B 让 Skill → AgentNode，保留推理灵活性。
async fn expand_passports_recursive(
    passports: Vec<axagent_harness::CapabilityPassportDto>,
    indexer: &Arc<dyn axagent_harness::CapabilityIndexer>,
    depth: u8,
) -> Vec<axagent_harness::CapabilityPassportDto> {
    let mut out: Vec<axagent_harness::CapabilityPassportDto> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let max_depth: u8 = 3;

    for p in passports {
        match p.kind {
            // Skill：原样保留，不展开。由 AssemblyBuilder 映射为 AgentNode。
            axagent_harness::CapabilityKind::Skill => {
                if seen_ids.insert(p.capability_id.clone()) {
                    out.push(p);
                }
            },
            axagent_harness::CapabilityKind::Toolchain => {
                for step_id in &p.steps {
                    if !seen_ids.insert(step_id.clone()) {
                        continue;
                    }
                    if let Some(child) = indexer.get_passport(step_id).await {
                        if depth < max_depth
                            && (child.kind == axagent_harness::CapabilityKind::Skill
                                || child.kind == axagent_harness::CapabilityKind::Toolchain)
                        {
                            let expanded = Box::pin(expand_passports_recursive(
                                vec![child],
                                indexer,
                                depth + 1,
                            ))
                            .await;
                            out.extend(expanded);
                        } else if child.kind != axagent_harness::CapabilityKind::Template {
                            out.push(child);
                        }
                    }
                }
            },
            axagent_harness::CapabilityKind::Template => {
                // Template 是占位符，不生成节点也不展开
            },
            _ => {
                // 其他类型原样保留
                if seen_ids.insert(p.capability_id.clone()) {
                    out.push(p);
                }
            },
        }
    }
    out
}

pub struct SaveAsWorkflowTool;

#[async_trait]
impl Tool for SaveAsWorkflowTool {
    fn name(&self) -> &str {
        "SaveAsWorkflow"
    }

    fn description(&self) -> &str {
        "把当前会话中已加载的能力组装为工作流模板并持久化。\
         适用于：多步任务完成后想把能力组合沉淀为可复用工作流，\
         避免下次重复手动加载。参数 name 为模板名（必填），\
         capability_ids 可选（省略则自动取本会话全部已加载能力）。"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "工作流模板名称（必填）"
                },
                "capability_ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "要组装的 capability_id 列表（省略则自动取本会话全部已加载能力）"
                },
                "icon": {
                    "type": "string",
                    "description": "图标 emoji 或标识，缺省 '🧩'"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "标签列表，缺省 ['capability-assembly', 'auto-saved']"
                },
                "strategy": {
                    "type": "string",
                    "enum": ["linear"],
                    "description": "组装策略，目前仅支持 'linear'（线性串接）"
                }
            },
            "required": ["name"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input["name"].as_str().unwrap_or("").trim();
        if name.is_empty() {
            return Err(ToolError::invalid_input_for("SaveAsWorkflow", "name 为必填参数"));
        }

        let conversation_id =
            ctx.conversation_id.clone().filter(|c| !c.trim().is_empty()).ok_or_else(|| {
                ToolError {
                    message: "缺少 conversation_id，无法读取会话状态".to_string(),
                    kind: ToolErrorKind::ExecutionFailed,
                    error_code: "SAVE_AS_WORKFLOW_NO_CONTEXT".to_string(),
                }
            })?;

        let store = SESSION_STATE.get().ok_or_else(|| ToolError {
            message: "会话状态存储未注入，SaveAsWorkflow 不可用".to_string(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: "SAVE_AS_WORKFLOW_NO_STORE".to_string(),
        })?;

        let indexer = capability_indexer().ok_or_else(|| ToolError {
            message: "能力索引器未初始化".to_string(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: "SAVE_AS_WORKFLOW_NO_INDEXER".to_string(),
        })?;

        // 1. 确定 capability_ids：优先用参数传入的，否则从 SessionStateStore 列出
        let capability_ids: Vec<String> = match &input["capability_ids"] {
            Value::Array(arr) if !arr.is_empty() => {
                arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()
            },
            _ => {
                let prefix =
                    namespace_prefix(StateScope::Temp, NS_SKILL_LOADED, &conversation_id, None);
                let entries = store.list_by_prefix(&prefix).await.map_err(|e| ToolError {
                    message: format!("读取会话状态失败: {e}"),
                    kind: ToolErrorKind::ExecutionFailed,
                    error_code: "SAVE_AS_WORKFLOW_LIST_FAILED".to_string(),
                })?;

                if entries.is_empty() {
                    return Err(ToolError {
                        message: "本会话没有已加载的能力，无法组装工作流。请先通过 CapabilityLoad 加载能力。".to_string(),
                        kind: ToolErrorKind::ExecutionFailed,
                        error_code: "SAVE_AS_WORKFLOW_NO_LOADED_CAPS".to_string(),
                    });
                }

                // value 是 JSON { capabilityId, kind, name, ... }，提取 capabilityId
                entries
                    .iter()
                    .filter_map(|e| {
                        serde_json::from_str::<Value>(&e.value)
                            .ok()
                            .and_then(|v| v["capabilityId"].as_str().map(str::to_string))
                    })
                    .collect()
            },
        };

        if capability_ids.is_empty() {
            return Err(ToolError {
                message: "没有有效的 capability_id，无法组装工作流".to_string(),
                kind: ToolErrorKind::ExecutionFailed,
                error_code: "SAVE_AS_WORKFLOW_EMPTY_IDS".to_string(),
            });
        }

        // 2. 逐个取护照
        let mut passports = Vec::new();
        for cap_id in &capability_ids {
            if let Some(passport) = indexer.get_passport(cap_id).await {
                passports.push(passport);
            }
            // 跳过找不到的，不阻塞整体组装
        }

        if passports.is_empty() {
            return Err(ToolError {
                message: "所有 capability_id 均未在索引中找到，无法组装".to_string(),
                kind: ToolErrorKind::ExecutionFailed,
                error_code: "SAVE_AS_WORKFLOW_ALL_MISSING".to_string(),
            });
        }

        // 2.5 递归展开 Skill/Toolchain 为子能力护照。
        // 这是 AssemblyBuilder 的上层 resolver 职责（foundation 层拿不到 CapabilityIndexer）。
        let original_count = passports.len();
        let passports = expand_passports_recursive(passports, &indexer, 0).await;

        if passports.is_empty() {
            return Err(ToolError {
                message: format!(
                    "选定的 {} 个能力全是 Skill/Toolchain/Template 类型，且无法展开为可组装节点。\
                     请确保 Skill 的 skill_steps 或 Toolchain 的 steps 引用了实际的 Tool/知识库/Agent。",
                    original_count
                ),
                kind: ToolErrorKind::ExecutionFailed,
                error_code: "SAVE_AS_WORKFLOW_ALL_UNEXPANDABLE".to_string(),
            });
        }

        // 3. 组装
        let builder = DefaultAssemblyBuilder::new().with_prefix("auto");
        let mut result = builder.assemble_linear(&passports);

        // 3.5 后处理：补全 AgentNode（Skill 映射而来）的 ToolDef.description。
        // AssemblyBuilder 是 foundation 层，拿不到 CapabilityIndexer，ToolDef.description 为空。
        // 本工具在 hybrid 层，有 indexer，遍历 AgentNode 的 tools 列表查子能力护照，
        // 把 passport.description 填进去——提升运行时 LLM 工具选择准确度。
        // 找不到子能力的 ToolDef.description 保持 None（Agent executor 会在运行时补全）。
        for node in result.nodes.iter_mut() {
            if let axagent_harness::workflow_types::WorkflowNode::Agent(agent_node) = node {
                for tool_def in agent_node.config.tools.iter_mut() {
                    // let-chain 合并嵌套 if：仅当描述为空时才查子能力护照（避免无谓的索引查询）
                    if tool_def.description.is_none()
                        && let Some(child_passport) = indexer.get_passport(&tool_def.name).await
                    {
                        tool_def.description = Some(child_passport.description.clone());
                    }
                }
            }
        }

        if result.nodes.is_empty() {
            return Err(ToolError {
                message: "展开后的能力仍无法生成工作流节点，请检查能力的 kind 和实现完整性"
                    .to_string(),
                kind: ToolErrorKind::ExecutionFailed,
                error_code: "SAVE_AS_WORKFLOW_NO_NODES".to_string(),
            });
        }

        // 4. 序列化为 WorkflowTemplateData 需要的 JSON 字符串
        let nodes_json = serde_json::to_string(&result.nodes).map_err(|e| ToolError {
            message: format!("序列化节点失败: {e}"),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: "SAVE_AS_WORKFLOW_SERIALIZE_FAILED".to_string(),
        })?;
        let edges_json = serde_json::to_string(&result.edges).map_err(|e| ToolError {
            message: format!("序列化边失败: {e}"),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: "SAVE_AS_WORKFLOW_SERIALIZE_FAILED".to_string(),
        })?;

        let icon = input["icon"].as_str().unwrap_or("🧩").to_string();
        // tags 落库列是 JSON 数组字符串（dao 层用 serde_json::to_string(&Vec<String>) 写入、
        // 读模型时用 serde_json::from_str 解析）。此前这里用 join(",") 拼逗号串，
        // 读回时 JSON 解析失败被降级为空数组，导致固化模板的标签全部丢失、
        // 能力索引拿不到语义标签。改为按 JSON 数组序列化。
        let tags: Option<String> = match &input["tags"] {
            Value::Array(arr) if !arr.is_empty() => {
                let list: Vec<String> =
                    arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
                serde_json::to_string(&list).ok()
            },
            _ => serde_json::to_string(&vec!["capability-assembly", "auto-saved"]).ok(),
        };

        let template_id = format!("auto_{}", uuid::Uuid::new_v4());
        let template = axagent_harness::repo_dtos::WorkflowTemplateData {
            id: template_id.clone(),
            name: name.to_string(),
            description: Some(format!(
                "由 SaveAsWorkflow 自动组装的工作流，包含 {} 个能力：{}",
                passports.len(),
                passports.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(" → ")
            )),
            icon,
            tags,
            version: 1,
            is_preset: false,
            is_editable: true,
            is_public: false,
            trigger_config: None,
            nodes: nodes_json,
            edges: edges_json,
            input_schema: None,
            output_schema: None,
            variables: None,
            error_config: None,
            cluster_id: None,
            route_path: None,
        };

        // 5. 写入。护照在 move 之前派生（capability_id 与 template.id 对齐，
        // create_workflow_template 返回的 saved_id 即 template.id）。
        let passport = template.to_capability_passport();
        let repo = axagent_harness::repositories::workflow_template_repository();
        let saved_id = repo.create_workflow_template(template).await.map_err(|e| ToolError {
            message: format!("持久化工作流模板失败: {e}"),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: "SAVE_AS_WORKFLOW_PERSIST_FAILED".to_string(),
        })?;

        // 6. 回灌能力索引。
        // 不补这一步，固化的模板在本会话内不可路由——能力护照只在
        // register_all_capabilities 启动时全量重建一次，新模板要等重启才进候选集。
        // 索引失败不影响已落库的模板（重启后会被重建），故仅 warn 不阻断。
        if let Err(e) = indexer.index_passport(&passport).await {
            tracing::warn!(
                target: "axagent.capability.index",
                capability_id = %passport.capability_id,
                error = %e,
                "SaveAsWorkflow 模板已落库但能力索引失败（重启后自动重建）"
            );
        }

        Ok(ToolResult {
            content: format!(
                "✅ 已保存工作流模板 '{}'（ID: {}），包含 {} 个节点、{} 条边。\n\
                 来源能力：{}",
                name,
                saved_id,
                result.nodes.len(),
                result.edges.len(),
                passports.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" → ")
            ),
            is_error: false,
            truncated: false,
            metadata: Some(json!({
                "templateId": saved_id,
                "nodeCount": result.nodes.len(),
                "edgeCount": result.edges.len(),
                "capabilityCount": passports.len(),
                "source": "SaveAsWorkflow",
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}
