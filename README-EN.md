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
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="License">
</p>

---

## What is AxAgent?

AxAgent is a comprehensive cross-platform AI desktop application that combines advanced AI agent capabilities with a rich set of developer tools. It features multi-provider model support, autonomous agent execution, visual workflow orchestration, local knowledge management, and a built-in API gateway.

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
- **Model Management** — Remote model list fetching, customizable parameters (temperature, max tokens, top-p, etc.)
- **Streaming Output** — Real-time token-by-token rendering with collapsible thinking blocks (Claude extended thinking)
- **Multi-Model Comparison** — Ask the same question to multiple models simultaneously with side-by-side comparison
- **Function Calling** — Structured function calling across all supported providers

### 🔐 AI Agent System

The agent system is built on a sophisticated architecture featuring:

- **ReAct Reasoning Engine** — Integrates reasoning and action with self-verification for reliable task execution
- **Hierarchical Planner** — Decomposes complex tasks into structured plans with phases and dependencies
- **Tool Registry** — Dynamic tool registration with semantic versioning and conflict detection
- **Deep Research** — Multi-source search orchestration, citation tracking, and credibility assessment
- **Fact Checking** — AI-driven fact verification with source classification
- **Computer Control** — AI-controlled mouse clicks, keyboard input, screen scrolling with vision model analysis
- **Screen Perception** — Screenshot capture and visual model analysis for UI element identification
- **Three Permission Levels** — Default (approval required), Accept Edits (auto-approve), Full Access (no prompts)
- **Sandbox Isolation** — Agent operations strictly confined to specified working directory
- **Tool Approval Panel** — Real-time display of tool call requests with per-item review
- **Cost Tracking** — Real-time token usage and cost statistics per session
- **Pause/Resume** — Pause agent execution anytime and resume later
- **Checkpoint System** — Persistent checkpoints for crash recovery and session resumption
- **Error Recovery Engine** — Automatic error classification and recovery strategy execution

### 👥 Multi-Agent Collaboration

- **Sub-Agent Coordination** — Master-slave architecture supporting multiple collaborative agents
- **Parallel Execution** — Multiple agents processing tasks in parallel with dependency-aware scheduling
- **Adversarial Debate** — Pro/Con debate rounds with argument strength scoring and refutation tracking
- **Agent Roles** — Predefined roles (researcher, planner, developer, reviewer, synthesizer) for team collaboration
- **Agent Orchestrator** — Centralized message routing and state management for multi-agent teams
- **Communication Graph** — Visual representation of agent interactions and message flow

### ⭐ Skills System

- **Skills Marketplace** — Built-in marketplace for browsing and installing community-contributed skills
- **Skill Creation** — Auto-create skills from proposals with Markdown editor
- **Skill Evolution** — AI-powered automatic analysis and improvement of existing skills based on execution feedback
- **Skill Matching** — Semantic matching to recommend relevant skills for conversation contexts
- **Atomic Skills** — Fine-grained skill components composable into complex workflows
- **Skill Decomposition** — Automatic breakdown of complex tasks into executable atomic skills
- **Generated Tools** — AI auto-generates and registers new tools to expand agent capabilities
- **Skills Hub** — Centralized management interface for skill discovery and configuration
- **Skills Hub Client** — Integration with remote skills hub for community sharing

### 🔄 Workflow System

The workflow engine implements a DAG-based task orchestration system:

- **Visual Workflow Editor** — Drag-and-drop workflow designer with node connection and configuration
- **Rich Node Types** — 14 node types: Trigger, Agent, LLM, Condition, Parallel, Loop, Merge, Delay, Tool, Code, AtomicSkill, VectorRetrieve, DocumentParser, Validation
- **Workflow Templates** — Built-in presets: Code Review, Bug Fix, Documentation, Testing, Refactoring, Exploration, Performance, Security, Feature Development
- **DAG Execution** — Kahn's algorithm for topological sorting with cycle detection
- **Parallel Dispatch** — Pipeline-style execution where fast steps don't wait for slow ones
- **Retry Policy** — Exponential backoff with configurable max retries per step
- **Partial Completion** — Failed steps don't block independent downstream steps
- **Version Management** — Workflow template versioning with rollback support
- **Execution History** — Detailed recording with status tracking and debugging
- **AI Assistance** — AI-assisted workflow design and optimization

