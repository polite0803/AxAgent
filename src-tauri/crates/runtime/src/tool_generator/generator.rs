use super::types::{
    GeneratedTool, GeneratedToolImplementation, GeneratedToolSourceInfo, ToolGenerationInput,
};

/// Tool generator that creates tool implementations using Agent + LLM
pub struct ToolGenerator;

impl ToolGenerator {
    /// Generate a Prompt template for a missing tool.
    ///
    /// This constructs a generation prompt, calls the Developer Agent
    /// to produce a Prompt template, and returns a GeneratedTool.
    ///
    /// The actual Agent call is handled by the Tauri command layer
    /// which has access to AppState. This method provides the
    /// generation prompt construction and result parsing.
    pub fn build_generation_prompt(input: &ToolGenerationInput) -> String {
        format!(
            r#"You are a tool implementation expert. Please generate a Prompt template for the following tool.

Tool Name: {}
Description: {}
Input Schema: {}
Output Schema: {}

Requirements:
1. The Prompt template must contain a {{{{input}}}} placeholder where the input JSON will be injected at runtime.
2. The template should instruct the LLM to process the input according to the tool's description.
3. The template should request output in a format that matches the output schema.
4. Keep the template concise and focused on the tool's purpose.
5. Output ONLY the Prompt template text, nothing else.

Generate the Prompt template:"#,
            input.name,
            input.description,
            serde_json::to_string_pretty(&input.input_schema).unwrap_or_default(),
            serde_json::to_string_pretty(&input.output_schema).unwrap_or_default(),
        )
    }

    /// Parse the Agent's response to extract the Prompt template
    pub fn parse_agent_response(
        response: &str,
        input: &ToolGenerationInput,
        agent_model: Option<&str>,
    ) -> Result<GeneratedTool, String> {
        // Validate that the response contains the {{input}} placeholder
        if !response.contains("{{input}}") {
            return Err(
                "Generated template does not contain {{input}} placeholder".to_string()
            );
        }

        let tool_name = format!("generated_{}", slugify(&input.name));
        let now = chrono::Utc::now().timestamp_millis();

        Ok(GeneratedTool {
            tool_name,
            implementation: GeneratedToolImplementation::PromptTemplate {
                template: response.to_string(),
                model: None,
                temperature: Some(0.1),
            },
            input_schema: input.input_schema.clone(),
            output_schema: input.output_schema.clone(),
            source_info: GeneratedToolSourceInfo {
                original_name: input.name.clone(),
                original_description: input.description.clone(),
                generation_method: "agent_prompt_template".to_string(),
                agent_model: agent_model.map(|s| s.to_string()),
                generated_at: now,
            },
        })
    }
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
