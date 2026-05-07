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
        let content = tokio::fs::read_to_string(&memory_path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Some(Self::parse_from_markdown(&content, project_path)))
    }

    pub async fn save(&self) -> Result<(), String> {
        let memory_path = PathBuf::from(&self.project_path).join(Self::MEMORY_FILE);
        if let Some(parent) = memory_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        let content = self.to_markdown();
        tokio::fs::write(&memory_path, content)
            .await
            .map_err(|e| e.to_string())
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
                    _ => {},
                }
            }
        }
        memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_memory_to_markdown_empty() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec![],
            architecture_notes: vec![],
            common_commands: vec![],
            tech_stack: vec![],
            user_preferences: vec![],
        };
        let md = memory.to_markdown();
        assert!(md.contains("# Project Memory"));
        assert!(!md.contains("## Tech Stack"));
        assert!(!md.contains("## Conventions"));
    }

    #[test]
    fn test_project_memory_to_markdown_with_tech_stack() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec![],
            architecture_notes: vec![],
            common_commands: vec![],
            tech_stack: vec!["Rust".to_string(), "Tauri".to_string()],
            user_preferences: vec![],
        };
        let md = memory.to_markdown();
        assert!(md.contains("## Tech Stack"));
        assert!(md.contains("- Rust"));
        assert!(md.contains("- Tauri"));
    }

    #[test]
    fn test_project_memory_to_markdown_with_conventions() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec!["Use tabs".to_string()],
            architecture_notes: vec![],
            common_commands: vec![],
            tech_stack: vec![],
            user_preferences: vec![],
        };
        let md = memory.to_markdown();
        assert!(md.contains("## Conventions"));
        assert!(md.contains("- Use tabs"));
    }

    #[test]
    fn test_project_memory_to_markdown_with_common_commands() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec![],
            architecture_notes: vec![],
            common_commands: vec!["cargo build".to_string()],
            tech_stack: vec![],
            user_preferences: vec![],
        };
        let md = memory.to_markdown();
        assert!(md.contains("## Common Commands"));
        assert!(md.contains("- cargo build"));
    }

    #[test]
    fn test_project_memory_to_markdown_with_architecture() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec![],
            architecture_notes: vec!["Modular design".to_string()],
            common_commands: vec![],
            tech_stack: vec![],
            user_preferences: vec![],
        };
        let md = memory.to_markdown();
        assert!(md.contains("## Architecture"));
        assert!(md.contains("- Modular design"));
    }

    #[test]
    fn test_project_memory_to_markdown_with_user_preferences() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec![],
            architecture_notes: vec![],
            common_commands: vec![],
            tech_stack: vec![],
            user_preferences: vec!["Dark mode".to_string()],
        };
        let md = memory.to_markdown();
        assert!(md.contains("## User Preferences"));
        assert!(md.contains("- Dark mode"));
    }

    #[test]
    fn test_project_memory_to_markdown_all_sections() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec!["Use tabs".to_string()],
            architecture_notes: vec!["Modular".to_string()],
            common_commands: vec!["cargo test".to_string()],
            tech_stack: vec!["Rust".to_string()],
            user_preferences: vec!["Dark mode".to_string()],
        };
        let md = memory.to_markdown();
        assert!(md.contains("## Tech Stack"));
        assert!(md.contains("## Conventions"));
        assert!(md.contains("## Common Commands"));
        assert!(md.contains("## Architecture"));
        assert!(md.contains("## User Preferences"));
    }

    #[test]
    fn test_project_memory_parse_from_markdown_basic() {
        let content = "# Project Memory\n\n## Tech Stack\n- Rust\n- Tauri\n\n## Conventions\n- Use tabs\n";
        let memory = ProjectMemory::parse_from_markdown(content, "/test");
        assert_eq!(memory.project_path, "/test");
        assert_eq!(memory.tech_stack, vec!["Rust", "Tauri"]);
        assert_eq!(memory.conventions, vec!["Use tabs"]);
    }

    #[test]
    fn test_project_memory_parse_from_markdown_all_sections() {
        let content = "# Project Memory\n\n## Tech Stack\n- Rust\n\n## Conventions\n- Use tabs\n\n## Common Commands\n- cargo build\n\n## Architecture\n- Modular\n\n## User Preferences\n- Dark mode\n";
        let memory = ProjectMemory::parse_from_markdown(content, "/test");
        assert_eq!(memory.tech_stack, vec!["Rust"]);
        assert_eq!(memory.conventions, vec!["Use tabs"]);
        assert_eq!(memory.common_commands, vec!["cargo build"]);
        assert_eq!(memory.architecture_notes, vec!["Modular"]);
        assert_eq!(memory.user_preferences, vec!["Dark mode"]);
    }

    #[test]
    fn test_project_memory_parse_from_markdown_empty() {
        let content = "# Project Memory\n";
        let memory = ProjectMemory::parse_from_markdown(content, "/test");
        assert!(memory.tech_stack.is_empty());
        assert!(memory.conventions.is_empty());
        assert!(memory.common_commands.is_empty());
        assert!(memory.architecture_notes.is_empty());
        assert!(memory.user_preferences.is_empty());
    }

    #[test]
    fn test_project_memory_roundtrip() {
        let original = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec!["Use tabs".to_string()],
            architecture_notes: vec!["Modular design".to_string()],
            common_commands: vec!["cargo test".to_string()],
            tech_stack: vec!["Rust".to_string(), "Tauri".to_string()],
            user_preferences: vec!["Dark mode".to_string()],
        };
        let md = original.to_markdown();
        let parsed = ProjectMemory::parse_from_markdown(&md, "/test");
        assert_eq!(parsed.tech_stack, original.tech_stack);
        assert_eq!(parsed.conventions, original.conventions);
        assert_eq!(parsed.common_commands, original.common_commands);
        assert_eq!(parsed.architecture_notes, original.architecture_notes);
        assert_eq!(parsed.user_preferences, original.user_preferences);
    }

    #[test]
    fn test_project_memory_parse_unknown_section_ignored() {
        let content = "# Project Memory\n\n## Unknown Section\n- should be ignored\n\n## Tech Stack\n- Rust\n";
        let memory = ProjectMemory::parse_from_markdown(content, "/test");
        assert_eq!(memory.tech_stack, vec!["Rust"]);
        assert!(memory.conventions.is_empty());
    }

    #[test]
    fn test_project_memory_serialization() {
        let memory = ProjectMemory {
            project_path: "/test".to_string(),
            conventions: vec!["Use tabs".to_string()],
            architecture_notes: vec![],
            common_commands: vec!["cargo build".to_string()],
            tech_stack: vec!["Rust".to_string()],
            user_preferences: vec![],
        };
        let json = serde_json::to_string(&memory).unwrap();
        let deserialized: ProjectMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.project_path, "/test");
        assert_eq!(deserialized.tech_stack, vec!["Rust"]);
        assert_eq!(deserialized.conventions, vec!["Use tabs"]);
        assert_eq!(deserialized.common_commands, vec!["cargo build"]);
    }

    #[test]
    fn test_project_memory_memory_file_constant() {
        assert_eq!(ProjectMemory::MEMORY_FILE, ".axagent/memory.md");
    }
}
