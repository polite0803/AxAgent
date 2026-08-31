// SPDX-License-Identifier: AGPL-3.0-only

//! 已加载能力注入器 —— 渐进式披露的 L2「就位层」。
//!
//! # 闭环位置
//!
//! ```text
//! L0    <capability-index>        目录摘要    系统提示静态注入
//! L1    CapabilityView            完整定义    只读查看
//! L1.5  CapabilityLoad            写状态      本轮工具调用
//! L2    本注入器                  内容就位    下一轮 LLM 调用前 ← 这里
//! ```
//!
//! 每轮 LLM 调用前从会话状态读出已加载能力，把完整定义拼成
//! `<loaded-capabilities>` 块注入系统提示。写入（工具调用）与读取（本注入器）
//! 因此落在不同轮次 —— 这正是会话状态作为解耦点的价值。
//!
//! # 多 Agent 隔离
//!
//! 读取前缀含 agent 段（`temp:skill:loaded:{conversation}:{agent}/`），
//! 子 Agent 加载的能力不会出现在主 Agent 的上下文里。

use axagent_harness::CapabilityIndexer;
use axagent_harness::context_contributor::{ContextContributor, ContextRequest};
use axagent_harness::session_state::{
    NS_SKILL_LOADED, SessionStateStore, StateScope, namespace_prefix,
};
use std::sync::Arc;

/// 注入块的最大字符数。
///
/// 已加载的内容是**用户显式请求**的，优先级高于目录，但仍需护栏：
/// 一个引用文件上千行的技能不能把上下文撑爆。超出按条目截断并标注。
const MAX_BLOCK_CHARS: usize = 24_000;

pub struct LoadedCapabilityContributor {
    store: Arc<dyn SessionStateStore>,
    indexer: Arc<dyn CapabilityIndexer>,
}

impl LoadedCapabilityContributor {
    pub fn new(store: Arc<dyn SessionStateStore>, indexer: Arc<dyn CapabilityIndexer>) -> Self {
        Self { store, indexer }
    }
}

#[async_trait::async_trait]
impl ContextContributor for LoadedCapabilityContributor {
    async fn contribute(&self, ctx: &ContextRequest<'_>) -> Option<String> {
        let conversation_id = ctx.conversation_id?;
        if conversation_id.trim().is_empty() {
            return None;
        }

        let prefix =
            namespace_prefix(StateScope::Temp, NS_SKILL_LOADED, conversation_id, ctx.agent_id);

        let entries = match self.store.list_by_prefix(&prefix).await {
            Ok(e) => e,
            Err(e) => {
                // 注入失败不该中断对话：记录后跳过本轮注入
                tracing::warn!("[loaded-capability] 读取会话状态失败，跳过本轮注入: {e}");
                return None;
            },
        };
        if entries.is_empty() {
            return None;
        }

        let mut body = String::from(
            "<loaded-capabilities>\n以下能力已由你在本会话中显式加载（CapabilityLoad），\
             可直接按其定义使用，无需再调 CapabilityView 展开。\n",
        );
        let mut loaded = 0usize;

        for entry in entries {
            // 状态值里记的是加载记录，正文始终从护照现取 —— 护照是唯一权威来源，
            // 不把正文快照进状态，避免能力更新后注入陈旧内容。
            let capability_id = serde_json::from_str::<serde_json::Value>(&entry.value)
                .ok()
                .and_then(|v| v["capabilityId"].as_str().map(str::to_string))
                .unwrap_or_else(|| entry.key.clone());

            let Some(passport) = self.indexer.get_passport(&capability_id).await else {
                tracing::debug!("[loaded-capability] 能力 {capability_id} 已从索引移除，跳过注入");
                continue;
            };
            if !passport.is_user_visible() {
                continue;
            }

            loaded += 1;
            body.push_str(&format!("\n## {}\n", passport.capability_id));
            if !passport.name.is_empty() {
                body.push_str(&format!("名称：{}\n", passport.name));
            }
            body.push_str(&format!("类型：{}\n", passport.kind.as_str()));

            let summary = passport.summary.as_deref().unwrap_or(&passport.description);
            if !summary.is_empty() {
                body.push_str(&format!("说明：{}\n", summary));
            }
            if let Some(schema) = &passport.input_schema {
                let text = serde_json::to_string_pretty(schema).unwrap_or_default();
                if !text.is_empty() {
                    body.push_str(&format!("入参 schema：\n{text}\n"));
                }
            }
            if !passport.preconditions.is_empty() {
                body.push_str("前置条件：\n");
                for p in &passport.preconditions {
                    body.push_str(&format!("- {p}\n"));
                }
            }
            if !passport.steps.is_empty() {
                body.push_str("执行步骤：\n");
                for (i, s) in passport.steps.iter().enumerate() {
                    body.push_str(&format!("{}. {}\n", i + 1, s));
                }
            }
            if !passport.skill_steps.is_empty() {
                body.push_str("技能步骤：\n");
                for s in &passport.skill_steps {
                    let cond = s.condition.as_deref().unwrap_or("-");
                    body.push_str(&format!("- {}（条件：{}）\n", s.capability_id, cond));
                }
            }
            if let Some(tpl) = &passport.template_body {
                body.push_str(&format!("模板正文：\n{tpl}\n"));
            }
            if !passport.attached_snippets.is_empty() {
                body.push_str("附带知识：\n");
                for s in &passport.attached_snippets {
                    body.push_str(&format!("- {}: {}\n", s.key, s.content));
                }
            }

            if body.chars().count() >= MAX_BLOCK_CHARS {
                body.push_str("\n（已加载内容超出注入预算，其余条目本轮不再展开）\n");
                break;
            }
        }

        if loaded == 0 {
            return None;
        }

        body.push_str("</loaded-capabilities>");
        Some(body)
    }

    fn name(&self) -> &str {
        "LoadedCapabilityContributor"
    }
}
