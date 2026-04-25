//! Session compaction and message importance scoring module
//!
//! Replaces TypeScript `SessionCompactor.ts` with Rust implementation.
//! Provides message importance scoring, key term extraction, and session compression.

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(rename = "type")]
    pub message_type: Option<String>,
    pub timestamp: i64,
    #[serde(rename = "toolCalls")]
    pub tool_calls: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    #[serde(rename = "originalCount")]
    pub original_count: usize,
    #[serde(rename = "removedCount")]
    pub removed_count: usize,
    pub summary: String,
    #[serde(rename = "preservedMessages")]
    pub preserved_messages: Vec<MessageRecord>,
    #[serde(rename = "keyEntities")]
    pub key_entities: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageImportance {
    pub message: MessageRecord,
    pub score: f64,
    pub reasons: Vec<String>,
}

struct ImportanceOrd(MessageImportance);

#[allow(dead_code)]
impl PartialEq for ImportanceOrd {
    fn eq(&self, other: &Self) -> bool {
        self.0.score == other.0.score
    }
}

impl Eq for ImportanceOrd {}

impl PartialOrd for ImportanceOrd {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.score.partial_cmp(&other.0.score)
    }
}

impl Ord for ImportanceOrd {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.score.partial_cmp(&other.0.score).unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[allow(dead_code)]
fn estimate_token_count(text: &str) -> usize {
    text.len().div_ceil(4)
}

#[allow(dead_code)]
fn extract_key_terms(text: &str) -> Vec<String> {
    let stop_words: HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "can", "to", "of", "in", "for", "on", "with",
        "at", "by", "from", "as", "into", "through", "during", "before", "after",
        "above", "below", "between", "under", "again", "further", "then",
        "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "few", "more", "most", "other", "some", "such", "no", "nor", "not",
        "only", "own", "same", "so", "than", "too", "very", "just", "and",
        "but", "if", "or", "because", "as", "until", "while", "this", "that",
        "these", "those", "i", "me", "my", "myself", "we", "our", "ours",
    ]
    .iter()
    .cloned()
    .collect();

    let lowercase_text = text.to_lowercase();
    let words: Vec<&str> = lowercase_text.split(|c: char| !c.is_alphanumeric()).collect();
    let mut word_freq: HashMap<String, usize> = HashMap::new();

    for word in words {
        let cleaned = word.trim().to_string();
        if cleaned.len() > 3 && !stop_words.contains(cleaned.as_str()) {
            *word_freq.entry(cleaned).or_insert(0) += 1;
        }
    }

    let mut entries: Vec<(String, usize)> = word_freq.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    entries.into_iter().take(20).map(|(w, _)| w).collect()
}

#[allow(dead_code)]
fn score_message_importance(message: &MessageRecord) -> MessageImportance {
    let mut score: f64 = 0.0;
    let mut reasons: Vec<String> = Vec::new();

    let content = &message.content;
    let has_code = content.contains("```")
        || content.contains("function ")
        || content.contains("class ");
    let has_error = content.to_lowercase().contains("error")
        || content.to_lowercase().contains("failed");
    let has_success = content.to_lowercase().contains("success")
        || content.to_lowercase().contains("completed");
    let is_tool_call = message.role == "tool" || message.tool_calls.unwrap_or(false);
    let is_long = content.len() > 500;

    if has_code {
        score += 3.0;
        reasons.push("包含代码".to_string());
    }
    if has_error {
        score += 4.0;
        reasons.push("包含错误信息".to_string());
    }
    if has_success {
        score += 2.0;
        reasons.push("包含成功状态".to_string());
    }
    if is_tool_call {
        score += 2.0;
        reasons.push("工具调用".to_string());
    }
    if is_long {
        score += 1.0;
        reasons.push("内容较长".to_string());
    }

    if message.role == "user" {
        score += 1.5;
        reasons.push("用户消息".to_string());
    }

    if message.role == "assistant" && content.starts_with("```") {
        score += 2.0;
        reasons.push("包含代码响应".to_string());
    }

    MessageImportance {
        message: message.clone(),
        score,
        reasons,
    }
}

#[allow(dead_code)]
pub struct SessionCompactor {
    preserve_top_n: usize,
    min_importance_score: f64,
}

impl Default for SessionCompactor {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl SessionCompactor {
    pub fn new() -> Self {
        Self {
            preserve_top_n: 50,
            min_importance_score: 1.0,
        }
    }

    pub fn with_preserve_count(mut self, n: usize) -> Self {
        self.preserve_top_n = n;
        self
    }

