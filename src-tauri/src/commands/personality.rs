// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_harness::types::{Message, MessageRole};
use axagent_trajectory::{
    CodeSample, CommentStyle as ExtCommentStyle, DetailLevel, DocumentStyleProfile,
    ExplanationDepth, ExtractedCodePatterns, FormattingPreferences, IndentStyle as ExtIndentStyle,
    MessageSample, NamingConvention, NamingPattern, NamingPatternType, ProfileCommentStyle,
    ProfileIndentStyle, StyleExtractor, StyleVectorizer, Tone, UserProfile,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub is_active: bool,
}

#[agent_command(domain = personality, safety = Safe, call_mode = StateOnly, description = "列出所有人格")]
#[tauri::command]
pub async fn personality_list(_state: State<'_, AppState>) -> Result<Vec<PersonalityInfo>, String> {
    let active =
        axagent_agent::personality::PersonalityManager::get_active().ok().flatten().map(|p| p.name);
    let personalities = axagent_agent::personality::PersonalityManager::list().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(personalities
        .into_iter()
        .map(|name| {
            let is_active = active.as_ref() == Some(&name);
            axagent_agent::personality::PersonalityManager::load(&name)
                .ok()
                .map(|p| PersonalityInfo {
                    name: p.name,
                    version: p.version,
                    description: p.description,
                    is_active,
                })
                .unwrap_or_else(|| PersonalityInfo {
                    name,
                    version: "?".to_string(),
                    description: String::new(),
                    is_active,
                })
        })
        .collect())
}

#[agent_command(domain = personality, safety = Safe, call_mode = StateInput, description = "获取人格详情")]
#[tauri::command]
pub async fn personality_get(
    name: String,
    _state: State<'_, AppState>,
) -> Result<axagent_agent::personality::Personality, String> {
    axagent_agent::personality::PersonalityManager::load(&name).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = personality, safety = Caution, call_mode = StateInput, description = "切换激活人格")]
#[tauri::command]
pub async fn personality_switch(name: String, _state: State<'_, AppState>) -> Result<(), String> {
    axagent_agent::personality::PersonalityManager::set_active(&name).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = personality, safety = Safe, call_mode = StateOnly, description = "获取当前激活人格")]
#[tauri::command]
pub async fn personality_current(
    _state: State<'_, AppState>,
) -> Result<Option<axagent_agent::personality::Personality>, String> {
    axagent_agent::personality::PersonalityManager::get_active().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[derive(Debug, Deserialize)]
pub struct PersonalityCreatePayload {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

#[agent_command(domain = personality, safety = Caution, call_mode = StateInput, description = "创建新人格")]
#[tauri::command]
pub async fn personality_create(
    payload: PersonalityCreatePayload,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let personality = axagent_agent::personality::Personality {
        name: payload.name,
        version: payload.version.unwrap_or_else(|| "1.0.0".to_string()),
        description: payload.description.unwrap_or_default(),
        content: payload.content,
        identity: String::new(),
        user: String::new(),
        created_at: chrono::Utc::now(),
    };
    axagent_agent::personality::PersonalityManager::save(&personality).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[derive(Debug, Deserialize)]
pub struct PersonalityCreateBootstrapPayload {
    pub name: String,
    pub soul: Option<String>,
    pub identity: Option<String>,
    pub user: Option<String>,
}

#[agent_command(domain = personality, safety = Caution, call_mode = StateInput, description = "创建引导人格")]
#[tauri::command]
pub async fn personality_create_bootstrap(
    payload: PersonalityCreateBootstrapPayload,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let personality = axagent_agent::personality::Personality {
        name: payload.name,
        version: "1.0.0".to_string(),
        description: String::new(),
        content: payload.soul.unwrap_or_default(),
        identity: payload.identity.unwrap_or_default(),
        user: payload.user.unwrap_or_default(),
        created_at: chrono::Utc::now(),
    };
    axagent_agent::personality::PersonalityManager::save(&personality).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = personality, safety = Caution, call_mode = StateInput, description = "更新人格身份")]
#[tauri::command]
pub async fn personality_update_identity(
    name: String,
    identity: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    axagent_agent::personality::PersonalityManager::save_identity(&name, &identity).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = personality, safety = Caution, call_mode = StateInput, description = "更新人格用户画像")]
#[tauri::command]
pub async fn personality_update_user(
    name: String,
    user: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    axagent_agent::personality::PersonalityManager::save_user(&name, &user).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = personality, safety = Dangerous, call_mode = StateInput, description = "删除人格")]
