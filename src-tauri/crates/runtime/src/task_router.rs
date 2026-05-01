//! Smart task router — millisecond-level task classification and automatic
//! dispatch to the appropriate engine (CodeEngine vs GeneralEngine).
//!
//! Uses keyword matching and context features to determine whether a user
//! request should be routed to the code engine (code reading, editing,
//! search, development) or the general engine (chat, documents, system ops).
//!
//! # Architecture
//!
//! The TaskRouter evaluates user input against keyword patterns and context
//! features to produce a RouteDecision. If the score exceeds the code threshold,
//! the request is dispatched to the CodeEngine fast path; otherwise it goes
//! through the GeneralEngine standard path. Accumulated context (consecutive
//! code turns, open files, recent tool usage) biases subsequent routing via
//! a "sticky" momentum mechanism.

use serde::{Deserialize, Serialize};

/// The routing decision produced by the TaskRouter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteDecision {
    /// Route to the code engine — task involves code reading, editing, search, or development.
    Code,
    /// Route to the general engine — task involves chat, documents, system operations, etc.
    General,
}

/// Contextual features used alongside keyword matching to improve classification accuracy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskContext {
    /// The current working directory.
    pub cwd: Option<String>,
    /// Whether the user has recently been editing code files.
    pub recent_code_activity: bool,
    /// The last tool used (if any).
    pub last_tool: Option<String>,
    /// Files currently open in the editor.
    pub open_files: Vec<String>,
    /// Number of consecutive code-related messages.
    pub consecutive_code_turns: u32,
    /// Number of consecutive general messages.
    pub consecutive_general_turns: u32,
}

impl TaskContext {
    /// Apply a routing decision to update consecutive turn counters.
    pub fn record_decision(&mut self, decision: RouteDecision) {
        match decision {
            RouteDecision::Code => {
                self.consecutive_code_turns += 1;
                self.consecutive_general_turns = 0;
            },
            RouteDecision::General => {
                self.consecutive_general_turns += 1;
                self.consecutive_code_turns = 0;
            },
        }
    }
}

/// Configuration for the task router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Minimum confidence threshold (0.0-1.0) to route to code engine.
    pub code_confidence_threshold: f32,
    /// Maximum confidence threshold below which routing falls back to general.
    pub general_confidence_threshold: f32,
    /// Weight given to keyword matches vs context.
    pub keyword_weight: f32,
    /// Weight given to context features vs keywords.
    pub context_weight: f32,
    /// Consecutive code turns required to "stick" to code engine.
    pub sticky_code_turns: u32,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            code_confidence_threshold: 0.4,
            general_confidence_threshold: 0.2,
            keyword_weight: 0.7,
            context_weight: 0.3,
            sticky_code_turns: 3,
        }
    }
}

pub struct TaskRouter {
    config: RouterConfig,
    context: TaskContext,
}

