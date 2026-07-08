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

> 生成日期：2026-07-08\
> 审查范围：`D:\OneManager\AxAgent\src` 下所有工具系统相关代码\
> 审查文件数：30+ 核心文件

---

## 缺陷统计

| 严重程度 | 数量   | 说明                   |
| -------- | ------ | ---------------------- |
| 🔴 高危  | 5      | 安全漏洞、数据丢失风险 |
| 🟡 中危  | 12     | 功能缺陷、边界条件缺失 |
| 🟢 低危  | 6      | 代码质量、性能隐患     |
| **总计** | **23** |                        |

---

## 一、安全漏洞

### 1.1 🔴 Pyodide 加载缺少 SRI 完整性校验

- **文件**: `src/lib/codeExecutor.ts`
- **行号**: 57–64
- **描述**: Pyodide 脚本从 CDN (`https://cdn.jsdelivr.net/pyodide/v0.24.1/full/pyodide.js`) 动态加载时，仅设置了 `script.crossOrigin = "anonymous"`，但未设置 `integrity` 属性。这意味着若 CDN 被入侵或中间人攻击，恶意脚本可被执行，获取用户敏感数据或操作文件系统。
- **严重程度**: 🔴 高危
- **修复建议**: 为 `<script>` 标签添加 `integrity` 属性，使用已知的 SRI 哈希值。同时考虑将 Pyodide 打包到应用本地而非从 CDN 加载。

### 1.2 🔴 Python 代码注入风险

- **文件**: `src/lib/codeExecutor.ts`
- **行号**: 132–146
- **描述**: Python 代码通过 `btoa` + 手动字符编码后，以字符串拼接方式嵌入 Python f-string 模板。流程为：`JavaScript template literal → base64 字符串 → Python f-string → base64.b64decode → exec()`。若 `code` 中包含精心构造的内容（如闭合 Python 字符串的引号），可能绕过预期的执行边界。虽然 btoa 编码降低了直接注入风险，但编码后的 base64 字符串仍可能包含 Python 特殊字符。
- **严重程度**: 🔴 高危
- **修复建议**: 使用结构化的数据传递方式（如 Pyodide 的 `pyodide.to_js` / `JsProxy` 机制），将代码作为独立参数传入，而非字符串拼接。或至少使用 `json.dumps()` 序列化代码字符串再在 Python 端反序列化。

### 1.3 🔴 `navigate` 动作无路径遍历防护

- **文件**: `src/lib/actionRouter.ts`
- **行号**: 242–257
- **描述**: `navigate` 执行器直接使用 `window.location.hash = action.path`，未对 path 做任何验证。恶意 skill 可通过声明 `"path": "../../../malicious"` 进行路径遍历，或注入特殊字符导致意外跳转。
- **严重程度**: 🔴 高危
- **修复建议**: 对 `action.path` 添加白名单校验或路径规范化，拒绝包含 `..`、`//` 等危险模式的路径。

### 1.4 🔴 `emit` 动作可触发保留 DOM 事件

- **文件**: `src/lib/actionRouter.ts`
- **行号**: 259–271
- **描述**: `emit` 执行器通过 `window.dispatchEvent(new CustomEvent(action.event, ...))` 派发事件，未校验事件名是否与内置 DOM 事件（如 `"click"`、`"submit"`、`"hashchange"`）冲突。恶意 skill 可派发伪装的 DOM 事件触发全局监听器。
- **严重程度**: 🔴 高危
- **修复建议**: 强制要求自定义事件名使用带命名空间前缀的格式（如 `skill:myevent`），拒绝无前缀的事件名。

### 1.5 🔴 `store` 动作无 Payload 结构校验

