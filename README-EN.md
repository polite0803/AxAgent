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

**AxAgent** is a Tauri 2-based cross-platform AI assistant desktop client (Windows / macOS / Linux / Android / iOS). It integrates a ReAct agent engine, visual workflow orchestration, local RAG knowledge bases, MCP protocol extensions, a unified multi-model gateway, browser automation, and computer control — serving as an AI-powered workstation for daily development, research, knowledge management, and automation.

> **Languages**: [简体中文](./README.md) | [English](./README-EN.md) | [繁體中文](./README-ZH-TW.md) | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

---

## Project Positioning

AxAgent addresses three core problems:

1. **Unified Multi-Model Access & Intelligent Routing** — Use OpenAI, Anthropic Claude, Google Gemini, Ollama local models, and any OpenAI-compatible API within a single interface, with multi-key quota-based rotation, task-type intelligent routing, and streaming comparison
2. **AI From Conversation to Execution** — 47+ built-in tools + visual workflows + MCP extensions + browser/computer control, enabling AI to operate files, run code, manage Git, and schedule tasks
3. **Local-First Data Sovereignty** — Conversations, knowledge bases, memories, and configuration are all stored in a local SQLite database, with AES-256-GCM encryption for API keys; core features run without third-party cloud services

---

## Core Capabilities

### Multi-Model Engine

- **9 Provider Adapters**: OpenAI (Chat Completions + Responses + Realtime), Anthropic Claude, Google Gemini, Ollama (with GGUF local model management), OpenClaw, Hermes, and all OpenAI-compatible APIs
- **Multi-Key Rotation**: Multiple API keys per provider with quota-based automatic rotation and single-key rate-limit auto-failover
- **Intelligent Routing**: Automatic model selection by task type (code review / summarization / translation / general), with customizable routing rules
- **Provider Health Monitoring**: Real-time tracking of success rate, latency, and availability, with tiered automatic fallback
- **AI Image Generation**: DALL-E 3 and Flux (Replicate) with multi-size presets
- **Real-Time Voice**: WebSocket-based voice conversation via OpenAI Realtime API, with interruption support and streaming transcription

### Agent System (ReAct Engine)

- **Hierarchical Planner** (`hierarchical_planner`): Decomposes complex tasks into Phase → Task structured plans, compiled into DAG-based topological execution
- **Deep Research** (`deep_research`): Multi-source search orchestration including search planning, search execution, content synthesis, and citation tracking
- **Fact Checker** (`fact_checker`): AI-driven fact verification with source classifier and credibility evaluation
- **Tree of Thoughts** (`tree_of_thoughts`): Multi-path reasoning exploration with branch evaluation and backtracking
- **Reflector** (`reflector`): Post-execution self-evaluation and improvement suggestions
- **Self-Verifier** (`self_verifier`): Automatic reasoning result validation with cycle detection
- **Error Recovery** (`error_recovery_engine`): Error type classification → recovery strategy selection → automatic retry or plan adjustment, with exponential backoff
- **A/B Testing** (`ab_testing`): Comparative evaluation of different reasoning strategies
- **Evaluation System** (`evaluator`): Built-in benchmark framework
- **LoRA Fine-Tuning** (`fine_tune`): Built-in training pipeline with LoRA adapter management
- **RL Optimizer** (`rl_optimizer`): Experience-based policy reinforcement learning

**Multi-Agent Collaboration**:

- Master-slave coordination architecture with parallel sub-agent execution and dependency-aware scheduling
- Shared blackboard for inter-agent information exchange
- Adversarial debate mode (Pro/Con rounds with argument strength scoring)
- Swarm cluster mode for multi-process agent clusters
- Proactive mode: agents can proactively initiate suggestions and operations

**Computer Control**: AI-driven mouse clicks, keyboard input, screen scrolling, with three permission tiers (default / accept edits / full access) and sandboxed path isolation

**Browser Automation**: Browser control via CDP protocol, supporting navigation, screenshots, clicks, form filling, and text extraction

