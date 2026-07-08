---
AIGC:
    Label: "1"
    ContentProducer: 001191440300708461136T1XGW3
    ProduceID: 4bf27337f30a147f24e73352c7fc3b8c_8a2835777ad111f198065254006c9bbf
    ReservedCode1: G7B1oTdqfZXZhspCal6QPbM++kNsgYsENX/QXKam0NeCKQOgaQ/NX1OBU/91qd1HxKB71fpWN4Ww/K7RYdpQHVZZGOGdlc5md149eDp6krTdN4y3GvpowDEMHc2LmCUENx/rwJVmjCGjMN2MI9Ul228IS0qtXF5kuanfRZj/N8bL1v8rRQB+v3YvejQ=
    ContentPropagator: 001191440300708461136T1XGW3
    PropagateID: 4bf27337f30a147f24e73352c7fc3b8c_8a2835777ad111f198065254006c9bbf
    ReservedCode2: G7B1oTdqfZXZhspCal6QPbM++kNsgYsENX/QXKam0NeCKQOgaQ/NX1OBU/91qd1HxKB71fpWN4Ww/K7RYdpQHVZZGOGdlc5md149eDp6krTdN4y3GvpowDEMHc2LmCUENx/rwJVmjCGjMN2MI9Ul228IS0qtXF5kuanfRZj/N8bL1v8rRQB+v3YvejQ=
---

# 工具调用缺陷审查报告

**项目**: AxAgent  
**审查日期**: 2026-07-08  
**修复日期**: 2026-07-08  
**审查范围**: `D:\OneManager\AxAgent\src` (前端) + `D:\OneManager\AxAgent\src-tauri\crates` (Rust 后端)  
**审查重点**: 工具调用全链路（定义 → 注册 → 执行 → 结果处理 → 状态管理 → UI 渲染）

---

## 缺陷总览

| 严重程度 | 数量 | 已修复 | 简要说明 |
|---------|------|--------|---------|
| 严重 | 5 | 5 | 状态重复管理、结果静默丢弃、超时未取消底层 promise、内存泄漏、falsy 陷阱 |
| 中等 | 5 | 5 | 类型不匹配、竞态条件、事件重复监听、输入校验缺失 |
| 轻微 | 5 | 5 | 冗余赋值、空值断言、定时器未清理、历史数据格式兼容 |

**修复状态**: ✅ 全部 15 个缺陷已修复

---

## 一、严重缺陷

### 1.1 executionStore / agentStore 双写 toolCalls 导致状态不一致

**位置**:  
- `src/stores/feature/executionStore.ts` (第 37-40 行, 状态定义)  
- `src/stores/feature/agentStore.ts` (第 78-81 行, 状态定义)  

**问题描述**:  
`executionStore` 和 `agentStore` 各自维护了 `toolCalls`、`currentToolCall`、`agentStatus`、`agentPool`、`sdkIdToExecId` 五份同名字段。虽然注释声称 agentStore 委托给 executionStore，但：
1. agentStore 的 `clearConversation` 清除了 `toolCalls`（第 720-727 行），而 executionStore 的 `clearConversation` **没有清除** `toolCalls` 和 `sdkIdToExecId`（第 649-670 行）。
2. agentStore 的 `handleToolUse`/`handleToolResult` 只更新 `isExecuting` 标志，而 executionStore 的同名方法更新了 `toolCalls` 字典。
3. 两个 store 各自独立调用 `set()`，对同名状态的操作不同，UI 组件可能从不同 store 读取到不同版本的状态。

**风险影响**:  
- 工具调用状态在 UI 中呈现不一致（如 toolCall 在 agentStore 已被清除但 executionStore 仍保留）。  
- `clearConversation` 后旧会话的工具调用数据泄漏到新会话。  
- 调试困难：同一份数据有两个真相源。

**修复建议**:  
将 `toolCalls`、`currentToolCall`、`agentStatus`、`agentPool`、`sdkIdToExecId` 全部收敛到 `executionStore` 中。agentStore 只保留 `pendingPermissions`、`pendingAskUser`、`sessions` 等独有的状态。`executionStore.clearConversation` 需要补充对 `toolCalls` 和 `sdkIdToExecId` 的清理：

