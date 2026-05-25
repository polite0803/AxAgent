[简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | **English** | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
  <a href="https://www.producthunt.com/products/axagent?embed=true&amp&utm_source=badge-featured&amp&utm_medium=badge&amp&amp;#10;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

<p align="center">
  <strong>Cross-Platform AI Desktop Client | Multi-Agent Collaboration | Local-First</strong>
</p>

<p align="center">
  <a href="https://github.com/polite0803/AxAgent/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/polite0803/AxAgent/actions" target="_blank">
    <img src="https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square" alt="Build">
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## What is AxAgent?

AxAgent v2.0 is a comprehensive cross-platform AI desktop/mobile application that combines advanced AI agent capabilities with a rich set of developer tools. It features multi-provider model support, autonomous agent execution, visual workflow orchestration, local knowledge management, and a built-in API gateway, covering Windows / macOS / Linux / Android / iOS platforms.

---

## Screenshots

| Chat & Model Selection | Multi-Agent Dashboard |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s5-0412.png) |

| Knowledge Base RAG | Memory & Context |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Workflow Editor | API Gateway |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

---

## Features

### 🤖 AI Model Support

- **Multi-Provider Support** — Native integration with OpenAI, Anthropic Claude, Google Gemini, Ollama, OpenClaw, Hermes and all OpenAI-compatible APIs
- **Multi-Key Rotation** — Configure multiple API keys per provider with automatic rotation to distribute rate limits
- **Local Model Support** — Full support for Ollama local models with GGUF/GGML file management
- **Candle Inference Engine** — Built-in Candle local inference with rerank/judge interfaces and on-demand GGUF downloads
- **Model Management** — Remote model list fetching, customizable parameters (temperature, max tokens, top-p, etc.)
- **Streaming Output** — Real-time token-by-token rendering with collapsible thinking blocks (Claude extended thinking)
- **Multi-Model Comparison** — Ask the same question to multiple models simultaneously with side-by-side comparison
- **Function Calling** — Structured function calling across all supported providers
- **OpenAI Responses API** — Support for OpenAI Responses format streaming
- **Realtime API** — WebSocket event push compatible with OpenAI Realtime API

### 🔐 AI Agent System

The agent system is built on a sophisticated architecture featuring:

- **ReAct Reasoning Engine** — Integrates reasoning and action with self-verification for reliable task execution
- **Hierarchical Planner** — Decomposes complex tasks into structured plans with phases and dependencies
- **Task Decomposer** — Automatic breakdown of complex tasks into executable sub-tasks
- **Deep Research** — Multi-source search orchestration, citation tracking, and credibility assessment
- **Fact Checking** — AI-driven fact verification with source classification
- **Search Orchestration** — Multi-search provider coordination with search planning and result synthesis
- **Academic Search** — Academic literature retrieval and citation analysis
- **Computer Control** — AI-controlled mouse clicks, keyboard input, screen scrolling with vision model analysis
- **Screen Perception** — Screenshot capture and visual model analysis for UI element identification
- **Three Permission Levels** — Default (approval required), Accept Edits (auto-approve), Full Access (no prompts)
- **Sandbox Isolation** — Agent operations strictly confined to specified working directory
- **Tool Approval Panel** — Real-time display of tool call requests with per-item review
- **Cost Tracking** — Real-time token usage and cost statistics per session
- **Pause/Resume** — Pause agent execution anytime and resume later
- **Checkpoint System** — Persistent checkpoints for crash recovery and session resumption
- **Error Recovery Engine** — Automatic error classification and recovery strategy execution
- **Loop Detection** — Automatic detection and interruption of cyclic behavior in agent reasoning
- **Thought Chain** — Reasoning visualization for agent decision-making with step-by-step breakdown
- **Proactive Mode** — Agent can proactively offer suggestions and execute actions
- **Purpose Management** — Maintain and track agent execution purpose and context

### 👥 Multi-Agent Collaboration