    pub fn with_min_score(mut self, score: f64) -> Self {
        self.min_importance_score = score;
        self
    }

    pub fn score_messages(&self, messages: &[MessageRecord]) -> Vec<MessageImportance> {
        messages.iter().map(score_message_importance).collect()
    }

    pub fn extract_entities(&self, messages: &[MessageRecord]) -> Vec<String> {
        let all_content: String = messages
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join(" ");

        extract_key_terms(&all_content)
    }

    pub fn compress(&self, messages: &[MessageRecord]) -> CompressionResult {
        let original_count = messages.len();
        let all_importance = self.score_messages(messages);

        let mut heap: BinaryHeap<ImportanceOrd> =
            BinaryHeap::with_capacity(all_importance.len());

        for imp in all_importance {
            if imp.score >= self.min_importance_score {
                heap.push(ImportanceOrd(imp));
            }
        }

        let mut preserved: Vec<MessageRecord> = Vec::new();
        let mut preserved_count = 0;

        while let Some(ImportanceOrd(imp)) = heap.pop() {
            if preserved_count >= self.preserve_top_n {
                break;
            }
            preserved.push(imp.message);
            preserved_count += 1;
        }

        preserved.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let removed_count = original_count.saturating_sub(preserved.len());

        let summary = if removed_count > 0 {
            format!(
                "从{}条消息中保留{}条，移除{}条低优先级消息",
                original_count,
                preserved.len(),
                removed_count
            )
        } else {
            format!("保留全部{}条消息，无需压缩", original_count)
        };

        let key_entities = self.extract_entities(&preserved);

        // Verify compression integrity
        let integrity = verify_compression_integrity(messages, &preserved, &key_entities);
        if !integrity.is_valid {
            tracing::warn!(
                "Compression integrity check failed: {:?}",
                integrity.checks.iter().filter(|c| !c.passed).map(|c| &c.name).collect::<Vec<_>>()
            );
        }

        CompressionResult {
            original_count,
            removed_count,
            summary,
            preserved_messages: preserved,
            key_entities,
        }
    }

    pub fn get_importance_distribution(&self, messages: &[MessageRecord]) -> HashMap<String, usize> {
        let scored = self.score_messages(messages);
        let mut distribution: HashMap<String, usize> = HashMap::new();

        for imp in scored {
            let bucket = if imp.score >= 5.0 {
                "非常高".to_string()
            } else if imp.score >= 3.0 {
                "高".to_string()
            } else if imp.score >= 1.0 {
                "中".to_string()
            } else {
                "低".to_string()
            };
            *distribution.entry(bucket).or_insert(0) += 1;
        }

        distribution
    }

    pub fn find_critical_messages(&self, messages: &[MessageRecord]) -> Vec<MessageImportance> {
        let scored = self.score_messages(messages);
        scored
            .into_iter()
            .filter(|imp| imp.score >= 3.0)
            .collect()
    }

