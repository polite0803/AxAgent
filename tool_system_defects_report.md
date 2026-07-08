---
AIGC:
    Label: "1"
    ContentProducer: 001191440300708461136T1XGW3
    ProduceID: 4bf27337f30a147f24e73352c7fc3b8c_97f642c17ad411f198065254006c9bbf
    ReservedCode1: aHyPDEKYuOyBvep+g+6EaXcJxa1OpyiBxRhM/bWgx0PTkbvAb7yvuAWPTJkfeyYO96Ok3ktPwl/NmmO/q76KN4gP7IHefS5CqAMciSd6FAUKAPm60q3o5oux8+ZmqDlB+a43yE5GOvgEXV3sPkgIUJFcDYVtf4FaJcKkAaeDuPUjzuaoiPBwAy0bgWA=
    ContentPropagator: 001191440300708461136T1XGW3
    PropagateID: 4bf27337f30a147f24e73352c7fc3b8c_97f642c17ad411f198065254006c9bbf
    ReservedCode2: aHyPDEKYuOyBvep+g+6EaXcJxa1OpyiBxRhM/bWgx0PTkbvAb7yvuAWPTJkfeyYO96Ok3ktPwl/NmmO/q76KN4gP7IHefS5CqAMciSd6FAUKAPm60q3o5oux8+ZmqDlB+a43yE5GOvgEXV3sPkgIUJFcDYVtf4FaJcKkAaeDuPUjzuaoiPBwAy0bgWA=
---

# AxAgent 工具系统缺陷报告

> 生成日期：2026-07-08  
> 修复日期：2026-07-08  
> 审查范围：`D:\OneManager\AxAgent\src` 下所有工具系统相关代码  
> 审查文件数：30+ 核心文件

---

## 缺陷修复统计

| 严重程度 | 总数 | 已修复 | 部分修复 | 说明 |
|---------|------|--------|----------|------|
| 🔴 高危 | 5 | 5 | 0 | 全部修复 |
| 🟡 中危 | 12 | 12 | 0 | 全部修复（含 2.6 软取消方案） |
| 🟢 低危 | 6 | 6 | 0 | 全部修复 |
| **总计** | **23** | **23** | **0** | |

> 设计建议（4.1-4.3）为架构级改进建议，未纳入本次修复范围。

---

## 一、安全漏洞

### 1.1 🔴 Pyodide 加载缺少 SRI 完整性校验 — **✅ 已修复**

- **文件**: `src/lib/codeExecutor.ts`
- **行号**: 57–64
- **描述**: Pyodide 脚本从 CDN 动态加载时未设置 `integrity` 属性。
- **修复内容**: 添加了 sha384 SRI integrity 哈希值，并在注释中说明了升级 Pyodide 时更新 hash 的方法。

### 1.2 🔴 Python 代码注入风险 — **✅ 已修复**

- **文件**: `src/lib/codeExecutor.ts`
- **行号**: 132–146
- **描述**: base64 编码的代码通过 f-string 直接拼接进 Python 模板字符串。
- **修复内容**: 使用 `JSON.stringify(JSON.stringify(encodedCode))` 双重序列化 + Python 端 `json.loads()` 反序列化，彻底杜绝字符串注入。

### 1.3 🔴 `navigate` 动作无路径遍历防护 — **✅ 已修复**

- **文件**: `src/lib/actionRouter.ts`
- **行号**: 242–257
- **描述**: `navigate` 执行器未验证 path 中的危险模式。
- **修复内容**: 添加了对 `..`、`//`、`\\` 的检测，拒绝包含路径遍历模式的请求。

### 1.4 🔴 `emit` 动作可触发保留 DOM 事件 — **✅ 已修复**

- **文件**: `src/lib/actionRouter.ts`
- **行号**: 259–271
- **描述**: 未校验事件名是否与内置 DOM 事件冲突。
- **修复内容**: 定义了 `RESERVED_DOM_EVENTS` 集合（30+ 常用 DOM 事件），并要求所有 emit 事件必须包含 `:` 命名空间前缀（如 `skill:myevent`）。

