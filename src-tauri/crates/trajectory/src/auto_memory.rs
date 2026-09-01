// SPDX-License-Identifier: AGPL-3.0-only

//! Auto Memory Extractor - 规则式启发提取器（非 LLM）
//!
//! 从已完成的轨迹中按启发规则提取结构化记忆：
//! - 用户偏好与习惯
//! - 用户项目、环境、工作流的关键事实
//! - 值得记住的重要模式
//! - 支持跨会话连续性的上下文
//!
//! 模板跟随会话语言：检测到中文会话时输出中文记忆，避免「中文对话里混入英文模板」。

use crate::insight::{InsightCategory, LearningInsight};
use crate::memory::MemoryService;
use crate::pattern::PatternLearner;
use crate::storage::TrajectoryStorage;
use crate::trajectory::{Trajectory, TrajectoryOutcome};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

const MEMORY_EXTRACTION_MIN_STEPS: usize = 4;
const MAX_MEMORY_ENTRIES_PER_TRAJECTORY: usize = 5;
const _MEMORY_DECAY_DAYS: i64 = 30;

/// CJK 字符（含扩展 A 区）占非空白字符比例 ≥ 15% 判为中文文本
fn is_cjk_text(s: &str) -> bool {
    let total = s.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }
    let cjk = s
        .chars()
        .filter(|c| matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}'))
        .count();
    cjk as f64 / total as f64 >= 0.15
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub memory_type: MemoryType,
    pub content: String,
    pub confidence: f64,
    pub source_trajectory: String,
    pub extraction_reason: String,
    /// Unix timestamp in milliseconds
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Preference,
    Fact,
    Pattern,
    Context,
    Project,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Preference => "preference",
            MemoryType::Fact => "fact",
            MemoryType::Pattern => "pattern",
            MemoryType::Context => "context",
            MemoryType::Project => "project",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExtractionResult {
    pub extracted_memories: Vec<ExtractedMemory>,
    pub insights_generated: Vec<LearningInsight>,
    pub trajectories_analyzed: usize,
}

pub struct AutoMemoryExtractor {
    memory_service: Arc<tokio::sync::RwLock<MemoryService>>,
    recent_extractions: Vec<ExtractedMemory>,
    extraction_cache: HashMap<String, Vec<ExtractedMemory>>,
}

impl AutoMemoryExtractor {
    pub fn new(
        _storage: Arc<TrajectoryStorage>,
        memory_service: Arc<tokio::sync::RwLock<MemoryService>>,
        _pattern_learner: Arc<tokio::sync::RwLock<PatternLearner>>,
    ) -> Self {
        Self { memory_service, recent_extractions: Vec::new(), extraction_cache: HashMap::new() }
    }

    pub fn analyze_trajectory(
        &mut self,
        trajectory: &Trajectory,
    ) -> Option<MemoryExtractionResult> {
        if trajectory.steps.len() < MEMORY_EXTRACTION_MIN_STEPS {
            return None;
        }

        if let Some(cached) = self.extraction_cache.get(&trajectory.id) {
            let insights = self.generate_insights(cached, trajectory);
            return Some(MemoryExtractionResult {
                extracted_memories: cached.clone(),
                insights_generated: insights,
                trajectories_analyzed: 1,
            });
        }

        let memories = self.extract_memories_from_trajectory(trajectory);
        let insights = self.generate_insights(&memories, trajectory);

        for memory in &memories {
            self.recent_extractions.push(memory.clone());
        }
        if self.recent_extractions.len() > 100 {
            self.recent_extractions.drain(0..50);
        }

        self.extraction_cache.insert(trajectory.id.clone(), memories.clone());

        Some(MemoryExtractionResult {
            extracted_memories: memories,
            insights_generated: insights,
            trajectories_analyzed: 1,
        })
    }

