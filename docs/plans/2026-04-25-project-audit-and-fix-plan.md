# AxAgent 项目全面审计与完善方案

> 审计日期: 2026-04-25
> 范围: 全面架构审查、代码缺陷扫描、测试覆盖分析、安全审计

---

## 一、测试失败问题 (37 个) — 最紧急

### 1.1 `workflowEditorStore.test.ts` — 33 个测试全部失败

**根因**: 导入路径错误
```typescript
// ❌ 错误的导入 (line 70)
const { default: { useWorkflowEditorStore } } = await import('@/stores/feature/workflowEditorStore');

// ✅ 应改为（该 Store 使用 named export）
const { useWorkflowEditorStore } = await import('@/stores/feature/workflowEditorStore');
```

**修复方案**: 统一修改 `workflowEditorStore.test.ts` 中所有 22 个测试用例的导入方式。

### 1.2 `conversationStore.test.ts` — 1 个测试失败

**根因**: Snake_case vs camelCase 命名不一致
- 测试期望 `model_id`, `provider_id`, `system_prompt` (snake_case)
- `createConversation` 实际传入 `modelId`, `providerId`, `systemPrompt` (camelCase)

**修复方案**: 将测试中的期望值更新为 camelCase，或确认 Tauri 命令是否期望 snake_case。

### 1.3 `workflowComponents.test.tsx` — 1 个测试失败

**根因**: `getByText('生成工作流')` 匹配到多个元素（图标按钮 + 文本按钮各一个）

**修复方案**: 改用 `getAllByText` 或使用更精确的选择器（如 `within` 限定范围）。

### 1.4 聚合统计

| 测试文件 | 状态 | 失败数 |
|---------|------|--------|
| workflowEditorStore.test.ts | ❌ 全部失败 | 33 |
| conversationStore.test.ts | ❌ 部分失败 | 1 |
| workflowComponents.test.tsx | ❌ 部分失败 | 1 |
| agentStore.test.ts | ✅ | 0 |
| fileStore.test.ts | ✅ | 0 |
| App.d2.test.tsx | ✅ | 0 |
| ChatPage.test.tsx | ✅ | 0 |
| GatewayPage.test.tsx | ✅ | 0 |
| **总计** | **37 失败** | **37** |

---

## 二、React Hook 违规 (运行时崩溃风险)

### 2.1 `AssistantMarkdown` — ChatView.tsx (~line 1070) — CRITICAL

**问题**: 流式渲染过程中，`singleD2Node` / `hasDeferredHeavyNodes` 状态变化导致返回不同 JSX 树类型 → React Error #310

**修复**（已规划）:
```tsx
// 统一用 <div className="axagent-chat-markdown"> 包裹所有返回路径
// 只变化内部内容，保持外层 JSX 树结构稳定
```

### 2.2 `UserAvatarIcon` — ChatMinimap.tsx (~line 250)

**问题**: 多个 `return` 语句返回不同 JSX 树，当 `avatarType` 变化时触发 hook 违规

**修复**: 使用内联条件表达式统一返回结构：
```tsx
const content = profile.avatarType === 'emoji'
  ? <div>{profile.avatarValue}</div>
  : <Avatar ... />;
return content;
```

---

## 三、安全漏洞 (11 个)

### 3.1 CRITICAL: SQL 注入风险 — context_manager.rs

**问题**: 消息内容直接用字符串匹配检测 marker，未使用元数据标志位。
攻击者可构造含 `<!-- context-clear -->` 的消息内容操纵压缩逻辑。

**修复**: 在数据库消息模型中加入 `is_context_clear` / `is_compression_marker` 布尔字段。

### 3.2 CRITICAL: Base64 解码 OOM 攻击 — conversations.rs

**问题**: 无大小限制直接解码附件 base64 数据，攻击者可发送超大 payload 耗尽内存。

**修复**:
```rust
const MAX_ATTACHMENT_SIZE: usize = 100 * 1024 * 1024; // 100MB
if attachment.data.len() > MAX_ATTACHMENT_SIZE {
    return Err(AxAgentError::Validation("Attachment too large"));
}
```