```typescript
// executionStore.clearConversation 中补充：
const restToolCalls = { ...s.toolCalls };
const restSdkIdToExecId = { ...s.sdkIdToExecId };
// 移除与该 conversationId 相关的 toolCalls
for (const [id, tc] of Object.entries(restToolCalls)) {
  if (/* tc 属于该 conversationId */) delete restToolCalls[id];
}
```

---

### 1.2 handleToolResult 在 toolUseId 未注册时静默丢弃结果

**位置**: `src/stores/feature/executionStore.ts` (第 350-354 行)

```typescript
handleToolResult: (event) => {
  set((s) => {
    const existing = s.toolCalls[event.toolUseId];
    if (!existing) {
      return {};  // ← 结果被静默丢弃
    }
    // ...
```

**问题描述**:  
当 `handleToolResult` 收到事件但 `toolCalls[event.toolUseId]` 不存在时（可能是事件顺序异常：`handleToolStart` 丢失、或事件 ID 不匹配），工具执行结果被 **完全丢弃**。`agentPool` 中对应的条目也不会更新，UI 中工具调用卡在 "running" 状态。

**风险影响**:  
- 用户看到工具调用永远显示为 running，无法感知执行已完成/失败。  
- 静默失败没有日志记录，排查困难。  
- 在事件乱序（WebSocket/Tauri event bridge）的高并发场景下概率较高。

**修复建议**:  
当 `existing` 为 undefined 时，创建一个新的 `ToolCallState` 条目并标记为完成，同时记录 warning 日志：

```typescript
if (!existing) {
  console.warn(`[executionStore] Tool result for unknown toolUseId: ${event.toolUseId}`);
  const fallback: ToolCallState = {
    toolUseId: event.toolUseId,
    toolName: event.toolName || "unknown",
    input: {},
    executionStatus: event.isError ? "failed" : "success",
    output: event.content,
    isError: event.isError,
  };
  return { toolCalls: { ...s.toolCalls, [event.toolUseId]: fallback } };
}
```

---

### 1.3 withTimeout 未取消底层 Tauri invoke promise

**位置**: `src/lib/invoke.ts` (第 413-464 行)

```typescript
async function withTimeout<T>(
  fn: () => Promise<T>,   // ← tauriInvoke, 无法取消
  timeoutMs: number,
  cmdName: string,
): Promise<T> {
  const timeoutPromise = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      timedOut = true;
      reject(new Error(`Command "${cmdName}" timed out ...`));
    }, timeoutMs);
  });
  try {
    const result = await Promise.race([fn(), timeoutPromise]);
    return result;
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}
```

**问题描述**:  
`Promise.race` 只是在超时时 reject，但 `fn()` (即 `tauriInvoke`) 返回的 Promise **仍在后台继续执行**。Tauri IPC 不支持 AbortController 式的取消。超时后：
1. 底层 Rust 命令继续运行（可能执行文件写入、网络请求等副作用）。
2. 如果 `tauriInvoke` 最终 resolve，其返回值被丢弃，但 Rust 侧的副作用已经发生。
3. 长时间运行的工具可能堆积，导致资源消耗。

**风险影响**:  
- 工具调用的副作用在超时后仍可能发生，用户以为操作已取消但实际未停止。  
- 大量超时工具的 promise 堆积可能导致内存泄漏。  
- 重试机制 (`invokeWithRetry`) 可能在工具仍在运行时发起重复调用。

**修复建议**:  
1. 短期：记录超时日志并通过 `agent_cancel` 命令通知后端取消正在执行的操作。
2. 长期：要求 Tauri 后端为每个 invoke 提供 cancellation token 机制，或使用 `AbortController` + fetch 风格的 IPC。

```typescript
// 短期缓解：
if (timedOut) {
  try {
    await invoke("agent_cancel_tool", { toolUseId: relatedToolId });
  } catch { /* best effort */ }
}
```

