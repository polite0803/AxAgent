pub use axagent_harness::trajectory_types::{GeneratedTool, LlmToolProvider, ToolCreationRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTestResult {
    pub passed: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl ToolTestResult {
    pub fn passed(output: &str, execution_time_ms: u64) -> Self {
        Self {
            passed: true,
            output: output.to_string(),
            error: None,
            execution_time_ms,
        }
    }

    pub fn failed(error: &str, execution_time_ms: u64) -> Self {
        Self {
            passed: false,
            output: String::new(),
            error: Some(error.to_string()),
            execution_time_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoToolCreatorConfig {
    pub min_pattern_frequency: u32,
    pub max_code_length: usize,
    pub require_tests: bool,
    pub sandbox_timeout_ms: u64,
}

impl Default for AutoToolCreatorConfig {
    fn default() -> Self {
        Self {
            min_pattern_frequency: 3,
            max_code_length: 4096,
            require_tests: true,
            sandbox_timeout_ms: 5000,
        }
    }
}

pub struct DefaultLlmToolProvider {
    #[allow(dead_code)]
    template_prefix: String,
}

impl DefaultLlmToolProvider {
    pub fn new() -> Self {
        Self {
            template_prefix: "auto_tool".to_string(),
        }
    }

    fn build_code_from_template(&self, request: &ToolCreationRequest) -> String {
        let tool_list = request.available_tools.join(", ");
        format!(
            r#"function {name}(input) {{
  // Auto-generated tool for: {desc}
  // Context: {ctx}
  // Available tools: {tools}
  const result = process(input);
  return {{ success: true, data: result }};
}}"#,
            name = slugify(&request.pattern_description),
            desc = request.pattern_description,
            ctx = request.context,
            tools = tool_list,
        )
    }

    fn build_improved_code(&self, tool: &GeneratedTool, error: &str) -> String {
        format!(
            r#"function {name}(input) {{
  // Improved tool for: {desc}
  // Previous error: {err}
  // Original code:
  // {orig}
  try {{
    const result = process(input);
    if (!result) throw new Error("empty result");
    return {{ success: true, data: result }};
  }} catch (e) {{
    return {{ success: false, error: e.message }};
  }}
}}"#,
            name = slugify(&tool.name),
            desc = tool.description,
            err = error,
            orig = tool.code.replace('\n', "\n  // "),
        )
    }
}

impl Default for DefaultLlmToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DefaultSandboxToolTester;

impl SandboxToolTester for DefaultSandboxToolTester {
    fn test_tool(
        &self,
        _tool: &GeneratedTool,
    ) -> Pin<Box<dyn Future<Output = Result<ToolTestResult, String>> + Send + '_>> {
        Box::pin(async { Ok(ToolTestResult::passed("Default sandbox: skipped execution", 0)) })
    }
}

impl LlmToolProvider for DefaultLlmToolProvider {
    fn generate_tool_code(
        &self,
        request: &ToolCreationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
        let code = self.build_code_from_template(request);
        let name = slugify(&request.pattern_description);
        let description = request.pattern_description.clone();
        Box::pin(async move { Ok(GeneratedTool::new(&name, &code, &description)) })
    }

    fn improve_tool_code(
        &self,
        tool: &GeneratedTool,
        error: &str,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
        let improved_code = self.build_improved_code(tool, error);
        let name = tool.name.clone();
        let description = tool.description.clone();
        let previous_usage = tool.usage_count;
        Box::pin(async move {
            let mut improved = GeneratedTool::new(&name, &improved_code, &description);
            improved.usage_count = previous_usage;
            Ok(improved)
        })
    }
}

pub trait SandboxToolTester: Send + Sync {
    fn test_tool(
        &self,
        tool: &GeneratedTool,
    ) -> Pin<Box<dyn Future<Output = Result<ToolTestResult, String>> + Send + '_>>;
}

pub struct AutoToolCreator {
    config: AutoToolCreatorConfig,
    llm_provider: Box<dyn LlmToolProvider>,
    sandbox_tester: Box<dyn SandboxToolTester>,
    created_tools: HashMap<String, GeneratedTool>,
    pattern_counts: HashMap<String, u32>,
}

