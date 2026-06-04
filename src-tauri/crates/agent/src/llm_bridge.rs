use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
#[cfg(test)]
use axagent_harness::trajectory_types::ProcedureStep;
use axagent_harness::trajectory_types::{
    GeneratedTool, LlmEvolutionProvider, LlmJudge, LlmJudgeFuture, LlmMutationRequest,
    LlmMutationResponse, LlmTextGradProvider, LlmToolProvider, PrmLlmProvider, RewardCategory,
    StepReward, ToolCreationRequest,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock};

static SCORE_NUMBER_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(-?\d+\.?\d*)").expect("hardcoded regex is valid"));
static CODE_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?s)```(?:javascript|js)?\s*\n(.*?)```").expect("hardcoded regex is valid")
});
static JSON_OBJECT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?s)\{.*\}").expect("hardcoded regex is valid"));

#[derive(Clone)]
pub struct ProviderLlmBridge {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    model: String,
}

impl ProviderLlmBridge {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: impl Into<String>,
    ) -> Self {
        Self {
            adapter,
            ctx,
            model: model.into(),
        }
    }

    pub async fn call_llm(&self, system: &str, user: &str) -> Result<String, String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(system.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(user.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
            ],
            stream: false,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        self.adapter
            .chat(&self.ctx, request)
            .await
            .map(|resp| resp.content)
            .map_err(|e| e.to_string())
    }

    async fn call_llm_low_temp(&self, system: &str, user: &str) -> Result<String, String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(system.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(user.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
            ],
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(64),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        self.adapter
            .chat(&self.ctx, request)
            .await
            .map(|resp| resp.content)
            .map_err(|e| e.to_string())
    }
}

fn extract_score_from_text(text: &str) -> f64 {
    SCORE_NUMBER_RE
        .captures(text)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0)
}

fn heuristic_mutation(request: &LlmMutationRequest) -> LlmMutationResponse {
    let mut revised = request.current_steps.clone();
    if !request.failure_evidence.is_empty() {
        for step in &mut revised {
            if step.error_handling.is_none() {
                step.error_handling =
                    Some("If this step fails, retry with alternative approach".to_string());
            }
            step.condition = Some("Verify prerequisites before execution".to_string());
        }
    }
    LlmMutationResponse {
        revised_steps: revised,
        reasoning: "Heuristic fallback: added error handling and condition checks".to_string(),
        confidence: 0.5,
    }
}

fn extract_code_from_response(text: &str) -> String {
    if let Some(cap) = CODE_BLOCK_RE.captures(text)
        && let Some(m) = cap.get(1)
    {
        return m.as_str().trim().to_string();
    }
    text.trim().to_string()
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

impl LlmEvolutionProvider for ProviderLlmBridge {
    fn generate_mutation(
        &self,
        request: &LlmMutationRequest,
    ) -> axagent_harness::trajectory_types::LlmMutationFuture<'_> {
        let steps_json = serde_json::to_string(&request.current_steps).unwrap_or_default();
        let failures = request.failure_evidence.join("\n");
        let successes = request.success_evidence.join("\n");
        let user_msg = format!(
            "Skill: {}\n\nCurrent steps:\n{}\n\nFailure evidence:\n{}\n\nSuccess evidence:\n{}\n\n\
             Respond with a JSON object: {{\"revised_steps\": [...], \"reasoning\": \"...\", \"confidence\": 0.0-1.0}}",
            request.skill_name, steps_json, failures, successes
        );
        let fallback = heuristic_mutation(request);

        Box::pin(async move {
            match self.call_llm(
                "You are a skill evolution expert. Analyze the current skill steps and failure evidence to suggest improved steps.",
                &user_msg,
            ).await {
                Ok(text) => {
                    match serde_json::from_str::<LlmMutationResponse>(&text) {
                        Ok(resp) => Ok(resp),
                        Err(_) => {
                            let json_re = &*JSON_OBJECT_RE;
                            if let Some(cap) = json_re.captures(&text)
                                && let Some(m) = cap.get(0)
                                && let Ok(resp) = serde_json::from_str::<LlmMutationResponse>(m.as_str()) {
                                    return Ok(resp);
                                }
                            Ok(fallback)
                        }
                    }
                }
                Err(_) => Ok(fallback),
            }
        })
    }

    fn evaluate_quality(
        &self,
        content: &str,
        context: &str,
    ) -> Pin<Box<dyn Future<Output = Result<f64, String>> + Send + '_>> {
        let user_msg = format!(
            "Evaluate the quality of the following content on a scale from 0.0 to 1.0.\n\nContent:\n{}\n\nContext:\n{}\n\nRespond with ONLY a number between 0.0 and 1.0.",
            content, context
        );

        Box::pin(async move {
            match self
                .call_llm_low_temp(
                    "You are a content quality evaluator. Score the quality from 0.0 to 1.0.",
                    &user_msg,
                )
                .await
            {
                Ok(text) => Ok(extract_score_from_text(&text)),
                Err(e) => Err(e),
            }
        })
    }
}

