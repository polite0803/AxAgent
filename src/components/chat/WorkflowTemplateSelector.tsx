import { invoke } from "@tauri-apps/api/core";
import { Card, Input, Modal, Tag } from "antd";
import {
  ArrowRight,
  BookOpen,
  Bug,
  Database,
  FileCode,
  GitBranch,
  Globe,
  Layers,
  MessageCircle,
  Network,
  Rocket,
  Search,
  Shield,
  TestTube,
  Users,
  Wrench,
  Zap,
} from "lucide-react";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";

interface WorkflowStepDef {
  id: string;
  goal: string;
  role: string;
  needs: string[];
  /** Agent profile ID for this step */
  agentProfileId?: string;
  /** Override agent_role from profile */
  agentRoleOverride?: string;
}

interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  icon: React.ReactNode;
  tags: string[];
  systemPrompt: string;
  initialMessage: string;
  permissionMode: "default" | "accept_edits" | "full_access";
  /** Workflow steps for multi-agent DAG orchestration.
   *  When present, selecting this template will call workflow_create
   *  to create a backend workflow in addition to setting the system prompt. */
  steps?: WorkflowStepDef[];
  /** Applicable scenarios for this template. Empty array means all scenarios. */
  scenarios?: string[];
}

const getWorkflowTemplates = (t: (key: string) => string): WorkflowTemplate[] => [
  {
    id: "code-review",
    name: t("chat.workflow.codeReview.name"),
    description: t("chat.workflow.codeReview.description"),
    icon: <FileCode size={20} />,
    tags: ["code", "review", "quality"],
    systemPrompt: `You are a senior code reviewer. Analyze the code thoroughly and provide:
1. **Bug detection**: Identify potential bugs, edge cases, and error handling gaps
2. **Security review**: Check for injection, XSS, CSRF, and other OWASP top 10 issues
3. **Performance**: Identify N+1 queries, unnecessary allocations, and optimization opportunities
4. **Best practices**: Check naming, error handling, DRY, SOLID principles
5. **Architecture**: Evaluate coupling, cohesion, and design patterns

Format findings by severity: 🔴 Critical → 🟠 High → 🟡 Medium → 🟢 Low`,
    initialMessage:
      "Please review the code in the current workspace. Start by listing the files, then review the most important ones.",
    permissionMode: "default",
    scenarios: ["coding"],
    steps: [
      { id: "explore", goal: "Explore codebase structure and identify key files", role: "researcher", needs: [] },
      {
        id: "review",
        goal: "Review code for bugs, security issues, and best practices",
        role: "reviewer",
        needs: ["explore"],
      },
      {
        id: "summarize",
        goal: "Synthesize review findings into a structured report",
        role: "synthesizer",
        needs: ["review"],
      },
    ],
  },
  {
    id: "bug-fix",
    name: t("chat.workflow.bugFix.name"),
    description: t("chat.workflow.bugFix.description"),
    icon: <Bug size={20} />,
    tags: ["debug", "fix", "troubleshoot"],
    systemPrompt: `You are a debugging specialist. Follow this systematic approach:
1. **Reproduce**: Understand the exact steps to reproduce the bug
2. **Isolate**: Narrow down the scope — which module, function, or line?
3. **Root cause**: Identify the underlying cause, not just the symptom
4. **Fix**: Implement the minimal fix that addresses the root cause
5. **Verify**: Suggest test cases to verify the fix and prevent regression

Always explain your reasoning at each step. Prefer minimal, targeted fixes over large refactors.`,
    initialMessage:
      "I have a bug to fix. Let me describe the issue and you can help me diagnose and fix it systematically.",
    permissionMode: "accept_edits",
    scenarios: ["coding"],
    steps: [
      { id: "reproduce", goal: "Understand and reproduce the bug", role: "researcher", needs: [] },
      { id: "diagnose", goal: "Identify root cause through analysis", role: "planner", needs: ["reproduce"] },
      { id: "fix", goal: "Implement the minimal fix", role: "developer", needs: ["diagnose"] },
      { id: "verify", goal: "Verify the fix and suggest regression tests", role: "reviewer", needs: ["fix"] },
    ],
  },
  {
    id: "doc-gen",
    name: t("chat.workflow.docGen.name"),
    description: t("chat.workflow.docGen.description"),
    icon: <BookOpen size={20} />,
    tags: ["docs", "api", "readme"],
    systemPrompt: `You are a documentation specialist. Generate clear, comprehensive documentation:
1. **API docs**: Document all public functions/methods with parameters, return types, and examples
2. **README**: Create a project README with setup, usage, and configuration sections
3. **Architecture**: Document the system architecture, data flow, and key decisions
4. **Examples**: Provide working code examples for common use cases

Use markdown formatting. Include code blocks with proper language tags.`,
    initialMessage: "Generate documentation for this project. Start by exploring the project structure and key files.",
    permissionMode: "default",
    scenarios: ["coding", "writing"],
    steps: [
      {
        id: "explore",
        goal: "Explore project structure and identify documentation targets",
        role: "researcher",
        needs: [],
      },
      { id: "generate", goal: "Generate documentation content", role: "developer", needs: ["explore"] },
      { id: "review", goal: "Review documentation quality and completeness", role: "reviewer", needs: ["generate"] },
    ],
  },
  {
    id: "test-gen",
    name: t("chat.workflow.testGen.name"),
    description: t("chat.workflow.testGen.description"),
    icon: <TestTube size={20} />,
    tags: ["testing", "tdd", "coverage"],
    systemPrompt: `You are a test engineering specialist. Generate comprehensive test suites:
1. **Unit tests**: Test individual functions/methods in isolation
2. **Integration tests**: Test component interactions
3. **Edge cases**: Boundary values, empty inputs, null/undefined, large inputs
4. **Error paths**: Verify error handling and error messages
5. **Coverage**: Aim for >80% code coverage

Use the project's existing test framework. Follow existing test patterns and naming conventions.`,
    initialMessage:
      "Generate tests for this project. Start by identifying the test framework and existing test patterns.",
    permissionMode: "accept_edits",
    scenarios: ["coding"],
    steps: [
      { id: "analyze", goal: "Analyze existing code and test patterns", role: "researcher", needs: [] },
      { id: "generate", goal: "Generate comprehensive test suites", role: "developer", needs: ["analyze"] },
      { id: "verify", goal: "Verify tests compile and cover edge cases", role: "reviewer", needs: ["generate"] },
    ],
  },
  {
    id: "refactor",
    name: t("chat.workflow.refactor.name"),
    description: t("chat.workflow.refactor.description"),
    icon: <GitBranch size={20} />,
    tags: ["refactor", "clean-code", "patterns"],
    systemPrompt: `You are a refactoring specialist. Apply behavior-preserving transformations:
1. **Analyze**: Identify code smells — duplication, long methods, deep nesting, god classes
2. **Plan**: Propose specific refactoring steps with before/after examples
3. **Execute**: Apply one refactoring at a time, verifying behavior is preserved
4. **Verify**: Suggest tests to run after each refactoring step

Follow the "Strangler Fig" pattern for large refactors. Never change behavior and structure simultaneously.`,
    initialMessage:
      "Analyze the codebase for refactoring opportunities. Start by identifying code smells and proposing a refactoring plan.",
    permissionMode: "accept_edits",
    scenarios: ["coding"],
    steps: [
      { id: "analyze", goal: "Identify code smells and refactoring opportunities", role: "researcher", needs: [] },
      {
        id: "plan",
        goal: "Create refactoring plan with safe transformation steps",
        role: "planner",
        needs: ["analyze"],
      },
      { id: "execute", goal: "Apply refactoring transformations one at a time", role: "developer", needs: ["plan"] },
      { id: "verify", goal: "Verify behavior is preserved after refactoring", role: "reviewer", needs: ["execute"] },
    ],
  },
  {
    id: "explore",
    name: t("chat.workflow.explore.name"),
    description: t("chat.workflow.explore.description"),
    icon: <Search size={20} />,
    tags: ["explore", "understand", "onboarding"],
    systemPrompt: `You are a code exploration guide. Help the user understand an unfamiliar codebase:
1. **Entry points**: Find main(), index.ts, app entry points
2. **Architecture**: Identify the overall architecture pattern (MVC, Clean Architecture, etc.)
3. **Data flow**: Trace how data flows through the system
4. **Key modules**: Explain the purpose of each major module/directory
5. **Dependencies**: Map internal and external dependencies

Use diagrams (mermaid) when helpful. Explain in terms a new team member would understand.`,
    initialMessage:
      "Help me understand this codebase. Start by exploring the project structure and identifying the architecture.",
    permissionMode: "default",
    scenarios: ["coding", "research"],
    steps: [
      { id: "explore", goal: "Explore project structure and entry points", role: "researcher", needs: [] },
      { id: "analyze", goal: "Analyze architecture and data flow", role: "planner", needs: ["explore"] },
      { id: "document", goal: "Create comprehensive codebase overview", role: "synthesizer", needs: ["analyze"] },
    ],
  },
  // ============== New Templates ==============
  {
    id: "performance",
    name: t("chat.workflow.performance.name"),
    description: t("chat.workflow.performance.description"),
    icon: <Zap size={20} />,
    tags: ["performance", "optimization", "profiling"],
    systemPrompt: `You are a performance optimization specialist. Analyze and improve code performance:
1. **Profile**: Identify bottlenecks using profiling data or code analysis
2. **Analyze**: Determine root causes — algorithmic complexity, I/O, memory, concurrency
3. **Optimize**: Apply targeted optimizations (caching, lazy loading, batching, indexing)
4. **Measure**: Suggest benchmarks to verify improvements
5. **Trade-offs**: Explain performance vs. readability/maintainability trade-offs

Focus on high-impact, measurable improvements. Avoid premature optimization.`,
    initialMessage:
      "Analyze the codebase for performance issues. Start by identifying potential bottlenecks and hot paths.",
    permissionMode: "accept_edits",
    scenarios: ["coding"],
    steps: [
      { id: "profile", goal: "Identify performance bottlenecks and hot paths", role: "researcher", needs: [] },
      { id: "analyze", goal: "Analyze root causes of performance issues", role: "planner", needs: ["profile"] },
      { id: "optimize", goal: "Apply targeted performance optimizations", role: "developer", needs: ["analyze"] },
      { id: "verify", goal: "Verify performance improvements with benchmarks", role: "reviewer", needs: ["optimize"] },
    ],
  },
  {
    id: "security",
    name: t("chat.workflow.security.name"),
    description: t("chat.workflow.security.description"),
    icon: <Shield size={20} />,
    tags: ["security", "audit", "vulnerability"],
    systemPrompt: `You are a security audit specialist. Perform comprehensive security analysis:
1. **Input validation**: Check for injection, XSS, CSRF, path traversal
2. **Authentication**: Review auth mechanisms, session management, password handling
3. **Authorization**: Verify access controls, privilege escalation risks
4. **Data protection**: Check encryption, sensitive data exposure, logging of secrets
5. **Dependencies**: Identify vulnerable dependencies and supply chain risks
6. **OWASP**: Map findings to OWASP Top 10 categories

Provide severity ratings and remediation steps for each finding.`,
    initialMessage:
      "Perform a security audit of this codebase. Start by identifying entry points and user input handling.",
    permissionMode: "default",
    scenarios: ["coding", "analysis"],
    steps: [
      { id: "scan", goal: "Scan for security vulnerabilities and entry points", role: "researcher", needs: [] },
      { id: "analyze", goal: "Analyze security risks and OWASP compliance", role: "reviewer", needs: ["scan"] },
      {
        id: "report",
        goal: "Generate security audit report with remediation",
        role: "synthesizer",
        needs: ["analyze"],
      },
    ],
  },
  {
    id: "migration",
    name: t("chat.workflow.migration.name"),
    description: t("chat.workflow.migration.description"),
    icon: <Globe size={20} />,
    tags: ["migration", "upgrade", "compatibility"],
    systemPrompt: `You are a migration and upgrade specialist. Help migrate code to new versions/frameworks:
1. **Assess**: Identify current version, target version, and breaking changes
2. **Plan**: Create migration plan with incremental steps and rollback strategy
3. **Execute**: Apply changes incrementally, testing at each step
4. **Validate**: Run tests, check deprecation warnings, verify functionality
5. **Cleanup**: Remove deprecated code, update dependencies, clean up workarounds

Prioritize backward compatibility and provide fallback options.`,
    initialMessage:
      "Help me migrate this codebase. What are we migrating from and to? I will analyze the current state and create a migration plan.",
    permissionMode: "accept_edits",
    scenarios: ["coding", "analysis"],
    steps: [
      { id: "assess", goal: "Assess current state and migration requirements", role: "researcher", needs: [] },
      { id: "plan", goal: "Create detailed migration plan with steps", role: "planner", needs: ["assess"] },
      { id: "execute", goal: "Execute migration steps incrementally", role: "developer", needs: ["plan"] },
      { id: "validate", goal: "Validate migration with tests and checks", role: "reviewer", needs: ["execute"] },
    ],
  },
  {
    id: "api-design",
    name: t("chat.workflow.apiDesign.name"),
    description: t("chat.workflow.apiDesign.description"),
    icon: <Database size={20} />,
    tags: ["api", "design", "rest", "graphql"],
    systemPrompt: `You are an API design specialist. Design and implement robust APIs:
1. **Requirements**: Understand use cases, consumers, and data models
2. **Design**: Define endpoints, request/response schemas, error handling
3. **Standards**: Follow REST/GraphQL best practices, versioning, pagination
4. **Security**: Implement authentication, authorization, rate limiting
5. **Documentation**: Generate OpenAPI/GraphQL schema and documentation

Ensure consistency, backward compatibility, and good developer experience.`,
    initialMessage:
      "Help me design an API. Describe the use cases and data requirements, and I will create a comprehensive API design.",
    permissionMode: "accept_edits",
    scenarios: ["coding", "analysis"],
    steps: [
      { id: "analyze", goal: "Analyze requirements and use cases", role: "researcher", needs: [] },
      { id: "design", goal: "Design API endpoints and schemas", role: "planner", needs: ["analyze"] },
      { id: "implement", goal: "Implement API endpoints", role: "developer", needs: ["design"] },
      { id: "document", goal: "Generate API documentation", role: "synthesizer", needs: ["implement"] },
    ],
  },
  {
    id: "debug-env",
    name: t("chat.workflow.debugEnv.name"),
    description: t("chat.workflow.debugEnv.description"),
    icon: <Wrench size={20} />,
    tags: ["debug", "environment", "setup", "config"],
    systemPrompt: `You are an environment and configuration specialist. Diagnose and fix environment issues:
1. **Diagnose**: Identify environment-related errors (missing deps, config, permissions)
2. **Inspect**: Check environment variables, config files, dependencies, permissions
3. **Fix**: Provide step-by-step solutions for environment issues
4. **Document**: Create setup instructions and troubleshooting guide
5. **Automate**: Suggest scripts/tools to prevent future issues

Focus on reproducibility and cross-platform compatibility.`,
    initialMessage:
      "Help me debug an environment or configuration issue. Describe the error and your environment setup.",
    permissionMode: "default",
    scenarios: ["coding"],
    steps: [
      { id: "diagnose", goal: "Diagnose environment issue from error messages", role: "researcher", needs: [] },
      {
        id: "investigate",
        goal: "Investigate root cause in config and environment",
        role: "planner",
        needs: ["diagnose"],
      },
      { id: "fix", goal: "Provide solution and fix steps", role: "developer", needs: ["investigate"] },
    ],
  },
  {
    id: "feature",
    name: t("chat.workflow.feature.name"),
    description: t("chat.workflow.feature.description"),
    icon: <Rocket size={20} />,
    tags: ["feature", "implementation", "development"],
    systemPrompt: `You are a feature implementation specialist. Build new features systematically:
1. **Understand**: Clarify requirements, acceptance criteria, and constraints
2. **Design**: Create technical design with architecture and data flow
3. **Plan**: Break down into tasks with dependencies and estimates
4. **Implement**: Write code following project conventions and patterns
5. **Test**: Write unit tests, integration tests, and manual test steps
6. **Review**: Self-review for code quality, security, and performance

Follow TDD when appropriate. Ensure backward compatibility.`,
    initialMessage:
      "Help me implement a new feature. Describe the feature requirements and I will guide you through the implementation.",
    permissionMode: "accept_edits",
    scenarios: ["coding"],
    steps: [
      { id: "understand", goal: "Understand and clarify feature requirements", role: "researcher", needs: [] },
      { id: "design", goal: "Create technical design and architecture", role: "planner", needs: ["understand"] },
      { id: "implement", goal: "Implement feature with tests", role: "developer", needs: ["design"] },
      { id: "review", goal: "Review implementation quality and completeness", role: "reviewer", needs: ["implement"] },
    ],
  },
  {
    id: "knowledge-extract",
    name: t("chat.workflow.knowledgeExtract.name"),
    description: t("chat.workflow.knowledgeExtract.description"),
    icon: <Network size={20} />,
    tags: ["knowledge", "business", "extract", "architecture"],
    systemPrompt:
      `You are a senior business analyst leading a 4-agent team to extract business knowledge from source code.

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

## Tool Schemas

### create_knowledge_entity
{
  "knowledge_base_id": "string (required)",
  "name": "string (required) - business name",
  "entity_type": "string - entity|value_object|aggregate|domain_service|event",
  "description": "string - plain language description",
  "source_path": "string - original code path",
  "source_language": "string - e.g., Rust, TypeScript",
  "properties": "object - business properties",
  "lifecycle": "object - state transitions",
  "behaviors": "object - business behaviors"
}

### create_knowledge_flow
{
  "knowledge_base_id": "string (required)",
  "name": "string (required)",
  "flow_type": "string - process|use_case|operation",
  "description": "string",
  "source_path": "string",
  "steps": "array - flow steps",
  "decision_points": "array - decision points",
  "error_handling": "object",
  "preconditions": "array",
  "postconditions": "array"
}

### create_knowledge_interface
{
  "knowledge_base_id": "string (required)",
  "name": "string (required)",
  "interface_type": "string - api|event|service",
  "description": "string",
  "source_path": "string",
  "input_schema": "object",
  "output_schema": "object",
  "error_codes": "array",
  "communication_pattern": "string - sync|async"
}

### add_knowledge_document
{
  "knowledge_base_id": "string (required)",
  "title": "string (required)",
  "content": "string (required) - markdown content"
}

## 5 Categories of Business Knowledge to Extract

### 1. Domain Concepts (领域概念)
- Entities with identity, lifecycle, business behavior
- Business dictionaries and terminology
- Business object meanings
- Value objects (immutable characteristics)

### 2. Business Rules (业务规则)
- Judgment logic and conditions
- Thresholds and constraints
- Risk control rules
- Flow conditions
- Complex if/else, switch, exception branches, fallback logic

### 3. Business Flows (业务流程)
- Call chains and invocation relationships
- State transitions and lifecycle
- Upstream and downstream business steps
- Main链路 vs branch链路 coverage

### 4. Data口径 (Data Specifications)
- Field meanings and definitions
- Calculation logic and formulas
- Aggregation formulas
- Data mapping rules

### 5. Business Boundaries (业务边界)
- Functional scope and boundaries
- Applicable scenarios
- Exception business handling

## 6-Dimension Acceptance Criteria

### 1. Completeness (完整性)
- All explicit business logic identified without omissions
- Complex branches, fallback logic fully extracted
- Main链路 + branch链路 100% covered

### 2. Accuracy (准确性)
- Extracted semantics 100% match code execution results
- No misinterpretation, amplification, or simplification
- Formulas, conditions, state transitions 100% restored

### 3. Business Readability (业务可读性)
- Translated from technical code to natural business language
- Understandable by developers, product managers, business users
- Strip variable names, function names, technical details

### 4. Consistency (一致性)
- Same business concept described uniformly throughout
- No duplicate logic, no conflation of different logic
- Unified knowledge granularity

### 5. Reusability (可复用性)
- Directly usable for: requirements, docs, test cases, traceability
- Understandable without code reference
- Supports system review and replay

### 6. Quantified Metrics (量化指标)
- Business logic extraction coverage ≥95%
- Business rule accuracy ≥98%
- Ambiguous knowledge ratio <3%
- Manual correction workload <5%

## Validation Checklist

For each extracted component, verify:
- [ ] All code branches covered (if/else/switch/exception/fallback)
- [ ] Semantic accuracy verified against code
- [ ] Business language natural and unambiguous
- [ ] Terminology consistent with other components
- [ ] Can be understood without technical context
- [ ] Meets quantified metrics thresholds

## Output Format

### Phase 1: Extraction
For each component:
1. Call tool to save to knowledge base
2. Output structured Markdown documentation

### Phase 2: Validation Report
Generate final validation report:
\`\`\`markdown
# Business Knowledge Extraction Validation Report

## Coverage Metrics
- Total components extracted: X
- Business rules identified: X
- Flows documented: X
- Interfaces cataloged: X

## Quality Scores (1-5)
- Completeness: X/5
- Accuracy: X/5
- Readability: X/5
- Consistency: X/5
- Reusability: X/5

## Quantified Metrics
- Extraction coverage: X% (target: ≥95%)
- Rule accuracy: X% (target: ≥98%)
- Ambiguity ratio: X% (target: <3%)
- Manual correction needed: X% (target: <5%)

## Issues Found
- [Issue 1] - impacts X components
- [Issue 2] - requires re-extraction

## Final Verdict
PASS / NEEDS_REEXTRACTION

## Language Erasure Rules
- Remove ALL language-specific keywords (class, fn, struct, impl, pub, etc.)
- Convert types generically: int→number, string→text, list→collection, map→dictionary
- Preserve ONLY business semantics and logic flow
- Remove all implementation details

## Collaboration Protocol
1. Agent1 outputs intermediate parsing results
2. Agent2 uses Agent1 output to generate initial extraction
3. Agent3 validates Agent2 output, flags issues
4. Agent4 consolidates and produces final validation report`,
    initialMessage:
      "Start the 4-agent business knowledge extraction workflow. Agent1: parse the codebase structure and AST. Agent2: extract business knowledge (entities, rules, flows, data口径, boundaries). Agent3: validate extraction accuracy. Agent4: consolidate and produce validation report. Save all knowledge to the knowledge base.",
    permissionMode: "default",
    scenarios: ["analysis", "research"],
    steps: [
      {
        id: "parse",
        goal:
          "Agent1: Parse codebase AST, structure, call relationships, branch logic - output pure logical intermediate text",
        role: "researcher",
        needs: [],
      },
      {
        id: "extract-entities",
        goal: "Agent2: Extract domain concepts, entities, business rules - save using create_knowledge_entity",
        role: "planner",
        needs: ["parse"],
      },
      {
        id: "extract-flows",
        goal: "Agent2: Extract business flows, state transitions, call chains - save using create_knowledge_flow",
        role: "planner",
        needs: ["parse"],
      },
      {
        id: "extract-interfaces",
        goal: "Agent2: Extract interfaces, data口径, API contracts - save using create_knowledge_interface",
        role: "planner",
        needs: ["parse"],
      },
      {
        id: "validate",
        goal: "Agent3: Validate extraction completeness, accuracy, consistency against code - flag issues",
        role: "reviewer",
        needs: ["extract-entities", "extract-flows", "extract-interfaces"],
      },
      {
        id: "consolidate",
        goal:
          "Agent4: Consolidate knowledge, unify terminology, generate validation report with quantified metrics - save report using add_knowledge_document",
        role: "synthesizer",
        needs: ["validate"],
      },
    ],
  },
  {
    id: "knowledge-to-code",
    name: t("chat.workflow.knowledgeToCode.name"),
    description: t("chat.workflow.knowledgeToCode.description"),
    icon: <Layers size={20} />,
    tags: ["knowledge", "code", "generation", "migration", "cross-language"],
    systemPrompt:
      `You are a cross-language code generation specialist. Your task is to generate production-quality code from business knowledge stored in the knowledge base.

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

## Required User Inputs (MUST request before proceeding)
Before starting code generation, you MUST collect the following information from the user:
1. **Target Knowledge Base ID** - from list_knowledge_bases result
2. **Target Programming Language** - e.g., Rust, TypeScript, Python, Go, Java, C#
3. **Target Framework** (optional) - e.g., React, Spring Boot, Django, Express, Axum
4. **Output Directory** - where to generate code files
5. **Project Type** (optional) - API, CLI, WebApp, Library

## Knowledge Base Tools
Use these tools to retrieve business knowledge:
1. **list_knowledge_bases**: View available knowledge bases
2. **query_knowledge**: Semantic search for relevant knowledge
3. **get_knowledge_entity**: Retrieve specific entity definitions
4. **get_knowledge_flow**: Retrieve business flow definitions
5. **get_knowledge_interface**: Retrieve interface/API definitions

## Code Generation Tools
Use these tools to generate and verify code:
1. **write_file**: Write generated code to specified output directory
2. **read_file**: Read generated files for verification
3. **list_directory**: Check output directory structure

## Entity Mapping Rules
| Knowledge Type | Target Code |
|----------------|-------------|
| Domain Entity | Data Model/Class/Struct |
| Business Rule | Validation Logic/Conditions |
| Business Flow | Function/Method Implementation |
| Data Interface | API Endpoint/Controller |
| State Transition | State Machine/Enum Logic |

## Language-Specific Conventions
When generating code, follow these conventions based on target language:
- **Rust**: Use struct, impl, fn, pub, Result<T, E>, ? operator
- **TypeScript**: Use class, interface, async/await, Promises
- **Python**: Use class, def, dataclasses, type hints
- **Go**: Use struct, func, error, goroutines if needed
- **Java**: Use class, public/private, checked exceptions
- **C#**: Use class, public/private, async/await, Task<T>

## Verification Report Format
Agent3 MUST output verification reports in this format:
\`\`\`markdown
# Verification Report: [Component Name]

## Coverage Metrics
- Business Rules Implemented: X/Y (Z%)
- Entity Properties Covered: X/Y (Z%)
- API Endpoints Matched: X/Y (Z%)

## Quality Scores (1-5)
- Completeness: X/5
- Accuracy: X/5
- Naming Convention: X/5

## Issues Found
| Issue | Severity | Description |
|-------|----------|-------------|
| 1 | HIGH/MEDIUM/LOW | Description |

## Final Verdict
PASS / FAIL (requires rework)
\`\`\`

## Verification Failure Handling
If verification fails (coverage < 95% or accuracy < 98%):
1. Flag specific issues with line numbers
2. Request Agent2 to fix specific issues
3. Re-verify after fixes
4. If still failing after 2 iterations, escalate to user

## Quality Metrics
| Metric | Target | Threshold |
|--------|--------|------------|
| Business Rule Coverage | ≥95% | 90% |
| Semantic Equivalence | ≥98% | 95% |
| Code Completeness | 100% | 100% |
| Verification Pass Rate | ≥90% | 85% |

## Collaboration Protocol
1. Request ALL required user inputs before proceeding
2. Agent1 retrieves knowledge and creates architecture spec
3. Present architecture to user for confirmation
4. Agent2 generates models, logic, and API in parallel after user confirmation
5. Agent3 verifies each component against knowledge
6. If all verifications pass, Agent4 integrates
7. If any verification fails, follow Verification Failure Handling

## User Interaction Format
When requesting inputs, use this format:
\`\`\`
## Information Required

Please provide the following information to start:

1. **Target Knowledge Base**: Run 'list_knowledge_bases' to see available options
2. **Programming Language**: [Rust/TypeScript/Python/Go/Java/C#/Other]
3. **Framework** (optional): [React/Spring/Django/None]
4. **Output Directory**: [path for generated code]
5. **Project Type** (optional): [API/CLI/WebApp/Library]
\`\`\``,
    initialMessage:
      "Start the knowledge-to-code migration workflow. First, you MUST request the required information from the user: (1) target knowledge base ID, (2) target programming language, (3) target framework (optional), (4) output directory, (5) project type (optional). Do NOT proceed until all required information is provided. Then retrieve business knowledge, present architecture for confirmation, generate code, verify against knowledge, and integrate.",
    permissionMode: "accept_edits",
    scenarios: ["coding"],
    steps: [
      {
        id: "collect-inputs",
        goal:
          "Request required information from user: target language, framework, knowledge base ID, output directory, project type. Do NOT proceed until all required inputs are provided.",
        role: "researcher",
        needs: [],
      },
      {
        id: "retrieve",
        goal:
          "Retrieve all relevant business knowledge from specified knowledge base: entities, flows, interfaces. Present retrieved content to user for confirmation.",
        role: "researcher",
        needs: ["collect-inputs"],
      },
      {
        id: "design",
        goal:
          "Agent1: Analyze knowledge and design target architecture with component mapping. Present architecture specification to user for confirmation before proceeding.",
        role: "planner",
        needs: ["retrieve"],
      },
      {
        id: "generate-entities",
        goal:
          "Agent2: Generate data models/classes from business entities using target language conventions - use write_file tool to output code",
        role: "developer",
        needs: ["design"],
      },
      {
        id: "generate-logic",
        goal:
          "Agent2: Implement business logic from business flows using target language - use write_file tool to output code",
        role: "developer",
        needs: ["design"],
      },
      {
        id: "generate-api",
        goal:
          "Agent2: Create API layer from interface definitions using target language/framework - use write_file tool to output code",
        role: "developer",
        needs: ["design"],
      },
      {
        id: "verify-entities",
        goal:
          "Agent3: Verify entity code matches knowledge entity definitions - read generated files, output verification report in specified format",
        role: "reviewer",
        needs: ["generate-entities"],
      },
      {
        id: "verify-logic",
        goal:
          "Agent3: Verify logic code implements all business rules from knowledge flows - read generated files, output verification report",
        role: "reviewer",
        needs: ["generate-logic"],
      },
      {
        id: "verify-api",
        goal:
          "Agent3: Verify API code matches interface definitions - read generated files, output verification report",
        role: "reviewer",
        needs: ["generate-api"],
      },
      {
        id: "handle-failures",
        goal:
          "If any verification failed (coverage <95% or accuracy <98%), request Agent2 fixes for specific issues. Re-verify after fixes. Escalate to user if still failing after 2 iterations.",
        role: "reviewer",
        needs: ["verify-entities", "verify-logic", "verify-api"],
      },
      {
        id: "integrate",
        goal:
          "Agent4: Integrate all components, resolve dependencies, ensure consistency - produce final integrated codebase in output directory",
        role: "synthesizer",
        needs: ["handle-failures"],
      },
      {
        id: "report",
        goal:
          "Agent4: Generate final validation report with quality metrics, verification results, and generated file list - save using add_knowledge_document",
        role: "synthesizer",
        needs: ["integrate"],
      },
    ],
  },
  {
    id: "multi-expert-collab",
    name: t("chat.workflow.multiExpertCollab.name"),
    description: t("chat.workflow.multiExpertCollab.description"),
    icon: <Users size={20} />,
    tags: ["collaboration", "expert", "multi-agent"],
    systemPrompt:
      `你是一位多专家协作协调员。你使用 Task 工具启动多个不同领域的专家智能体，让他们并行或串行协作完成任务。

## 协作原则

1. **分而治之**: 将复杂任务拆分为子任务，分配给最合适的专家
2. **并行优先**: 无依赖的子任务使用并行 Task 调用
3. **结果合并**: 收集所有专家的输出，整合为统一报告
4. **专家角色**: 每个 Task 智能体应使用对应的专家角色 system prompt

## 可用专家类型

- **explore**: 代码探索、文件搜索
- **plan**: 方案设计、任务分解
- **build/general**: 代码实现、文档生成
- **review**: 审查、验证、质量检查

## 协作模式

### 并行审查模式
启动 explore(扫描) + review(审查) 同时进行 → 合并结果

### 流水线模式  
explore(分析) → plan(设计) → build(实现) → review(验证)

### 多视角模式
同一问题启动多个 build agent，从不同角度解决，择最优方案

## 输出格式

每个 Task 完成后展示其输出，最后给出合并的总结报告。`,
    initialMessage: "我将启动多专家协作来处理这个任务。请描述你需要完成的任务，我会分配最合适的专家团队。",
    permissionMode: "accept_edits",
    scenarios: ["coding", "analysis", "research"],
    steps: [
      { id: "analyze", goal: "Analyze the task and determine which experts are needed", role: "planner", needs: [] },
      {
        id: "execute",
        goal: "Launch expert sub-agents in parallel to execute their assigned tasks",
        role: "developer",
        needs: ["analyze"],
      },
      {
        id: "synthesize",
        goal: "Collect all expert results and synthesize into a unified deliverable",
        role: "synthesizer",
        needs: ["execute"],
      },
    ],
  },
];

