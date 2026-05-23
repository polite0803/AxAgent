//! 上下文 token 占用分解，为前端 ContextClassificationBar 提供数据。

use crate::AppState;
use serde::Serialize;
use tauri::State;

/// 单个分类的 token 占用估算结果。
#[derive(Debug, Clone, Serialize)]
pub struct ContextCategoryBreakdown {
    /// 分类标识: system / messages / knowledge / memories / skills / tools
    pub key: String,
    /// 中文标签
    pub label: String,
    /// 估算 token 数
    pub tokens: usize,
    /// 十六进制颜色
    pub color: String,
}

/// 各分类默认颜色（深色主题适配）。
fn default_color(key: &str) -> &str {
    match key {
        "system" => "#6366f1",
        "messages" => "#22c55e",
        "knowledge" => "#f59e0b",
        "memories" => "#3b82f6",
        "skills" => "#ec4899",
        "tools" => "#06b6d4",
        _ => "#6b7280",
    }
}

/// 各分类默认中文标签。
fn default_label(key: &str) -> &str {
    match key {
        "system" => "系统提示",
        "messages" => "对话消息",
        "knowledge" => "知识库检索",
        "memories" => "记忆注入",
        "skills" => "技能定义",
        "tools" => "工具定义",
        _ => key,
    }
}

/// 统计对话中各类消息的数量及记录的 token 总量。
async fn count_messages_by_role(
    db: &sea_orm::DatabaseConnection,
    conversation_id: &str,
) -> Result<(u64, u64, u64, u64), String> {
    use sea_orm::QueryFilter;
    use sea_orm::entity::prelude::*;

    let rows = axagent_core::entity::messages::Entity::find()
        .filter(axagent_core::entity::messages::Column::ConversationId.eq(conversation_id))
        .filter(axagent_core::entity::messages::Column::IsActive.eq(1))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let user_count = rows.iter().filter(|m| m.role == "user").count() as u64;
    let tool_count = rows.iter().filter(|m| m.role == "tool").count() as u64;
    let total = rows.len() as u64;

    // 在 metadata 中累计 token
    let prompt_total: u64 = rows
        .iter()
        .filter_map(|m| m.prompt_tokens)
        .map(|t| t as u64)
        .sum();
    let completion_total: u64 = rows
        .iter()
        .filter_map(|m| m.completion_tokens)
        .map(|t| t as u64)
        .sum();

    Ok((total, user_count, tool_count, prompt_total + completion_total))
}

#[tauri::command]
pub async fn get_context_breakdown(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ContextCategoryBreakdown>, String> {
    let db = &state.sea_db;

    // ── 1) 获取对话配置 ──
    let conv = axagent_core::repo::conversation::get_conversation(db, &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    // ── 2) 统计消息与 token ──
    let (msg_total, _user_count, tool_count, recorded_tokens) =
        count_messages_by_role(db, &conversation_id).await?;

    // ── 3) 各分类估算 ──
    // 系统提示：从 conversation.system_prompt 估算，上限 token_budget::SYSTEM_PROMPT
    let sys_prompt_len = conv.system_prompt.as_deref().unwrap_or("").chars().count();
    // 约 0.4-0.6 token/char for Chinese, 0.25 for English; 取平均 0.5
    let system_tokens = ((sys_prompt_len as f64 * 0.5) as usize)
        .min(crate::context_manager::token_budget::SYSTEM_PROMPT)
        .max(if sys_prompt_len > 0 { 200 } else { 0 });

    // 对话消息：优先用数据库记录值，否则按消息数估算
    let messages_tokens = if recorded_tokens > 0 {
        recorded_tokens as usize
    } else {
        // 粗略估算：平均每消息 ~200 token
        (msg_total as usize).saturating_mul(200)
    };

    // 知识库检索：取 token_budget 上限 × 已启用知识库数量
    let kb_count = conv.enabled_knowledge_base_ids.len();
    let knowledge_tokens = if kb_count > 0 {
        // 未精确计算时按比例估算
        crate::context_manager::token_budget::RETRIEVED_MEMORIES
            .min(kb_count * 2000)
            .max(if kb_count > 0 { 500 } else { 0 })
    } else {
        0
    };

    // 记忆注入：取 token_budget 上限 × 已启用记忆命名空间数
    let mem_ns_count = conv.enabled_memory_namespace_ids.len();
    let memories_tokens = if mem_ns_count > 0 {
        crate::context_manager::token_budget::WORKING_MEMORY
            .min(mem_ns_count * 400)
            .max(if mem_ns_count > 0 { 100 } else { 0 })
    } else {
        0
    };

    // 技能定义：每个技能 ~800 token (SKILL.md 平均长度)
    let skill_count = conv.enabled_skill_ids.len();
    let skills_tokens = if skill_count > 0 {
        crate::context_manager::token_budget::SKILLS.min(skill_count.saturating_mul(800))
    } else {
        0
    };

    // 工具定义：从 MCP 服务器数量和工具调用消息估算
    let mcp_count = conv.enabled_mcp_server_ids.len();
    let tools_tokens = if mcp_count > 0 || tool_count > 0 {
        // MCP 工具声明 + 内建工具声明
        (mcp_count.saturating_mul(500) + tool_count.saturating_mul(150) as usize)
            .clamp(1, crate::context_manager::token_budget::SKILLS)
    } else {
        0
    };

    // ── 4) 构建返回数组 ──
    let categories: Vec<ContextCategoryBreakdown> = vec![
        ContextCategoryBreakdown {
            key: "system".into(),
            label: default_label("system").into(),
            tokens: system_tokens,
            color: default_color("system").into(),
        },
        ContextCategoryBreakdown {
            key: "messages".into(),
            label: default_label("messages").into(),
            tokens: messages_tokens,
            color: default_color("messages").into(),
        },
        ContextCategoryBreakdown {
            key: "knowledge".into(),
            label: default_label("knowledge").into(),
            tokens: knowledge_tokens,
            color: default_color("knowledge").into(),
        },
        ContextCategoryBreakdown {
            key: "memories".into(),
            label: default_label("memories").into(),
            tokens: memories_tokens,
            color: default_color("memories").into(),
        },
        ContextCategoryBreakdown {
            key: "skills".into(),
            label: default_label("skills").into(),
            tokens: skills_tokens,
            color: default_color("skills").into(),
        },
        ContextCategoryBreakdown {
            key: "tools".into(),
            label: default_label("tools").into(),
            tokens: tools_tokens,
            color: default_color("tools").into(),
        },
    ];

    Ok(categories)
}