impl AutoToolCreator {
    pub fn new(
        config: AutoToolCreatorConfig,
        llm_provider: Box<dyn LlmToolProvider>,
        sandbox_tester: Box<dyn SandboxToolTester>,
    ) -> Self {
        Self {
            config,
            llm_provider,
            sandbox_tester,
            created_tools: HashMap::new(),
            pattern_counts: HashMap::new(),
        }
    }

    pub fn observe_pattern(&mut self, pattern: &str) {
        *self.pattern_counts.entry(pattern.to_string()).or_insert(0) += 1;
    }

    pub fn pattern_frequency(&self, pattern: &str) -> u32 {
        *self.pattern_counts.get(pattern).unwrap_or(&0)
    }

    pub async fn create_tool_from_pattern(
        &mut self,
        pattern: &str,
        context: &str,
        available_tools: Vec<String>,
    ) -> Result<GeneratedTool, String> {
        let frequency = self.pattern_frequency(pattern);
        if frequency < self.config.min_pattern_frequency {
            return Err(format!(
                "Pattern '{}' observed {} times, minimum required is {}",
                pattern, frequency, self.config.min_pattern_frequency
            ));
        }

        let request = ToolCreationRequest::new(pattern, context, available_tools);
        let mut tool = self.llm_provider.generate_tool_code(&request).await?;

        if tool.code.len() > self.config.max_code_length {
            return Err(format!(
                "Generated code length {} exceeds maximum {}",
                tool.code.len(),
                self.config.max_code_length
            ));
        }

        if self.config.require_tests {
            let test_result = self.validate_and_register(tool.clone()).await?;
            tool = test_result;
        } else {
            self.created_tools.insert(tool.name.clone(), tool.clone());
        }

        Ok(tool)
    }

    pub async fn validate_and_register(
        &mut self,
        tool: GeneratedTool,
    ) -> Result<GeneratedTool, String> {
        let test_result = self
            .sandbox_tester
            .test_tool(&tool)
            .await
            .map_err(|e| format!("Sandbox error: {}", e))?;

        if test_result.passed {
            let mut registered = tool;
            registered.test_coverage = 1.0;
            self.created_tools
                .insert(registered.name.clone(), registered.clone());
            Ok(registered)
        } else {
            let error_msg = test_result
                .error
                .unwrap_or_else(|| "Test failed with no error message".to_string());
            Err(format!("Tool validation failed: {}", error_msg))
        }
    }

    pub fn get_tool(&self, name: &str) -> Option<&GeneratedTool> {
        self.created_tools.get(name)
    }

    pub fn get_tool_mut(&mut self, name: &str) -> Option<&mut GeneratedTool> {
        self.created_tools.get_mut(name)
    }

    pub fn list_tools(&self) -> Vec<&GeneratedTool> {
        self.created_tools.values().collect()
    }

    pub async fn improve_tool(
        &mut self,
        tool_name: &str,
        error: &str,
    ) -> Result<GeneratedTool, String> {
        let existing = self
            .created_tools
            .get(tool_name)
            .ok_or_else(|| format!("Tool '{}' not found", tool_name))?
            .clone();

        let improved = self
            .llm_provider
            .improve_tool_code(&existing, error)
            .await?;

        if improved.code.len() > self.config.max_code_length {
            return Err(format!(
                "Improved code length {} exceeds maximum {}",
                improved.code.len(),
                self.config.max_code_length
            ));
        }

        if self.config.require_tests {
            let test_result = self
                .sandbox_tester
                .test_tool(&improved)
                .await
                .map_err(|e| format!("Sandbox error: {}", e))?;

            if test_result.passed {
                let mut registered = improved;
                registered.test_coverage = 1.0;
                self.created_tools
                    .insert(registered.name.clone(), registered.clone());
                return Ok(registered);
            } else {
                let error_msg = test_result
                    .error
                    .unwrap_or_else(|| "Test failed with no error message".to_string());
                return Err(format!("Improved tool still fails validation: {}", error_msg));
            }
        }

        self.created_tools
            .insert(improved.name.clone(), improved.clone());
        Ok(improved)
    }

    pub fn tool_count(&self) -> usize {
        self.created_tools.len()
    }

    pub fn remove_tool(&mut self, name: &str) -> Option<GeneratedTool> {
        self.created_tools.remove(name)
    }

    pub fn set_llm_provider(&mut self, provider: Box<dyn LlmToolProvider>) {
        self.llm_provider = provider;
    }

