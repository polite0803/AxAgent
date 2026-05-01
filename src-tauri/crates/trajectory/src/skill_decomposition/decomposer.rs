use serde::{Deserialize, Serialize};

use super::tool_resolver::ToolDependency;
use crate::atomic_skill::types::AtomicSkill;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub id: String,
    pub language: Option<String>,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillContentType {
    Metadata,
    TextualInstruction,
    CodeScript,
    ToolConfig,
    ContextKnowledge,
}

impl CodeBlock {
    pub fn new(
        id: String,
        language: Option<String>,
        content: String,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        Self {
            id,
            language,
            content,
            start_line,
            end_line,
        }
    }

    pub fn infer_dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();

        match self.language.as_deref() {
            Some("python") | Some("py") => {
                for line in self.content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("import ") {
                        if let Some(module) = trimmed
                            .strip_prefix("import ")
                            .and_then(|s| s.split_whitespace().next())
                        {
                            deps.push(module.to_string());
                        }
                    } else if trimmed.starts_with("from ") {
                        if let Some(module) = trimmed
                            .strip_prefix("from ")
                            .and_then(|s| s.split_whitespace().next())
                        {
                            if module != "typing" && module != "collections" {
                                deps.push(module.to_string());
                            }
                        }
                    }
                }
            },
            Some("javascript") | Some("js") | Some("typescript") | Some("ts") => {
                for line in self.content.lines() {
                    let trimmed = line.trim();
                    if (trimmed.starts_with("const ")
                        || trimmed.starts_with("let ")
                        || trimmed.starts_with("var "))
                        && trimmed.contains("require(")
                    {
                        if let Some(module) = trimmed
                            .split("require(")
                            .nth(1)
                            .and_then(|s| s.split(')').next())
                        {
                            let module = module.trim().trim_matches(|c| c == '\'' || c == '"');
                            deps.push(module.to_string());
                        }
                    }
                    if trimmed.starts_with("import ") {
                        if let Some(module) = trimmed
                            .strip_prefix("import ")
                            .and_then(|s| s.split_whitespace().next())
                        {
                            if !module.starts_with('.') && module != "react" && module != "node:fs"
                            {
                                deps.push(module.to_string());
                            }
                        }
                    }
                }
            },
            Some("yaml") | Some("yml") | Some("json") => {
                for line in self.content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("dependencies:") || trimmed.starts_with("requires:") {
                        continue;
                    }
                }
            },
            _ => {},
        }

        deps
    }

    pub fn infer_schema(&self) -> Option<serde_json::Value> {
        match self.language.as_deref() {
            Some("json") => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&self.content) {
                    return Some(parsed);
                }
            },
            Some("yaml") | Some("yml") => {
                if let Ok(parsed) = serde_yaml::from_str::<serde_json::Value>(&self.content) {
                    return Some(parsed);
                }
            },
            _ => {},
        }
        None
    }

    pub fn is_script(&self) -> bool {
        matches!(
            self.language.as_deref(),
            Some("python")
                | Some("py")
                | Some("javascript")
                | Some("js")
                | Some("typescript")
                | Some("ts")
                | Some("bash")
                | Some("sh")
                | Some("shell")
                | Some("ruby")
                | Some("rb")
                | Some("go")
                | Some("rust")
        )
    }

    pub fn is_config(&self) -> bool {
        matches!(
            self.language.as_deref(),
            Some("json") | Some("yaml") | Some("yml") | Some("toml") | Some("ini") | Some("conf")
        )
    }
}

pub struct ContentPreprocessor;

impl ContentPreprocessor {
    pub fn extract_code_blocks(content: &str) -> (String, Vec<CodeBlock>) {
        let mut blocks = Vec::new();
        let mut placeholders = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        let mut block_counter = 0;

        while i < lines.len() {
            let line = lines[i];

            if line.trim().starts_with("```") {
                let language = line.trim().trim_start_matches("```").trim();
                let lang = if language.is_empty() {
                    None
                } else {
                    Some(language.to_string())
                };

                let start_idx = i;
                let mut end_idx = start_idx + 1;
                while end_idx < lines.len() && !lines[end_idx].trim().starts_with("```") {
                    end_idx += 1;
                }

                let block_content: String =
                    lines[start_idx + 1..end_idx.min(lines.len())].join("\n");

                let block_id = format!("cb_{}", block_counter);
                let placeholder = format!("__CODE_BLOCK_{}__", block_counter);
                placeholders.push((start_idx, end_idx, placeholder.clone()));
                blocks.push(CodeBlock::new(
                    block_id,
                    lang,
                    block_content,
                    start_idx,
                    end_idx.min(lines.len()),
                ));

                block_counter += 1;
                i = end_idx + 1;
                continue;
            }

            i += 1;
        }

        let mut result = content.to_string();
        for (start, end, placeholder) in placeholders.into_iter().rev() {
            let old: String = lines[start..end.min(lines.len())].join("\n");
            result = result.replacen(&old, &placeholder, 1);
        }

        (result, blocks)
    }

    pub fn restore_code_blocks(content: &str, blocks: &[CodeBlock]) -> String {
        let mut result = content.to_string();
        for (i, block) in blocks.iter().enumerate() {
            let placeholder = format!("__CODE_BLOCK_{}__", i);
            result = result.replace(&placeholder, &block.content);
        }
        result
    }
}

