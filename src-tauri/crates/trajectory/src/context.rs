//! Context assembly and token budget management module
//!
//! Replaces TypeScript `ContextAssembler.ts` with Rust implementation.
//! Provides context assembly with token budget management for LLM prompts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    #[serde(rename = "maxTokens")]
    pub max_tokens: u32,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: u32,
    #[serde(rename = "workingMemory")]
    pub working_memory: u32,
    #[serde(rename = "sessionHistory")]
    pub session_history: u32,
    #[serde(rename = "retrievedMemories")]
    pub retrieved_memories: u32,
    pub skills: u32,
    pub nudges: u32,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_tokens: 200000,
            system_prompt: 8000,
            working_memory: 800,
            session_history: 150000,
            retrieved_memories: 10000,
            skills: 5000,
            nudges: 2000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextBlockType {
    System,
    WorkingMemory,
    Retrieved,
    Skill,
    Message,
    Nudge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    #[serde(rename = "type")]
    pub block_type: ContextBlockType,
    pub content: String,
    pub tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    pub blocks: Vec<ContextBlock>,
    pub metadata: ContextMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub budget: TokenBudget,
    #[serde(rename = "usedTokens")]
    pub used_tokens: u32,
    #[serde(rename = "compressionApplied")]
    pub compression_applied: bool,
    #[serde(rename = "nudgesApplied")]
    pub nudges_applied: u32,
}

fn estimate_token_count(text: &str) -> u32 {
    text.len().div_ceil(4) as u32
}

pub struct ContextAssembler {
    budget: TokenBudget,
}

impl Default for ContextAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextAssembler {
    pub fn new() -> Self {
        Self {
            budget: TokenBudget::default(),
        }
    }

    pub fn with_budget(budget: TokenBudget) -> Self {
        Self { budget }
    }

    pub fn update_budget(&mut self, budget: TokenBudget) {
        self.budget = budget;
    }

    pub fn get_budget(&self) -> &TokenBudget {
        &self.budget
    }

    pub fn estimate_tokens(&self, text: &str) -> u32 {
        estimate_token_count(text)
    }

    pub fn truncate_to_limit(&self, text: &str, limit: u32) -> String {
        let tokens = estimate_token_count(text);
        if tokens <= limit {
            return text.to_string();
        }

        let ratio = limit as f64 / tokens as f64;
        let target_len = (text.len() as f64 * ratio) as usize;
        let chars_to_keep = target_len.saturating_sub(20);

        if chars_to_keep >= text.len() {
            text.to_string()
        } else {
            format!("{}...", &text[..chars_to_keep])
        }
    }

