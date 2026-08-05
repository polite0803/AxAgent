// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 工作流自动学习钩子 — 在工作流完成后自动触发反思/进化/RL
//!
//! # 问题背景
//!
//! OPC 行业工作流（9 大行业）执行完成后，反思、进化、RL 经验积累全靠手动触发，
//! 导致自动学习闭环实际断裂。本模块提供 `try_auto_learn_workflow()`，
//! 在工作流完成时自动执行：
//!
//! ```text
//! 工作流完成 → 自动识别行业 → 计算质量分 → 记录 RL 经验
//!     → 触发反思 → 质量分低于阈值则触发进化 → 自我改进 → 策略优化
//! ```
//!
//! # 使用方式
//!
//! 在任何工作流执行完成的地方调用：
//! ```ignore
//! crate::commands::opc_learning_hook::try_auto_learn_workflow(
//!     &template_id, &result, &learning_state,
//! ).await;
//! ```

use agent_macro::agent_command;
use tauri::State;
use tracing::{debug, info, warn};

use crate::AppState;
use crate::commands::opc_industry_actions::load_rl_config;
use crate::state::learning::LearningEngineState;
use axagent_orchestrator::{EvolutionRequest, ReflectionRequest, SelfImprovementRequest};

// ── 行业映射：模板 ID → 行业 ID ──────────────────────────

/// OPC 行业模板前缀列表
///
/// 工作流模板 ID 以 `workflow-` 开头且匹配以下前缀时，
/// 被识别为 OPC 行业工作流并自动触发学习。
const INDUSTRY_TEMPLATE_PREFIXES: &[(&str, &str)] = &[
    ("workflow-finance-invest", "finance-invest"),
    ("workflow-accounting", "accounting"),
    ("workflow-sales-growth", "sales-growth"),
    ("workflow-software-dev", "software-dev"),
    ("workflow-content-media", "content-media"),
    ("workflow-education", "education"),
    ("workflow-ai-research", "ai-research"),
    ("workflow-ecommerce", "ecommerce"),
    ("workflow-industry-consulting", "industry-consulting"),
];

/// 领域工作流前缀（17 个领域包，需要额外映射）
const DOMAIN_TEMPLATE_PREFIXES: &[(&str, &str)] = &[
    ("wf-finance-", "finance-invest"),
    ("wf-accounting-", "accounting"),
    ("wf-sales-", "sales-growth"),
    ("wf-engineering-", "software-dev"),
    ("wf-content-", "content-media"),
    ("wf-education-", "education"),
    ("wf-research-", "ai-research"),
    ("wf-ecommerce-", "ecommerce"),
    ("wf-consulting-", "industry-consulting"),
];

/// 根据工作流模板 ID 识别所属行业
///
/// 返回 `Some(industry_id)` 表示这是 OPC 行业工作流，
/// `None` 表示非 OPC 工作流（如股票分析等）。
pub fn identify_industry_from_template(template_id: &str) -> Option<&'static str> {
    for (prefix, industry_id) in INDUSTRY_TEMPLATE_PREFIXES {
        if template_id.starts_with(prefix) {
            return Some(industry_id);
        }
    }
    for (prefix, industry_id) in DOMAIN_TEMPLATE_PREFIXES {
        if template_id.starts_with(prefix) {
            return Some(industry_id);
        }
    }
    None
}

// ── 质量分自动计算 ──────────────────────────────────────

/// 从工作流执行结果自动计算质量评分 (0.0-1.0)
///
/// 评估维度：
/// - 执行成功率（是否有错误）
/// - 输出完整度（results 字段是否非空）
/// - 步骤完成率
pub fn compute_quality_score(result: &serde_json::Value) -> f64 {
    let mut score = 0.5_f64;

    if let Some(error) = result.get("error") {
        if !error.is_null() && !error.as_str().map(|s| s.is_empty()).unwrap_or(false) {
            score -= 0.3;
        }
    }

    if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
        match status {
            "completed" | "success" => score += 0.3,
            "partial" | "degraded" => score += 0.1,
            "failed" | "error" => score -= 0.3,
            _ => {},
        }
    }

    let has_results =
        result.get("results").and_then(|r| r.as_object()).map(|m| !m.is_empty()).unwrap_or(false);
    if has_results {
        score += 0.1;
    }

    if let Some(steps) = result.get("steps").and_then(|s| s.as_array()) {
        let total = steps.len() as f64;
        let completed = steps
            .iter()
            .filter(|s| s.get("status").and_then(|st| st.as_str()) == Some("completed"))
            .count() as f64;
        if total > 0.0 {
            score += 0.1 * (completed / total);
        }
    }

    score.clamp(0.0, 1.0)
}

// ── 主入口：自动学习钩子 ──────────────────────────────