### Skill System

- **Skill Marketplace**: Browse and install community skills
- **AI-Assisted Creation**: Auto-create skill structures from natural language proposals (`skill:create`)
- **Skill Evolution** (`evolution_engine`): Automatic analysis and improvement of skills based on execution feedback
- **Semantic Matching**: Context-aware semantic skill recommendation
- **Skill Decomposition** (`skill_decomposition`): Automatic decomposition of complex tasks into atomic skill combinations
- **Generated Tools**: AI-generated and registered new tools
- **Sandbox Execution**: Skills execute in isolated sandbox environments

### Visual Workflow

Drag-and-drop DAG workflow editor based on ReactFlow 12:

- **17 Node Types**: Trigger, Agent, LLM Call, Conditional Branch, Parallel Fork, Loop, Merge, Delay, Tool Call, Code Execution, Sub-workflow, Vector Retrieval, Document Parsing, Validation, End, Business Rule, Agent Role
- **Kahn Topological Sort Execution**: Automatic cycle detection with parallel pipeline scheduling
- **Built-in Templates**: Code review, bug fix, documentation, testing, refactoring, exploration, performance analysis, security audit, feature development
- **YAML Serialization**: Workflow import/export in YAML format
- **Version Management**: Template version control
- **AI-Assisted Design**: AI-assisted workflow design and node recommendation

### Knowledge Management

- **Multi-Knowledge-Base RAG**: Document upload → auto-parsing (PDF/DOCX/XLSX/PPTX/TXT) → chunking → vector indexing
- **Hybrid Retrieval**: Vector similarity (sqlite-vec + candle local embeddings) + BM25 full-text search (FTS5), hybrid ranking
- **Self-RAG**: Automatic reflection and validation of retrieval results
- **Re-Ranking**: Cross-encoder result re-ranking for improved precision
- **Knowledge Graph**: Entity extraction → relationship construction → visual graph
- **File Watching**: Real-time file change monitoring via `notify` with automatic incremental indexing
- **LLM Wiki**: AI-assisted Wiki compiler and validator

### Memory System

- **Multi-Namespace Memory**: Project/topic-isolated memory with manual entry and automatic AI extraction
- **Persistent Integration**: Honcho and Mem0 closed-loop memory
- **User Profile**: Automatic learning of coding style, tech stack preferences, and communication style
- **Style Transfer**: Code style feature extraction → application to AI-generated code
- **Dream Integration**: Background automatic consolidation of memory fragments and behavioral patterns into structured knowledge
- **Project Memory**: Per-project context persistence

### API Gateway

Built-in HTTP + WebSocket gateway based on `axum`:

- **Compatible Endpoints**: OpenAI `/v1/chat/completions`, Claude Messages API, Gemini API, plus OpenAI Responses and Realtime WebSocket
- **Key Management**: Generate, revoke, enable/disable access keys with expiration support
- **Usage Tracking**: Per-key, per-provider, per-date request and token consumption statistics with Prometheus metrics export
- **Rate Limiting**: Token bucket algorithm via `governor`
- **SSL/TLS**: Built-in self-signed certificates (`rcgen`) with custom certificate support
- **External Linking**: One-click integration with Claude CLI, OpenCode, and other external tools with automatic API key sync
- **Real-Time Tickets**: HMAC-based temporary authentication tickets for secure WebSocket connection handoff

### Messaging Platform Integration

Multi-platform gateway via `rt-messaging`, supporting message reception, command parsing, and AI auto-reply for **DingTalk, Feishu, QQ, Slack, WeChat, WhatsApp, Telegram, and Discord**.

### Tool System

47+ built-in tools, uniformly registered through the `Tool` trait:

| Category         | Tools                                                                                                                                                                                                      |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| File Operations  | `file_read`, `file_write`, `file_edit`, `file_system`                                                                                                                                                      |
| Code Execution   | `bash`, `repl`                                                                                                                                                                                             |
| Search           | `grep`, `glob`                                                                                                                                                                                             |
| Browser          | `browser` (CDP)                                                                                                                                                                                            |
| Computer Control | `computer_use` (mouse/keyboard/screenshot)                                                                                                                                                                 |
| Web              | `web_search`, `web_fetch`                                                                                                                                                                                  |
| Knowledge Base   | `knowledge`, `document`                                                                                                                                                                                    |
| Git              | `git` (commit/push/branch/diff)                                                                                                                                                                            |
| Dev Tools        | `lsp`, `workspace`                                                                                                                                                                                         |
| Task Management  | `plan`, `task_system`, `todo_write`, `cron`                                                                                                                                                                |
| Messaging        | `push_notification`, `messaging`                                                                                                                                                                           |
| Database         | `database`                                                                                                                                                                                                 |
| Storage          | `storage`                                                                                                                                                                                                  |
| Other            | `agent`, `agent_memory`, `context`, `export`, `integration`, `media`, `media_delivery`, `migration_tool`, `monitor`, `obsidian`, `ocr`, `personality`, `shared_path`, `system_info`, `testing`, `worktree` |

### MCP Protocol

Complete MCP (Model Context Protocol) implementation based on `rmcp`:

- **Transport**: stdio subprocess + Streamable HTTP + WebSocket
- **OAuth Authentication**: OAuth authorization flow for MCP servers
- **Tool Discovery**: Automatic discovery and registration of MCP server-exposed tools
- **MCP Manager**: Server lifecycle management, health checks, automatic reconnection

### Plugin System

OpenClaw-compatible three-tier plugin architecture (built-in / bundled / external):

- npm package installation with marketplace UI for search and install
- Plugin manifest definition, permission declaration, sandbox-isolated execution
- Custom tool registration, Agent providers, Hook interception
- Skill installer: install skills from plugin packages into the skill system

### Security

- **AES-256-GCM Encryption**: Local encrypted storage of API keys and sensitive configuration (`crypto` crate)
- **Prompt Injection Protection**: Four-tier defense pipeline (`prompt-guard`) — pattern detection → delimiter escaping → XML wrapper → trust labels, integrated across conversations, prompt building, Git, and RAG
- **SSRF Protection**: URL safety checking to block requests to internal network addresses
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
- **Theme Engine** (`rt-theme`): Dark/light themes + multiple presets (including 21th monospace theme), deeply customized with Ant Design 6
- **Monaco Editor**: Syntax highlighting, diff preview, multi-language support
- **xterm.js Terminal**: WebLinks, Unicode 11, search
- **Virtual Scrolling**: @tanstack/react-virtual + react-virtuoso
- **Chart Rendering**: D2 + Mermaid + Recharts
- **Global Copy Menu**: Custom text selection copy menu, suppressing native context menu
- **Command Palette**: Ctrl+K global command palette
- **System Tray + Global Shortcuts + Auto-Start**: Non-intrusive background operation
- **Auto-Update**: Configurable-interval GitHub Releases version checking
- **Proxy Support**: HTTP / SOCKS5 proxy configuration
- **Cloud Workspace**: S3 and WebDAV storage sync with conflict detection and bidirectional sync

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
| Charts             | D2 + Mermaid + Recharts                  |         |
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

Rust workspace architecture with **32 crates**, following the **Harness DI pattern**:

> All crates are decoupled through trait interfaces defined by axagent-harness, with axagent-runtime assembling and injecting dependencies at runtime.
> Dependency direction: `concrete implementations → harness ← callers`

