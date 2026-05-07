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
- **Plugin System** — Three-tier plugin architecture (builtin/bundled/external) with tool registration, hooks, and lifecycle management
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

### Rust Backend Architecture

The backend is organized as a Rust workspace with 10 specialized crates:

```
src-tauri/crates/
├── agent/         # AI Agent core
│   ├── react_engine.rs          # ReAct reasoning engine
│   ├── coordinator.rs           # Agent coordination
│   ├── hierarchical_planner.rs  # Task decomposition
│   ├── task_decomposer.rs       # Sub-task decomposition
│   ├── self_verifier.rs         # Output verification
│   ├── verification_agent.rs    # Verification agent
│   ├── error_recovery_engine.rs # Error recovery engine
│   ├── error_classifier.rs      # Error classification
│   ├── recovery_strategies.rs   # Recovery strategies
│   ├── loop_detector.rs         # Loop detection
│   ├── vision_pipeline.rs       # Screen perception
│   ├── deep_research.rs         # Deep research
│   ├── fact_checker.rs          # Fact checking
│   ├── research_agent.rs        # Research agent
│   ├── search_planner.rs        # Search planning
│   ├── search_orchestrator.rs   # Search orchestration
│   ├── academic_search.rs       # Academic search
│   ├── source_validator.rs      # Source validation
│   ├── source_classifier.rs     # Source classification
│   ├── credibility_evaluator.rs # Credibility evaluation
│   ├── citation_tracker.rs      # Citation tracking
│   ├── content_synthesizer.rs   # Content synthesis
│   ├── outline_builder.rs       # Outline building
│   ├── reference_builder.rs     # Reference building
│   ├── proactive_mode.rs        # Proactive mode
│   ├── purpose_manager.rs       # Purpose management
│   ├── graph_insights.rs        # Graph insights
│   ├── insight_generator.rs     # Insight generation
│   ├── schema_manager.rs        # Schema management
│   ├── ingest_pipeline.rs       # Data ingest pipeline
│   ├── session_manager.rs       # Session management
│   ├── health_checker.rs        # Health checking
│   ├── metrics.rs               # Metrics collection
│   ├── evaluator/               # Benchmark evaluation
│   ├── fine_tune/               # LoRA fine-tuning
│   ├── rl_optimizer/            # RL policy optimization
│   └── tool_recommender/        # Tool recommendation engine
│
├── core/          # Core utilities
│   ├── db.rs                   # SeaORM database
│   ├── vector_store.rs         # sqlite-vec integration
│   ├── rag.rs                  # RAG abstraction layer
│   ├── hybrid_search.rs        # Vector + FTS5 search
│   ├── recall_pipeline.rs      # Three-level recall pipeline
│   ├── crypto.rs               # AES-256 encryption
│   ├── mcp_client.rs           # MCP protocol client
│   ├── browser_automation.rs   # Browser automation
│   ├── computer_control.rs     # Computer control
│   ├── screen_vision.rs        # Screen vision
│   ├── screen_capture.rs       # Screen capture
│   ├── ui_automation.rs        # UI automation
│   ├── ast_index.rs            # AST index
│   ├── incremental_indexer.rs  # Incremental indexing
│   ├── document_parser.rs      # Document parsing
│   ├── markdown_parser.rs      # Markdown parsing
│   ├── text_chunker.rs         # Text chunking
│   ├── token_counter.rs        # Token counting
│   ├── token_budget.rs         # Token budget
│   ├── file_index.rs           # File index
│   ├── file_authorizer.rs      # File authorization
│   ├── file_store.rs           # File storage
│   ├── cache.rs                # Cache management
│   ├── disk_cache.rs           # Disk cache
│   ├── cache_persister.rs      # Cache persistence
│   ├── cache_snapshot.rs       # Cache snapshot
│   ├── vector_cache.rs         # Vector cache
│   ├── marketplace_service.rs  # Marketplace service
│   ├── marketplace.rs          # Marketplace abstraction
│   ├── operation_audit.rs      # Operation audit
│   ├── unified_config.rs       # Unified config
│   ├── platform_config.rs      # Platform config
│   ├── command_validator.rs    # Command validation
│   ├── shell_parser.rs         # Shell parsing
│   ├── output_processor.rs     # Output processing
│   ├── storage_inventory.rs    # Storage inventory
│   ├── storage_migration.rs    # Storage migration
│   ├── storage_paths.rs        # Storage paths
│   ├── s3_backup.rs            # S3 backup
│   ├── webdav.rs               # WebDAV sync
│   ├── git_tools.rs            # Git tools
│   ├── sandbox_runner.rs       # Sandbox runner
│   ├── search.rs               # Search abstraction
│   ├── reranker.rs             # Reranking
│   ├── model_knowledge.rs      # Model knowledge
│   ├── prompt_template.rs      # Prompt template
│   ├── preset_templates.rs     # Preset templates
│   ├── workflow_types.rs       # Workflow types
│   ├── workflow_version.rs     # Workflow versioning
│   ├── path_vars.rs            # Path variables
│   ├── entity/                 # SeaORM entities (40+ tables)
│   └── repo/                   # Data repositories (30+ repos)
│
├── gateway/       # API Gateway
│   ├── server.rs               # HTTP server
│   ├── handlers.rs             # API handlers
│   ├── routes.rs               # Route definitions
│   ├── auth.rs                 # Authentication
│   ├── middleware.rs           # Middleware
│   ├── metrics.rs              # Metrics collection
│   ├── native.rs               # Native integration
│   ├── marketplace_handlers.rs # Marketplace endpoints
│   └── realtime.rs             # WebSocket support
│
├── plugins/       # Plugin system
│   ├── hooks.rs                # Hook runner
│   ├── agent_provider.rs       # Agent provider
│   ├── test_isolation.rs       # Test isolation
│   └── lib.rs                  # Plugin registry and lifecycle
│
├── providers/     # Model adapters
│   ├── adapter.rs              # Adapter interface
│   ├── registry.rs             # Provider registry
│   ├── openai.rs               # OpenAI API
│   ├── openai_responses.rs     # OpenAI Responses API
│   ├── anthropic.rs            # Claude API
│   ├── gemini.rs               # Gemini API
│   ├── ollama.rs               # Ollama local
│   ├── openclaw.rs             # OpenClaw
│   ├── hermes.rs               # Hermes
│   ├── image_gen.rs            # Image generation
│   ├── realtime_client.rs      # Realtime API client
│   └── transport/              # Transport layer (Chat Completions / Responses / Anthropic)
│
├── runtime/       # Runtime services
│   ├── session.rs              # Session management
│   ├── workflow_engine.rs      # DAG orchestration
│   ├── work_engine/            # Work engine (node executors + scheduler + cache layer)
│   ├── mcp.rs                  # MCP server
│   ├── mcp_client.rs           # MCP client
│   ├── mcp_server.rs           # MCP server implementation
│   ├── mcp_stdio.rs            # MCP stdio transport
│   ├── mcp_autostart.rs        # MCP autostart
│   ├── mcp_lifecycle_hardened.rs # MCP lifecycle management
│   ├── mcp_tool_bridge.rs      # MCP tool bridge
│   ├── cron/                   # Task scheduling
│   ├── terminal/               # Terminal backends (Local/Docker/SSH)
│   ├── benchmarks/             # SWE-bench / Terminal-bench
│   ├── collaboration/          # CRDT collaboration & session sharing
│   ├── tool_generator/         # AI tool generation
│   ├── message_gateway/        # Platform integrations (DingTalk/Feishu/QQ/Slack/WeChat/WhatsApp/Telegram/Discord)
│   ├── buddy/                  # Buddy system (species/attributes/manager)
│   ├── swarm/                  # Swarm cluster (process backend/permission sync/reconnect)
│   ├── tasks/                  # Background tasks (dream/remote agent/in-process teammate)
│   ├── adversarial_debate.rs   # Adversarial debate
│   ├── agent_orchestrator.rs   # Multi-agent orchestration
│   ├── agent_roles.rs          # Agent roles
│   ├── webhook_dispatcher.rs   # Webhook dispatching
│   ├── webhook_server.rs       # Webhook server
│   ├── session_search.rs       # Session search
│   ├── dashboard_plugin.rs     # Dashboard plugins
│   ├── dashboard_registry.rs   # Dashboard registry
│   ├── permissions.rs          # Permission management
│   ├── permission_enforcer.rs  # Permission enforcement
│   ├── policy_engine.rs        # Policy engine
│   ├── trust_resolver.rs       # Trust resolution
│   ├── resource_governor.rs    # Resource governor
│   ├── green_contract.rs       # Green contract
│   ├── feature_flags.rs        # Feature flags
│   ├── module_switch.rs        # Module switch
│   ├── mode_selector.rs        # Mode selector
│   ├── config.rs               # Runtime config
│   ├── config_validate.rs      # Config validation
│   ├── prompt.rs               # Prompt management
│   ├── prompt_cache.rs         # Prompt cache
│   ├── compact.rs              # Context compression
│   ├── summary_compression.rs  # Summary compression
│   ├── compact_thresholds.rs   # Compression thresholds
│   ├── compact_warning.rs      # Compression warning
│   ├── reactive_compact.rs     # Reactive compression
│   ├── session_memory_compact.rs # Session memory compression
│   ├── message_importance.rs   # Message importance assessment
│   ├── message_batching.rs     # Message batching
│   ├── rate_limiter.rs         # Rate limiter
│   ├── connection_pool.rs      # Connection pool
│   ├── persistent_queue.rs     # Persistent queue
│   ├── persistent_queue_manager.rs # Queue manager
│   ├── health_check.rs         # Health check
│   ├── cache_guard.rs          # Cache guard
│   ├── checkpoint.rs           # Checkpoint
│   ├── branch_lock.rs          # Branch lock
│   ├── stale_base.rs           # Stale base detection
│   ├── watch_patterns.rs       # Watch patterns
│   ├── lan_transfer.rs         # LAN transfer
│   ├── tls_config.rs           # TLS config
│   ├── sse.rs                  # SSE event stream
│   ├── api_server.rs           # API server
│   ├── gateway_auth.rs         # Gateway auth
│   ├── gateway_metrics.rs      # Gateway metrics
│   ├── bash.rs                 # Bash execution
│   ├── bash_validation.rs      # Bash validation
│   ├── shell_hooks.rs          # Shell hooks
│   ├── shell_completer.rs      # Shell completion
│   ├── terminal_analyzer.rs    # Terminal analyzer
│   ├── git_context.rs          # Git context
│   ├── git_tools.rs            # Git tools
│   ├── file_ops.rs             # File operations
│   ├── hooks.rs                # Hook management
│   ├── hook_chain.rs           # Hook chain
│   ├── hook_config.rs          # Hook config
│   ├── plugin_hooks.rs         # Plugin hooks
│   ├── plugin_lifecycle.rs     # Plugin lifecycle
│   ├── profile.rs              # Profile
│   ├── profile_manager.rs      # Profile manager
│   ├── oauth.rs                # OAuth authentication
│   ├── usage.rs                # Usage statistics
│   ├── bootstrap.rs            # Bootstrap
│   ├── worker_boot.rs          # Worker boot
│   ├── fork_bridge.rs          # Fork bridge
│   ├── task_packet.rs          # Task packet
│   ├── task_router.rs          # Task router
│   ├── task_registry.rs        # Task registry
│   ├── transform_pipeline.rs   # Transform pipeline
│   ├── transport_handlers.rs   # Transport handlers
│   ├── general_engine.rs       # General engine
│   ├── engine_bridge.rs        # Engine bridge
│   ├── conversation.rs         # Conversation management
│   ├── session_control.rs      # Session control
│   ├── shared_memory.rs        # Shared memory
│   ├── validation_executor.rs  # Validation executor
│   ├── recovery_recipes.rs     # Recovery recipes
│   ├── error_recovery.rs       # Error recovery
│   ├── theme_engine.rs         # Theme engine
│   ├── token_budget_predictor.rs # Token budget prediction
│   ├── team_cron_registry.rs   # Team cron registry
│   ├── module_dream.rs         # Dream module
│   ├── json.rs                 # JSON utilities
│   └── lane_events.rs          # Lane events
│
├── telemetry/     # Telemetry & tracing
│   ├── tracer.rs              # Distributed tracing
│   ├── metrics.rs             # Metrics collection
│   ├── span.rs                # Span management
│   ├── event.rs               # Event definitions
│   ├── collector.rs           # Data collection
│   ├── exporter.rs            # Data export
│   └── storage.rs             # Storage backend
│
├── tools/         # Tool system
│   ├── registry.rs             # Tool registry
│   ├── builtin_tools.rs        # Built-in tool definitions
│   ├── builtin_handlers.rs     # Built-in tool handlers
│   ├── orchestration.rs        # Tool orchestration
│   ├── streaming.rs            # Streaming output
│   ├── stats.rs                # Usage statistics
│   ├── recorder.rs             # Execution recording
│   ├── agent_def_loader.rs     # Agent definition loader
│   ├── agent_def_types.rs      # Agent definition types
│   ├── bash/                   # Bash tool (parser/sandbox/security/path validation)
│   ├── hooks/                  # Hooks (registry/executor)
│   ├── mcp/                    # MCP tools (registry/OAuth/wrapper)
│   ├── permissions/            # Permissions (classifier/rules/tracker)
│   └── tools/                  # Concrete tool implementations
│       ├── agent.rs            # Agent tool
│       ├── bash.rs             # Bash execution
│       ├── context.rs          # Context management
│       ├── cron.rs             # Cron scheduling
│       ├── glob.rs             # File globbing
│       ├── grep.rs             # Content search
│       ├── lsp.rs              # LSP tool
│       ├── monitor.rs          # Monitor tool
│       ├── plan.rs             # Plan tool
│       ├── repl.rs             # REPL tool
│       ├── skill.rs            # Skill tool
│       ├── web_fetch.rs        # Web fetch
│       ├── web_search.rs       # Web search
│       ├── file_read.rs        # File read
│       ├── file_write.rs       # File write
│       ├── file_edit.rs        # File edit
│       ├── computer_use.rs     # Computer control
│       ├── messaging.rs        # Message sending
│       ├── push_notification.rs # Push notification
│       ├── task_system.rs      # Task system
│       ├── todo_write.rs       # Todo items
│       └── batch_missing.rs    # Batch missing detection
│
├── trajectory/    # Learning system
│   ├── memory.rs              # Memory management
│   ├── memory_provider.rs     # Memory provider interface
│   ├── auto_memory.rs         # Auto memory extraction
│   ├── skill.rs               # Skill system
│   ├── skill_manager.rs       # Skill manager
│   ├── skill_evolution.rs     # Skill evolution
│   ├── skill_matcher.rs       # Skill matching
│   ├── skill_proposal.rs      # Skill proposal
│   ├── skills_hub_adapter.rs  # Skills hub adapter
│   ├── skills_hub_client.rs   # Skills hub client
│   ├── skill_decomposition/   # Skill decomposition (LLM-assisted/multi-turn/workflow validation/tool parsing)
│   ├── rl.rs                  # RL reward signals
│   ├── rl_trainer.rs          # RL trainer
│   ├── training_env.rs        # Training environment
│   ├── behavior_learner.rs    # Behavior learning
│   ├── behavior_tracker.rs    # Behavior tracking
│   ├── pattern.rs             # Pattern recognition
│   ├── pattern_analyzer.rs    # Pattern analysis
│   ├── user_profile.rs        # User profiling
│   ├── preference_learner.rs  # Preference learning
│   ├── adaptation.rs          # Adaptation
│   ├── dream_consolidation.rs # Dream consolidation
│   ├── parallel_execution.rs  # Parallel execution service
│   ├── style_extractor.rs     # Style extraction
│   ├── style_applier.rs       # Style application
│   ├── style_vectorizer.rs    # Style vectorization
│   ├── style_migrator.rs      # Style migration
│   ├── suggestion_engine.rs   # Suggestion engine
│   ├── proactive_assistant.rs # Proactive assistant
│   ├── context_predictor.rs   # Context prediction
│   ├── task_prefetcher.rs     # Task prefetching
│   ├── reminder_manager.rs    # Reminder management
│   ├── nudge.rs               # Nudge system
│   ├── insight.rs             # Insight generation
│   ├── compactor.rs           # Data compaction
│   ├── trajectory.rs          # Trajectory management
│   ├── trajectory_compressor.rs # Trajectory compression
│   ├── sub_agent.rs           # Sub-agent
│   ├── batch.rs               # Batch processing
│   ├── context.rs             # Context management
│   ├── fts5.rs                # FTS5 search
│   ├── hooks.rs               # Hooks
│   ├── storage.rs             # Storage
│   ├── scheduled_task.rs      # Scheduled task
│   └── memory_providers/      # Memory providers (Honcho/Mem0/Closed-loop/Service)
│
└── migration/     # Database migration
    └── m20240101_000001~000010  # 10 migration files
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

# Code formatting
npm run format

# CI check
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
│   ├── stores/                  # Zustand state management
│   │   ├── domain/            # Core business state (6 stores)
│   │   ├── feature/           # Feature module state (30+ stores)
│   │   ├── devtools/          # DevTools state (5 stores)
│   │   └── shared/            # Shared state (4 stores)
│   ├── hooks/                   # React hooks (10)
│   ├── lib/                     # Utility functions (with Web Worker)
│   ├── types/                   # TypeScript definitions (22)
│   ├── sdk/                     # SDK (including Python SDK)
│   └── i18n/                    # 11 language translations
│
├── src-tauri/                    # Backend source (Rust)
│   ├── crates/                  # Rust workspace (10 crates)
│   │   ├── agent/             # AI Agent core
│   │   ├── core/              # Database, crypto, RAG
│   │   ├── gateway/           # API gateway server
│   │   ├── plugins/           # Plugin system
│   │   ├── providers/         # Model provider adapters
│   │   ├── runtime/           # Runtime services
│   │   ├── tools/             # Tool system
│   │   ├── trajectory/        # Memory & learning
│   │   ├── telemetry/         # Tracing & metrics
│   │   └── migration/         # Database migrations
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
