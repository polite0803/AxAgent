# API Gateway

## What is the API Gateway?

AxAgent includes a built-in local API server that exposes your configured providers as **OpenAI-compatible**, **Claude-native**, and **Gemini-native** endpoints. Any tool or client that speaks one of these protocols can use AxAgent as its backend — no separate API keys or relay services required.

Use cases:

- Run **Claude Code CLI**, **OpenAI Codex CLI**, **Gemini CLI**, or **OpenCode** through AxAgent.
- Feed your IDE extensions through a single, locally managed endpoint.
- Share one set of provider keys across many tools with per-key rate limiting.

---

## Getting Started

1. Open **Settings → API Gateway** (or press <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>).
2. Click **Start** to launch the gateway server.
3. By default the server listens on `127.1.0.0:8080` (HTTP). You can change the listen address and port in the **Settings** tab.

::: tip
Enable **Auto-start** in the gateway settings to launch the server automatically when AxAgent starts.
:::

---

## API Key Management

The gateway authenticates incoming requests with its own API keys, separate from your provider keys.

1. Go to the **API Keys** tab.
2. Click **Generate New Key**.
3. Optionally add a **description** (e.g. *Claude Code*, *VS Code*) to identify each key.
4. Copy the key — it is only displayed once.

Each key shows its **prefix**, **creation date**, and **last used** timestamp. You can **enable/disable** or **delete** keys at any time.

::: warning
Treat gateway keys like any other API key. Anyone with the key can send requests to your local gateway while it is running.
:::

---

## SSL/TLS

The gateway can serve HTTPS alongside or instead of HTTP.

### Auto-Generated Certificate

1. Open the **Settings** tab.
2. Enable **SSL/TLS** and select **Generate** mode.
3. AxAgent creates a self-signed certificate and private key at:

   ```
   ~/.axagent/ssl/cert.pem
   ~/.axagent/ssl/key.pem
   ```

4. Set the **HTTPS port** (default `8443`).
5. Optionally enable **Force SSL** to redirect all HTTP requests to HTTPS.

### Custom Certificate

Select **Upload** mode and provide the paths to your own certificate and private key files.

::: info
The private key is stored with file mode `0600` on Unix systems to prevent other users from reading it.
:::

---

## Usage Analytics

The **Metrics** tab provides dashboards for monitoring gateway activity:

| View | Description |
|------|-------------|
| **By API Key** | Request volume and token usage broken down by each gateway key |
| **By Provider** | Requests and tokens routed to each upstream AI provider |
| **By Day** | Daily usage trends over time |

Use these dashboards to track cost, spot anomalies, and balance load across providers.

---

## Request Logs

The **Overview** tab shows recent request logs with the following details:

- **Timestamp**, **HTTP method**, **path**
- **Status code** and **response time**
- **Provider** and **model** used
- **Token count** (prompt + completion)

Click any log entry to inspect the full request and response. Use the **Clear Logs** button to reset the log history.

---

## Configuration Templates

AxAgent ships with ready-made configuration snippets for popular CLI tools. Go to the **Templates** tab, pick a tool, and click **Copy** to get the configuration you need.

### Claude Code CLI

```bash
claude config set --global apiUrl http://127.1.0.0:8080
claude config set --global apiKey axagent-xxxx
```

### OpenAI Codex CLI

```bash
export OPENAI_BASE_URL=http://127.1.0.0:8080/v1
export OPENAI_API_KEY=axagent-xxxx
codex
```

### OpenCode

Add to your OpenCode configuration:

```json
{
  "provider": "openai",
  "baseURL": "http://127.1.0.0:8080/v1",
  "apiKey": "axagent-xxxx"
}
```

### Gemini CLI

```bash
export GEMINI_API_BASE=http://127.1.0.0:8080
export GEMINI_API_KEY=axagent-xxxx
gemini
```

### Custom Client

Any tool that accepts an OpenAI-compatible base URL can connect to the gateway:

```
Base URL:  http://127.1.0.0:8080/v1
API Key:   axagent-xxxx
```

Replace `axagent-xxxx` with a key generated in the **API Keys** tab. If SSL is enabled, use `https://` and port `8443` (or your configured HTTPS port).

---

## Connecting External Tools

Below is a general pattern for connecting any OpenAI-compatible client:

1. **Generate a gateway API key** in AxAgent.
2. **Set the base URL** in your tool's configuration to `http://127.1.0.0:8080/v1` (or the HTTPS equivalent).
3. **Set the API key** to the gateway key you generated.
4. **Select a model** that you have configured in one of your AxAgent providers.
5. Start using the tool — all requests route through AxAgent to the upstream provider.

::: tip
If a tool requires a specific API format (e.g. Claude or Gemini native), AxAgent automatically detects the request format and routes it to the correct provider. You do not need to configure separate endpoints per format.
:::

---

## Next Steps

- [Getting Started](./getting-started) — return to the quick start guide
- [Configure Providers](./providers) — add the upstream providers the gateway routes to
- [MCP Servers](./mcp) — connect external tools for AI tool calling
