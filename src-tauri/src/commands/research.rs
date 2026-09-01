// SPDX-License-Identifier: AGPL-3.0-only

//! 研究报告生成命令模块
//!
//! 提供两种研究能力：
//! 1. `generate_research_report` — 根据对话历史调用 LLM 生成结构化研究报告
//! 2. `deep_research_topic`  — 多轮 Web 搜索 + 差距分析 + 交叉验证的深度研究

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::research as research_err;
use axagent_agent::deep_research::{DeepResearchConfig, DeepResearcher};
use axagent_agent::ingest_pipeline::IngestPipeline;
use axagent_agent::noop_kit::NoopHtmlCleaner;
use axagent_agent::web_search::WebSearchProvider;
use axagent_agent_macro::agent_command;
use axagent_crypto::decrypt_key;
use axagent_dao::repo::conversation as conversation_repo;
use axagent_dao::repo::message as message_repo;
use axagent_dao::repo::provider::{self as provider_repo, get_active_key};
use axagent_harness::provider::ProviderRequestContext;
use axagent_harness::repositories;
use axagent_harness::resolve_base_url_for_type;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest, MessageRole};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// 报告节结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSection {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// 报告引用
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportCitation {
    pub id: String,
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    #[serde(rename = "sourceTitle")]
    pub source_title: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    pub credibility: f64,
    #[serde(rename = "inReport")]
    pub in_report: bool,
    #[serde(rename = "accessedAt")]
    pub accessed_at: Option<String>,
    #[serde(rename = "usedInSection")]
    pub used_in_section: Option<String>,
}

