# 配置服务商

AxAgent 支持接入多种 AI 服务商，你可以同时配置多个服务商并在对话中自由切换。

## 支持的服务商

AxAgent 内置以下服务商类型，并支持任何兼容 OpenAI API 格式的自定义端点：

| 服务商 | 代表模型 | 说明 |
|-------|---------|------|
| **OpenAI** | GPT-4o、GPT-4、o3、o4-mini | 最广泛支持的 API 格式 |
| **Anthropic** | Claude 4 Sonnet/Opus、Claude 3.5 Sonnet | 原生 Claude API |
| **Google** | Gemini 2.5 Pro/Flash、Gemini 2.0 | Google AI Studio 或 Vertex |
| **DeepSeek** | DeepSeek-V3、DeepSeek-R1 | 高性价比推理模型 |
| **阿里云通义千问** | Qwen-Max、Qwen-Plus、Qwen-Turbo | 兼容 OpenAI 格式 |
| **智谱 GLM** | GLM-4、GLM-4-Flash | 国产大模型 |
| **xAI** | Grok-3、Grok-3-mini | xAI 出品 |
| **OpenAI 兼容** | 任意模型 | 适配所有兼容 OpenAI 格式的第三方服务 |

## 添加服务商

### 基本步骤

1. 进入 **设置 → 服务商**
2. 点击左下角的 **+** 按钮
3. 填写以下信息：
   - **名称** — 自定义名称，用于在界面中区分不同服务商
   - **类型** — 选择对应的服务商类型（如 OpenAI、Anthropic 等）
   - **图标** — 可选，为服务商选择一个显示图标
4. 点击确认创建

### 配置 API

创建服务商后，需要填写 API 连接信息：

#### API 密钥

填入从服务商处获取的 API Key。例如 OpenAI 的密钥格式为 `sk-...`。

::: tip 密钥安全
AxAgent 使用 AES-256 加密存储所有 API 密钥，密钥数据保存在本地 `~/.axagent/axagent.db` 中，不会上传到任何外部服务器。
:::

#### Base URL

API 的基础地址。各服务商的官方地址：

| 服务商 | 官方 Base URL |
|-------|--------------|
| OpenAI | `https://api.openai.com` |
| Anthropic | `https://api.anthropic.com` |
| Google | `https://generativelanguage.googleapis.com` |
| DeepSeek | `https://api.deepseek.com` |
| 阿里云通义 | `https://dashscope.aliyuncs.com/compatible-mode` |
| 智谱 GLM | `https://open.bigmodel.cn` |
| xAI | `https://api.x.ai` |

如果你使用第三方中转服务或自建代理，将 Base URL 替换为中转地址即可。

#### API 路径

API 请求的路径部分，默认为 `/v1/chat/completions`。一般情况下无需修改，除非服务商使用了非标准路径。

## 多密钥轮询

AxAgent 支持为同一个服务商配置多个 API 密钥，实现自动轮换：

### 添加多个密钥

在服务商配置页面的 API 密钥区域，可以添加多个密钥。所有密钥共享同一个 Base URL 和配置。

### 自动轮换机制

当配置了多个密钥时，AxAgent 会在每次请求时自动轮换使用不同的密钥。

### 限流分散

多密钥轮询的一个重要用途是分散 API 限流。每个密钥有独立的速率配额，使用多个密钥可以有效提高整体吞吐量，降低单个密钥触发限流的概率。

::: tip 适用场景
如果你有团队共享的多个 API 密钥，或者单个密钥的速率限制无法满足需求，多密钥轮询是一个简单高效的解决方案。
:::

## 模型管理

### 远程拉取模型列表

点击 **获取模型** 按钮，AxAgent 会调用服务商的模型列表 API，自动拉取当前可用的全部模型。拉取后的模型会显示在列表中供你选择。

### 手动添加模型

如果服务商的模型列表 API 不完整或你需要添加特定模型，可以手动输入模型 ID 进行添加。例如：

- `gpt-4o`
- `claude-sonnet-4-20250514`
- `gemini-2.5-pro`

### 模型参数

每个模型可以独立配置以下默认参数：

| 参数 | 说明 | 典型范围 |
|-----|------|---------|
| **温度 (Temperature)** | 控制输出的随机性，值越高越有创意 | 0 – 2 |
| **最大 Token (Max Tokens)** | 限制单次回复的最大长度 | 1 – 模型上限 |
| **Top-P** | 核采样概率阈值，与温度配合使用 | 0 – 1 |
| **频率惩罚 (Frequency Penalty)** | 降低已出现词汇的重复概率 | -2 – 2 |
| **存在惩罚 (Presence Penalty)** | 鼓励模型讨论新话题 | -2 – 2 |

::: info 参数建议
对于日常对话，保持默认参数即可。如果需要更有创意的输出，可以适当提高温度；需要精确回答时，降低温度到 0–0.3。
:::

## 自定义/本地端点

AxAgent 支持连接任何兼容 OpenAI API 格式的服务，以下是常见场景的配置示例。

### Ollama

[Ollama](https://ollama.ai/) 可以在本地运行开源大模型。配置方法：

1. 添加服务商，类型选择 **OpenAI**
2. Base URL 填写 `http://localhost:11434`
3. API Key 留空或填写任意值
4. 点击 **获取模型** 拉取本地已下载的模型

::: warning 注意
请确保 Ollama 服务已启动。部分 Ollama 版本可能需要设置 `OLLAMA_ORIGINS=*` 环境变量以允许跨域请求。
:::

### vLLM / TGI

如果你使用 vLLM 或 Text Generation Inference 部署了自己的模型：

1. 添加服务商，类型选择 **OpenAI**
2. Base URL 填写你的部署地址，如 `http://your-server:8000`
3. API Key 根据你的服务配置填写
4. 手动添加你部署的模型 ID

### API 中转服务

很多第三方提供 OpenAI 兼容的 API 中转服务，配置方法相同：

1. 添加服务商，类型选择 **OpenAI**
2. Base URL 填写中转服务提供的地址
3. API Key 填写中转服务提供的密钥
4. 获取或手动添加模型

## 默认模型设置

AxAgent 支持为不同用途设置默认模型，在 **设置 → 默认模型** 中配置：

### 全局默认助手模型

新建对话时自动使用的模型。建议选择你最常用的模型，例如 `gpt-4o` 或 `claude-sonnet-4-20250514`。

### 默认话题命名模型

用于自动为对话生成标题的模型。这个模型不需要很强大，选择一个响应快且便宜的模型即可，如 `gpt-4o-mini` 或 `gemini-2.0-flash`。

::: tip
默认模型设置是可选的。如果不设置，每次新建对话时需要手动选择模型。
:::