    fn extract_memories_from_trajectory(&self, trajectory: &Trajectory) -> Vec<ExtractedMemory> {
        let mut memories = Vec::new();
        let mut seen_content: HashMap<String, usize> = HashMap::new();

        let user_messages: Vec<_> = trajectory
            .steps
            .iter()
            .filter(|s| matches!(s.role, crate::trajectory::MessageRole::User))
            .collect();

        let assistant_messages: Vec<_> = trajectory
            .steps
            .iter()
            .filter(|s| matches!(s.role, crate::trajectory::MessageRole::Assistant))
            .collect();

        // 跟随会话语言：以首条用户消息判定中英文，输出对应语言的记忆模板
        let zh = user_messages.first().map(|m| is_cjk_text(&m.content)).unwrap_or(false);

        if let Some(first_user) = user_messages.first() {
            let content_lower = first_user.content.to_lowercase();
            let is_greeting = content_lower.contains("hello")
                || content_lower.contains("hi ")
                || content_lower.contains("hey")
                || content_lower.contains("你好")
                || content_lower.contains("您好");
            if !is_greeting {
                let snippet: String = first_user.content.chars().take(200).collect();
                let content = if zh {
                    format!("用户正在处理：{snippet}")
                } else {
                    format!("User is working on: {snippet}")
                };
                memories.push(ExtractedMemory {
                    memory_type: MemoryType::Context,
                    content,
                    confidence: 0.7,
                    source_trajectory: trajectory.id.clone(),
                    extraction_reason: if zh {
                        "首条用户消息指示任务上下文".to_string()
                    } else {
                        "First user message indicates task context".to_string()
                    },
                    created_at: Utc::now().timestamp_millis(),
                });
                *seen_content.entry(first_user.content.clone()).or_insert(0) += 1;
            }
        }

        for (i, step) in assistant_messages.iter().enumerate() {
            if let Some(ref tool_calls) = step.tool_calls
                && !tool_calls.is_empty()
            {
                let tool_names: Vec<String> = tool_calls.iter().map(|tc| tc.name.clone()).collect();
                let unique_tools: Vec<String> = tool_names
                    .iter()
                    .cloned()
                    .collect::<std::collections::HashSet<_>>()
                    .iter()
                    .cloned()
                    .collect();

                if unique_tools.len() >= 2 {
                    let pattern_key = unique_tools.join(",");
                    let count = seen_content.entry(pattern_key.clone()).or_insert(0);
                    *count += 1;

                    if *count >= 2 {
                        let content = if zh {
                            format!("用户经常组合使用工具：{}", unique_tools.join(" -> "))
                        } else {
                            format!(
                                "User frequently uses tools together: {}",
                                unique_tools.join(" -> ")
                            )
                        };
                        memories.push(ExtractedMemory {
                            memory_type: MemoryType::Pattern,
                            content,
                            confidence: 0.8,
                            source_trajectory: trajectory.id.clone(),
                            extraction_reason: if zh {
                                "检测到重复的工具组合".to_string()
                            } else {
                                "Repeated tool combination detected".to_string()
                            },
                            created_at: Utc::now().timestamp_millis(),
                        });
                    }
                }
            }

            if let Some(ref reasoning) = step.reasoning
                && reasoning.len() > 100
                && i == 0
            {
                let key = "detailed_reasoning".to_string();
                let count = seen_content.entry(key).or_insert(0);
                *count += 1;

                if *count >= 2 {
                    let (content, reason) = if zh {
                        (
                            "用户偏好详细的推理与逐步分析".to_string(),
                            "多次观察到详细的推理链".to_string(),
                        )
                    } else {
                        (
                            "User appreciates detailed reasoning and step-by-step problem solving"
                                .to_string(),
                            "Multiple detailed reasoning chains observed".to_string(),
                        )
                    };
                    memories.push(ExtractedMemory {
                        memory_type: MemoryType::Preference,
                        content,
                        confidence: 0.75,
                        source_trajectory: trajectory.id.clone(),
                        extraction_reason: reason,
                        created_at: Utc::now().timestamp_millis(),
                    });
                }
            }
        }

        match trajectory.outcome {
            TrajectoryOutcome::Success => {
                let (content, reason) = if zh {
                    (format!("任务「{}」已完成", trajectory.topic), "任务成功完成".to_string())
                } else {
                    (
                        format!("Task '{}' was completed successfully", trajectory.topic),
                        "Successful task completion".to_string(),
                    )
                };
                memories.push(ExtractedMemory {
                    memory_type: MemoryType::Fact,
                    content,
                    confidence: 0.9,
                    source_trajectory: trajectory.id.clone(),
                    extraction_reason: reason,
                    created_at: Utc::now().timestamp_millis(),
                });
            },
            TrajectoryOutcome::Failure => {
                let error_tools: usize = trajectory
                    .steps
                    .iter()
                    .filter_map(|s| {
                        s.tool_results.as_ref().and_then(|r| {
                            r.iter()
                                .find(|tr| tr.is_error || tr.output.contains("error"))
                                .map(|_| &s.tool_calls)
                        })
                    })
                    .count();

                if error_tools > 0 {
                    let (content, reason) = if zh {
                        (
                            format!("任务「{}」失败，可能需要排查", trajectory.topic),
                            "失败任务且存在错误指标".to_string(),
                        )
                    } else {
                        (
                            format!(
                                "Task '{}' failed - may need troubleshooting approach",
                                trajectory.topic
                            ),
                            "Failed task with error indicators".to_string(),
                        )
                    };
                    memories.push(ExtractedMemory {
                        memory_type: MemoryType::Context,
                        content,
                        confidence: 0.6,
                        source_trajectory: trajectory.id.clone(),
                        extraction_reason: reason,
                        created_at: Utc::now().timestamp_millis(),
                    });
                }
            },
            TrajectoryOutcome::Partial => {
                let (content, reason) = if zh {
                    (
                        format!("任务「{}」部分完成，可能需要跟进", trajectory.topic),
                        "任务部分完成".to_string(),
                    )
                } else {
                    (
                        format!(
                            "Task '{}' partially completed - follow-up may be needed",
                            trajectory.topic
                        ),
                        "Partial task completion".to_string(),
                    )
                };
                memories.push(ExtractedMemory {
                    memory_type: MemoryType::Context,
                    content,
                    confidence: 0.65,
                    source_trajectory: trajectory.id.clone(),
                    extraction_reason: reason,
                    created_at: Utc::now().timestamp_millis(),
                });
            },
            TrajectoryOutcome::Abandoned => {},
        }

        // 用标准哈希集合去重，缓存命中时也保证一致性
        let mut dedup_set = std::collections::HashSet::new();
        for memory in &memories {
            let key = memory.content.chars().take(50).collect::<String>();
            dedup_set.insert(key);
        }
        let deduplicated: Vec<_> = memories
            .into_iter()
            .filter(|m| {
                let key = m.content.chars().take(50).collect::<String>();
                // seen_content 的 key 与去重 key 类型不同；这里改用 dedup_set 查首次出现
                dedup_set.take(&key).is_some()
            })
            .take(MAX_MEMORY_ENTRIES_PER_TRAJECTORY)
            .collect();

        deduplicated
    }