    pub fn observe_trajectory(&mut self, trajectory: &crate::trajectory::Trajectory) {
        let mut tool_sequence: Vec<String> = Vec::new();
        for step in &trajectory.steps {
            if let Some(ref calls) = step.tool_calls {
                for call in calls {
                    tool_sequence.push(call.name.clone());
                }
            }
        }
        if tool_sequence.len() >= 2 {
            for window in tool_sequence.windows(2) {
                let pattern = format!("{} -> {}", window[0], window[1]);
                self.observe_pattern(&pattern);
            }
        }
        if tool_sequence.len() >= 3 {
            for window in tool_sequence.windows(3) {
                let pattern = format!("{} -> {} -> {}", window[0], window[1], window[2]);
                self.observe_pattern(&pattern);
            }
        }
    }

    pub fn get_frequent_patterns(&self, min_freq: u32) -> Vec<(String, u32)> {
        let mut patterns: Vec<(String, u32)> = self
            .pattern_counts
            .iter()
            .filter(|&(_, &count)| count >= min_freq)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        patterns.sort_by_key(|b| std::cmp::Reverse(b.1));
        patterns
    }
}

pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLlmProvider;

    impl LlmToolProvider for MockLlmProvider {
        fn generate_tool_code(
            &self,
            request: &ToolCreationRequest,
        ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
            let name = slugify(&request.pattern_description);
            let code = format!("function {}(input) {{ return input; }}", name);
            let description = request.pattern_description.clone();
            Box::pin(async move { Ok(GeneratedTool::new(&name, &code, &description)) })
        }

        fn improve_tool_code(
            &self,
            tool: &GeneratedTool,
            _error: &str,
        ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>> {
            let name = tool.name.clone();
            let code = format!("function {}_v2(input) {{ return input; }}", name);
            let description = tool.description.clone();
            Box::pin(async move { Ok(GeneratedTool::new(&name, &code, &description)) })
        }
    }

    struct AlwaysPassTester;

    impl SandboxToolTester for AlwaysPassTester {
        fn test_tool(
            &self,
            _tool: &GeneratedTool,
        ) -> Pin<Box<dyn Future<Output = Result<ToolTestResult, String>> + Send + '_>> {
            Box::pin(async move { Ok(ToolTestResult::passed("ok", 10)) })
        }
    }

    struct AlwaysFailTester;

    impl SandboxToolTester for AlwaysFailTester {
        fn test_tool(
            &self,
            _tool: &GeneratedTool,
        ) -> Pin<Box<dyn Future<Output = Result<ToolTestResult, String>> + Send + '_>> {
            Box::pin(async move { Ok(ToolTestResult::failed("assertion failed", 5)) })
        }
    }

    fn make_creator_with_tester(tester: Box<dyn SandboxToolTester>) -> AutoToolCreator {
        let config = AutoToolCreatorConfig {
            min_pattern_frequency: 2,
            max_code_length: 4096,
            require_tests: true,
            sandbox_timeout_ms: 5000,
        };
        AutoToolCreator::new(config, Box::new(MockLlmProvider), tester)
    }

    #[test]
    fn test_generated_tool_new() {
        let tool = GeneratedTool::new("my_tool", "return 1", "A test tool");
        assert_eq!(tool.name, "my_tool");
        assert_eq!(tool.code, "return 1");
        assert_eq!(tool.description, "A test tool");
        assert_eq!(tool.test_coverage, 0.0);
        assert_eq!(tool.usage_count, 0);
        assert_eq!(tool.success_rate, 0.0);
        assert!(!tool.id.is_empty());
    }

    #[test]
    fn test_generated_tool_record_success() {
        let mut tool = GeneratedTool::new("t", "c", "d");
        tool.record_success();
        assert_eq!(tool.usage_count, 1);
        assert_eq!(tool.success_rate, 1.0);

        tool.record_failure();
        assert_eq!(tool.usage_count, 2);
        assert!((tool.success_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_generated_tool_record_failure() {
        let mut tool = GeneratedTool::new("t", "c", "d");
        tool.record_failure();
        assert_eq!(tool.usage_count, 1);
        assert_eq!(tool.success_rate, 0.0);
    }

    #[test]
    fn test_tool_test_result_passed() {
        let result = ToolTestResult::passed("output", 100);
        assert!(result.passed);
        assert_eq!(result.output, "output");
        assert!(result.error.is_none());
        assert_eq!(result.execution_time_ms, 100);
    }

    #[test]
    fn test_tool_test_result_failed() {
        let result = ToolTestResult::failed("error msg", 50);
        assert!(!result.passed);
        assert_eq!(result.output, "");
        assert_eq!(result.error, Some("error msg".to_string()));
        assert_eq!(result.execution_time_ms, 50);
    }

    #[test]
    fn test_auto_tool_creator_config_default() {
        let config = AutoToolCreatorConfig::default();
        assert_eq!(config.min_pattern_frequency, 3);
        assert_eq!(config.max_code_length, 4096);
        assert!(config.require_tests);
        assert_eq!(config.sandbox_timeout_ms, 5000);
    }

    #[test]
    fn test_tool_creation_request_new() {
        let req = ToolCreationRequest::new(
            "read files",
            "file system",
            vec!["bash".to_string(), "read".to_string()],
        );
        assert_eq!(req.pattern_description, "read files");
        assert_eq!(req.context, "file system");
        assert_eq!(req.available_tools, vec!["bash", "read"]);
    }

    #[tokio::test]
    async fn test_create_tool_insufficient_frequency() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        let result = creator
            .create_tool_from_pattern("rare_pattern", "ctx", vec![])
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("minimum required"));
    }

    #[tokio::test]
    async fn test_create_tool_sufficient_frequency() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        creator.observe_pattern("common_pattern");
        creator.observe_pattern("common_pattern");

        let result = creator
            .create_tool_from_pattern("common_pattern", "ctx", vec![])
            .await;
        assert!(result.is_ok());
        let tool = result.unwrap();
        assert_eq!(tool.name, "common_pattern");
        assert_eq!(tool.test_coverage, 1.0);
        assert_eq!(creator.tool_count(), 1);
    }

    #[tokio::test]
    async fn test_create_tool_test_failure() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysFailTester));
        creator.observe_pattern("failing_pattern");
        creator.observe_pattern("failing_pattern");

        let result = creator
            .create_tool_from_pattern("failing_pattern", "ctx", vec![])
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation failed"));
    }

    #[tokio::test]
    async fn test_validate_and_register_pass() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        let tool = GeneratedTool::new("test_tool", "code", "desc");

        let result = creator.validate_and_register(tool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().test_coverage, 1.0);
        assert_eq!(creator.tool_count(), 1);
    }

    #[tokio::test]
    async fn test_validate_and_register_fail() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysFailTester));
        let tool = GeneratedTool::new("test_tool", "code", "desc");

        let result = creator.validate_and_register(tool).await;
        assert!(result.is_err());
        assert_eq!(creator.tool_count(), 0);
    }

    #[tokio::test]
    async fn test_get_tool() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        creator.observe_pattern("lookup_pattern");
        creator.observe_pattern("lookup_pattern");

        creator
            .create_tool_from_pattern("lookup_pattern", "ctx", vec![])
            .await
            .unwrap();

        assert!(creator.get_tool("lookup_pattern").is_some());
        assert!(creator.get_tool("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_list_tools() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        assert!(creator.list_tools().is_empty());

        creator.observe_pattern("list_a");
        creator.observe_pattern("list_a");
        creator.observe_pattern("list_b");
        creator.observe_pattern("list_b");

        creator
            .create_tool_from_pattern("list_a", "ctx", vec![])
            .await
            .unwrap();
        creator
            .create_tool_from_pattern("list_b", "ctx", vec![])
            .await
            .unwrap();

        assert_eq!(creator.list_tools().len(), 2);
    }

    #[tokio::test]
    async fn test_improve_tool_success() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        creator.observe_pattern("improve_me");
        creator.observe_pattern("improve_me");

        creator
            .create_tool_from_pattern("improve_me", "ctx", vec![])
            .await
            .unwrap();

        let result = creator.improve_tool("improve_me", "some error").await;
        assert!(result.is_ok());
        let improved = result.unwrap();
        assert!(improved.code.contains("_v2"));
    }

    #[tokio::test]
    async fn test_improve_tool_not_found() {
        let _creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        let result = AutoToolCreator::improve_tool(
            &mut AutoToolCreator::new(
                AutoToolCreatorConfig::default(),
                Box::new(MockLlmProvider),
                Box::new(AlwaysPassTester),
            ),
            "ghost",
            "error",
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_improve_tool_still_fails() {
        let test_config = AutoToolCreatorConfig {
            require_tests: true,
            min_pattern_frequency: 1,
            ..Default::default()
        };
        let mut reg_creator = AutoToolCreator::new(
            test_config,
            Box::new(MockLlmProvider),
            Box::new(AlwaysFailTester),
        );

        let tool = GeneratedTool::new("broken", "code", "desc");
        reg_creator.created_tools.insert("broken".to_string(), tool);

        let result = reg_creator.improve_tool("broken", "error").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("still fails"));
    }

    #[tokio::test]
    async fn test_create_tool_without_tests() {
        let config = AutoToolCreatorConfig {
            min_pattern_frequency: 1,
            max_code_length: 4096,
            require_tests: false,
            sandbox_timeout_ms: 5000,
        };
        let mut creator =
            AutoToolCreator::new(config, Box::new(MockLlmProvider), Box::new(AlwaysFailTester));

        creator.observe_pattern("no_test_pattern");

        let result = creator
            .create_tool_from_pattern("no_test_pattern", "ctx", vec![])
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().test_coverage, 0.0);
        assert_eq!(creator.tool_count(), 1);
    }

    #[tokio::test]
    async fn test_create_tool_code_too_long() {
        let config = AutoToolCreatorConfig {
            min_pattern_frequency: 1,
            max_code_length: 5,
            require_tests: false,
            sandbox_timeout_ms: 5000,
        };
        let mut creator =
            AutoToolCreator::new(config, Box::new(MockLlmProvider), Box::new(AlwaysPassTester));

        creator.observe_pattern("long_code");

        let result = creator
            .create_tool_from_pattern("long_code", "ctx", vec![])
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum"));
    }

    #[test]
    fn test_remove_tool() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        let tool = GeneratedTool::new("removable", "code", "desc");
        creator.created_tools.insert("removable".to_string(), tool);

        assert!(creator.remove_tool("removable").is_some());
        assert!(creator.get_tool("removable").is_none());
        assert!(creator.remove_tool("nonexistent").is_none());
    }

    #[test]
    fn test_observe_pattern_and_frequency() {
        let mut creator = make_creator_with_tester(Box::new(AlwaysPassTester));
        assert_eq!(creator.pattern_frequency("x"), 0);

        creator.observe_pattern("x");
        assert_eq!(creator.pattern_frequency("x"), 1);

        creator.observe_pattern("x");
        assert_eq!(creator.pattern_frequency("x"), 2);
    }

    #[tokio::test]
    async fn test_default_llm_provider_generate() {
        let provider = DefaultLlmToolProvider::new();
        let request =
            ToolCreationRequest::new("search files", "workspace", vec!["bash".to_string()]);

        let result = provider.generate_tool_code(&request).await;
        assert!(result.is_ok());
        let tool = result.unwrap();
        assert_eq!(tool.name, "search_files");
        assert!(tool.code.contains("search files"));
        assert!(tool.code.contains("bash"));
    }

    #[tokio::test]
    async fn test_default_llm_provider_improve() {
        let provider = DefaultLlmToolProvider::new();
        let tool = GeneratedTool::new("my_tool", "original code", "desc");

        let result = provider.improve_tool_code(&tool, "crashed").await;
        assert!(result.is_ok());
        let improved = result.unwrap();
        assert!(improved.code.contains("crashed"));
        assert!(improved.code.contains("try"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello_world");
        assert_eq!(slugify("read-files-now"), "read_files_now");
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify("already_good"), "already_good");
        assert_eq!(slugify("CamelCase"), "camelcase");
    }

    #[test]
    fn test_generated_tool_serialization() {
        let tool = GeneratedTool::new("ser_tool", "code", "desc");
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: GeneratedTool = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "ser_tool");
        assert_eq!(deserialized.code, "code");
        assert_eq!(deserialized.description, "desc");
    }

    #[test]
    fn test_tool_test_result_serialization() {
        let passed = ToolTestResult::passed("ok", 100);
        let json = serde_json::to_string(&passed).unwrap();
        let deserialized: ToolTestResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.passed);

        let failed = ToolTestResult::failed("err", 50);
        let json = serde_json::to_string(&failed).unwrap();
        let deserialized: ToolTestResult = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.passed);
    }
}
