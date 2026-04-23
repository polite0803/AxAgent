# Configure Providers

AxAgent connects to any number of AI providers simultaneously. Each provider has its own API keys, model list, and parameter defaults.

## Supported Providers

AxAgent includes first-class support for the following providers. Any service that exposes an OpenAI-compatible API also works out of the box.

| Provider | Example Models |
|----------|---------------|
| **OpenAI** | GPT-4o, GPT-4, o3, o4-mini |
| **Anthropic** | Claude 4 Sonnet, Claude 4 Opus, Claude 3.5 Sonnet |
| **Google** | Gemini 2.5 Pro, Gemini 2.5 Flash, Gemini 2.0 |
| **DeepSeek** | DeepSeek V3, DeepSeek R1 |
| **Alibaba Cloud** | Qwen series |
| **Zhipu AI** | GLM series |
| **xAI** | Grok series |
| **Any OpenAI-compatible API** | Ollama, vLLM, LiteLLM, third-party relays, etc. |

---

## Adding a Provider

1. Go to **Settings → Providers**.
2. Click the **+** button at the bottom left.
3. Fill in the provider details:

| Field | Description |
|-------|-------------|
| **Name** | A display name for the sidebar (e.g. *OpenAI*) |
| **Type** | Provider type — determines the default base URL and API behavior |
| **Icon** | Optional icon for visual identification |
| **API Key** | The secret key from your provider's dashboard |
| **Base URL** | API endpoint (pre-filled for built-in types, changeable for relays) |
| **API Path** | Request path — defaults to `/v1/chat/completions` |

::: tip
For third-party relay services, keep the type set to **OpenAI** (or the matching upstream type) and change the **Base URL** to the relay's endpoint.
:::

---

## Multi-Key Rotation

AxAgent supports multiple API keys per provider for load distribution and rate-limit avoidance.

### Adding Keys

In the provider detail panel, click **Add Key** to add additional API keys. Each key shows its prefix, creation date, and last-used timestamp.

### How Rotation Works

AxAgent rotates through enabled keys automatically using a round-robin index. When a request completes, the rotation index advances to the next key. If a key is disabled or fails validation, it is skipped.

### Validating Keys

Click the **validate** button next to a key to verify it against the provider's API. The validation result and any error message are recorded for reference.

---

## Model Management

### Fetching Models

Click **Fetch Models** in the provider detail panel to pull the full list of available models from the provider's API. The discovered models are added to your local list automatically.

### Adding Models Manually

If a model is not returned by the API (e.g. a fine-tuned model or a new release), you can add it by typing the model ID directly.

### Per-Model Parameters

Each model can have its own default parameter overrides. Open the model's settings to configure:

| Parameter | Description |
|-----------|-------------|
| **Temperature** | Controls randomness (0 = deterministic, higher = more creative) |
| **Max Tokens** | Maximum number of tokens in the response |
| **Top P** | Nucleus sampling threshold |
| **Frequency Penalty** | Reduces repetition of token sequences |
| **Presence Penalty** | Encourages the model to introduce new topics |

### Model Capabilities

AxAgent tracks capabilities per model — such as **Vision**, **Function Calling**, and **Reasoning** — and displays them as tags. These capabilities affect which features are available during a conversation.

---

## Custom & Local Endpoints

AxAgent works with any endpoint that implements the OpenAI chat completions API.

### Ollama (Local Models)

1. Install and start [Ollama](https://ollama.com/).
2. In AxAgent, create a new provider with type **OpenAI**.
3. Set the **Base URL** to `http://localhost:11434`.
4. Click **Fetch Models** to discover the models you have pulled locally.

### vLLM / TGI

Point the **Base URL** to the address of your inference server (e.g. `http://localhost:8000`) and fetch or add models as usual.

### API Relay Services

For relay or aggregator services (e.g. OpenRouter, one-api), set the type to **OpenAI**, enter the relay's base URL, and provide the relay's API key.

---

## Default Model Settings

### Default Assistant Model

Go to **Settings → Default Model** to choose the provider and model that new conversations use by default. You can always override the model on a per-conversation basis from the model selector.

### Topic Naming Model

AxAgent can automatically generate a title for each conversation. In the default model settings, you can assign a separate, lightweight model for topic naming to save cost and latency. Configure a custom prompt and context window size for title generation.

---

## Next Steps

- [MCP Servers](./mcp) — connect external tools to extend AI capabilities
- [API Gateway](./gateway) — expose your providers as a local API server