/// 研究报告完整产出
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchReport {
    pub id: String,
    pub topic: String,
    pub summary: String,
    pub content: String,
    pub sections: Vec<ReportSection>,
    pub citations: Vec<ReportCitation>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// LLM 返回的报告原始结构（用于反序列化）
#[derive(Debug, Clone, Deserialize)]
struct LlmReportRaw {
    topic: String,
    summary: String,
    sections: Option<Vec<RawSection>>,
    citations: Option<Vec<RawCitation>>,
    raw_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSection {
    title: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCitation {
    #[serde(rename = "sourceUrl")]
    source_url: String,
    #[serde(rename = "sourceTitle")]
    source_title: String,
    #[serde(rename = "sourceType")]
    source_type: Option<String>,
    credibility: Option<f64>,
}

/// 从 LLM 返回文本中提取 JSON（容错处理 markdown 代码块）
fn extract_json(text: &str) -> Result<String, String> {
    let trimmed = text.trim();
    // 尝试直接找最外层的大括号
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return Ok(trimmed[start..=end].to_string());
        }
    }
    // 尝试去掉 ```json ... ``` 包装
    if trimmed.starts_with("```") {
        let stripped = trimmed
            .strip_prefix("```")
            .and_then(|s| s.strip_prefix("json").or(Some(s)))
            .unwrap_or(trimmed);
        let stripped = stripped.strip_suffix("```").map(|s| s.trim()).unwrap_or(stripped.trim());
        if let Some(start) = stripped.find('{') {
            if let Some(end) = stripped.rfind('}') {
                return Ok(stripped[start..=end].to_string());
            }
        }
    }
    Err(format!("无法从响应中提取 JSON: {}", &text[..200.min(text.len())]))
}

/// 构建研究提示词里的系统提示
fn research_system_prompt(topic: &Option<String>) -> String {
    let topic_hint = match topic {
        Some(t) if !t.is_empty() => format!("\n\n研究主题提示: {}", t),
        _ => String::new(),
    };
    format!(
        r#"你是一位专业的研究分析师，需要根据对话内容生成一份结构化的研究报告。
{}

请严格按照以下 JSON 格式输出（不要包含其他文字）：
{{
  "topic": "报告主题",
  "summary": "200字以内的摘要",
  "sections": [
    {{ "title": "章节标题", "content": "章节内容" }}
  ],
  "citations": [
    {{
      "sourceUrl": "https://example.com",
      "sourceTitle": "来源名称",
      "sourceType": "web|academic|documentation|news",
      "credibility": 0.8
    }}
  ],
  "raw_content": "完整的 Markdown 格式报告内容"
}}

规则：
1. topic 根据对话主题自动推断
2. sections 至少包含 3 个章节，每个章节内容充实
3. citations 从对话中出现的引用/链接提取，没有则返回空数组
4. credibility 取值范围 0.0-1.0，反映来源可信度
5. raw_content 是完整的 Markdown 格式报告，包含标题、摘要、所有章节和引用列表
6. 只输出 JSON，不要其他文字"#,
        topic_hint
    )
}

#[agent_command(domain = research, safety = Safe, call_mode = StateInput, description = "根据对话历史生成结构化研究报告")]
#[tauri::command]
pub async fn generate_research_report(
    state: State<'_, AppState>,
    conversation_id: String,
    topic: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();

    // 1. 加载对话信息和消息列表
    let conversation = conversation_repo::get_conversation(db, &conversation_id)
        .await
        .map_err(|e| format!("加载对话失败: {}", e))?;

    let messages = message_repo::list_messages(db, &conversation_id)
        .await
        .map_err(|e| format!("加载消息失败: {}", e))?;

    if messages.len() < 2 {
        return Err(ErrorResponse::err(research_err::NOT_ENOUGH_MESSAGES));
    }

    // 2. 获取 LLM 提供商和密钥
    let provider_config = provider_repo::get_provider(db, &conversation.provider_id)
        .await
        .map_err(|e| format!("获取提供商配置失败: {}", e))?;

    let key_row = get_active_key(db, &conversation.provider_id)
        .await
        .map_err(|e| format!("无活跃密钥: {}", e))?;

    let api_key = decrypt_key(&key_row.key_encrypted, state.harness.master_key())
        .map_err(|e| format!("密钥解密失败: {}", e))?;

    // 3. 创建 ProviderAdapter
    let registry_key = axagent_harness::types::provider_model::provider_registry_key(
        &provider_config.provider_type,
    );

    let adapter = state
        .harness
        .provider_registry()
        .get(&registry_key)
        .ok_or_else(|| format!("未找到供应商适配器: {}", registry_key))?;

    let ctx = axagent_providers::ProviderRequestContext {
        api_key,
        key_id: key_row.id,
        provider_id: conversation.provider_id.clone(),
        base_url: Some(resolve_base_url_for_type(
            &provider_config.api_host,
            &provider_config.provider_type,
        )),
        api_path: provider_config.api_path,
        proxy_config: provider_config.proxy_config,
        custom_headers: provider_config
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    // 4. 构建对话文本
    let mut transcript = String::new();
    for msg in &messages {
        let role_str = match msg.role {
            MessageRole::User => "用户",
            MessageRole::Assistant => "助手",
            MessageRole::Tool => "工具",
            MessageRole::System => "系统",
        };
        transcript.push_str(&format!("{}: {}\n\n", role_str, msg.content));
    }

    // 限制长度避免 token 溢出
    if transcript.len() > 30000 {
        transcript = transcript[..30000].to_string();
    }

    let system_prompt = research_system_prompt(&topic);

    let chat_request = ChatRequest {
        model: conversation.model_id.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(format!(
                    "请根据以下对话内容生成研究报告：\n\n{}",
                    transcript
                )),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        stream: false,
        temperature: Some(0.3),
        max_tokens: Some(8192),
        top_p: None,
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

    // 5. 调用 LLM
    let response = adapter
        .chat(&ctx, chat_request.into())
        .await
        .map_err(|e| format!("LLM 调用失败: {}", e))?;

    let json_text = extract_json(&response.content).map_err(|e| format!("JSON 提取失败: {}", e))?;

    let raw: LlmReportRaw = serde_json::from_str(&json_text).map_err(|e| {
        format!("JSON 解析失败: {}. 原始响应: {}", e, &json_text[..200.min(json_text.len())])
    })?;

    // 6. 组装最终报告
    let sections: Vec<ReportSection> = raw
        .sections
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, s)| ReportSection {
            id: format!("section-{}", i + 1),
            title: s.title,
            content: s.content,
        })
        .collect();

    let citations: Vec<ReportCitation> = raw
        .citations
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, c)| ReportCitation {
            id: format!("cite-{}", i + 1),
            source_url: c.source_url,
            source_title: c.source_title,
            source_type: c.source_type.unwrap_or_else(|| "web".to_string()),
            credibility: c.credibility.unwrap_or(0.5),
            in_report: true,
            accessed_at: None,
            used_in_section: None,
        })
        .collect();

    let content = raw.raw_content.unwrap_or_else(|| {
        // 如果没有 raw_content，用 sections 拼接
        let mut md = format!("# {}\n\n## 摘要\n\n{}\n", raw.topic, raw.summary);
        for s in &sections {
            md.push_str(&format!("\n## {}\n\n{}\n", s.title, s.content));
        }
        md
    });

    let report = ResearchReport {
        id: conversation_id.clone(),
        topic: raw.topic,
        summary: raw.summary,
        content,
        sections,
        citations,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let report_json =
        serde_json::to_value(&report).map_err(|e| format!("序列化报告失败: {}", e))?;

    Ok(report_json)
}