### 1.5 🔴 `store` 动作无 Payload 结构校验 — **✅ 已修复**

- **文件**: `src/lib/actionRouter.ts`
- **行号**: 285–334
- **描述**: store 的 `set` 和 `get` 操作未校验 payload 结构。
- **修复内容**: `set` 操作要求 payload 为非数组的普通对象；`get` 操作的 selector 字段必须为 string 类型。不匹配时返回明确错误。

---

## 二、功能缺陷与边界条件

### 2.1 🟡 `handleToolResult` 孤儿条目泄漏 — **✅ 已修复**

- **文件**: `src/stores/feature/executionStore.ts`
- **行号**: 262–310
- **描述**: fallback 条目未加入 agentPool，导致 clearConversation 无法清理。
- **修复内容**: fallback 创建时同步写入 agentPool，包含完整的 AgentPoolItem 信息。

### 2.2 🟡 `clearConversation` 工具匹配逻辑缺陷 — **✅ 已修复**

- **文件**: `src/stores/feature/executionStore.ts`
- **行号**: 524–572
- **描述**: 孤儿 toolCalls 未被清理；worker- 前缀条目处理不精确。
- **修复内容**: 增加了 `worker-` 前缀的独立追踪；新增直接扫描 toolCalls 的逻辑，通过 `assistantMessageId` 关联 pool 外的孤儿条目。

### 2.3 🟡 `toolCalls` 双重键值导致的潜在不一致 — **✅ 已修复**

- **文件**: `src/stores/feature/executionStore.ts`
- **行号**: 206–244
- **描述**: `handleToolResult` 仅更新 toolUseId 键，executionId 键停留在过期状态。
- **修复内容**: 在 `handleToolResult` 中通过 `sdkIdToExecId` 映射找到 executionId 键并同步更新。

### 2.4 🟡 `toggleTool` 全量替换 groups 的竞态条件 — **✅ 已修复**

- **文件**: `src/stores/feature/localToolStore.ts`
- **行号**: 72–83
- **描述**: 后端返回的全量 groups 直接替换本地状态。
- **修复内容**: 改为局部合并策略——对已有 group 按 groupId 匹配更新，新 group 追加，未在响应中的已有 group 保留。

### 2.5 🟡 `ToolUpgradeRequest` 字段缺失 — **✅ 已修复**

- **文件**: `src/components/settings/ToolSemanticCheck.tsx`
- **行号**: 125–134
- **描述**: schema 字段未传入升级请求。
- **修复内容**: 将 `existing_input_schema`、`existing_output_schema`、`generated_input_schema`、`generated_output_schema` 四个字段从 match/source 对象中提取并传入。当前数据模型中这些字段可能不存在，作为 undefined 传入（可选字段）。

### 2.6 🟡 `withTimeout` 超时后后端操作仍在运行 — **✅ 已修复**

- **文件**: `src/lib/invoke.ts`
- **行号**: 429–508
- **描述**: 前端超时后 Tauri 后端 IPC 调用无法真正中断（Tauri v2 IPC 无 abort 机制，后端确认无 cancel 接口）。
- **修复内容**: 
  1. 新增 `TimeoutError` 类，超时错误有明确类型标识
  2. `withTimeout` 引入显式 cancellation token（`{ value: false }`），超时后即使后端结果返回也会被 `guardedFn` 永久挂起，绝不更新前端状态
  3. `isRetryableError` 显式拒绝 `TimeoutError`（超时表示操作过重，重试只会雪上加霜）
  4. 从 `RETRYABLE_ERROR_PATTERNS` 中移除 `/timeout/i`（之前会将超时误判为可重试的网络超时）
  5. 清理了误导性的 `agent_cancel` 提示（后端不存在该接口）

### 2.7 🟡 `invokeWithRetry` jitter 范围过窄 — **✅ 已修复**

- **文件**: `src/lib/invoke.ts`
- **行号**: 83–94
- **描述**: jitter 为 ±5%，不足以分散高并发请求尖峰。
- **修复内容**: jitter 范围从 `delay * 0.1`（±5%）扩大到 `delay * 0.5`（±25%）。