- **Sub-Agent Coordination** — Master-slave architecture supporting multiple collaborative agents
- **Parallel Execution** — Multiple agents processing tasks in parallel with dependency-aware scheduling
- **Adversarial Debate** — Pro/Con debate rounds with argument strength scoring and refutation tracking
- **Agent Roles** — Predefined roles (researcher, planner, developer, reviewer, synthesizer) for team collaboration
- **Agent Orchestrator** — Centralized message routing and state management for multi-agent teams
- **Communication Graph** — Visual representation of agent interactions and message flow
- **Swarm Cluster** — Multi-process agent cluster with permission sync and auto-reconnect
- **Buddy System** — Configurable agent buddies with species and attribute definitions
- **Shared Memory** — Cross-agent shared memory space with statistics and queries
- **Team Cron Registry** — Team-level scheduled task coordination

### ⭐ Skills System

- **Skills Marketplace** — Built-in marketplace for browsing and installing community-contributed skills
- **Skill Creation** — Auto-create skills from proposals with Markdown editor
- **Skill Evolution** — AI-powered automatic analysis and improvement of existing skills based on execution feedback
- **Skill Matching** — Semantic matching to recommend relevant skills for conversation contexts
- **Skill Decomposition** — Automatic breakdown of complex tasks into executable atomic skills (LLM-assisted/multi-turn/workflow validation)
- **Generated Tools** — AI auto-generates and registers new tools to expand agent capabilities
- **Skills Hub** — Centralized management interface for skill discovery and configuration
- **Skills Hub Client** — Integration with remote skills hub for community sharing
- **Skill Dependency Check** — Automatic detection of skill dependencies and tool availability
- **Skill Sandbox Container** — Skills execute safely in an isolated environment

### 🔄 Workflow System

The workflow engine implements a DAG-based task orchestration system:

- **Visual Workflow Editor** — Drag-and-drop workflow designer with node connection and configuration
- **Rich Node Types** — 15 node types: Trigger, Agent, LLM, Condition, Parallel, Loop, Merge, Delay, Tool, Code, SubWorkflow, VectorRetrieve, DocumentParser, Validation, End
- **Workflow Templates** — Built-in presets: Code Review, Bug Fix, Documentation, Testing, Refactoring, Exploration, Performance, Security, Feature Development
- **DAG Execution** — Kahn's algorithm for topological sorting with cycle detection
- **Parallel Dispatch** — Pipeline-style execution where fast steps don't wait for slow ones
- **Retry Policy** — Exponential backoff with configurable max retries per step
- **Partial Completion** — Failed steps don't block independent downstream steps
- **Version Management** — Workflow template versioning with rollback support
- **Execution History** — Detailed recording with status tracking and debugging
- **AI Assistance** — AI-assisted workflow design, node recommendation, and agent prompt optimization
- **Semantic Check** — Workflow semantic validation to detect potential issues
- **n8n Import** — Support importing workflows from n8n directory
- **Debug Panel** — Real-time debugging and status viewing during workflow execution

### 📚 Knowledge & Memory

- **Knowledge Base (RAG)** — Multi-knowledgebase support with document upload, automatic parsing, chunking, and vector indexing
- **Hybrid Search** — Combines vector similarity search with BM25 full-text ranking
- **Reranking** — Cross-encoder reranking for improved retrieval precision
- **Three-Level Recall Pipeline** — Multi-level recall mechanism with AST index + vector search + FTS5
- **Knowledge Graph** — Entity relationship visualization of knowledge connections (entities, attributes, relations, flows, interfaces)
- **Wiki System** — LLM Wiki compiler and validator with knowledge graph visualization and incremental sync
- **Wiki Notes** — Bidirectional linked notes system with graph view and auto-link sync
- **Memory System** — Multi-namespace memory with manual entry or AI-powered automatic extraction
- **Closed-Loop Memory** — Integration with Honcho and Mem0 for persistent memory providers
- **FTS5 Full-Text Search** — Fast retrieval across conversations, files, and memories
- **Session Search** — Advanced search across all conversation sessions
- **Context Management** — Flexible attachment of files, search results, knowledge snippets, memories, tool outputs
- **Document Parser** — Multi-format document automatic parsing and content extraction
- **Incremental Indexer** — Incremental index updates for file changes

### 🌐 API Gateway

