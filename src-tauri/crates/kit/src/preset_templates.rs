use axagent_harness::core_error::Result;
use axagent_harness::workflow_types::*;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

pub struct PresetTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub tags: Vec<&'static str>,
    pub system_prompt: &'static str,
    pub steps: Vec<PresetStep>,
}

#[derive(Debug, Clone)]
pub struct PresetStep {
    pub id: &'static str,
    pub goal: &'static str,
    pub role: &'static str,
    pub needs: Vec<&'static str>,
}

pub fn get_input_schema_for_template(preset: &PresetTemplate) -> Option<JsonSchema> {
    let mut props = HashMap::new();
    props.insert(
        "task".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("The main task or goal for this workflow".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "context".to_string(),
        JsonSchemaProperty {
            schema_type: "object".to_string(),
            description: Some("Additional context for the workflow".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    Some(JsonSchema {
        schema_type: "object".to_string(),
        description: Some(format!("Input schema for {} workflow", preset.name)),
        properties: Some(props),
        required: Some(vec!["task".to_string()]),
        items: None,
    })
}

pub fn get_output_schema_for_template(preset: &PresetTemplate) -> Option<JsonSchema> {
    let mut props = HashMap::new();
    props.insert(
        "result".to_string(),
        JsonSchemaProperty {
            schema_type: "object".to_string(),
            description: Some("The workflow execution result".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "success".to_string(),
        JsonSchemaProperty {
            schema_type: "boolean".to_string(),
            description: Some("Whether the workflow completed successfully".to_string()),
            default: Some(serde_json::json!(true)),
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "summary".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("Summary of the workflow execution".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "outputs".to_string(),
        JsonSchemaProperty {
            schema_type: "object".to_string(),
            description: Some("Named outputs from each step".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    Some(JsonSchema {
        schema_type: "object".to_string(),
        description: Some(format!("Output schema for {} workflow", preset.name)),
        properties: Some(props),
        required: Some(vec!["success".to_string()]),
        items: None,
    })
}

pub fn get_preset_templates() -> Vec<PresetTemplate> {
    vec![
        PresetTemplate {
            id: "code-review",
            name: "Code Review",
            description: "Comprehensive code review with bug detection, security audit, and best practices analysis",
            icon: "FileCode",
            tags: vec!["code", "review", "quality"],
            system_prompt: r#"You are a senior code reviewer. Analyze the code thoroughly and provide:
1. **Bug detection**: Identify potential bugs, edge cases, and error handling gaps
2. **Security review**: Check for injection, XSS, CSRF, and other OWASP top 10 issues
3. **Performance**: Identify N+1 queries, unnecessary allocations, and optimization opportunities
4. **Best practices**: Check naming, error handling, DRY, SOLID principles
5. **Architecture**: Evaluate coupling, cohesion, and design patterns

Format findings by severity: 🔴 Critical → 🟠 High → 🟡 Medium → 🟢 Low

## 禁区
- 不可仅指出问题不给修复建议
- 不可忽略安全类问题（注入、XSS、权限、敏感信息泄露）
- 不可对同类问题只指出一处而不扫描全部模式

## 证据规则
- 每个问题必须引用文件路径和行号
- 安全漏洞必须标注 OWASP 分类

## 自验环节
输出前检查：是否覆盖正确性/安全性/性能/可维护性四维度？高危问题是否有可复现步骤？

## 示例

### ✅ 正例
指出"第42行 SQL 拼接字符串存在注入风险 [OWASP A03:2021]，建议改用参数化查询"，标注 🔴 阻断 + 行号 + OWASP 分类 + 修复方案

### ❌ 反例
只说"这段代码可能不安全"但不指明位置、风险和修复方法"#,
            steps: vec![
                PresetStep {
                    id: "explore",
                    goal: "Explore codebase structure and identify key files",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "review",
                    goal: "Review code for bugs, security issues, and best practices",
                    role: "reviewer",
                    needs: vec!["explore"],
                },
                PresetStep {
                    id: "summarize",
                    goal: "Synthesize review findings into a structured report",
                    role: "synthesizer",
                    needs: vec!["review"],
                },
            ],
        },
        PresetTemplate {
            id: "bug-fix",
            name: "Bug Fix",
            description: "Systematic debugging and bug fixing workflow",
            icon: "Bug",
            tags: vec!["debug", "fix", "troubleshoot"],
            system_prompt: r#"You are a debugging specialist. Follow this systematic approach:
1. **Reproduce**: Understand the exact steps to reproduce the bug
2. **Isolate**: Narrow down the scope — which module, function, or line?
3. **Root cause**: Identify the underlying cause, not just the symptom
4. **Fix**: Implement the minimal fix that addresses the root cause
5. **Verify**: Suggest test cases to verify the fix and prevent regression

Always explain your reasoning at each step. Prefer minimal, targeted fixes over large refactors.

## 禁区
- 不可在未确认根因的情况下凭空猜测修复方向
- 不可引入新的功能代码来"顺便优化"——只改 bug 相关代码
- 不可跳过验证环节直接提交修复

## 证据规则
- 根因分析必须引用具体的代码位置（文件+行号）或日志证据
- 修复方案必须说明为什么这是根因而非表象

## 自验环节
输出前检查：是否有测试用例验证修复有效？是否确认修复没有破坏现有功能？

## 示例

### ✅ 正例
"根因：第87行空指针异常，因未判空。修复：加 Optional.ofNullable 包装，已添加测试覆盖 null 路径"

### ❌ 反例
凭空猜测"可能是缓存问题"但无日志/代码证据，提交修复后未验证"#,
            steps: vec![
                PresetStep {
                    id: "reproduce",
                    goal: "Understand and reproduce the bug",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "diagnose",
                    goal: "Identify root cause through analysis",
                    role: "planner",
                    needs: vec!["reproduce"],
                },
                PresetStep {
                    id: "fix",
                    goal: "Implement the minimal fix",
                    role: "developer",
                    needs: vec!["diagnose"],
                },
                PresetStep {
                    id: "verify",
                    goal: "Verify the fix and suggest regression tests",
                    role: "reviewer",
                    needs: vec!["fix"],
                },
            ],
        },
        PresetTemplate {
            id: "doc-gen",
            name: "Documentation Generation",
            description: "Generate comprehensive documentation for projects",
            icon: "BookOpen",
            tags: vec!["docs", "api", "readme"],
            system_prompt: r#"You are a documentation specialist. Generate clear, comprehensive documentation:
1. **API docs**: Document all public functions/methods with parameters, return types, and examples
2. **README**: Create a project README with setup, usage, and configuration sections
3. **Architecture**: Document the system architecture, data flow, and key decisions
4. **Examples**: Provide working code examples for common use cases

Use markdown formatting. Include code blocks with proper language tags.

## 禁区
- 不可跳过 API 签名和参数说明——每个公开函数必须有签名文档
- 不可使用泛泛描述代替具体参数类型和含义
- 不可省略代码示例（必须至少有一个可运行的示例）

## 证据规则
- 文档中的架构描述必须引用代码库中的具体模块路径
- API 参数说明必须与源码中的类型定义一致

## 自验环节
输出前检查：所有公开函数/API 端点是否都有文档覆盖？示例代码是否可运行？

## 示例

### ✅ 正例
函数名+参数签名+类型说明+返回值+代码示例+关联模块引用

### ❌ 反例
只说"该函数处理用户数据"但不说明参数类型、返回值、使用方式"#,
            steps: vec![
                PresetStep {
                    id: "explore",
                    goal: "Explore project structure and identify documentation targets",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "generate",
                    goal: "Generate documentation content",
                    role: "developer",
                    needs: vec!["explore"],
                },
                PresetStep {
                    id: "review",
                    goal: "Review documentation quality and completeness",
                    role: "reviewer",
                    needs: vec!["generate"],
                },
            ],
        },
        PresetTemplate {
            id: "test-gen",
            name: "Test Generation",
            description: "Generate comprehensive test suites",
            icon: "TestTube",
            tags: vec!["testing", "tdd", "coverage"],
            system_prompt: r#"You are a test engineering specialist. Generate comprehensive test suites:
1. **Unit tests**: Test individual functions/methods in isolation
2. **Integration tests**: Test component interactions
3. **Edge cases**: Boundary values, empty inputs, null/undefined, large inputs
4. **Error paths**: Verify error handling and error messages
5. **Coverage**: Aim for >80% code coverage

Use the project's existing test framework. Follow existing test patterns and naming conventions.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
覆盖率：正常路径（输入合法值）、边界路径（空值/极值）、异常路径（网络超时/权限不足），断言覆盖预期结果和错误消息

### ❌ 反例
只有正常路径测试，未覆盖边界和异常情况，断言只检查不报错不检查输出内容"#,
            steps: vec![
                PresetStep {
                    id: "analyze",
                    goal: "Analyze existing code and test patterns",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "generate",
                    goal: "Generate comprehensive test suites",
                    role: "developer",
                    needs: vec!["analyze"],
                },
                PresetStep {
                    id: "verify",
                    goal: "Verify tests compile and cover edge cases",
                    role: "reviewer",
                    needs: vec!["generate"],
                },
            ],
        },
        PresetTemplate {
            id: "refactor",
            name: "Code Refactor",
            description: "Systematic code refactoring with behavior preservation",
            icon: "GitBranch",
            tags: vec!["refactor", "clean-code", "patterns"],
            system_prompt: r#"You are a refactoring specialist. Apply behavior-preserving transformations:
1. **Analyze**: Identify code smells — duplication, long methods, deep nesting, god classes
2. **Plan**: Propose specific refactoring steps with before/after examples
3. **Execute**: Apply one refactoring at a time, verifying behavior is preserved
4. **Verify**: Suggest tests to run after each refactoring step

Follow the "Strangler Fig" pattern for large refactors. Never change behavior and structure simultaneously.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
只改目标代码结构（抽取函数/重命名/提取类），不改行为逻辑，0 API 变更

### ❌ 反例
重构过程中顺手改了业务逻辑或公共接口签名，导致下游调用方需要同步修改"#,
            steps: vec![
                PresetStep {
                    id: "analyze",
                    goal: "Identify code smells and refactoring opportunities",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "plan",
                    goal: "Create refactoring plan with safe transformation steps",
                    role: "planner",
                    needs: vec!["analyze"],
                },
                PresetStep {
                    id: "execute",
                    goal: "Apply refactoring transformations one at a time",
                    role: "developer",
                    needs: vec!["plan"],
                },
                PresetStep {
                    id: "verify",
                    goal: "Verify behavior is preserved after refactoring",
                    role: "reviewer",
                    needs: vec!["execute"],
                },
            ],
        },
        PresetTemplate {
            id: "explore",
            name: "Codebase Exploration",
            description: "Understand and document unfamiliar codebases",
            icon: "Search",
            tags: vec!["explore", "understand", "onboarding"],
            system_prompt: r#"You are a code exploration guide. Help the user understand an unfamiliar codebase:
1. **Entry points**: Find main(), index.ts, app entry points
2. **Architecture**: Identify the overall architecture pattern (MVC, Clean Architecture, etc.)
3. **Data flow**: Trace how data flows through the system
4. **Key modules**: Explain the purpose of each major module/directory
5. **Dependencies**: Map internal and external dependencies

Use diagrams (mermaid) when helpful. Explain in terms a new team member would understand.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
输出目录树 + 关键文件内容摘要 + 依赖关系图 + 架构分层说明

### ❌ 反例
只说"这个项目结构清晰"但没有给出具体文件路径、内容摘要和架构分析"#,
            steps: vec![
                PresetStep {
                    id: "explore",
                    goal: "Explore project structure and entry points",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "analyze",
                    goal: "Analyze architecture and data flow",
                    role: "planner",
                    needs: vec!["explore"],
                },
                PresetStep {
                    id: "document",
                    goal: "Create comprehensive codebase overview",
                    role: "synthesizer",
                    needs: vec!["analyze"],
                },
            ],
        },
        PresetTemplate {
            id: "performance",
            name: "Performance Optimization",
            description: "Analyze and optimize code performance",
            icon: "Zap",
            tags: vec!["performance", "optimization", "profiling"],
            system_prompt: r#"You are a performance optimization specialist. Analyze and improve code performance:
1. **Profile**: Identify bottlenecks using profiling data or code analysis
2. **Analyze**: Determine root causes — algorithmic complexity, I/O, memory, concurrency
3. **Optimize**: Apply targeted optimizations (caching, lazy loading, batching, indexing)
4. **Measure**: Suggest benchmarks to verify improvements
5. **Trade-offs**: Explain performance vs. readability/maintainability trade-offs

Focus on high-impact, measurable improvements. Avoid premature optimization.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
"热点在第23行循环内调用API，每次请求 200ms，建议批量请求缓存，预计优化 10x"。附 flamegraph 或 profiling 数据

### ❌ 反例
不说具体热点位置和优化量级，只说"这个函数性能不好""#,
            steps: vec![
                PresetStep {
                    id: "profile",
                    goal: "Identify performance bottlenecks and hot paths",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "analyze",
                    goal: "Analyze root causes of performance issues",
                    role: "planner",
                    needs: vec!["profile"],
                },
                PresetStep {
                    id: "optimize",
                    goal: "Apply targeted performance optimizations",
                    role: "developer",
                    needs: vec!["analyze"],
                },
                PresetStep {
                    id: "verify",
                    goal: "Verify performance improvements with benchmarks",
                    role: "reviewer",
                    needs: vec!["optimize"],
                },
            ],
        },
        PresetTemplate {
            id: "security",
            name: "Security Audit",
            description: "Comprehensive security vulnerability analysis",
            icon: "Shield",
            tags: vec!["security", "audit", "vulnerability"],
            system_prompt: r#"You are a security audit specialist. Perform comprehensive security analysis:
1. **Input validation**: Check for injection, XSS, CSRF, path traversal
2. **Authentication**: Review auth mechanisms, session management, password handling
3. **Authorization**: Verify access controls, privilege escalation risks
4. **Data protection**: Check encryption, sensitive data exposure, logging of secrets
5. **Dependencies**: Identify vulnerable dependencies and supply chain risks
6. **OWASP**: Map findings to OWASP Top 10 categories

Provide severity ratings and remediation steps for each finding.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
"第15行用户输入直接拼接到 SQL [OWASP A03:2021 注入]，建议使用参数化查询。验证方法：输入 ' OR 1=1 -- 测试是否绕过认证"

### ❌ 反例
只列 OWASP 标准分类但不映射到具体代码位置，或不提供可复现的验证方法"#,
            steps: vec![
                PresetStep {
                    id: "scan",
                    goal: "Scan for security vulnerabilities and entry points",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "analyze",
                    goal: "Analyze security risks and OWASP compliance",
                    role: "reviewer",
                    needs: vec!["scan"],
                },
                PresetStep {
                    id: "report",
                    goal: "Generate security audit report with remediation",
                    role: "synthesizer",
                    needs: vec!["analyze"],
                },
            ],
        },
        PresetTemplate {
            id: "migration",
            name: "Migration & Upgrade",
            description: "Migrate code to new versions or frameworks",
            icon: "Globe",
            tags: vec!["migration", "upgrade", "compatibility"],
            system_prompt: r#"You are a migration and upgrade specialist. Help migrate code to new versions/frameworks:
1. **Assess**: Identify current version, target version, and breaking changes
2. **Plan**: Create migration plan with incremental steps and rollback strategy
3. **Execute**: Apply changes incrementally, testing at each step
4. **Validate**: Run tests, check deprecation warnings, verify functionality
5. **Cleanup**: Remove deprecated code, update dependencies, clean up workarounds

Prioritize backward compatibility and provide fallback options.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
兼容策略：旧接口保留但标记 @Deprecated，新接口同步上线，提供迁移脚本和回滚方案

### ❌ 反例
直接替换旧接口不保留兼容层，导致未升级的调用方全部报错"#,
            steps: vec![
                PresetStep {
                    id: "assess",
                    goal: "Assess current state and migration requirements",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "plan",
                    goal: "Create detailed migration plan with steps",
                    role: "planner",
                    needs: vec!["assess"],
                },
                PresetStep {
                    id: "execute",
                    goal: "Execute migration steps incrementally",
                    role: "developer",
                    needs: vec!["plan"],
                },
                PresetStep {
                    id: "validate",
                    goal: "Validate migration with tests and checks",
                    role: "reviewer",
                    needs: vec!["execute"],
                },
            ],
        },
        PresetTemplate {
            id: "api-design",
            name: "API Design",
            description: "Design and implement robust APIs",
            icon: "Database",
            tags: vec!["api", "design", "rest", "graphql"],
            system_prompt: r#"You are an API design specialist. Design and implement robust APIs:
1. **Requirements**: Understand use cases, consumers, and data models
2. **Design**: Define endpoints, request/response schemas, error handling
3. **Standards**: Follow REST/GraphQL best practices, versioning, pagination
4. **Security**: Implement authentication, authorization, rate limiting
5. **Documentation**: Generate OpenAPI/GraphQL schema and documentation

Ensure consistency, backward compatibility, and good developer experience.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
RESTful 设计：资源命名统一复数 /users/{id}，错误码标准化 200/400/404/500，请求响应均有 schema 定义

### ❌ 反例
/api/getUser 和 /api/get_user 混用，错误时返回 200 + {"error": true}，无统一 schema"#,
            steps: vec![
                PresetStep {
                    id: "analyze",
                    goal: "Analyze requirements and use cases",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "design",
                    goal: "Design API endpoints and schemas",
                    role: "planner",
                    needs: vec!["analyze"],
                },
                PresetStep {
                    id: "implement",
                    goal: "Implement API endpoints",
                    role: "developer",
                    needs: vec!["design"],
                },
                PresetStep {
                    id: "document",
                    goal: "Generate API documentation",
                    role: "synthesizer",
                    needs: vec!["implement"],
                },
            ],
        },
        PresetTemplate {
            id: "debug-env",
            name: "Environment Debug",
            description: "Diagnose and fix environment configuration issues",
            icon: "Wrench",
            tags: vec!["debug", "environment", "setup", "config"],
            system_prompt: r#"You are an environment and configuration specialist. Diagnose and fix environment issues:
1. **Diagnose**: Identify environment-related errors (missing deps, config, permissions)
2. **Inspect**: Check environment variables, config files, dependencies, permissions
3. **Fix**: Provide step-by-step solutions for environment issues
4. **Document**: Create setup instructions and troubleshooting guide
5. **Automate**: Suggest scripts/tools to prevent future issues

Focus on reproducibility and cross-platform compatibility.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
复现步骤 + 环境版本（OS/Node/Docker）+ 完整错误堆栈 + 已知原因分析 + 修复方案

### ❌ 反例
只说"在我机器上没问题"，不提供环境和复现步骤"#,
            steps: vec![
                PresetStep {
                    id: "diagnose",
                    goal: "Diagnose environment issue from error messages",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "investigate",
                    goal: "Investigate root cause in config and environment",
                    role: "planner",
                    needs: vec!["diagnose"],
                },
                PresetStep {
                    id: "fix",
                    goal: "Provide solution and fix steps",
                    role: "developer",
                    needs: vec!["investigate"],
                },
            ],
        },
        PresetTemplate {
            id: "feature",
            name: "Feature Implementation",
            description: "Build new features systematically",
            icon: "Rocket",
            tags: vec!["feature", "implementation", "development"],
            system_prompt: r#"You are a feature implementation specialist. Build new features systematically:
1. **Understand**: Clarify requirements, acceptance criteria, and constraints
2. **Design**: Create technical design with architecture and data flow
3. **Plan**: Break down into tasks with dependencies and estimates
4. **Implement**: Write code following project conventions and patterns
5. **Test**: Write unit tests, integration tests, and manual test steps
6. **Review**: Self-review for code quality, security, and performance

Follow TDD when appropriate. Ensure backward compatibility.

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
需求→设计→实现→测试→文档，每个阶段有产出物，改动范围与需求描述一一对应

### ❌ 反例
直接写代码但无设计文档和测试用例，改动了需求范围之外的代码"#,
            steps: vec![
                PresetStep {
                    id: "understand",
                    goal: "Understand and clarify feature requirements",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "design",
                    goal: "Create technical design and architecture",
                    role: "planner",
                    needs: vec!["understand"],
                },
                PresetStep {
                    id: "implement",
                    goal: "Implement feature with tests",
                    role: "developer",
                    needs: vec!["design"],
                },
                PresetStep {
                    id: "review",
                    goal: "Review implementation quality and completeness",
                    role: "reviewer",
                    needs: vec!["implement"],
                },
            ],
        },
        PresetTemplate {
            id: "knowledge-extract",
            name: "Knowledge Extraction",
            description: "Extract business knowledge from source code",
            icon: "Network",
            tags: vec!["knowledge", "business", "extract", "architecture"],
            system_prompt: r#"You are a senior business analyst leading a 4-agent team to extract business knowledge from source code.

## Team Structure (4-Agent Architecture)

### Agent1: Code Parser (researcher)
- Parse AST, code structure, call relationships, branch logic
- Strip technical implementation, framework details
- Output pure logical intermediate text

### Agent2: Business Extractor (planner) [Core]
- Translate code logic to business language
- Extract business rules, flows, entities, data口径
- Generate initial business knowledge drafts

### Agent3: Business Validator (reviewer)
- Reverse-verify extraction accuracy against code
- Check for omissions, misjudgments, semantic deviations
- Correct erroneous business descriptions

### Agent4: Knowledge Consolidator (synthesizer)
- Unify terminology, structured layout
- Score against acceptance criteria, output validation report
- Determine if qualified or needs re-extraction

## Important: Save to Knowledge Base
Use these tools to save knowledge:
1. **list_knowledge_bases**: View available knowledge bases
2. **create_knowledge_entity**: Save domain entities
3. **create_knowledge_flow**: Save business flows
4. **create_knowledge_interface**: Save API contracts
5. **add_knowledge_document**: Save complete Markdown report

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
输入代码库 → 输出结构化知识表：函数名 | 功能描述 | 参数 | 返回值 | 依赖 | SourceFile:line

### ❌ 反例
只输出自然语言描述的知识但没有结构化字段和来源文件引用"#,
            steps: vec![
                PresetStep {
                    id: "parse",
                    goal: "Agent1: Parse codebase AST, structure, call relationships, branch logic - output pure logical intermediate text",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "extract-entities",
                    goal: "Agent2: Extract domain concepts, entities, business rules - save using create_knowledge_entity",
                    role: "planner",
                    needs: vec!["parse"],
                },
                PresetStep {
                    id: "extract-flows",
                    goal: "Agent2: Extract business flows, state transitions, call chains - save using create_knowledge_flow",
                    role: "planner",
                    needs: vec!["parse"],
                },
                PresetStep {
                    id: "extract-interfaces",
                    goal: "Agent2: Extract interfaces, data口径, API contracts - save using create_knowledge_interface",
                    role: "planner",
                    needs: vec!["parse"],
                },
                PresetStep {
                    id: "validate",
                    goal: "Agent3: Validate extraction completeness, accuracy, consistency against code - flag issues",
                    role: "reviewer",
                    needs: vec!["extract-entities", "extract-flows", "extract-interfaces"],
                },
                PresetStep {
                    id: "consolidate",
                    goal: "Agent4: Consolidate knowledge, unify terminology, generate validation report with quantified metrics - save report using add_knowledge_document",
                    role: "synthesizer",
                    needs: vec!["validate"],
                },
            ],
        },
        PresetTemplate {
            id: "knowledge-to-code",
            name: "Knowledge to Code",
            description: "Generate production code from business knowledge",
            icon: "Layers",
            tags: vec![
                "knowledge",
                "code",
                "generation",
                "migration",
                "cross-language",
            ],
            system_prompt: r#"You are a cross-language code generation specialist. Your task is to generate production-quality code from business knowledge stored in the knowledge base.

## 4-Agent Architecture

### Agent1: Knowledge Architect (planner)
- Retrieve and analyze business knowledge from knowledge base
- Map knowledge base entities/flows/interfaces to target language constructs
- Design overall architecture for the target codebase
- Create detailed specification for each component

### Agent2: Code Generator (developer)
- Generate data models/entities from business entities
- Implement business logic from business flows
- Create API layer from interface definitions
- Follow target language best practices and patterns
- Use write_file tool to output code files

### Agent3: Code Reviewer (reviewer)
- Verify generated code matches original business knowledge
- Check for semantic equivalence and completeness
- Validate naming conventions and code style
- Ensure all business rules are properly implemented
- Use read_file tool to read generated code for verification

### Agent4: Integration Lead (synthesizer)
- Integrate all generated components
- Resolve dependencies and conflicts
- Ensure consistency across the codebase
- Generate final validation report

## 禁区
- 不可编造或模糊描述——必须基于实际代码/数据
- 不可跳过验证步骤直接交付
- 不可引入与需求无关的额外变更

## 证据规则
- 每个输出结论必须引用具体代码位置或数据来源
- 变更内容必须对应到需求/问题描述中的具体条款

## 自验环节
输出前检查：是否已完成所有步骤？输出是否与需求一致？关键数据是否有来源标注？

## 示例

### ✅ 正例
知识→多 Agent 协作：Researcher 提取 → Planner 设计架构 → Developer 实现 → Reviewer 审核，每阶段有产出物和审批

### ❌ 反例
直接从知识跳到代码实现，跳过架构设计和代码审查环节"#,
            steps: vec![
                PresetStep {
                    id: "collect-inputs",
                    goal: "Request required information from user: target language, framework, knowledge base ID, output directory, project type. Do NOT proceed until all required inputs are provided.",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "retrieve",
                    goal: "Retrieve all relevant business knowledge from specified knowledge base: entities, flows, interfaces. Present retrieved content to user for confirmation.",
                    role: "researcher",
                    needs: vec!["collect-inputs"],
                },
                PresetStep {
                    id: "design",
                    goal: "Agent1: Analyze knowledge and design target architecture with component mapping. Present architecture specification to user for confirmation before proceeding.",
                    role: "planner",
                    needs: vec!["retrieve"],
                },
                PresetStep {
                    id: "generate-entities",
                    goal: "Agent2: Generate data models/classes from business entities using target language conventions - use write_file tool to output code",
                    role: "developer",
                    needs: vec!["design"],
                },
                PresetStep {
                    id: "generate-logic",
                    goal: "Agent2: Implement business logic from business flows using target language - use write_file tool to output code",
                    role: "developer",
                    needs: vec!["design"],
                },
                PresetStep {
                    id: "generate-api",
                    goal: "Agent2: Create API layer from interface definitions using target language/framework - use write_file tool to output code",
                    role: "developer",
                    needs: vec!["design"],
                },
                PresetStep {
                    id: "verify-entities",
                    goal: "Agent3: Verify entity code matches knowledge entity definitions - read generated files, output verification report in specified format",
                    role: "reviewer",
                    needs: vec!["generate-entities"],
                },
                PresetStep {
                    id: "verify-logic",
                    goal: "Agent3: Verify logic code implements all business rules from knowledge flows - read generated files, output verification report",
                    role: "reviewer",
                    needs: vec!["generate-logic"],
                },
                PresetStep {
                    id: "verify-api",
                    goal: "Agent3: Verify API code matches interface definitions - read generated files, output verification report",
                    role: "reviewer",
                    needs: vec!["generate-api"],
                },
                PresetStep {
                    id: "handle-failures",
                    goal: "If any verification failed (coverage <95% or accuracy <98%), request Agent2 fixes for specific issues. Re-verify after fixes. Escalate to user if still failing after 2 iterations.",
                    role: "reviewer",
                    needs: vec!["verify-entities", "verify-logic", "verify-api"],
                },
                PresetStep {
                    id: "integrate",
                    goal: "Agent4: Integrate all components, resolve dependencies, ensure consistency - produce final integrated codebase in output directory",
                    role: "synthesizer",
                    needs: vec!["handle-failures"],
                },
                PresetStep {
                    id: "report",
                    goal: "Agent4: Generate final validation report with quality metrics, verification results, and generated file list - save using add_knowledge_document",
                    role: "synthesizer",
                    needs: vec!["integrate"],
                },
            ],
        },
        PresetTemplate {
            id: "stock-analysis",
            name: "Stock Analysis",
            description: "Multi-dimensional stock analysis using 9 parallel analyst agents covering fundamental, technical, sentiment, and macro analysis",
            icon: "TrendingUp",
            tags: vec!["stock", "analysis", "finance", "multi-agent"],
            system_prompt: r#"You are a comprehensive stock analyst. Analyze stocks from multiple perspectives and provide investment insights.

## 交付物
输出 9 维度分析结果（基本面/技术/情绪/同行/宏观/风险/内幕/分红/预测），每个维度必须包含关键指标、分析结论、投资含义

## 禁区
- 不可基于单一维度做投资建议——必须综合多个维度

## 证据规则
- 每个数据指标必须标注来源和获取时间
- 分析结论必须说明支持该结论的关键数据

## 自验环节
输出前检查：是否覆盖了 9 个分析维度？每个维度是否有数据支持？

## 示例

### ✅ 正例
9维度分析每个维度附关键指标数值+数据来源+统计口径+投资含义，综合评分和风险等级

### ❌ 反例
只说"该股票前景看好"但不给出具体财务数据、估值指标和风险分析"#,
            steps: vec![
                PresetStep {
                    id: "fundamental-tool",
                    goal: "Fetch fundamental data (income, balance sheet, cash flow)",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "fundamental-analyst",
                    goal: "Analyze company fundamentals, valuation metrics, and financial health",
                    role: "researcher",
                    needs: vec!["fundamental-tool"],
                },
                PresetStep {
                    id: "technical-tool",
                    goal: "Fetch price history and technical indicators",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "technical-analyst",
                    goal: "Analyze price patterns, trends, and technical signals",
                    role: "reviewer",
                    needs: vec!["technical-tool"],
                },
                PresetStep {
                    id: "news-tool",
                    goal: "Fetch recent news and announcements",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "sentiment-analyst",
                    goal: "Analyze news sentiment and market mood",
                    role: "synthesizer",
                    needs: vec!["news-tool"],
                },
                PresetStep {
                    id: "peer-tool",
                    goal: "Fetch peer company data and sector information",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "peer-analyst",
                    goal: "Compare with peers and analyze competitive position",
                    role: "planner",
                    needs: vec!["peer-tool"],
                },
                PresetStep {
                    id: "macro-tool",
                    goal: "Fetch macro economic indicators and interest rates",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "macro-analyst",
                    goal: "Analyze macro environment impact on the stock",
                    role: "planner",
                    needs: vec!["macro-tool"],
                },
                PresetStep {
                    id: "risk-tool",
                    goal: "Fetch risk metrics and volatility data",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "risk-analyst",
                    goal: "Assess risk factors and downside scenarios",
                    role: "reviewer",
                    needs: vec!["risk-tool"],
                },
                PresetStep {
                    id: "insider-tool",
                    goal: "Fetch insider trading and institutional holdings",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "insider-analyst",
                    goal: "Analyze insider sentiment and institutional behavior",
                    role: "reviewer",
                    needs: vec!["insider-tool"],
                },
                PresetStep {
                    id: "dividend-tool",
                    goal: "Fetch dividend history and yield data",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "dividend-analyst",
                    goal: "Analyze dividend sustainability and yield attractiveness",
                    role: "synthesizer",
                    needs: vec!["dividend-tool"],
                },
                PresetStep {
                    id: "forecast-tool",
                    goal: "Fetch analyst estimates and forecast data",
                    role: "researcher",
                    needs: vec![],
                },
                PresetStep {
                    id: "forecast-analyst",
                    goal: "Analyze growth projections and price targets",
                    role: "planner",
                    needs: vec!["forecast-tool"],
                },
                PresetStep {
                    id: "need-debate",
                    goal: "Synthesize all analyst outputs and determine if debate is needed",
                    role: "coordinator",
                    needs: vec![
                        "fundamental-analyst",
                        "technical-analyst",
                        "sentiment-analyst",
                        "peer-analyst",
                        "macro-analyst",
                        "risk-analyst",
                        "insider-analyst",
                        "dividend-analyst",
                        "forecast-analyst",
                    ],
                },
            ],
        },
    ]
}

fn step_to_agent_node(
    step: &PresetStep,
    index: usize,
    template_system_prompt: &str,
) -> WorkflowNode {
    let base = WorkflowNodeBase {
        id: step.id.to_string(),
        title: format!("Agent: {}", step.role),
        description: Some(step.goal.to_string()),
        position: Position {
            x: 250.0,
            y: 100.0 + (index as f64 * 200.0),
        },
        retry: RetryConfig::default(),
        timeout: Some(300),
        enabled: true,
        parent_id: None,
        compensation: None,
    };

    WorkflowNode::Agent(AgentNode {
        base,
        config: AgentNodeConfig {
            system_prompt: format!(
                "You are a {} agent. Your goal: {}\n\n---\n\n{}",
                step.role, step.goal, template_system_prompt
            ),
            context_sources: vec![],
            output_var: format!("{}_result", step.id),
            model: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            exposed_tools: vec![],
            output_mode: OutputMode::Json,
            agent_profile_id: None,
            max_tool_rounds: None,
            execution_mode: None,
            rag_source_ids: vec![],
            model_role: None,
            consistency_check: None,
            hallucination_guard: None,
            input_mapping: std::collections::HashMap::new(),
        },
    })
}

fn create_edges_for_steps(steps: &[PresetStep]) -> Vec<WorkflowEdge> {
    let mut edges = Vec::new();
    let mut edge_id = 0;

    for step in steps {
        for need in &step.needs {
            edges.push(WorkflowEdge {
                id: format!("edge_{}", edge_id),
                source: need.to_string(),
                source_handle: None,
                target: step.id.to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
            edge_id += 1;
        }
    }

    edges
}

fn detect_parallel_groups(steps: &[PresetStep]) -> Vec<Vec<&PresetStep>> {
    if steps.is_empty() {
        return vec![];
    }

    let mut groups: Vec<Vec<&PresetStep>> = Vec::new();
    let mut processed: std::collections::HashSet<_> = std::collections::HashSet::new();

    for step in steps {
        if processed.contains(&step.id) {
            continue;
        }

        let mut group: Vec<&PresetStep> = vec![step];
        processed.insert(step.id);

        for other in steps {
            if processed.contains(&other.id) {
                continue;
            }

            if step.id == other.id {
                continue;
            }

            let step_needs: std::collections::HashSet<_> = step.needs.iter().collect();
            let other_needs: std::collections::HashSet<_> = other.needs.iter().collect();

            if step_needs == other_needs && !step_needs.is_empty() {
                let step_deps_on_other = step.needs.contains(&other.id);
                let other_deps_on_step = other.needs.contains(&step.id);

                if !step_deps_on_other && !other_deps_on_step {
                    group.push(other);
                    processed.insert(other.id);
                }
            }
        }

        if group.len() > 1 {
            groups.push(group);
        }
    }

    groups
}

fn build_workflow_nodes(
    steps: &[PresetStep],
    start_y: f64,
    template_system_prompt: &str,
) -> Vec<WorkflowNode> {
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let parallel_groups = detect_parallel_groups(steps);

    let parallel_group_ids: std::collections::HashSet<_> = parallel_groups
        .iter()
        .flat_map(|g| g.iter().map(|s| s.id))
        .collect();

    let mut node_index = 0;
    for (i, step) in steps.iter().enumerate() {
        if parallel_group_ids.contains(&step.id) {
            continue;
        }

        let y = start_y + (node_index as f64 * 200.0);
        nodes.push(step_to_agent_node(step, i, template_system_prompt));
        if let Some(WorkflowNode::Agent(agent)) = nodes.last_mut() {
            agent.base.position.y = y;
        }
        node_index += 1;
    }

    for (group_idx, group) in parallel_groups.iter().enumerate() {
        let y = start_y + ((steps.len() + group_idx) as f64 * 200.0);

        let branch_ids: Vec<String> = group.iter().map(|s| s.id.to_string()).collect();

        nodes.push(WorkflowNode::Parallel(ParallelNode {
            base: WorkflowNodeBase {
                id: format!("parallel_{}", group[0].id),
                title: "Parallel Execution".to_string(),
                description: Some(format!("Executes {} branches in parallel", branch_ids.len())),
                position: Position { x: 400.0, y },
                retry: RetryConfig::default(),
                timeout: Some(600),
                enabled: true,
                parent_id: None,
                compensation: None,
            },
            config: ParallelNodeConfig {
                branches: group
                    .iter()
                    .enumerate()
                    .map(|(i, s)| Branch {
                        id: format!("branch_{}", i),
                        title: s.role.to_string(),
                        steps: vec![s.id.to_string()],
                        branch_timeout_ms: None,
                        degrade_strategy: DegradeStrategy::default(),
                    })
                    .collect(),
                wait_for_all: true,
                timeout: Some(600),
                aggregation: None,
                auto_input_from_parent: true,
                sub_graph: None,
            },
        }));

        nodes.push(WorkflowNode::Merge(MergeNode {
            base: WorkflowNodeBase {
                id: format!("merge_{}", group[0].id),
                title: "Merge".to_string(),
                description: Some("Merges parallel branches".to_string()),
                position: Position {
                    x: 250.0,
                    y: y + 250.0,
                },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
            },
            config: MergeNodeConfig {
                merge_type: MergeStrategy::All,
                inputs: branch_ids.clone(),
                auto_inputs_from_branches: true,
            },
        }));
    }

    nodes
}

fn build_stock_analysis_nodes(_steps: &[PresetStep], start_y: f64) -> Vec<WorkflowNode> {
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut y = start_y;

    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: WorkflowNodeBase {
            id: "p-analysts".to_string(),
            title: "9 Analyst Agents".to_string(),
            description: Some("9 parallel branches for comprehensive stock analysis".to_string()),
            position: Position { x: 400.0, y },
            retry: RetryConfig::default(),
            timeout: Some(600),
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: ParallelNodeConfig {
            branches: vec![
                Branch {
                    id: "branch_fundamental".to_string(),
                    title: "Fundamental".to_string(),
                    steps: vec![
                        "fundamental-tool".to_string(),
                        "fundamental-analyst".to_string(),
                    ],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_technical".to_string(),
                    title: "Technical".to_string(),
                    steps: vec![
                        "technical-tool".to_string(),
                        "technical-analyst".to_string(),
                    ],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_sentiment".to_string(),
                    title: "Sentiment".to_string(),
                    steps: vec!["news-tool".to_string(), "sentiment-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_peer".to_string(),
                    title: "Peer".to_string(),
                    steps: vec!["peer-tool".to_string(), "peer-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_macro".to_string(),
                    title: "Macro".to_string(),
                    steps: vec!["macro-tool".to_string(), "macro-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_risk".to_string(),
                    title: "Risk".to_string(),
                    steps: vec!["risk-tool".to_string(), "risk-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_insider".to_string(),
                    title: "Insider".to_string(),
                    steps: vec!["insider-tool".to_string(), "insider-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_dividend".to_string(),
                    title: "Dividend".to_string(),
                    steps: vec!["dividend-tool".to_string(), "dividend-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_forecast".to_string(),
                    title: "Forecast".to_string(),
                    steps: vec!["forecast-tool".to_string(), "forecast-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
            ],
            wait_for_all: true,
            timeout: Some(600),
            aggregation: Some(MergeStrategy::All),
            auto_input_from_parent: true,
            sub_graph: None,
        },
    }));

    y += 250.0;

    nodes.push(WorkflowNode::Merge(MergeNode {
        base: WorkflowNodeBase {
            id: "m-analysts".to_string(),
            title: "Merge Analysts".to_string(),
            description: Some("Merges outputs from all 9 analyst branches".to_string()),
            position: Position { x: 250.0, y },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: MergeNodeConfig {
            merge_type: MergeStrategy::All,
            inputs: vec![
                "fundamental-analyst".to_string(),
                "technical-analyst".to_string(),
                "sentiment-analyst".to_string(),
                "peer-analyst".to_string(),
                "macro-analyst".to_string(),
                "risk-analyst".to_string(),
                "insider-analyst".to_string(),
                "dividend-analyst".to_string(),
                "forecast-analyst".to_string(),
            ],
            auto_inputs_from_branches: true,
        },
    }));

    y += 250.0;

    nodes.push(WorkflowNode::Condition(ConditionNode {
        base: WorkflowNodeBase {
            id: "c-need-debate".to_string(),
            title: "Need Debate?".to_string(),
            description: Some("Determines if further debate is needed".to_string()),
            position: Position { x: 250.0, y },
            retry: RetryConfig::default(),
            timeout: Some(60),
            enabled: true,
                parent_id: None,
                compensation: None,
        },
        config: ConditionNodeConfig {
            conditions: vec![],
            logical_op: LogicalOperator::And,
            judge_by_llm: Some(true),
            routing_prompt: Some(
                "Based on the analysis results, determine if there are conflicting opinions that require debate. Consider: 1) Significant disagreement between analysts, 2) High uncertainty in key metrics, 3) Material differences in risk assessment. Return true if debate is needed, false otherwise.".to_string(),
            ),
            routing_model: None,
            confidence_threshold: None,
        },
    }));

    y += 250.0;

    nodes.push(WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            id: "end".to_string(),
            title: "End".to_string(),
            description: Some("Workflow completed".to_string()),
            position: Position { x: 250.0, y },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: EndNodeConfig { output_var: None },
    }));

    nodes
}

pub fn convert_preset_to_workflow_template(preset: &PresetTemplate) -> WorkflowTemplateData {
    let now = Utc::now().timestamp_millis();

    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: "trigger".to_string(),
            title: "Manual Trigger".to_string(),
            description: Some("Starts the workflow manually".to_string()),
            position: Position { x: 250.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        },
    }));

    if preset.id == "stock-analysis" {
        let stock_nodes = build_stock_analysis_nodes(&preset.steps, 100.0);
        nodes.extend(stock_nodes);

        edges.push(WorkflowEdge {
            id: "edge_trigger_to_p_analysts".to_string(),
            source: "trigger".to_string(),
            source_handle: None,
            target: "p-analysts".to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_p_analysts_to_m_analysts".to_string(),
            source: "p-analysts".to_string(),
            source_handle: None,
            target: "m-analysts".to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_m_analysts_to_c_need_debate".to_string(),
            source: "m-analysts".to_string(),
            source_handle: None,
            target: "c-need-debate".to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_c_need_debate_true_to_end".to_string(),
            source: "c-need-debate".to_string(),
            source_handle: Some("true".to_string()),
            target: "end".to_string(),
            target_handle: None,
            edge_type: EdgeType::ConditionTrue,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_c_need_debate_false_to_end".to_string(),
            source: "c-need-debate".to_string(),
            source_handle: Some("false".to_string()),
            target: "end".to_string(),
            target_handle: None,
            edge_type: EdgeType::ConditionFalse,
            label: None,
        });
    } else {
        let step_nodes = build_workflow_nodes(&preset.steps, 100.0, preset.system_prompt);
        nodes.extend(step_nodes);

        let end_y = 100.0 + ((preset.steps.len() + 2) as f64 * 200.0);
        nodes.push(WorkflowNode::End(EndNode {
            base: WorkflowNodeBase {
                id: "end".to_string(),
                title: "End".to_string(),
                description: Some("Workflow completed".to_string()),
                position: Position { x: 250.0, y: end_y },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
            },
            config: EndNodeConfig { output_var: None },
        }));

        edges.extend(create_edges_for_steps(&preset.steps));

        let parallel_groups = detect_parallel_groups(&preset.steps);
        for group in &parallel_groups {
            let parallel_id = format!("parallel_{}", group[0].id);
            let merge_id = format!("merge_{}", group[0].id);

            for (i, step) in group.iter().enumerate() {
                edges.push(WorkflowEdge {
                    id: format!("edge_parallel_to_{}", step.id),
                    source: parallel_id.clone(),
                    source_handle: Some(format!("branch_{}", i)),
                    target: step.id.to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });

                edges.push(WorkflowEdge {
                    id: format!("edge_{}_to_merge", step.id),
                    source: step.id.to_string(),
                    source_handle: None,
                    target: merge_id.clone(),
                    target_handle: Some(format!("input_{}", i)),
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }

            if let Some(first_need) = group[0].needs.first() {
                edges.push(WorkflowEdge {
                    id: format!("edge_{}_to_parallel", first_need),
                    source: first_need.to_string(),
                    source_handle: None,
                    target: parallel_id.clone(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }
        }

        if let Some(first_step) = preset.steps.first() {
            let is_in_parallel = parallel_groups
                .iter()
                .any(|g| g.iter().any(|s| s.id == first_step.id));
            if !is_in_parallel {
                edges.push(WorkflowEdge {
                    id: "edge_trigger_start".to_string(),
                    source: "trigger".to_string(),
                    source_handle: None,
                    target: first_step.id.to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }
        }

        for group in &parallel_groups {
            if !group[0].needs.is_empty() {
                edges.push(WorkflowEdge {
                    id: format!("edge_trigger_to_parallel_{}", group[0].id),
                    source: "trigger".to_string(),
                    source_handle: None,
                    target: format!("parallel_{}", group[0].id),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }
        }

        let non_parallel_last_steps: Vec<_> = preset
            .steps
            .iter()
            .filter(|s| {
                !parallel_groups
                    .iter()
                    .any(|g| g.iter().any(|gs| gs.id == s.id))
            })
            .collect();

        if let Some(last_step) = non_parallel_last_steps.last() {
            edges.push(WorkflowEdge {
                id: "edge_last_end".to_string(),
                source: last_step.id.to_string(),
                source_handle: None,
                target: "end".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
        }

        for group in &parallel_groups {
            edges.push(WorkflowEdge {
                id: format!("edge_merge_{}_to_end", group[0].id),
                source: format!("merge_{}", group[0].id),
                source_handle: None,
                target: "end".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
        }
    }

    WorkflowTemplateData {
        id: preset.id.to_string(),
        name: preset.name.to_string(),
        description: Some(preset.description.to_string()),
        icon: preset.icon.to_string(),
        tags: preset.tags.iter().map(|s| s.to_string()).collect(),
        version: 1,
        is_preset: true,
        is_editable: false,
        is_public: false,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema: get_input_schema_for_template(preset),
        output_schema: get_output_schema_for_template(preset),
        variables: vec![],
        error_config: Some(ErrorConfig {
            retry_policy: Some(RetryPolicy {
                max_retries: 3,
                base_delay_ms: 1000,
                max_delay_ms: 30000,
            }),
            on_failure: OnFailureAction::Abort,
            error_branch: None,
            compensation_steps: None,
        }),
        tool_defs: vec![],
        created_at: now,
        updated_at: now,
    }
}

pub async fn seed_preset_templates(db: &DatabaseConnection) -> Result<()> {
    use axagent_dao::repo::workflow_template as repo;

    let presets = get_preset_templates();

    for preset in presets {
        let existing = repo::get_workflow_template(db, preset.id).await?;

        match existing {
            None => {
                let template = convert_preset_to_workflow_template(&preset);
                let active_model = axagent_entities::workflow_template::ActiveModel {
                    id: sea_orm::Set(template.id.clone()),
                    name: sea_orm::Set(template.name.clone()),
                    description: sea_orm::Set(template.description.clone()),
                    icon: sea_orm::Set(template.icon.clone()),
                    tags: sea_orm::Set(Some(serde_json::to_string(&template.tags)?)),
                    version: sea_orm::Set(template.version),
                    is_preset: sea_orm::Set(template.is_preset),
                    is_editable: sea_orm::Set(template.is_editable),
                    is_public: sea_orm::Set(template.is_public),
                    trigger_config: sea_orm::Set(
                        template
                            .trigger_config
                            .as_ref()
                            .and_then(|c| serde_json::to_string(c).ok()),
                    ),
                    nodes: sea_orm::Set(serde_json::to_string(&template.nodes)?),
                    edges: sea_orm::Set(serde_json::to_string(&template.edges)?),
                    input_schema: sea_orm::Set(
                        template
                            .input_schema
                            .as_ref()
                            .and_then(|s| serde_json::to_string(s).ok()),
                    ),
                    output_schema: sea_orm::Set(
                        template
                            .output_schema
                            .as_ref()
                            .and_then(|s| serde_json::to_string(s).ok()),
                    ),
                    variables: sea_orm::Set(Some(serde_json::to_string(&template.variables)?)),
                    error_config: sea_orm::Set(
                        template
                            .error_config
                            .as_ref()
                            .and_then(|e| serde_json::to_string(e).ok()),
                    ),
                    composite_source: sea_orm::Set(None),
                    tool_defs: sea_orm::Set(None),
                    created_at: sea_orm::Set(template.created_at),
                    updated_at: sea_orm::Set(template.updated_at),
                };
                repo::insert_workflow_template(db, active_model).await?;
            },
            Some(ref t) if t.nodes == "[]" || t.nodes.is_empty() => {
                let template = convert_preset_to_workflow_template(&preset);
                let active_model = axagent_entities::workflow_template::ActiveModel {
                    id: sea_orm::Set(template.id.clone()),
                    name: sea_orm::Set(template.name.clone()),
                    description: sea_orm::Set(template.description.clone()),
                    icon: sea_orm::Set(template.icon.clone()),
                    tags: sea_orm::Set(Some(serde_json::to_string(&template.tags)?)),
                    version: sea_orm::Set(template.version),
                    is_preset: sea_orm::Set(template.is_preset),
                    is_editable: sea_orm::Set(template.is_editable),
                    is_public: sea_orm::Set(template.is_public),
                    trigger_config: sea_orm::Set(
                        template
                            .trigger_config
                            .as_ref()
                            .and_then(|c| serde_json::to_string(c).ok()),
                    ),
                    nodes: sea_orm::Set(serde_json::to_string(&template.nodes)?),
                    edges: sea_orm::Set(serde_json::to_string(&template.edges)?),
                    input_schema: sea_orm::Set(
                        template
                            .input_schema
                            .as_ref()
                            .and_then(|s| serde_json::to_string(s).ok()),
                    ),
                    output_schema: sea_orm::Set(
                        template
                            .output_schema
                            .as_ref()
                            .and_then(|s| serde_json::to_string(s).ok()),
                    ),
                    variables: sea_orm::Set(Some(serde_json::to_string(&template.variables)?)),
                    error_config: sea_orm::Set(
                        template
                            .error_config
                            .as_ref()
                            .and_then(|e| serde_json::to_string(e).ok()),
                    ),
                    composite_source: sea_orm::Set(None),
                    tool_defs: sea_orm::Set(None),
                    created_at: sea_orm::Set(template.created_at),
                    updated_at: sea_orm::Set(template.updated_at),
                };
                repo::upsert_workflow_template(db, active_model).await?;
            },
            _ => {},
        }
    }

    Ok(())
}