### 2.8 🟡 连接错误检测依赖于错误消息字符串匹配 — **✅ 已修复**

- **文件**: `src/lib/invoke.ts`
- **行号**: 349–356
- **描述**: withTimeout 中用字符串包含检测连接错误。
- **修复内容**: 使用统一的 `classifyIpcError()` / `isConnectionError()` 函数替代字符串匹配，同时检查 Error.code 属性（结构化错误码优先）。

### 2.9 🟡 `deleteTool` 乐观删除无回滚机制 — **✅ 已修复**

- **文件**: `src/stores/feature/localToolStore.ts`
- **行号**: 41–47
- **描述**: 后端失败后 UI 中工具不恢复。
- **修复内容**: 改为"先乐观移除 → 调后端 → 失败时恢复"模式：在调用前保存 `previousTools` 快照，catch 中恢复完整列表。

### 2.10 🟡 `React.memo` 比较函数未深度比较 `input` — **✅ 已修复**

- **文件**: `src/components/chat/ToolCallCard.tsx`
- **行号**: 278–301
- **描述**: memo 比较函数跳过 input 对象。
- **修复内容**: 添加 `JSON.stringify(a.input) !== JSON.stringify(b.input)` 比较。

### 2.11 🟡 `isStorePermCovered` 不支持数组索引路径 — **✅ 已修复**

- **文件**: `src/lib/skillPermissions.ts`
- **行号**: 88–103
- **描述**: 字段路径匹配不支持数字索引段。
- **修复内容**: 扩展匹配逻辑，允许 `parsed.fieldPath` 中包含数字段（如 `items.0`），使 `items` 权限能覆盖 `items.0.name`。

### 2.12 🟡 执行阶段状态机缺少 planning → waiting_permission 转换 — **✅ 已修复**

- **文件**: `src/stores/feature/executionPhaseMachine.ts`
- **行号**: 13–27
- **描述**: planning 状态无法转向 waiting_permission。
- **修复内容**: 在 `PHASE_TRANSITIONS.planning` 中添加 `"waiting_permission"`。

---

## 三、代码质量与性能

### 3.1 🟢 ComponentRegistry `get()` 线性扫描 — **✅ 已修复**

- **文件**: `src/lib/dynamicUI/ComponentRegistry.ts`
- **行号**: 24–36
- **描述**: 精确 key 未命中时 O(n) 扫描。
- **修复内容**: 新增 `typeIndex` 反向索引 Map，精确 key 未命中时通过 O(1) 查找，注册/注销时同步维护。

### 3.2 🟢 `getAllTypes()` 类型断言不安全 — **✅ 已修复**

- **文件**: `src/lib/dynamicUI/ComponentRegistry.ts`
- **行号**: 63–65
- **描述**: 命名空间 key 被断言为 DynamicComponentType。
- **修复内容**: 返回前用 `.filter((key) => !key.includes(":"))` 过滤命名空间前缀条目。

### 3.3 🟢 SchemaValidator 无嵌套深度限制 — **✅ 已修复**

- **文件**: `src/lib/dynamicUI/SchemaValidator.ts`
- **行号**: 27–125
- **描述**: 无深度限制的递归可能导致栈溢出。
- **修复内容**: 添加 `MAX_NESTING_DEPTH = 50`，`validateNode` 增加 `depth` 参数，超过阈值时报告错误并停止递归。

### 3.4 🟢 `ToolSemanticCheck` useEffect 中使用 setTimeout(0) 反模式 — **✅ 已修复**

- **文件**: `src/components/settings/ToolSemanticCheck.tsx`
- **行号**: 120–123
- **描述**: 无意义的 setTimeout 包装。
- **修复内容**: 直接调用 `checkSemanticMatches(selectedTool)`，移除 setTimeout。

### 3.5 🟢 连接错误分类检测分散且不一致 — **✅ 已修复**

