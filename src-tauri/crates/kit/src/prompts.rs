//! LLM 提示词多语言注册表
//!
//! 所有发送给 LLM 的提示模板集中管理，支持根据用户语言选择对应语言的模板。
//! 编译时嵌入所有语言文本，运行时零开销查找。
//!
//! 使用方式:
//! ```rust,ignore
//! use axagent_core::prompts::{PromptRegistry, PromptLang};
//!
//! let prompt = PromptRegistry::get("extraction.system_prompt", PromptLang::ZhCN);
//! let formatted = PromptRegistry::format(
//!     "extraction.user_template",
//!     PromptLang::ZhCN,
//!     &["{transcript}"],
//! );
//! ```

use std::collections::HashMap;

/// 支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptLang {
    /// 简体中文（默认）
    ZhCN,
    /// 英文（回退语言）
    EnUS,
}

impl PromptLang {
    /// 从前端 locale 字符串转换
    pub fn from_locale(locale: &str) -> Self {
        match locale {
            "en-US" | "en" => Self::EnUS,
            "zh-CN" | "zh" => Self::ZhCN,
            // 其他语言暂回退到英文，后续按需添加
            _ => Self::ZhCN,
        }
    }

    /// 从语言代码简写转换
    pub fn from_lang_code(code: &str) -> Self {
        match code {
            "en" => Self::EnUS,
            "zh" => Self::ZhCN,
            _ => Self::ZhCN,
        }
    }
}

/// 提示词注册表 — 编译时嵌入所有语言的提示模板
pub struct PromptRegistry;

impl PromptRegistry {
    /// 获取指定 key 和语言的提示模板
    ///
    /// key 格式: "category.sub_key"，例如 "extraction.system_prompt"
    pub fn get(key: &str, lang: PromptLang) -> &'static str {
        match lang {
            PromptLang::ZhCN => get_zh_cn(key),
            PromptLang::EnUS => get_en_us(key),
        }
    }

    /// 获取提示模板并执行简单的占位符替换
    ///
    /// 占位符: {0}, {1}, {2} ...
    pub fn format(key: &str, lang: PromptLang, args: &[&str]) -> String {
        let template = Self::get(key, lang);
        let mut result = template.to_string();
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }
        result
    }

    /// 获取指定 key 在所有语言中的映射
    pub fn get_all_languages(key: &str) -> HashMap<String, &'static str> {
        let mut map = HashMap::new();
        map.insert("zh-CN".to_string(), get_zh_cn(key));
        map.insert("en-US".to_string(), get_en_us(key));
        map
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 中文 (zh-CN) 提示模板
// ═══════════════════════════════════════════════════════════════════════════