- **文件**: `src/lib/actionRouter.ts`
- **行号**: 285–334
- **描述**: `store` 执行器在权限校验通过后直接调用 `store[operation](action.payload)`。但 `action.payload` 的结构未经验证——若某个 Zustand store 的 `set` 方法期望特定字段格式，传入不匹配的 payload 会静默失败或触发不可预期的状态变更。
- **严重程度**: 🔴 高危
- **修复建议**: 为每个注册的 store 定义 payload schema，在执行前进行结构校验，拒绝不匹配的 payload。

---

## 二、功能缺陷与边界条件

### 2.1 🟡 `handleToolResult` 孤儿条目泄漏

- **文件**: `src/stores/feature/executionStore.ts`
- **行号**: 262–310
- **描述**: 当 `handleToolResult` 收到一个未知 `toolUseId` 时，会创建 fallback 条目写入 `toolCalls`，但**不会将其加入 `agentPool`**。这意味着后续 `clearConversation` 中的池扫描逻辑无法识别该条目，导致工具调用数据无法被正确清理。
- **严重程度**: 🟡 中危
- **修复建议**: 在 fallback 创建时同步将条目加入 `agentPool`，或在 `clearConversation` 中增加独立于 pool 的工具调用清理逻辑。

### 2.2 🟡 `clearConversation` 工具匹配逻辑缺陷

- **文件**: `src/stores/feature/executionStore.ts`
- **行号**: 524–572
- **描述**: 清理会话时通过 `item.id.startsWith("tool-")` + `item.id.replace("tool-", "")` 识别工具调用。但若某些工具调用因 `handleToolStart` 未被调用而从未加入 pool，它们不会被清理。此外，`worker-` 前缀的条目与 `tool-` 前缀共享同一个 pool，清理逻辑可能误删 worker 条目。
- **严重程度**: 🟡 中危
- **修复建议**: 为 tool/worker 使用不同的 pool 或增加类型字段区分，确保清理精确匹配。

### 2.3 🟡 `toolCalls` 双重键值入导致的潜在不一致

- **文件**: `src/stores/feature/executionStore.ts`
- **行号**: 206–244
- **描述**: `handleToolUse` 中，当 `event.executionId` 存在时，同一个工具调用以两个不同键（`toolUseId` 和 `executionId`）写入 `toolCalls`。后续 `handleToolResult` 只更新 `toolUseId` 键，导致 `executionId` 键对应的条目停留在过期状态。
- **严重程度**: 🟡 中危
- **修复建议**: 在 `handleToolResult` 中，同步更新 `sdkIdToExecId` 映射对应的两条记录，或使用引用而非复制来避免双写。

### 2.4 🟡 `toggleTool` 全量替换 groups 的竞态条件

- **文件**: `src/stores/feature/localToolStore.ts`
- **行号**: 72–83
- **描述**: `toggleTool` 在 toggle 单个工具后，直接用后端返回的 `updatedGroups` 替换全部 `groups`。若在请求飞行期间有其他工具状态变更（如异步批量操作），此替换会丢失并发变更。
- **严重程度**: 🟡 中危
- **修复建议**: 改为局部更新：仅替换被 toggle 的工具所在的 group，而非全量替换。

### 2.5 🟡 `ToolUpgradeRequest` 字段缺失

- **文件**: `src/components/settings/ToolSemanticCheck.tsx`
- **行号**: 125–134
- **描述**: `handleUpgradeTool` 构造 `ToolUpgradeRequest` 时，只填充了 `existing_tool_name/description/type` 和 `generated_name/description`，但类型定义中还有 `existing_input_schema/existing_output_schema/generated_input_schema/generated_output_schema` 四个可选字段，均未填充。后端若依赖这些字段做语义分析，升级质量会下降。
- **严重程度**: 🟡 中危
- **修复建议**: 从 `selectedMatch` 或 `selectedTool` 中提取 schema 信息并传入请求。

### 2.6 🟡 `withTimeout` 超时后后端操作仍在运行