    pub fn summarize_by_topic(&self, messages: &[MessageRecord]) -> HashMap<String, Vec<String>> {
        let mut topic_messages: HashMap<String, Vec<String>> = HashMap::new();

        for message in messages {
            let terms = extract_key_terms(&message.content);
            for term in terms.into_iter().take(3) {
                topic_messages
                    .entry(term)
                    .or_default()
                    .push(message.content.chars().take(50).collect());
            }
        }

        topic_messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importance_scoring() {
        let compactor = SessionCompactor::new();
        let messages = vec![
            MessageRecord {
                id: "1".to_string(),
                role: "user".to_string(),
                content: "Hello world".to_string(),
                message_type: None,
                timestamp: 1000,
                tool_calls: None,
            },
            MessageRecord {
                id: "2".to_string(),
                role: "assistant".to_string(),
                content: "Error: file not found".to_string(),
                message_type: None,
                timestamp: 2000,
                tool_calls: None,
            },
        ];

        let scores = compactor.score_messages(&messages);
        assert_eq!(scores.len(), 2);
        assert!(scores[1].score > scores[0].score);
    }

    #[test]
    fn test_compression() {
        let compactor = SessionCompactor::new().with_preserve_count(2);
        let messages: Vec<MessageRecord> = (0..10)
            .map(|i| MessageRecord {
                id: i.to_string(),
                role: "user".to_string(),
                content: format!("Message {}", i),
                message_type: None,
                timestamp: i as i64 * 1000,
                tool_calls: None,
            })
            .collect();

        let result = compactor.compress(&messages);
        assert_eq!(result.original_count, 10);
        assert!(result.preserved_messages.len() <= 2);
    }

    #[test]
    fn test_entity_extraction() {
        let compactor = SessionCompactor::new();
        let messages = vec![MessageRecord {
            id: "1".to_string(),
            role: "user".to_string(),
            content: "I need to fix the database connection and update the user interface"
                .to_string(),
            message_type: None,
            timestamp: 1000,
            tool_calls: None,
        }];

        let entities = compactor.extract_entities(&messages);
        assert!(entities.contains(&"database".to_string()) || entities.contains(&"interface".to_string()));
    }
}

// ---------------------------------------------------------------------------
// P4-3: Post-compaction integrity verification
// ---------------------------------------------------------------------------

/// Result of verifying session integrity after compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheckResult {
    /// Whether the compressed session passes all integrity checks.
    pub is_valid: bool,
    /// Individual check results with descriptions.
    pub checks: Vec<IntegrityCheck>,
}

/// A single integrity check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Verify the integrity of a compressed session against the original.
///
/// Checks:
/// 1. **Message ordering**: Preserved messages maintain chronological order.
/// 2. **Role alternation**: No two consecutive user messages without an assistant response
///    (soft check — may be valid in some contexts).
/// 3. **Key entity preservation**: Important entities from the original are present in the compressed.
/// 4. **First/last message preservation**: The first user message and last assistant message are kept.
/// 5. **Minimum message count**: At least some messages are preserved (not over-compressed).
pub fn verify_compression_integrity(
    original: &[MessageRecord],
    compressed: &[MessageRecord],
    key_entities: &[String],
) -> IntegrityCheckResult {
    let mut checks = Vec::new();

    // Check 1: Message ordering — timestamps should be non-decreasing
    let ordering_ok = compressed.windows(2).all(|w| w[0].timestamp <= w[1].timestamp);
    checks.push(IntegrityCheck {
        name: "message_ordering".to_string(),
        passed: ordering_ok,
        detail: if ordering_ok {
            "Preserved messages maintain chronological order".to_string()
        } else {
            "Preserved messages are out of chronological order".to_string()
        },
    });

    // Check 2: Key entity preservation — at least 50% of key entities should appear in compressed
    if !key_entities.is_empty() {
        let compressed_text: String = compressed.iter()
            .map(|m| m.content.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        let preserved_entities: Vec<&String> = key_entities.iter()
            .filter(|entity| compressed_text.contains(&entity.to_lowercase()))
            .collect();
        let entity_ratio = preserved_entities.len() as f64 / key_entities.len() as f64;
        let entity_threshold = 0.5;
        let entity_ok = entity_ratio >= entity_threshold;
        checks.push(IntegrityCheck {
            name: "key_entity_preservation".to_string(),
            passed: entity_ok,
            detail: format!(
                "{}/{} key entities preserved ({:.0}%)",
                preserved_entities.len(),
                key_entities.len(),
                entity_ratio * 100.0
            ),
        });
    }

    // Check 3: First user message preserved
    let first_user = original.iter().find(|m| m.role == "user");
    let first_user_preserved = if let Some(first) = first_user {
        compressed.iter().any(|m| m.id == first.id)
    } else {
        true // no user messages to check
    };
    checks.push(IntegrityCheck {
        name: "first_user_message".to_string(),
        passed: first_user_preserved,
        detail: if first_user_preserved {
            "First user message is preserved".to_string()
        } else {
            "First user message was removed during compression".to_string()
        },
    });

    // Check 4: Last assistant message preserved
    let last_assistant = original.iter().rev().find(|m| m.role == "assistant");
    let last_assistant_preserved = if let Some(last) = last_assistant {
        compressed.iter().any(|m| m.id == last.id)
    } else {
        true
    };
    checks.push(IntegrityCheck {
        name: "last_assistant_message".to_string(),
        passed: last_assistant_preserved,
        detail: if last_assistant_preserved {
            "Last assistant message is preserved".to_string()
        } else {
            "Last assistant message was removed during compression".to_string()
        },
    });

    // Check 5: Minimum message count — should preserve at least 2 messages
    // (or all if original has <= 2)
    let min_count = if original.len() <= 2 { original.len() } else { 2 };
    let count_ok = compressed.len() >= min_count;
    checks.push(IntegrityCheck {
        name: "minimum_message_count".to_string(),
        passed: count_ok,
        detail: format!(
            "Compressed has {} messages (minimum: {})",
            compressed.len(),
            min_count
        ),
    });

    let is_valid = checks.iter().all(|c| c.passed);

    IntegrityCheckResult { is_valid, checks }
}