- **文件**: `src/lib/invoke.ts`
- **行号**: 43–52、255–279、349–356
- **描述**: 三处独立硬编码的错误匹配列表。
- **修复内容**: 提取统一的 `classifyIpcError()` 函数，返回 `"connection" | "transient" | "other"` 三类。`isRetryableError()` 和 `isConnectionError()` 基于此函数实现，`recordDiag` 和 `withTimeout` 统一使用。

### 3.6 🟢 `storage.ts` 无容量检查 — **✅ 已修复**

- **文件**: `src/lib/storage.ts`
- **行号**: 24–31
- **描述**: 仅处理 QuotaExceededError，无预防性检查。
- **修复内容**: 序列化后检查大小，超过 500KB 阈值时输出 `console.warn` 建议拆分或使用 IndexedDB。

---

## 四、设计问题与建议（未纳入本次修复）

以下为架构级改进建议，需要更大范围的代码变更，建议在后续版本中规划：

### 4.1 工具执行结果缺少结构化错误模型
在整个工具调用链路中，错误信息以纯字符串形式传递。建议定义统一的 `ToolError` 接口。

### 4.2 工具调用缺少统一的审计/追踪机制
当前工具调用的记录分散在三个独立系统中，建议引入统一的端到端追踪 ID。

### 4.3 `browserMock.ts` 维护负担
3157 行的 mock 文件需手动同步。建议改为自动生成或共享接口定义。

---

## 附录：审查文件清单

| 文件 | 路径 | 行数 |
|------|------|------|
| invoke.ts | `src/lib/invoke.ts` | 596 |
| codeExecutor.ts | `src/lib/codeExecutor.ts` | 185 |
| actionRouter.ts | `src/lib/actionRouter.ts` | 519 |
| skillPermissions.ts | `src/lib/skillPermissions.ts` | 277 |
| SchemaValidator.ts | `src/lib/dynamicUI/SchemaValidator.ts` | 222 |
| ComponentRegistry.ts | `src/lib/dynamicUI/ComponentRegistry.ts` | 87 |
| skillActionExecutor.ts | `src/lib/skillActionExecutor.ts` | 53 |
| storage.ts | `src/lib/storage.ts` | 177 |
| executionStore.ts | `src/stores/feature/executionStore.ts` | 866 |
| executionPhaseMachine.ts | `src/stores/feature/executionPhaseMachine.ts` | 41 |
| executionToolCallUtils.ts | `src/stores/feature/executionToolCallUtils.ts` | 34 |
| localToolStore.ts | `src/stores/feature/localToolStore.ts` | 89 |
| recommendationStore.ts | `src/stores/devtools/recommendationStore.ts` | 120 |
| conversationStoreSend.ts | `src/stores/domain/conversationStoreSend.ts` | 1739 |
| localTool.ts | `src/types/localTool.ts` | 67 |
| dynamicUI.ts | `src/types/dynamicUI.ts` | 298 |
| index.ts | `src/types/index.ts` | 1574 |
| ToolCallCard.tsx | `src/components/chat/ToolCallCard.tsx` | 301 |
| toolCallDisplay.ts | `src/components/chat/toolCallDisplay.ts` | 62 |
| ToolManager.tsx | `src/components/settings/ToolManager.tsx` | 272 |
| LocalToolSettings.tsx | `src/components/settings/LocalToolSettings.tsx` | 208 |
| ToolSemanticCheck.tsx | `src/components/settings/ToolSemanticCheck.tsx` | 449 |
| ToolRecommendationPanel.tsx | `src/components/recommendation/ToolRecommendationPanel.tsx` | 222 |
| ToolNode.tsx | `src/components/workflow/Nodes/ToolNode.tsx` | 122 |
| ToolPropertyPanel.tsx | `src/components/workflow/Panels/PropertyPanels/ToolPropertyPanel.tsx` | 215 |
| browserMock.ts | `src/lib/browserMock.ts` | 3157 |

---

*修复完成。23 个缺陷全部修复。2.6（withTimeout 超时取消）通过前端软取消方案实现：超时后结果绝不更新 UI、不会被重试，虽无法真正中断后端执行（Tauri v2 IPC 无此能力），但已满足"超时后不再影响前端状态"的修复目标。*
*（内容由AI生成，仅供参考）*