/// Raw composite skill data from marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSkillData {
    pub name: String,
    pub description: String,
    pub content: String,
    pub source: String,
    pub version: Option<String>,
    pub repo: Option<String>,
}

/// Parsed composite skill structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedComposite {
    pub name: String,
    pub description: String,
    pub source: String,
    pub version: Option<String>,
    pub repo: Option<String>,
    pub steps: Vec<ParsedStep>,
    pub is_fully_parsed: bool,
    pub code_blocks: Vec<CodeBlock>,
}

/// A single parsed step from a composite skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedStep {
    pub title: String,
    pub description: String,
    pub raw_content: String,
    pub tool_name: Option<String>,
    pub tool_type: Option<String>,
    pub is_condition: bool,
    pub is_loop: bool,
    pub is_parallel: bool,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub condition_expression: Option<String>,
    pub then_branch: Option<String>,
    pub else_branch: Option<String>,
    pub loop_items_var: Option<String>,
    pub max_iterations: Option<u32>,
    pub loop_body_raw: Option<String>,
    pub loop_body_steps: Vec<ParsedStep>,
    pub parallel_branches: Vec<ParallelBranch>,
    pub code_blocks: Vec<CodeBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelBranch {
    pub name: String,
    pub steps: Vec<String>,
    pub raw_content: Option<String>,
}

impl ParsedStep {
    pub fn new(title: String) -> Self {
        Self {
            title,
            description: String::new(),
            raw_content: String::new(),
            tool_name: None,
            tool_type: None,
            is_condition: false,
            is_loop: false,
            is_parallel: false,
            input_schema: None,
            output_schema: None,
            condition_expression: None,
            then_branch: None,
            else_branch: None,
            loop_items_var: None,
            max_iterations: None,
            loop_body_raw: None,
            loop_body_steps: Vec::new(),
            parallel_branches: Vec::new(),
            code_blocks: Vec::new(),
        }
    }

    pub fn with_tool(mut self, name: String, tool_type: Option<String>) -> Self {
        self.tool_name = Some(name);
        self.tool_type = tool_type;
        self
    }

    pub fn with_condition(mut self, expression: String) -> Self {
        self.is_condition = true;
        self.condition_expression = Some(expression);
        self
    }

    pub fn with_branches(
        mut self,
        then_branch: Option<String>,
        else_branch: Option<String>,
    ) -> Self {
        self.then_branch = then_branch;
        self.else_branch = else_branch;
        self
    }

    pub fn with_loop(mut self, items_var: Option<String>, max_iterations: Option<u32>) -> Self {
        self.is_loop = true;
        self.loop_items_var = items_var;
        self.max_iterations = max_iterations;
        self
    }

    pub fn with_loop_body(mut self, raw: String, steps: Vec<ParsedStep>) -> Self {
        self.loop_body_raw = Some(raw);
        self.loop_body_steps = steps;
        self
    }

    pub fn with_parallel(mut self, branches: Vec<ParallelBranch>) -> Self {
        self.is_parallel = true;
        self.parallel_branches = branches;
        self
    }

    pub fn with_schema(
        mut self,
        input: Option<serde_json::Value>,
        output: Option<serde_json::Value>,
    ) -> Self {
        self.input_schema = input;
        self.output_schema = output;
        self
    }

    pub fn with_raw_content(mut self, raw: String) -> Self {
        self.raw_content = raw;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_code_blocks(mut self, blocks: Vec<CodeBlock>) -> Self {
        self.code_blocks = blocks;
        self
    }

    pub fn with_code_blocks_from_content(mut self, content: &str) -> Self {
        let (_, blocks) = ContentPreprocessor::extract_code_blocks(content);
        self.code_blocks = blocks;
        self
    }
}

/// Result of decomposing a composite skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    pub atomic_skills: Vec<AtomicSkill>,
    pub tool_dependencies: Vec<ToolDependency>,
    pub workflow_nodes: serde_json::Value,
    pub workflow_edges: serde_json::Value,
    pub original_source: CompositeSourceInfo,
    pub original_content: String,
    pub parsed_steps_metadata: Vec<StepMetadata>,
    pub code_blocks: Vec<CodeBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMetadata {
    pub index: usize,
    pub title: String,
    pub raw_content: String,
    pub has_then_branch: bool,
    pub has_else_branch: bool,
    pub loop_body_steps_count: usize,
    pub input_schema_inferred: bool,
    pub output_schema_inferred: bool,
}

/// Source info for traceability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSourceInfo {
    pub market: String,
    pub repo: Option<String>,
    pub version: Option<String>,
}

/// Composite skill decomposer
pub struct SkillDecomposer;

impl SkillDecomposer {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(composite: &CompositeSkillData) -> Result<ParsedComposite, DecompositionError> {
        let (clean_content, code_blocks) =
            ContentPreprocessor::extract_code_blocks(&composite.content);
        let content = &clean_content;
        let lines: Vec<&str> = content.lines().collect();
        let mut steps = Vec::new();
        let mut is_fully_parsed = true;

        let mut current_step_lines: Vec<String> = Vec::new();
        let mut current_title: Option<String> = None;
        let mut current_raw: Vec<String> = Vec::new();
        let mut in_loop_body = false;
        let mut loop_body_indent: Option<usize> = None;
        let mut loop_body_lines: Vec<String> = Vec::new();