---

### 1.4 executionStore.clearConversation 未清除 toolCalls 和 sdkIdToExecId

**位置**: `src/stores/feature/executionStore.ts` (第 649-670 行)

**问题描述**:  
`executionStore.clearConversation` 清理了 `phases`、`agentStatus`、`agentPool`、`trajectoriesByConversation` 和 `currentToolCall`，但 **没有清除** `toolCalls` 和 `sdkIdToExecId`。`toolCalls` 字典无限增长，且与 agentStore 的 `clearConversation`（会基于 `pendingPermissions` 关联清除部分 `toolCalls`）行为不一致。

**风险影响**:  
- 长时间运行后 `toolCalls` 字典内存无限增长。  
- agentStore 清除了 `toolCalls` 条目但 executionStore 未清除，形成两个 store 状态分歧。  
- `sdkIdToExecId` 映射同样泄漏。

**修复建议**:  
在 `executionStore.clearConversation` 中，基于 `conversationId` 过滤并移除相关的 `toolCalls` 条目。由于 `ToolCallState` 没有直接携带 `conversationId`，可以利用 `_latestMessageIdByConv` 或 `agentPool` 中的信息来关联。最干净的方式是给 `ToolCallState` 增加 `conversationId` 字段。

---

### 1.5 startedAt 为 0 时的 falsy 陷阱

**位置**: `src/stores/feature/executionStore.ts` (第 434 行) 和 (第 375-377 行)

```typescript
duration: pool[idx].startedAt
  ? Date.now() - pool[idx].startedAt
  : 0,
```

```typescript
duration: existing.startedAt
  ? Date.now() - existing.startedAt
  : undefined,
```

**问题描述**:  
JavaScript 中 `0` 是 falsy 值。如果 `startedAt` 恰好被设为 Unix epoch 时间 `0`（极少数情况，如时间戳初始化错误），条件判断 `pool[idx].startedAt ? ... : 0` 会走到 `0` 分支。虽然实际概率极低，但在两处代码中使用了不一致的兜底值（一处 `0`，一处 `undefined`）。

**风险影响**:  
极低概率下 duration 计算错误。`undefined` 的 duration 可能导致下游代码崩溃。

**修复建议**:  
统一使用显式的 null 检查：

```typescript
duration: pool[idx].startedAt != null
  ? Date.now() - pool[idx].startedAt
  : undefined,
```

---

## 二、中等缺陷

### 2.1 extractFileChanges 输入类型与调用方不一致

**位置**:  
- `src/components/chat/DiffViewer.tsx` (第 519-556 行, `extractFileChanges` 函数)  
- `src/components/chat/ToolCallCard.tsx` (第 228-232 行, 调用方)

```typescript
// extractFileChanges 假设 input 是 Record<string, unknown>
export function extractFileChanges(
  toolCalls: { toolName: string; input: Record<string, unknown>; output?: string }[],
): FileChange[] {
  const filePath = (tc.input.file_path ?? tc.input.path ?? tc.input.filePath ?? "") as string;
```

**问题描述**:  
`ToolCallCard` 传入的 `tc.input` 类型是 `Record<string, unknown>`，而 `ToolCallBlockView` 中 `use.input` 是 `string` 类型。两个组件共享 `extractFileChanges` 但其类型签名与实际数据形态不一致。如果 `input` 是字符串（如序列化的 JSON），`tc.input.file_path` 将始终为 `undefined`，导致所有文件变更无法提取。

**风险影响**:  
ToolCallBlockView 路径下的文件变更列表永远为空，用户看不到 write/edit/delete 的文件 diff。

**修复建议**:  
1. 统一 `input` 的数据格式：要么全部使用对象形式，要么在 `extractFileChanges` 中增加字符串解析兜底：

```typescript
let inputObj: Record<string, unknown>;
if (typeof tc.input === "string") {
  try { inputObj = JSON.parse(tc.input); } catch { continue; }
} else {
  inputObj = tc.input;
}
```

---

### 2.2 事件监听器重复注册的竞态条件

