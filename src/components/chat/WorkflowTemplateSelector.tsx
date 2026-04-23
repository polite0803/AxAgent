import React, { useState } from 'react';
import { Modal, Card, Tag, Input } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import {
  Bug, FileCode, BookOpen, GitBranch, TestTube,
  Search, ArrowRight, Zap, Shield, Globe, Database, Wrench, Rocket, Network,
} from 'lucide-react';

interface WorkflowStepDef {
  id: string;
  goal: string;
  role: string;
  needs: string[];
}

interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  icon: React.ReactNode;
  tags: string[];
  systemPrompt: string;
  initialMessage: string;
  permissionMode: 'default' | 'accept_edits' | 'full_access';
  /** Workflow steps for multi-agent DAG orchestration.
   *  When present, selecting this template will call workflow_create
   *  to create a backend workflow in addition to setting the system prompt. */
  steps?: WorkflowStepDef[];
}

const getWorkflowTemplates = (t: (key: string) => string): WorkflowTemplate[] => [
  {
    id: 'code-review',
    name: t('chat.workflow.codeReview.name'),
    description: t('chat.workflow.codeReview.description'),
    icon: <FileCode size={20} />,
    tags: ['code', 'review', 'quality'],
    systemPrompt: `You are a senior code reviewer. Analyze the code thoroughly and provide:
1. **Bug detection**: Identify potential bugs, edge cases, and error handling gaps
2. **Security review**: Check for injection, XSS, CSRF, and other OWASP top 10 issues
3. **Performance**: Identify N+1 queries, unnecessary allocations, and optimization opportunities
4. **Best practices**: Check naming, error handling, DRY, SOLID principles
5. **Architecture**: Evaluate coupling, cohesion, and design patterns

Format findings by severity: 🔴 Critical → 🟠 High → 🟡 Medium → 🟢 Low`,
    initialMessage: 'Please review the code in the current workspace. Start by listing the files, then review the most important ones.',
    permissionMode: 'default',
    steps: [
      { id: 'explore', goal: 'Explore codebase structure and identify key files', role: 'researcher', needs: [] },
      { id: 'review', goal: 'Review code for bugs, security issues, and best practices', role: 'reviewer', needs: ['explore'] },
      { id: 'summarize', goal: 'Synthesize review findings into a structured report', role: 'synthesizer', needs: ['review'] },
    ],
  },
  {
    id: 'bug-fix',
    name: t('chat.workflow.bugFix.name'),
    description: t('chat.workflow.bugFix.description'),
    icon: <Bug size={20} />,
    tags: ['debug', 'fix', 'troubleshoot'],
    systemPrompt: `You are a debugging specialist. Follow this systematic approach:
1. **Reproduce**: Understand the exact steps to reproduce the bug
2. **Isolate**: Narrow down the scope — which module, function, or line?
3. **Root cause**: Identify the underlying cause, not just the symptom
4. **Fix**: Implement the minimal fix that addresses the root cause
5. **Verify**: Suggest test cases to verify the fix and prevent regression

Always explain your reasoning at each step. Prefer minimal, targeted fixes over large refactors.`,
    initialMessage: 'I have a bug to fix. Let me describe the issue and you can help me diagnose and fix it systematically.',
    permissionMode: 'accept_edits',
    steps: [
      { id: 'reproduce', goal: 'Understand and reproduce the bug', role: 'researcher', needs: [] },
      { id: 'diagnose', goal: 'Identify root cause through analysis', role: 'planner', needs: ['reproduce'] },
      { id: 'fix', goal: 'Implement the minimal fix', role: 'developer', needs: ['diagnose'] },
      { id: 'verify', goal: 'Verify the fix and suggest regression tests', role: 'reviewer', needs: ['fix'] },
    ],
  },
  {
    id: 'doc-gen',
    name: t('chat.workflow.docGen.name'),
    description: t('chat.workflow.docGen.description'),
    icon: <BookOpen size={20} />,
    tags: ['docs', 'api', 'readme'],
    systemPrompt: `You are a documentation specialist. Generate clear, comprehensive documentation:
1. **API docs**: Document all public functions/methods with parameters, return types, and examples
2. **README**: Create a project README with setup, usage, and configuration sections
3. **Architecture**: Document the system architecture, data flow, and key decisions
4. **Examples**: Provide working code examples for common use cases

Use markdown formatting. Include code blocks with proper language tags.`,
    initialMessage: 'Generate documentation for this project. Start by exploring the project structure and key files.',
    permissionMode: 'default',
    steps: [
      { id: 'explore', goal: 'Explore project structure and identify documentation targets', role: 'researcher', needs: [] },
      { id: 'generate', goal: 'Generate documentation content', role: 'developer', needs: ['explore'] },
      { id: 'review', goal: 'Review documentation quality and completeness', role: 'reviewer', needs: ['generate'] },
    ],
  },
  {
    id: 'test-gen',
    name: t('chat.workflow.testGen.name'),
    description: t('chat.workflow.testGen.description'),
    icon: <TestTube size={20} />,
    tags: ['testing', 'tdd', 'coverage'],
    systemPrompt: `You are a test engineering specialist. Generate comprehensive test suites:
1. **Unit tests**: Test individual functions/methods in isolation
2. **Integration tests**: Test component interactions
3. **Edge cases**: Boundary values, empty inputs, null/undefined, large inputs
4. **Error paths**: Verify error handling and error messages
5. **Coverage**: Aim for >80% code coverage

Use the project's existing test framework. Follow existing test patterns and naming conventions.`,
    initialMessage: 'Generate tests for this project. Start by identifying the test framework and existing test patterns.',
    permissionMode: 'accept_edits',
    steps: [
      { id: 'analyze', goal: 'Analyze existing code and test patterns', role: 'researcher', needs: [] },
      { id: 'generate', goal: 'Generate comprehensive test suites', role: 'developer', needs: ['analyze'] },
      { id: 'verify', goal: 'Verify tests compile and cover edge cases', role: 'reviewer', needs: ['generate'] },
    ],
  },
  {
    id: 'refactor',
    name: t('chat.workflow.refactor.name'),
    description: t('chat.workflow.refactor.description'),
    icon: <GitBranch size={20} />,
    tags: ['refactor', 'clean-code', 'patterns'],
    systemPrompt: `You are a refactoring specialist. Apply behavior-preserving transformations:
1. **Analyze**: Identify code smells — duplication, long methods, deep nesting, god classes
2. **Plan**: Propose specific refactoring steps with before/after examples
3. **Execute**: Apply one refactoring at a time, verifying behavior is preserved
4. **Verify**: Suggest tests to run after each refactoring step

Follow the "Strangler Fig" pattern for large refactors. Never change behavior and structure simultaneously.`,
    initialMessage: 'Analyze the codebase for refactoring opportunities. Start by identifying code smells and proposing a refactoring plan.',
    permissionMode: 'accept_edits',
    steps: [
      { id: 'analyze', goal: 'Identify code smells and refactoring opportunities', role: 'researcher', needs: [] },
      { id: 'plan', goal: 'Create refactoring plan with safe transformation steps', role: 'planner', needs: ['analyze'] },
      { id: 'execute', goal: 'Apply refactoring transformations one at a time', role: 'developer', needs: ['plan'] },
      { id: 'verify', goal: 'Verify behavior is preserved after refactoring', role: 'reviewer', needs: ['execute'] },
    ],
  },
  {
    id: 'explore',
    name: t('chat.workflow.explore.name'),
    description: t('chat.workflow.explore.description'),
    icon: <Search size={20} />,
    tags: ['explore', 'understand', 'onboarding'],
    systemPrompt: `You are a code exploration guide. Help the user understand an unfamiliar codebase:
1. **Entry points**: Find main(), index.ts, app entry points
2. **Architecture**: Identify the overall architecture pattern (MVC, Clean Architecture, etc.)
3. **Data flow**: Trace how data flows through the system
4. **Key modules**: Explain the purpose of each major module/directory
5. **Dependencies**: Map internal and external dependencies

Use diagrams (mermaid) when helpful. Explain in terms a new team member would understand.`,
    initialMessage: 'Help me understand this codebase. Start by exploring the project structure and identifying the architecture.',
    permissionMode: 'default',
    steps: [
      { id: 'explore', goal: 'Explore project structure and entry points', role: 'researcher', needs: [] },
      { id: 'analyze', goal: 'Analyze architecture and data flow', role: 'planner', needs: ['explore'] },
      { id: 'document', goal: 'Create comprehensive codebase overview', role: 'synthesizer', needs: ['analyze'] },
    ],
  },
  // ============== New Templates ==============
  {
    id: 'performance',
    name: t('chat.workflow.performance.name'),
    description: t('chat.workflow.performance.description'),
    icon: <Zap size={20} />,
    tags: ['performance', 'optimization', 'profiling'],
    systemPrompt: `You are a performance optimization specialist. Analyze and improve code performance:
1. **Profile**: Identify bottlenecks using profiling data or code analysis
2. **Analyze**: Determine root causes — algorithmic complexity, I/O, memory, concurrency
3. **Optimize**: Apply targeted optimizations (caching, lazy loading, batching, indexing)
4. **Measure**: Suggest benchmarks to verify improvements
5. **Trade-offs**: Explain performance vs. readability/maintainability trade-offs

Focus on high-impact, measurable improvements. Avoid premature optimization.`,
    initialMessage: 'Analyze the codebase for performance issues. Start by identifying potential bottlenecks and hot paths.',
    permissionMode: 'accept_edits',
    steps: [
      { id: 'profile', goal: 'Identify performance bottlenecks and hot paths', role: 'researcher', needs: [] },
      { id: 'analyze', goal: 'Analyze root causes of performance issues', role: 'planner', needs: ['profile'] },
      { id: 'optimize', goal: 'Apply targeted performance optimizations', role: 'developer', needs: ['analyze'] },
      { id: 'verify', goal: 'Verify performance improvements with benchmarks', role: 'reviewer', needs: ['optimize'] },
    ],
  },
  {
    id: 'security',
    name: t('chat.workflow.security.name'),
    description: t('chat.workflow.security.description'),
    icon: <Shield size={20} />,
    tags: ['security', 'audit', 'vulnerability'],
    systemPrompt: `You are a security audit specialist. Perform comprehensive security analysis:
1. **Input validation**: Check for injection, XSS, CSRF, path traversal
2. **Authentication**: Review auth mechanisms, session management, password handling
3. **Authorization**: Verify access controls, privilege escalation risks
4. **Data protection**: Check encryption, sensitive data exposure, logging of secrets
5. **Dependencies**: Identify vulnerable dependencies and supply chain risks
6. **OWASP**: Map findings to OWASP Top 10 categories

Provide severity ratings and remediation steps for each finding.`,
    initialMessage: 'Perform a security audit of this codebase. Start by identifying entry points and user input handling.',
    permissionMode: 'default',
    steps: [
      { id: 'scan', goal: 'Scan for security vulnerabilities and entry points', role: 'researcher', needs: [] },
      { id: 'analyze', goal: 'Analyze security risks and OWASP compliance', role: 'reviewer', needs: ['scan'] },
      { id: 'report', goal: 'Generate security audit report with remediation', role: 'synthesizer', needs: ['analyze'] },
    ],
  },
  {
    id: 'migration',
    name: t('chat.workflow.migration.name'),
    description: t('chat.workflow.migration.description'),
    icon: <Globe size={20} />,
    tags: ['migration', 'upgrade', 'compatibility'],
    systemPrompt: `You are a migration and upgrade specialist. Help migrate code to new versions/frameworks:
1. **Assess**: Identify current version, target version, and breaking changes
2. **Plan**: Create migration plan with incremental steps and rollback strategy
3. **Execute**: Apply changes incrementally, testing at each step
4. **Validate**: Run tests, check deprecation warnings, verify functionality
5. **Cleanup**: Remove deprecated code, update dependencies, clean up workarounds

Prioritize backward compatibility and provide fallback options.`,
    initialMessage: 'Help me migrate this codebase. What are we migrating from and to? I will analyze the current state and create a migration plan.',
    permissionMode: 'accept_edits',
    steps: [
      { id: 'assess', goal: 'Assess current state and migration requirements', role: 'researcher', needs: [] },
      { id: 'plan', goal: 'Create detailed migration plan with steps', role: 'planner', needs: ['assess'] },
      { id: 'execute', goal: 'Execute migration steps incrementally', role: 'developer', needs: ['plan'] },
      { id: 'validate', goal: 'Validate migration with tests and checks', role: 'reviewer', needs: ['execute'] },
    ],
  },
  {
    id: 'api-design',
    name: t('chat.workflow.apiDesign.name'),
    description: t('chat.workflow.apiDesign.description'),
    icon: <Database size={20} />,
    tags: ['api', 'design', 'rest', 'graphql'],
    systemPrompt: `You are an API design specialist. Design and implement robust APIs:
1. **Requirements**: Understand use cases, consumers, and data models
2. **Design**: Define endpoints, request/response schemas, error handling
3. **Standards**: Follow REST/GraphQL best practices, versioning, pagination
4. **Security**: Implement authentication, authorization, rate limiting
5. **Documentation**: Generate OpenAPI/GraphQL schema and documentation

Ensure consistency, backward compatibility, and good developer experience.`,
    initialMessage: 'Help me design an API. Describe the use cases and data requirements, and I will create a comprehensive API design.',
    permissionMode: 'accept_edits',
    steps: [
      { id: 'analyze', goal: 'Analyze requirements and use cases', role: 'researcher', needs: [] },
      { id: 'design', goal: 'Design API endpoints and schemas', role: 'planner', needs: ['analyze'] },
      { id: 'implement', goal: 'Implement API endpoints', role: 'developer', needs: ['design'] },
      { id: 'document', goal: 'Generate API documentation', role: 'synthesizer', needs: ['implement'] },
    ],
  },
  {
    id: 'debug-env',
    name: t('chat.workflow.debugEnv.name'),
    description: t('chat.workflow.debugEnv.description'),
    icon: <Wrench size={20} />,
    tags: ['debug', 'environment', 'setup', 'config'],
    systemPrompt: `You are an environment and configuration specialist. Diagnose and fix environment issues:
1. **Diagnose**: Identify environment-related errors (missing deps, config, permissions)
2. **Inspect**: Check environment variables, config files, dependencies, permissions
3. **Fix**: Provide step-by-step solutions for environment issues
4. **Document**: Create setup instructions and troubleshooting guide
5. **Automate**: Suggest scripts/tools to prevent future issues

Focus on reproducibility and cross-platform compatibility.`,
    initialMessage: 'Help me debug an environment or configuration issue. Describe the error and your environment setup.',
    permissionMode: 'default',
    steps: [
      { id: 'diagnose', goal: 'Diagnose environment issue from error messages', role: 'researcher', needs: [] },
      { id: 'investigate', goal: 'Investigate root cause in config and environment', role: 'planner', needs: ['diagnose'] },
      { id: 'fix', goal: 'Provide solution and fix steps', role: 'developer', needs: ['investigate'] },
    ],
  },
  {
    id: 'feature',
    name: t('chat.workflow.feature.name'),
    description: t('chat.workflow.feature.description'),
    icon: <Rocket size={20} />,
    tags: ['feature', 'implementation', 'development'],
    systemPrompt: `You are a feature implementation specialist. Build new features systematically:
1. **Understand**: Clarify requirements, acceptance criteria, and constraints
2. **Design**: Create technical design with architecture and data flow
3. **Plan**: Break down into tasks with dependencies and estimates
4. **Implement**: Write code following project conventions and patterns
5. **Test**: Write unit tests, integration tests, and manual test steps
6. **Review**: Self-review for code quality, security, and performance

Follow TDD when appropriate. Ensure backward compatibility.`,
    initialMessage: 'Help me implement a new feature. Describe the feature requirements and I will guide you through the implementation.',
    permissionMode: 'accept_edits',
    steps: [
      { id: 'understand', goal: 'Understand and clarify feature requirements', role: 'researcher', needs: [] },
      { id: 'design', goal: 'Create technical design and architecture', role: 'planner', needs: ['understand'] },
      { id: 'implement', goal: 'Implement feature with tests', role: 'developer', needs: ['design'] },
      { id: 'review', goal: 'Review implementation quality and completeness', role: 'reviewer', needs: ['implement'] },
    ],
  },
  {
    id: 'knowledge-extract',
    name: t('chat.workflow.knowledgeExtract.name'),
    description: t('chat.workflow.knowledgeExtract.description'),
    icon: <Network size={20} />,
    tags: ['knowledge', 'business', 'extract', 'architecture'],
    systemPrompt: `You are a business knowledge extraction specialist. Your task is to extract language-agnostic business knowledge from source code.

## Your Mission
Transform code into pure business knowledge that is completely independent of any programming language (C++, Rust, Java, Python, TypeScript, etc.).

## What to Extract

### 1. Domain Model
- **Entities**: Objects with identity, lifecycle, and business behavior
- **Value Objects**: Immutable objects describing characteristics (no identity)
- **Aggregate Roots**: Boundaries that protect internal consistency
- **Domain Events**: Business state changes that trigger side effects

### 2. Business Logic
- **Business Rules**: Validation, calculation, and policy rules
- **Constraints**: Invariants and boundary conditions
- **State Machines**: States, transitions, events, and guards
- **Domain Services**: Cross-entity business operations

### 3. Flow Definition
- **Use Cases**: Business scenarios with actors, preconditions, postconditions
- **Operations**: Step sequences, decision points, branches
- **Error Handling**: Error flows and rollback logic

### 4. Interface Contracts
- **API Contracts**: Input/output formats, error codes
- **Communication Patterns**: Sync/async, message formats, protocols

### 5. Data Structures
- **DTOs**: Data transfer objects with field definitions and types
- **Mapping Rules**: Relationships between data and domain objects

### 6. Dependencies
- **Module Dependencies**: Boundaries and inter-module relationships
- **External Services**: Service interfaces and integration patterns

## Output Format
For each component found, provide:

**Markdown Documentation**:
\`\`\`markdown
## [Component Name]

### Type: [Entity/Value Object/Aggregate Root/Domain Service/etc.]

### Description
[Plain language description of what this component does in business terms]

### Identity
[How this component is identified]

### Lifecycle/States
[State transitions and what triggers them]

### Business Rules
- [Rule 1]
- [Rule 2]

### Relationships
- [Related component 1] - [relationship type]
- [Related component 2] - [relationship type]

### Data Structure
\`\`\`json
{
  "field_name": {
    "type": "business type",
    "description": "what this field represents",
    "constraints": ["business constraints"]
  }
}
\`\`\`
\`\`\`

**JSON Metadata** (for each component):
\`\`\`json
{
  "id": "unique_identifier",
  "type": "entity|value_object|aggregate|service|flow|interface",
  "name": "BusinessName",
  "source": {
    "path": "original/file/path",
    "language": "OriginalLanguage"
  },
  "properties": {...},
  "relationships": [...],
  "businessRules": [...]
}
\`\`\`

## Language Erasure Rules
- Remove ALL language-specific keywords (class, fn, struct, impl, pub, etc.)
- Convert types to generic business types:
  - int/float/double → number
  - string/str → text
  - list/array/Vec → collection
  - map/HashMap/Dict → dictionary
  - enum → enumerated list
- Preserve ONLY business semantics and logic flow
- Remove implementation details

## Granularity Guidelines
- Small projects (< 10 files): File-level extraction
- Medium projects (10-100 files): Class/Component-level extraction
- Large projects (100+ files): Module/Package-level aggregation

Focus on making the output understandable by non-programmers while maintaining enough detail for cross-language reimplementation.`,
    initialMessage: 'Extract business knowledge from this codebase. Start by exploring the project structure to understand the architecture, then systematically extract domain models, business rules, flows, and interfaces.',
    permissionMode: 'default',
    steps: [
      { id: 'explore', goal: 'Explore codebase structure and identify architecture', role: 'researcher', needs: [] },
      { id: 'parse', goal: 'Parse code structure, AST, and dependencies', role: 'researcher', needs: ['explore'] },
      { id: 'abstract', goal: 'Abstract business domain models and rules', role: 'planner', needs: ['parse'] },
      { id: 'extract-flows', goal: 'Extract business flows and interfaces', role: 'planner', needs: ['parse'] },
      { id: 'assemble', goal: 'Assemble knowledge into structured output', role: 'synthesizer', needs: ['abstract', 'extract-flows'] },
    ],
  },
];