interface WorkflowTemplateSelectorProps {
  open: boolean;
  onClose: () => void;
  onSelect: (template: WorkflowTemplate, workflowId?: string) => void;
  scenario?: string | null;
  /** Expert category for template filtering/prioritization */
  expertCategory?: string | null;
}

const WorkflowTemplateSelector: React.FC<WorkflowTemplateSelectorProps> = ({
  open,
  onClose,
  onSelect,
  scenario,
  expertCategory,
}) => {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [creatingWorkflow, setCreatingWorkflow] = useState<string | null>(null);

  const workflowTemplates = getWorkflowTemplates(t);

  // Map expert category to matching scenario names
  const EXPERT_TO_SCENARIO: Record<string, string> = {
    development: "coding",
    security: "coding",
    data: "analysis",
    devops: "coding",
    design: "coding",
    writing: "writing",
    business: "analysis",
  };

  const filteredTemplates = workflowTemplates.filter((template) => {
    const matchesSearch = template.name.toLowerCase().includes(searchQuery.toLowerCase())
      || template.description.toLowerCase().includes(searchQuery.toLowerCase())
      || template.tags.some((tag) => tag.includes(searchQuery.toLowerCase()));
    const matchesScenario = !scenario
      || !template.scenarios
      || template.scenarios.length === 0
      || template.scenarios.includes(scenario);
    return matchesSearch && matchesScenario;
  });

  // Sort: matching expert category first, then others
  if (expertCategory && !searchQuery.trim()) {
    const targetScenario = EXPERT_TO_SCENARIO[expertCategory];
    if (targetScenario) {
      filteredTemplates.sort((a, b) => {
        const aMatch = a.scenarios?.includes(targetScenario) ? 1 : 0;
        const bMatch = b.scenarios?.includes(targetScenario) ? 1 : 0;
        return bMatch - aMatch;
      });
    }
  }

  const handleSelect = async (template: WorkflowTemplate) => {
    // If the template has workflow steps, create a backend workflow
    if (template.steps && template.steps.length > 0) {
      setCreatingWorkflow(template.id);
      try {
        const result = await invoke<{ workflowId: string; name: string; stepCount: number }>("workflow_create", {
          request: {
            name: template.name,
            steps: template.steps,
          },
        });
        onSelect(template, result.workflowId);
      } catch (e) {
        console.error("[WorkflowTemplateSelector] Failed to create workflow:", e);
        // Fall back to non-workflow mode
        onSelect(template);
      } finally {
        setCreatingWorkflow(null);
      }
    } else {
      onSelect(template);
    }
  };

  return (
    <Modal
      title={t("chat.workflow.title")}
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
    >
      <Input
        placeholder={t("chat.workflow.searchPlaceholder")}
        value={searchQuery}
        onChange={(e) => setSearchQuery(e.target.value)}
        style={{ marginBottom: 16 }}
        allowClear
      />

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {/* 对话模式 — 默认选项，不绑定任何工作流模板 */}
        <Card
          key="conversation-mode"
          size="small"
          hoverable
          onClick={() => {
            onSelect({
              id: "",
              name: t("chat.workflow.conversationMode"),
              description: t("chat.workflow.conversationModeDesc"),
              icon: <MessageCircle size={20} />,
              tags: [],
              systemPrompt: "",
              initialMessage: "",
              permissionMode: "default",
            } as WorkflowTemplate);
          }}
          className="cursor-pointer border-dashed"
          style={{ borderStyle: "dashed" }}
        >
          <div className="flex items-start gap-3">
            <div className="shrink-0 text-gray-400 mt-0.5">
              <MessageCircle size={20} />
            </div>
            <div className="flex-1 min-w-0">
              <div className="font-medium text-sm text-gray-500">
                {t("chat.workflow.conversationMode")}
              </div>
              <div className="text-xs text-gray-400 mt-1">
                {t("chat.workflow.conversationModeDesc")}
              </div>
            </div>
          </div>
        </Card>
        {filteredTemplates.map((template) => (
          <Card
            key={template.id}
            size="small"
            hoverable
            onClick={() => handleSelect(template)}
            className="cursor-pointer"
            loading={creatingWorkflow === template.id}
          >
            <div className="flex items-start gap-3">
              <div className="shrink-0 text-blue-500 mt-0.5">
                {template.icon}
              </div>
              <div className="flex-1 min-w-0">
                <div className="font-medium text-sm">{template.name}</div>
                <div className="text-xs text-gray-500 mt-1 line-clamp-2">
                  {template.description}
                </div>
                <div className="flex flex-wrap gap-1 mt-2">
                  {template.tags.map((tag) => (
                    <Tag key={tag} className="text-xs py-0 leading-tight">
                      {tag}
                    </Tag>
                  ))}
                </div>
              </div>
              <ArrowRight size={14} className="text-gray-400 shrink-0 mt-1" />
            </div>
          </Card>
        ))}
      </div>
    </Modal>
  );
};

export default WorkflowTemplateSelector;
export { getWorkflowTemplates };
export type { WorkflowTemplate };