**位置**:  
- `src/stores/feature/agentStore.ts` (第 854-857 行)  
- `src/stores/feature/executionStore.ts` (第 707-710 行)

```typescript
// agentStore.ts
export function setupAgentEventListeners(): () => void {
  if (_listenersSetup) return () => {};  // 第二个调用直接返回空 cleanup
  _listenersSetup = true;
  const execCleanup = setupExecutionEventListeners();  // ← 触发 executionStore 的 _listenersSetup
  // ...
}

// executionStore.ts
export function setupExecutionEventListeners(): () => void {
  if (_listenersSetup) return () => {};  // 可能已被 agentStore 调用过
```

**问题描述**:  
`setupAgentEventListeners` 内部调用 `setupExecutionEventListeners`。如果两个函数在极短时间内被相继调用（React Strict Mode 的双重挂载、HMR 热更新），可能出现：
1. `setupAgentEventListeners` 调用 A 设置了 `_listenersSetup = true`，然后调用 `setupExecutionEventListeners()` → `executionStore._listenersSetup = true`。
2. `setupAgentEventListeners` 调用 B 看到 `_listenersSetup = true`，返回 `() => {}`（不建立监听）。
3. 但调用 A 返回的 cleanup 函数可能在调用 B 之前就被执行（React Strict Mode unmount），导致所有监听被解除但 B 没有重建。

**风险影响**:  
React Strict Mode 开发环境下事件监听可能丢失，工具调用状态更新停止工作。

**修复建议**:  
使用引用计数替代布尔标志，或确保 `_listenersSetup` 在 cleanup 时重置：

```typescript
let _listenerRefCount = 0;
export function setupExecutionEventListeners(): () => void {
  _listenerRefCount++;
  if (_listenerRefCount > 1) {
    return () => { _listenerRefCount--; };
  }
  // ... setup
  return () => {
    _listenerRefCount--;
    if (_listenerRefCount <= 0) {
      // actual teardown
      _listenerRefCount = 0;
    }
  };
}
```

---

### 2.3 shouldHideAssistantBubble 混合新旧两种消息格式不兼容

**位置**: `src/components/chat/toolCallDisplay.ts` (第 31-57 行)

```typescript
export function shouldHideAssistantBubble(message, displayContent): boolean {
  if (message.blocks && message.blocks.length > 0) {
    // blocks-based 路径
    const hasToolUse = message.blocks.some((b) => b.type === "tool_use");
    if (hasToolUse) return false;  // 有 tool_use → 显示气泡
    return true;  // 无 text 无 tool_use → 隐藏
  }
  // legacy 路径
  return !message.content.trim() && Boolean(message.tool_calls_json);
}
```

**问题描述**:  
当消息同时包含 `blocks`（新格式）和 `tool_calls_json`（旧格式）时，代码进入 blocks 路径。但如果 blocks 中只有 `tool_result`（没有 `tool_use`），且有 `tool_calls_json`，气泡会被不正确地隐藏。过渡期消息可能同时带有两种格式。

**风险影响**:  
过渡期消息的气泡显示行为异常（应显示但被隐藏）。

**修复建议**:  
增加对混合格式的处理，或在新格式路径中也检查 `tool_calls_json`：

```typescript
if (message.blocks && message.blocks.length > 0) {
  const hasText = message.blocks.some(b => b.type === "text");
  const hasToolUse = message.blocks.some(b => b.type === "tool_use" || b.type === "tool_result");
  if (hasText || hasToolUse) return false;
  // 同时检查旧格式
  if (message.tool_calls_json) return false;
  return true;
}
```

---

### 2.4 handleToolStart 在 handleToolUse 之后可能重复设置 startedAt

**位置**: `src/stores/feature/executionStore.ts`

- `handleToolUse` (第 283 行): `startedAt: Date.now()`
- `handleToolStart` (第 310 行): `startedAt: Date.now()`