- **Local API Server** — Built-in OpenAI-compatible, Claude, and Gemini interface server
- **External Links** — One-click integration with Claude CLI, OpenCode with automatic API key and model sync
- **Key Management** — Generate, revoke, enable/disable access keys with descriptions
- **Usage Analytics** — Request volume and token usage by key, provider, and date
- **SSL/TLS Support** — Built-in self-signed certificates with custom certificate support
- **Request Logging** — Complete recording of all API requests and responses
- **Configuration Templates** — Pre-built templates for Claude, Codex, OpenCode, Gemini
- **Realtime API** — WebSocket event push compatible with OpenAI Realtime API
- **Platform Integrations** — Support for DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord
- **Gateway Diagnostics** — Connection diagnostics and program policy management
- **Rate Limiter** — API request rate limiting and traffic control
- **Persistent Queue** — Request persistent queue management

### 🔧 Tools & Extensions

- **MCP Protocol** — Full Model Context Protocol implementation with stdio and HTTP/WebSocket transports
- **OAuth Authentication** — OAuth flow support for MCP servers
- **MCP Autostart** — MCP server auto-start and lifecycle management
- **MCP Tool Bridge** — Bridge between MCP tools and agent tool system
- **Plugin System** — OpenClaw-compatible three-tier plugin architecture (builtin/bundled/external) with npm package installation, tool registration, hooks, and lifecycle management
- **Plugin Marketplace** — Built-in marketplace UI with npm search, install, and confirmation dialogs
- **Built-in Tools** — Comprehensive tool set for file operations (read/write/edit), code execution, search (Grep/Glob), Bash, Web search, Web fetch, plan management, Cron scheduling, REPL, LSP, context management, computer control, messaging, todo items, and more
- **Tool Permission System** — Tool permission classification, rule management, and usage tracking
- **Bash Security** — Command parsing, path validation, and sandbox security controls
- **LSP Client** — Built-in Language Server Protocol for code completion and diagnostics
- **AST Index** — Code file AST parsing and index building
- **Terminal Backends** — Support for Local, Docker, and SSH terminal connections
- **Browser Automation** — Integrated browser control via CDP (navigation, screenshots, clicks, form filling, text extraction, etc.)
- **UI Automation** — Cross-platform UI element identification and control
- **Git Tools** — Git operations with branch detection and conflict awareness
- **Tool Recommendation** — Context-aware intelligent tool recommendation engine
- **Tool Orchestration** — Multi-tool coordinated execution with streaming output
- **Tool Stats** — Tool usage frequency and performance statistics

### 📊 Content Rendering

- **Markdown Rendering** — Full support for code highlighting, LaTeX math, tables, task lists
- **Monaco Code Editor** — Embedded editor with syntax highlighting, copy, diff preview
- **Diagram Rendering** — Mermaid flowcharts, D2 architecture diagrams, ECharts interactive charts
- **Artifact Panel** — Code snippets, HTML drafts, React components, Markdown notes with live preview
- **Four Preview Modes** — Code (editor), Split (side-by-side), Preview (rendered only), React component preview
- **Session Inspector** — Tree view of session structure for quick navigation
- **Citation Panel** — Track and display source citations with credibility scoring
- **Infographic Rendering** — Support for infographic visualization display

### 🛡️ Data & Security

- **AES-256 Encryption** — API keys and sensitive data encrypted with AES-256-GCM
- **Isolated Storage** — Application state in `~/.axagent/`, user files in `~/Documents/axagent/`
- **Auto Backup** — Scheduled backups to local directories or WebDAV storage
- **Backup Restore** — One-click restore from historical backups
- **Export Options** — PNG screenshots, Markdown, plain text, JSON formats
- **Storage Management** — Visual disk usage display with cleanup tools
- **File Authorization** — File access authorization and revocation management
- **Operation Audit** — Audit logging for critical operations

### 🖥️ Desktop Experience

- **Theme Engine** — Dark/light themes with system-follow or manual preference
- **Interface Languages** — 11 languages: Simplified Chinese, Traditional Chinese, English, Japanese, Korean, French, German, Spanish, Russian, Hindi, Arabic
- **System Tray** — Minimize to tray without interrupting background services
- **Always on Top** — Pin window above others
- **Global Shortcuts** — Customizable shortcuts to summon main window
- **QuickBar** — Quick access floating bar, one-click summon
- **Auto Start** — Optional launch on system startup
- **Proxy Support** — HTTP and SOCKS5 proxy configuration
- **Auto Update** — Automatic version checking with update prompts
- **Command Palette** — `Cmd/Ctrl+K` for quick command access
- **Onboarding Wizard** — Interactive first-use guide with Ollama detection
- **Notification Center** — Unified in-app notification management

