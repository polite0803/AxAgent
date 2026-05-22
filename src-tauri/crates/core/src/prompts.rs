//! LLM 提示词多语言注册表
//! 编译时嵌入所有语言文本，运行时按语言选择。

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptLang {
    ZhCN,
    EnUS,
}

impl PromptLang {
    pub fn from_locale(locale: &str) -> Self {
        match locale {
            "en-US" | "en" => Self::EnUS,
            _ => Self::ZhCN,
        }
    }
}

pub struct PromptRegistry;

impl PromptRegistry {
    pub fn get(key: &str, lang: PromptLang) -> &'static str {
        match lang {
            PromptLang::ZhCN => get_zh(key),
            PromptLang::EnUS => get_en(key),
        }
    }
    pub fn format(key: &str, lang: PromptLang, args: &[&str]) -> String {
        let t = Self::get(key, lang);
        let mut r = t.to_string();
        for (i, a) in args.iter().enumerate() {
            r = r.replace(&format!("{{{}}}", i), a);
        }
        r
    }
    pub fn get_all_languages(key: &str) -> HashMap<String, &'static str> {
        let mut m = HashMap::new();
        m.insert("zh-CN".into(), get_zh(key));
        m.insert("en-US".into(), get_en(key));
        m
    }
}

// ═══ 中文 ═══
fn get_zh(key: &str) -> &'static str {
    match key {
        "extraction.system_prompt" => {
            r#"你是一个知识提取助手。你的任务是从对话转录中提取重要、可复用的知识。

提取以下类型的知识：
1. **事实 (Facts)**：用户分享的重要事实信息（如"用户的项目使用 React 18 + TypeScript"）
2. **偏好 (Preferences)**：用户的偏好和习惯模式（如"用户更喜欢函数组件而非类组件"）
3. **流程 (Procedures)**：讨论过的分步流程或解决方案（如"部署步骤：先构建，再推送到 S3"）
4. **上下文 (Context)**：关于用户工作环境的重要背景（如"用户正在开发一个 Tauri 桌面应用"）

对于每个提取项，同时确定：
- **importance**：0.0 到 1.0 的重要程度评分（0.3=次要，0.5=一般，0.7=重要，0.9=关键）
- **nature**：是"episodic"（特定事件/交互）还是"semantic"（一般知识/偏好）
- **tags**：1-3 个相关的分类标签

规则：
- 只提取在未来的对话中有用的知识
- 不要提取琐碎或显而易见的信息
- 每项内容应自包含，无需上下文即可理解
- 标题简洁（50 字符以内），内容详细但精炼（200 字符以内）

返回一个 JSON 数组。如果没有发现重要知识，返回空数组：[]"#
        },

        "extraction.user_template" => "从以下对话中提取可复用的知识：\n\n{0}",

        "consolidation.system_prompt" => {
            r#"你是一个知识整合助手。将多条相似的记忆合并为一条简洁的摘要。

规则：保留所有重要信息，去除冗余，合并为单条连贯记忆。保留最高 importance 评分和最合适的 category。

返回 JSON：{"title","content","category","importance","nature","tags"}"#
        },

        "consolidation.user_template" => "将以下 {0} 条相似记忆合并为一条：\n\n{1}",

        "entity_extraction.system_prompt" => {
            r#"你是一个实体和关系提取助手。从对话转录中提取关键实体及其关系。

实体类型：person、project、tool、technology、file、concept
关系类型：uses、depends_on、creates、mentions、owns、works_on

返回 JSON：{"entities":[...],"relations":[...]}"#
        },

        "entity_extraction.user_template" => "从以下对话中提取实体和关系：\n\n{0}",

        "incremental_extract.system_prompt" => {
            r#"你是一个知识提取助手。从最近的对话中提取新的知识。
规则：只提取之前不知道的新信息，重点关注用户的偏好、决策和项目背景。返回 JSON 数组，如果没有新知识返回 []。"#
        },

        "incremental_extract.user_template" => "从以下最近的对话中提取新知识：\n\n{0}",

        "conversation_summary.merge_template" => {
            "你是一个对话摘要助手。请将以下新增对话内容合并到已有摘要中。\n\n已有摘要：\n{0}\n\n新增对话：\n{1}\n\n请输出合并后的摘要，保留所有重要信息，去除冗余。"
        },

        "conversation_summary.compress_template" => {
            "你是一个对话摘要助手。请将以下对话历史压缩为简洁摘要。\n\n对话内容：\n{0}\n\n请保留关键决策、用户偏好、正在进行的工作和重要上下文。"
        },

        "title_generation.system_prompt" => {
            "你是一个对话标题生成助手。根据对话内容生成一个简洁的标题（不超过 50 字符）。只返回标题文本。"
        },

        "title_generation.user_template" => "请为以下对话生成标题：\n\n{0}",

        "web_search.function_name" => "web_search",
        "web_search.function_desc" => "搜索网络获取最新信息。",

        "compact.continuation_preamble" => {
            "此会话从之前的对话续接而来。以下摘要覆盖了对话的早期部分。\n\n"
        },
        "compact.recent_messages_note" => "最近的消息被原样保留。",
        "compact.resume_instruction" => "从上次中断的地方继续对话，不要再向用户提问。直接继续。",

        "workflow_ai.generation_system" => {
            "你是一个工作流设计助手。根据用户的描述，生成一个结构化的自动化工作流。输出 JSON 格式。"
        },
        "workflow_ai.generation_user" => "基于以下描述生成工作流：{0}",

        _ => "",
    }
}

