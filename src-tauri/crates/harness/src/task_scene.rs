// SPDX-License-Identifier: AGPL-3.0-only

//! 3.7 P2:TaskScene 下沉工作流层。
//!
//! 历史上 `TaskScene` 定义在 `axagent_runtime::prompt`(wiring 层),
//! 导致 `axagent_rt_workflow`(hybrid 层)无法依赖它来给工作流 Agent 节点
//! 应用场景化 prompt。按 AGENTS.md 「依赖方向铁律」,共享类型应定义在
//! `harness`(foundation 层),由 consumer / implementor / hybrid / wiring
//! 各层 `pub use` 引用。
//!
//! 本模块提供权威定义,`axagent_runtime::prompt` 通过 re-export 保持
//! 向后兼容(原有 API 不变)。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 任务场景 — 决定动态加载哪些 prompt 模块。
///
/// 工作流 Agent 节点可在 `AgentNodeConfig.task_scene` 显式指定;
/// 未指定时由 `TaskScene::infer` 从输入文本自动推断。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../generated/task_scene.ts")]
pub enum TaskScene {
    /// 通用聊天、文档处理、系统操作
    #[default]
    General,
    /// 代码阅读、项目修改、功能开发、代码搜索
    Code,
    /// 研究型任务:知识抽取、学术检索
    Research,
    /// 自动模式 — 由上下文推断,不显式选择
    Auto,
}

impl TaskScene {
    /// 从字符串解析(容错:未知值回退到 `General`)。
    ///
    /// 接受大小写不敏感的 "general" / "code" / "research" / "auto"。
    /// 用于 `AgentNodeConfig.task_scene` 字段的反序列化兜底。
    pub fn from_str_or_general(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "code" => Self::Code,
            "research" => Self::Research,
            "auto" => Self::Auto,
            _ => Self::General,
        }
    }

    /// 从用户输入文本推断任务场景。
    ///
    /// 评分规则:
    /// - code_score >= 2 → Code
    /// - research_score >= 2 → Research
    /// - code_score > 0 → Code(弱信号兜底)
    /// - 否则 → General
    pub fn infer(text: &str) -> Self {
        let lowered = text.to_lowercase();

        let code_keywords = [
            "code",
            "function",
            "class",
            "impl",
            "compile",
            "build",
            "cargo",
            "npm",
            "test",
            "debug",
            "error",
            "fix",
            "refactor",
            "rust",
            "typescript",
            "javascript",
            "python",
            "golang",
            "java",
            "struct",
            "trait",
            "mod",
            "import",
            "export",
            "component",
            "hook",
            "api",
            "endpoint",
        ];
        let research_keywords = [
            "research",
            "analyze",
            "knowledge",
            "extract",
            "academic",
            "paper",
            "search for",
            "find information",
            "learn about",
            "explain concept",
        ];

        let code_score = code_keywords.iter().filter(|k| lowered.contains(*k)).count();
        let research_score = research_keywords.iter().filter(|k| lowered.contains(*k)).count();

        if code_score >= 2 {
            Self::Code
        } else if research_score >= 2 {
            Self::Research
        } else if code_score > 0 {
            Self::Code
        } else {
            Self::General
        }
    }

    /// 返回该场景的简短输出约束指令(注入到 prompt 尾部)。
    ///
    /// - `Code`:强调直接给代码、少废话
    /// - `Research`:强调结构化分析、引用、权衡
    /// - `General` / `Auto`:空字符串(无约束)
    pub fn concise_directive(&self) -> &'static str {
        match self {
            Self::Code => concat!(
                "## Output Constraints for Code Mode\n",
                "- Provide code solutions directly without lengthy explanations.\n",
                "- Do not restate what the code does unless asked.\n",
                "- Minimize commentary; focus on implementation.\n",
                "- Include only essential comments in generated code.\n",
                "- Skip boilerplate explanations (e.g., \"Here is how you...\").\n",
                "- If the solution is short, output only the code."
            ),
            Self::Research => concat!(
                "## Output Constraints for Research Mode\n",
                "- Provide thorough analysis with citations.\n",
                "- Structure output with clear headings.\n",
                "- Include trade-offs and alternatives where relevant."
            ),
            Self::General | Self::Auto => "",
        }
    }
}

impl std::fmt::Display for TaskScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General => write!(f, "general"),
            Self::Code => write!(f, "code"),
            Self::Research => write!(f, "research"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_code_returns_code_for_strong_signal() {
        assert!(matches!(TaskScene::infer("fix the rust code error"), TaskScene::Code));
    }

    #[test]
    fn infer_research_returns_research_for_strong_signal() {
        assert!(matches!(
            TaskScene::infer("research and analyze the academic paper"),
            TaskScene::Research
        ));
    }

    #[test]
    fn infer_general_for_neutral_text() {
        assert!(matches!(TaskScene::infer("hello world"), TaskScene::General));
    }

    #[test]
    fn concise_directive_non_empty_only_for_code_and_research() {
        assert!(!TaskScene::Code.concise_directive().is_empty());
        assert!(!TaskScene::Research.concise_directive().is_empty());
        assert!(TaskScene::General.concise_directive().is_empty());
        assert!(TaskScene::Auto.concise_directive().is_empty());
    }

    #[test]
    fn from_str_or_general_handles_inputs() {
        assert!(matches!(TaskScene::from_str_or_general("code"), TaskScene::Code));
        assert!(matches!(TaskScene::from_str_or_general("RESEARCH"), TaskScene::Research));
        assert!(matches!(TaskScene::from_str_or_general("auto"), TaskScene::Auto));
        assert!(matches!(TaskScene::from_str_or_general("unknown"), TaskScene::General));
    }

    #[test]
    fn display_round_trips() {
        assert_eq!(TaskScene::Code.to_string(), "code");
        assert_eq!(TaskScene::Research.to_string(), "research");
        assert_eq!(TaskScene::General.to_string(), "general");
        assert_eq!(TaskScene::Auto.to_string(), "auto");
    }
}