### 🔬 Advanced Features

- **Deep Research** — Multi-source search, citation tracking, credibility assessment, and content synthesis
- **Fact Checking** — AI-driven fact verification with source classification
- **Cron Scheduler** — Automated task scheduling with daily, weekly, monthly templates and custom cron expressions
- **Webhook System** — Event subscriptions for tool completion, agent errors, session end notifications
- **User Profiling** — Automatic learning of coding style, naming conventions, indentation, comment style, communication preferences
- **RL Optimizer** — Reinforcement learning for tool selection and task strategy optimization
- **LoRA Fine-Tuning** — Custom model adaptation with local training using LoRA
- **Proactive Suggestions** — Context-aware nudges based on conversation content and user patterns
- **Context Prediction** — Predict user's next action and prefetch relevant resources
- **Dream Consolidation** — Background auto-consolidation of memories and patterns for long-term knowledge optimization
- **Error Recovery** — Automatic error classification, root cause analysis, and recovery suggestions
- **DevTools** — Trace, span, timeline visualization for debugging and performance analysis
- **Benchmark System** — SWE-bench / Terminal-bench task performance evaluation and metrics with score cards
- **Style Transfer** — Apply learned coding style preferences to generated code
- **Dashboard Plugins** — Extensible dashboard with custom panels and widgets
- **Collaboration** — CRDT-based real-time collaboration and one-click session sharing
- **Browser Extension** — Wiki Clipper browser extension for quick web clipping to LLM Wiki
- **Python SDK** — Python SDK for integration with AxAgent
- **Smart Router** — Intelligent request routing and classification
- **Semantic Cache** — Semantic-based response caching to reduce redundant computation
- **Context Compression** — Automatic compression of long contexts to optimize token usage
- **Message Batching** — Message batch sending and optimization
- **Connection Pool** — Database and API connection pool management
- **Feature Flags** — Configurable feature flag system
- **Policy Engine** — Centralized management of permission and operation policies
- **Resource Governor** — Agent resource usage limits and governance
- **LAN Transfer** — Local area network file transfer capability

### 🛡️ Prompt Injection Protection (Prompt-Guard)

- **Four-Level Protection** — L1 pattern detection (high-risk block + medium-risk flag) → L2 delimiter escaping → L3 XML wrapper → L4 trust tags
- **Pipeline Orchestrator** — Multi-level detection pipeline with customizable risk thresholds
- **Token Smuggling Detection** — Specialized detection for encoding obfuscation and token smuggling attacks
- **Strict Mode** — Strict mode testing + medium-risk reason naming + custom mode documentation
- **Full Pipeline Integration** — Integrated into session / prompt / git / RAG workflows

### 📱 Mobile Support

- **Android Native** — APK/AAB builds, supporting arm64-v8a / armeabi-v7a / x86_64
- **iOS Native** — IPA builds, supporting arm64
- **Adaptive Layout** — Desktop/tablet/phone three-tier auto-adaptation
- **Mobile Navigation** — Drawer slide-out navigation + bottom nav bar + flash floating action button
- **Safe Area Adaptation** — Android system status bar/navigation bar CSS env() adaptation
- **CSP Optimization** — Android WebView CSP protocol whitelist

---

## Technical Architecture

### Tech Stack