### 3.3 CRITICAL: 路径遍历 — file_store / indexing

**问题**: `file_path` 来源于数据库或用户输入，未做路径合法性校验，可读取任意文件。

**修复**:
```rust
let full_path = storage_root.join(&file_path);
if !full_path.canonicalize()?.starts_with(storage_root.canonicalize()?) {
    return Err("Path traversal detected");
}
```

### 3.4 HIGH: 主密钥内存残留 — database.rs

**问题**: `Vec` drop 不保证覆盖敏感数据，主密钥可能残留在内存中。

**修复**: 使用 `zeroize` crate 显式清零。

### 3.5 HIGH: Prompt 注入 — agent.rs / conversations.rs

**问题**: 用户自定义 system_prompt 直接拼入提示词模板，可突破标签注入恶意指令。

**修复**: HTML 转义或使用安全分隔符 + 哈希验证。

### 3.6 HIGH: 并发取消竞争 — conversations.rs

**问题**: `cancel_flag` 使用 `Ordering::Relaxed`，可能导致取消信号延迟或丢失。

**修复**: 改用 `Ordering::SeqCst` 或 `Ordering::AcqRel`。

### 3.7 MEDIUM: 指数退避溢出 — indexing.rs

**问题**: `2u64.pow(attempt)` 在 `attempt` 较大时可能 panic。

**修复**: 使用 `saturating_mul` + `checked_pow` + 上限截断（60s）。

### 3.8 MEDIUM: 令牌计数溢出 — context_manager.rs

**问题**: `.sum::<usize>()` 可能溢出，无 `saturating_add` 防护。

**修复**: 使用 `saturating_sum` 或逐元素 `saturating_add`。

### 3.9 MEDIUM: 工具执行缺少权限重验 — agent.rs

**问题**: 工具权限批准后执行前未重新验证。

**修复**: 执行前检查 `always_allowed` 白名单。

### 3.10 MEDIUM: JSON Schema 未校验 — atomic_skills.rs

**问题**: `input_schema` 存储为字符串，无有效性验证，下游可能解析失败。

**修复**: 使用 `jsonschema` crate 编译验证。

### 3.11 LOW: SSRF 风险 — skills.rs

**问题**: GitHub 搜索请求无 URL 白名单校验。

**修复**: 限制请求目标为 `api.github.com`。

---

## 四、前端-后端数据流缺陷

### 4.1 Snake_case vs CamelCase 不一致

**问题**: 前端某些 invoke 调用使用 camelCase 参数名，而 Rust 后端可能期望 snake_case。

**排查清单**:
| 前端调用 | 参数格式 | 后端期望 | 状态 |
|---------|---------|---------|------|
| `createConversation` | camelCase | 待确认 | ❓ |
| `send_message` | snake_case | snake_case | ✅ |
| `update_conversation` | snake_case | snake_case | ✅ |
| `list_messages` | snake_case | snake_case | ✅ |

**修复**: 统一使用 snake_case（与 Rust serde 默认行为一致），或为 Rust 结构体添加 `#[serde(rename_all = "camelCase")]`。

### 4.2 流式缓冲的竞态条件

**问题**: `streamStore.ts` 使用模块级可变状态 (`_streamBuffer`, `_pendingUiChunk` 等)，多会话并发时可能产生竞态。

**场景**: 用户在 A 会话发送消息时切换到 B 会话，再切回 A，缓冲状态可能错乱。

**修复**: 
- 增加 `conversationId` 校验（已部分实现，但不完备）
- 考虑改用 Zustand store 管理缓冲状态

### 4.3 Agent 事件监听内存泄漏

**问题**: `sendAgentMessage` 每次调用都会创建新的 Tauri event listener，如果 `cleanup()` 因异常未被执行可能导致泄漏。

**修复**: 在 `sendAgentMessage` 入口处先清理旧 listener：
```typescript
// 在创建新 listener 前
useStreamStore.getState().stopStreamListening();
```

