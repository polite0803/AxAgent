// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::VecDeque;

use crate::reasoning_state::ReasoningState;
use crate::thought_chain::ThoughtChain;
use crate::thought_chain::ThoughtStep;

/// 上下文窗口配置
#[derive(Debug, Clone)]
pub struct ContextWindowConfig {
    /// 保留最近 N 步全量内容
    pub recent_count: usize,
    /// 摘要最大 token 数
    pub max_summary_chars: usize,
    /// 超过此步数后开始摘要旧步骤
    pub summarize_older_than: usize,
    /// 相似 observation 去重
    pub deduplicate_similar: bool,
}

impl Default for ContextWindowConfig {
    fn default() -> Self {
        Self {
            recent_count: 5,
            max_summary_chars: 300,
            summarize_older_than: 12,
            deduplicate_similar: true,
        }
    }
}

/// 上下文窗口：为 LLM prompt 提供智能化的历史步骤摘要
///
/// 策略：
/// - 最近 N 步全量保留
/// - 关键步骤（已验证通过 + 有工具调用结果）始终保留
/// - 中间步骤生成摘要
/// - 可选 observation 去重
pub struct ContextWindow {
    /// 最近的步骤（全量保留）
    pub recent_steps: Vec<ThoughtStep>,
    /// 旧步骤的摘要文本
    pub summarized_prefix: Option<String>,
    /// 始终保留的关键步骤
    pub crucial_steps: Vec<ThoughtStep>,
}

impl ContextWindow {
    /// 从 ThoughtChain 构建上下文窗口
    pub fn from_chain(chain: &ThoughtChain, config: &ContextWindowConfig) -> Self {
        let steps = &chain.steps;
        let total = steps.len();

        if total <= config.recent_count {
            return Self {
                recent_steps: steps.to_vec(),
                summarized_prefix: None,
                crucial_steps: Vec::new(),
            };
        }

        let split_point = total.saturating_sub(config.recent_count);
        let (old_steps, recent) = steps.split_at(split_point);

        // 提取关键步骤（已验证通过且有观察结果）
        let crucial: Vec<ThoughtStep> = {
            let qualified: Vec<ThoughtStep> = old_steps
                .iter()
                .filter(|s| {
                    s.is_verified
                        && s.observation.is_some()
                        && matches!(s.state, ReasoningState::Acting | ReasoningState::Observing)
                })
                .cloned()
                .collect();
            if config.deduplicate_similar {
                let mut seen = std::collections::HashSet::new();
                qualified
                    .into_iter()
                    .filter(|s| {
                        let key = s.observation.as_deref().unwrap_or("").to_lowercase();
                        seen.insert(key)
                    })
                    .collect()
            } else {
                qualified
            }
        };

        // 生成旧步骤摘要
        let summary = if old_steps.len() > config.summarize_older_than {
            Some(Self::summarize_steps(
                old_steps,
                config.max_summary_chars,
                config.deduplicate_similar,
            ))
        } else {
            None
        };

        Self {
            recent_steps: recent.to_vec(),
            summarized_prefix: summary,
            crucial_steps: crucial,
        }
    }

    /// 格式化为 LLM prompt 可用的字符串
    pub fn to_prompt_string(&self) -> String {
        let mut parts = Vec::new();

        // 1. 摘要（如果有）
        if let Some(ref summary) = self.summarized_prefix {
            parts.push(format!("早期步骤摘要:\n{}", summary));
        }

        // 2. 关键步骤
        for step in &self.crucial_steps {
            let obs = step.observation.as_deref().unwrap_or("");
            let obs_short = truncate_string(obs, 80);
            parts.push(format!(
                "[{}] {} → 结果: {}",
                step.state.as_str(),
                truncate_string(&step.reasoning, 60),
                obs_short
            ));
        }

        if !self.crucial_steps.is_empty() {
            parts.push(String::new());
        }

        // 3. 最近步骤
        for step in &self.recent_steps {
            let verified = if step.is_verified { " ✓" } else { "" };
            let obs = step.observation.as_deref().unwrap_or("");
            let obs_short = if obs.is_empty() {
                String::new()
            } else {
                format!(" → {}", truncate_string(obs, 80))
            };
            parts.push(format!(
                "[{}]{}{}{}",
                step.state.as_str(),
                verified,
                if !step.reasoning.is_empty() {
                    format!(" {}", truncate_string(&step.reasoning, 80))
                } else {
                    String::new()
                },
                obs_short
            ));
        }

        parts.join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.recent_steps.is_empty() && self.crucial_steps.is_empty()
    }

