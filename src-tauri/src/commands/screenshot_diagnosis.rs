// SPDX-License-Identifier: AGPL-3.0-only
//! G6 截图持仓诊断完整闭环 Tauri 命令层
//!
//! 对应前端 IPC 调用，全部走 `#[tauri::command]`，返回 `Result<T, String>`。
//! 业务实现委托给 `axagent_analysis_engine::screenshot_diagnosis`。
//!
//! 命令清单：
//! - `screenshot_diagnosis_create_from_image` —— 上传截图自动诊断（OCR + 结构化 + 风险诊断）
//! - `screenshot_diagnosis_create` —— 直接传入 positions 创建诊断（前端预填时用）
//! - `screenshot_diagnosis_get` —— 按 ID 获取
//! - `screenshot_diagnosis_list_recent` —— 列出最近 N 条
//! - `screenshot_diagnosis_list_by_status` —— 按状态过滤
//! - `screenshot_diagnosis_archive` —— 归档诊断
//! - `screenshot_diagnosis_update` —— 更新诊断字段
//! - `screenshot_diagnosis_to_paper_portfolio` —— 一键转为模拟观察组合
//!
//! ## 截图诊断流程
//!
//! 1. 前端上传截图 base64 → `screenshot_diagnosis_create_from_image`
//! 2. 计算 image_hash (SHA256)，查重；若已有则返回既有诊断
//! 3. 调用 LLM 一次完成 OCR + 结构化解析（输出 positions JSON）
//! 4. 调用 `compute_risk_diagnosis(&positions)` 计算 7 项风险指标
//! 5. 调用 LLM 生成中文 narrative（自然语言诊断说明）
//! 6. 调用 `stock_analysis::create_diagnosis` 持久化
//! 7. 返回完整诊断记录给前端

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_analysis_engine::screenshot_diagnosis::{
    self, CreateScreenshotDiagnosisInput, UpdateScreenshotDiagnosisInput,
};
#[cfg(not(mobile))]
use axagent_analysis_engine::screenshot_diagnosis::{RiskDiagnosis, ScreenshotPosition};
#[cfg(not(mobile))]
use axagent_harness::LlmCallConfig;
#[cfg(not(mobile))]
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest, ContentPart, ImageUrl};
use serde::{Deserialize, Serialize};
#[cfg(not(mobile))]
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::State;

// 复用 provider_ctx 的 VisionContext / build_vision_context
#[cfg(not(mobile))]
use crate::commands::provider_ctx::{VisionContext, build_vision_context};

// ── LLM 调用辅助 ─────────────────────────────────────────────────────────

/// 一次 LLM 调用完成 OCR + 结构化解析（输出 positions JSON 数组）
#[cfg(not(mobile))]
async fn llm_extract_positions(
    vision: &VisionContext,
    model_id: &str,
    image_base64: &str,
) -> Result<(Vec<ScreenshotPosition>, String), String> {
    let system_prompt = r#"你是 A 股持仓截图解析官。任务：从用户上传的券商 App / 同花顺 / 东方财富 / 雪球截图中提取所有持仓信息。

## 交付物

输出 JSON 数组（包裹在 ```json 围栏中），每项包含字段：
- code: 股票代码（A 股 6 位数字 / 港股 5 位数字+.HK / 美股字母）
- name: 股票名称
- qty: 持仓数量（股）
- costPrice: 成本价
- marketValue: 当前市值（元）
- weight: 权重（百分比 0-100，可选，若截图未显示则留 0）

## 禁区

- 不要推测截图未显示的字段，未知字段填 0
- 不要把非持仓信息（如总资产 / 盈亏汇总）当作持仓输出
- A 股代码必须 6 位数字，不要带前缀 SH/SZ
- 港股代码格式 XXXXX.HK

## 自验环节

输出前检查：每条持仓的 code 和 name 是否对应？市值 = 数量 × 现价（若截图显示现价）？"#;

    let user_prompt = "请解析这张持仓截图，输出 JSON 数组。";

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(system_prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Multipart(vec![
                ContentPart {
                    r#type: "text".to_string(),
                    text: Some(user_prompt.to_string()),
                    image_url: None,
                },
                ContentPart {
                    r#type: "image_url".to_string(),
                    text: None,
                    image_url: Some(ImageUrl { url: image_base64.to_string() }),
                },
            ]),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        },
    ];

    let request = ChatRequest {
        model: model_id.to_string(),
        messages,
        temperature: Some(0.1),
        top_p: None,
        max_tokens: Some(8192),
        stream: false,
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
        response_format: None,
    };

    let result = axagent_harness::execute_llm(
        &*vision.adapter,
        &vision.ctx,
        request,
        &LlmCallConfig::default(),
    )
    .await
    .map_err(|e| format!("LLM 调用失败: {}", e))?;

    let raw = &result.response.content;
    let positions = parse_positions_from_llm_output(raw)?;
    Ok((positions, raw.clone()))
}

/// 从 LLM 输出中解析 positions JSON 数组（支持 ```json 围栏 / 裸 JSON）
#[cfg(not(mobile))]
fn parse_positions_from_llm_output(raw: &str) -> Result<Vec<ScreenshotPosition>, String> {
    // 1. 尝试提取 ```json 围栏
    let json_str = if let Some(start) = raw.find("```json") {
        let after = &raw[start + 7..];
        if let Some(end) = after.find("```") {
            &after[..end]
        } else {
            raw
        }
    } else if let Some(start) = raw.find("```") {
        let after = &raw[start + 3..];
        if let Some(end) = after.find("```") {
            &after[..end]
        } else {
            raw
        }
    } else {
        raw
    };

    // 2. 尝试解析为 Vec<ScreenshotPosition>
    let trimmed = json_str.trim();
    match serde_json::from_str::<Vec<ScreenshotPosition>>(trimmed) {
        Ok(v) => Ok(v),
        Err(_) => {
            // 3. 尝试找第一个 [ 到最后一个 ]
            if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
                let arr = &trimmed[start..=end];
                serde_json::from_str::<Vec<ScreenshotPosition>>(arr)
                    .map_err(|e| format!("解析 positions JSON 失败: {} | 原文: {}", e, raw))
            } else {
                Err(format!("LLM 输出未包含 positions JSON 数组 | 原文: {}", raw))
            }
        },
    }
}