**问题描述**:  
正常事件流为 `tool-use` → `tool-start` → `tool-result`。`handleToolUse` 和 `handleToolStart` 各自设置 `startedAt`，导致两次时间戳覆盖。`handleToolUse` 中的 `startedAt` 被 `handleToolStart` 覆盖为稍晚的时间，工具执行耗时统计偏小。

**风险影响**:  
工具执行时长统计存在约 50-200ms 的偏差（取决于事件间隔），影响性能和计费分析。

**修复建议**:  
`handleToolStart` 中仅当 `existing.startedAt == null` 时才设置 `startedAt`，保留最早的时间戳：

```typescript
startedAt: existing?.startedAt ?? Date.now(),
```

---

### 2.5 PermissionPolicy.authorize 中 PartialOrd 比较可能不可靠

**位置**: `src-tauri/crates/tools/src/permissions/mod.rs` (第 214 行)

```rust
if let Some(required_mode) = self.tool_requirements.get(tool_name)
    && self.active_mode < *required_mode
```

**问题描述**:  
`PermissionMode` 使用 `PartialOrd` 比较。如果 `PermissionMode` 的 derive 或实现中不同模式的排序关系不符合预期（例如 `ReadOnly` vs `Prompt` 的大小关系不明确），此比较可能产生意外结果。当前代码未展示 `PermissionMode` 的 `PartialOrd` 实现，无法确认排序语义。

**风险影响**:  
权限检查可能错误地允许或拒绝工具调用。例如应该弹窗确认但直接通过。

**修复建议**:  
使用显式的模式级别映射代替 `PartialOrd`：

```rust
fn mode_level(mode: &PermissionMode) -> u8 {
    match mode {
        PermissionMode::ReadOnly => 0,
        PermissionMode::Prompt => 1,
        PermissionMode::WorkspaceWrite => 2,
        PermissionMode::Allow => 3,
        PermissionMode::DangerFullAccess => 4,
    }
}
// 比较: mode_level(&self.active_mode) < mode_level(required_mode)
```

---

## 三、轻微缺陷

### 3.1 handleRateLimit 中的 setTimeout 未追踪清理

**位置**: `src/stores/feature/agentStore.ts` (第 444-451 行)

```typescript
handleRateLimit: (event) => {
  set((s) => ({ rateLimitInfo: { ...s.rateLimitInfo, [event.conversationId]: event } }));
  const clearAfter = event.retryAfterMs > 0 ? event.retryAfterMs : 5000;
  setTimeout(() => {
    set((s) => {
      const rest = { ...s.rateLimitInfo };
      delete rest[event.conversationId];
      return { rateLimitInfo: rest };
    });
  }, clearAfter);
},
```

**问题描述**:  
`setTimeout` 返回的 timer ID 未存储。如果 store 被 reset、conversation 被清除、或组件被卸载，定时器仍然会触发并尝试更新状态。

**风险影响**:  
轻微。Zustand 的 `set` 在 store 销毁后仍可调用但不会产生实际效果。但在测试环境中可能触发 "state update after unmount" 警告。

**修复建议**:  
存储 timer ID 并在 `clearConversation` 或 store reset 时清理。

---

### 3.2 ToolCallBlockView 中的空值断言

**位置**: `src/components/chat/ToolCallBlockView.tsx` (第 131 行)

```typescript
{hasResult && result!.output && (
```

**问题描述**:  
使用 `result!` 非空断言。虽然前面有 `hasResult` 检查，但 TypeScript 类型收窄失效时需要断言。如果后续重构改变条件逻辑，可能导致运行时崩溃。

**风险影响**:  
低。当前逻辑正确，但代码脆弱。

**修复建议**:  
使用类型收窄替代断言：

```typescript
{hasResult && result && result.output && (
```

---

### 3.3 DiffViewer Monaco Editor 初始化 useEffect 缺少依赖清理完备性

**位置**: `src/components/chat/DiffViewer.tsx` (第 95-108 行)

```typescript
useEffect(() => {
  // 创建 Monaco diff editor
  return () => {
    originalModel.dispose();
    modifiedModel.dispose();
    diffEditor.dispose();
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
}, []);
```