### 📚 Knowledge & Memory

- **Knowledge Base (RAG)** — Multi-knowledgebase support with document upload, automatic parsing, chunking, and vector indexing
- **Hybrid Search** — Combines vector similarity search with BM25 full-text ranking
- **Reranking** — Cross-encoder reranking for improved retrieval precision
- **Knowledge Graph** — Entity relationship visualization of knowledge connections
- **Wiki System** — LLM Wiki compiler and validator with knowledge graph visualization and incremental sync
- **Memory System** — Multi-namespace memory with manual entry or AI-powered automatic extraction
- **Closed-Loop Memory** — Integration with Honcho and Mem0 for persistent memory providers
- **FTS5 Full-Text Search** — Fast retrieval across conversations, files, and memories
- **Session Search** — Advanced search across all conversation sessions
- **Context Management** — Flexible attachment of files, search results, knowledge snippets, memories, tool outputs

### 🌐 API Gateway

- **Local API Server** — Built-in OpenAI-compatible, Claude, and Gemini interface server
- **External Links** — One-click integration with Claude CLI, OpenCode with automatic API key sync
- **Key Management** — Generate, revoke, enable/disable access keys with descriptions
- **Usage Analytics** — Request volume and token usage by key, provider, and date
- **SSL/TLS Support** — Built-in self-signed certificates with custom certificate support
- **Request Logging** — Complete recording of all API requests and responses
- **Configuration Templates** — Pre-built templates for Claude, Codex, OpenCode, Gemini
- **Realtime API** — WebSocket event push compatible with OpenAI Realtime API
- **Platform Integrations** — Support for DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, Discord

### 🔧 Tools & Extensions

- **MCP Protocol** — Full Model Context Protocol implementation with stdio and HTTP/WebSocket transports
- **OAuth Authentication** — OAuth flow support for MCP servers
- **Plugin System** — Three-tier plugin architecture (builtin/bundled/external) with tool registration, hooks, and lifecycle management
- **Built-in Tools** — Comprehensive tool set for file operations, code execution, search, and more
- **LSP Client** — Built-in Language Server Protocol for code completion and diagnostics
- **Code Engine** — Lightweight code search runtime with AST indexing and three-level recall pipeline
- **Terminal Backends** — Support for Local, Docker, and SSH terminal connections
- **Browser Automation** — Integrated browser control capabilities via CDP
- **UI Automation** — Cross-platform UI element identification and control
- **Git Tools** — Git operations with branch detection and conflict awareness
- **Tool Recommendation** — Context-aware intelligent tool recommendation engine

### 📊 Content Rendering

- **Markdown Rendering** — Full support for code highlighting, LaTeX math, tables, task lists
- **Monaco Code Editor** — Embedded editor with syntax highlighting, copy, diff preview
- **Diagram Rendering** — Mermaid flowcharts, D2 architecture diagrams, ECharts interactive charts
- **Artifact Panel** — Code snippets, HTML drafts, React components, Markdown notes with live preview
- **Three Preview Modes** — Code (editor), Split (side-by-side), Preview (rendered only)
- **Session Inspector** — Tree view of session structure for quick navigation
- **Citation Panel** — Track and display source citations with credibility scoring

### 🛡️ Data & Security

- **AES-256 Encryption** — API keys and sensitive data encrypted with AES-256-GCM
- **Isolated Storage** — Application state in `~/.axagent/`, user files in `~/Documents/axagent/`
- **Auto Backup** — Scheduled backups to local directories or WebDAV storage
- **Backup Restore** — One-click restore from historical backups
- **Export Options** — PNG screenshots, Markdown, plain text, JSON formats
- **Storage Management** — Visual disk usage display with cleanup tools

### 🖥️ Desktop Experience