#[tauri::command]
pub async fn personality_delete(name: String, _state: State<'_, AppState>) -> Result<(), String> {
    axagent_agent::personality::PersonalityManager::delete(&name).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ---------------------------------------------------------------------------
// P2: Persona 自动学习 — 从对话消息中提取用户风格，回写 USER.md
// ---------------------------------------------------------------------------

/// 自动学习结果摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLearnResult {
    /// 是否成功学习（样本数过少时为 false）
    pub learned: bool,
    /// 人类可读的风格摘要
    pub style_summary: String,
    /// 实际更新过的字段名列表
    pub updated_fields: Vec<String>,
    /// 收集到的代码样本数
    pub code_sample_count: usize,
    /// 收集到的消息样本数
    pub message_sample_count: usize,
    /// 风格置信度（0.0-1.0）
    pub confidence: f32,
    /// 回写到的 Persona 名称
    pub persona_name: String,
}

/// 从对话消息中自动学习用户风格，更新当前激活 Persona 的 USER.md
///
/// 内部流程：
/// 1. 通过 dao 拉取对话消息
/// 2. 从消息中解析 markdown 代码块作为 CodeSample，user 消息作为 MessageSample
/// 3. 调用 trajectory 的 StyleExtractor + StyleVectorizer 提取风格
/// 4. 更新 AppState.user_profile 的 coding_style 与 communication 字段
/// 5. 通过 UserProfile::to_user_md() 回写到当前激活的 Personality 的 USER.md
#[agent_command(domain = personality, safety = Caution, call_mode = StateInput, description = "从对话自动学习人格")]
#[tauri::command]
pub async fn personality_auto_learn_from_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<AutoLearnResult, String> {
    // 1. 拉取对话消息
    let messages = axagent_dao::repo::message::list_messages(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if messages.is_empty() {
        return Err("Conversation has no messages to learn from".to_string());
    }

    // 2. 从消息中提取 CodeSample 和 MessageSample
    let (code_samples, message_samples) = extract_samples_from_messages(&messages);

    let code_sample_count = code_samples.len();
    let message_sample_count = message_samples.len();

    if code_samples.is_empty() && message_samples.is_empty() {
        return Ok(AutoLearnResult {
            learned: false,
            style_summary: "No usable code or message samples found in conversation".to_string(),
            updated_fields: Vec::new(),
            code_sample_count,
            message_sample_count,
            confidence: 0.0,
            persona_name: String::new(),
        });
    }

    // 3. 调用 StyleExtractor + StyleVectorizer
    let extractor = StyleExtractor::new();
    let vectorizer = StyleVectorizer::new();

    let code_patterns = extractor.extract_from_code(&code_samples);
    let doc_style = extractor.extract_from_messages(&message_samples);
    let formatting_prefs = extractor.extract_formatting_preferences(&code_samples);
    let code_style_vector = vectorizer.from_coding_samples(&code_samples);
    let _msg_style_vector = vectorizer.from_messages(&message_samples);

    let confidence = code_style_vector.source_confidence;

    // 4. 更新 AppState.user_profile
    let mut updated_fields = Vec::new();
    {
        let mut profile = state.user_profile.write().await;

        apply_code_patterns_to_profile(
            &mut profile,
            &code_patterns,
            &formatting_prefs,
            confidence,
            &mut updated_fields,
        );
        apply_doc_style_to_profile(&mut profile, &doc_style, confidence, &mut updated_fields);

        profile.update_timestamp();
    }

    // 5. 读取更新后的 profile，生成 USER.md 并回写
    let profile_snapshot = state.user_profile.read().await.clone();
    let user_md = profile_snapshot.to_user_md();

    // 获取当前激活的 Persona 名称
    let active_name = axagent_agent::personality::PersonalityManager::get_active()
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .map(|p| p.name)
        .ok_or_else(|| {
            "No active persona set; activate a persona before auto-learning".to_string()
        })?;

    axagent_agent::personality::PersonalityManager::save_user(&active_name, &user_md).map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )?;

    let style_summary = build_style_summary(
        &code_patterns,
        &doc_style,
        &formatting_prefs,
        code_sample_count,
        message_sample_count,
        confidence,
    );

    Ok(AutoLearnResult {
        learned: true,
        style_summary,
        updated_fields,
        code_sample_count,
        message_sample_count,
        confidence,
        persona_name: active_name,
    })
}

