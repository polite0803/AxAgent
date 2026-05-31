use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Instant;

const BUILTIN_COMMANDS: &[&str] = &[
    "new",
    "model",
    "personality",
    "theme",
    "tools",
    "compress",
    "usage",
    "stop",
    "skills",
    "bundles",
    "diagnose",
];

static BUNDLE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("skill-bundles")
});

static ACTIVE_PERSONALITY_FILE: LazyLock<PathBuf> = LazyLock::new(|| {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
        .join("personalities")
        .join(".active")
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SlashCommandAction {
    LoadBundle { name: String, args: String },
    LoadSkill { name: String, args: String },
    SwitchPersonality { name: String },
    BuiltIn { command: String, args: String },
    Unknown,
}

pub struct SlashCommandRouter;

impl SlashCommandRouter {
    pub fn process(text: &str) -> Option<SlashCommandAction> {
        process_slash_command(text)
    }
}

pub fn process_slash_command(text: &str) -> Option<SlashCommandAction> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let without_slash = &trimmed[1..];
    if without_slash.is_empty() {
        return None;
    }

    let mut parts = without_slash.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("").trim().to_string();

    if command.is_empty() {
        return None;
    }

    if BUILTIN_COMMANDS.contains(&command) {
        return Some(SlashCommandAction::BuiltIn {
            command: command.to_string(),
            args,
        });
    }

    if is_bundle_name(command) {
        return Some(SlashCommandAction::LoadBundle {
            name: command.to_string(),
            args,
        });
    }

    if is_skill_name(command) {
        return Some(SlashCommandAction::LoadSkill {
            name: command.to_string(),
            args,
        });
    }

    if is_personality_name(command) {
        return Some(SlashCommandAction::SwitchPersonality {
            name: command.to_string(),
        });
    }

    Some(SlashCommandAction::Unknown)
}

static BUNDLE_NAMES_CACHE: LazyLock<std::sync::Mutex<Option<(Vec<String>, Instant)>>> =
    LazyLock::new(|| std::sync::Mutex::new(None));

const BUNDLE_CACHE_TTL_SECS: u64 = 60;

fn is_bundle_name(name: &str) -> bool {
    let slug = name.to_lowercase().replace(' ', "-");
    let yaml_path = BUNDLE_DIR.join(format!("{}.yaml", slug));
    let yml_path = BUNDLE_DIR.join(format!("{}.yml", slug));
    if yaml_path.exists() || yml_path.exists() {
        return true;
    }
    let names = get_cached_bundle_names();
    names
        .iter()
        .any(|b| b == name || b.to_lowercase().replace(' ', "-") == slug)
}

fn get_cached_bundle_names() -> Vec<String> {
    let mut guard = BUNDLE_NAMES_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((ref names, built_at)) = *guard {
        if built_at.elapsed().as_secs() < BUNDLE_CACHE_TTL_SECS {
            return names.clone();
        }
    }
    let names = scan_bundle_names();
    *guard = Some((names.clone(), Instant::now()));
    names
}

fn is_skill_name(name: &str) -> bool {
    let dirs = crate::skill_dirs::skill_dirs();
    dirs.iter().any(|(_, dir)| dir.join(name).is_dir())
}

fn is_personality_name(name: &str) -> bool {
    let personalities_dir = ACTIVE_PERSONALITY_FILE.parent().unwrap();
    personalities_dir.join(name).join("SOUL.md").exists()
}

pub fn switch_personality(name: &str) -> Result<String, String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.starts_with('.') {
        return Err(format!("Invalid personality name: '{}'", name));
    }
    let personalities_dir = ACTIVE_PERSONALITY_FILE.parent().unwrap();
    let dir = personalities_dir.join(name);
    if !dir.exists() || !dir.join("SOUL.md").exists() {
        return Err(format!(
            "Personality '{}' does not exist. Available: {}",
            name,
            list_personality_names().join(", ")
        ));
    }
    std::fs::create_dir_all(personalities_dir)
        .map_err(|e| format!("Failed to create personalities directory: {}", e))?;
    std::fs::write(&*ACTIVE_PERSONALITY_FILE, name)
        .map_err(|e| format!("Failed to write active personality: {}", e))?;
    Ok(format!("Switched to personality: {}", name))
}

fn list_personality_names() -> Vec<String> {
    let personalities_dir = ACTIVE_PERSONALITY_FILE.parent().unwrap();
    if !personalities_dir.exists() {
        return Vec::new();
    }
    std::fs::read_dir(personalities_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| e.path().join("SOUL.md").exists())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        None
                    } else {
                        Some(name)
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Deserialize)]
struct BundleNameOnly {
    #[allow(dead_code)]
    name: String,
}