        let mut _in_condition_body = false;
        let mut then_lines: Vec<String> = Vec::new();
        let mut else_lines: Vec<String> = Vec::new();
        let mut in_then = false;
        let mut in_else = false;

        fn get_indent(line: &str) -> usize {
            line.len() - line.trim_start_matches(' ').len()
        }

        fn is_step_heading(line: &str) -> bool {
            let trimmed = line.trim();
            trimmed.starts_with("## ")
                || trimmed.starts_with("### ")
                || trimmed.starts_with("**") && trimmed.contains("**Step")
        }

        fn is_condition_heading(line: &str) -> bool {
            let lower = line.to_lowercase();
            lower.contains("if ")
                || lower.contains("check ")
                || lower.contains("condition")
                || lower.contains("branch")
        }

        fn is_loop_heading(line: &str) -> bool {
            let lower = line.to_lowercase();
            lower.contains("loop")
                || lower.contains("repeat")
                || lower.contains("for each")
                || lower.contains("while")
        }

        fn parse_step_from_lines(
            title: &str,
            lines: &[String],
            composite_name: &str,
        ) -> ParsedStep {
            let description = lines.join("\n").trim().to_string();
            let mut step = ParsedStep::new(title.to_string()).with_raw_content(lines.join("\n"));

            if let Some(tool_name) = SkillDecomposer::extract_tool_name(title) {
                step = step.with_tool(tool_name, SkillDecomposer::extract_tool_type(title));
            }

            if is_condition_heading(title) {
                let condition_expr =
                    SkillDecomposer::extract_condition_expression(title, &description)
                        .unwrap_or_else(|| title.to_string());
                step = step.with_condition(condition_expr);

                let (then_br, else_br) = SkillDecomposer::extract_condition_branches(&description);
                step = step.with_branches(then_br, else_br);
            } else if is_loop_heading(title) {
                let (items_var, max_iter) = SkillDecomposer::extract_loop_info(&description);
                step = step.with_loop(items_var, max_iter);

                let (body_raw, body_steps) =
                    SkillDecomposer::extract_loop_body(title, &description, composite_name);
                step = step.with_loop_body(body_raw, body_steps);
            }

            if step.is_parallel {
                let branches = SkillDecomposer::extract_parallel_branches(&description);
                step = step.with_parallel(branches);
            }

            let (input_schema, output_schema) =
                SkillDecomposer::infer_schema_from_description(&description);
            step = step.with_schema(input_schema, output_schema);

            step
        }

        for line in lines.iter() {
            let trimmed = line.trim();

            if is_step_heading(line) {
                if let Some(title) = current_title.take() {
                    let step_lines = if current_step_lines.is_empty() {
                        current_raw.clone()
                    } else {
                        std::mem::take(&mut current_step_lines)
                    };
                    let step = parse_step_from_lines(&title, &step_lines, &composite.name);
                    steps.push(step);
                    current_raw.clear();
                }

                current_title = Some(trimmed.trim_start_matches('#').trim().to_string());
                current_step_lines.push(line.to_string());
                current_raw.push(line.to_string());
                continue;
            }

            if let Some(ref title) = current_title {
                let lower_trimmed = trimmed.to_lowercase();

                if lower_trimmed.contains("then:") || lower_trimmed.contains("then\n") {
                    in_then = true;
                    in_else = false;
                    continue;
                }
                if lower_trimmed.contains("else:") || lower_trimmed.contains("else\n") {
                    in_else = true;
                    in_then = false;
                    continue;
                }

                if in_then {
                    then_lines.push(line.to_string());
                } else if in_else {
                    else_lines.push(line.to_string());
                } else if in_loop_body {
                    if let Some(indent) = loop_body_indent {
                        let current_indent = get_indent(line);
                        if current_indent <= indent && !line.trim().is_empty() {
                            in_loop_body = false;
                            loop_body_indent = None;
                            let body_step = parse_step_from_lines(
                                &format!("{} (loop body)", title),
                                &loop_body_lines,
                                &composite.name,
                            );
                            if let Some(ref mut last_step) = steps.last_mut() {
                                if last_step.is_loop {
                                    last_step.loop_body_steps.push(body_step);
                                }
                            }
                            loop_body_lines.clear();
                        } else {
                            loop_body_lines.push(line.to_string());
                        }
                    }
                } else if lower_trimmed.starts_with("- ") || lower_trimmed.starts_with("* ") {
                    if is_loop_heading(title) {
                        in_loop_body = true;
                        loop_body_indent = Some(get_indent(line));
                        loop_body_lines.push(line.to_string());
                    } else {
                        current_step_lines.push(line.to_string());
                        current_raw.push(line.to_string());
                    }
                } else {
                    current_step_lines.push(line.to_string());
                    current_raw.push(line.to_string());
                }
            }
        }

        if let Some(title) = current_title.take() {
            let step_lines = if current_step_lines.is_empty() {
                current_raw.clone()
            } else {
                std::mem::take(&mut current_step_lines)
            };
            let mut step = parse_step_from_lines(&title, &step_lines, &composite.name);

            if in_then || in_else || !then_lines.is_empty() || !else_lines.is_empty() {
                step = step.with_branches(
                    if then_lines.is_empty() {
                        None
                    } else {
                        Some(then_lines.join("\n"))
                    },
                    if else_lines.is_empty() {
                        None
                    } else {
                        Some(else_lines.join("\n"))
                    },
                );
            }

            if in_loop_body && !loop_body_lines.is_empty() {
                let body_step = parse_step_from_lines(
                    &format!("{} (loop body)", title),
                    &loop_body_lines,
                    &composite.name,
                );
                if step.is_loop {
                    step.loop_body_steps.push(body_step);
                }
            }

            steps.push(step);
        }

