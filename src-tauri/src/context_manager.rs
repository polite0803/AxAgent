//! Context manager for conversation history compression.
//!
//! Two modes:
//! - **Sliding window** (compression OFF): trims oldest messages to fit the token budget.
//! - **Compression** (manual or auto): all messages are compressed into an LLM summary,
//!   a `<!-- context-compressed -->` marker is inserted, and subsequent sends use
//!   the summary + only messages after the marker.
//!
//! Token budget management is delegated to `ContextAssembler` from the trajectory crate,
//! which provides fine-grained budget allocation across working memory, retrieved memories,
//! skills, nudges, and session history.

use axagent_core::token_counter;
use axagent_core::types::{ChatContent, ChatMessage};
use axagent_trajectory::{ContextAssembler, TokenBudget};

/// Fraction of context window that triggers auto-compression (70%).
const THRESHOLD_RATIO: f64 = 0.70;

/// Token budget allocation constants (aligned with ContextAssembler design).
/// These define the maximum token allocation for each context component.
pub mod token_budget {
    /// Maximum tokens for the system prompt.
    pub const SYSTEM_PROMPT: usize = 8_000;
    /// Maximum tokens for working memory injection.
    pub const WORKING_MEMORY: usize = 800;
    /// Maximum tokens for retrieved RAG context.
    pub const RETRIEVED_MEMORIES: usize = 10_000;
    /// Maximum tokens for enabled skills.
    pub const SKILLS: usize = 5_000;
    /// Maximum tokens for nudge suggestions.
    pub const NUDGES: usize = 2_000;
    /// Fraction of context window reserved for session history (after other components).
    pub const HISTORY_RATIO: f64 = 0.65;
}

/// Content string for the compression marker message.
pub const COMPRESSION_MARKER: &str = "<!-- context-compressed -->";

/// Estimate the token count of a single `ChatMessage`.
pub fn message_tokens(msg: &ChatMessage) -> usize {
    let text = match &msg.content {
        ChatContent::Text(s) => s.as_str(),
        ChatContent::Multipart(parts) => {
            return token_counter::estimate_tokens(
                &parts
                    .iter()
                    .filter_map(|p| p.text.as_deref())
                    .collect::<Vec<_>>()
                    .join(" "),
            ) + parts.iter().filter(|p| p.image_url.is_some()).count() * 85
                + 4;
        }
    };
    token_counter::estimate_message_tokens(&msg.role, text)
}

/// Check whether the current context exceeds the auto-compression threshold.
///
/// Returns `true` if total tokens (system + history) > model_context_window * 0.70.
///
/// When `model_context_window` is `None` (model has no configured limit), always
/// returns `false` — we never auto-compress without a known budget.
pub fn should_auto_compress(
    system_messages: &[ChatMessage],
    history_messages: &[ChatMessage],
    model_context_window: Option<u32>,
) -> bool {
    let context_window = match model_context_window {
        Some(v) => v as usize,
        None => return false,
    };
    let threshold = (context_window as f64 * THRESHOLD_RATIO) as usize;

    let total: usize = system_messages
        .iter()
        .chain(history_messages.iter())
        .map(message_tokens)
        .try_fold(0usize, |acc, tokens| acc.checked_add(tokens))
        .unwrap_or(usize::MAX);

    total > threshold
}

/// Build the final context for LLM from system messages + optional summary + history.
///
/// If a summary exists, it is prepended as a system message.
/// Sliding window is applied only when `model_context_window` is `Some`.
/// When the model has no configured limit, all history messages are included.
///
/// Uses `ContextAssembler`'s `TokenBudget` for budget-aware history allocation,
/// ensuring consistent token allocation across all context components.
///
/// When `query` is provided, uses relevance-based pruning instead of simple
/// sliding window, keeping messages most pertinent to the current conversation.
pub fn build_context(
    system_messages: &[ChatMessage],
    history_messages: &[ChatMessage],
    existing_summary: Option<&str>,
    model_context_window: Option<u32>,
) -> Vec<ChatMessage> {
    build_context_with_query(
        system_messages,
        history_messages,
        existing_summary,
        model_context_window,
        None,
    )
}

