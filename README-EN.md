# AxAgent

[![Release](https://img.shields.io/github/v/release/polite0803/AxAgent?style=flat-square)](https://github.com/polite0803/AxAgent/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/polite0803/AxAgent/release.yml?style=flat-square)](https://github.com/polite0803/AxAgent/actions)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20iOS-blue?style=flat-square)
![License](https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square)

<p align="center">
  <a href="./media/poster-axagent.svg">
    <img src="./media/poster-axagent.svg" alt="AxAgent Poster" width="80%" />
  </a>
</p>

**AxAgent** is a Tauri 2-based cross-platform AI desktop client (Windows / macOS / Linux / Android / iOS), positioned as an AI-driven workstation for daily development, research, knowledge management, and automation. It comes with a built-in ReAct agent engine, cognitive routing (three-tier hierarchical routing + Retrieval-Augmented Routing RAR), visual workflow orchestration, local RAG knowledge bases, MCP protocol extensions, a unified multi-model gateway, browser automation, and computer control — taking AI from "conversation" to "execution".

> **Languages**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Project Positioning

AxAgent solves three core problems:

1. **Unified Multi-Model Access & Intelligent Scheduling** — Use OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, ERNIE (Wenxin), Ollama local models, and any OpenAI-compatible API from a single interface, with automatic multi-key quota rotation, task-type intelligent routing, and streaming comparison
2. **The Closed Loop from Conversation to Execution** — 163+ built-in tools + visual workflows + MCP extensions + browser/computer control, enabling AI to operate files, run code, manage Git, and schedule tasks
3. **Local-First Data Sovereignty** — Conversations, knowledge bases, memories, and configuration are all stored in a local SQLite database; API keys are encrypted with AES-256-GCM, and core features run without any third-party cloud service

---

## Core Capabilities

### Cognitive Routing System (Cognitive Router)

AxAgent uses `cognitive_query` as the unified entry point for all conversations, mapping user intent to concrete capabilities through **three-tier hierarchical routing**:

- **L1 Domain Routing** (`domain_router`): Rules + LLM fallback, recognizing 9 major business domains (data analysis / content creation / communication / operations / AI media / finance / automation / general, etc.)
- **L2 Cluster Routing** (`cluster_router`): Locates capability clusters within a domain (27 clusters, covering 8 major business domains)
- **L3 Capability Routing**: **Retrieval-Augmented Routing (RAR)** — recalls Top-K similar workflows from the capability vector store and injects them into the prompt, combined with workflow DAG graph pathfinding, outputting path addresses (e.g., `/finance/stock_analysis/tech`) and execution modes
- **Execution Modes**: `Ask / Plan / Act / Workflow / Direct / Delegate / ParameterExtract / Clarify`, automatically selected by confidence
- **Capability System**: Unified registry (`CapabilityRegistry`) + vector index (`CapabilityIndexer`) + hybrid retrieval (`CapabilityRetriever`, vector + BM25 + tag hard matching + negative sample exclusion)
- **System Capability Isolation**: The cognitive orchestrator is physically isolated from business workflows; system capabilities carry a `SYSTEM_ONLY` visibility marker, and the routing layer has built-in self-reference circuit breaking to prevent self-referential paradoxes
- **Three-Tier Routing Implemented as Workflow DAGs**: 4 preset routing workflow templates (main orchestration ~20 nodes + L1/L2/L3 sub-routers), executed by the `rt-workflow` engine

### Multi-Model Engine

- **13 Provider Adapters**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, DeepSeek, Qwen, GLM, Kimi, ERNIE Bot (Wenxin Yiyan), Ollama, Llama.cpp (GGUF local models), OpenClaw, Hermes, and all OpenAI-compatible APIs
- **Multi-Key Rotation**: Multiple API keys per provider with automatic quota-based rotation and automatic failover when a single key is rate-limited
- **Intelligent Routing**: Automatically selects the optimal model by task type (code review / summarization / translation / general), with support for custom rules
- **Provider Health Monitoring**: Real-time tracking of success rate, latency, and availability, with tiered automatic degradation
- **AI Image Generation**: DALL-E 3 and Flux with multi-size presets
- **Real-Time Voice**: WebSocket voice conversation based on the OpenAI Realtime API, with interruption support and streaming transcription

### Agent System (ReAct Engine)

- **Hierarchical Planner** (`hierarchical_planner`): Decomposes complex tasks into Phase → Task structured plans, compiled into DAG topological execution
- **Deep Research** (`deep_research`): Multi-source search orchestration, including search planning, search execution, content synthesis, and citation tracking
- **Fact Checker** (`fact_checker`): AI-driven fact verification, including source classification and credibility assessment
- **Tree of Thoughts** (`tree_of_thoughts`): Multi-path reasoning exploration with branch evaluation and backtracking
- **Reflector** (`reflector`): Post-execution self-evaluation and improvement suggestions
- **Self-Verifier** (`self_verifier`): Automatic validation of reasoning results, including cycle detection
- **Error Recovery** (`error_recovery_engine`): Error type classification → recovery strategy selection → automatic retry or plan adjustment, with exponential backoff
- **A/B Testing** (`ab_testing`): Comparative evaluation of different reasoning strategies
- **Evaluation System** (`evaluator`): Built-in benchmark framework
- **LoRA Fine-Tuning** (`fine_tune`): Built-in training pipeline with LoRA adapter management
- **RL Optimizer** (`rl_optimizer`): Policy reinforcement learning based on experiential feedback

**Multi-Agent Collaboration**:

- Master-slave coordination architecture with parallel sub-agent execution and dependency-aware scheduling
- Shared blackboard for inter-agent information exchange
- Adversarial debate mode (Pro/Con rounds with argument strength scoring)
- Swarm cluster mode for multi-process agent clusters
- Proactive mode: agents can proactively initiate suggestions and operations

**Computer Control**: AI-driven mouse clicks, keyboard input, and screen scrolling, with three permission tiers (default / accept edits / full access) and sandboxed path isolation

**Browser Automation**: Browser control via the CDP protocol, supporting navigation, screenshots, clicking, form filling, and text extraction

### Skills System

- **Skill Marketplace**: Browse and install community skills
- **AI-Assisted Creation**: Automatically create skill structures from natural language proposals (`skill:create`)
- **Skill Evolution** (`evolution_engine`): Automatically analyze and improve skills based on execution feedback
- **Semantic Matching**: Automatically recommend relevant skills based on conversational context semantics
- **Skill Decomposition** (`skill_decomposition`): Automatically decompose complex tasks into combinations of atomic skills
- **Generated Tools**: AI generates and registers new tools
- **Sandbox Execution**: Skills execute safely in isolated sandboxes

### Visual Workflow

Drag-and-drop DAG workflow editor based on ReactFlow 12:

- **32 Node Types**: Trigger, Agent, LLM Call, Conditional Branch, Parallel Fork, Loop, Merge, Delay, Tool Call, Code Execution, Sub-workflow, Vector Retrieval, Document Parsing, Validation, End, HTTP Request, Switch, Database Query, Notification, Approval, File Operation, Data Transformation, Webhook Send, Log, LLM Classifier, Aggregator, Email, Debate, Swarm, Multi-Agent, Storage, Business Rule
- **Kahn Topological Sort Execution**: Automatic detection of circular dependencies with parallel pipeline scheduling
- **Built-in Templates**: Code review, bug fix, documentation, testing, refactoring, exploration, performance analysis, security audit, feature development
- **YAML Serialization**: Workflow definition import/export
- **Version Management**: Template version control
- **AI-Assisted Design**: AI-assisted workflow design, node recommendation, and diagnostics

### Knowledge Management

- **Multi-Knowledge-Base RAG**: Document upload → auto-parsing (PDF/DOCX/XLSX/PPTX/TXT) → chunking → vector indexing
- **Hybrid Retrieval**: Vector similarity (sqlite-vec + candle local embeddings) + BM25 full-text search (FTS5), with hybrid ranking
- **Self-RAG**: Automatic reflection and validation of retrieval results
- **Re-Ranking**: Cross-encoder result re-ranking
- **Knowledge Graph**: Entity extraction → relationship construction → visual graph
- **File Watching**: Real-time file change monitoring based on `notify`, with automatic incremental indexing
- **LLM Wiki**: AI-assisted Wiki compiler and validator

### Memory System

- **Multi-Namespace Memory**: Isolated by project/topic, supporting manual entry and automatic AI extraction
- **Persistent Integration**: Honcho and Mem0 closed-loop memory
- **User Profile**: Automatically learns coding style, tech stack preferences, and communication style
- **Style Transfer**: Extracts code style features → applies them to AI-generated code
- **Dream Integration**: Background automatic consolidation of memory fragments and behavioral patterns into structured knowledge
- **Project Memory**: Per-project context persistence

### API Gateway

Built-in HTTP + WebSocket gateway based on `axum`:

- **Compatible Endpoints**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API, plus OpenAI Responses and Realtime WebSocket
- **Key Management**: Generate, revoke, enable/disable access keys, with expiration support
- **Usage Tracking**: Per-key/per-provider/per-date request and token consumption statistics, with Prometheus metrics export
- **Rate Limiting**: Token bucket algorithm based on `governor`
- **SSL/TLS**: Built-in self-signed certificates (`rcgen`), with custom certificate support
- **External Linking**: One-click integration with Claude CLI, OpenCode, and other external tools, with automatic API key sync
- **Real-Time Tickets**: HMAC-based temporary authentication tickets for secure WebSocket connection handoff
- **Server Mode**: Optional `axagent-server` binary that exposes desktop application capabilities as a service

### Messaging Platform Integration

Multi-platform gateway via `rt-messaging`, supporting message reception, command parsing, and AI auto-reply for **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, and Discord**.

### Tool System

**163+ built-in tools**, uniformly registered through the `Tool` trait, covering 15 major categories:

| Category         | Example Tools                                                                                                                                                               |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| File Operations  | `file_read`, `file_write`, `file_edit`, `glob`, `grep`, directory/delete/move, etc. — 11 in total                                                                           |
| Shell/Web        | `bash`, `web_fetch`, `web_search`                                                                                                                                           |
| Network          | `http_request`, `ping`, `dns_lookup`, `json_api`, `rss_reader`, `graphql`, `websocket`                                                                                      |
| Browser          | `browser_navigate`, `browser_click`, `browser_fill`, `browser_screenshot`, etc. — 10 in total (CDP)                                                                         |
| Computer Control | `computer_use` (mouse/keyboard/screenshot)                                                                                                                                  |
| Git              | `git_status`, `git_diff`, `git_commit`, `git_log`, `git_branch`, `git_review`                                                                                               |
| Knowledge Base   | `list_knowledge_bases`, `search_knowledge`, `add_knowledge_document`, etc. — 6 in total                                                                                     |
| Task Management  | `todo_write`, `task_*` (6), `cron_*` (3), `plan`-related                                                                                                                    |
| Messaging        | `push_notification`, `send_message`, team collaboration tools                                                                                                               |
| Database         | `database_query`, `database_list_tables`, `database_migration_status`                                                                                                       |
| Storage          | `get_storage_info`, `upload_storage_file`, `download_storage_file`, etc. — 5 in total                                                                                       |
| Export/Format    | `export_word`, `export_pdf`, `export_xlsx`, `export_pptx`, `render_markdown`, etc. — 9 in total                                                                             |
| OCR              | `ocr_image`, `ocr_detect_langs`                                                                                                                                             |
| Obsidian         | `obsidian_search`, `obsidian_read`, `obsidian_backlinks`, etc. — 9 in total                                                                                                 |
| Other            | `agent`, `delegate_task`, `skills_*`, `lsp`, `repl`, `monitor`, `workspace_*`, `session_search`, `generate_image`, `sequential_thinking`, CI/CD, DevOps, RPC, testing, etc. |

### MCP Protocol

Complete MCP (Model Context Protocol) implementation based on `rmcp`:

- **Transport Layer**: stdio subprocess + Streamable HTTP + SSE
- **OAuth Authentication**: OAuth authorization flow for MCP servers
- **Tool Discovery**: Automatic discovery and registration of tools exposed by MCP servers
- **MCP Manager**: Server lifecycle management, health checks, automatic reconnection

### Plugin System

OpenClaw-compatible three-tier plugin architecture (built-in / bundled / external):

- Installed via npm packages, with a built-in marketplace UI for searching and installing
- Plugin manifest definition, permission declaration, sandbox-isolated execution
- Custom tool registration, Agent providers, Hook interception
- Skill installer: installs skills from plugin packages into the skills system

### Dynamic UI Engine

- **Schema-Driven**: Build interfaces declaratively via JSON Schema, without writing code
- **31 Built-in Components**: Containers (7) / Data Display (6) / Forms (9) / Media (4) / Other (5)
- **Data Binding**: Declarative data source binding and conditional rendering
- **NL2UI**: Generate dynamic UI interfaces directly from natural language

### ACP Client SDK

- **ACP (Agent Client Protocol)**: Dual-language SDK (TypeScript + Python) with zero third-party dependencies
- Session management, prompt sending, tool call recording, WebSocket event streams
- Communicates with the AxAgent service via the `/acp/v1/*` endpoints

### Security

- **AES-256-GCM Encryption**: Local encrypted storage of API keys and sensitive configuration (`crypto` crate)
- **Prompt Injection Protection**: Four-tier defense pipeline (`prompt-guard`) — pattern detection → delimiter escaping → XML wrapper → trust labels, integrated across conversations, prompt building, Git, and RAG
- **SSRF Protection**: URL safety checks that block requests to internal network addresses
- **Content Filtering**: Multi-type content safety filtering
- **Rate Limiting**: Token bucket rate limiting for tool calls and API requests
- **Circuit Breaker**: Automatic circuit breaking on consecutive failures
- **Access Control**: Policy-based tool access permission control
- **Sandbox Isolation**: Execution environment isolation for agents and skills

### Developer Tools

- **Distributed Tracing** (`telemetry`): OpenTelemetry integration with Span/Trace visualization
- **Structured Logging**: tracing-subscriber + chrono timestamps
- **Replay Debugging**: Agent execution trajectory recording (`trajectory_recorder`) and replay
- **DevTools Panel**: Trace Explorer timeline viewer, Benchmark Runner, Tool Recommender
- **Benchmarks**: Criterion benchmarks (tool_exec / llm_call / search)
- **CI Checks**: `npm run ci:check` integrating type checking, linting, and format validation

### Desktop & Mobile Experience

- **Responsive Layout**: CSS breakpoint-based adaptive layout for desktop/tablet/mobile (3 device tiers: `desktop` / `tablet` / `mobile`)
- **11 Languages**: Simplified Chinese, Traditional Chinese, English, Japanese, Korean, French, German, Spanish, Russian, Hindi, Arabic
- **Theme Engine** (`rt-theme`): Dark/light themes + multiple presets, deeply customized with Ant Design 6
- **Monaco Editor**: Syntax highlighting, diff preview, multi-language support
- **xterm.js Terminal**: WebLinks, Unicode 11, search
- **Virtual Scrolling**: @tanstack/react-virtual + react-virtuoso
- **Chart Rendering**: D2 + Mermaid + Recharts + Sigma (graphs)
- **Command Palette**: Ctrl+K global command palette
- **System Tray + Global Shortcuts + Auto-Start**: Non-intrusive background operation
- **Auto-Update**: Configurable-interval GitHub Releases version checking
- **Proxy Support**: HTTP / SOCKS5 proxy configuration
- **Cloud Workspace**: S3 and WebDAV storage sync, with conflict detection and bidirectional sync

### Mobile

- Android APK/AAB (arm64-v8a, armeabi-v7a, x86_64)
- iOS IPA (arm64)
- Mobile-specific adaptations: safe area insets, bottom navigation, drawer navigation

---

## Technical Architecture

### Tech Stack

| Layer              | Technology                               | Version |
| ------------------ | ---------------------------------------- | ------- |
| Desktop Framework  | Tauri                                    | 2.11    |
| Frontend Framework | React                                    | 19      |
| Type System        | TypeScript                               | 7       |
| UI Library         | Ant Design                               | 6       |
| CSS Framework      | TailwindCSS                              | 4       |
| State Management   | Zustand                                  | 5       |
| Routing            | React Router                             | 7       |
| Code Editor        | Monaco Editor                            | 0.55    |
| Terminal           | xterm.js                                 | 6       |
| Workflow Editor    | ReactFlow                                | 12      |
| Charts             | D2 + Mermaid + Recharts + Sigma          |         |
| Animation          | Framer Motion                            | 12      |
| Virtual Scrolling  | @tanstack/react-virtual + react-virtuoso |         |
| Drag & Drop        | @dnd-kit                                 | 6       |
| Markdown Rendering | markstream-react + stream-markdown       |         |
| i18n               | i18next + react-i18next                  |         |
| Build Tool         | Vite                                     | 8       |
| Testing            | Vitest + Playwright                      |         |
| Formatting         | dprint (TS/JSON/Markdown/TOML) + rustfmt |         |
| Linting            | ESLint + Oxlint + Clippy                 |         |

### Backend Architecture: Harness Dependency Injection

Rust workspace architecture with **37 members** (main crate + 35 library crates + schema-gen), following the **Harness dependency injection architecture**:

> All crates are decoupled through trait interfaces defined by axagent-harness, with axagent-runtime assembling and injecting dependencies at runtime.
> Dependency direction: `concrete implementations → harness ← callers`

**harness** is the architectural cornerstone — zero business logic, zero concrete implementations, containing only trait definitions, pure data DTOs, constants, and unified error types. It is depended upon by all other crates and depends on no axagent-* crate itself (200+ trait definitions covering Agent/Provider/Tool/RAG/Storage/MCP/Plugins/Security/Observability/Memory/Learning/Browser/Messaging/Cognitive Routing, etc.).

```
src-tauri/crates/
├── harness/          # Architectural cornerstone — trait interfaces, DTOs, error types, DI contracts
├── entities/         # SeaORM entity models
├── dao/              # Data access layer (CRUD)
├── migration/        # Database migrations
├── crypto/           # AES-256-GCM encryption/decryption & key management
├── credential/       # Secure credential storage
├── storage/          # File storage abstraction (local/S3/WebDAV), ZIP read/write
├── cache/            # In-memory cache layer
├── disk-cache/       # Disk-level file cache
├── search/           # Search engine (FTS5 + sqlite-vec + candle local embeddings)
├── document-parser/  # Document text extraction (PDF/DOCX/XLSX/PPTX)
├── kit/              # General utilities (paths/encoding/hashing/dates)
├── runtime-core/     # Runtime shared types, config constants
├── runtime/          # Runtime service orchestration — DI container assembling all crates
├── rt-workflow/      # Workflow engine — DAG orchestration, node executors, YAML serialization
├── rt-messaging/     # Messaging platform gateway — DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord
├── rt-webhook/       # General webhook server
├── rt-dashboard/     # Dashboard plugin framework
├── rt-theme/         # Theme engine
├── agent/            # AI agent core — 80+ modules
│                     #   ReAct engine/hierarchical planning/deep research/fact checking/tree of thoughts/
│                     #   reflection/self-verification/error recovery/RL optimization/LoRA fine-tuning/
│                     #   evaluation/tool recommendation/A-B testing/coordinator/blackboard/vision pipeline/
│                     #   web search/academic search/wiki compilation, etc.
├── orchestrator/     # Agent orchestration — multi-agent scheduling, DAG decomposition, dynamic subgraph execution
├── providers/        # Model provider adapters (13)
├── tools/            # Tool system — Tool trait/registry/orchestration/streaming/sandbox/163+ built-in tools
├── gateway/          # API gateway — axum HTTP/WS server, OAuth, rate limiting, Prometheus
├── mcp/              # MCP protocol — stdio + Streamable HTTP + SSE, based on rmcp
├── trajectory/       # Learning system — memory/skill evolution/user profiles/dream integration
├── plugins/          # Plugin system — OpenClaw compatible, npm package install, marketplace
├── telemetry/        # Observability — OpenTelemetry, structured logging, runtime metrics
├── prompt-guard/     # Prompt injection protection — L1-L4 multi-level detection pipeline
├── npm/              # npm registry client
├── crdt/             # Collaborative editing data structures
├── device/           # Device management
├── axagent-mobile/   # Mobile adaptation layer
├── agent-macro/      # Agent macros
├── agent-command-types/ # Agent command types
└── schema-gen/       # Database schema generation tool
```

### Frontend Architecture

```
src/
├── pages/            # Pages (24)
│   ├── ChatPage           # Chat interface — sidebar/message stream/Agent panel/multi-tab
│   ├── DashboardPage      # Dashboard — usage stats/model distribution/trend charts
│   ├── WorkflowPage       # Workflow editor — ReactFlow DAG visualization
│   ├── KnowledgeHubPage   # Knowledge base management — document upload/index/retrieval
│   ├── MemoryPage         # Memory management
│   ├── SkillsPage         # Skill marketplace
│   ├── SettingsPage       # Settings panel — 40+ configuration items
│   ├── TerminalPage       # Built-in terminal — xterm.js
│   ├── FilesPage          # File management
│   ├── GatewayLinkPage    # API gateway & external link management
│   ├── QuickBarPage       # Quick bar (standalone window)
│   ├── DynamicUIManagerPage / DynamicPageViewer  # Dynamic UI engine
│   ├── WikiGraphPage / WikiEditPage / IngestPage # LLM Wiki
│   ├── LearningGraphPage  # Learning graph
│   ├── FineTunePage       # LoRA fine-tuning
│   ├── PersonaPage        # Persona management
│   ├── WorkflowMarketplace # Workflow marketplace
│   ├── DevTools/          # TraceExplorer / BenchmarkRunner / ToolRecommender
│   └── Workflow/          # WorkflowListPage
│
├── components/       # 33 modules, 500+ components
│   ├── chat/         # Chat (message stream/input/ChatView/TabBar/RightPanel/attachments/tool call rendering)
│   ├── layout/       # Layout — AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader, etc.
│   ├── agent/        # Agent panel/entry/mini-panel
│   ├── workflow/     # Workflow editor (nodes/edges/panels/templates/AI assist)
│   ├── settings/     # Settings panel (40+ sub-components)
│   ├── skill/        # Skill editor/renderer/floating panels
│   ├── dynamicUI/    # Dynamic UI components (31 built-in components)
│   ├── gateway/      # API gateway management
│   ├── files/        # File management
│   ├── terminal/     # Terminal components
│   ├── search/       # Search interface
│   ├── benchmark/    # Benchmark panel
│   ├── decomposition/# Skill decomposition & tool generation
│   ├── devtools/     # Trace/Span timeline + RL Training panel
│   ├── approval/     # Approval workflow UI
│   ├── recommendation/ # Tool/model recommendation
│   ├── onboarding/   # WelcomeWizard / InteractiveTutorial
│   ├── help/         # Help panel
│   ├── notification/ # Notification components
│   ├── proactive/    # Proactive suggestions
│   ├── llm-wiki/     # LLM Wiki components
│   ├── wiki/         # Wiki components
│   ├── fine-tune/    # Fine-tuning UI
│   ├── trace/        # Trace components
│   ├── style/        # Style/theme
│   ├── shared/       # Shared components (ErrorBoundary / PageContextProvider)
│   └── common/       # Common components (Icon, etc.)
│
├── stores/           # Zustand state management (82 stores)
│   ├── domain/       # 9 core business stores (conversation/stream/compression/preferences/multi-model, etc.)
│   ├── feature/      # 61 feature module stores (agent/workflow/knowledge/skills/gateway/memory/terminal, etc.)
│   ├── shared/       # 8 cross-component shared stores (UI/tabs/workspace/backend state, etc.)
│   └── devtools/     # 4 developer tool stores
│
├── hooks/            # React Hooks (shortcuts/command palette/responsive/scrollbar/theme/avatar, etc.)
├── lib/              # Utility library (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout, etc. — 45+ modules)
├── types/            # TypeScript type definitions
├── theme/            # Shadcn theme engine
├── i18n/             # 11 language translation files (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # Constants & feature flags
└── sdk/              # ACP client SDK (TypeScript + Python)
```

### Feature Flags

The project manages progressive feature rollout via `featureFlags.ts`:

| Flag                | Status | Description                                         |
| ------------------- | ------ | --------------------------------------------------- |
| `AGENT_IN_THE_LOOP` | ✅     | Global Agent Panel + page context injection         |
| `DYNAMIC_UI`        | ✅     | Dynamic UI builder engine                           |
| `SELF_EVOLUTION_UI` | ❌     | Self-evolution frontend control panel               |
| `NL_EXTENSION`      | ❌     | Natural language-driven dynamic business extensions |

### Tauri Plugins

| Plugin              | Purpose                      |
| ------------------- | ---------------------------- |
| `autostart`         | Auto-start on boot           |
| `clipboard-manager` | Clipboard read/write         |
| `dialog`            | File selection dialogs       |
| `fs`                | File system access           |
| `global-shortcut`   | Global shortcut registration |
| `notification`      | System notifications         |
| `opener`            | External link/file opening   |
| `process`           | Process management           |
| `updater`           | Auto-update                  |

---

## Data Directories

```
~/.axagent/                    # Application configuration
├── axagent.db                 # SQLite main database (SeaORM)
├── master.key                 # AES-256 master key
├── vector_db/                 # sqlite-vec vector index
└── ssl/                       # Self-signed SSL certificates

~/Documents/axagent/          # User files
├── images/                   # Image attachments
├── files/                    # File attachments
└── backups/                  # Automatic backups
```

---

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+ (edition 2024)
- [npm](https://www.npmjs.com/) 10+
- Windows: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC + Windows SDK)
- macOS: Xcode Command Line Tools
- Linux: `build-essential` + `libwebkit2gtk-4.1-dev` + `libssl-dev`

### Development

```bash
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent
npm install
npm run tauri dev      # Development mode (frontend Vite HMR + Tauri window)
```

### Build

```bash
npm run tauri build    # Desktop production build

npm run tauri:android:build   # Android build
npm run tauri:ios:build       # iOS build
```

Desktop build artifacts are located in `src-tauri/target/release/`.

### Testing

```bash
npm run test           # Frontend unit tests (Vitest watch)
npm run test:run       # Frontend unit tests (single run)
npm run test:e2e       # E2E tests (Playwright)

# Rust backend tests
cd src-tauri && cargo test

# Type checking & Linting
npm run typecheck
cd src-tauri && cargo clippy -- -D warnings
npm run format         # dprint formatting
npm run lint:eslint    # ESLint check
npm run contracts      # API contract check

# Full CI check
npm run ci:check
```

### Common Scripts

| Command                  | Purpose                        |
| ------------------------ | ------------------------------ |
| `npm run bump`           | Interactive version bump       |
| `npm run docs`           | Generate TypeDoc documentation |
| `npm run skill:create`   | Create new skill scaffold      |
| `npm run skill:validate` | Validate skill definition      |
| `npm run check:types`    | Type consistency check         |

---

## Platform Support

| Platform | Architecture                          |
| -------- | ------------------------------------- |
| Windows  | x86_64, ARM64                         |
| macOS    | Apple Silicon (arm64), Intel (x86_64) |
| Linux    | x86_64, ARM64                         |
| Android  | arm64-v8a, armeabi-v7a, x86_64        |
| iOS      | arm64                                 |

---

## Open Source License

This project is open-sourced under the [AGPL-3.0-only](LICENSE) license.

---

## Acknowledgments

AxAgent is built upon many outstanding open-source projects:

- [Tauri](https://tauri.app/) — Cross-platform desktop framework
- [React](https://react.dev/) + [Ant Design](https://ant.design/) — Frontend UI
- [SeaORM](https://www.sea-ql.org/SeaORM/) — Rust ORM
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — Vector search
- [candle](https://github.com/huggingface/candle) — Local embedding inference
- [rmcp](https://github.com/nicholasxjy/rmcp) — Rust MCP SDK
- [ReactFlow](https://reactflow.dev/) — Visual workflow editor
- [axum](https://github.com/tokio-rs/axum) — HTTP framework
- [Monaco Editor](https://microsoft.github.io/monaco-editor/) — Code editor
- [xterm.js](https://xtermjs.org/) — Terminal emulator
- [Zustand](https://zustand.docs.pmnd.rs/) — State management
- [Framer Motion](https://www.framer.com/motion/) — Animation library
- [Recharts](https://recharts.org/) — Chart library
