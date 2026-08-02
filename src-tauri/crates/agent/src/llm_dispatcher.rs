// SPDX-License-Identifier: AGPL-3.0-only

//! LlmDispatcher — 基于 LLM Function Calling 的群聊智能路由实现。
//!
//! ## 工作流程
//!
//! 1. `dispatch_stream` 接收 fleet_id + 用户消息 + 历史
//! 2. 从 `FleetRepository` 加载该 fleet 的所有成员
//! 3. 构造 system prompt（含成员列表与路由规则）+ user prompt（含用户消息+历史摘要）
//! 4. 调用 `FleetIntentLlm::route()` 让 LLM 选择最合适的 agent
//! 5. 解析 LLM 返回的 JSON，得到 agent_slug
//! 6. 产生 `DispatchEvent::Routing` 事件，由上层 commands 调用 `SessionManager` 执行
//!
//! ## 兜底策略
//!
//! - LLM 调用失败 / JSON 解析失败 / slug 不在成员列表中
//!   → 兜底为第一个 Idle 状态成员（保证不阻塞用户）
//! - 无可用成员 → 返回 `DispatchEvent::Error`
//!
//! ## 直接 DM (`direct_message_stream`)
//!
//! 绕过 LLM 路由，直接产生 `Routing` 事件指向指定 slug 的成员。
//! 用于前端用户主动点击某个 agent 与之对话的场景。
//!
//! ## 设计要点
//!
//! - **不执行 agent**：本 dispatcher 只产生 Routing 决策，真正执行由上层 commands
//!   调用 `SessionManager::run_turn_with_tools` 完成。这避免了 dispatcher 持有
//!   重型 runtime host，保持轻量。
//! - **LLM 能力注入**：通过 `FleetIntentLlm` trait 注入，agent crate 不直接依赖
//!   providers crate（遵守 consumer crate 铁律）。
//! - **成员状态过滤**：路由时仅考虑 Idle/Busy 状态成员，过滤 Paused/Error/Offline。

use async_trait::async_trait;
use axagent_harness::fleet::{
    DispatchChatMessage, DispatchEvent, FleetIntentLlm, FleetMemberStatus, FleetRepository,
    IntentDispatcher,
};
use std::sync::Arc;

/// 基于 LLM 的意图分发器
pub struct LlmDispatcher {
    fleet_repo: Arc<dyn FleetRepository>,
    intent_llm: Arc<dyn FleetIntentLlm>,
}

impl LlmDispatcher {
    pub fn new(fleet_repo: Arc<dyn FleetRepository>, intent_llm: Arc<dyn FleetIntentLlm>) -> Self {
        Self { fleet_repo, intent_llm }
    }
}

#[async_trait]
impl IntentDispatcher for LlmDispatcher {
    async fn dispatch_stream(
        &self,
        fleet_id: &str,
        user_message: &str,
        history: Vec<DispatchChatMessage>,
    ) -> Result<Vec<DispatchEvent>, String> {
        let mut events = Vec::new();

        // 1. 加载 fleet 成员
        let members = self
            .fleet_repo
            .list_members(fleet_id)
            .await
            .map_err(|e| format!("加载成员失败: {e}"))?;

        if members.is_empty() {
            events.push(DispatchEvent::Error { message: "舰队无可用成员".to_string() });
            return Ok(events);
        }

        // 2. 过滤可路由成员（Idle / Busy）
        let routable: Vec<_> = members
            .iter()
            .filter(|m| matches!(m.status, FleetMemberStatus::Idle | FleetMemberStatus::Busy))
            .collect();
        if routable.is_empty() {
            events.push(DispatchEvent::Error {
                message: "舰队所有成员均不可用（已暂停/错误/离线）".to_string(),
            });
            return Ok(events);
        }

        // 3. 构造 LLM prompt
        let system_prompt = build_system_prompt(&routable);
        let user_prompt = build_user_prompt(user_message, &history);

        // 4. 调用 LLM 路由
        let llm_response =
            self.intent_llm.route(&system_prompt, &user_prompt).await.unwrap_or_default();

        // 5. 解析 LLM 返回的 JSON
        let target_slug = parse_route_response(&llm_response)
            .and_then(|slug| {
                // 校验 slug 在可路由成员列表中
                routable.iter().find(|m| m.agent_slug == slug).map(|m| m.agent_slug.clone())
            })
            .unwrap_or_else(|| {
                // 兜底：第一个可路由成员
                let fallback = &routable[0];
                tracing::warn!(
                    "LLM 路由响应无法解析或 slug 不匹配，兜底到首个成员: {}",
                    fallback.agent_slug
                );
                fallback.agent_slug.clone()
            });

        // 6. 找到目标成员，产生 Routing 事件
        if let Some(member) = routable.into_iter().find(|m| m.agent_slug == target_slug) {
            events.push(DispatchEvent::Routing {
                agent_slug: member.agent_slug.clone(),
                agent_id: member.agent_id.clone(),
                room_id: member.room_id.clone(),
                task_summary: user_message.to_string(),
            });
        }

        events.push(DispatchEvent::Complete);
        Ok(events)
    }