    fn generate_insights(
        &self,
        memories: &[ExtractedMemory],
        trajectory: &Trajectory,
    ) -> Vec<LearningInsight> {
        let mut insights = Vec::new();

        for memory in memories {
            if memory.confidence >= 0.7 && memory.memory_type == MemoryType::Pattern {
                let zh = is_cjk_text(&memory.content);
                let (title, action) = if zh {
                    (
                        format!("检测到：{}", memory.content.chars().take(40).collect::<String>()),
                        Some("考虑将此模式加入用户画像".to_string()),
                    )
                } else {
                    (
                        format!(
                            "Detected: {}",
                            memory.content.chars().take(40).collect::<String>()
                        ),
                        Some("Consider adding this pattern to user profile".to_string()),
                    )
                };
                insights.push(LearningInsight {
                    id: format!("insight_{}_{}", trajectory.id, memory.memory_type.as_str()),
                    category: InsightCategory::Pattern,
                    title,
                    description: memory.extraction_reason.clone(),
                    confidence: memory.confidence,
                    evidence: vec![memory.source_trajectory.clone()],
                    suggested_action: action,
                    created_at: chrono::Utc::now().timestamp_millis(),
                });
            }
        }

        insights
    }

    pub fn get_recent_extractions(&self) -> Vec<ExtractedMemory> {
        self.recent_extractions.clone()
    }

    pub fn clear_cache(&mut self) {
        self.extraction_cache.clear();
    }

    pub async fn apply_memories_to_service(
        &self,
        memories: &[ExtractedMemory],
    ) -> anyhow::Result<usize> {
        let memory_service = self.memory_service.write().await;
        let mut applied = 0;

        for memory in memories {
            let result = memory_service
                .add_memory_with_dedup(memory.memory_type.as_str(), &memory.content)
                .await;
            if result.success {
                applied += 1;
            } else {
                tracing::debug!("Memory dedup skipped: {}", result.message);
            }
        }

        Ok(applied)
    }
}