    pub fn assemble_blocks(
        &self,
        working_memory_text: &str,
        retrieved_memories: Vec<(String, String)>,
        skills_text: Vec<String>,
        nudges_text: Vec<String>,
        recent_messages: Vec<(String, String)>,
    ) -> AssembledContext {
        let mut blocks: Vec<ContextBlock> = Vec::new();
        let mut total_tokens: u32 = 0;
        let mut nudges_applied: u32 = 0;
        let mut compression_applied = false;

        let wm_tokens = estimate_token_count(working_memory_text);
        if wm_tokens <= self.budget.working_memory {
            blocks.push(ContextBlock {
                block_type: ContextBlockType::WorkingMemory,
                content: working_memory_text.to_string(),
                tokens: wm_tokens,
            });
            total_tokens += wm_tokens;
        } else {
            compression_applied = true;
            let truncated = self.truncate_to_limit(working_memory_text, self.budget.working_memory);
            let truncated_tokens = self.estimate_tokens(&truncated);
            blocks.push(ContextBlock {
                block_type: ContextBlockType::WorkingMemory,
                content: truncated,
                tokens: truncated_tokens,
            });
            total_tokens += truncated_tokens;
        }

        let mut retrieved_tokens: u32 = 0;
        for (session_title, message_content) in retrieved_memories.iter().take(5) {
            let mem_text = format!("[相关记忆] {}: {}", session_title, message_content);
            let tokens = estimate_token_count(&mem_text);

            if total_tokens + retrieved_tokens + tokens <= self.budget.retrieved_memories {
                retrieved_tokens += tokens;
                blocks.push(ContextBlock {
                    block_type: ContextBlockType::Retrieved,
                    content: mem_text,
                    tokens,
                });
            }
        }
        total_tokens += retrieved_tokens;

        let mut skills_tokens: u32 = 0;
        for skill_text in skills_text.iter().take(10) {
            let tokens = estimate_token_count(skill_text);
            if total_tokens + skills_tokens + tokens <= self.budget.skills {
                skills_tokens += tokens;
                blocks.push(ContextBlock {
                    block_type: ContextBlockType::Skill,
                    content: skill_text.clone(),
                    tokens,
                });
            }
        }
        total_tokens += skills_tokens;

        let mut nudges_tokens: u32 = 0;
        for nudge_text in nudges_text.iter().take(5) {
            let tokens = estimate_token_count(nudge_text);
            if total_tokens + nudges_tokens + tokens <= self.budget.nudges {
                nudges_tokens += tokens;
                nudges_applied += 1;
                blocks.push(ContextBlock {
                    block_type: ContextBlockType::Nudge,
                    content: nudge_text.clone(),
                    tokens,
                });
            }
        }
        total_tokens += nudges_tokens;

        let mut session_tokens: u32 = 0;
        for (role, content) in recent_messages.iter().rev().take(50) {
            let msg_text = format!("{}: {}", role, content);
            let tokens = estimate_token_count(&msg_text);
            if total_tokens + session_tokens + tokens <= self.budget.session_history {
                session_tokens += tokens;
            }
        }
        total_tokens += session_tokens;

        let system_prompt = format!(
            "System context with {} tokens budget. {} blocks assembled.",
            self.budget.max_tokens,
            blocks.len()
        );

        AssembledContext {
            system_prompt,
            blocks,
            metadata: ContextMetadata {
                budget: self.budget.clone(),
                used_tokens: total_tokens,
                compression_applied,
                nudges_applied,
            },
        }
    }

    pub fn calculate_efficiency(&self, context: &AssembledContext) -> f64 {
        if self.budget.max_tokens == 0 {
            return 0.0;
        }
        context.metadata.used_tokens as f64 / self.budget.max_tokens as f64
    }

    pub fn suggest_optimizations(&self, context: &AssembledContext) -> Vec<String> {
        let mut suggestions = Vec::new();
        let efficiency = self.calculate_efficiency(context);

        if efficiency > 0.9 {
            suggestions.push("Token使用率过高，建议增加压缩比".to_string());
        }

        let retrieved_count = context
            .blocks
            .iter()
            .filter(|b| b.block_type == ContextBlockType::Retrieved)
            .count();
        if retrieved_count == 0 {
            suggestions.push("未使用检索记忆，考虑添加".to_string());
        }

        let nudge_count = context
            .blocks
            .iter()
            .filter(|b| b.block_type == ContextBlockType::Nudge)
            .count();
        if nudge_count == 0 {
            suggestions.push("未应用nudges，考虑添加".to_string());
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let assembler = ContextAssembler::new();
        assert_eq!(assembler.estimate_tokens("hello world"), 3);
        assert_eq!(assembler.estimate_tokens(""), 0);
        assert_eq!(assembler.estimate_tokens("abcd"), 1);
    }

    #[test]
    fn test_truncation() {
        let assembler = ContextAssembler::new();
        let short = "short";
        assert_eq!(assembler.truncate_to_limit(short, 10), short);

        let long = "a".repeat(1000);
        let truncated = assembler.truncate_to_limit(&long, 10);
        assert!(truncated.len() < 1000);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_assembly() {
        let assembler = ContextAssembler::new();
        let context = assembler.assemble_blocks(
            "working memory content",
            vec![("session".to_string(), "message".to_string())],
            vec!["skill1".to_string()],
            vec!["nudge1".to_string()],
            vec![("user".to_string(), "hello".to_string())],
        );

        assert!(!context.blocks.is_empty());
        assert!(context.metadata.used_tokens > 0);
    }
}