/// Extended version of `build_context` that accepts an optional query for
/// relevance-based history pruning.
pub fn build_context_with_query(
    system_messages: &[ChatMessage],
    history_messages: &[ChatMessage],
    existing_summary: Option<&str>,
    model_context_window: Option<u32>,
    query: Option<&str>,
) -> Vec<ChatMessage> {
    let mut out = system_messages.to_vec();

    // Insert summary as a system message if present
    if let Some(summary_text) = existing_summary {
        out.push(ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(format!(
                "[对话历史摘要 / Conversation History Summary]\n{}",
                summary_text
            )),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    match model_context_window {
        Some(ctx_window) => {
            // Use ContextAssembler's TokenBudget for consistent budget allocation
            let budget = TokenBudget {
                max_tokens: ctx_window,
                ..TokenBudget::default()
            };
            let _assembler = ContextAssembler::with_budget(budget);

            // Calculate history budget: total window minus fixed component budgets
            let fixed_overhead = token_budget::SYSTEM_PROMPT
                + token_budget::WORKING_MEMORY
                + token_budget::RETRIEVED_MEMORIES
                + token_budget::SKILLS
                + token_budget::NUDGES;
            let history_budget = ((ctx_window as usize).saturating_sub(fixed_overhead) as f64
                * token_budget::HISTORY_RATIO) as usize;
            let system_tokens: usize = out
                .iter()
                .map(message_tokens)
                .try_fold(0usize, |acc, t| acc.checked_add(t))
                .unwrap_or(usize::MAX);
            let available = history_budget.saturating_sub(system_tokens);

            // Use relevance pruning when query is available; otherwise sliding window
            let trimmed = match query {
                Some(q) if !q.is_empty() => prune_by_relevance(history_messages, q, available),
                _ => sliding_window(history_messages, available),
            };
            let trimmed_len = trimmed.len();
            out.extend(trimmed);

            // Log budget utilization for diagnostics
            let total_used: usize = out
                .iter()
                .map(message_tokens)
                .try_fold(0usize, |acc, t| acc.checked_add(t))
                .unwrap_or(usize::MAX);
            tracing::debug!(
                "Context built: {} system + {} history = {} total tokens (budget: {})",
                system_tokens,
                trimmed_len,
                total_used,
                ctx_window
            );
        }
        None => {
            // No known context limit — include all history messages
            out.extend(history_messages.iter().cloned());
        }
    }

    out
}

/// Sliding window: keep as many recent messages as fit within `budget` tokens.
/// Always includes at least the last message to prevent the current user input
/// from being silently dropped.
fn sliding_window(history: &[ChatMessage], budget: usize) -> Vec<ChatMessage> {
    if history.is_empty() {
        return Vec::new();
    }

    let mut total = 0usize;
    let mut start_idx = history.len();

    for (i, msg) in history.iter().enumerate().rev() {
        let tokens = message_tokens(msg);
        if total + tokens > budget {
            break;
        }
        total += tokens;
        start_idx = i;
    }

    // Always include at least the last message
    if start_idx == history.len() {
        start_idx = history.len() - 1;
    }

    history[start_idx..].to_vec()
}

// ─── Relevance-based pruning ───

/// Minimum number of most recent messages to always keep regardless of relevance.
const RECENCY_WINDOW: usize = 5;

/// Weight of recency vs relevance in the combined score (0.0 = pure relevance, 1.0 = pure recency).
const RECENCY_WEIGHT: f64 = 0.3;

/// Score a message's relevance to the query using TF-IDF-like word overlap.
/// Returns a score between 0.0 (irrelevant) and 1.0 (highly relevant).
fn relevance_score(message_text: &str, query_terms: &[String]) -> f64 {
    if query_terms.is_empty() || message_text.is_empty() {
        return 0.0;
    }

    let msg_lower = message_text.to_lowercase();
    let mut hits = 0usize;
    let mut total_term_weight = 0usize;

    for term in query_terms {
        let count = msg_lower.matches(term.as_str()).count();
        hits += count;
        // Weight longer terms more heavily (less likely to be noise)
        total_term_weight += term.len().max(1);
    }

    if hits == 0 {
        return 0.0;
    }

    // Normalize: hits / (msg_length * term_count) with bonus for multiple matches
    let msg_len = msg_lower.len().max(1) as f64;
    let density = hits as f64 / msg_len;
    let term_weight =
        total_term_weight as f64 / query_terms.iter().map(|t| t.len().max(1)).sum::<usize>() as f64;
    (density * term_weight).min(1.0)
}

/// Extract meaningful query terms from a user message.
fn extract_query_terms(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    // Split on non-alphanumeric, filter short/stop words
    lower
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|w| w.len() >= 3)
        .filter(|w| !is_stop_word(w))
        .map(|w| w.to_string())
        .collect()
}

/// Check if a word is a common stop word (English + Chinese).
fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "and"
            | "for"
            | "that"
            | "this"
            | "with"
            | "you"
            | "are"
            | "not"
            | "but"
            | "from"
            | "have"
            | "has"
            | "was"
            | "were"
            | "can"
            | "will"
            | "what"
            | "when"
            | "where"
            | "which"
            | "how"
            | "all"
            | "just"
            | "like"
            | "very"
            | "been"
            | "would"
            | "could"
            | "should"
            | "about"
            | "also"
            | "的"
            | "了"
            | "是"
            | "在"
            | "我"
            | "有"
            | "和"
            | "就"
            | "不"
            | "人"
            | "都"
            | "一"
            | "一个"
            | "上"
            | "也"
            | "很"
            | "到"
            | "说"
            | "要"
            | "去"
            | "你"
            | "会"
            | "着"
            | "没有"
            | "看"
            | "好"
            | "自己"
            | "这"
    )
}