fn get_zh_cn(key: &str) -> &'static str {
    match key {
        // ── 知识提取 ──
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
- 不要提取仅与当前对话相关的信息
- 每项内容应自包含，无需上下文即可理解
- 标题简洁（50 字符以内）
- 内容详细但精炼（200 字符以内）
- 偏好和事实标记为"semantic"
- 特定事件或交互标记为"episodic"

返回一个 JSON 数组，每项包含：
- "title"：简短标签
- "content"：详细知识
- "category"：fact、preference、procedure 或 context
- "importance"：0.0 到 1.0 的数字
- "nature"：episodic 或 semantic
- "tags"：1-3 个相关标签的数组

如果没有发现重要知识，返回空数组：[]"#
        },

        "extraction.user_template" => "从以下对话中提取可复用的知识：\n\n{0}",

        // ── 记忆合并 ──
        "consolidation.system_prompt" => {
            r#"你是一个知识整合助手。你的任务是将多条相似的记忆合并为一条简洁的摘要。

规则：
- 保留所有重要的、独特的信息
- 去除冗余和重复
- 合并为单条连贯的记忆
- 保留原始记忆中最高的 importance 评分
- 保留最合适的 category 分类
- 使用第一个记忆的 nature 属性
- 合并并去重所有 tags

以 JSON 格式返回合并后的记忆：
- "title"：合并后的标题
- "content"：合并后的内容
- "category"：最合适的分类
- "importance"：最高的重要度评分
- "nature"：原始 nature
- "tags"：合并去重后的标签数组"#
        },

        "consolidation.user_template" => "将以下 {0} 条相似记忆合并为一条：\n\n{1}",

        // ── 实体提取 ──
        "entity_extraction.system_prompt" => {
            r#"你是一个实体和关系提取助手。从对话转录中提取关键实体及其关系。

提取以下内容：
1. **实体**：人物、项目、工具、技术、文件、概念等
   - name：实体名称
   - type：实体类型（person、project、tool、technology、file、concept）
   - description：简要描述
2. **关系**：实体之间的有向关系
   - source：源实体名称
   - target：目标实体名称
   - relation：关系类型（uses、depends_on、creates、mentions、owns、works_on）
   - description：关系说明

规则：
- 每个实体名称应唯一
- 关系必须引用已提取的实体
- 只提取在当前对话中有意义的实体
- 不需要提取通用的、显而易见的实体

返回 JSON：
{
  "entities": [...],
  "relations": [...]
}"#
        },

        "entity_extraction.user_template" => "从以下对话中提取实体和关系：\n\n{0}",

        // ── 增量提取 ──
        "incremental_extract.system_prompt" => {
            r#"你是一个知识提取助手。从最近的对话中提取新的知识。

规则：
- 只提取之前不知道的新信息
- 重点关注用户的偏好、决策和项目背景
- 保持条目简洁
- 不要重复已有知识

返回 JSON 数组，格式与标准提取相同。如果没有新知识，返回 []。"#
        },

        "incremental_extract.user_template" => {
            "从以下最近的对话中提取新知识。只提取之前不知道的信息：\n\n{0}"
        },

        // ── 对话摘要 ──
        "conversation_summary.merge_template" => {
            r#"你是一个对话摘要助手。请将以下新增对话内容合并到已有摘要中。

## 交付物
输出合并后的摘要文本，保留所有重要信息（决策、用户偏好、项目背景、正在执行的任务），去除冗余。

## 禁区
- 不可丢失原始摘要中的已有信息
- 不可添加新信息——仅合并输入的两部分
- 不可对内容做价值判断（如"用户做了正确决定"）

## 自验环节
输出前检查：合并后的摘要是否完整覆盖了原始摘要和新增对话中的所有关键信息？

已有摘要：
{0}

新增对话：
{1}

请输出合并后的摘要。"#
        },

        "conversation_summary.compress_template" => {
            r#"你是一个对话摘要助手。请将以下对话历史压缩为简洁摘要。

## 交付物
输出结构化的压缩摘要，保留：关键决策、用户偏好、正在执行的工作、重要上下文、技术选型及理由。

## 禁区
- 不可丢失关键决策和用户偏好
- 不可保留无关的寒暄或重复内容
- 不可添加新的解释或评论——只压缩已有内容

## 自验环节
输出前检查：是否保留了所有关键决策？是否有足够上下文让读者理解对话背景？

对话内容：
{0}"#
        },

        "conversation_summary.truncation_note" => "...[{0} 条消息已截断]",

        // ── 标题生成 ──
        "title_generation.system_prompt" => {
            "你是一个对话标题生成助手。请根据对话内容生成一个简洁的标题（不超过 50 字符）。只返回标题文本，不需要引号或其他格式。"
        },

        "title_generation.user_template" => "请为以下对话生成标题：\n\n{0}",

        // ── 工作流 AI ──
        "workflow_ai.generation_system" => {
            r#"你是一个工作流设计助手。根据用户的描述，生成一个结构化的自动化工作流。

## 交付物
1. 输出 JSON 格式的工作流定义，结构：{ nodes: [{id, type, title, config}], edges: [{id, source, target, edge_type}], variables: [{name, type, default}] }
2. 每个节点必须有明确的类型和配置
3. 每条边必须有 source 和 target，确保 DAG

## 禁区
- 不可遗漏节点间的连接——每个节点除 trigger 外至少有一条入边
- 不可创建循环依赖——工作流必须是 DAG
- 不可使用未定义的节点 id 作为边引用
- 不可凭空编造节点配置——配置字段必须与节点类型匹配

## 证据规则
- 工作流节点必须与用户描述中的功能一一对应
- 变量定义必须说明用途和预期来源

## 自验环节
输出前检查：所有节点的 source/target 是否正确？是否有无入边的非 trigger 节点？配置字段是否与节点类型匹配？"#
        },

        "workflow_ai.generation_user" => "基于以下描述生成工作流：{0}",

        "workflow_ai.generation_reason" => "基于您的描述 '{0}' 生成了工作流",

        // ── 技能分解 ──
        "skill_decomposition.analyze" => {
            r#"分析以下技能描述，将其分解为可重用的子技能组件：

技能名称：{0}
技能描述：{1}

请识别：
1. 可以独立存在的子功能
2. 通用的、可跨技能复用的组件
3. 与其他技能的依赖关系
"#
        },

        "skill_decomposition.decompose" => {
            r#"将以下技能分解为独立的子技能：

{0}

每个子技能应：
1. 有明确、单一的功能边界
2. 可以独立开发和测试
3. 通过清晰的接口与其他子技能交互
"#
        },

        // ── Web 搜索 ──
        "web_search.function_name" => "web_search",

        "web_search.function_desc" => {
            "搜索网络获取最新信息。当需要查找最新资讯、事实信息或超过知识截止日期的内容时使用此工具。"
        },

        // ── 会话续接 ──
        "compact.continuation_preamble" => {
            "此会话从之前的对话续接而来，之前的对话因超出上下文限制而被压缩。以下摘要覆盖了对话的早期部分。\n\n"
        },

        "compact.recent_messages_note" => "最近的消息被原样保留。",

        "compact.resume_instruction" => {
            "从上次中断的地方继续对话，不要再向用户提问。直接继续——不要确认摘要内容，不要回顾之前发生的事情，不要添加续接说明文字。"
        },

        // ── 回退 ──
        _ => "",
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 英文 (en-US) 提示模板
// ═══════════════════════════════════════════════════════════════════════════