- **Theme Engine** — Dark/light themes with system-follow or manual preference
- **Interface Languages** — 11 languages: Simplified Chinese, Traditional Chinese, English, Japanese, Korean, French, German, Spanish, Russian, Hindi, Arabic
- **System Tray** — Minimize to tray without interrupting background services
- **Always on Top** — Pin window above others
- **Global Shortcuts** — Customizable shortcuts to summon main window
- **Auto Start** — Optional launch on system startup
- **Proxy Support** — HTTP and SOCKS5 proxy configuration
- **Auto Update** — Automatic version checking with update prompts
- **Command Palette** — `Cmd/Ctrl+K` for quick command access

### 🔬 Advanced Features

- **Deep Research** — Multi-source search, citation tracking, credibility assessment, and content synthesis
- **Fact Checking** — AI-driven fact verification with source classification
- **Cron Scheduler** — Automated task scheduling with daily, weekly, monthly templates and custom cron expressions
- **Webhook System** — Event subscriptions for tool completion, agent errors, session end notifications
- **User Profiling** — Automatic learning of coding style, naming conventions, indentation, comment style, communication preferences
- **RL Optimizer** — Reinforcement learning for tool selection and task strategy optimization
- **LoRA Fine-Tuning** — Custom model adaptation with local training using LoRA
- **Proactive Suggestions** — Context-aware nudges based on conversation content and user patterns
- **Dream Consolidation** — Background auto-consolidation of memories and patterns for long-term knowledge optimization
- **Thought Chain** — Reasoning visualization for agent decision-making with step-by-step breakdown
- **Error Recovery** — Automatic error classification, root cause analysis, and recovery suggestions
- **DevTools** — Trace, span, timeline visualization for debugging and performance analysis
- **Benchmark System** — SWE-bench / Terminal-bench task performance evaluation and metrics with score cards
- **Style Transfer** — Apply learned coding style preferences to generated code
- **Dashboard Plugins** — Extensible dashboard with custom panels and widgets
- **Collaboration** — CRDT-based real-time collaboration and one-click session sharing
- **Browser Extension** — Chrome extension for quick interaction with AxAgent

---

## Technical Architecture

### Tech Stack

| Layer | Technology |
|-------|------------|
| **Framework** | Tauri 2 + React 19 + TypeScript |
| **UI** | Ant Design 6 + TailwindCSS 4 |
| **State** | Zustand 5 |
| **i18n** | i18next + react-i18next |
| **Backend** | Rust + SeaORM + SQLite |
| **Vector DB** | sqlite-vec |
| **Code Editor** | Monaco Editor |
| **Diagrams** | Mermaid + D2 + ECharts (CDN) |
| **Terminal** | xterm.js |
| **Build** | Vite + npm |

### Rust Backend Architecture

The backend is organized as a Rust workspace with specialized crates:

```
src-tauri/crates/
├── agent/         # AI Agent core
│   ├── react_engine.rs          # ReAct reasoning engine
│   ├── tool_registry.rs         # Dynamic tool registration
│   ├── coordinator.rs           # Agent coordination
│   ├── hierarchical_planner.rs  # Task decomposition
│   ├── self_verifier.rs         # Output verification
│   ├── error_recovery_engine.rs # Error handling
│   ├── vision_pipeline.rs       # Screen perception
│   ├── deep_research.rs         # Deep research
│   ├── fact_checker.rs          # Fact checking
│   ├── research_agent.rs        # Research agent
│   ├── local_tool_registry.rs   # Local tool registry
│   ├── evaluator/               # Benchmark evaluation
│   ├── fine_tune/               # LoRA fine-tuning
│   ├── rl_optimizer/            # RL policy optimization
│   └── tool_recommender/        # Tool recommendation engine
│
├── code_engine/   # Code engine
│   └── lib.rs                  # Lightweight code search runtime (AST + three-level recall)
│
├── core/          # Core utilities
│   ├── db.rs                   # SeaORM database
│   ├── vector_store.rs         # sqlite-vec integration
│   ├── rag.rs                  # RAG abstraction layer
│   ├── hybrid_search.rs        # Vector + FTS5 search
│   ├── crypto.rs               # AES-256 encryption
│   ├── mcp_client.rs           # MCP protocol client
│   ├── browser_automation.rs   # Browser automation
│   ├── incremental_indexer.rs  # Incremental indexing
│   ├── marketplace_service.rs  # Marketplace service
│   └── storage_migration.rs    # Storage migration
│
├── gateway/       # API Gateway
│   ├── server.rs               # HTTP server
│   ├── handlers.rs              # API handlers
│   ├── auth.rs                 # Authentication
│   ├── marketplace_handlers.rs  # Marketplace endpoints
│   └── realtime.rs             # WebSocket support
│
├── plugins/       # Plugin system
│   ├── hooks.rs                # Hook runner
│   └── lib.rs                  # Plugin registry and lifecycle
│
├── providers/     # Model adapters
│   ├── openai.rs              # OpenAI API
│   ├── anthropic.rs           # Claude API
│   ├── gemini.rs              # Gemini API
│   ├── ollama.rs              # Ollama local
│   ├── openclaw.rs            # OpenClaw
│   └── hermes.rs              # Hermes
│
├── runtime/       # Runtime services
│   ├── session.rs             # Session management
│   ├── workflow_engine.rs     # DAG orchestration
│   ├── work_engine/           # Work engine (node executors)
│   ├── mcp.rs                 # MCP server
│   ├── cron/                  # Task scheduling
│   ├── terminal/              # Terminal backends (Local/Docker/SSH)
│   ├── benchmarks/            # SWE-bench / Terminal-bench
│   ├── collaboration/         # CRDT collaboration & session sharing
│   ├── tool_generator/        # AI tool generation
│   ├── message_gateway/       # Platform integrations (DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
│   ├── adversarial_debate.rs  # Adversarial debate
│   ├── agent_orchestrator.rs  # Multi-agent orchestration
│   ├── webhook_dispatcher.rs  # Webhook dispatching
│   ├── session_search.rs      # Session search
│   └── dashboard_plugin.rs    # Dashboard plugins
│
├── telemetry/     # Telemetry & tracing
│   ├── tracer.rs              # Distributed tracing
│   ├── metrics.rs             # Metrics collection
│   └── span.rs                # Span management
│
└── trajectory/    # Learning system
    ├── memory.rs              # Memory management
    ├── skill.rs               # Skill system
    ├── rl.rs                  # RL reward signals
    ├── behavior_learner.rs    # Pattern learning
    ├── user_profile.rs        # User profiling
    ├── auto_memory.rs         # Auto memory extraction
    ├── dream_consolidation.rs # Dream consolidation
    ├── parallel_execution.rs  # Parallel execution service
    ├── style_extractor.rs     # Style extraction
    ├── style_applier.rs       # Style application
    ├── suggestion_engine.rs   # Suggestion engine
    ├── atomic_skill/          # Atomic skill executor
    ├── memory_providers/      # Memory providers (Honcho/Mem0/Closed-loop)
    └── skill_decomposition/   # Skill decomposition (LLM-assisted/multi-turn)
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
│   │   └── workspaceStore.ts
│   ├── feature/              # Feature module state
│   │   ├── agentStore.ts
│   │   ├── gatewayStore.ts
│   │   ├── workflowEditorStore.ts
│   │   ├── knowledgeStore.ts
│   │   ├── atomicSkillStore.ts
│   │   ├── expertStore.ts
│   │   ├── memoryStore.ts
│   │   ├── skillStore.ts
│   │   ├── workEngineStore.ts
│   │   └── ... (30+ feature modules)
│   ├── devtools/             # DevTools state
│   │   ├── tracerStore.ts
│   │   ├── evaluatorStore.ts
│   │   ├── rlStore.ts
│   │   └── fineTuneStore.ts
│   └── shared/               # Shared state
│
├── components/
│   ├── chat/                # Chat interface (80+ components)
│   ├── workflow/            # Workflow editor
│   ├── gateway/             # API gateway UI
│   ├── settings/            # Settings panels
│   ├── terminal/            # Terminal UI
│   ├── atomicSkill/         # Atomic skill editor
│   ├── benchmark/           # Benchmark panels
│   ├── decomposition/       # Skill decomposition & tool generation
│   ├── files/               # File management page
│   ├── fine-tune/           # LoRA fine-tuning config
│   ├── link/                # External link management
│   ├── llm-wiki/            # LLM Wiki editor
│   ├── proactive/           # Proactive suggestion system
│   ├── recommendation/      # Tool recommendation panel
│   ├── wiki/                # Wiki management
│   ├── workEngine/          # Work engine controls
│   ├── devtools/            # Trace/Span timeline
│   ├── style/               # Code style transfer
│   ├── rl/                  # RL training monitor
│   └── shared/              # Shared components
│
└── pages/                   # Page components
    ├── ChatPage.tsx
    ├── KnowledgePage.tsx
    ├── MemoryPage.tsx
    ├── WorkflowPage.tsx
    ├── WorkflowMarketplace.tsx
    ├── GatewayPage.tsx
    ├── LinkPage.tsx
    ├── FilesPage.tsx
    ├── FineTunePage.tsx
    ├── SkillsPage.tsx
    ├── WikiPage.tsx
    ├── LlmWikiPage.tsx
    ├── PromptTemplatesPage.tsx
    ├── IngestPage.tsx
    └── SettingsPage.tsx
```

