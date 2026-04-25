use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::future::Future;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParseRequest {
    pub content: String,
    pub context: LlmParseContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParseContext {
    pub composite_name: String,
    pub existing_skills: Vec<String>,
    pub available_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParseResponse {
    pub steps: Vec<LlmParsedStep>,
    pub confidence: f32,
    pub raw_llm_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParsedStep {
    pub title: String,
    pub description: String,
    pub raw_content: String,
    pub tool_name: Option<String>,
    pub tool_type: Option<String>,
    pub step_type: StepType,
    pub condition_expression: Option<String>,
    pub then_branch: Option<String>,
    pub else_branch: Option<String>,
    pub loop_items_var: Option<String>,
    pub max_iterations: Option<u32>,
    pub loop_body: Option<Vec<LlmParsedStep>>,
    pub parallel_branches: Option<Vec<LlmParsedBranch>>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Atomic,
    Condition,
    Loop,
    Parallel,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParsedBranch {
    pub name: String,
    pub steps: Vec<String>,
    pub raw_content: Option<String>,
}

pub type LlmParseFuture<'a> = Pin<Box<dyn Future<Output = Result<LlmParseResponse, String>> + Send + 'a>>;

pub trait LlmAssistedParser: Send + Sync {
    fn parse_with_llm(&self, request: &LlmParseRequest) -> LlmParseFuture<'_>;
}

pub struct LlmParsePrompt;

impl LlmParsePrompt {
    pub fn build_prompt(request: &LlmParseRequest) -> String {
        let context = &request.context;
        let tools_list = context.available_tools.join(", ");

        format!(
            r#"You are a skill decomposition assistant. Parse the following composite skill into structured steps.

Composite Skill: {}

Available Tools: {}

Instructions:
1. Identify each step in the skill
2. For each step, determine if it's an atomic tool call, condition, loop, or parallel branch
3. Extract then/else branches for conditions
4. Extract loop body for loops
5. Infer input/output schemas from descriptions if possible

Return your analysis as a JSON object with this structure:
{{
  "steps": [
    {{
      "title": "Step title",
      "description": "Step description",
      "raw_content": "Original markdown content",
      "tool_name": "tool_name or null",
      "tool_type": "mcp, plugin, local, builtin, or null",
      "step_type": "atomic, condition, loop, parallel, or generic",
      "condition_expression": "expression or null",
      "then_branch": "then content or null",
      "else_branch": "else content or null",
      "loop_items_var": "iteration variable or null",
      "max_iterations": number or null,
      "loop_body": [nested steps] or null,
      "parallel_branches": [branches] or null,
      "input_schema": {{"type": "object", "properties": {{}}}} or null,
      "output_schema": {{"type": "object", "properties": {{}}}} or null
    }}
  ],
  "confidence": 0.0-1.0,
  "raw_llm_output": "original response text"
}}

Content to parse:
{}
"#,
            context.composite_name,
            tools_list,
            request.content
        )
    }
}