impl LlmJudge for ProviderLlmBridge {
    fn evaluate_reasoning(&self, reasoning: &str, context: &str) -> LlmJudgeFuture<'_> {
        let user_msg = format!(
            "Evaluate the reasoning quality on a scale from 0.0 to 1.0.\n\nReasoning:\n{}\n\nContext:\n{}\n\nRespond with ONLY a number between 0.0 and 1.0.",
            reasoning, context
        );

        Box::pin(async move {
            match self.call_llm_low_temp(
                "You are a reasoning quality evaluator. Score the reasoning quality from 0.0 to 1.0.",
                &user_msg,
            ).await {
                Ok(text) => Ok(extract_score_from_text(&text)),
                Err(e) => Err(e),
            }
        })
    }

    fn evaluate_tool_efficiency(
        &self,
        tool_name: &str,
        args: &str,
        result: &str,
    ) -> LlmJudgeFuture<'_> {
        let user_msg = format!(
            "Evaluate the tool usage efficiency on a scale from 0.0 to 1.0.\n\nTool: {}\nArguments: {}\nResult: {}\n\nRespond with ONLY a number between 0.0 and 1.0.",
            tool_name, args, result
        );

        Box::pin(async move {
            match self
                .call_llm_low_temp(
                    "You are a tool usage evaluator. Score the tool efficiency from 0.0 to 1.0.",
                    &user_msg,
                )
                .await
            {
                Ok(text) => Ok(extract_score_from_text(&text)),
                Err(e) => Err(e),
            }
        })
    }
}

impl LlmTextGradProvider for ProviderLlmBridge {
    fn compute_gradient(
        &self,
        node_content: &str,
        output_feedback: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>> {
        let user_msg = format!(
            "Node content:\n{}\n\nOutput feedback:\n{}\n\nSuggest specific improvements to the node content based on the feedback.",
            node_content, output_feedback
        );

        Box::pin(async move {
            self.call_llm(
                "You are a text gradient optimizer. Given a node's content and output feedback, suggest specific improvements.",
                &user_msg,
            ).await
        })
    }
}

impl LlmToolProvider for ProviderLlmBridge {
    fn generate_tool_code(
        &self,
        request: &ToolCreationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
        let tool_list = request.available_tools.join(", ");
        let user_msg = format!(
            "Pattern: {}\nContext: {}\nAvailable tools: {}\n\nGenerate a JavaScript function that implements this pattern. Wrap the code in ```javascript``` code blocks.",
            request.pattern_description, request.context, tool_list
        );
        let name = slugify(&request.pattern_description);
        let description = request.pattern_description.clone();

        Box::pin(async move {
            match self.call_llm(
                "You are a tool code generator. Generate a JavaScript function that implements the described pattern.",
                &user_msg,
            ).await {
                Ok(text) => {
                    let code = extract_code_from_response(&text);
                    Ok(GeneratedTool::new(&name, &code, &description))
                }
                Err(e) => Err(e),
            }
        })
    }