/// LLM 生成中文诊断说明（narrative）
#[cfg(not(mobile))]
async fn llm_generate_narrative(
    vision: &VisionContext,
    model_id: &str,
    positions: &[ScreenshotPosition],
    diagnosis: &RiskDiagnosis,
) -> Result<String, String> {
    let system_prompt = r#"你是 A 股持仓风险诊断官。基于提供的持仓列表 + 风险诊断 schema，生成 1-3 段中文诊断说明。

## 输出格式

纯文本（无 markdown），1-3 段，每段 1-3 句话。

## 内容要求

1. 第一段：整体持仓结构评估（持仓数量 / 总市值 / 集中度）
2. 第二段：核心风险点（最高 level 的风险项 + 具体数据）
3. 第三段（可选）：建议动作（减持 / 分散 / 关注）

## 禁区

- 不要重复 schema 中的字段名（concentration_risk 等），用自然语言描述
- 不要给出具体的买卖价格建议
- 不要假设截图未显示的信息"#;

    let positions_json = serde_json::to_string_pretty(positions).unwrap_or_else(|_| "[]".into());
    let diagnosis_json = serde_json::to_string_pretty(diagnosis).unwrap_or_else(|_| "{}".into());
    let user_prompt = format!(
        "## 持仓列表\n{}\n\n## 风险诊断 schema\n{}\n\n请生成中文诊断说明。",
        positions_json, diagnosis_json
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(system_prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(user_prompt),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        },
    ];

    let request = ChatRequest {
        model: model_id.to_string(),
        messages,
        temperature: Some(0.4),
        top_p: None,
        max_tokens: Some(2048),
        stream: false,
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
        response_format: None,
    };

    let result = axagent_harness::execute_llm(
        &*vision.adapter,
        &vision.ctx,
        request,
        &LlmCallConfig::default(),
    )
    .await
    .map_err(|e| format!("LLM 生成 narrative 失败: {}", e))?;

    Ok(result.response.content)
}

// ── Tauri 命令 ───────────────────────────────────────────────────────────

