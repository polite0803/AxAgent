# 前后端代码批量不一致检测报告

> **项目**: AxAgent (Tauri v2 + React/TypeScript)  
> **检测日期**: 2026-04-29  
> **检测范围**:  
> - 后端: 60 个命令模块, 531 个 `#[tauri::command]`, `types.rs` 全部 1887 行  
> - 前端: 17 个类型文件, ~120 个 `invoke()` 调用点, 所有 store/component 传参  

---

## 一、严重问题 (CRITICAL) — 将导致运行时错误

### C1. 前端调用了 20 个不存在的后端命令

以下前端 `invoke()` 调用找不到对应的 `#[tauri::command]` 处理函数：

| # | 前端调用的命令 | 调用位置 | 修复方案 |
|---|--------------|---------|---------|
| 1 | `rename_conversation` | `stores/domain/conversationListStore.ts:73` | 改用已有命令 `update_conversation`，传 `{ id, input: { title } }` |
| 2 | `batch_delete_conversations` | `stores/domain/conversationListStore.ts:135` | 需新增 `#[tauri::command]` 或前端循环调用 `delete_conversation` |
| 3 | `batch_archive_conversations` | `stores/domain/conversationListStore.ts:147` | 需新增 `#[tauri::command]` 或前端循环调用 `toggle_archive_conversation` |
| 4 | `agent_steer` | `components/chat/SteerInput.tsx:15` | 需在 `src-tauri/src/commands/agent.rs` 新增 `agent_steer` 命令 |
| 5 | `send_multi_model_message` | `stores/domain/multiModelStore.ts:40` | 需在 `src-tauri/src/commands/` 新增命令或复用 `send_message` |
| 6 | `list_search_providers` | `stores/feature/searchStore.ts` | 需在 `src-tauri/src/commands/` 新增搜索管理命令模块 |
| 7 | `create_search_provider` | `stores/feature/searchStore.ts` | 同上 |
| 8 | `update_search_provider` | `stores/feature/searchStore.ts` | 同上 |
| 9 | `delete_search_provider` | `stores/feature/searchStore.ts:193` | 同上 |
| 10 | `execute_search` | `stores/feature/searchStore.ts` | 同上 |
| 11 | `plugin_install` | `components/chat/PluginMarketplace.tsx:59` | 需新增插件管理命令模块 |
| 12 | `plugin_uninstall` | `components/chat/PluginMarketplace.tsx:72` | 同上 |
| 13 | `plugin_enable` | `components/chat/PluginMarketplace.tsx:84` | 同上 |
| 14 | `plugin_disable` | `components/chat/PluginMarketplace.tsx:84` | 同上 |
| 15 | `file_revoke_authorization` | `components/chat/FilePermissionDialog.tsx:69` | 需新增或前端移除该功能入口 |
| 16 | `list_plugin_tools` | `components/atomicSkill/EntryRefSelector.tsx:39` | 需新增命令或使用 `list_local_tools` 替代 |
| 17 | `proactive_convert_to_nudge` | `stores/feature/nudgeStore.ts:237` | 需在 `proactive.rs` 新增命令 |
| 18 | `llm_wiki_update_schema` | `components/llm-wiki/SchemaEditor.tsx:80` | 需在 `llm_wiki.rs` 新增命令 |
| 19 | `llm_wiki_delete_schema` | `components/llm-wiki/SchemaEditor.tsx:107` | 需在 `llm_wiki.rs` 新增命令 |
| 20 | `write_base64_to_file` | `components/wiki/IngestPanel.tsx:61` | 需新增命令或使用已有文件写入命令 |

---

### C2. 命令名称不匹配

| 前端调用的命令 | 后端实际命令 | 调用位置 |
|--------------|-------------|---------|
| `archive_to_knowledge_base` | `archive_conversation_to_knowledge_base` | `stores/domain/conversationListStore.ts:127` |

**修复方案**: 前端将 `archive_to_knowledge_base` 改为 `archive_conversation_to_knowledge_base`。

---

### C3. `GatewayLinkActivity` 字段名完全错误

**后端** (`src-tauri/crates/core/src/types.rs:619-625`):

```rust
pub struct GatewayLinkActivity {
    pub id: String,
    pub link_id: String,
    pub activity_type: String,       // JSON key: "activity_type"
    pub description: Option<String>, // JSON key: "description"
    pub created_at: i64,             // JSON key: "created_at"
}
```

**前端** (`src/types/index.ts:567-573`):