    fn summarize_steps(steps: &[ThoughtStep], max_chars: usize, deduplicate: bool) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut seen_obs: VecDeque<String> = VecDeque::with_capacity(3);
        let mut char_count = 0;

        for step in steps.iter().rev() {
            let obs = step.observation.as_deref().unwrap_or("");
            let obs_short = truncate_string(obs, 60);

            // 相似 observation 去重
            if deduplicate && !obs_short.is_empty() {
                let dedup_key = obs_short.to_lowercase();
                if seen_obs.contains(&dedup_key) {
                    continue;
                }
                seen_obs.push_back(dedup_key);
                if seen_obs.len() > 3 {
                    seen_obs.pop_front();
                }
            }

            let line =
                format!("[{}] {}", step.state.as_str(), truncate_string(&step.reasoning, 60));
            char_count += line.len();
            if char_count > max_chars {
                lines.push("... (更早的步骤已省略)".to_string());
                break;
            }
            lines.push(line);
        }

        lines.reverse();
        lines.join("\n")
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning_state::ReasoningState;
    use crate::thought_chain::{ThoughtChain, ThoughtStep};

    #[test]
    fn test_small_chain_no_summary() {
        let mut chain = ThoughtChain::new();
        chain.add_step(ThoughtStep::new(ReasoningState::Thinking, "step 1".to_string()));

        let config = ContextWindowConfig::default();
        let window = ContextWindow::from_chain(&chain, &config);
        assert!(window.summarized_prefix.is_none());
        assert!(window.crucial_steps.is_empty());
        assert_eq!(window.recent_steps.len(), 1);
    }

    #[test]
    fn test_large_chain_summarizes_old_steps() {
        let mut chain = ThoughtChain::new();
        // 添加 20 个步骤，前 10 个为已验证步骤
        for i in 0..10 {
            let mut step = ThoughtStep::new(ReasoningState::Observing, format!("old step {}", i));
            step.is_verified = true;
            step.observation = Some(format!("result {}", i));
            chain.add_step(step);
        }
        for i in 0..10 {
            chain
                .add_step(ThoughtStep::new(ReasoningState::Thinking, format!("recent step {}", i)));
        }

        let config = ContextWindowConfig {
            recent_count: 5,
            max_summary_chars: 200,
            summarize_older_than: 3,
            deduplicate_similar: false,
        };

        let window = ContextWindow::from_chain(&chain, &config);
        assert!(window.summarized_prefix.is_some());
        assert!(!window.crucial_steps.is_empty());
        assert_eq!(window.recent_steps.len(), 5);
    }

    #[test]
    fn test_dedup_similar_observations() {
        let mut chain = ThoughtChain::new();
        for i in 0..20 {
            let mut step = ThoughtStep::new(ReasoningState::Acting, format!("step {}", i));
            step.observation = Some("same output every time".to_string());
            step.is_verified = true;
            chain.add_step(step);
        }

        let config = ContextWindowConfig {
            recent_count: 3,
            max_summary_chars: 500,
            summarize_older_than: 3,
            deduplicate_similar: true,
        };

        let window = ContextWindow::from_chain(&chain, &config);
        let prompt = window.to_prompt_string();
        // 相同的 observation 去重后不应出现多次
        let occurrences = prompt.matches("same output every time").count();
        // 应该只在 crucial steps 中出现最多一次
        assert!(occurrences <= 5, "Got {} occurrences", occurrences);
    }

    #[test]
    fn test_empty_chain() {
        let chain = ThoughtChain::new();
        let window = ContextWindow::from_chain(&chain, &ContextWindowConfig::default());
        assert!(window.is_empty());
        assert!(window.to_prompt_string().is_empty());
    }
}