| Layer | Technology |
|-------|------------|
| **Framework** | Tauri 2 + React 19 + TypeScript 6 |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **State** | Zustand 5 |
| **Routing** | React Router 7 |
| **i18n** | i18next + react-i18next |
| **Backend** | Rust + SeaORM 2 + SQLite |
| **Vector DB** | sqlite-vec |
| **Code Editor** | Monaco Editor |
| **Diagrams** | Mermaid + D2 + ECharts (CDN) |
| **Terminal** | xterm.js 6 |
| **Workflow** | ReactFlow 11 |
| **Build** | Vite 8 + npm |
| **Infographic** | @antv/infographic |
| **Icons** | Iconify + Lucide |
| **Drag & Drop** | @dnd-kit |
| **Testing** | Vitest + Playwright + cargo-nextest |
| **Formatting** | dprint (TS/JSON) + rustfmt |
| **Lint** | TS: eslint + oxlint / Rust: clippy + cargo-deny |
| **Mobile** | Tauri Android + iOS native builds |
| **Desktop** | Windows (MSI) · macOS (DMG) · Linux (AppImage/deb/rpm) |

### Platform Support

| Platform | Architectures |
|----------|---------------|
| Windows | x86_64, ARM64 |
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Linux | x86_64, ARM64 |
| Android | arm64-v8a, armeabi-v7a, x86_64 (emulator) |
| iOS | arm64 |

### Rust Backend Architecture

The backend is organized as a Rust workspace with 18 specialized crates:

```
src-tauri/crates/
├── agent/            # AI Agent core (ReAct engine, coordination, planning, deep research, fact-checking, etc.)
├── core/             # Core utilities (database, RAG, crypto, MCP, browser automation, AST index, etc.)
├── providers/        # Model provider adapters (OpenAI, Anthropic, Gemini, Ollama, OpenClaw, etc.)
├── runtime-core/     # Runtime abstraction layer (common types, trait definitions, config)
├── runtime/          # Runtime services (session management, MCP, terminal, rate limiting, webhooks, permissions, etc.)
├── rt-workflow/      # Workflow engine (DAG orchestration, node executors, scheduler)
├── rt-messaging/     # Message gateway (DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord integrations)
├── rt-webhook/       # Webhook server & dispatching
├── rt-dashboard/     # Dashboard plugin system
├── rt-theme/         # Theme engine
├── gateway/          # API Gateway (HTTP server, auth, routes, OpenAI-compatible interface)
├── tools/            # Tool system (registry, orchestration, streaming, 40+ built-in tools)
├── trajectory/       # Learning system (memory, skills, RL, user profiling, dream consolidation)
├── telemetry/        # Telemetry & distributed tracing
├── plugins/          # Plugin system (OpenClaw-compatible, npm package installation)
├── prompt-guard/     # Prompt injection protection (L1-L4 multi-level detection & defense)
├── migration/        # Database migrations
├── npm/              # npm package parsing & registry
└── code_engine/      # Candle local inference engine (deprecated, merged into core)
```

### Frontend Architecture