/// 从对话消息中提取 CodeSample（markdown 代码块）和 MessageSample（仅 user 消息）
fn extract_samples_from_messages(messages: &[Message]) -> (Vec<CodeSample>, Vec<MessageSample>) {
    let mut code_samples = Vec::new();
    let mut message_samples = Vec::new();

    for msg in messages {
        // 仅 user 消息进入 MessageSample（assistant 消息代表 AI 风格而非用户）
        if matches!(msg.role, MessageRole::User) {
            let timestamp = timestamp_to_datetime(msg.created_at);
            message_samples.push(MessageSample {
                content: msg.content.clone(),
                role: "user".to_string(),
                timestamp,
            });
        }
        // 所有消息中的代码块都参与代码风格提取（用户输入的代码 + AI 生成并被用户接受的代码）
        for (code, language) in extract_code_blocks(&msg.content) {
            let timestamp = timestamp_to_datetime(msg.created_at);
            code_samples.push(CodeSample { code, language, timestamp });
        }
    }

    (code_samples, message_samples)
}

/// 从 markdown 内容中解析 ```fenced``` 代码块，返回 (代码, 语言) 列表
fn extract_code_blocks(content: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            // rest 可能是语言标识（如 "rust"）或空
            let language = rest.trim().to_string();
            let mut code_lines = Vec::new();
            let mut closed = false;

            for inner in lines.by_ref() {
                if inner.trim_start().starts_with("```") {
                    closed = true;
                    break;
                }
                code_lines.push(inner);
            }

            if closed {
                let code = code_lines.join("\n");
                // 过滤过短的代码块（如单行 inline 标记或纯空行）
                let meaningful_lines = code.lines().filter(|l| !l.trim().is_empty()).count();
                if meaningful_lines >= 2 {
                    let lang = if language.is_empty() {
                        "unknown".to_string()
                    } else {
                        language
                    };
                    blocks.push((code, lang));
                }
            }
        }
    }

    blocks
}

/// 将 Unix 秒时间戳转换为 DateTime<Utc>，失败时回退到当前时间
fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

/// 将代码模式应用到 UserProfile 的 coding_style 字段
fn apply_code_patterns_to_profile(
    profile: &mut UserProfile,
    patterns: &ExtractedCodePatterns,
    formatting: &FormattingPreferences,
    confidence: f32,
    updated_fields: &mut Vec<String>,
) {
    // 命名约定
    let new_naming = pick_naming_convention(&patterns.naming_patterns);
    if profile.coding_style.naming_convention != new_naming {
        profile.coding_style.naming_convention = new_naming.clone();
        updated_fields.push(format!("coding_style.naming_convention={:?}", new_naming));
    }

    // 缩进风格
    let new_indent = convert_indent_style(&formatting.indent_style);
    if profile.coding_style.indentation_style != new_indent {
        profile.coding_style.indentation_style = new_indent.clone();
        updated_fields.push(format!("coding_style.indentation_style={:?}", new_indent));
    }

    // 注释风格
    let new_comment = pick_comment_style(patterns);
    if profile.coding_style.comment_style != new_comment {
        profile.coding_style.comment_style = new_comment.clone();
        updated_fields.push(format!("coding_style.comment_style={:?}", new_comment));
    }

    // 置信度（仅在新值更高时覆盖，避免学习样本少时把已有置信度拉低）
    if confidence > profile.coding_style.confidence {
        profile.coding_style.confidence = confidence;
        updated_fields.push(format!("coding_style.confidence={:.2}", confidence));
    }
}

/// 从 NamingPattern 列表中选出出现次数最多的命名约定
fn pick_naming_convention(patterns: &[NamingPattern]) -> NamingConvention {
    use NamingPatternType as N;
    let mut snake = 0u32;
    let mut camel = 0u32;
    let mut pascal = 0u32;
    let mut kebab = 0u32;

    for p in patterns {
        match p.pattern_type {
            N::Snake => snake += p.count,
            N::Camel => camel += p.count,
            N::Pascal => pascal += p.count,
            N::Kebab => kebab += p.count,
            _ => {},
        }
    }

    let max = snake.max(camel).max(pascal).max(kebab);
    if max == 0 {
        return NamingConvention::Mixed;
    }
    if snake == max {
        NamingConvention::SnakeCase
    } else if camel == max {
        NamingConvention::CamelCase
    } else if pascal == max {
        NamingConvention::PascalCase
    } else {
        NamingConvention::KebabCase
    }
}

/// 将 extractor 的 IndentStyle 转换为 profile 的 IndentationStyle
fn convert_indent_style(style: &ExtIndentStyle) -> ProfileIndentStyle {
    match style {
        ExtIndentStyle::Tabs => ProfileIndentStyle::Tabs,
        ExtIndentStyle::Spaces(size) => {
            if *size <= 2 {
                ProfileIndentStyle::TwoSpaces
            } else {
                ProfileIndentStyle::FourSpaces
            }
        },
    }
}

