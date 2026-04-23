[简体中文](./README.md) | [繁體中文](./README-ZH-TW.md) | **English** | [日本語](./README-JA.md) | [한국어](./README-KO.md) | [Français](./README-FR.md) | [Deutsch](./README-DE.md) | [Español](./README-ES.md) | [Русский](./README-RU.md) | [हिन्दी](./README-HI.md) | [العربية](./README-AR.md)

[![AxAgent](https://github.com/polite0803/AxAgent/blob/main/src/assets/image/logo.png?raw=true)](https://github.com/polite0803/AxAgent)

<p align="center">
    <a href="https://www.producthunt.com/products/axagent?embed=true&amp;utm_source=badge-featured&amp;utm_medium=badge&amp;utm_campaign=badge-axagent" target="_blank" rel="noopener noreferrer"><img alt="AxAgent - Lightweight, high-perf cross-platform AI desktop client | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1118403&amp;theme=light&amp;t=1775627359538"></a>
</p>

## Screenshots

| Chat Chart Rendering | Providers & Models |
|:---:|:---:|
| ![](.github/images/s1-0412.png) | ![](.github/images/s2-0412.png) |

| Knowledge Base | Memory |
|:---:|:---:|
| ![](.github/images/s3-0412.png) | ![](.github/images/s4-0412.png) |

| Agent - Ask User | API Gateway One-Click Access |
|:---:|:---:|
| ![](.github/images/s5-0412.png) | ![](.github/images/s6-0412.png) |

| Chat Model Selection | Chat Navigation |
|:---:|:---:|
| ![](.github/images/s7-0412.png) | ![](.github/images/s8-0412.png) |

| Agent - Permission Approval | API Gateway Overview |
|:---:|:---:|
| ![](.github/images/s9-0412.png) | ![](.github/images/s10-0412.png) |

## Features

### Conversation & Models

- **Multi-Provider Support** — Compatible with OpenAI, Anthropic Claude, Google Gemini, and all OpenAI-compatible APIs; also supports Ollama for local models, OpenClaw/Hermes for remote gateway connections
- **Model Management** — Fetch remote model lists, customize parameters (temperature, max tokens, top-p, etc.)
- **Multi-Key Rotation** — Configure multiple API keys per provider with automatic rotation to distribute rate limit pressure
- **Streaming Output** — Real-time token-by-token rendering with collapsible thinking blocks
- **Message Versions** — Switch between multiple response versions per message to compare model or parameter effects
- **Conversation Branching** — Fork new branches from any message node, with side-by-side branch comparison
- **Conversation Management** — Pin, archive, time-grouped display, and bulk operations
- **Conversation Compression** — Automatically compress lengthy conversations, preserving key information to save context space
- **Multi-Model Simultaneous Response** — Ask the same question to multiple models at once, with side-by-side comparison of answers
- **Category System** — Custom conversation categories with topic-based organization

### AI Agent

- **Agent Mode** — Switch to Agent mode for autonomous multi-step task execution: read/write files, run commands, analyze code, and more
- **Three Permission Levels** — Default (writes need approval), Accept Edits (auto-approve file changes), Full Access (no prompts) — safe and controllable
- **Working Directory Sandbox** — Agent operations are strictly confined to the specified working directory, preventing unauthorized access
- **Tool Approval Panel** — Real-time display of tool call requests with per-tool review, one-click "always allow", or deny
- **Cost Tracking** — Real-time token usage and cost statistics per session
- **Pause/Resume** — Pause agent tasks at any time for review before resuming
- **Bash Command Execution** — Execute shell commands in sandboxed environment with automatic risk validation

### Multi-Agent System

- **Sub-Agent Coordination** — Create multiple sub-agents with master-slave coordination architecture
- **Parallel Execution** — Process multiple agents in parallel for improved efficiency on complex tasks
- **Adversarial Debate** — Multiple agents debate different viewpoints to produce better solutions through collision of ideas
- **Workflow Engine** — Powerful workflow orchestration supporting conditional branches, loops, and parallel execution
- **Team Roles** — Assign specific roles to different agents (code review, testing, documentation, etc.) for collaborative task completion

### Skill System

- **Skill Marketplace** — Built-in marketplace to browse and install community-contributed skills
- **Skill Creation** — Auto-create skills from proposals with Markdown editor support
- **Skill Evolution** — AI automatically analyzes and improves existing skills for better execution
- **Skill Matching** — Intelligent recommendations to automatically apply relevant skills to appropriate conversation scenarios
- **Local Skill Registry** — Register custom local tools as skills for reuse
- **Plugin Hooks** — Support pre/post hooks to inject custom logic before and after skill execution

### Content Rendering

- **Markdown Rendering** — Full support for code highlighting, LaTeX math formulas, tables, and task lists
- **Monaco Code Editor** — Embedded Monaco Editor in code blocks with syntax highlighting, copy, and diff preview
- **Diagram Rendering** — Built-in Mermaid flowchart and D2 architecture diagram rendering
- **Artifact Panel** — Code snippets, HTML drafts, Markdown notes, and reports viewable in a dedicated panel
- **Session Inspector** — Real-time display of session structure as a tree view for quick navigation to any message
- **Code Block Header Actions** — Code blocks support preview, copy, and other operations
- **Mermaid Chart Controls** — Support for zoom, mode switching, and other operations

### Search & Knowledge

- **Web Search** — Integrated with Tavily, Zhipu WebSearch, Bocha, and more, with citation source annotations
- **Local Knowledge Base (RAG)** — Supports multiple knowledge bases; upload documents for automatic parsing, chunking, and vector indexing, with semantic retrieval during conversations
- **Knowledge Graph** — Knowledge entity relationship graphs visualizing connections between knowledge points
- **Memory System** — Multi-namespace memory with manual entry or AI-powered automatic key information extraction
- **Full-Text Search** — FTS5 engine for fast retrieval across conversations, files, and memories
- **Context Management** — Flexibly attach file attachments, search results, knowledge base passages, memory entries, and tool outputs

### Tools & Extensions

- **MCP Protocol** — Full Model Context Protocol implementation supporting both stdio and HTTP/WebSocket transports
- **OAuth Authentication** — OAuth authentication flow support for MCP servers
- **Built-in Tools** — Ready-to-use built-in tools for file operations, code execution, search, and more
- **Tool Execution Panel** — Visual display of tool call requests and return results
- **LSP Client** — Built-in LSP protocol support for intelligent code completion and diagnostics

### API Gateway

- **Local API Gateway** — Built-in local API server with native support for OpenAI-compatible, Claude, and Gemini interfaces
- **External Links** — One-click integration with external tools like Claude CLI and OpenCode with automatic API key sync
- **API Key Management** — Generate, revoke, and enable/disable access keys with description notes
- **Usage Analytics** — Request volume and token usage analysis by key, provider, and date
- **Diagnostic Tools** — Gateway health checks, connection testing, and request debugging
- **SSL/TLS Support** — Built-in self-signed certificate generation, with support for custom certificates
- **Request Logs** — Complete recording of all API requests and responses passing through the gateway
- **Configuration Templates** — Pre-built integration templates for popular CLI tools such as Claude, Codex, OpenCode, and Gemini
- **Real-Time Communication** — WebSocket real-time event push, compatible with OpenAI Realtime API

### Data & Security

- **AES-256 Encryption** — API keys and sensitive data encrypted locally with AES-256-GCM
- **Isolated Data Directories** — Application state in `~/.axagent/`; user files in `~/Documents/axagent/`
- **Auto Backup** — Scheduled automatic backups to local directories or WebDAV storage
- **Backup Restore** — One-click restore from historical backups
- **Conversation Export** — Export conversations as PNG screenshots, Markdown, plain text, or JSON
- **Storage Space Management** — Visual display of disk usage with cleanup of unnecessary files

### Desktop Experience

- **Theme Switching** — Dark/light themes that follow the system preference or can be set manually
- **Interface Language** — Full support for Simplified Chinese, Traditional Chinese, English, Japanese, Korean, French, German, Spanish, Russian, Hindi, and Arabic
- **System Tray** — Minimize to system tray on window close without interrupting background services
- **Always on Top** — Pin the main window to stay above all other windows
- **Global Shortcuts** — Customizable global keyboard shortcuts to summon the main window at any time
- **Auto Start** — Optional launch on system startup
- **Proxy Support** — HTTP and SOCKS5 proxy configuration
- **Auto Update** — Automatically checks for new versions on startup and prompts for update
- **Command Palette** — `Cmd/Ctrl+K` for quick access to all commands and settings

## Core Functionality Modules

### Conversation System
- **Message Management**: Support for multiple versions, branching, and compression
- **Model Selection**: Multi-provider support with customizable parameters
- **Rendering System**: Markdown, code, and diagram rendering
- **Context Management**: Flexible mounting of various context sources

### Agent System
- **Single Agent**: Tool calling, file operations, command execution
- **Multi-Agent**: Collaboration, parallel execution, adversarial debate
- **Workflow**: Conditional branching, loops, and parallel execution

### Knowledge System
- **Knowledge Base**: Document upload, parsing, indexing, and retrieval
- **Knowledge Graph**: Entity relationship visualization
- **Memory**: Multi-namespace memory management
- **Search**: Web search and local full-text search

### API Gateway
- **Local Server**: OpenAI-compatible interface
- **External Links**: Integration with third-party tools
- **Key Management**: Generation, revocation, and permission control
- **Usage Statistics**: Detailed usage analysis

### Skill System
- **Skill Marketplace**: Browse and install skills
- **Skill Creation**: Auto-creation from proposals
- **Skill Evolution**: AI-powered skill improvement
- **Skill Matching**: Intelligent recommendation of applicable skills

## Technical Features

1. **Cross-Platform**: Based on Tauri framework, supporting Windows, macOS, and Linux
2. **High Performance**: Rust backend provides excellent performance and security
3. **Secure and Reliable**: Local storage, AES-256 encryption, sandbox isolation
4. **Extensible**: MCP protocol support, plugin system, skill system
5. **User-Friendly**: Modern UI, multi-language support, global shortcuts
6. **Feature-Rich**: From basic conversation to advanced Agent collaboration

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | Tauri 2 + React 19 + TypeScript |
| UI | Ant Design 6 + TailwindCSS 4 |
| State Management | Zustand 5 |
| Internationalization | i18next + react-i18next |
| Backend | Rust + SeaORM + SQLite |
| Vector Database | sqlite-vec |
| Build | Vite + npm |
| Charts | Mermaid + D2 |
| Code Editor | Monaco Editor |

## Platform Support

| Platform | Architecture |
|----------|-------------|
| macOS | Apple Silicon (arm64), Intel (x86_64) |
| Windows 10/11 | x86_64, arm64 |
| Linux | x86_64 (AppImage/deb/rpm), arm64 (AppImage/deb/rpm) |

## Getting Started

Head to the [Releases](https://github.com/polite0803/AxAgent/releases) page and download the installer for your platform.

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.75+
- [npm](https://www.npmjs.com/) 10+
- Windows requires [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and [Rust MSVC targets](https://doc.rust-lang.org/cargo/reference/config.html#cfgtarget)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/polite0803/AxAgent.git
cd AxAgent

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build frontend only
npm run build

# Build desktop application
npm run tauri build
```

Build artifacts are located in `src-tauri/target/release/`.

### Testing

```bash
# Run unit tests
npm run test

# Run end-to-end tests
npm run test:e2e

# Type checking
npm run typecheck
```

## Project Structure

```
AxAgent/
├── src/                    # Frontend source code
│   ├── components/         # React components
│   │   ├── chat/          # Chat-related components
│   │   ├── common/        # Common components
│   │   ├── files/         # File management components
│   │   ├── gateway/       # API gateway components
│   │   ├── layout/        # Layout components
│   │   ├── link/          # Gateway link components
│   │   ├── settings/      # Settings components
│   │   └── shared/        # Shared components
│   ├── pages/             # Page components
│   ├── stores/            # Zustand state management
│   │   ├── domain/        # Core business state
│   │   ├── feature/       # Feature module state
│   │   └── shared/        # Shared state
│   ├── hooks/             # React Hooks
│   ├── lib/               # Utility functions
│   ├── types/             # TypeScript type definitions
│   └── i18n/              # Internationalization resources
│
├── src-tauri/             # Rust backend source code
│   ├── crates/            # Rust workspace crates
│   │   ├── agent/         # Agent module
│   │   ├── core/          # Core module
│   │   ├── gateway/       # API gateway
│   │   ├── migration/     # Database migration
│   │   ├── plugins/       # Plugin system
│   │   ├── providers/     # Model providers
│   │   ├── runtime/       # Runtime
│   │   ├── telemetry/     # Telemetry and statistics
│   │   └── trajectory/    # Trajectory management
│   └── src/               # Tauri main entry
│
├── scripts/               # Build scripts
├── e2e/                   # E2E tests
└── website/               # Documentation website
```

## Configuration & Data

### Directory Structure

```
~/.axagent/                    # Configuration directory
├── axagent.db                 # SQLite database
├── master.key                 # AES-256 master key
├── vector_db/                 # Vector database
└── ssl/                       # SSL certificates

~/Documents/axagent/           # Documents directory
├── images/                    # Image attachments
├── files/                     # File attachments
└── backups/                   # Backup files
```

## FAQ

### macOS: "App Is Damaged" or "Cannot Verify Developer"

Since the application is not signed by Apple, macOS may show one of the following prompts:

- "AxAgent" is damaged and can't be opened
- "AxAgent" can't be opened because Apple cannot check it for malicious software

**Steps to resolve:**

**1. Allow apps from "Anywhere"**

```bash
sudo spctl --master-disable
```

Then go to **System Settings → Privacy & Security → Security** and select **Anywhere**.

**2. Remove the quarantine attribute**

```bash
sudo xattr -dr com.apple.quarantine /Applications/AxAgent.app
```

> Tip: You can drag the app icon onto the terminal after typing `sudo xattr -dr com.apple.quarantine `.

**3. Additional step for macOS Ventura and later**

After completing the above steps, the first launch may still be blocked. Go to **System Settings → Privacy & Security**, then click **Open Anyway** in the Security section. This only needs to be done once.

## Community
- [LinuxDO](https://linux.do)

## License

This project is licensed under the [AGPL-3.0](LICENSE) License.