```
src/
├── stores/                    # Zustand state management
│   ├── domain/               # Core business state
│   │   ├── conversationStore.ts
│   │   ├── messageStore.ts
│   │   ├── streamStore.ts
│   │   ├── multiModelStore.ts
│   │   ├── preferenceStore.ts
│   │   └── compressStore.ts
│   ├── feature/              # Feature module state (30+ stores)
│   │   ├── agentStore.ts
│   │   ├── agentProfileStore.ts
│   │   ├── appConfigStore.ts
│   │   ├── backupStore.ts
│   │   ├── buddyStore.ts
│   │   ├── categoryStore.ts
│   │   ├── decompositionStore.ts
│   │   ├── dreamStore.ts
│   │   ├── executionStore.ts
│   │   ├── expertStore.ts
│   │   ├── fileStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── gatewayLinkStore.ts
│   │   ├── generatedToolStore.ts
│   │   ├── helpStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── llmWikiStore.ts
│   │   ├── localToolStore.ts
│   │   ├── memoryStore.ts
│   │   ├── mcpStore.ts
│   │   ├── nudgeStore.ts
│   │   ├── onboardingStore.ts
│   │   ├── planStore.ts
│   │   ├── platformStore.ts
│   │   ├── proactiveStore.ts
│   │   ├── promptTemplateStore.ts
│   │   ├── providerStore.ts
│   │   ├── searchStore.ts
│   │   ├── settingsStore.ts
│   │   ├── skillExtensionStore.ts
│   │   ├── skillStore.ts
│   │   ├── styleStore.ts
│   │   ├── terminalStore.ts
│   │   ├── themeStore.ts
│   │   ├── trajectoryStore.ts
│   │   ├── userProfileStore.ts
│   │   ├── wikiStore.ts
│   │   ├── workEngineStore.ts
│   │   └── workflowEditorStore.ts
│   ├── devtools/             # DevTools state
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   ├── fineTuneStore.ts
│   │   └── recommendationStore.ts
│   └── shared/               # Shared state
│       ├── artifactStore.ts
│       ├── chatWorkspaceStore.ts
│       ├── tabStore.ts
│       └── uiStore.ts
│
├── components/                # React components (24 modules)
│   ├── chat/                # Chat interface (90+ components)
│   ├── workflow/            # Workflow editor (nodes/panels/templates/AI assist)
│   ├── gateway/             # API gateway UI
│   ├── settings/            # Settings panels (40+ components)
│   ├── terminal/            # Terminal UI
│   ├── skill/               # Skill editor & renderer
│   ├── benchmark/           # Benchmark panels
│   ├── decomposition/       # Skill decomposition & tool generation
│   ├── files/               # File management page
│   ├── fine-tune/           # LoRA fine-tuning config
│   ├── link/                # External link management
│   ├── llm-wiki/            # LLM Wiki editor
│   ├── proactive/           # Proactive suggestion system
│   ├── recommendation/      # Tool recommendation panel
│   ├── wiki/                # Wiki management
│   ├── devtools/            # Trace/Span timeline
│   ├── style/               # Code style transfer
│   ├── layout/              # Layout components (titlebar/sidebar/command palette)
│   ├── help/                # Help panel
│   ├── onboarding/          # Onboarding wizard
│   ├── notification/        # Notification center
│   ├── search/              # Session search
│   ├── common/              # Common components
│   └── shared/              # Shared components
│
├── pages/                    # Page components (22 pages)
│   ├── ChatPage.tsx
│   ├── KnowledgePage.tsx
│   ├── KnowledgeHubPage.tsx
│   ├── MemoryPage.tsx
│   ├── WorkflowPage.tsx
│   ├── WorkflowMarketplace.tsx
│   ├── GatewayPage.tsx
│   ├── GatewayLinkPage.tsx
│   ├── LinkPage.tsx
│   ├── FilesPage.tsx
│   ├── FineTunePage.tsx
│   ├── SkillsPage.tsx
│   ├── WikiPage.tsx
│   ├── WikiEditorPage.tsx
│   ├── WikiGraphPage.tsx
│   ├── LlmWikiPage.tsx
│   ├── LlmWikiEditorPage.tsx
│   ├── IngestPage.tsx
│   ├── QuickBarPage.tsx
│   ├── SettingsPage.tsx
│   └── DevTools/
│       ├── TraceExplorer.tsx
│       ├── BenchmarkRunner.tsx
│       └── ToolRecommender.tsx
│
├── hooks/                    # React hooks (10)
├── lib/                      # Utility functions (with Web Worker)
├── types/                    # TypeScript definitions (22)
├── sdk/                      # SDK (including Python SDK)
└── i18n/                     # 11 language translations
```

## Getting Started

### Download Pre-built

