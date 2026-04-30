use crate::trajectory::{MessageRole, Trajectory};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedStep {
    pub role: String,
    pub content_summary: String,
    pub tool_calls: Vec<CompressedToolCall>,
    pub is_decision_point: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedToolCall {
    pub name: String,
    pub arguments_summary: String,
    pub result_summary: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedTrajectory {
    pub id: String,
    pub session_id: String,
    pub topic: String,
    pub outcome: String,
    pub steps: Vec<CompressedStep>,
    pub decision_points: usize,
    pub compression_ratio: f64,
}

pub struct TrajectoryCompressor {
    max_content_length: usize,
}

impl TrajectoryCompressor {
    pub fn new(max_content_length: usize) -> Self {
        Self { max_content_length }
    }

    pub fn compress(&self, trajectory: &Trajectory) -> CompressedTrajectory {
        let original_steps = trajectory.steps.len();
        let mut compressed_steps = Vec::new();
        let mut decision_points = 0;

        for step in &trajectory.steps {
            let is_decision = step.tool_calls.is_some() || step.reasoning.is_some();
            if is_decision {
                decision_points += 1;
            }
            let tool_calls: Vec<CompressedToolCall> = step
                .tool_calls
                .as_ref()
                .map(|calls| {
                    calls
                        .iter()
                        .map(|tc| CompressedToolCall {
                            name: tc.name.clone(),
                            arguments_summary: summarize(&tc.arguments, self.max_content_length),
                            result_summary: String::new(),
                            is_error: false,
                        })
                        .collect()
                })
                .unwrap_or_default();

            let mut compressed_tool_calls = tool_calls;
            if let Some(results) = &step.tool_results {
                for (i, result) in results.iter().enumerate() {
                    if i < compressed_tool_calls.len() {
                        compressed_tool_calls[i].result_summary =
                            summarize(&result.output, self.max_content_length);
                        compressed_tool_calls[i].is_error = result.is_error;
                    }
                }
            }

            compressed_steps.push(CompressedStep {
                role: match step.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .to_string(),
                content_summary: summarize(&step.content, self.max_content_length),
                tool_calls: compressed_tool_calls,
                is_decision_point: is_decision,
            });
        }

        let compression_ratio = if original_steps > 0 {
            decision_points as f64 / original_steps as f64
        } else {
            1.0
        };

        CompressedTrajectory {
            id: trajectory.id.clone(),
            session_id: trajectory.session_id.clone(),
            topic: trajectory.topic.clone(),
            outcome: format!("{:?}", trajectory.outcome).to_lowercase(),
            steps: compressed_steps,
            decision_points,
            compression_ratio,
        }
    }

    pub fn to_jsonl(
        &self,
        trajectories: &[CompressedTrajectory],
    ) -> Result<String, serde_json::Error> {
        let lines: Result<Vec<String>, _> =
            trajectories.iter().map(serde_json::to_string).collect();
        Ok(lines?.join("\n"))
    }
}

fn summarize(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let half = max_len / 2;
        format!(
            "{}...{}",
            &content[..half],
            &content[content.len() - half..]
        )
    }
}
