# AxAgent SDK

AxAgent 的 ACP 协议客户端 SDK，支持 TypeScript 和 Python。

## 安装

### TypeScript / JavaScript

```bash
npm install @axagent/sdk
```

### Python

```bash
pip install axagent-sdk
```

或者从源码安装：

```bash
cd src/sdk/python
pip install .
```

## 快速开始

### TypeScript

```typescript
import { AxAgentClient } from "@axagent/sdk";

const client = new AxAgentClient("http://localhost:9876");

// 创建会话
const session = await client.createSession({
  workDir: "/path/to/project",
  model: "deepseek-v4-pro",
});

// 发送 prompt
const result = await client.sendPrompt(
  session.sessionId,
  "分析项目结构"
);
console.log(result.content);

// 关闭会话
await client.closeSession(session.sessionId);
```

使用会话上下文管理器：

```typescript
import { AxAgentClient, AxAgentSession } from "@axagent/sdk";

const client = new AxAgentClient("http://localhost:9876");

await using (const session = new AxAgentSession(client, "/path/to/project")) {
  const result = await session.send("分析项目结构");
  console.log(result.content);
}
// 会话自动关闭
```

### Python

```python
from axagent_sdk import AxAgentClient

client = AxAgentClient("http://localhost:9876")

# 创建会话
session = client.create_session(work_dir="/path/to/project")

# 发送 prompt
result = client.send_prompt(session["sessionId"], "分析项目结构")
print(result["content"])

# 关闭会话
client.close_session(session["sessionId"])
```

使用会话上下文管理器：

```python
from axagent_sdk import AxAgentClient, AxAgentSession

client = AxAgentClient("http://localhost:9876")

with AxAgentSession(client, "/path/to/project") as session:
    result = session.send("分析项目结构")
    print(result["content"])
# 会话自动关闭
```

## API 参考

| 方法 | 说明 |
|------|------|
| `create_session(work_dir, ...)` | 创建新会话 |
| `get_session(session_id)` | 获取会话状态 |
| `list_sessions()` | 列出所有会话 |
| `send_prompt(session_id, prompt)` | 发送 prompt 并获取响应 |
| `interrupt(session_id)` | 中断执行 |
| `close_session(session_id)` | 关闭会话 |
| `register_hook(session_id, event, callback_url)` | 注册 hook 回调 |
| `health_check()` | 健康检查 |

## 依赖

- **Python SDK**: 仅使用 Python 标准库，无第三方依赖，要求 Python >= 3.8
- **TypeScript SDK**: 仅使用 Node.js 内置模块，无第三方依赖