Visit the [Releases](https://github.com/polite0803/AxAgent/releases) page and download the installer for your platform.

### Build from Source

#### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + Rust MSVC targets

#### Build Steps

```bash
# Clone repository
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Install dependencies
npm install

# Development mode
npm run tauri dev

# Build frontend only
npm run build

# Build desktop application
npm run tauri build
```

Build artifacts are located in `src-tauri/target/release/`.

### Testing

```bash
# Unit tests
npm run test          # Vitest watch
npm run test:run      # Vitest single run

# E2E tests
npm run test:e2e      # Playwright
npm run test:e2e:ui   # Playwright UI mode

# Rust backend tests
cd src-tauri && cargo nextest run   # cargo-nextest (2-3x faster)
cd src-tauri && cargo test          # Standard tests

# Type checking
npm run typecheck     # TypeScript
cd src-tauri && cargo clippy -- -D warnings  # Rust

# Code formatting
npm run format        # dprint
cd src-tauri && cargo fmt

# CI full check
npm run ci:check
```

---

## Project Structure

```
AxAgent/
├── src/                         # Frontend source (React + TypeScript)
│   ├── components/              # React components (24 modules)
│   │   ├── chat/               # Chat interface (90+ components)
│   │   ├── workflow/           # Workflow editor components
│   │   ├── gateway/            # API gateway components
│   │   ├── settings/           # Settings panels (40+ components)
│   │   ├── terminal/           # Terminal components
│   │   ├── skill/              # Skill editor & renderer
│   │   ├── benchmark/          # Benchmark
│   │   ├── decomposition/      # Skill decomposition
│   │   ├── files/              # File management
│   │   ├── fine-tune/          # LoRA fine-tuning
│   │   ├── link/               # External links
│   │   ├── llm-wiki/           # LLM Wiki
│   │   ├── proactive/          # Proactive suggestions
│   │   ├── recommendation/     # Tool recommendation
│   │   ├── wiki/               # Wiki management
│   │   ├── devtools/           # DevTools
│   │   ├── style/              # Code style
│   │   ├── layout/             # Layout components
│   │   ├── help/               # Help panel
│   │   ├── onboarding/         # Onboarding wizard
│   │   ├── notification/       # Notification center
│   │   ├── search/             # Session search
│   │   ├── common/             # Common components
│   │   └── shared/             # Shared components
│   ├── pages/                   # Page components (22 pages)
│   ├── stores/                  # Zustand state management (62 stores)
│   │   ├── domain/            # Core business state (9 stores)
│   │   ├── feature/           # Feature module state (44 stores)
│   │   ├── devtools/          # DevTools state (5 stores)
│   │   └── shared/            # Shared state (4 stores)
│   ├── hooks/                   # React hooks (10)
│   ├── lib/                     # Utility functions (with Web Worker)
│   ├── types/                   # TypeScript definitions (22)
│   ├── sdk/                     # SDK (including Python SDK)
│   └── i18n/                    # 11 language translations
│
├── src-tauri/                    # Backend source (Rust)
│   ├── crates/                  # Rust workspace (18 crates)
│   │   ├── agent/             # AI Agent core
│   │   ├── core/              # Database, crypto, RAG, MCP
│   │   ├── providers/         # Model provider adapters
│   │   ├── runtime-core/      # Runtime abstraction layer
│   │   ├── runtime/           # Runtime services
│   │   ├── rt-workflow/       # Workflow engine
│   │   ├── rt-messaging/      # Message gateway
│   │   ├── rt-webhook/        # Webhook server
│   │   ├── rt-dashboard/      # Dashboard plugin
│   │   ├── rt-theme/          # Theme engine
│   │   ├── gateway/           # API gateway server
│   │   ├── tools/             # Tool system
│   │   ├── trajectory/        # Memory & learning
│   │   ├── telemetry/         # Tracing & metrics
│   │   ├── plugins/           # Plugin system
│   │   ├── prompt-guard/      # Prompt injection protection
│   │   ├── migration/         # Database migrations
│   │   └── npm/               # npm package parsing
│   └── src/                    # Tauri entry point (70+ command modules)
│
├── extension/                  # Browser extension (Wiki Clipper)
├── e2e/                        # Playwright E2E tests
├── scripts/                    # Build & utility scripts
└── website/                    # Project website (VitePress)
```

## Data Directories

```
~/.axagent/                      # Configuration directory
├── axagent.db                   # SQLite database
├── master.key                   # AES-256 master key
├── vector_db/                   # Vector database (sqlite-vec)
└── ssl/                         # SSL certificates

~/Documents/axagent/            # User files directory
├── images/                      # Image attachments
├── files/                       # File attachments
└── backups/                     # Backup files
```

---

## FAQ

### macOS: "App Is Damaged" or "Cannot Verify Developer"

Since the application is not signed by Apple:

**1. Allow apps from "Anywhere"**
```bash
sudo spctl --master-disable
```

Then go to **System Settings → Privacy & Security → Security** and select **Anywhere**.

**2. Remove quarantine attribute**
```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

**3. macOS Ventura+ additional step**
Go to **System Settings → Privacy & Security**, click **Open Anyway**.

---

## Community

- [LinuxDO](https://linux.do)

## License

This project is licensed under the [AGPL-3.0](LICENSE) License.