---

## 五、代码质量问题

### 5.1 流式逻辑重复

`conversationStore.ts` 中 `sendMessage`、`sendAgentMessage`、`regenerateMessage`、`regenerateWithModel`、`sendMultiModelMessage` 都包含相似的流式处理逻辑（占位消息创建、stream store 状态设置、错误处理），代码重复度高。

**修复**: 提取公共的流式处理中间件/辅助函数。

### 5.2 错误处理不一致

| 方法 | 错误模式 | 是否同步到后端 |
|------|---------|--------------|
| `sendMessage` | 设置 error + 保留 temp 消息 | ✅（120ms 后 fetchMessages） |
| `sendAgentMessage` | 设置 error + 保留 temp 消息 | ✅（120ms 后 fetchMessages） |
| `regenerateMessage` | 设置 error | ❌ 未 fetchMessages |
| `toggleMcpServer` | 乐观更新 → 失败回滚 | ✅ |
| 其他偏好设置 | 乐观更新 → 失败回滚 | ✅ |

**修复**: 统一错误处理模式。

### 5.3 类型安全

- `agentStore.ts` 多处使用 `Record<string, unknown>` 和 `any`
- `ToolCallState.input` 类型为 `Record<string, unknown>`，应使用更精确的类型
- 缺少运行时 JSON schema 校验

### 5.4 缺少请求超时

`sendMessage`、`sendAgentMessage` 等长时间操作没有前端超时机制，后端卡住时 UI 会无限等待。

**修复**: 为所有 invoke 调用添加 `AbortController` 超时（建议 5 分钟）。

---

## 六、架构建议

### 6.1 测试策略完善
- 增加流式处理的集成测试
- 增加错误恢复场景测试
- 增加并发（快速切换会话）场景测试
- 使用 `vi.advanceTimersByTime` 测试流式缓冲刷新

### 6.2 错误边界
- 为 ChatView 等重要组件添加 React Error Boundary
- 后端添加全局 panic 恢复中间件
- 统一前端错误提示 UI（当前部分错误仅 console.error）

### 6.3 性能优化
- 大消息列表虚拟化（目前 ChatView 已有 `useVirtualizer`，需确认全覆盖）
- 知识库检索增加查询超时和结果截断
- 附件解码增加内存池复用

### 6.4 监控与日志
- 增加 Tauri 命令调用埋点
- 增加流式传输性能指标（首 token 延迟、吞吐量）
- 统一前后端错误码体系

---

## 七、分阶段完善方案

### 阶段 1 — 紧急修复 (1-2 天)
1. ✅ 修复 `workflowEditorStore.test.ts` 导入路径 → 恢复 33 个测试
2. ✅ 修复 `conversationStore.test.ts` snake_case 问题 → 恢复 1 个测试
3. ✅ 修复 `workflowComponents.test.tsx` 模糊查询 → 恢复 1 个测试
4. ✅ 修复 AssistantMarkdown Hook 违规 → 消除运行时崩溃
5. ✅ 修复 UserAvatarIcon Hook 违规

### 阶段 2 — 安全加固 (3-5 天)
6. ✅ Context 标志位改用元数据布尔字段
7. ✅ Base64 解码增加大小限制
8. ✅ 文件路径遍历防护
9. ✅ 主密钥内存清零 (zeroize)
10. ✅ System prompt 注入防护

### 阶段 3 — 代码重构 (5-7 天)
11. ✅ 提取流式处理公共逻辑
12. ✅ 统一错误处理模式
13. ✅ 增加请求超时机制
14. ✅ Agent 事件监听泄漏修复
15. ✅ 流式缓冲竞态条件修复

### 阶段 4 — 架构增强 (1-2 周)
16. ✅ 增加 React Error Boundary
17. ✅ 完善集成测试套件
18. ✅ 前后端类型一致性检查自动化
19. ✅ 运行时 JSON Schema 校验
20. ✅ 监控埋点基础设施