- **文件**: `src/lib/invoke.ts`
- **行号**: 314–356
- **描述**: `withTimeout` 使用 `Promise.race` 实现超时。超时后虽然 reject 了前端 promise，但 Tauri 后端的 IPC 调用仍在执行。代码中虽有 `console.warn` 提示，但没有取消机制。对于长时间运行的工具调用（如大文件处理），这会导致后端资源泄漏。
- **严重程度**: 🟡 中危
- **修复建议**: 为可取消的命令实现取消机制（如通过 `agent_cancel` 传递取消信号），在超时后主动取消后端操作。

### 2.7 🟡 `invokeWithRetry` jitter 范围过窄

- **文件**: `src/lib/invoke.ts`
- **行号**: 83–94
- **描述**: 指数退避的 jitter 计算为 `delay * 0.1 * (Math.random() - 0.5)`，即 ±5% 的抖动范围。对于高并发场景下多个客户端同时重试的情况，5% 的抖动不足以有效分散请求尖峰。
- **严重程度**: 🟡 中危
- **修复建议**: 将 jitter 范围扩大到 ±25%（即 `delay * 0.5 * (Math.random() - 0.5)` 或使用 "decorrelated jitter"）。

### 2.8 🟡 `actionRouter` 连接错误检测依赖于错误消息字符串匹配

- **文件**: `src/lib/invoke.ts`
- **行号**: 349–356
- **描述**: `withTimeout` 中检测 "连接失败" 的条件是：`msg.includes("connection") || msg.includes("refused") || msg.includes("fetch") || msg.includes("ipc") || msg.includes("protocol")`。这种基于字符串包含的检测：
  - 可能误匹配（如用户数据中包含 "connection" 字样）
  - 可能漏掉（如 `"ECONNABORTED"` 不匹配任何模式）
  - 依赖于错误消息的英文措辞，多语言环境下可能失效
- **严重程度**: 🟡 中危
- **修复建议**: 使用结构化错误类型（自定义 Error 子类或错误码），而非字符串匹配。

### 2.9 🟡 `deleteTool` 乐观删除无回滚机制

- **文件**: `src/stores/feature/localToolStore.ts`
- **行号**: 41–47
- **描述**: `deleteTool` 先从 UI 中移除工具（乐观更新），再发起后端删除请求。若后端请求失败，catch 块仅设置 error 状态，工具不会重新出现在列表中。除非用户手动刷新，否则 UI 会一直显示不一致的状态。
- **严重程度**: 🟡 中危
- **修复建议**: 在 catch 块中恢复被删除的工具，或重新调用 `loadTools()` 重新加载完整列表。

### 2.10 🟡 `React.memo` 比较函数未深度比较 `input`

- **文件**: `src/components/chat/ToolCallCard.tsx`
- **行号**: 278–301
- **描述**: `ToolCallCard` 的 `React.memo` 比较函数虽然比较了 `output`、`isError` 等字段，但完全跳过了 `input` 对象的比较。若同一工具调用的 `input` 参数在流式更新中发生变化（如参数逐步填充），组件不会重新渲染。
- **严重程度**: 🟡 中危
- **修复建议**: 添加 `JSON.stringify(prev.input) !== JSON.stringify(next.input)` 比较。

### 2.11 🟡 `isStorePermCovered` 不支持数组索引路径

- **文件**: `src/lib/skillPermissions.ts`
- **行号**: 88–103
- **描述**: `isStorePermCovered` 的字段路径匹配逻辑使用 `.` 分隔符检查前缀匹配，但不支持数组索引（如 `storeName:items.0.name`）。若 store 中存储的是数组，无法精确控制对数组元素的访问权限。
- **严重程度**: 🟡 中危
- **修复建议**: 扩展字段路径语法支持数字索引，或在文档中明确说明不支持。

### 2.12 🟡 执行阶段状态机允许非预期的转换路径

