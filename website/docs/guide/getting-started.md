# Getting Started

## Installation

Download the latest installer from the [Download page](/download) or the [GitHub Releases](https://github.com/polite0803/AxAgent/releases).

### macOS

| Chip | File |
|------|------|
| Apple Silicon (M1 / M2 / M3 / M4) | `AxAgent_x.x.x_aarch64.dmg` |
| Intel | `AxAgent_x.x.x_x64.dmg` |

1. Open the `.dmg` and drag **AxAgent** into the **Applications** folder.
2. Launch AxAgent. If macOS blocks the app, go to **System Settings → Privacy & Security** and click **Open Anyway**.

::: warning macOS: "App Is Damaged" or "Cannot Verify Developer"
If you see either of these messages, open Terminal and run:

```bash
xattr -c /Applications/AxAgent.app
```

Then launch the app again. This removes the quarantine flag that macOS applies to unsigned downloads.
:::

### Windows

| Architecture | File |
|--------------|------|
| x64 (most PCs) | `AxAgent_x.x.x_x64-setup.exe` |
| ARM64 | `AxAgent_x.x.x_arm64-setup.exe` |

Run the installer and follow the wizard. Launch AxAgent from the Start Menu or the desktop shortcut.

### Linux

| Format | Architecture | File |
|--------|--------------|------|
| Debian / Ubuntu | x64 | `AxAgent_x.x.x_amd64.deb` |
| Debian / Ubuntu | ARM64 | `AxAgent_x.x.x_arm64.deb` |
| Fedora / openSUSE | x64 | `AxAgent_x.x.x_x86_64.rpm` |
| Fedora / openSUSE | ARM64 | `AxAgent_x.x.x_aarch64.rpm` |
| Any distro | x64 | `AxAgent_x.x.x_amd64.AppImage` |
| Any distro | ARM64 | `AxAgent_x.x.x_aarch64.AppImage` |

```bash
# Debian / Ubuntu
sudo dpkg -i AxAgent_x.x.x_amd64.deb

# Fedora / openSUSE
sudo rpm -i AxAgent_x.x.x_x86_64.rpm

# AppImage (any distro)
chmod +x AxAgent_x.x.x_amd64.AppImage
./AxAgent_x.x.x_amd64.AppImage
```

---

## First-Time Setup

### 1. Open Settings

Launch AxAgent and click the **gear icon** at the bottom of the sidebar, or press <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd>.

### 2. Add a Provider

Navigate to **Settings → Providers** and click the **+** button.

1. Enter a display name (e.g. *OpenAI*).
2. Select the provider type (OpenAI, Anthropic, Google Gemini, etc.).
3. Paste your API key.
4. Confirm the **Base URL** — the official endpoint is pre-filled for built-in types. Change it only if you use a third-party relay or proxy.

::: tip
You can add as many providers as you like. Each provider manages its own set of API keys and models independently.
:::

### 3. Fetch Models

Click **Fetch Models** to pull the list of available models from the provider's API. You can also add model IDs manually if needed.

### 4. Set a Default Model

Go to **Settings → Default Model** and choose the provider and model that new conversations should use by default.

---

## Your First Conversation

1. Click **New Chat** in the sidebar (or press <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd>).
2. Select a model from the model selector at the top of the chat.
3. Type a message and press <kbd>Enter</kbd>.
4. AxAgent streams the response in real time. Models that support thinking blocks (e.g. Claude, DeepSeek R1) display the reasoning process in a collapsible section above the answer.

---

## Key Concepts

### Conversation Branching

Edit or regenerate any message to create a branch. The original and new versions coexist in the same conversation tree — navigate between them with the arrow controls on each message.

### Message Versions

Every regeneration creates a new version. Switch between versions using the **◀ ▶** arrows on the message bubble.

### Context Attachments

Enrich your prompts by attaching files, search results, or MCP tool outputs directly to the conversation context.

---

## Shortcuts

Below are the default keyboard shortcuts. All shortcuts can be customized in **Settings → Shortcuts**.

| Shortcut | Action |
|----------|--------|
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> | Show / hide the current window |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Alt</kbd>+<kbd>A</kbd> | Show / hide all windows |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>W</kbd> | Close window |
| <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd> | New conversation |
| <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd> | Open Settings |
| <kbd>Cmd/Ctrl</kbd>+<kbd>K</kbd> | Command palette |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> | Toggle model selector |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>↑</kbd> | Fill last message |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>K</kbd> | Clear context |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Backspace</kbd> | Clear conversation messages |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd> | Toggle API Gateway |

::: info
Global shortcuts (show/hide window) work system-wide even when AxAgent is in the background. AxAgent detects conflicts with shortcuts from other applications and warns you in Settings.
:::

---

## Desktop Settings

Open **Settings** to configure the desktop experience.

| Setting | Options |
|---------|---------|
| **Theme** | Dark, Light, or System (auto) |
| **Language** | English, 简体中文 |
| **System tray** | Minimize to tray, close to tray |
| **Auto-start** | Launch AxAgent when your computer starts |
| **Always on top** | Keep the window above other applications |
| **Proxy** | HTTP or SOCKS5 proxy with host and port |

---

## Data & Backup

### Data Directories

AxAgent stores data in two locations:

| Path | Contents |
|------|----------|
| `~/.axagent/` | Application state — database, encryption keys, vector DB, SSL certificates |
| `~/Documents/axagent/` | User-visible files — images, documents, backups |

::: tip
On Windows the paths are `%USERPROFILE%\.axagent\` and `%USERPROFILE%\Documents\axagent\`.
:::

### Auto Backup

Go to **Settings → Backup** to configure automatic backups:

- **Enabled** — toggle automatic backups on or off.
- **Interval** — how often to back up (in hours).
- **Max count** — number of backups to keep before the oldest is deleted.
- **Storage target** — local directory (default: `~/Documents/axagent/backups/`) or WebDAV server.

### Manual Backup & Restore

Create a manual backup at any time from the Backup settings panel. To restore, select a backup from the history list and click **Restore**.

### Conversation Export

Right-click a conversation in the sidebar to export it as:

- **PNG** — rendered screenshot of the chat
- **Markdown** — formatted with headers, code blocks, and LaTeX
- **Plain text** — simple text with message separators
- **JSON** — structured data with full metadata

---

## Next Steps

- [Configure Providers](./providers) — add and manage AI providers
- [MCP Servers](./mcp) — connect external tools to extend AI capabilities
- [API Gateway](./gateway) — expose your providers as a local API server
