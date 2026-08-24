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

use axagent_agent_macro::agent_command;
use tauri::State;
use tracing::{debug, info, warn};

use crate::AppState;
use crate::commands::opc_industry_actions::load_rl_config;
use crate::state::learning::LearningEngineState;
use axagent_orchestrator::{EvolutionRequest, ReflectionRequest, SelfImprovementRequest};

// ── 行业映射：模板 ID → 行业 ID ──────────────────────────

/// 领域工作流前缀（17 个领域包，需额外映射；行业包走动态扫描，见下）
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
/// v1.1 行业独立版：**动态扫描行业包目录**（`config/opc/industries/{dir}/`，
/// 模板前缀约定 `workflow-{dir 下划线转连字符}`），新增行业无需改代码
/// （消灭 M3 硬编码前缀表）；领域包前缀仍走静态表兼容。
///
/// 返回 `Some(industry_id)` 表示这是 OPC 行业工作流，
/// `None` 表示非 OPC 工作流（如股票分析等）。
pub fn identify_industry_from_template(template_id: &str) -> Option<String> {
    // 行业包动态注册：扫描 `config/opc/industries/*/` 目录
    let base = crate::commands::opc_workflows::resolve_industries_dir(None);
    if let Ok(rd) = std::fs::read_dir(&base) {
        for entry in rd.filter_map(Result::ok) {
            if !entry.path().is_dir() {
                continue;
            }
            let dir_name = entry.file_name().to_string_lossy().to_string();
            let industry_id = dir_name.replace('_', "-");
            if template_id.starts_with(&format!("workflow-{industry_id}")) {
                return Some(industry_id);
            }
        }
    }
    // 领域包静态表（兼容）
    for (prefix, industry_id) in DOMAIN_TEMPLATE_PREFIXES {
        if template_id.starts_with(prefix) {
            return Some((*industry_id).to_string());
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
///
/// `app_dir`：用户数据目录（生产环境），用于解析学习配置文件。
pub async fn try_auto_learn_workflow(
    template_id: &str,
    result: &serde_json::Value,
    state: &LearningEngineState,
    app_dir: Option<&std::path::Path>,
) {
    let industry_id = match identify_industry_from_template(template_id) {
        Some(id) => id,
        None => {
            debug!("[opc-auto-learn] 模板 {} 非 OPC 行业工作流，跳过自动学习", template_id);
            return;
        },
    };

    // P1-4：尊重行业 YAML 的 reflection/evolution/self_improvement/rl 开关，
    // 关闭的环节不再触发（此前无视开关全部执行）。
    let config =
        crate::commands::opc_industry_actions::get_industry_learning_config(&industry_id, app_dir);
    let Some(config) = config else {
        debug!("[opc-auto-learn] 行业 {} 学习配置缺失，跳过自动学习", industry_id);
        return;
    };
    let rl_enabled = config.reinforcement_learning_enabled;

    let quality_score = compute_quality_score(result);
    let quality_score_100 = quality_score * 100.0;

    info!(
        "[opc-auto-learn] 触发自动学习: industry={}, template={}, quality={:.1}, rl={}, reflect={}, evolve={}, self_improve={}",
        industry_id,
        template_id,
        quality_score_100,
        rl_enabled,
        config.reflection_enabled,
        config.evolution_enabled,
        config.self_improvement_enabled
    );

    // 步骤 1：记录 RL 经验（尊重 rl 开关）
    if rl_enabled {
        if let Err(e) =
            record_experience(&industry_id, template_id, quality_score, result, state, app_dir)
                .await
        {
            warn!("[opc-auto-learn] RL 经验记录失败: {}", e);
        }
    }

    // 步骤 2：触发反思（尊重 reflection 开关）
    let mut reflection_result = None;
    if config.reflection_enabled {
        reflection_result =
            Some(trigger_reflection(&industry_id, template_id, result, state).await);
    }

    // 步骤 3：如果反思质量分低于阈值，触发进化（尊重 evolution 开关）
    if let Some(Ok(reflection)) = &reflection_result {
        if reflection.quality_score < 70.0 && config.evolution_enabled {
            info!("[opc-auto-learn] 质量分 {:.1} 低于阈值 70，触发进化", reflection.quality_score);
            if let Err(e) = trigger_evolution(
                &industry_id,
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

    // 步骤 4：自我改进（尊重 self_improvement 开关）
    if config.self_improvement_enabled {
        if let Err(e) = trigger_self_improvement(&industry_id, template_id, state).await {
            warn!("[opc-auto-learn] 自我改进失败: {}", e);
        }
    }

    // 步骤 5：RL 策略优化（尊重 rl 开关；经验不足时静默跳过）
    if rl_enabled {
        if let Err(e) = trigger_rl_optimization(&industry_id, state, app_dir).await {
            debug!("[opc-auto-learn] RL 策略优化（可能经验不足）: {}", e);
        }
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
    app_dir: Option<&std::path::Path>,
) -> Result<(), String> {
    let rl_config = load_rl_config(industry_id, app_dir)
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
    _workflow_id: &str,
    state: &LearningEngineState,
) -> Result<axagent_orchestrator::SelfImprovementResult, String> {
    let engine = &state.industry_learning_engine;
    let request = SelfImprovementRequest {
        industry_id: industry_id.to_string(),
        // P4-4 修复：target 由畸形 `workflow_{workflow_id}_optimization`
        // （会拼出 workflow_workflow-xxx_optimization）改为按行业寻址
        target: format!("industry_{}_optimization", industry_id),
    };

    engine.run_self_improvement(&request).await
}

async fn trigger_rl_optimization(
    industry_id: &str,
    state: &LearningEngineState,
    app_dir: Option<&std::path::Path>,
) -> Result<(), String> {
    let rl_config = load_rl_config(industry_id, app_dir)
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

    try_auto_learn_workflow(
        &template_id,
        &workflow_result,
        &state.learning,
        Some(&state.app_data_dir),
    )
    .await;

    Ok(serde_json::json!({
        "success": true,
        "industryId": industry_id,
        "templateId": template_id,
        "qualityScore": quality_score,
        "message": "自动学习已触发",
    }))
}