    async fn direct_message_stream(
        &self,
        fleet_id: &str,
        agent_slug: &str,
        user_message: &str,
        _history: Vec<DispatchChatMessage>,
    ) -> Result<Vec<DispatchEvent>, String> {
        let mut events = Vec::new();

        let members = self
            .fleet_repo
            .list_members(fleet_id)
            .await
            .map_err(|e| format!("加载成员失败: {e}"))?;

        let target = members
            .iter()
            .find(|m| m.agent_slug == agent_slug)
            .ok_or_else(|| format!("未找到 slug 为 '{agent_slug}' 的成员"))?;

        events.push(DispatchEvent::Routing {
            agent_slug: target.agent_slug.clone(),
            agent_id: target.agent_id.clone(),
            room_id: target.room_id.clone(),
            task_summary: user_message.to_string(),
        });
        events.push(DispatchEvent::Complete);
        Ok(events)
    }
}

// ── Prompt 构造 ──────────────────────────────────────────────────────

fn build_system_prompt(members: &[&axagent_harness::fleet::FleetMember]) -> String {
    let member_list: Vec<String> = members
        .iter()
        .map(|m| {
            format!(
                "- slug: \"{}\", 角色: \"{}\", 房间: \"{}\", 状态: {:?}",
                m.agent_slug, m.role, m.room_id, m.status
            )
        })
        .collect();

    format!(
        "你是一个智能调度员,负责将用户消息路由到最合适的 AI agent。\n\n\
         ## 可用成员\n{}\n\n\
         ## 路由规则\n\
         1. 仔细分析用户消息的意图\n\
         2. 根据成员的角色描述选择最合适的一个\n\
         3. 仅返回 JSON,不要任何额外文本\n\n\
         ## 返回格式\n\
         {{\"agent_slug\": \"<成员 slug>\", \"reason\": \"<选择原因,简短>\"}}",
        member_list.join("\n")
    )
}

fn build_user_prompt(user_message: &str, history: &[DispatchChatMessage]) -> String {
    if history.is_empty() {
        return format!("用户消息:\n{user_message}");
    }

    let history_text: Vec<String> = history
        .iter()
        .filter(|h| h.role == "user" || h.role == "assistant")
        .map(|h| {
            let speaker = h.agent_slug.as_deref().unwrap_or("user");
            format!("[{speaker}]: {}", h.content)
        })
        .collect();

    format!("历史对话:\n{}\n\n用户消息:\n{user_message}", history_text.join("\n"))
}

/// 解析 LLM 返回的 JSON,提取 agent_slug
fn parse_route_response(response: &str) -> Option<String> {
    let trimmed = response.trim();
    // 兼容 LLM 可能包裹的 markdown 代码块
    let json_str = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .map(|s| s.trim())
        .unwrap_or(trimmed);

    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let slug = parsed.get("agent_slug")?.as_str()?.to_string();
    if slug.is_empty() { None } else { Some(slug) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_route_response_plain_json() {
        let resp = r#"{"agent_slug": "copywriter", "reason": "写文案"}"#;
        assert_eq!(parse_route_response(resp), Some("copywriter".to_string()));
    }

    #[test]
    fn test_parse_route_response_markdown_wrapped() {
        let resp = "```json\n{\"agent_slug\": \"analyst\"}\n```";
        assert_eq!(parse_route_response(resp), Some("analyst".to_string()));
    }

    #[test]
    fn test_parse_route_response_empty_slug() {
        let resp = r#"{"agent_slug": ""}"#;
        assert_eq!(parse_route_response(resp), None);
    }

    #[test]
    fn test_parse_route_response_invalid_json() {
        assert_eq!(parse_route_response("not json"), None);
    }

    #[test]
    fn test_build_system_prompt_includes_members() {
        let member = axagent_harness::fleet::FleetMember {
            id: "1".to_string(),
            fleet_id: "f1".to_string(),
            agent_id: "a1".to_string(),
            agent_slug: "copywriter".to_string(),
            display_name: "文案".to_string(),
            role: "撰写产品文案".to_string(),
            agent_profile_id: None,
            room_id: "showroom".to_string(),
            status: FleetMemberStatus::Idle,
            joined_at: 0,
            today_tokens: 0,
            total_tokens: 0,
        };
        let members: Vec<_> = vec![&member];
        let prompt = build_system_prompt(&members);
        assert!(prompt.contains("copywriter"));
        assert!(prompt.contains("撰写产品文案"));
        assert!(prompt.contains("agent_slug"));
    }

    #[test]
    fn test_build_user_prompt_with_history() {
        let history = vec![
            DispatchChatMessage {
                role: "user".to_string(),
                content: "你好".to_string(),
                agent_slug: None,
            },
            DispatchChatMessage {
                role: "assistant".to_string(),
                content: "需要写文案吗".to_string(),
                agent_slug: Some("copywriter".to_string()),
            },
        ];
        let prompt = build_user_prompt("写产品文案", &history);
        assert!(prompt.contains("[user]: 你好"));
        assert!(prompt.contains("[copywriter]: 需要写文案吗"));
        assert!(prompt.contains("写产品文案"));
    }
}