/// 多轮深度研究：通过 Web 搜索 + 差距分析 + 交叉验证进行深度调研。
///
/// 对接 agent crate 的 DeepResearcher 引擎，实现：
/// 1. LLM 生成多样化搜索查询
/// 2. 并行 Web 搜索（DuckDuckGo / 自定义 API）
/// 3. 覆盖率分析与知识差距识别
/// 4. 多轮迭代填补差距
/// 5. 来源交叉验证与矛盾检测
#[agent_command(domain = research, safety = Caution, call_mode = StateInput, description = "多轮深度研究，通过Web搜索和差距分析进行调研")]
#[tauri::command]
pub async fn deep_research_topic(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    topic: String,
    provider_id: Option<String>,
    model_id: Option<String>,
    max_rounds: Option<usize>,
) -> Result<serde_json::Value, String> {
    if topic.trim().is_empty() {
        return Err(ErrorResponse::err(research_err::TOPIC_EMPTY));
    }

    // 1. 构建 DeepResearch 配置
    let mut config = DeepResearchConfig::default();
    if let Some(rounds) = max_rounds {
        config.max_rounds = rounds.min(10);
    }

    // 2. 创建 WebSearchProvider（默认使用 DuckDuckGo，无需 API key）
    let html_cleaner = Arc::new(NoopHtmlCleaner);
    let search_provider = Arc::new(WebSearchProvider::new(html_cleaner));

    // 3. 创建 IngestPipeline（从 harness 全局仓库获取 wiki 仓储）
    let wiki_repo = repositories::wiki_repository();
    let wiki_source_repo = repositories::wiki_source_repository();
    let note_repo = repositories::note_repository();
    let ingest_pipeline = Arc::new(IngestPipeline::new(wiki_repo, wiki_source_repo, note_repo));

    // 4. 创建 DeepResearcher
    let researcher = DeepResearcher::new(config, search_provider, ingest_pipeline);

    // 5. 获取 LLM 提供商（如果指定了 provider_id）
    let (llm_adapter, llm_ctx, llm_model) = if let (Some(pid), Some(mid)) =
        (provider_id.as_deref(), model_id.as_deref())
    {
        let db = state.harness.db();

        let provider_config = provider_repo::get_provider(db, pid)
            .await
            .map_err(|e| format!("获取提供商配置失败: {}", e))?;

        let key_row = get_active_key(db, pid).await.map_err(|e| format!("无活跃密钥: {}", e))?;

        let api_key = decrypt_key(&key_row.key_encrypted, state.harness.master_key())
            .map_err(|e| format!("密钥解密失败: {}", e))?;

        let registry_key = axagent_harness::types::provider_model::provider_registry_key(
            &provider_config.provider_type,
        );

        let adapter = state
            .harness
            .provider_registry()
            .get(&registry_key)
            .ok_or_else(|| format!("未找到供应商适配器: {}", registry_key))?;

        let ctx = ProviderRequestContext {
            api_key,
            key_id: key_row.id,
            provider_id: pid.to_string(),
            base_url: Some(resolve_base_url_for_type(
                &provider_config.api_host,
                &provider_config.provider_type,
            )),
            api_path: provider_config.api_path,
            proxy_config: provider_config.proxy_config,
            custom_headers: provider_config
                .custom_headers
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };

        (Some(adapter), Some(ctx), Some(mid.to_string()))
    } else {
        (None, None, None)
    };

    // 6. 执行深度研究
    let result = researcher
        .research("_deep_research", &topic, None, llm_adapter, llm_ctx, llm_model.as_deref())
        .await
        .map_err(|e| format!("深度研究失败: {}", e))?;

    // R2 修复：研究生成的 wiki 页面补入 RAG 索引（此前只落库不入索引）
    if !result.pages_created.is_empty() {
        crate::indexing::spawn_wiki_note_batch_indexing(crate::indexing::WikiBatchIndexingTask {
            app,
            db: state.harness.db().clone(),
            master_key: state.harness.master_key_owned(),
            vector_store: state.vector_store.clone(),
            wiki_id: "_deep_research".to_string(),
            note_ids: result.pages_created.clone(),
            log_label: "deep_research.indexing",
            completion_event: None,
        });
    }

    // 7. 序列化返回
    let json = serde_json::to_value(&result).map_err(|e| format!("序列化结果失败: {}", e))?;

    Ok(json)
}