### Platform Support

| Platform | Architectures |
|----------|---------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows | x86_64, ARM64 |
| Linux | x86_64, ARM64 (AppImage/deb/rpm) |

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
npm run test

# E2E tests
npm run test:e2e

# Type checking
npm run typecheck
```

---

## Project Structure

```
AxAgent/
├── src/                         # Frontend source (React + TypeScript)
│   ├── components/              # React components
│   │   ├── chat/               # Chat interface (80+ components)
│   │   ├── workflow/           # Workflow editor components
│   │   ├── gateway/            # API gateway components
│   │   ├── settings/           # Settings panels
│   │   ├── terminal/           # Terminal components
│   │   ├── atomicSkill/       # Atomic skill editor
│   │   ├── benchmark/         # Benchmark
│   │   ├── decomposition/     # Skill decomposition
│   │   ├── files/             # File management
│   │   ├── fine-tune/         # LoRA fine-tuning
│   │   ├── link/              # External links
│   │   ├── llm-wiki/          # LLM Wiki
│   │   ├── proactive/         # Proactive suggestions
│   │   ├── recommendation/    # Tool recommendation
│   │   ├── wiki/              # Wiki management
│   │   ├── workEngine/        # Work engine
│   │   ├── devtools/          # DevTools
│   │   ├── style/             # Code style
│   │   ├── rl/                # RL training
│   │   └── shared/            # Shared components
│   ├── pages/                   # Page components (15+ pages)
│   ├── stores/                  # Zustand state management
│   │   ├── domain/            # Core business state
│   │   ├── feature/           # Feature module state (30+ stores)
│   │   ├── devtools/          # DevTools state
│   │   └── shared/            # Shared state
│   ├── hooks/                   # React hooks (16)
│   ├── lib/                     # Utility functions (with Web Worker)
│   ├── types/                   # TypeScript definitions
│   └── i18n/                    # 11 language translations
│
├── src-tauri/                    # Backend source (Rust)
│   ├── crates/                  # Rust workspace (11 crates)
│   │   ├── agent/             # AI Agent core
│   │   ├── code_engine/       # Code search engine
│   │   ├── core/              # Database, crypto, RAG
│   │   ├── gateway/           # API gateway server
│   │   ├── plugins/           # Plugin system
│   │   ├── providers/         # Model provider adapters
│   │   ├── runtime/           # Runtime services
│   │   ├── trajectory/        # Memory & learning
│   │   ├── telemetry/         # Tracing & metrics
│   │   └── migration/         # Database migrations
│   └── src/                    # Tauri entry point
│
├── extension/                  # Browser extension (Chrome)
├── e2e/                        # Playwright E2E tests
├── scripts/                    # Build & utility scripts
└── website/                    # Project website
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
└── backups/                      # Backup files
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