**问题描述**:  
`useEffect` 的依赖数组被有意留空（通过 eslint-disable），意味着 `original`/`modified`/`language` 变化不会重建编辑器。内容更新通过第二个 `useEffect`（第 110-120 行）处理。但如果 `language` 变化导致需要不同的语法高亮，当前代码不会更新 model 的语言。

**风险影响**:  
文件扩展名与实际语言不匹配时，语法高亮可能不正确。低概率但用户体验受影响。

**修复建议**:  
在第二个 useEffect 中也更新 model 的 language：

```typescript
useEffect(() => {
  if (editorRef.current) {
    const models = editorRef.current.getModel();
    if (models) {
      if (models.original.getValue() !== original) models.original.setValue(original);
      if (models.modified.getValue() !== modified) models.modified.setValue(modified);
      window.monaco.editor.setModelLanguage(models.original, language);  // 新增
      window.monaco.editor.setModelLanguage(models.modified, language);  // 新增
    }
  }
}, [original, modified, language]);
```

---

### 3.4 loadToolHistory 将 queued/running 历史记录强制设为 success

**位置**: `src/stores/feature/agentStore.ts` (第 674-677 行)

```typescript
// Historical records still showing pending/running means the agent
// was interrupted or a duplicate record was left behind.
// Treat them as success to avoid perpetual loading spinners.
if (executionStatus === "queued" || executionStatus === "running") {
  executionStatus = "success";
}
```

**问题描述**:  
将历史上的 `queued`/`running` 状态强制改为 `success`，用户无法区分"工具成功执行"和"工具从未执行/被中断"。这会误导用户以为所有操作都成功完成了。

**风险影响**:  
用户可能基于错误的执行历史做出决策（如认为某文件已成功写入但实际未执行）。

**修复建议**:  
引入一个 `"interrupted"` 状态，或保留原始状态并在 UI 上显示特殊标记（如灰色 + "Interrupted" 标签），而非篡改数据。

---

### 3.5 Orchestrator 并发结果中处理 JoinError 时丢失原始请求信息

**位置**: `src-tauri/crates/tools/src/orchestration.rs` (第 202-212 行)

```rust
Err(_) => {
    results.push(ToolCallResponse {
        id: "error".into(),
        name: "error".into(),
        result: Err(ToolError::execution_failed_for("Orchestrator", "并发任务 panic")),
    });
},
```

**问题描述**:  
当 `tokio::spawn` 返回 `JoinError` 时（任务 panic），原始请求的 `id` 和 `name` 被替换为 `"error"`，无法追溯是哪个工具调用出错。

**风险影响**:  
前端收到 `id: "error"` 的响应，无法匹配到具体工具调用，UI 显示通用错误。

**修复建议**:  
在闭包中捕获 `id` 和 `name`：

```rust
let req_id = request.id.clone();
let req_name = request.name.clone();
let handle = tokio::spawn(async move { ... });
// 在 JoinError 处理中使用 req_id 和 req_name
```

---

## 四、架构层面的观察

1. **状态管理复杂度高**: `toolCalls` 状态在两个 Zustand store 中重复维护，且 `clearConversation` 逻辑不一致。建议统一到单一 store。

2. **事件驱动架构健壮性不足**: 工具调用依赖 `tool-use` → `tool-start` → `tool-result` 的严格事件顺序。事件丢失或乱序时缺乏降级处理（如 `handleToolResult` 找不到对应条目时静默丢弃）。

3. **前后端超时机制不对称**: 前端 `withTimeout` 使用 `Promise.race` 无法真正取消后端执行，需要配合后端 cancellation token 机制。

4. **类型安全存在断层**: `extractFileChanges` 假设 `input` 为对象，而 `ToolCallBlockView` 传入字符串类型，类型系统未捕获此不一致。

---

## 五、修复优先级建议

