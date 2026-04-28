use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemory {
    pub project_path: String,
    pub conventions: Vec<String>,
    pub architecture_notes: Vec<String>,
    pub common_commands: Vec<String>,
    pub tech_stack: Vec<String>,
    pub user_preferences: Vec<String>,
}

impl ProjectMemory {
    const MEMORY_FILE: &'static str = ".axagent/memory.md";

    pub async fn load(project_path: &str) -> Result<Option<Self>, String> {
        let memory_path = PathBuf::from(project_path).join(Self::MEMORY_FILE);
        if !memory_path.exists() {
            return Ok(None);
        }
        let content =
            tokio::fs::read_to_string(&memory_path).await.map_err(|e| e.to_string())?;
        Ok(Some(Self::parse_from_markdown(&content, project_path)))
    }

    pub async fn save(&self) -> Result<(), String> {
        let memory_path = PathBuf::from(&self.project_path).join(Self::MEMORY_FILE);
        if let Some(parent) = memory_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        let content = self.to_markdown();
        tokio::fs::write(&memory_path, content).await.map_err(|e| e.to_string())
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Project Memory\n\n");
        if !self.tech_stack.is_empty() {
            md.push_str("## Tech Stack\n");
            for item in &self.tech_stack {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }
        if !self.conventions.is_empty() {
            md.push_str("## Conventions\n");
            for item in &self.conventions {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }
        if !self.common_commands.is_empty() {
            md.push_str("## Common Commands\n");
            for item in &self.common_commands {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }
        if !self.architecture_notes.is_empty() {
            md.push_str("## Architecture\n");
            for item in &self.architecture_notes {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }
        if !self.user_preferences.is_empty() {
            md.push_str("## User Preferences\n");
            for item in &self.user_preferences {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }
        md
    }

    pub fn parse_from_markdown(content: &str, project_path: &str) -> Self {
        let mut memory = Self {
            project_path: project_path.into(),
            conventions: vec![],
            architecture_notes: vec![],
            common_commands: vec![],
            tech_stack: vec![],
            user_preferences: vec![],
        };
        let mut current_section = "";
        for line in content.lines() {
            if line.starts_with("## ") {
                current_section = line.trim_start_matches("## ").trim();
            } else if line.starts_with("- ") {
                let item = line.trim_start_matches("- ").trim().to_string();
                match current_section {
                    "Tech Stack" => memory.tech_stack.push(item),
                    "Conventions" => memory.conventions.push(item),
                    "Common Commands" => memory.common_commands.push(item),
                    "Architecture" => memory.architecture_notes.push(item),
                    "User Preferences" => memory.user_preferences.push(item),
                    _ => {}
                }
            }
        }
        memory
    }
}