/// Recency weight: exponential decay by position (most recent = 1.0).
fn recency_weight(position_from_end: usize, total_messages: usize) -> f64 {
    if total_messages <= RECENCY_WINDOW {
        return 1.0;
    }
    let position = position_from_end as f64;
    let max_pos = (total_messages - RECENCY_WINDOW) as f64;
    if position <= RECENCY_WINDOW as f64 {
        1.0
    } else {
        // Exponential decay after recency window
        let normalized = (position - RECENCY_WINDOW as f64) / max_pos.max(1.0);
        (-3.0 * normalized).exp()
    }
}

/// Prune history messages by relevance to the current query + recency.
///
/// Returns a subset of history that fits within `budget` tokens, prioritizing
/// messages that are both recent AND relevant to the query.
///
/// Strategy:
/// 1. Always keep the most recent `RECENCY_WINDOW` messages (guaranteed recency)
/// 2. Score remaining messages by combined relevance + recency
/// 3. Greedily select highest-scoring messages until budget exhausted
pub fn prune_by_relevance(history: &[ChatMessage], query: &str, budget: usize) -> Vec<ChatMessage> {
    if history.is_empty() || budget == 0 {
        return Vec::new();
    }

    let query_terms = extract_query_terms(query);

    // If no meaningful query terms, fall back to sliding window
    if query_terms.is_empty() {
        return sliding_window(history, budget);
    }

    let n = history.len();

    // Always include the last RECENCY_WINDOW messages
    let recency_start = if n <= RECENCY_WINDOW {
        0
    } else {
        n - RECENCY_WINDOW
    };

    let mut selected: Vec<bool> = vec![false; n];
    let mut used_tokens = 0usize;

    // Mark recency window as always selected
    for i in recency_start..n {
        selected[i] = true;
        used_tokens += message_tokens(&history[i]);
    }

    // If recency window alone exceeds budget, trim from oldest within window
    if used_tokens > budget {
        let mut trimmed: Vec<ChatMessage> = Vec::new();
        let mut tokens = 0usize;
        for i in (0..n).rev() {
            if !selected[i] {
                continue;
            }
            let t = message_tokens(&history[i]);
            if tokens + t > budget {
                break;
            }
            tokens += t;
            trimmed.push(history[i].clone());
        }
        trimmed.reverse();
        return trimmed;
    }

    // Score remaining messages for relevance + recency
    let mut scored: Vec<(usize, f64)> = Vec::new();

    for i in 0..recency_start {
        let content_text = match &history[i].content {
            ChatContent::Text(s) => s.as_str(),
            ChatContent::Multipart(_) => "",
        };
        let rel = if content_text.is_empty() {
            0.0
        } else {
            relevance_score(content_text.trim(), &query_terms)
        };
        let rec = recency_weight(n - 1 - i, n);
        let combined = rel * (1.0 - RECENCY_WEIGHT) + rec * RECENCY_WEIGHT;

        if combined > 0.01 {
            scored.push((i, combined));
        }
    }

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Greedily fill budget
    for (idx, _score) in scored {
        let t = message_tokens(&history[idx]);
        if used_tokens + t > budget {
            break;
        }
        used_tokens += t;
        selected[idx] = true;
    }

    // Collect selected messages in order
    let result: Vec<ChatMessage> = history
        .iter()
        .enumerate()
        .filter(|(i, _)| selected[*i])
        .map(|(_, msg)| msg.clone())
        .collect();

    tracing::debug!(
        "Relevance pruning: {}/{} messages selected, {} tokens (budget: {})",
        result.len(),
        n,
        used_tokens,
        budget
    );

    result
}