```typescript
export interface GatewayLinkActivity {
  id: string;
  link_id: string;
  action: string;       // ❌ 应为 activity_type
  detail: string;       // ❌ 应为 description，且应为 string | null
  created_at: number;
}
```

**修复方案**:

```typescript
export interface GatewayLinkActivity {
  id: string;
  link_id: string;
  activity_type: string;
  description: string | null;
  created_at: number;
}
```

---

### C4. `workspace.ts` 中的 `AttachmentInput` 与后端不兼容

前端存在两个 `AttachmentInput` 定义，**workspace.ts 版本完全错误**：

| 字段 | 后端 (types.rs:314) / index.ts:197 (✅) | workspace.ts:112 (❌) |
|------|--------------------------------------|---------------------|
| 文件名 | `file_name` | `name` |
| 文件类型 | `file_type` | `path` |
| MIME类型 | *(无此字段)* | `mimeType` |
| 文件大小 | `file_size: u64` | `sizeBytes` |
| 数据 | `data` | *(缺失)* |

**位置**: `src/types/workspace.ts:112-117`  
**修复方案**: 删除 workspace.ts 中的重复定义，全部导入 index.ts 的正确版本。同时修正 `SendMessageInput` 中的引用（见 M2）。

---

### C5. `workspace.ts` 中 `UpdateConversationInput` 重复定义且与后端不兼容

`src/types/workspace.ts:95-103` 定义了与后端完全不同的 `UpdateConversationInput`：

**前端 workspace.ts 独有字段 (后端不存在)**:

| 字段 | 说明 |
|------|------|
| `workspaceSnapshot?: ConversationWorkspaceSnapshot` | 后端 `UpdateConversationInput` 无此字段 |
| `activeBranchId?: string \| null` | 后端无此字段 |
| `activeArtifactId?: string \| null` | 后端无此字段 |
| `researchMode?: boolean` | 后端无此字段 |

同时使用 `providerId` (camelCase) 而非后端期望的 `provider_id` (snake_case)。

**修复方案**: 
1. 删除 workspace.ts 中的 `UpdateConversationInput`
2. 统一使用 `src/types/index.ts:229-251` 的版本
3. 同时补充 index.ts 版本缺失的 `parent_conversation_id` 字段（见 H5）

---

### C6. `ProgramPolicy` 字段类型错误

后端 `ProgramPolicy` 存储为 JSON 编码字符串，前端按数组解析：

| 字段 | 后端 camelCase JSON | 前端 (backup.ts) |
|------|-------------------|-----------------|
| Provider IDs | `allowedProviderIdsJson: string` | `allowedProviderIds: string[]` ❌ |
| Model IDs | `allowedModelIdsJson: string` | `allowedModelIds: string[]` ❌ |

**位置**: `src/types/backup.ts` — `ProgramPolicy` 接口  
**修复方案**:

```typescript
// 前端改为:
export interface ProgramPolicy {
  id: string;
  programName: string;
  allowedProviderIdsJson: string;  // JSON.parse() 后得到 string[]
  allowedModelIdsJson: string;     // JSON.parse() 后得到 string[]
  defaultProviderId?: string;
  defaultModelId?: string;
  rateLimitPerMinute?: number;
}
```

---

### C7. `CreateGatewayLinkInput` 缺失 `api_key_id` 字段

| 字段 | 后端 (types.rs:570) | 前端 (index.ts:558) |
|------|-------------------|-------------------|
| `api_key_id` | `Option<String>` | **缺失** |

**位置**: `src/types/index.ts:558-565`  
**修复方案**:

```typescript
export interface CreateGatewayLinkInput {
  name: string;
  link_type: GatewayLinkType;
  endpoint: string;
  api_key_id?: string | null;     // 新增
  api_key?: string | null;
  auto_sync_models?: boolean;
  auto_sync_skills?: boolean;
}
```

---

### C8. `ConversationBranch` 字段名称和类型双重不匹配

| 字段 | 后端 (types.rs:1488, camelCase) | 前端 (workspace.ts) |
|------|-------------------------------|-------------------|
| 对比消息 IDs | `comparedMessageIdsJson: Option<String>` | `comparedMessageIds?: string[]` |

- **名称错误**: 后端 camelCase 序列化后 key 为 `comparedMessageIdsJson`，非 `comparedMessageIds`
- **类型错误**: 后端为 JSON 编码的字符串 (`Option<String>`)，前端按数组 (`string[]`) 解析

