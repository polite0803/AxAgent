use crate::skill::{HermesMetadata, Skill, SkillMetadata};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesSkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub commands: Vec<HermesCommand>,
    pub triggers: Vec<String>,
    pub examples: Vec<HermesExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesCommand {
    pub name: String,
    pub description: String,
    pub syntax: String,
    pub parameters: Vec<HermesParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesExample {
    pub input: String,
    pub output: String,
    pub description: Option<String>,
}

pub struct SkillsHubAdapter {
    manifest: Option<HermesSkillManifest>,
}

impl Default for SkillsHubAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillsHubAdapter {
    pub fn new() -> Self {
        Self { manifest: None }
    }

    pub fn parse_hermes_manifest(&mut self, content: &str) -> Result<(), String> {
        self.manifest = Some(
            serde_json::from_str(content)
                .map_err(|e| format!("Failed to parse manifest JSON: {}", e))?,
        );
        Ok(())
    }

    pub fn parse_hermes_skill_md(&mut self, content: &str) -> Result<(), String> {
        let manifest = Self::extract_frontmatter(content)?;
        self.manifest = Some(
            serde_json::from_str(&manifest)
                .map_err(|e| format!("Failed to parse manifest: {}", e))?,
        );
        Ok(())
    }

    fn extract_frontmatter(content: &str) -> Result<String, String> {
        let trimmed = content.trim();
        if !trimmed.starts_with("---") {
            return Err("No frontmatter found".to_string());
        }
        let end = trimmed[3..]
            .find("---")
            .ok_or("Frontmatter not closed")?;
        Ok(trimmed[3..3 + end].trim().to_string())
    }

    pub fn to_axagent_skill(&self) -> Result<Skill, String> {
        let manifest = self.manifest.as_ref().ok_or("No manifest loaded")?;
        let triggers_str = manifest.triggers.join("\n");
        let instruction = Self::generate_instruction(manifest);
        Ok(Skill {
            id: format!("{}-{}", manifest.name.to_lowercase().replace(' ', "-"), manifest.version),
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            version: manifest.version.clone(),
            content: instruction,
            category: manifest.category.clone(),
            tags: manifest.tags.clone(),
            platforms: vec![],
            scenarios: vec![triggers_str],
            quality_score: 0.0,
            success_rate: 0.0,
            avg_execution_time_ms: 0,
            total_usages: 0,
            successful_usages: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_used_at: None,
            metadata: SkillMetadata {
                hermes: HermesMetadata {
                    tags: manifest.tags.clone(),
                    category: manifest.category.clone(),
                    fallback_for_toolsets: vec![],
                    requires_toolsets: vec![],
                    config: vec![],
                    source_kind: Some("skills_hub".to_string()),
                    source_ref: None,
                    commit: None,
                    skill_dependencies: None,
                },
                references: vec![],
            },
        })
    }

    fn generate_instruction(manifest: &HermesSkillManifest) -> String {
        let mut instruction = format!("# {}\n\n{}\n\n## Commands\n\n", manifest.name, manifest.description);
        for cmd in &manifest.commands {
            instruction.push_str(&format!("### /{}\n\n{}\n\n", cmd.name, cmd.description));
            if !cmd.parameters.is_empty() {
                instruction.push_str("| Parameter | Type | Required | Description |\n");
                instruction.push_str("|-----------|------|----------|-------------|\n");
                for param in &cmd.parameters {
                    instruction.push_str(&format!(
                        "| {} | {} | {} | {} |\n",
                        param.name, param.param_type, param.required, param.description
                    ));
                }
                instruction.push('\n');
            }
        }
        if !manifest.examples.is_empty() {
            instruction.push_str("## Examples\n\n");
            for example in &manifest.examples {
                instruction.push_str(&format!("**Input:** {}\n\n", example.input));
                if let Some(desc) = &example.description {
                    instruction.push_str(&format!("**Description:** {}\n\n", desc));
                }
            }
        }
        instruction
    }

    pub fn from_axagent_skill(skill: &Skill) -> HermesSkillManifest {
        let commands = skill
            .scenarios
            .iter()
            .filter_map(|t| {
                if t.starts_with('/') {
                    Some(HermesCommand {
                        name: t.trim_start_matches('/').to_string(),
                        description: skill.description.clone(),
                        syntax: t.clone(),
                        parameters: vec![],
                    })
                } else {
                    None
                }
            })
            .collect();
        HermesSkillManifest {
            name: skill.name.clone(),
            version: skill.version.clone(),
            description: skill.description.clone(),
            author: None,
            category: skill.category.clone(),
            tags: skill.tags.clone(),
            commands,
            triggers: skill.scenarios.clone(),
            examples: vec![],
        }
    }

    pub fn to_hermes_md(&self) -> Result<String, String> {
        let manifest = self.manifest.as_ref().ok_or("No manifest loaded")?;
        let yaml = serde_yaml::to_string(manifest).map_err(|e| e.to_string())?;
        let content = Self::generate_instruction(manifest);
        Ok(format!("---\n{}...\n\n{}", yaml, content))
    }
}
