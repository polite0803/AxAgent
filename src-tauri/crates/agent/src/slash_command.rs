pub use axagent_core::slash_command::{
    SlashCommandAction, SlashCommandPreprocessed, SlashCommandRouter, apply_slash_command_to_input,
    load_bundle_content, load_skill_content, process_slash_command, switch_personality,
};

use crate::personality::PersonalityManager;

pub fn handle_switch_personality(name: &str) -> Result<String, String> {
    PersonalityManager::set_active(name)?;
    let personality = PersonalityManager::load(name)?;
    Ok(format!(
        "Switched to personality: {}.\n\n{}",
        personality.name,
        if personality.description.is_empty() {
            String::new()
        } else {
            format!("*{}*\n\n", personality.description)
        }
    ))
}

pub fn apply_slash_command_for_agent(text: &str) -> SlashCommandPreprocessed {
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
                    "Use the SkillBundleLoad tool with bundle=\"{}\" and args=\"{}\".\n\nAlternatively, here is the pre-loaded bundle content for reference:\n\n{}\n\nUser request: {}",
                    name, args, content, user_request
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
                    "Use the SkillView tool with skill=\"{}\" and args=\"{}\".\n\nAlternatively, here is the pre-loaded skill content for reference:\n\n{}\n\nUser request: {}",
                    name, args, content, user_request
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
            let personality_prompt = match handle_switch_personality(&name) {
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