fn scan_bundle_names() -> Vec<String> {
    if !BUNDLE_DIR.exists() {
        return Vec::new();
    }
    std::fs::read_dir(&*BUNDLE_DIR)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "yaml" || ext == "yml")
                })
                .filter_map(|e| {
                    let content = std::fs::read_to_string(e.path()).ok()?;
                    serde_yaml::from_str::<BundleNameOnly>(&content)
                        .ok()
                        .map(|b| b.name)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn load_bundle_content(name: &str, args: &str) -> Option<String> {
    let slug = name.to_lowercase().replace(' ', "-");
    let yaml_path = BUNDLE_DIR.join(format!("{}.yaml", slug));
    let yml_path = BUNDLE_DIR.join(format!("{}.yml", slug));

    let path = if yaml_path.exists() {
        yaml_path
    } else if yml_path.exists() {
        yml_path
    } else {
        let names = scan_bundle_names();
        let matched = names
            .iter()
            .find(|b| *b == name || b.to_lowercase().replace(' ', "-") == slug)?;
        let matched_slug = matched.to_lowercase().replace(' ', "-");
        let p = BUNDLE_DIR.join(format!("{}.yaml", matched_slug));
        if p.exists() {
            p
        } else {
            BUNDLE_DIR.join(format!("{}.yml", matched_slug))
        }
    };

    let content = std::fs::read_to_string(&path).ok()?;
    let bundle: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;

    let bundle_name = bundle["name"].as_str().unwrap_or(name);
    let description = bundle["description"].as_str().unwrap_or("");
    let instruction = bundle["instruction"].as_str().unwrap_or("");
    let skills: Vec<String> = bundle["skills"]
        .as_sequence()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut output = format!("# Skill Bundle: {}\n\n", bundle_name);
    if !description.is_empty() {
        output.push_str(&format!("**Description**: {}\n\n", description));
    }
    if !instruction.is_empty() {
        output.push_str(&format!("**Instruction**: {}\n\n---\n\n", instruction));
    }

    let dirs = crate::skill_dirs::skill_dirs();
    for skill_name in &skills {
        let found = dirs.iter().find(|(_, dir)| dir.join(skill_name).is_dir());
        if let Some((_, dir)) = found {
            let skill_md = dir.join(skill_name).join("SKILL.md");
            if let Ok(skill_content) = std::fs::read_to_string(skill_md) {
                output
                    .push_str(&format!("## Skill: {}\n\n{}\n\n---\n\n", skill_name, skill_content));
            }
        }
    }

    if !args.is_empty() {
        output.push_str(&format!("\n**User args**: {}\n", args));
    }

    Some(output)
}

pub fn load_skill_content(name: &str, args: &str) -> Option<String> {
    let dirs = crate::skill_dirs::skill_dirs();
    let found = dirs.iter().find(|(_, dir)| dir.join(name).is_dir());
    let dir = &found?.1;

    let skill_md = dir.join(name).join("SKILL.md");
    let content = if skill_md.exists() {
        std::fs::read_to_string(&skill_md).ok()?
    } else {
        let alt = dir.join(name).join(format!("{}.md", name));
        if alt.exists() {
            std::fs::read_to_string(&alt).ok()?
        } else {
            return None;
        }
    };

    let mut output = format!("# Skill: {}\n\n{}\n\n", name, content);
    if !args.is_empty() {
        output.push_str(&format!("**User args**: {}\n", args));
    }

    Some(output)
}

pub fn apply_slash_command_to_input(text: &str) -> SlashCommandPreprocessed {
    let Some(action) = process_slash_command(text) else {
        return SlashCommandPreprocessed {
            modified_text: text.to_string(),
            personality_prompt: None,
            is_builtin: false,
        };
    };

    match action {
        SlashCommandAction::LoadBundle { name, args } => {
            let modified_text = if let Some(content) = load_bundle_content(&name, &args) {
                let user_request = if args.is_empty() {
                    name.clone()
                } else {
                    args.clone()
                };
                format!(
                    "The user activated the skill bundle '/{}'. The bundle content is loaded below as context. Follow the bundle's instructions to assist the user.\n\n{}\n\nUser request: {}",
                    name, content, user_request
                )
            } else {
                format!("Skill bundle '{}' not found. Use /bundles to see available bundles.", name)
            };
            SlashCommandPreprocessed {
                modified_text,
                personality_prompt: None,
                is_builtin: false,
            }
        },
        SlashCommandAction::LoadSkill { name, args } => {
            let modified_text = if let Some(content) = load_skill_content(&name, &args) {
                let user_request = if args.is_empty() {
                    name.clone()
                } else {
                    args.clone()
                };
                format!(
                    "The user activated the skill '/{}'. The skill content is loaded below as context. Follow the skill's instructions to assist the user.\n\n{}\n\nUser request: {}",
                    name, content, user_request
                )
            } else {
                format!("Skill '{}' not found. Use /skills to see available skills.", name)
            };
            SlashCommandPreprocessed {
                modified_text,
                personality_prompt: None,
                is_builtin: false,
            }
        },
        SlashCommandAction::SwitchPersonality { name } => {
            let personality_prompt = match switch_personality(&name) {
                Ok(msg) => Some(msg),
                Err(e) => {
                    return SlashCommandPreprocessed {
                        modified_text: format!("Failed to switch personality: {}", e),
                        personality_prompt: None,
                        is_builtin: false,
                    };
                },
            };
            SlashCommandPreprocessed {
                modified_text: format!("Switched to personality: {}", name),
                personality_prompt,
                is_builtin: false,
            }
        },
        SlashCommandAction::BuiltIn { command, args } => SlashCommandPreprocessed {
            modified_text: format!(
                "/{}{}",
                command,
                if args.is_empty() {
                    String::new()
                } else {
                    format!(" {}", args)
                }
            ),
            personality_prompt: None,
            is_builtin: true,
        },
        SlashCommandAction::Unknown => SlashCommandPreprocessed {
            modified_text: text.to_string(),
            personality_prompt: None,
            is_builtin: false,
        },
    }
}

#[derive(Debug, Clone)]
pub struct SlashCommandPreprocessed {
    pub modified_text: String,
    pub personality_prompt: Option<String>,
    pub is_builtin: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_slash_text_returns_none() {
        assert!(process_slash_command("hello world").is_none());
    }

    #[test]
    fn test_empty_slash_returns_none() {
        assert!(process_slash_command("/").is_none());
    }

    #[test]
    fn test_builtin_new() {
        let action = process_slash_command("/new").unwrap();
        assert_eq!(
            action,
            SlashCommandAction::BuiltIn {
                command: "new".to_string(),
                args: String::new()
            }
        );
    }

    #[test]
    fn test_builtin_model_with_args() {
        let action = process_slash_command("/model gpt-4").unwrap();
        assert_eq!(
            action,
            SlashCommandAction::BuiltIn {
                command: "model".to_string(),
                args: "gpt-4".to_string()
            }
        );
    }

    #[test]
    fn test_builtin_personality() {
        let action = process_slash_command("/personality").unwrap();
        assert_eq!(
            action,
            SlashCommandAction::BuiltIn {
                command: "personality".to_string(),
                args: String::new()
            }
        );
    }

    #[test]
    fn test_builtin_commands_all_recognized() {
        for cmd in BUILTIN_COMMANDS {
            let action = process_slash_command(&format!("/{}", cmd)).unwrap();
            assert!(matches!(action, SlashCommandAction::BuiltIn { .. }));
        }
    }

    #[test]
    fn test_unknown_slash_command() {
        let action = process_slash_command("/nonexistent-command-xyz").unwrap();
        assert_eq!(action, SlashCommandAction::Unknown);
    }

    #[test]
    fn test_slash_with_whitespace() {
        let action = process_slash_command("  /new  ").unwrap();
        assert_eq!(
            action,
            SlashCommandAction::BuiltIn {
                command: "new".to_string(),
                args: String::new()
            }
        );
    }

    #[test]
    fn test_slash_command_router_process() {
        let action = SlashCommandRouter::process("/stop").unwrap();
        assert_eq!(
            action,
            SlashCommandAction::BuiltIn {
                command: "stop".to_string(),
                args: String::new()
            }
        );
    }

    #[test]
    fn test_slash_command_with_multi_word_args() {
        let action = process_slash_command("/model gpt-4 turbo mode").unwrap();
        assert_eq!(
            action,
            SlashCommandAction::BuiltIn {
                command: "model".to_string(),
                args: "gpt-4 turbo mode".to_string()
            }
        );
    }

    #[test]
    fn test_builtin_takes_priority_over_bundle() {
        let action = process_slash_command("/bundles").unwrap();
        assert_eq!(
            action,
            SlashCommandAction::BuiltIn {
                command: "bundles".to_string(),
                args: String::new()
            }
        );
    }

    #[test]
    fn test_builtin_skills_command() {
        let action = process_slash_command("/skills").unwrap();
        assert!(matches!(action, SlashCommandAction::BuiltIn { .. }));
    }

    #[test]
    fn test_action_debug_format() {
        let action = SlashCommandAction::LoadBundle {
            name: "test".to_string(),
            args: "arg1".to_string(),
        };
        let debug = format!("{:?}", action);
        assert!(debug.contains("LoadBundle"));
    }

    #[test]
    fn test_action_clone() {
        let action = SlashCommandAction::BuiltIn {
            command: "new".to_string(),
            args: String::new(),
        };
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_action_serialize_deserialize() {
        let action = SlashCommandAction::LoadSkill {
            name: "my-skill".to_string(),
            args: "do something".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: SlashCommandAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }
}