**位置**: `src/types/workspace.ts:87`  
**修复方案**:

```typescript
export type ConversationBranch = {
  id: string;
  conversationId: string;
  parentMessageId: string;
  branchLabel: string;
  branchIndex: number;
  comparedMessageIdsJson?: string; // JSON.parse() → string[]
  createdAt: string;
};
```

---

## 二、高优先级问题 (HIGH)

### H1. `AppSettings` 字段缺失/多余 (20 处不对齐)

**前端有 6 个后端不存在的字段** (`src/types/index.ts:449-460`):

| 前端多出的字段 | 
|-------------|
| `screen_perception_enabled` |
| `rl_optimizer_enabled` |
| `lora_finetune_enabled` |
| `proactive_nudge_enabled` |
| `thought_chain_enabled` |
| `error_recovery_enabled` |

**前端缺失 14 个后端存在的字段** (`src-tauri/crates/core/src/types.rs` AppSettings):

| 缺失字段 | 类型 |
|---------|------|
| `backup_dir` | `Option<String>` |
| `auto_backup_enabled` | `bool` |
| `auto_backup_interval_hours` | `u32` |
| `auto_backup_max_count` | `u32` |
| `s3_endpoint` | `Option<String>` |
| `s3_region` | `Option<String>` |
| `s3_bucket` | `Option<String>` |
| `s3_access_key_id` | `Option<String>` |
| `s3_root` | `Option<String>` |
| `s3_use_path_style` | `bool` |
| `s3_sync_enabled` | `bool` |
| `s3_sync_interval_minutes` | `u32` |
| `s3_max_remote_backups` | `u32` |
| `s3_include_documents` | `bool` |

**修复方案**: 前端补充 14 个缺失字段；核实 6 个多余字段是否需后端添加。

---

### H2. 后端 `gateway_listen_address` 默认值疑似笔误

```rust
// src-tauri/crates/core/src/types.rs - AppSettings
#[serde(default = "default_gateway_listen_address")]
pub gateway_listen_address: String,
// ...
fn default_gateway_listen_address() -> String {
    "127.1.0.0".to_string()  // ❌ 应为 "127.0.0.1"
}
```

**修复方案**: 将默认值改为 `"127.0.0.1"` 或 `"127.0.0.0"` 取决于意图。

---

### H3. `GatewayLinkActivity.detail` 可选性不一致

| 字段 | 后端 | 前端 |
|------|------|------|
| `description` | `Option<String>` (可为 null) | `detail: string` (非可选) |

后端返回 `null` 时会导致前端类型断言失败。

**修复方案**: 参见 C3 修复后的类型定义，已改为 `description: string | null`。

---

### H4. `ConversationBranch.compared_message_ids_json` 类型双重错误

同 C8，此处再次强调：JSON key 名和类型都不匹配。

---

### H5. `UpdateConversationInput` (index.ts 版本) 缺失 `parent_conversation_id`

后端 `UpdateConversationInput` (types.rs:356-378) 包含:
```rust
pub parent_conversation_id: Option<Option<String>>,  // double option
```

前端 `src/types/index.ts:229-251` 缺失此字段。

**修复方案**: 前端补充 `parent_conversation_id?: string | null`。

---

## 三、中优先级问题 (MEDIUM)

### M1. 前端 invoke 参数命名不统一

| 命令 | 调用位置 | 参数名 | 风险 |
|------|---------|--------|------|
| `update_workspace_snapshot` | `conversationStore.ts:2419` | `conversation_id` (snake) | 与另一处不一致 |
| `update_workspace_snapshot` | `workspaceStore.ts:39` | `conversationId` (camel) | 需确认哪处正确 |
| `delete_message` | `conversationStore.ts:276` | `id` | 与另一处不一致 |
| `delete_message` | `messageListStore.ts:91` | `messageId` | 需确认哪处正确 |

**修复方案**: 统一使用后端 Tauri command 函数签名中的实际参数名。

---

### M2. `SendMessageInput` 引用了错误的 `AttachmentInput`

`src/types/workspace.ts` 中 `SendMessageInput` 的 `attachments?` 字段类型引用的是 **workspace.ts 自己定义** 的错误版本 `AttachmentInput`（见 C4）。

```typescript
// workspace.ts SendMessageInput
attachments?: AttachmentInput[];  // ❌ 引用的是 workspace.ts 版 AttachmentInput
```