/// 截图诊断完整流程：上传截图 → OCR + 结构化 → 风险诊断 → 持久化
#[cfg(not(mobile))]
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "截图上传自动诊断")]
#[tauri::command]
pub async fn screenshot_diagnosis_create_from_image(
    state: State<'_, AppState>,
    image_base64: String,
    source_app: Option<String>,
    provider_id: String,
    model_id: String,
) -> Result<axagent_entities::screenshot_diagnoses::Model, String> {
    use base64::Engine;

    // 1. 解码 base64 取 bytes，计算 SHA256
    let image_data = if let Some(stripped) = image_base64.strip_prefix("data:image/png;base64,") {
        base64::engine::general_purpose::STANDARD
            .decode(stripped)
            .map_err(|e| format!("base64 解码失败: {}", e))?
    } else if let Some(stripped) = image_base64.strip_prefix("data:image/jpeg;base64,") {
        base64::engine::general_purpose::STANDARD
            .decode(stripped)
            .map_err(|e| format!("base64 解码失败: {}", e))?
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(&image_base64)
            .map_err(|e| format!("base64 解码失败: {}", e))?
    };

    let mut hasher = Sha256::new();
    hasher.update(&image_data);
    let image_hash = hex::encode(hasher.finalize());

    // 2. 查重：若已有同 hash 的诊断，直接返回
    if let Some(existing) =
        screenshot_diagnosis::find_by_image_hash(state.harness.db(), &image_hash).await.map_err(
            |e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            },
        )?
    {
        return Ok(existing);
    }

    // 3. 构造 VisionContext
    let vision =
        build_vision_context(state.harness.db(), state.harness.master_key(), &provider_id).await?;

    // 4. 一次 LLM 调用完成 OCR + 结构化解析
    let (positions, ocr_text) = llm_extract_positions(&vision, &model_id, &image_base64).await?;

    if positions.is_empty() {
        return Err("截图未识别出任何持仓".to_string());
    }

    // 5. 计算风险诊断 schema
    let diagnosis = screenshot_diagnosis::compute_risk_diagnosis(&positions);

    // 6. LLM 生成中文 narrative
    let narrative = llm_generate_narrative(&vision, &model_id, &positions, &diagnosis).await?;

    // 7. 持久化
    let input = CreateScreenshotDiagnosisInput {
        image_hash: Some(image_hash),
        image_path: None,
        image_thumbnail_base64: None,
        image_width: None,
        image_height: None,
        source_app,
        ocr_text: Some(ocr_text),
        positions,
        total_market_value: 0.0,
        diagnosis: Some(diagnosis),
        narrative,
        recommended_actions: vec![],
        source_workflow_execution_id: None,
        provider_id: Some(provider_id),
        model_id: Some(model_id),
        status: "active".to_string(),
    };

    screenshot_diagnosis::create_diagnosis(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 直接传入 positions 创建诊断（前端预填时用，不调 LLM）
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "直接创建截图诊断")]
#[tauri::command]
pub async fn screenshot_diagnosis_create(
    state: State<'_, AppState>,
    input: CreateScreenshotDiagnosisInput,
) -> Result<axagent_entities::screenshot_diagnoses::Model, String> {
    screenshot_diagnosis::create_diagnosis(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 按 ID 获取诊断
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "获取截图诊断详情")]
#[tauri::command]
pub async fn screenshot_diagnosis_get(
    state: State<'_, AppState>,
    diagnosis_id: String,
) -> Result<Option<axagent_entities::screenshot_diagnoses::Model>, String> {
    screenshot_diagnosis::get_diagnosis(state.harness.db(), &diagnosis_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 列出最近 N 条诊断（按 created_at 降序）
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "列出最近截图诊断")]
#[tauri::command]
pub async fn screenshot_diagnosis_list_recent(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<axagent_entities::screenshot_diagnoses::Model>, String> {
    let l = limit.unwrap_or(20);
    screenshot_diagnosis::list_recent_diagnoses(state.harness.db(), l).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 按状态过滤
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "按状态筛选截图诊断")]
#[tauri::command]
pub async fn screenshot_diagnosis_list_by_status(
    state: State<'_, AppState>,
    status: String,
) -> Result<Vec<axagent_entities::screenshot_diagnoses::Model>, String> {
    screenshot_diagnosis::list_diagnoses_by_status(state.harness.db(), &status).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 归档诊断
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "归档截图诊断")]
#[tauri::command]
pub async fn screenshot_diagnosis_archive(
    state: State<'_, AppState>,
    diagnosis_id: String,
) -> Result<axagent_entities::screenshot_diagnoses::Model, String> {
    screenshot_diagnosis::archive_diagnosis(state.harness.db(), &diagnosis_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 更新诊断字段（narrative / recommended_actions / status / error_message）
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "更新截图诊断字段")]
#[tauri::command]
pub async fn screenshot_diagnosis_update(
    state: State<'_, AppState>,
    input: UpdateScreenshotDiagnosisInput,
) -> Result<axagent_entities::screenshot_diagnoses::Model, String> {
    screenshot_diagnosis::update_diagnosis(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 一键转为模拟观察组合（调 paper_portfolio::create_portfolio_from_screenshot_diagnosis）
#[agent_command(domain = "general", safety = Safe, call_mode = StateOnly, description =  "截图诊断转模拟组合")]
#[tauri::command]
pub async fn screenshot_diagnosis_to_paper_portfolio(
    state: State<'_, AppState>,
    diagnosis_id: String,
    name: String,
    source_event: String,
) -> Result<axagent_entities::paper_portfolios::Model, String> {
    axagent_analysis_engine::paper_portfolio::create_portfolio_from_screenshot_diagnosis(
        state.harness.db(),
        &diagnosis_id,
        &name,
        &source_event,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── mobile stub（移动端不支持截图诊断，避免依赖 screen_vision） ──────────

#[cfg(mobile)]
#[agent_command(domain = core, safety = Safe, call_mode = StateInput, description = "从图片创建截图诊断")]
#[tauri::command]
#[allow(dead_code)]
pub async fn screenshot_diagnosis_create_from_image(
    _state: State<'_, AppState>,
    _image_base64: String,
    _source_app: Option<String>,
    _provider_id: String,
    _model_id: String,
) -> Result<axagent_entities::screenshot_diagnoses::Model, String> {
    Err("截图诊断在移动端不可用".to_string())
}

// 显式触发 Arc / Serialize / Deserialize 引用，避免 mobile 模式下未使用警告
#[allow(dead_code)]
fn _touch_unused() {
    let _: Arc<()> = Arc::new(());
    let _ = serde_json::to_string::<()>(&()).unwrap_or_default();
    fn _f<T: Serialize + for<'de> Deserialize<'de>>() {}
    _f::<()>();
}