interface WorkflowTemplateSelectorProps {
  open: boolean;
  onClose: () => void;
  onSelect: (template: WorkflowTemplate, workflowId?: string) => void;
}

const WorkflowTemplateSelector: React.FC<WorkflowTemplateSelectorProps> = ({
  open,
  onClose,
  onSelect,
}) => {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [creatingWorkflow, setCreatingWorkflow] = useState<string | null>(null);

  const workflowTemplates = getWorkflowTemplates(t);

  const filteredTemplates = workflowTemplates.filter(
    (template) =>
      template.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      template.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      template.tags.some((tag) => tag.includes(searchQuery.toLowerCase())),
  );

  const handleSelect = async (template: WorkflowTemplate) => {
    // If the template has workflow steps, create a backend workflow
    if (template.steps && template.steps.length > 0) {
      setCreatingWorkflow(template.id);
      try {
        const result = await invoke<{ workflowId: string; name: string; stepCount: number }>('workflow_create', {
          request: {
            name: template.name,
            steps: template.steps,
          },
        });
        onSelect(template, result.workflowId);
      } catch (e) {
        console.error('[WorkflowTemplateSelector] Failed to create workflow:', e);
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
      title={t('chat.workflow.title')}
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
    >
      <Input
        placeholder={t('chat.workflow.searchPlaceholder')}
        value={searchQuery}
        onChange={(e) => setSearchQuery(e.target.value)}
        style={{ marginBottom: 16 }}
        allowClear
      />

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
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