        if steps.is_empty() {
            is_fully_parsed = false;
            let (input_schema, output_schema) =
                Self::infer_schema_from_description(&composite.description);
            steps.push(
                ParsedStep::new(composite.name.clone())
                    .with_schema(input_schema, output_schema)
                    .with_raw_content(composite.content.clone()),
            );
        }

        Ok(ParsedComposite {
            name: composite.name.clone(),
            description: composite.description.clone(),
            source: composite.source.clone(),
            version: composite.version.clone(),
            repo: composite.repo.clone(),
            steps,
            is_fully_parsed,
            code_blocks,
        })
    }

    /// Decompose a parsed composite into atomic skills, tool dependencies,
    /// and a workflow definition.
    pub fn decompose(
        parsed: &ParsedComposite,
        existing_skills: &[AtomicSkill],
    ) -> Result<DecompositionResult, DecompositionError> {
        let mut atomic_skills = Vec::new();
        let mut tool_dependencies = Vec::new();
        let mut workflow_nodes = Vec::new();
        let mut workflow_edges = Vec::new();

        for (i, step) in parsed.steps.iter().enumerate() {
            let node_id = format!("node_{}", i);

            if !step.code_blocks.is_empty() {
                for (block_idx, block) in step.code_blocks.iter().enumerate() {
                    let block_node_id = format!("{}_block_{}", node_id, block_idx);

                    if block.is_script() {
                        let deps = block.infer_dependencies();
                        let deps_for_node = deps.clone();
                        for dep in &deps {
                            if !tool_dependencies
                                .iter()
                                .any(|t: &ToolDependency| t.name == *dep)
                            {
                                tool_dependencies.push(ToolDependency {
                                    name: dep.clone(),
                                    tool_type: "script_dependency".to_string(),
                                    source_info: Some(format!("from {} script in step {}", block.language.clone().unwrap_or_default(), step.title)),
                                    status: super::tool_resolver::ToolDependencyStatus::ManualInstallable,
                                });
                            }
                        }

                        let input_schema = block.infer_schema();

                        let skill = AtomicSkill {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: format!(
                                "script_{}_{}_{}",
                                slugify(&step.title),
                                block_idx,
                                block.language.clone().unwrap_or_else(|| "code".to_string())
                            ),
                            description: format!(
                                "Script from step '{}' (language: {:?})\n\n```{}...\n```",
                                step.title,
                                block.language,
                                &block.content.chars().take(50).collect::<String>()
                            ),
                            input_schema,
                            output_schema: None,
                            entry_type: crate::atomic_skill::types::EntryType::Builtin,
                            entry_ref: format!("inline_script_{}", block_idx),
                            category: "inline_script".to_string(),
                            tags: vec![
                                "inline_script".to_string(),
                                block
                                    .language
                                    .clone()
                                    .unwrap_or_else(|| "unknown".to_string()),
                            ],
                            version: "1.0.0".to_string(),
                            enabled: true,
                            source: "inline".to_string(),
                            created_at: chrono::Utc::now().timestamp_millis(),
                            updated_at: chrono::Utc::now().timestamp_millis(),
                        };
                        let skill_id = skill.id.clone();
                        atomic_skills.push(skill);

                        workflow_nodes.push(serde_json::json!({
                            "id": block_node_id,
                            "type": "inline_script",
                            "data": {
                                "skill_id": skill_id,
                                "language": block.language,
                                "code_content": block.content,
                                "dependencies": deps_for_node,
                            }
                        }));

                        workflow_edges.push(serde_json::json!({
                            "id": format!("edge_{}_script_{}", i, block_idx),
                            "source": node_id.clone(),
                            "target": block_node_id,
                            "edge_type": "code_block",
                        }));
                    } else if block.is_config() {
                        if let Some(schema) = block.infer_schema() {
                            let config_node_id = format!("{}_config_{}", node_id, block_idx);
                            workflow_nodes.push(serde_json::json!({
                                "id": config_node_id,
                                "type": "config",
                                "data": {
                                    "language": block.language,
                                    "config_content": block.content,
                                    "schema": schema,
                                }
                            }));

                            workflow_edges.push(serde_json::json!({
                                "id": format!("edge_{}_config_{}", i, block_idx),
                                "source": node_id.clone(),
                                "target": config_node_id,
                                "edge_type": "config_block",
                            }));
                        }
                    }
                }
            }

            if !parsed.is_fully_parsed && i == 0 && parsed.steps.len() == 1 {
                // Cannot decompose: keep as a single atomic skill
                let skill = AtomicSkill {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("atomic_{}", slugify(&parsed.name)),
                    description: parsed.description.clone(),
                    input_schema: None,
                    output_schema: None,
                    entry_type: crate::atomic_skill::types::EntryType::Builtin,
                    entry_ref: "undecomposed".to_string(),
                    category: "undecomposed".to_string(),
                    tags: vec!["undecomposed".to_string()],
                    version: "1.0.0".to_string(),
                    enabled: true,
                    source: "atomic".to_string(),
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                };
                atomic_skills.push(skill);
                workflow_nodes.push(serde_json::json!({
                    "id": node_id,
                    "type": "atomic_skill",
                    "data": { "skill_id": atomic_skills.last().unwrap().id }
                }));
            } else if let Some(tool_name) = &step.tool_name {
                // Check for semantic duplicate
                let entry_type_str = step.tool_type.as_deref().unwrap_or("local");
                let entry_type = match entry_type_str {
                    "mcp" => crate::atomic_skill::types::EntryType::Mcp,
                    "plugin" => crate::atomic_skill::types::EntryType::Plugin,
                    "builtin" => crate::atomic_skill::types::EntryType::Builtin,
                    _ => crate::atomic_skill::types::EntryType::Local,
                };

                // Try to reuse existing skill
                let existing = existing_skills.iter().find(|s| {
                    s.entry_type == entry_type
                        && s.entry_ref == *tool_name
                        && s.input_schema == step.input_schema
                        && s.output_schema == step.output_schema
                });

                let skill_id = if let Some(existing) = existing {
                    existing.id.clone()
                } else {
                    let skill = AtomicSkill {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: format!("atomic_{}", slugify(tool_name)),
                        description: step.description.clone(),
                        input_schema: step.input_schema.clone(),
                        output_schema: step.output_schema.clone(),
                        entry_type: entry_type.clone(),
                        entry_ref: tool_name.clone(),
                        category: "decomposed".to_string(),
                        tags: vec!["decomposed".to_string()],
                        version: "1.0.0".to_string(),
                        enabled: true,
                        source: "atomic".to_string(),
                        created_at: chrono::Utc::now().timestamp_millis(),
                        updated_at: chrono::Utc::now().timestamp_millis(),
                    };
                    let id = skill.id.clone();
                    atomic_skills.push(skill);
                    id
                };

                // Add tool dependency
                tool_dependencies.push(ToolDependency {
                    name: tool_name.clone(),
                    tool_type: entry_type_str.to_string(),
                    source_info: None,
                    status: super::tool_resolver::ToolDependencyStatus::Satisfied,
                });

                workflow_nodes.push(serde_json::json!({
                    "id": node_id,
                    "type": "atomic_skill",
                    "data": { "skill_id": skill_id }
                }));
            } else if step.is_condition {
                let _condition_expr = step
                    .condition_expression
                    .clone()
                    .unwrap_or_else(|| step.title.clone());

                workflow_nodes.push(serde_json::json!({
                    "id": node_id,
                    "type": "condition",
                    "data": {
                        "title": step.title,
                        "config": {
                            "conditions": [{
                                "var_path": "result",
                                "operator": "isNotEmpty",
                                "value": serde_json::Value::Null
                            }],
                            "logical_op": "and"
                        }
                    }
                }));

                workflow_edges.push(serde_json::json!({
                    "id": format!("edge_{}_true", i),
                    "source": node_id.clone(),
                    "sourceHandle": "true",
                    "target": format!("node_{}", i + 1),
                    "edge_type": "conditionTrue",
                    "label": "True"
                }));

                if i + 1 < parsed.steps.len() {
                    workflow_edges.push(serde_json::json!({
                        "id": format!("edge_{}_false", i),
                        "source": node_id.clone(),
                        "sourceHandle": "false",
                        "target": format!("node_{}", i + 1),
                        "edge_type": "conditionFalse",
                        "label": "False"
                    }));
                }
            } else if step.is_loop {
                let loop_type = if step.loop_items_var.is_some() {
                    "forEach"
                } else {
                    "while"
                };
                let iteratee_var: Option<String> =
                    step.loop_items_var.as_ref().map(|_| "item".to_string());

                let loop_config = serde_json::json!({
                    "loop_type": loop_type,
                    "items_var": step.loop_items_var,
                    "iteratee_var": iteratee_var,
                    "max_iterations": step.max_iterations.unwrap_or(100),
                    "continue_on_error": false,
                    "body_steps": Vec::<String>::new()
                });

                workflow_nodes.push(serde_json::json!({
                    "id": node_id,
                    "type": "loop",
                    "data": {
                        "title": step.title,
                        "config": loop_config
                    }
                }));

                workflow_edges.push(serde_json::json!({
                    "id": format!("edge_{}_back", i),
                    "source": node_id.clone(),
                    "sourceHandle": "back",
                    "target": node_id.clone(),
                    "edge_type": "loopBack"
                }));
            } else if step.is_parallel {
                let branches: Vec<serde_json::Value> = step
                    .parallel_branches
                    .iter()
                    .map(|b| {
                        serde_json::json!({
                            "id": format!("{}_{}", node_id, slugify(&b.name)),
                            "title": b.name,
                            "steps": b.steps
                        })
                    })
                    .collect();

                let parallel_config = serde_json::json!({
                    "branches": branches,
                    "wait_for_all": true,
                    "timeout": 300
                });

                workflow_nodes.push(serde_json::json!({
                    "id": node_id,
                    "type": "parallel",
                    "data": {
                        "title": step.title,
                        "config": parallel_config
                    }
                }));

                for branch in &step.parallel_branches {
                    let branch_node_id = format!("{}_{}", node_id, slugify(&branch.name));
                    workflow_edges.push(serde_json::json!({
                        "id": format!("edge_{}_{}", i, slugify(&branch.name)),
                        "source": node_id.clone(),
                        "target": branch_node_id,
                        "edge_type": "parallelBranch"
                    }));
                }
            } else {
                // Generic step as atomic skill
                let skill = AtomicSkill {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("atomic_{}_{}", slugify(&parsed.name), i),
                    description: step.description.clone(),
                    input_schema: step.input_schema.clone(),
                    output_schema: step.output_schema.clone(),
                    entry_type: crate::atomic_skill::types::EntryType::Builtin,
                    entry_ref: format!("step_{}", i),
                    category: "decomposed".to_string(),
                    tags: vec!["decomposed".to_string()],
                    version: "1.0.0".to_string(),
                    enabled: true,
                    source: "atomic".to_string(),
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                };
                let skill_id = skill.id.clone();
                atomic_skills.push(skill);
                workflow_nodes.push(serde_json::json!({
                    "id": node_id,
                    "type": "atomic_skill",
                    "data": { "skill_id": skill_id }
                }));
            }

            // Add edge from previous node
            if i > 0 {
                workflow_edges.push(serde_json::json!({
                    "id": format!("edge_{}", i),
                    "source": format!("node_{}", i - 1),
                    "target": node_id,
                }));
            }
        }

        Ok(DecompositionResult {
            atomic_skills,
            tool_dependencies,
            workflow_nodes: serde_json::to_value(&workflow_nodes)
                .unwrap_or(serde_json::Value::Null),
            workflow_edges: serde_json::to_value(&workflow_edges)
                .unwrap_or(serde_json::Value::Null),
            original_source: CompositeSourceInfo {
                market: parsed.source.clone(),
                repo: parsed.repo.clone(),
                version: parsed.version.clone(),
            },
            original_content: parsed
                .steps
                .iter()
                .map(|s| s.raw_content.clone())
                .collect::<Vec<_>>()
                .join("\n\n"),
            parsed_steps_metadata: parsed
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| StepMetadata {
                    index: i,
                    title: s.title.clone(),
                    raw_content: s.raw_content.clone(),
                    has_then_branch: s.then_branch.is_some(),
                    has_else_branch: s.else_branch.is_some(),
                    loop_body_steps_count: s.loop_body_steps.len(),
                    input_schema_inferred: s.input_schema.is_some(),
                    output_schema_inferred: s.output_schema.is_some(),
                })
                .collect(),
            code_blocks: parsed.code_blocks.clone(),
        })
    }

    fn extract_tool_name(title: &str) -> Option<String> {
        let lower = title.to_lowercase();
        // Patterns: "Use X tool", "Call X", "Run X", "Execute X"
        for prefix in &["use ", "call ", "run ", "execute "] {
            if lower.starts_with(prefix) {
                let rest = &title[prefix.len()..];
                // Take first word as tool name
                if let Some(name) = rest.split_whitespace().next() {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    fn extract_tool_type(title: &str) -> Option<String> {
        let lower = title.to_lowercase();
        if lower.contains("mcp") {
            Some("mcp".to_string())
        } else if lower.contains("plugin") {
            Some("plugin".to_string())
        } else if lower.contains("builtin") || lower.contains("built-in") {
            Some("builtin".to_string())
        } else {
            None
        }
    }

    fn extract_condition_expression(title: &str, description: &str) -> Option<String> {
        let combined = format!("{} {}", title, description);

        let if_regex = regex::Regex::new(r"(?i)if\s+(.+?)(?:\s+then|\s*[:.])").ok()?;
        if let Some(caps) = if_regex.captures(&combined) {
            return Some(caps.get(1)?.as_str().trim().to_string());
        }

        let check_regex =
            regex::Regex::new(r"(?i)check\s+(?:whether\s+)?(.+?)(?:\s+and|\s*[:.])").ok()?;
        if let Some(caps) = check_regex.captures(&combined) {
            return Some(caps.get(1)?.as_str().trim().to_string());
        }

        let condition_in_brackets = regex::Regex::new(r"(?i)\[condition:\s*(.+?)\]").ok()?;
        if let Some(caps) = condition_in_brackets.captures(&combined) {
            return Some(caps.get(1)?.as_str().trim().to_string());
        }

        None
    }

    fn extract_loop_info(description: &str) -> (Option<String>, Option<u32>) {
        let Ok(for_each_regex) = regex::Regex::new(r"(?i)for\s+each\s+(\w+)(?:\s+in|\s*:)") else {
            return (None, None);
        };
        if let Some(caps) = for_each_regex.captures(description) {
            let items_var = caps.get(1).map(|m| m.as_str().to_string());
            let max_iter = Self::extract_max_iterations(description);
            return (items_var, max_iter);
        }

        let Ok(while_regex) = regex::Regex::new(r"(?i)while\s+(\w+)(?:\s+<|\s+>|\s+==)") else {
            return (None, None);
        };
        if while_regex.is_match(description) {
            return (None, None);
        }

        let max_iter = Self::extract_max_iterations(description);
        (None, max_iter)
    }

    fn extract_max_iterations(description: &str) -> Option<u32> {
        let max_iter_regex =
            regex::Regex::new(r"(?i)max(?:imum)?\s*iterations?\s*[:=]?\s*(\d+)").ok()?;
        let caps = max_iter_regex.captures(description)?;
        caps.get(1).and_then(|m| m.as_str().parse().ok())
    }

    fn infer_schema_from_description(
        description: &str,
    ) -> (Option<serde_json::Value>, Option<serde_json::Value>) {
        let input_schema = Self::extract_schema_from_text(description, "input");
        let output_schema = Self::extract_schema_from_text(description, "output");
        (input_schema, output_schema)
    }

    fn extract_schema_from_text(text: &str, schema_type: &str) -> Option<serde_json::Value> {
        let pattern = format!("(?i){}\\s*schema\\s*[:=]?\\s*\\{{(.+?)\\}}", schema_type);
        let schema_regex = regex::Regex::new(&pattern).ok()?;
        let caps = schema_regex.captures(text)?;
        let schema_content = caps.get(1)?.as_str();

        let properties_regex = regex::Regex::new(r#""(\w+)"\s*:\s*"([^"]+)""#).ok()?;
        let mut properties = serde_json::Map::new();
        for caps in properties_regex.captures_iter(schema_content) {
            let key = caps.get(1)?.as_str().to_string();
            let value_type = caps.get(2)?.as_str();
            let value = match value_type {
                "string" => serde_json::Value::String("".to_string()),
                "number" | "integer" => serde_json::json!(0),
                "boolean" => serde_json::Value::Bool(false),
                "array" => serde_json::Value::Array(vec![]),
                "object" => serde_json::Value::Object(serde_json::Map::new()),
                _ => serde_json::Value::Null,
            };
            properties.insert(key, value);
        }

        if properties.is_empty() {
            return None;
        }

        Some(serde_json::json!({
            "type": "object",
            "properties": properties
        }))
    }

    fn extract_parallel_branches(description: &str) -> Vec<ParallelBranch> {
        let Ok(branch_regex) =
            regex::Regex::new(r"(?i)branch\s+(\w+)\s*[:]?\s*([\s\S]*?)(?:\n\n|\nbranch|$)")
        else {
            return Vec::new();
        };
        let mut branches = Vec::new();

        for caps in branch_regex.captures_iter(description) {
            let name = match caps.get(1) {
                Some(m) => m.as_str().to_string(),
                None => continue,
            };
            let steps_str = match caps.get(2) {
                Some(m) => m.as_str(),
                None => continue,
            };
            let steps: Vec<String> = steps_str
                .split([',', '\n'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if !steps.is_empty() {
                branches.push(ParallelBranch {
                    name,
                    steps,
                    raw_content: None,
                });
            }
        }

        branches
    }

    fn extract_condition_branches(description: &str) -> (Option<String>, Option<String>) {
        let then_regex =
            match regex::Regex::new(r"(?i)then\s*[:.]?\s*([\s\S]*?)(?:\s*else\s*[:.]|$)") {
                Ok(r) => r,
                Err(_) => return (None, None),
            };
        let else_regex =
            match regex::Regex::new(r"(?i)else\s*[:.]?\s*([\s\S]*?)(?:\s*(?:endif|end\s+if)\s*|$)")
            {
                Ok(r) => r,
                Err(_) => return (None, None),
            };

        let then_content = then_regex
            .captures(description)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()));

        let else_content = else_regex
            .captures(description)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()));

        (then_content, else_content)
    }

    fn extract_loop_body(
        title: &str,
        description: &str,
        _composite_name: &str,
    ) -> (String, Vec<ParsedStep>) {
        let for_each_body_regex =
            regex::Regex::new(r"(?i)for\s+each\s+\w+\s+in\s+.+?\s*[:.]\s*([\s\S]*?)(?:\n\n|$)")
                .ok();

        let while_body_regex =
            regex::Regex::new(r"(?i)while\s+.+?\s*[:.]\s*([\s\S]*?)(?:\n\n|$)").ok();

        let loop_start_regex =
            regex::Regex::new(r"(?i)(?:do|repeat|loop)\s*[:.]\s*([\s\S]*?)(?:\n\n|$)").ok();

        let body_text = for_each_body_regex
            .and_then(|r| r.captures(description))
            .or_else(|| while_body_regex.and_then(|r| r.captures(description)))
            .or_else(|| loop_start_regex.and_then(|r| r.captures(description)))
            .map(|caps| {
                caps.get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let body_lines: Vec<String> = description
            .lines()
            .skip_while(|line| {
                let lower = line.to_lowercase();
                !lower.contains("do:")
                    && !lower.contains("repeat:")
                    && !lower.contains("loop:")
                    && !lower.contains(":")
            })
            .skip(1)
            .map(|l| l.to_string())
            .collect();

        let body_raw = if body_text.is_empty() {
            body_lines.join("\n")
        } else {
            body_text
        };

        let sub_steps = Self::parse_loop_body_steps(&body_raw, title);

        (body_raw, sub_steps)
    }

    fn parse_loop_body_steps(body_raw: &str, parent_title: &str) -> Vec<ParsedStep> {
        let lines: Vec<&str> = body_raw.lines().collect();
        let mut steps = Vec::new();
        let mut current_lines: Vec<String> = Vec::new();
        let mut current_subtitle: Option<String> = None;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("• ")
            {
                let item_text = trimmed
                    .trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim_start_matches('•')
                    .trim();

                if item_text.to_lowercase().contains("use ")
                    || item_text.to_lowercase().contains("call ")
                    || item_text.to_lowercase().contains("run ")
                {
                    if let Some(title) = current_subtitle.take() {
                        let step = ParsedStep::new(title)
                            .with_raw_content(current_lines.join("\n"))
                            .with_description(current_lines.join("\n"));
                        steps.push(step);
                        current_lines.clear();
                    }
                    current_subtitle = Some(format!("{} - {}", parent_title, item_text));
                    current_lines.push(item_text.to_string());
                } else {
                    current_lines.push(item_text.to_string());
                }
            } else if !trimmed.is_empty() && !current_lines.is_empty() {
                current_lines.push(trimmed.to_string());
            }
        }

        if let Some(title) = current_subtitle.take() {
            let step = ParsedStep::new(title)
                .with_raw_content(current_lines.join("\n"))
                .with_description(current_lines.join("\n"));
            steps.push(step);
        }

        steps
    }

    pub fn parse_with_fallback(
        composite: &CompositeSkillData,
        llm_parser: Option<&dyn crate::skill_decomposition::llm_assisted::LlmAssistedParser>,
        llm_request: Option<&crate::skill_decomposition::llm_assisted::LlmParseRequest>,
    ) -> Result<ParsedComposite, DecompositionError> {
        let mut parsed = Self::parse(composite)?;

        let steps_needing_augmentation: Vec<usize> = parsed
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| Self::step_needs_llm_augmentation(step))
            .map(|(i, _)| i)
            .collect();

        if steps_needing_augmentation.is_empty() {
            return Ok(parsed);
        }

        if let (Some(parser), Some(request)) = (llm_parser, llm_request) {
            let rt = tokio::runtime::Runtime::new().map_err(|e| DecompositionError {
                message: e.to_string(),
            })?;
            let llm_response = rt
                .block_on(parser.parse_with_llm(request))
                .map_err(|e| DecompositionError { message: e })?;

            for (llm_step, &original_idx) in llm_response
                .steps
                .iter()
                .zip(steps_needing_augmentation.iter())
                .take(
                    llm_response
                        .steps
                        .len()
                        .min(steps_needing_augmentation.len()),
                )
            {
                if original_idx < parsed.steps.len() {
                    parsed.steps[original_idx] = Self::merge_llm_step(
                        std::mem::replace(
                            &mut parsed.steps[original_idx],
                            ParsedStep::new(llm_step.title.clone()),
                        ),
                        llm_step,
                    );
                }
            }

            parsed.is_fully_parsed = true;
        }

        Ok(parsed)
    }

    fn step_needs_llm_augmentation(step: &ParsedStep) -> bool {
        if !step.description.is_empty() && step.description.len() < 20 {
            return true;
        }

        if step.is_condition {
            return step.condition_expression.is_none()
                || (step.then_branch.is_none() && step.else_branch.is_none());
        }

        if step.is_loop {
            return step.loop_items_var.is_none() && step.loop_body_raw.is_none();
        }

        if step.is_parallel && step.parallel_branches.is_empty() {
            return true;
        }

        if step.input_schema.is_none() && step.description.len() > 50 {
            return true;
        }

        false
    }

    fn merge_llm_step(
        original: ParsedStep,
        llm_step: &crate::skill_decomposition::llm_assisted::LlmParsedStep,
    ) -> ParsedStep {
        use crate::skill_decomposition::llm_assisted::StepType;

        let mut step = ParsedStep::new(llm_step.title.clone())
            .with_description(llm_step.description.clone())
            .with_raw_content(llm_step.raw_content.clone());

        if let Some(tool_name) = &llm_step.tool_name {
            step = step.with_tool(tool_name.clone(), llm_step.tool_type.clone());
        }

        match llm_step.step_type {
            StepType::Condition => {
                let expr = llm_step.condition_expression.clone().unwrap_or_default();
                step = step.with_condition(expr);
                step =
                    step.with_branches(llm_step.then_branch.clone(), llm_step.else_branch.clone());
            },
            StepType::Loop => {
                step = step.with_loop(llm_step.loop_items_var.clone(), llm_step.max_iterations);
                if let Some(body) = &llm_step.loop_body {
                    let body_steps: Vec<ParsedStep> = body
                        .iter()
                        .map(|bs| {
                            ParsedStep::new(bs.title.clone())
                                .with_description(bs.description.clone())
                                .with_raw_content(bs.raw_content.clone())
                        })
                        .collect();
                    step = step.with_loop_body(
                        body_steps
                            .iter()
                            .map(|s| s.raw_content.clone())
                            .collect::<Vec<_>>()
                            .join("\n"),
                        body_steps,
                    );
                }
            },
            StepType::Parallel => {
                if let Some(branches) = &llm_step.parallel_branches {
                    let pb: Vec<ParallelBranch> = branches
                        .iter()
                        .map(|b| ParallelBranch {
                            name: b.name.clone(),
                            steps: b.steps.clone(),
                            raw_content: b.raw_content.clone(),
                        })
                        .collect();
                    step = step.with_parallel(pb);
                }
            },
            StepType::Atomic | StepType::Generic => {},
        }

        step = step.with_schema(
            llm_step.input_schema.clone(),
            llm_step.output_schema.clone(),
        );

        if original.description.is_empty() && !step.description.is_empty() {
            step.description = original.description;
        }
        if original.raw_content.is_empty() && !step.raw_content.is_empty() {
            step.raw_content = original.raw_content;
        }

        step
    }
}

impl Default for SkillDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

/// Decomposition error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionError {
    pub message: String,
}

impl std::fmt::Display for DecompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecompositionError: {}", self.message)
    }
}

impl std::error::Error for DecompositionError {}

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