**harness** is the architectural cornerstone — zero business logic, zero concrete implementations, containing only trait definitions, pure data DTOs, constants, and unified error types. It is depended upon by all other crates and depends on no axagent-* crate itself (200+ trait definitions covering Agent/Provider/Tool/RAG/Storage/MCP/Plugins/Security/Observability/Memory/Learning/Browser/Messaging, etc.).

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
├── runtime/          # Runtime service orchestration — DI container assembling all 30+ crates
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
├── providers/        # Model provider adapters
├── tools/            # Tool system — Tool trait/registry/orchestration/streaming/sandbox/47+ built-in tools
├── gateway/          # API gateway — axum HTTP/WS server, OAuth, rate limiting, Prometheus
├── mcp/              # MCP protocol — stdio + Streamable HTTP, based on rmcp
├── trajectory/       # Learning system — memory/skill evolution/user profiles/dream integration
├── plugins/          # Plugin system — OpenClaw compatible, npm package install, marketplace
├── telemetry/        # Observability — OpenTelemetry, structured logging, runtime metrics
├── prompt-guard/     # Prompt injection protection — L1-L4 multi-level detection pipeline
├── npm/              # npm registry client
└── schema-gen/       # Database schema generation tool
```

### Frontend Architecture

```
src/
├── pages/            # Pages (23+ including sub-pages)
│   ├── ChatPage           # Chat interface — sidebar/message stream/Agent panel/multi-tab
│   ├── DashboardPage      # Dashboard — usage stats/model distribution/trend charts
│   ├── WorkflowPage       # Workflow editor — ReactFlow DAG visualization
│   ├── KnowledgeHubPage   # Knowledge base management — document upload/index/search
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
├── components/       # 28 modules, 450+ components
│   ├── chat/         # Chat (message stream/input/ChatView/TabBar/RightPanel/attachments/tool call rendering)
│   ├── layout/       # Layout — 17 components
│   │                 #   AppInitializer / Sidebar / ContentArea / TitleBar /
│   │                 #   CommandPalette / GlobalCopyMenu / GlobalErrorBoundary /
│   │                 #   GlobalStatusBar / ErrorNotificationToast / AppHeader /
│   │                 #   BackendStatusIndicator / IpcReconnectBanner /
│   │                 #   ModuleErrorBoundary / NotificationBell / UserProfileModal etc.
│   ├── agent/        # Agent panel/entry/mini-panel
│   ├── workflow/     # Workflow editor (nodes/edges/panels/templates/AI assist)
│   ├── settings/     # Settings panel (40+ sub-components)
│   ├── skill/        # Skill editor/renderer/floating panels
│   ├── dynamicUI/    # Dynamic UI component registry (26 built-in components)
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
├── stores/           # Zustand state management
│   ├── domain/       # 10 core business stores (conversation/stream/compression/preferences/multi-model, etc.)
│   ├── feature/      # 48 feature module stores (agent/workflow/knowledge/skills/gateway/memory/terminal, etc.)
│   └── devtools/     # 4 developer tool stores
│
├── hooks/            # React Hooks (shortcuts/command palette/responsive/scrollbar/theme/avatar, etc.)
├── lib/              # Utility library (invoke/pageRegistry/shortcuts/skillLifecycle/
│                     #   chartGenerator/codeExecutor/tokenEstimator/workflowLayout etc. — 45+ modules)
├── types/            # TypeScript type definitions
├── theme/            # Shadcn theme engine
├── i18n/             # 11 language translation files (zh-CN/zh-TW/en-US/ja/ko/fr/de/es/ru/hi/ar)
├── constants/        # Constants & feature flags
└── sdk/              # External integration SDK
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

| Plugin              | Purpose                           |
| ------------------- | --------------------------------- |
| `autostart`         | Auto-start on boot                |
| `clipboard-manager` | Clipboard read/write              |
| `dialog`            | File selection dialogs            |
| `fs`                | File system access                |
| `global-shortcut`   | Global shortcut registration      |
| `notification`      | System notifications              |
| `opener`            | External link/file opening        |
| `process`           | Process management                |
| `updater`           | Auto-update                       |
| `mcp-bridge`        | MCP protocol bridge (non-Android) |

---

## Data Directory

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
npm run tauri dev      # Development mode (Vite HMR + Tauri window)
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
cd src-tauri && cargo nextest run
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

### Scripts

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

## License

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