fn get_en_us(key: &str) -> &'static str {
    match key {
        // ── Knowledge Extraction ──
        "extraction.system_prompt" => {
            r#"You are a knowledge extraction assistant. Your task is to extract important, reusable knowledge from conversation transcripts.

Extract the following types of knowledge:
1. **Facts**: Important factual information the user shared (e.g., "User's project uses React 18 with TypeScript")
2. **Preferences**: User preferences and patterns (e.g., "User prefers functional components over class components")
3. **Procedures**: Step-by-step processes or solutions discussed (e.g., "To deploy: run build, then push to S3")
4. **Context**: Important context about the user's work environment (e.g., "User works on a Tauri desktop app")

For each extracted item, also determine:
- **importance**: A score from 0.0 to 1.0 indicating how important this memory is (0.3=minor, 0.5=moderate, 0.7=important, 0.9=critical)
- **nature**: Whether this is "episodic" (a specific event/interaction) or "semantic" (general knowledge/preference)
- **tags**: 1-3 relevant tags for categorization

Rules:
- Only extract knowledge that would be useful in FUTURE conversations
- Do NOT extract trivial or obvious information
- Do NOT extract information that is only relevant to the current conversation
- Each item should be self-contained and understandable without context
- Keep titles concise (under 50 characters)
- Keep content detailed but concise (under 200 characters)
- Preferences and facts should be marked as "semantic"
- Specific events or interactions should be marked as "episodic"

Respond with a JSON array of extracted items. Each item should have:
- "title": a short label
- "content": the detailed knowledge
- "category": one of "fact", "preference", "procedure", "context"
- "importance": a number from 0.0 to 1.0
- "nature": either "episodic" or "semantic"
- "tags": an array of 1-3 relevant tags

If no significant knowledge is found, return an empty array: []"#
        },

        "extraction.user_template" => "Extract reusable knowledge from this conversation:\n\n{0}",

        // ── Memory Consolidation ──
        "consolidation.system_prompt" => {
            r#"You are a knowledge consolidation assistant. Your task is to merge multiple similar memories into a single concise summary.

Rules:
- Preserve all important, unique information
- Remove redundancy and duplication
- Merge into a single coherent memory
- Keep the highest importance score from the original memories
- Keep the most appropriate category
- Use the nature from the first memory
- Merge and deduplicate all tags

Return the consolidated memory as JSON:
- "title": merged title
- "content": merged content
- "category": most appropriate category
- "importance": highest importance score
- "nature": original nature
- "tags": merged and deduplicated tag array"#
        },

        "consolidation.user_template" => "Consolidate these {0} similar memories into one:\n\n{1}",

        // ── Entity Extraction ──
        "entity_extraction.system_prompt" => {
            r#"You are an entity and relationship extraction assistant. Extract key entities and their relationships from conversation transcripts.

Extract the following:
1. **Entities**: people, projects, tools, technologies, files, concepts, etc.
   - name: entity name
   - type: entity type (person, project, tool, technology, file, concept)
   - description: brief description
2. **Relationships**: directed relationships between entities
   - source: source entity name
   - target: target entity name
   - relation: relation type (uses, depends_on, creates, mentions, owns, works_on)
   - description: relationship description

Rules:
- Each entity name should be unique
- Relationships must reference already extracted entities
- Only extract entities meaningful in the current conversation
- Do not extract generic, obvious entities

Return JSON:
{
  "entities": [...],
  "relations": [...]
}"#
        },

        "entity_extraction.user_template" => {
            "Extract entities and relationships from this conversation:\n\n{0}"
        },

        // ── Incremental Extract ──
        "incremental_extract.system_prompt" => {
            r#"You are a knowledge extraction assistant. Extract NEW knowledge from recent conversation exchanges.

Rules:
- Only extract information NOT already known
- Focus on user preferences, decisions, and project context
- Keep entries concise
- Do not repeat existing knowledge

Return JSON array in same format as standard extraction. Return [] if no new knowledge."#
        },

        "incremental_extract.user_template" => {
            "Extract NEW knowledge from this recent conversation exchange. Focus on information NOT already known:\n\n{0}"
        },

        // ── Conversation Summary ──
        "conversation_summary.merge_template" => {
            r#"You are a conversation summary assistant. Merge the following new conversation content into the existing summary.

## Deliverable
Output the merged summary preserving: key decisions, user preferences, project context, ongoing tasks. Remove redundancy.

## 禁区 (Forbidden)
- Do NOT lose information that exists in the original summary
- Do NOT add new information — only merge the two inputs
- Do NOT make value judgments about the content

## Self-Verification
Before output: Does the merged summary cover all key information from both the original summary and the new conversation?

Existing summary:
{0}

New conversation:
{1}"#
        },

        "conversation_summary.compress_template" => {
            r#"You are a conversation summary assistant. Compress the following conversation history into a concise summary.

## Deliverable
Output a structured compressed summary preserving: key decisions, user preferences, ongoing work, important context, technical choices and rationale.

## 禁区 (Forbidden)
- Do NOT drop key decisions or preferences
- Do NOT keep irrelevant greetings or repetition
- Do NOT add new explanations or commentary — only compress existing content

## Self-Verification
Before output: Are all key decisions preserved? Is there enough context to understand the conversation background?

Conversation content:
{0}"#
        },

        "conversation_summary.truncation_note" => "...[{0} messages truncated]",

        // ── Title Generation ──
        "title_generation.system_prompt" => {
            "You are a conversation title generator. Generate a concise title (under 50 characters) based on the conversation content. Return only the title text, no quotes or formatting."
        },

        "title_generation.user_template" => "Generate a title for this conversation:\n\n{0}",

        // ── Workflow AI ──
        "workflow_ai.generation_system" => {
            r#"You are a workflow design assistant. Generate a structured automation workflow based on the user's description.

## Deliverable
1. Output JSON workflow definition: { nodes: [{id, type, title, config}], edges: [{id, source, target, edge_type}], variables: [{name, type, default}] }
2. Each node must have a clear type and configuration
3. Each edge must have source and target, ensuring a valid DAG

## 禁区 (Forbidden)
- Do NOT leave any node disconnected — every node except trigger must have at least one incoming edge
- Do NOT create circular dependencies — the workflow must be a DAG
- Do NOT reference undefined node ids in edges
- Do NOT invent node configurations that don't match the node type

## Evidence Rules
- Workflow nodes must map one-to-one with functions described by the user
- Variable definitions must state their purpose and expected source

## Self-Verification
Before output: Are all node source/target references correct? Are there any non-trigger nodes without incoming edges? Do configurations match node types?"#
        },

        "workflow_ai.generation_user" => "Generate a workflow based on this description: {0}",

        "workflow_ai.generation_reason" => "Generated workflow based on your description '{0}'",

        // ── Skill Decomposition ──
        "skill_decomposition.analyze" => {
            r#"Analyze the following skill description and decompose it into reusable sub-skill components:

Skill name: {0}
Skill description: {1}

Identify:
1. Sub-functions that can stand independently
2. Generic, cross-skill reusable components
3. Dependencies with other skills
"#
        },

        "skill_decomposition.decompose" => {
            r#"Decompose the following skill into independent sub-skills:

{0}

Each sub-skill should:
1. Have a clear, single-function boundary
2. Be independently developable and testable
3. Interact with other sub-skills through clear interfaces
"#
        },

        // ── Web Search ──
        "web_search.function_name" => "web_search",

        "web_search.function_desc" => {
            "Search the web for up-to-date information. Use this tool when you need to find recent news, factual information, or content beyond your knowledge cutoff date."
        },

        // ── Session Continuation ──
        "compact.continuation_preamble" => {
            "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\n"
        },

        "compact.recent_messages_note" => "Recent messages are preserved verbatim.",

        "compact.resume_instruction" => {
            "Continue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, and do not preface with continuation text."
        },

        // ── Fallback ──
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
        assert_eq!(PromptLang::from_locale("ja"), PromptLang::ZhCN); // 默认回退中文
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
            PromptRegistry::format("extraction.user_template", PromptLang::ZhCN, &["测试对话内容"]);
        assert!(result.contains("测试对话内容"));
        assert!(!result.contains("{0}"));
    }

    #[test]
    fn test_missing_key_returns_empty() {
        let result = PromptRegistry::get("nonexistent.key", PromptLang::ZhCN);
        assert_eq!(result, "");
    }

    #[test]
    fn test_get_all_languages() {
        let map = PromptRegistry::get_all_languages("extraction.system_prompt");
        assert!(map.contains_key("zh-CN"));
        assert!(map.contains_key("en-US"));
        assert!(!map.get("zh-CN").unwrap().is_empty());
        assert!(!map.get("en-US").unwrap().is_empty());
    }
}