/// Messages that need to be summarized (passed to LLM).
pub struct SummarizationRequest {
    /// Existing summary to merge with, if any.
    pub existing_summary: Option<String>,
    /// Messages to incorporate into the summary.
    pub messages_to_compress: Vec<ChatMessage>,
}

/// Build the LLM prompt for generating a conversation summary.
pub fn build_summary_prompt(request: &SummarizationRequest) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    let instruction = if request.existing_summary.is_some() {
        "你是一个对话摘要助手。请将以下新增对话内容合并到已有摘要中。\n\n\
         要求：\n\
         1. 保留所有用户明确表达的需求、偏好和决策\n\
         2. 保留关键技术细节（代码片段、配置、错误信息等）\n\
         3. 保留待办事项和未解决的问题\n\
         4. 用简洁的要点形式组织\n\
         5. 如果有冲突信息，以最新的为准\n\
         6. 保持摘要简洁，不超过 500 字"
    } else {
        "你是一个对话摘要助手。请将以下对话历史压缩为简洁摘要。\n\n\
         要求：\n\
         1. 保留所有用户明确表达的需求、偏好和决策\n\
         2. 保留关键技术细节（代码片段、配置、错误信息等）\n\
         3. 保留待办事项和未解决的问题\n\
         4. 用简洁的要点形式组织\n\
         5. 保持摘要简洁，不超过 500 字"
    };

    messages.push(ChatMessage {
        role: "system".to_string(),
        content: ChatContent::Text(instruction.to_string()),
        tool_calls: None,
        tool_call_id: None,
    });

    if let Some(ref summary) = request.existing_summary {
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(format!("已有摘要：\n{}", summary)),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    let conversation_text: Vec<String> = request
        .messages_to_compress
        .iter()
        .map(|m| {
            let content_text = match &m.content {
                ChatContent::Text(s) => s.clone(),
                ChatContent::Multipart(parts) => parts
                    .iter()
                    .filter_map(|p| p.text.as_deref())
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            let truncated = if content_text.len() > 2000 {
                format!("{}...[已截断]", &content_text[..2000])
            } else {
                content_text
            };
            format!("{}: {}", m.role, truncated)
        })
        .collect();

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: ChatContent::Text(format!(
            "{}对话内容：\n{}",
            if request.existing_summary.is_some() {
                "新增"
            } else {
                ""
            },
            conversation_text.join("\n")
        )),
        tool_calls: None,
        tool_call_id: None,
    });

    messages
}

/// Build summary prompt with a custom system instruction (from settings).
pub fn build_summary_prompt_with_custom(
    request: &SummarizationRequest,
    custom_prompt: &str,
) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    messages.push(ChatMessage {
        role: "system".to_string(),
        content: ChatContent::Text(custom_prompt.to_string()),
        tool_calls: None,
        tool_call_id: None,
    });

    if let Some(ref summary) = request.existing_summary {
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(format!("已有摘要：\n{}", summary)),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    let conversation_text: Vec<String> = request
        .messages_to_compress
        .iter()
        .map(|m| {
            let content_text = match &m.content {
                ChatContent::Text(s) => s.clone(),
                ChatContent::Multipart(parts) => parts
                    .iter()
                    .filter_map(|p| p.text.as_deref())
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            let truncated = if content_text.len() > 2000 {
                format!("{}...[已截断]", &content_text[..2000])
            } else {
                content_text
            };
            format!("{}: {}", m.role, truncated)
        })
        .collect();

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: ChatContent::Text(format!(
            "{}对话内容：\n{}",
            if request.existing_summary.is_some() {
                "新增"
            } else {
                ""
            },
            conversation_text.join("\n")
        )),
        tool_calls: None,
        tool_call_id: None,
    });

    messages
}