impl TaskRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            context: TaskContext::default(),
        }
    }

    /// Infer the routing decision from user input and accumulated context.
    pub fn infer(&mut self, input: &str) -> RouteDecision {
        let code_score = self.code_keyword_score(input);
        let context_bonus = self.context_code_bonus();

        let total_score =
            self.config.keyword_weight * code_score + self.config.context_weight * context_bonus;

        let decision = if total_score >= self.config.code_confidence_threshold {
            RouteDecision::Code
        } else {
            RouteDecision::General
        };

        self.context.record_decision(decision);
        decision
    }

    /// Score input text for code-related keywords.
    fn code_keyword_score(&self, input: &str) -> f32 {
        let lowered = input.to_lowercase();

        let strong_code_keywords = [
            "fn ",
            "def ",
            "function ",
            "class ",
            "struct ",
            "enum ",
            "impl ",
            "trait ",
            "import ",
            "export ",
            "const ",
            "let ",
            "pub fn",
            "mod ",
            "cargo ",
            "npm ",
            "pip ",
            "go build",
            "javac",
            "gcc ",
            "rustc",
            "compile",
            "compiler",
        ];

        let medium_code_keywords = [
            "code",
            "debug",
            "fix",
            "refactor",
            "implement",
            "test",
            "build",
            "commit",
            "push",
            "pull request",
            "merge",
            "branch",
            "repository",
            "dependency",
            "syntax",
            "lint",
            "format",
        ];

        let file_extensions = [
            ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".java", ".cpp", ".c", ".h",
            ".toml", ".json",
        ];

        let mut score = 0.0f32;

        for kw in &strong_code_keywords {
            if lowered.contains(kw) {
                score += 0.25;
            }
        }

        for kw in &medium_code_keywords {
            if lowered.contains(kw) {
                score += 0.15;
            }
        }

        for ext in &file_extensions {
            if lowered.contains(ext) {
                score += 0.1;
            }
        }

        // Detect code blocks in the input
        if input.contains("```") {
            score += 0.4;
        }

        // Detect explicit code tool references
        if lowered.contains("write_file")
            || lowered.contains("edit_file")
            || lowered.contains("read_file")
            || lowered.contains("grep")
        {
            score += 0.3;
        }

        score.min(1.0)
    }

    /// Compute a context-based bonus for code routing.
    fn context_code_bonus(&self) -> f32 {
        let mut bonus = 0.0f32;

        // Sticky code mode: if user has been coding for several turns, bias toward code
        let sticky_ratio =
            self.context.consecutive_code_turns as f32 / self.config.sticky_code_turns as f32;
        bonus += sticky_ratio.min(1.0) * 0.5;

        // Recent code activity
        if self.context.recent_code_activity {
            bonus += 0.2;
        }

        // Open files suggest coding context
        let code_file_count = self
            .context
            .open_files
            .iter()
            .filter(|f| {
                f.ends_with(".rs")
                    || f.ends_with(".ts")
                    || f.ends_with(".py")
                    || f.ends_with(".go")
                    || f.ends_with(".js")
            })
            .count();
        if code_file_count > 0 {
            bonus += 0.15 * code_file_count.min(3) as f32;
        }

        // Last tool was code-related
        if let Some(ref tool) = self.context.last_tool {
            let code_tools = [
                "read_file",
                "write_file",
                "edit_file",
                "grep_search",
                "glob_search",
                "git_diff",
                "git_log",
                "git_status",
                "lsp_diagnostics",
                "lsp_hover",
            ];
            if code_tools.contains(&tool.as_str()) {
                bonus += 0.3;
            }
        }

        bonus.min(1.0)
    }

    /// Update the task context with new information.
    pub fn update_context(
        &mut self,
        cwd: Option<String>,
        recent_code: bool,
        last_tool: Option<String>,
        open_files: Vec<String>,
    ) {
        self.context.cwd = cwd;
        self.context.recent_code_activity = recent_code;
        self.context.last_tool = last_tool;
        self.context.open_files = open_files;
    }

    /// Get the current task context.
    pub fn context(&self) -> &TaskContext {
        &self.context
    }

    /// Reset the context counters.
    pub fn reset_context(&mut self) {
        self.context = TaskContext::default();
    }

    /// Get the current routing configuration.
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }
}

impl Default for TaskRouter {
    fn default() -> Self {
        Self::new(RouterConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_code_task() {
        let mut router = TaskRouter::default();
        let decision = router.infer("Fix the bug in src/main.rs by implementing a new fn that refactors the calculate function");
        assert_eq!(decision, RouteDecision::Code);
    }

    #[test]
    fn test_routes_general_task() {
        let mut router = TaskRouter::default();
        let decision = router.infer("Hello, how are you today? What's the weather like?");
        assert_eq!(decision, RouteDecision::General);
    }

    #[test]
    fn test_sticky_code_mode() {
        let mut router = TaskRouter::default();
        // Simulate several strongly code-related turns
        for _ in 0..5 {
            router.infer(
                "Fix the bug in main.rs fn calculate and trait Calculator and struct Config",
            );
        }
        // With heavy code momentum, even a light code-related query routes to code
        let decision = router.infer("Can you check this code and fix the build?");
        assert_eq!(decision, RouteDecision::Code);
    }

    #[test]
    fn test_code_blocks_are_strong_signal() {
        let mut router = TaskRouter::default();
        let decision = router.infer("Look at this: ```rust\nfn main() {}\n```");
        assert_eq!(decision, RouteDecision::Code);
    }

    #[test]
    fn test_context_update() {
        let mut router = TaskRouter::default();
        // Build up code context with several code turns first
        for _ in 0..4 {
            router.infer("Fix the bug in main.rs fn hello and struct World");
        }
        router.update_context(
            Some("/home/user/project".to_string()),
            true,
            Some("read_file".to_string()),
            vec!["main.rs".to_string(), "lib.rs".to_string()],
        );
        // With accumulated code context, a neutral message routes to code
        let decision = router.infer("what does this function do");
        assert_eq!(decision, RouteDecision::Code);
    }
}