// ═══ 英文 ═══
fn get_en(key: &str) -> &'static str {
    match key {
        "extraction.system_prompt" => {
            r#"You are a knowledge extraction assistant. Extract important, reusable knowledge from conversation transcripts.

Types: Facts, Preferences, Procedures, Context.
For each: importance (0.0-1.0), nature (episodic/semantic), tags.

Return JSON array. Return [] if no significant knowledge found."#
        },

        "extraction.user_template" => "Extract reusable knowledge from this conversation:\n\n{0}",

        "consolidation.system_prompt" => {
            r#"You are a knowledge consolidation assistant. Merge multiple similar memories into one.
Preserve all important info, remove redundancy. Keep highest importance score.
Return JSON: {"title","content","category","importance","nature","tags"}"#
        },

        "consolidation.user_template" => "Consolidate these {0} similar memories into one:\n\n{1}",

        "entity_extraction.system_prompt" => {
            r#"You are an entity extraction assistant. Extract entities and relationships.
Entity types: person, project, tool, technology, file, concept.
Relation types: uses, depends_on, creates, mentions, owns, works_on.
Return JSON: {"entities":[...],"relations":[...]}"#
        },

        "entity_extraction.user_template" => "Extract entities and relationships:\n\n{0}",

        "incremental_extract.system_prompt" => {
            r#"You are a knowledge extraction assistant. Extract NEW knowledge only.
Focus on user preferences, decisions, and project context.
Return JSON array. Return [] if no new knowledge."#
        },

        "incremental_extract.user_template" => "Extract NEW knowledge from recent exchange:\n\n{0}",

        "conversation_summary.merge_template" => {
            "Merge new conversation content into existing summary.\n\nExisting:\n{0}\n\nNew:\n{1}\n\nOutput merged summary, preserving important info."
        },

        "conversation_summary.compress_template" => {
            "Compress conversation history into concise summary.\n\nContent:\n{0}\n\nPreserve key decisions, preferences, ongoing work."
        },

        "title_generation.system_prompt" => {
            "Generate a concise title (max 50 chars) based on the conversation. Return only the title text."
        },

        "title_generation.user_template" => "Generate title for:\n\n{0}",

        "web_search.function_name" => "web_search",
        "web_search.function_desc" => "Search the web for up-to-date information.",

        "compact.continuation_preamble" => {
            "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\n"
        },
        "compact.recent_messages_note" => "Recent messages are preserved verbatim.",
        "compact.resume_instruction" => {
            "Continue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, and do not preface with continuation text."
        },

        "workflow_ai.generation_system" => {
            "You are a workflow design assistant. Generate a structured automation workflow based on the user's description. Output JSON format."
        },
        "workflow_ai.generation_user" => "Generate workflow based on: {0}",

        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_lang_from_locale() {
        assert_eq!(PromptLang::from_locale("zh-CN"), PromptLang::ZhCN);
        assert_eq!(PromptLang::from_locale("en-US"), PromptLang::EnUS);
    }

    #[test]
    fn test_get_prompt_both_languages() {
        let zh = PromptRegistry::get("extraction.system_prompt", PromptLang::ZhCN);
        let en = PromptRegistry::get("extraction.system_prompt", PromptLang::EnUS);
        assert!(!zh.is_empty());
        assert!(!en.is_empty());
        assert_ne!(zh, en);
    }

    #[test]
    fn test_format_with_args() {
        let result =
            PromptRegistry::format("extraction.user_template", PromptLang::ZhCN, &["测试"]);
        assert!(result.contains("测试"));
        assert!(!result.contains("{0}"));
    }
}