/// 根据注释模式推断用户的注释风格倾向
fn pick_comment_style(patterns: &ExtractedCodePatterns) -> ProfileCommentStyle {
    if patterns.comment_patterns.is_empty() {
        return ProfileCommentStyle::Moderate;
    }

    // 找到频率最高的注释风格
    let mut best: Option<(ExtCommentStyle, f32)> = None;
    for p in &patterns.comment_patterns {
        match best {
            Some((_, freq)) if p.frequency <= freq => {},
            _ => {
                best = Some((p.style.clone(), p.frequency));
            },
        }
    }

    match best {
        Some((ExtCommentStyle::Documentation, _)) => ProfileCommentStyle::DocBlock,
        Some((_, f)) if f > 0.7 => ProfileCommentStyle::Extensive,
        Some((_, f)) if f < 0.3 => ProfileCommentStyle::Minimal,
        _ => ProfileCommentStyle::Moderate,
    }
}

/// 将文档风格应用到 UserProfile 的 communication 字段
fn apply_doc_style_to_profile(
    profile: &mut UserProfile,
    doc: &DocumentStyleProfile,
    confidence: f32,
    updated_fields: &mut Vec<String>,
) {
    // tone: formality > 0.6 Formal, < 0.4 Casual, else Neutral
    let new_tone = if doc.formality_level > 0.6 {
        Tone::Formal
    } else if doc.formality_level < 0.4 {
        Tone::Casual
    } else {
        Tone::Neutral
    };
    if profile.communication.tone != new_tone {
        profile.communication.tone = new_tone.clone();
        updated_fields.push(format!("communication.tone={:?}", new_tone));
    }

    // detail_level: explanation_detail_level > 0.7 Comprehensive, < 0.3 Minimal, else Moderate
    let new_detail = if doc.explanation_detail_level > 0.7 {
        DetailLevel::Comprehensive
    } else if doc.explanation_detail_level < 0.3 {
        DetailLevel::Minimal
    } else {
        DetailLevel::Moderate
    };
    if profile.communication.detail_level != new_detail {
        profile.communication.detail_level = new_detail.clone();
        updated_fields.push(format!("communication.detail_level={:?}", new_detail));
    }

    // explanation_depth: 与 detail_level 类似但更细分
    let new_depth = if doc.explanation_detail_level > 0.7 {
        ExplanationDepth::Detailed
    } else if doc.explanation_detail_level < 0.3 {
        ExplanationDepth::Brief
    } else {
        ExplanationDepth::Standard
    };
    if profile.communication.explanation_depth != new_depth {
        profile.communication.explanation_depth = new_depth.clone();
        updated_fields.push(format!("communication.explanation_depth={:?}", new_depth));
    }

    // 通信置信度
    if confidence > profile.communication.confidence {
        profile.communication.confidence = confidence;
        updated_fields.push(format!("communication.confidence={:.2}", confidence));
    }
}

/// 构建人类可读的风格摘要
fn build_style_summary(
    code_patterns: &ExtractedCodePatterns,
    doc: &DocumentStyleProfile,
    formatting: &FormattingPreferences,
    code_count: usize,
    msg_count: usize,
    confidence: f32,
) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "Learned from {} code samples and {} user messages (confidence: {:.0}%)",
        code_count,
        msg_count,
        confidence * 100.0
    ));

    if !code_patterns.naming_patterns.is_empty() {
        let names: Vec<String> = code_patterns
            .naming_patterns
            .iter()
            .map(|p| format!("{:?} (×{})", p.pattern_type, p.count))
            .collect();
        parts.push(format!("Naming conventions: {}", names.join(", ")));
    }

    if !code_patterns.comment_patterns.is_empty() {
        let comments: Vec<String> = code_patterns
            .comment_patterns
            .iter()
            .map(|p| format!("{:?} ({:.0}%)", p.style, p.frequency * 100.0))
            .collect();
        parts.push(format!("Comment styles: {}", comments.join(", ")));
    }

    parts.push(format!(
        "Indentation: {:?} (size {})",
        formatting.indent_style, formatting.indent_size
    ));

    parts.push(format!(
        "Communication: formality {:.2}, structure {:.2}, tech vocabulary {:.2}",
        doc.formality_level, doc.structure_level, doc.technical_vocabulary_ratio
    ));

    parts.join("\n")
}