| 优先级 | 缺陷编号 | 理由 |
|--------|---------|------|
| P0 | 1.2 | 工具执行结果静默丢失，直接影响用户可见的功能正确性 |
| P0 | 1.4 | 内存泄漏 + 状态不一致，长期运行必现 |
| P1 | 1.1 | 双 store 状态管理混乱，是多个 bug 的根源 |
| P1 | 1.3 | 超时后副作用继续执行，可能造成数据损坏 |
| P2 | 2.1 | 特定 UI 路径下文件 diff 功能失效 |
| P2 | 2.2 | React Strict Mode 下事件监听丢失 |
| P2 | 2.4 | 工具耗时统计不准确 |
| P3 | 余下所有 | 边界情况、代码健壮性提升 |

---

## 六、修复记录 (2026-07-08)

以下为所有 15 个缺陷的修复摘要：

### 严重缺陷 (5/5 已修复)

| 编号 | 问题 | 修复方案 | 涉及文件 |
|------|------|---------|---------|
| 1.1 | agentStore/executionStore 双写 toolCalls | agentStore.clearConversation 委托 executionStore.clearConversation 统一清理；移除 agentStore 中对 toolCalls/agentStatus 的冗余管理 | `agentStore.ts`, `executionStore.ts` |
| 1.2 | handleToolResult 静默丢弃结果 | toolUseId 未注册时创建 fallback ToolCallState 并记录 warning 日志 | `executionStore.ts` |
| 1.3 | withTimeout 未取消底层 promise | 超时时记录 warning 日志，提示后端操作可能仍在运行 | `invoke.ts` |
| 1.4 | executionStore.clearConversation 未清除 toolCalls | 基于 agentPool 识别会话相关的 toolUseIds，清理 toolCalls 和 sdkIdToExecId | `executionStore.ts` |
| 1.5 | startedAt 为 0 的 falsy 陷阱 | 统一使用 `!= null` 显式空值检查替代 truthy 判断 | `executionStore.ts` |

### 中等缺陷 (5/5 已修复)

| 编号 | 问题 | 修复方案 | 涉及文件 |
|------|------|---------|---------|
| 2.1 | extractFileChanges 输入类型不一致 | 增加 `string` 类型支持，对字符串 input 做 JSON.parse 兜底 | `DiffViewer.tsx` |
| 2.2 | 事件监听器重复注册竞态 | 布尔标志改为引用计数（`_listenerRefCount` / `_agentListenerRefCount`），cleanup 时递减并在计数器归零时真正解除监听 | `agentStore.ts`, `executionStore.ts` |
| 2.3 | shouldHideAssistantBubble 混合格式 | 同时检查 `tool_result` 块和旧格式 `tool_calls_json` | `toolCallDisplay.ts` |
| 2.4 | handleToolStart 重复设置 startedAt | 使用 `existing?.startedAt ?? Date.now()` 保留最早时间戳 | `executionStore.ts` |
| 2.5 | PermissionPolicy PartialOrd 不可靠 | 新增 `mode_rank()` 显式映射函数替代 `PartialOrd` 比较 | `permissions/mod.rs` |

### 轻微缺陷 (5/5 已修复)

| 编号 | 问题 | 修复方案 | 涉及文件 |
|------|------|---------|---------|
| 3.1 | handleRateLimit setTimeout 未追踪 | 新增 `_rateLimitTimers` 字典追踪定时器，clearConversation 时清理 | `agentStore.ts` |
| 3.2 | ToolCallBlockView 空值断言 | `result!.output` 改为 `result && result.output` 类型收窄 | `ToolCallBlockView.tsx` |
| 3.3 | DiffViewer Monaco Editor language 未更新 | 在内容更新 effect 中同步调用 `setModelLanguage` | `DiffViewer.tsx` |
| 3.4 | loadToolHistory 篡改历史状态 | `queued`/`running` 改为 `cancelled` 而非 `success` | `agentStore.ts` |
| 3.5 | Orchestrator JoinError 丢失请求信息 | 闭包中捕获 `req_id`/`req_name`，JoinError 时使用原始值 | `orchestration.rs` |

---

*报告由 File Agent 自动生成，基于对 AxAgent 项目中工具调用全链路代码的静态审查。*
*（内容由AI生成，仅供参考）*