/// 在工作流完成后自动触发学习管线
///
/// # 执行流程
/// 1. 识别行业（非 OPC 行业工作流则跳过）
/// 2. 计算质量分
/// 3. 记录 RL 经验
/// 4. 触发反思
/// 5. 根据质量分阈值触发进化
/// 6. 执行自我改进
/// 7. RL 策略优化（由 optimize_policy 内部判断阈值）
///
/// 所有步骤均为异步非阻塞，失败仅记录日志，不影响主流程。
pub async fn try_auto_learn_workflow(
    template_id: &str,
    result: &serde_json::Value,
    state: &LearningEngineState,
) {
    let industry_id = match identify_industry_from_template(template_id) {
        Some(id) => id,
        None => {
            debug!("[opc-auto-learn] 模板 {} 非 OPC 行业工作流，跳过自动学习", template_id);
            return;
        },
    };

    let quality_score = compute_quality_score(result);
    let quality_score_100 = quality_score * 100.0;

    info!(
        "[opc-auto-learn] 触发自动学习: industry={}, template={}, quality={:.1}",
        industry_id, template_id, quality_score_100
    );

    // 步骤 1：记录 RL 经验
    if let Err(e) = record_experience(industry_id, template_id, quality_score, result, state).await
    {
        warn!("[opc-auto-learn] RL 经验记录失败: {}", e);
    }

    // 步骤 2：触发反思
    let reflection_result = trigger_reflection(industry_id, template_id, result, state).await;

    // 步骤 3：如果反思质量分低于阈值，触发进化
    if let Ok(reflection) = &reflection_result {
        if reflection.quality_score < 70.0 {
            info!("[opc-auto-learn] 质量分 {:.1} 低于阈值 70，触发进化", reflection.quality_score);
            if let Err(e) = trigger_evolution(
                industry_id,
                template_id,
                &format!("反思质量分较低 ({:.1})，自动触发进化", reflection.quality_score),
                state,
            )
            .await
            {
                warn!("[opc-auto-learn] 进化触发失败: {}", e);
            }
        }
    }

    // 步骤 4：自我改进
    if let Err(e) = trigger_self_improvement(industry_id, template_id, state).await {
        warn!("[opc-auto-learn] 自我改进失败: {}", e);
    }

    // 步骤 5：RL 策略优化（内部判断阈值，经验不足时静默跳过）
    if let Err(e) = trigger_rl_optimization(industry_id, state).await {
        debug!("[opc-auto-learn] RL 策略优化（可能经验不足）: {}", e);
    }

    info!(
        "[opc-auto-learn] 自动学习完成: industry={}, template={}, quality={:.1}",
        industry_id, template_id, quality_score_100
    );
}

// ── 各步骤实现 ──────────────────────────────────────

async fn record_experience(
    industry_id: &str,
    workflow_id: &str,
    quality_score: f64,
    result: &serde_json::Value,
    state: &LearningEngineState,
) -> Result<(), String> {
    let rl_config = load_rl_config(industry_id)
        .ok_or_else(|| format!("行业 {} 的 RL 配置不存在", industry_id))?;

    let engine = &state.industry_learning_engine;
    engine.record_experience(industry_id, workflow_id, quality_score, result, &rl_config).await?;

    debug!("[opc-auto-learn] RL 经验已记录: industry={}, workflow={}", industry_id, workflow_id);
    Ok(())
}

async fn trigger_reflection(
    industry_id: &str,
    workflow_id: &str,
    result: &serde_json::Value,
    state: &LearningEngineState,
) -> Result<axagent_orchestrator::ReflectionResult, String> {
    let registry = state.industry_adapter_registry.lock().await;
    let adapter =
        registry.get(industry_id).ok_or_else(|| format!("行业适配器不存在: {}", industry_id))?;

    let template = adapter.reflection_template().clone();
    drop(registry);

    let engine = &state.industry_learning_engine;
    let request = ReflectionRequest {
        industry_id: industry_id.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_result: result.clone(),
        ..Default::default()
    };

    engine.reflect_on_workflow(&template, &request).await
}

async fn trigger_evolution(
    industry_id: &str,
    workflow_id: &str,
    reason: &str,
    state: &LearningEngineState,
) -> Result<axagent_orchestrator::EvolutionResult, String> {
    let registry = state.industry_adapter_registry.lock().await;
    let adapter =
        registry.get(industry_id).ok_or_else(|| format!("行业适配器不存在: {}", industry_id))?;

    let constraints = adapter.evolution_constraints().clone();
    drop(registry);

    let engine = &state.industry_learning_engine;
    let request = EvolutionRequest {
        industry_id: industry_id.to_string(),
        workflow_id: workflow_id.to_string(),
        reason: reason.to_string(),
    };

    engine.evolve_workflow(&constraints, &request).await
}

async fn trigger_self_improvement(
    industry_id: &str,
    workflow_id: &str,
    state: &LearningEngineState,
) -> Result<axagent_orchestrator::SelfImprovementResult, String> {
    let engine = &state.industry_learning_engine;
    let request = SelfImprovementRequest {
        industry_id: industry_id.to_string(),
        target: format!("workflow_{}_optimization", workflow_id),
    };

    engine.run_self_improvement(&request).await
}

async fn trigger_rl_optimization(
    industry_id: &str,
    state: &LearningEngineState,
) -> Result<(), String> {
    let rl_config = load_rl_config(industry_id)
        .ok_or_else(|| format!("行业 {} 的 RL 配置不存在", industry_id))?;

    let engine = &state.industry_learning_engine;
    engine.optimize_policy(industry_id, &rl_config).await?;
    Ok(())
}

// ── Tauri 命令 ──────────────────────────────────────

/// 手动触发自动学习（可由前端或 Agent 显式调用）
#[agent_command(domain = opc, safety = Safe, call_mode = StateInput, description = "触发 OPC 行业工作流的自动学习管线")]
#[tauri::command]
pub async fn opc_auto_learn_workflow(
    state: State<'_, AppState>,
    template_id: String,
    workflow_result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let industry_id = identify_industry_from_template(&template_id)
        .ok_or_else(|| format!("模板 {} 不是 OPC 行业工作流", template_id))?;

    let quality_score = compute_quality_score(&workflow_result);

    try_auto_learn_workflow(&template_id, &workflow_result, &state.learning).await;

    Ok(serde_json::json!({
        "success": true,
        "industryId": industry_id,
        "templateId": template_id,
        "qualityScore": quality_score,
        "message": "自动学习已触发",
    }))
}