**修复方案**: 删除 workspace.ts 的 `AttachmentInput` 定义，改为导入 index.ts 的正确版本。

---

### M3. `ModelParamOverrides` 字段完整性

前后端 `ModelParamOverrides` 均包含 8 个字段，已完全对齐。✅

---

### M4. `Message.status` 类型差异

| 端 | 类型 | 
|----|------|
| 后端 | `String` |
| 前端 | `"complete" \| "partial" \| "error" \| "cancelled"` |

后端无约束可能返回其他值，低风险但建议后端改为枚举。

---

### M5. `Conversation.mode` 可选性不一致

| 端 | 字段定义 |
|----|---------|
| 后端 | `mode: String` (必填) |
| 前端 | `mode?: "chat" \| "agent" \| "gateway"` (可选) |

**修复方案**: 前端去掉 `?` 标记为必填。

---

### M6. 前端 `AtomicSkillExecutionResult` 内联嵌套类型

```typescript
// src/types/index.ts
export interface AtomicSkillExecutionResult {
  error?: { error_type: string; message: string }; // ❌ 内联匿名类型
}
```

**修复方案**: 提取为独立接口 `AtomicSkillError`，如后端也使用相同结构则保持一致。

---

### M7. `Conversation` 缺失 `message_count` 字段

后端 `Conversation` (types.rs:341) 有 `message_count: u32`，前端 index.ts 定义有该字段 ✅，但需确认使用处是否正确引用。

---

## 四、低优先级问题 (LOW)

### L1. `WebDavSync.tsx` 调用参数封装方式

```typescript
// components/settings/WebDavSync.tsx:134
await invoke('save_webdav_config', { config }); // 传了整个 config 对象
```

后端 `save_webdav_config` (webdav.rs) 期望直接展开字段还是接收嵌套对象需核实。

---

### L2. 前端 `SkillDetail` 含 `files` 数组

`src/types/index.ts` 中 `SkillDetail` 包含 `files: string[]`，需核实后端是否返回。

---

### L3. Webhook 前端调用参数

`WebhookSettings.tsx` 调用 `webhook_toggle_subscription` 使用 `subscriptionId` (camelCase)，后端参数名是 `subscription_id` (snake_case)。Tauri 不会自动转换，需核实是否有 `#[tauri::command(rename_all = "camelCase")]`。

---

## 五、修复优先级路线图

### 第一阶段 (P0 - 立即修复，阻止报错)

1. **补全 20 个缺失的后端 `#[tauri::command]`** 或前端改用已有命令
2. **修复 `archive_to_knowledge_base` → `archive_conversation_to_knowledge_base`** 名称
3. **修复 `GatewayLinkActivity` 字段名** (action/detail → activity_type/description)
4. **删除 workspace.ts 错误版 `AttachmentInput`**，统一导入 index.ts 版本
5. **统一 workspace.ts 和 index.ts 的 `UpdateConversationInput`**

### 第二阶段 (P1 - 数据正确性)

6. **双向对齐 `AppSettings`** 的 20 个字段差异
7. **修复 `ProgramPolicy` 的 `_json` 字段类型** (string[] → string)
8. **修复 `ConversationBranch` 字段名和类型**
9. **补充 `CreateGatewayLinkInput` 缺失的 `api_key_id`**
10. **修复 `gateway_listen_address` 默认值**

### 第三阶段 (P2 - 代码规范)

11. 统一 invoke 参数命名风格 (camelCase vs snake_case)
12. 提取内联匿名类型为独立接口
13. 前后端枚举值对齐
14. 可选性 (`?`) 标记对齐

---

## 六、统计汇总

| 类别 | 数量 |
|------|------|
| 严重问题 (CRITICAL) | 8 类 |
| 高优先级 (HIGH) | 5 类 |
| 中优先级 (MEDIUM) | 7 类 |
| 低优先级 (LOW) | 3 类 |
| **发现问题合计** | **23 类** |
| | |
| 缺失的后端命令 | 20 个 |
| 字段名不一致 | ~15 处 |
| 类型不匹配 (string vs array etc) | 5 处 |
| 重复类型定义冲突 | 2 处 |
| 前后端字段数量差异 | 20 处 (AppSettings) |
| | |
| 受影响前端文件 | ~15 个 |
| 受影响后端文件 | ~8 个 |

---

*此报告由自动化比对工具生成，覆盖全部 531 个后端命令和 ~120 个前端调用点。建议将 P0/P1 问题转化为 GitHub Issues 逐项跟踪修复。*