    fn improve_tool_code(
        &self,
        tool: &GeneratedTool,
        error: &str,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
        let user_msg = format!(
            "Current code:\n```javascript\n{}\n```\n\nError:\n{}\n\nFix the errors and return the improved code wrapped in ```javascript``` code blocks.",
            tool.code, error
        );
        let name = tool.name.clone();
        let description = tool.description.clone();

        Box::pin(async move {
            match self
                .call_llm(
                    "You are a tool code improver. Fix the errors in the provided tool code.",
                    &user_msg,
                )
                .await
            {
                Ok(text) => {
                    let code = extract_code_from_response(&text);
                    Ok(GeneratedTool::new(&name, &code, &description))
                },
                Err(e) => Err(e),
            }
        })
    }
}

impl PrmLlmProvider for ProviderLlmBridge {
    fn evaluate_step(
        &self,
        step_content: &str,
        context: &str,
        previous_steps: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<StepReward, String>> + Send + '_>> {
        let prev_summary = previous_steps
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        let user_msg = format!(
            "Step content:\n{}\n\nTask context:\n{}\n\nPrevious steps summary:\n{}\n\n\
             Evaluate this step on each dimension. Respond with JSON:\n\
             {{\"correctness\": 0.0-1.0, \"coherence\": 0.0-1.0, \"completeness\": 0.0-1.0, \"efficiency\": 0.0-1.0, \"safety\": 0.0-1.0, \"reasoning\": \"...\"}}",
            step_content, context, prev_summary
        );

        Box::pin(async move {
            match self.call_llm(
                "You are a process reward model evaluator. Score each dimension from 0.0 to 1.0.",
                &user_msg,
            ).await {
                Ok(text) => {
                    let json_re = &*JSON_OBJECT_RE;
                    if let Some(cap) = json_re.captures(&text)
                        && let Some(m) = cap.get(0)
                        && let Ok(v) = serde_json::from_str::<serde_json::Value>(m.as_str()) {
                            let correctness = v.get("correctness").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let coherence = v.get("coherence").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let completeness = v.get("completeness").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let efficiency = v.get("efficiency").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let safety = v.get("safety").and_then(|v| v.as_f64()).unwrap_or(0.5);
                            let reasoning = v.get("reasoning").and_then(|v| v.as_str()).unwrap_or("LLM evaluation").to_string();

                            let categories = vec![
                                (RewardCategory::Correctness, correctness.clamp(0.0, 1.0)),
                                (RewardCategory::Coherence, coherence.clamp(0.0, 1.0)),
                                (RewardCategory::Completeness, completeness.clamp(0.0, 1.0)),
                                (RewardCategory::Efficiency, efficiency.clamp(0.0, 1.0)),
                                (RewardCategory::Safety, safety.clamp(0.0, 1.0)),
                            ];
                            let reward: f64 = categories.iter().map(|(c, s)| c.weight() * s).sum();

                            return Ok(StepReward {
                                step_index: 0,
                                reward,
                                reasoning,
                                categories,
                            });
                        }
                    let score = extract_score_from_text(&text);
                    Ok(StepReward {
                        step_index: 0,
                        reward: score,
                        reasoning: format!("LLM PRM fallback score: {:.2}", score),
                        categories: vec![
                            (RewardCategory::Correctness, score),
                            (RewardCategory::Coherence, score),
                            (RewardCategory::Completeness, score),
                            (RewardCategory::Efficiency, score),
                            (RewardCategory::Safety, score),
                        ],
                    })
                }
                Err(e) => Err(e),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_score_from_text_number_only() {
        assert!((extract_score_from_text("0.75") - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_with_context() {
        let score = extract_score_from_text("I would rate this 0.85 out of 1.0");
        assert!((score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_integer() {
        let score = extract_score_from_text("Score: 1");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_zero() {
        let score = extract_score_from_text("0.0");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_no_number() {
        let score = extract_score_from_text("no numbers here");
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_clamped_high() {
        let score = extract_score_from_text("9.5");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_score_from_text_clamped_negative() {
        let score = extract_score_from_text("-0.3");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_code_from_response_with_block() {
        let text = "Here is the code:\n```javascript\nfunction hello() { return 42; }\n```\nDone.";
        let code = extract_code_from_response(text);
        assert_eq!(code, "function hello() { return 42; }");
    }

    #[test]
    fn test_extract_code_from_response_with_js_tag() {
        let text = "```\nfunction foo() {}\n```";
        let code = extract_code_from_response(text);
        assert_eq!(code, "function foo() {}");
    }

    #[test]
    fn test_extract_code_from_response_no_block() {
        let text = "function bar() { return 1; }";
        let code = extract_code_from_response(text);
        assert_eq!(code, "function bar() { return 1; }");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Search Files"), "search_files");
        assert_eq!(slugify("hello-world"), "hello_world");
        assert_eq!(slugify("  multiple   spaces  "), "multiple_spaces");
    }

    #[test]
    fn test_heuristic_mutation_with_failures() {
        let request = LlmMutationRequest {
            skill_name: "test".to_string(),
            current_steps: vec![ProcedureStep {
                order: 0,
                action: "Use tool1".to_string(),
                tool: Some("tool1".to_string()),
                condition: None,
                error_handling: None,
            }],
            failure_evidence: vec!["error occurred".to_string()],
            success_evidence: vec![],
        };
        let response = heuristic_mutation(&request);
        assert!(response.revised_steps[0].error_handling.is_some());
        assert!(response.revised_steps[0].condition.is_some());
        assert!(response.confidence < 0.6);
    }

    #[test]
    fn test_heuristic_mutation_no_failures() {
        let request = LlmMutationRequest {
            skill_name: "test".to_string(),
            current_steps: vec![ProcedureStep {
                order: 0,
                action: "Use tool1".to_string(),
                tool: Some("tool1".to_string()),
                condition: None,
                error_handling: None,
            }],
            failure_evidence: vec![],
            success_evidence: vec!["worked".to_string()],
        };
        let response = heuristic_mutation(&request);
        assert!(response.revised_steps[0].error_handling.is_none());
    }
}