- **文件**: `src/stores/feature/executionPhaseMachine.ts`
- **行号**: 13–27
- **描述**: `PHASE_TRANSITIONS` 允许 `completed -> executing` 和 `failed -> executing` 的转换。虽然可能是为了支持"重新发送消息"场景，但没有清理上轮执行状态的逻辑。`planning` 状态没有到 `waiting_permission` 的转换路径，但在 `ACTIVE_PHASES` 中 `planning` 被视为活跃状态——若 plan 执行期间需要用户权限确认，状态机会卡住。
- **严重程度**: 🟡 中危
- **修复建议**: 添加 `planning -> waiting_permission` 转换；为 `* -> executing` 的"重入"转换添加前置清理钩子。

---

## 三、代码质量与性能

### 3.1 🟢 ComponentRegistry `get()` 线性扫描

- **文件**: `src/lib/dynamicUI/ComponentRegistry.ts`
- **行号**: 24–36
- **描述**: 当精确 key 未命中时，`get()` 方法执行全量 Map 迭代进行线性扫描。在高频调用场景（如动态 UI 渲染大量组件时），O(n) 扫描会影响性能。
- **严重程度**: 🟢 低危
- **修复建议**: 维护一个去命名空间前缀的反向索引 Map，使查找保持 O(1)。

### 3.2 🟢 `getAllTypes()` 类型断言不安全

- **文件**: `src/lib/dynamicUI/ComponentRegistry.ts`
- **行号**: 63–65
- **描述**: `getAllTypes()` 将 Map 的 keys（可能包含 `namespace:Type` 格式的字符串）直接断言为 `DynamicComponentType[]`。命名空间前缀不是有效的 `DynamicComponentType` 值，这违反了类型约定。
- **严重程度**: 🟢 低危
- **修复建议**: 在返回前过滤或剥离命名空间前缀。

### 3.3 🟢 SchemaValidator 无嵌套深度限制

- **文件**: `src/lib/dynamicUI/SchemaValidator.ts`
- **行号**: 27–125
- **描述**: `validateNode` 递归遍历 `children` 数组时未限制最大嵌套深度。恶意或错误的 schema 可能包含极深的嵌套导致调用栈溢出。
- **严重程度**: 🟢 低危
- **修复建议**: 添加深度计数器，超过合理阈值（如 50 层）时报告错误并停止递归。

### 3.4 🟢 `ToolSemanticCheck` useEffect 中使用 setTimeout(0) 反模式

- **文件**: `src/components/settings/ToolSemanticCheck.tsx`
- **行号**: 120–123
- **描述**: `useEffect` 中使用 `setTimeout(() => checkSemanticMatches(selectedTool), 0)` 延迟执行语义检查。此写法无实际异步调度需求，应直接在 effect 中调用。
- **严重程度**: 🟢 低危
- **修复建议**: 直接在 `useEffect` 中调用 `checkSemanticMatches(selectedTool)`，无需 setTimeout。

### 3.5 🟢 连接错误分类检测脆弱

- **文件**: `src/lib/invoke.ts`
- **行号**: 43–52（重试模式）、255–279（诊断记录）、349–356（错误转换）
- **描述**: 三处连接/瞬时错误检测均使用独立的硬编码正则/字符串匹配列表，且三处列表不完全一致。修改其中一处时容易遗漏同步更新其他两处。
- **严重程度**: 🟢 低危
- **修复建议**: 提取统一的错误分类函数，集中管理重试模式、诊断记录和错误转换逻辑。

### 3.6 🟢 `storage.ts` 无容量检查

- **文件**: `src/lib/storage.ts`
- **行号**: 24–31
- **描述**: `storage.set()` 仅在捕获到 `QuotaExceededError` 时记录警告，但未做任何容量预估或降级处理。工具系统状态（toolCalls、agentPool）在长时间运行后可能变得很大，写入 localStorage 时可能导致数据截断丢失。
- **严重程度**: 🟢 低危
- **修复建议**: 在写入前检查序列化后的大小，超过阈值（如 500KB）时发出警告或切换到分片存储。

---

## 四、设计问题与建议

### 4.1 工具执行结果缺少结构化错误模型

在整个工具调用链路（`invoke.ts → executionStore.ts → ToolCallCard.tsx`）中，错误信息以纯字符串形式传递。后端返回的结构化错误（包含错误码、可重试标记、修复建议等）在 `String(e)` 转换后丢失了所有元信息。建议定义统一的 `ToolError` 接口，贯穿整个调用链路保持错误结构。

### 4.2 工具调用缺少统一的审计/追踪机制

当前工具调用的记录分散在 `executionStore`（运行时状态）、`invoke.ts`（IPC 指标）和对话消息（持久化）三个独立系统中，缺少统一的端到端追踪 ID。排查工具调用问题时需要跨系统关联数据，成本很高。

### 4.3 `browserMock.ts` 维护负担

`browserMock.ts` 文件 3157 行，为纯浏览器模式提供后端模拟。新增 Tauri 命令时需同步更新此文件，但无机制保证二者的行为一致性。建议改为自动生成 mock 或使用共享的接口定义。

---

## 附录：审查文件清单

| 文件                        | 路径                                                                  | 行数 |
| --------------------------- | --------------------------------------------------------------------- | ---- |
| invoke.ts                   | `src/lib/invoke.ts`                                                   | 596  |
| codeExecutor.ts             | `src/lib/codeExecutor.ts`                                             | 185  |
| actionRouter.ts             | `src/lib/actionRouter.ts`                                             | 519  |
| skillPermissions.ts         | `src/lib/skillPermissions.ts`                                         | 277  |
| SchemaValidator.ts          | `src/lib/dynamicUI/SchemaValidator.ts`                                | 222  |
| ComponentRegistry.ts        | `src/lib/dynamicUI/ComponentRegistry.ts`                              | 87   |
| skillActionExecutor.ts      | `src/lib/skillActionExecutor.ts`                                      | 53   |
| storage.ts                  | `src/lib/storage.ts`                                                  | 177  |
| executionStore.ts           | `src/stores/feature/executionStore.ts`                                | 866  |
| executionPhaseMachine.ts    | `src/stores/feature/executionPhaseMachine.ts`                         | 41   |
| executionToolCallUtils.ts   | `src/stores/feature/executionToolCallUtils.ts`                        | 34   |
| localToolStore.ts           | `src/stores/feature/localToolStore.ts`                                | 89   |
| recommendationStore.ts      | `src/stores/devtools/recommendationStore.ts`                          | 120  |
| conversationStoreSend.ts    | `src/stores/domain/conversationStoreSend.ts`                          | 1739 |
| localTool.ts                | `src/types/localTool.ts`                                              | 67   |
| dynamicUI.ts                | `src/types/dynamicUI.ts`                                              | 298  |
| index.ts                    | `src/types/index.ts`                                                  | 1574 |
| ToolCallCard.tsx            | `src/components/chat/ToolCallCard.tsx`                                | 301  |
| toolCallDisplay.ts          | `src/components/chat/toolCallDisplay.ts`                              | 62   |
| ToolManager.tsx             | `src/components/settings/ToolManager.tsx`                             | 272  |
| LocalToolSettings.tsx       | `src/components/settings/LocalToolSettings.tsx`                       | 208  |
| ToolSemanticCheck.tsx       | `src/components/settings/ToolSemanticCheck.tsx`                       | 449  |
| ToolRecommendationPanel.tsx | `src/components/recommendation/ToolRecommendationPanel.tsx`           | 222  |
| ToolNode.tsx                | `src/components/workflow/Nodes/ToolNode.tsx`                          | 122  |
| ToolPropertyPanel.tsx       | `src/components/workflow/Panels/PropertyPanels/ToolPropertyPanel.tsx` | 215  |
| browserMock.ts              | `src/lib/browserMock.ts`                                              | 3157 |

---

_报告结束。建议按严重程度优先级（🔴 → 🟡 → 🟢）顺序修复。_
_（内容由AI生成，仅供参考）_
