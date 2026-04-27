# AxAgent 改进方案实施指南

> 文档版本：v1.0.0
> 创建日期：2026-04-28
> 状态：待实施

---

## 一、文档概述

### 1.1 目的

本文档基于 AxAgent 项目技术架构分析报告，制定详细的编码实现方案，用于指导后续改进工作的落地实施。

### 1.2 改进项清单

| 序号 | 类别 | 改进项 | 优先级 | 工作量 |
|------|------|--------|--------|--------|
| 1 | 架构 | 引入 Trait Bounds 替代动态分发 | P1 | 中 |
| 2 | 架构 | 统一事件系统 | P2 | 中 |
| 3 | 架构 | 前端 Store 拆分 | P2 | 中 |
| 4 | 性能 | 数据库连接池调优 | P1 | 小 |
| 5 | 性能 | 引入向量缓存层 | P2 | 中 |
| 6 | 性能 | 工具执行结果缓存 | P2 | 中 |
| 7 | 安全 | 工具执行白名单 | P1 | 小 |
| 8 | 安全 | 命令注入防护 | P1 | 小 |
| 9 | 可维护性 | 统一配置管理 | P2 | 中 |
| 10 | 可维护性 | 统一错误处理 | P2 | 中 |

---

## 二、架构优化实施

### 2.1 引入 Trait Bounds 替代动态分发

#### 2.1.1 当前问题

当前 [coordinator.rs](file:///run/user/1000/gvfs/smb-share:server=hustniu.local,share=onemanager/AxAgent/src-tauri/crates/agent/src/coordinator.rs) 使用 `Arc<std::sync::Mutex<dyn AgentImpl>>` 实现动态分发，导致：

- 编译时无法检查接口兼容性
- 运行时错误难以定位
- 性能损耗（锁竞争）

#### 2.1.2 实施步骤

**Step 1: 定义 Agent Trait**

在 `agent/src/traits.rs`（新建文件）中定义：

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not initialized")]
    NotInitialized,
    #[error("Agent already running")]
    AlreadyRunning,
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub timeout_secs: Option<u64>,
    pub enable_self_verification: bool,
    pub enable_error_recovery: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            timeout_secs: Some(300),
            enable_self_verification: false,
            enable_error_recovery: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub content: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorOutput {
    pub content: String,
    pub status: AgentStatus,
    pub iterations: usize,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Initializing,
    Running,
    WaitingForConfirmation,
    Paused,
    Completed,
    Failed(String),
}

#[async_trait]
pub trait Agent: Send + Sync {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError>;
    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError>;
    async fn pause(&mut self) -> Result<(), AgentError>;
    async fn resume(&mut self) -> Result<(), AgentError>;
    async fn cancel(&mut self) -> Result<(), AgentError>;
    fn status(&self) -> AgentStatus;
    fn agent_type(&self) -> &'static str;
}
```

**Step 2: 修改 Coordinator 实现**

修改 `agent/src/coordinator.rs`：

```rust
// 删除原有的 dyn AgentImpl 相关代码
// 修改为泛型约束

pub struct UnifiedAgentCoordinator<T: Agent> {
    status: Arc<RwLock<AgentStatus>>,
    config: Arc<RwLock<AgentConfig>>,
    implementation: Arc<T>,  // 泛型替代 dyn
    event_bus: Arc<AgentEventBus>,
    correlation_counter: std::sync::atomic::AtomicU64,
}

impl<T: Agent> UnifiedAgentCoordinator<T> {
    pub fn new(implementation: Arc<T>, event_bus: Option<Arc<AgentEventBus>>) -> Self {
        let event_bus = event_bus.unwrap_or_else(|| Arc::new(AgentEventBus::new("coordinator")));

        Self {
            status: Arc::new(RwLock::new(AgentStatus::Idle)),
            config: Arc::new(RwLock::new(AgentConfig::default())),
            implementation,
            event_bus,
            correlation_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub async fn execute(&self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        let mut status = self.status.write().await;
        
        if *status == AgentStatus::Running {
            return Err(AgentError::AlreadyRunning);
        }

        if !matches!(*status, AgentStatus::Idle | AgentStatus::Paused) {
            return Err(AgentError::InvalidState(format!(
                "Cannot execute from status {}",
                status
            )));
        }

        *status = AgentStatus::Running;
        drop(status);

        let result = self.implementation.execute(input).await;

        let mut status = self.status.write().await;
        match &result {
            Ok(output) => {
                *status = output.status.clone();
            }
            Err(e) => {
                *status = AgentStatus::Failed(e.to_string());
            }
        }

        result
    }

    // ... 其他方法保持类似结构
}
```

**Step 3: 更新 lib.rs 导出**

修改 `agent/src/lib.rs`：

```rust
pub mod traits;  // 新增

pub use traits::{
    Agent, AgentConfig, AgentError, AgentInput, AgentStatus, CoordinatorOutput,
};
// ... 保留其他导出
```

#### 2.1.3 验收标准

- [ ] `cargo check` 通过
- [ ] 所有调用 `UnifiedAgentCoordinator` 的地方需要显式指定类型参数
- [ ] 运行时性能提升 5-10%（通过减少锁竞争）

---

### 2.2 统一事件系统

#### 2.2.1 当前问题

项目存在两套事件系统：
- `event_bus.rs` - AgentEventBus, EventSubscription
- `event_emitter.rs` - AgentPermissionPayload

#### 2.2.2 实施步骤

**Step 1: 创建统一事件类型**

在 `agent/src/events.rs`（新建文件）：

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentEventType {
    TurnStarted,
    TurnCompleted,
    ToolUse,
    ToolResult,
    ToolError,
    StateChanged,
    IterationComplete,
    ChainComplete,
    ResearchPhaseChanged,
    SourceFound,
    CitationAdded,
    ReportGenerated,
    Error,
    Warning,
    Debug,
    LlmGenerationStarted,
    LlmGenerationCompleted,
    PermissionRequest,
    PermissionGranted,
    PermissionDenied,
}

impl std::fmt::Display for AgentEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub event_type: AgentEventType,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
}

impl AgentEvent {
    pub fn new(source: impl Into<String>, event_type: AgentEventType, payload: serde_json::Value) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            source: source.into(),
            payload,
            correlation_id: None,
        }
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct EventSubscription {
    pub event_types: Vec<AgentEventType>,
    pub receiver: tokio::sync::broadcast::Receiver<AgentEvent>,
}

#[derive(Debug)]
pub struct UnifiedEventBus {
    sender: tokio::sync::broadcast::Sender<AgentEvent>,
    subscriptions: tokio::sync::RwLock<HashMap<String, EventSubscription>>,
    name: String,
}

impl UnifiedEventBus {
    pub fn new(name: impl Into<String>) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(1000);
        Self {
            sender,
            subscriptions: tokio::sync::RwLock::new(HashMap::new()),
            name: name.into(),
        }
    }

    pub async fn subscribe(&self, id: &str, event_types: Vec<AgentEventType>) -> EventSubscription {
        let receiver = self.sender.subscribe();
        let subscription = EventSubscription {
            event_types,
            receiver,
        };

        let mut subs = self.subscriptions.write().await;
        subs.insert(id.to_string(), subscription.clone());

        subscription
    }

    pub async fn unsubscribe(&self, id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(id);
    }

    pub fn emit(&self, event: AgentEvent) {
        let _ = self.sender.send(event);
    }

    pub fn try_emit(&self, event: AgentEvent) -> bool {
        self.sender.send(event).is_ok()
    }
}
```

**Step 2: 删除旧的 event_emitter.rs**

将 `event_emitter.rs` 中的 `AgentPermissionPayload` 迁移到新的事件系统后，删除该文件。

**Step 3: 更新依赖方**

需要更新以下文件的引用：

```bash
# 搜索使用 event_emitter 的文件
grep -r "event_emitter" src-tauri/crates/agent/src/
```

#### 2.2.3 验收标准

- [ ] 所有事件类型统一使用 `UnifiedEventBus`
- [ ] 删除 `event_emitter.rs` 文件
- [ ] `cargo check` 通过

---

### 2.3 前端 Store 拆分

#### 2.3.1 当前问题

[conversationStore.ts](file:///run/user/1000/gvfs/smb-share:server=hustniu.local,share=onemanager/AxAgent/src/stores/domain/conversationStore.ts) 包含 50+ 个状态字段和方法，违反单一职责原则。

#### 2.3.2 拆分方案

```
conversationStore.ts → 拆分为：
├── conversationListStore.ts    // 对话列表管理
├── messageStore.ts            // 消息操作
├── streamingStore.ts          // 流式响应状态
└── uiStateStore.ts            // UI 状态（loading, error 等）
```

#### 2.3.3 实施步骤

**Step 1: 创建 conversationListStore.ts**

```typescript
// src/stores/domain/conversationListStore.ts
import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import type { Conversation, UpdateConversationInput } from '@/types';

interface ConversationListState {
  conversations: Conversation[];
  activeConversationId: string | null;
  totalActiveCount: number;
  
  fetchConversations: () => Promise<void>;
  setActiveConversation: (id: string | null) => void;
  createConversation: (
    title: string,
    model_id: string,
    providerId: string,
    options?: { categoryId?: string | null; scenario?: string | null }
  ) => Promise<Conversation>;
  updateConversation: (id: string, input: UpdateConversationInput) => Promise<void>;
  renameConversation: (id: string, title: string) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  togglePin: (id: string) => Promise<void>;
  toggleArchive: (id: string) => Promise<void>;
}

export const useConversationListStore = create<ConversationListState>((set, get) => ({
  conversations: [],
  activeConversationId: null,
  totalActiveCount: 0,

  fetchConversations: async () => {
    set({ loading: true, error: null });
    try {
      const conversations = await invoke<Conversation[]>('list_conversations');
      set({ conversations, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  setActiveConversation: (id) => set({ activeConversationId: id }),

  createConversation: async (title, model_id, providerId, options) => {
    const conversation = await invoke<Conversation>('create_conversation', {
      title,
      model_id,
      provider_id: providerId,
      category_id: options?.categoryId,
      scenario: options?.scenario,
    });
    set((state) => ({
      conversations: [conversation, ...state.conversations],
    }));
    return conversation;
  },

  updateConversation: async (id, input) => {
    await invoke('update_conversation', { id, input });
    set((state) => ({
      conversations: state.conversations.map((c) =>
        c.id === id ? { ...c, ...input } : c
      ),
    }));
  },

  renameConversation: async (id, title) => {
    await get().updateConversation(id, { title });
  },

  deleteConversation: async (id) => {
    await invoke('delete_conversation', { id });
    set((state) => ({
      conversations: state.conversations.filter((c) => c.id !== id),
      activeConversationId: state.activeConversationId === id ? null : state.activeConversationId,
    }));
  },

  togglePin: async (id) => {
    const conv = get().conversations.find((c) => c.id === id);
    if (conv) {
      await get().updateConversation(id, { pinned: !conv.pinned });
    }
  },

  toggleArchive: async (id) => {
    const conv = get().conversations.find((c) => c.id === id);
    if (conv) {
      await get().updateConversation(id, { archived: !conv.archived });
    }
  },
}));
```

**Step 2: 创建 messageStore.ts**

```typescript
// src/stores/domain/messageStore.ts
import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import type { Message, AttachmentInput } from '@/types';

interface MessageState {
  messages: Message[];
  loading: boolean;
  loadingOlder: boolean;
  hasOlderMessages: boolean;
  oldestLoadedMessageId: string | null;
  streamingMessageId: string | null;

  fetchMessages: (conversationId: string, preserveMessageIds?: string[]) => Promise<void>;
  loadOlderMessages: () => Promise<void>;
  sendMessage: (content: string, attachments?: AttachmentInput[]) => Promise<void>;
  deleteMessage: (messageId: string) => Promise<void>;
  updateMessageContent: (messageId: string, content: string) => Promise<void>;
}

const MESSAGE_PAGE_SIZE = 50;

export const useMessageStore = create<MessageState>((set, get) => ({
  messages: [],
  loading: false,
  loadingOlder: false,
  hasOlderMessages: false,
  oldestLoadedMessageId: null,
  streamingMessageId: null,

  fetchMessages: async (conversationId, preserveMessageIds) => {
    set({ loading: true });
    try {
      const result = await invoke<{ messages: Message[]; has_more: boolean }>('get_messages', {
        conversation_id: conversationId,
        limit: MESSAGE_PAGE_SIZE,
      });

      set({
        messages: result.messages,
        hasOlderMessages: result.has_more,
        oldestLoadedMessageId: result.messages[result.messages.length - 1]?.id ?? null,
        loading: false,
      });
    } catch (error) {
      set({ loading: false, error: String(error) });
    }
  },

  loadOlderMessages: async () => {
    const { oldestLoadedMessageId } = get();
    if (!oldestLoadedMessageId) return;

    set({ loadingOlder: true });
    try {
      const result = await invoke<{ messages: Message[]; has_more: boolean }>('get_messages', {
        before_id: oldestLoadedMessageId,
        limit: MESSAGE_PAGE_SIZE,
      });

      set((state) => ({
        messages: [...state.messages, ...result.messages],
        hasOlderMessages: result.has_more,
        oldestLoadedMessageId: result.messages[result.messages.length - 1]?.id ?? null,
        loadingOlder: false,
      }));
    } catch (error) {
      set({ loadingOlder: false });
    }
  },

  sendMessage: async (content, attachments = []) => {
    const activeId = useConversationListStore.getState().activeConversationId;
    if (!activeId) return;

    await invoke('send_message', {
      conversation_id: activeId,
      content,
      attachments,
    });
  },

  deleteMessage: async (messageId) => {
    await invoke('delete_message', { message_id: messageId });
    set((state) => ({
      messages: state.messages.filter((m) => m.id !== messageId),
    }));
  },

  updateMessageContent: async (messageId, content) => {
    await invoke('update_message', { message_id: messageId, content });
    set((state) => ({
      messages: state.messages.map((m) =>
        m.id === messageId ? { ...m, content } : m
      ),
    }));
  },
}));

// 导入依赖
import { useConversationListStore } from './conversationListStore';
```

**Step 3: 创建 streamingStore.ts**

```typescript
// src/stores/domain/streamingStore.ts
import { create } from 'zustand';
import { invoke, listen } from '@/lib/invoke';

interface StreamChunk {
  messageId: string;
  content: string;
  delta: string;
}

interface StreamingState {
  isStreaming: boolean;
  streamBuffer: Map<string, string>;
  currentStreamingMessageId: string | null;

  startStream: (messageId: string) => void;
  appendChunk: (chunk: StreamChunk) => void;
  stopStream: (messageId: string) => void;
  clearBuffer: (messageId: string) => void;
}

export const useStreamingStore = create<StreamingState>((set, get) => ({
  isStreaming: false,
  streamBuffer: new Map(),
  currentStreamingMessageId: null,

  startStream: (messageId) => {
    set((state) => {
      state.streamBuffer.set(messageId, '');
      return {
        isStreaming: true,
        currentStreamingMessageId: messageId,
      };
    });
  },

  appendChunk: (chunk) => {
    set((state) => {
      const current = state.streamBuffer.get(chunk.messageId) || '';
      state.streamBuffer.set(chunk.messageId, current + chunk.delta);
      return { streamBuffer: new Map(state.streamBuffer) };
    });
  },

  stopStream: (messageId) => {
    set((state) => {
      const isLast = state.currentStreamingMessageId === messageId;
      return {
        isStreaming: !isLast && state.streamBuffer.size > 1,
        currentStreamingMessageId: isLast ? null : state.currentStreamingMessageId,
      };
    });
  },

  clearBuffer: (messageId) => {
    set((state) => {
      state.streamBuffer.delete(messageId);
      return { streamBuffer: new Map(state.streamBuffer) };
    });
  },
}));
```

**Step 4: 创建 uiStateStore.ts**

```typescript
// src/stores/domain/uiStateStore.ts
import { create } from 'zustand';

interface UIState {
  error: string | null;
  isSettingsOpen: boolean;
  sidebarCollapsed: boolean;
  
  setError: (error: string | null) => void;
  setSettingsOpen: (open: boolean) => void;
  toggleSidebar: () => void;
}

export const useUIStateStore = create<UIState>((set) => ({
  error: null,
  isSettingsOpen: false,
  sidebarCollapsed: false,

  setError: (error) => set({ error }),

  setSettingsOpen: (open) => set({ isSettingsOpen: open }),

  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
}));
```

**Step 5: 更新原 conversationStore.ts**

将原 `conversationStore.ts` 简化为委托模式：

```typescript
// src/stores/domain/conversationStore.ts (简化版)
export { useConversationListStore } from './conversationListStore';
export { useMessageStore } from './messageStore';
export { useStreamingStore } from './streamingStore';
export { useUIStateStore } from './uiStateStore';

// 保持向后兼容的委托 Store
import { create } from 'zustand';
import { useConversationListStore, useMessageStore, useStreamingStore, useUIStateStore } from './index';

export const useConversationStore = create((set, get) => ({
  // 委托给子 Store
  get conversations() { return useConversationListStore.getState().conversations; },
  get activeConversationId() { return useConversationListStore.getState().activeConversationId; },
  // ... 其他委托
}));
```

#### 2.3.4 验收标准

- [ ] 所有原有 `conversationStore` 的功能正常工作
- [ ] TypeScript 类型检查通过
- [ ] Store 文件行数减少 70%

---

## 三、性能优化实施

### 3.1 数据库连接池调优

#### 3.1.1 修改位置

[db.rs](file:///run/user/1000/gvfs/smb-share:server=hustniu.local,share=onemanager/AxAgent/src-tauri/crates/core/src/db.rs)

#### 3.1.2 实施步骤

```rust
// db.rs 修改
pub async fn create_pool(db_path: &str) -> Result<DbHandle> {
    let url = if db_path.starts_with("sqlite:") {
        format!("{}?mode=rwc", db_path)
    } else {
        format!("sqlite:{}?mode=rwc", db_path)
    };

    let mut opt = ConnectOptions::new(&url);
    opt.max_connections(20)         // 从 5 增加到 20
        .min_connections(5)          // 从 1 增加到 5
        .acquire_timeout(std::time::Duration::from_secs(30))
        .sqlx_logging(false);

    let conn = Database::connect(opt).await?;
    // ... 其余代码不变
}
```

#### 3.1.3 验收标准

- [ ] `cargo check` 通过
- [ ] 并发场景下无连接超时错误

---

### 3.2 引入向量缓存层

#### 3.2.1 修改位置

新建 `core/src/vector_cache.rs`，修改 `core/src/hybrid_search.rs`

#### 3.2.2 实施步骤

**Step 1: 创建 vector_cache.rs**

```rust
// core/src/vector_cache.rs
use lru::LruCache;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct VectorCache {
    cache: Arc<Mutex<LruCache<String, Vec<f32>>>>,
    ttl_secs: u64,
    max_size: usize,
}

impl VectorCache {
    pub fn new(max_size: usize, ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(max_size))),
            ttl_secs,
            max_size,
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<f32>> {
        let mut cache = self.cache.lock().await;
        cache.get(key).cloned()
    }

    pub async fn insert(&self, key: String, value: Vec<f32>) {
        let mut cache = self.cache.lock().await;
        if cache.len() >= self.max_size {
            // LRU 会自动淘汰最老的条目
        }
        cache.put(key, value);
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    pub async fn remove(&self, key: &str) {
        let mut cache = self.cache.lock().await;
        cache.pop(key);
    }
}

impl Default for VectorCache {
    fn default() -> Self {
        Self::new(10000, 3600) // 默认 10000 条，最大 TTL 1小时
    }
}
```

**Step 2: 修改 hybrid_search.rs**

```rust
// core/src/hybrid_search.rs
pub struct HybridSearch {
    vector_store: Arc<VectorStore>,
    reranker: Arc<Reranker>,
    cache: Arc<VectorCache>,  // 新增
}

impl HybridSearch {
    pub fn new(vector_store: VectorStore, reranker: Reranker) -> Self {
        Self {
            vector_store: Arc::new(vector_store),
            reranker: Arc::new(reranker),
            cache: Arc::new(VectorCache::default()),  // 新增
        }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        // 生成查询向量
        let cache_key = format!("search:{}", query);
        
        // 先查缓存
        if let Somecached_vector) = self.cache.get(&cache_key).await {
            let cached_vector = cached_vector;
            return self.search_with_vector(&cached_vector, query, limit).await;
        }

        // 缓存未命中，生成新向量
        let query_vector = self.vector_store.embed(query).await?;
        
        // 存入缓存
        self.cache.insert(cache_key, query_vector.clone()).await;

        self.search_with_vector(&query_vector, query, limit).await
    }

    async fn search_with_vector(
        &self,
        vector: &[f32],
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        // ... 原有搜索逻辑
    }
}
```

**Step 3: 添加依赖**

在 `core/Cargo.toml` 中添加：

```toml
lru = "0.12"
```

#### 3.2.3 验收标准

- [ ] `cargo check` 通过
- [ ] 相同查询第二次执行响应时间减少 50%

---

### 3.3 工具执行结果缓存

#### 3.3.1 修改位置

修改 `agent/src/tool_registry.rs`

#### 3.3.2 实施步骤

```rust
// agent/src/tool_registry.rs

use lru::LruCache;

#[derive(Clone)]
pub struct ToolExecutionCache {
    cache: Arc<tokio::sync::Mutex<LruCache<String, CachedToolResult>>>,
    ttl: std::time::Duration,
}

#[derive(Clone)]
struct CachedToolResult {
    output: String,
    cached_at: std::time::Instant,
}

impl ToolExecutionCache {
    pub fn new(max_size: usize, ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(tokio::sync::Mutex::new(LruCache::new(max_size))),
            ttl: std::time::Duration::from_secs(ttl_secs),
        }
    }

    fn make_key(tool_name: &str, input: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(tool_name.as_bytes());
        hasher.update(b":");
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub async fn get(&self, tool_name: &str, input: &str) -> Option<String> {
        let key = Self::make_key(tool_name, input);
        let mut cache = self.cache.lock().await;
        
        if let Some(cached) = cache.get(&key) {
            if cached.cached_at.elapsed() < self.ttl {
                return Some(cached.output.clone());
            }
        }
        None
    }

    pub async fn insert(&self, tool_name: &str, input: &str, output: String) {
        let key = Self::make_key(tool_name, input);
        let mut cache = self.cache.lock().await;
        cache.put(key, CachedToolResult {
            output,
            cached_at: std::time::Instant::now(),
        });
    }
}

pub struct ToolRegistry {
    // ... 现有字段
    cache: Option<ToolExecutionCache>,  // 新增
}

impl ToolRegistry {
    pub fn with_cache(mut self, cache: ToolExecutionCache) -> Self {
        self.cache = Some(cache);
        self
    }

    pub async fn execute(&self, tool_name: &str, input: &str) -> Result<ToolResult, ToolError> {
        // 尝试从缓存获取
        if let Some(ref cache) = self.cache {
            if let Some(cached_output) = cache.get(tool_name, input).await {
                tracing::debug!("Tool {} result served from cache", tool_name);
                return Ok(ToolResult {
                    output: cached_output,
                    execution_id: String::new(),
                    duration_ms: None,
                });
            }
        }

        // 缓存未命中，执行工具
        let result = self.execute_internal(tool_name, input).await?;
        
        // 存入缓存
        if let Some(ref cache) = self.cache {
            cache.insert(tool_name, input, result.output.clone()).await;
        }

        Ok(result)
    }

    async fn execute_internal(&self, tool_name: &str, input: &str) -> Result<ToolResult, ToolError> {
        // ... 原有执行逻辑
    }
}
```

#### 3.3.3 验收标准

- [ ] `cargo check` 通过
- [ ] 相同工具调用第二次执行响应时间减少 80%

---

## 四、安全加固实施

### 4.1 工具执行白名单

#### 4.1.1 修改位置

修改 `agent/src/tool_registry.rs`

#### 4.1.2 实施步骤

```rust
// agent/src/tool_registry.rs

#[derive(Clone)]
pub struct ToolRegistry {
    // ... 现有字段
    
    // 安全相关字段
    allowed_tools: Arc<std::sync::RwLock<HashSet<String>>>,
    blocked_tools: Arc<std::sync::RwLock<HashSet<String>>>,
    strict_mode: bool,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            // ... 现有初始化
            allowed_tools: Arc::new(std::sync::RwLock::new(HashSet::new())),
            blocked_tools: Arc::new(std::sync::RwLock::new(HashSet::new())),
            strict_mode: false,
        }
    }

    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        *self.allowed_tools.write().unwrap() = tools.into_iter().collect();
        self
    }

    pub fn with_blocked_tools(mut self, tools: Vec<String>) -> Self {
        *self.blocked_tools.write().unwrap() = tools.into_iter().collect();
        self
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    fn is_tool_allowed(&self, tool_name: &str) -> bool {
        let blocked = self.blocked_tools.read().unwrap();
        if blocked.contains(tool_name) {
            return false;
        }

        if self.strict_mode {
            let allowed = self.allowed_tools.read().unwrap();
            allowed.is_empty() || allowed.contains(tool_name)
        } else {
            true
        }
    }

    pub async fn execute(&self, tool_name: &str, input: &str) -> Result<ToolResult, ToolError> {
        // 安全检查
        if !self.is_tool_allowed(tool_name) {
            tracing::warn!("Tool '{}' execution denied", tool_name);
            return Err(ToolError::PermissionDenied(format!(
                "Tool '{}' is not allowed to execute", tool_name
            )));
        }

        // ... 原有执行逻辑
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    // ... 现有错误变体
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
```

#### 4.1.3 验收标准

- [ ] `cargo check` 通过
- [ ] 配置白名单后，非白名单工具无法执行
- [ ] 配置黑名单后，黑名单工具无法执行

---

### 4.2 命令注入防护

#### 4.2.1 修改位置

新建 `core/src/command_validator.rs`，修改 `core/src/builtin_tools.rs`

#### 4.2.2 实施步骤

**Step 1: 创建 command_validator.rs**

```rust
// core/src/command_validator.rs

#[derive(Debug, Clone)]
pub struct CommandValidationResult {
    pub is_safe: bool,
    pub sanitized: Option<String>,
    pub warnings: Vec<String>,
    pub dangerous_patterns: Vec<String>,
}

pub struct CommandValidator {
    dangerous_patterns: Vec<String>,
    max_length: usize,
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self {
            dangerous_patterns: vec![
                ";".to_string(),
                "|".to_string(),
                "&".to_string(),
                "`".to_string(),
                "$(".to_string(),
                "${".to_string(),
                "\n".to_string(),
                "\r".to_string(),
                ">".to_string(),
                "<".to_string(),
                ">>".to_string(),
                "<<".to_string(),
                "2>".to_string(),
                "2>&1".to_string(),
                "&&".to_string(),
                "||".to_string(),
                "(".to_string(),
                ")".to_string(),
                "{".to_string(),
                "}".to_string(),
                "[".to_string(),
                "]".to_string(),
                "!".to_string(),
                "#".to_string(),
                "~".to_string(),
                "%".to_string(),
                "^".to_string(),
                "*".to_string(),
                "?".to_string(),
                "\\".to_string(),
            ],
            max_length: 10000,
        }
    }
}

impl CommandValidator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_custom_patterns(mut self, patterns: Vec<String>) -> Self {
        self.dangerous_patterns = patterns;
        self
    }

    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = max;
        self
    }

    pub fn validate(&self, command: &str) -> CommandValidationResult {
        let mut warnings = Vec::new();
        let mut dangerous_patterns = Vec::new();

        // 检查长度
        if command.len() > self.max_length {
            return CommandValidationResult {
                is_safe: false,
                sanitized: None,
                warnings: vec![format!("Command exceeds maximum length of {} bytes", self.max_length)],
                dangerous_patterns: vec![],
            };
        }

        // 检查危险模式
        for pattern in &self.dangerous_patterns {
            if command.contains(pattern) {
                dangerous_patterns.push(pattern.clone());
                warnings.push(format!("Dangerous pattern '{}' found", pattern));
            }
        }

        // 检查可疑的 URL 编码
        if command.contains("%") && command.contains(";") {
            dangerous_patterns.push("url-encoded-injection".to_string());
            warnings.push("Potential URL-encoded command injection".to_string());
        }

        let is_safe = dangerous_patterns.is_empty();

        CommandValidationResult {
            is_safe,
            sanitized: if is_safe { Some(command.to_string()) } else { None },
            warnings,
            dangerous_patterns,
        }
    }

    pub fn sanitize(&self, command: &str) -> String {
        let mut result = command.to_string();
        
        // 移除尾随的危险字符
        for pattern in &[";", "|", "&", "`", "$", ">", "<", "\n", "\r"] {
            result = result.trim_end_matches(pattern).to_string();
        }
        
        result
    }
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: 修改 builtin_tools.rs**

```rust
// core/src/builtin_tools.rs

static COMMAND_VALIDATOR: once_cell::sync::Lazy<CommandValidator> = 
    once_cell::sync::Lazy::new(CommandValidator::new);

pub async fn execute_bash(command: &str, _working_dir: Option<&str>) -> Result<String, BuiltinToolError> {
    // 命令注入检查
    let validation = COMMAND_VALIDATOR.validate(command);
    
    if !validation.is_safe {
        tracing::warn!(
            "Blocked potentially dangerous command: {:?}, patterns: {:?}",
            command,
            validation.dangerous_patterns
        );
        return Err(BuiltinToolError::SecurityViolation(format!(
            "Command contains dangerous patterns: {:?}",
            validation.dangerous_patterns
        )));
    }

    if !validation.warnings.is_empty() {
        tracing::warn!("Command warnings: {:?}", validation.warnings);
    }

    // 执行命令
    let output = tokio::process::Command::new("sh")
        .args(["-c", &command])
        .output()
        .await
        .map_err(|e| BuiltinToolError::ExecutionFailed(e.to_string()))?;

    // ... 其余逻辑
}
```

#### 4.2.3 验收标准

- [ ] `cargo check` 通过
- [ ] 包含 `; rm -rf` 等危险命令被阻止
- [ ] 误报率 < 1%（合法命令不被阻止）

---

## 五、可维护性改进实施

### 5.1 统一配置管理

#### 5.1.1 修改位置

新建 `agent/src/settings.rs`

#### 5.1.2 实施步骤

```rust
// agent/src/settings.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettings {
    pub react: ReActConfig,
    pub task: TaskExecutorConfig,
    pub retry: RetryPolicy,
    pub loop_detector: LoopDetectorConfig,
    pub coordinator: CoordinatorConfig,
    pub session: SessionConfig,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            react: ReActConfig::default(),
            task: TaskExecutorConfig::default(),
            retry: RetryPolicy::default(),
            loop_detector: LoopDetectorConfig::default(),
            coordinator: CoordinatorConfig::default(),
            session: SessionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub enable_self_verification: bool,
    pub enable_error_recovery: bool,
    pub event_buffer_size: usize,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            enable_self_verification: false,
            enable_error_recovery: true,
            event_buffer_size: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub auto_compaction_threshold: usize,
    pub compaction_token_limit: usize,
    pub max_history_messages: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_compaction_threshold: 100_000,
            compaction_token_limit: 50_000,
            max_history_messages: 1000,
        }
    }
}

impl AgentSettings {
    pub fn from_file(path: &std::path::Path) -> Result<Self, serde_json::Error> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
    }

    pub fn to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.react.max_iterations == 0 {
            return Err(ValidationError::InvalidValue("max_iterations cannot be 0".into()));
        }
        if self.retry.max_attempts == 0 {
            return Err(ValidationError::InvalidValue("max_attempts cannot be 0".into()));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActConfig {
    pub max_iterations: usize,
    pub max_depth: usize,
    pub max_retry_attempts: usize,
    pub reflection_threshold: usize,
    pub enable_analyzing: bool,
    pub enable_reflection: bool,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_depth: 10,
            max_retry_attempts: 3,
            reflection_threshold: 3,
            enable_analyzing: true,
            enable_reflection: true,
        }
    }
}
```

#### 5.1.3 验收标准

- [ ] `cargo check` 通过
- [ ] 配置文件可以被正确加载和保存

---

### 5.2 统一错误处理

#### 5.2.1 修改位置

新建 `agent/src/errors.rs`

#### 5.2.2 实施步骤

```rust
// agent/src/errors.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("初始化失败: {0}")]
    Init(String),

    #[error("执行失败: {0}")]
    Execution(String),

    #[error("工具错误 [{name}]: {inner}")]
    Tool {
        name: String,
        #[source]
        inner: anyhow::Error,
    },

    #[error("超时 ({duration}s)")]
    Timeout { duration: u64 },

    #[error("取消操作")]
    Cancelled,

    #[error("无效状态转换: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("会话错误: {0}")]
    Session(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl AgentError {
    pub fn init(msg: impl Into<String>) -> Self {
        Self::Init(msg.into())
    }

    pub fn execution(msg: impl Into<String>) -> Self {
        Self::Execution(msg.into())
    }

    pub fn tool(name: impl Into<String>, inner: anyhow::Error) -> Self {
        Self::Tool {
            name: name.into(),
            inner,
        }
    }

    pub fn timeout(duration: u64) -> Self {
        Self::Timeout { duration }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. } | Self::Execution(_) | Self::Tool { .. }
        )
    }
}

// 为常见的错误类型实现转换
impl From<std::io::Error> for AgentError {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(e: serde_json::Error) -> Self {
        Self::Config(format!("JSON error: {}", e))
    }
}
```

#### 5.2.3 验收标准

- [ ] `cargo check` 通过
- [ ] 错误消息统一使用中文描述

---

## 六、实施优先级与时间估算

### 6.1 第一阶段（1-2周）

| 序号 | 改进项 | 优先级 | 预估工时 |
|------|--------|--------|----------|
| 4 | 数据库连接池调优 | P1 | 0.5h |
| 7 | 工具执行白名单 | P1 | 2h |
| 8 | 命令注入防护 | P1 | 3h |
| 1 | Trait Bounds 替代动态分发 | P1 | 8h |

### 6.2 第二阶段（2-3周）

| 序号 | 改进项 | 优先级 | 预估工时 |
|------|--------|--------|----------|
| 5 | 引入向量缓存层 | P2 | 6h |
| 6 | 工具执行结果缓存 | P2 | 4h |
| 10 | 统一错误处理 | P2 | 4h |
| 9 | 统一配置管理 | P2 | 6h |

### 6.3 第三阶段（3-4周）

| 序号 | 改进项 | 优先级 | 预估工时 |
|------|--------|--------|----------|
| 2 | 统一事件系统 | P2 | 8h |
| 3 | 前端 Store 拆分 | P2 | 12h |

---

## 七、风险与缓解措施

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Trait Bounds 修改导致大量依赖方需要更新 | 高 | 先在小范围模块试点，确认无误后全面推广 |
| 缓存引入导致内存泄漏 | 中 | 添加监控和 LRU 淘汰机制 |
| 命令注入防护误报 | 中 | 建立回归测试集，确保合法命令不被阻止 |
| 前端 Store 拆分破坏现有功能 | 高 | 保持向后兼容，逐步迁移 |

---

## 八、验收清单

### 8.1 功能验收

- [ ] 所有单元测试通过
- [ ] 集成测试通过
- [ ] E2E 测试通过
- [ ] 手动测试覆盖关键路径

### 8.2 性能验收

- [ ] 数据库查询响应时间 < 100ms
- [ ] 向量搜索缓存命中率 > 50%（相同查询）
- [ ] 工具调用缓存命中率 > 30%

### 8.3 安全验收

- [ ] 渗透测试通过
- [ ] 危险命令注入被阻止
- [ ] 非白名单工具无法执行

---

## 九、附录

### 9.1 相关文件清单

需要修改的 Rust 文件：

```
src-tauri/crates/agent/src/
├── coordinator.rs
├── lib.rs
├── traits.rs          # 新建
├── events.rs          # 新建
├── settings.rs        # 新建
├── errors.rs         # 新建
└── tool_registry.rs

src-tauri/crates/core/src/
├── db.rs
├── hybrid_search.rs
├── vector_cache.rs   # 新建
├── command_validator.rs  # 新建
└── builtin_tools.rs

src/stores/domain/
├── conversationListStore.ts  # 新建
├── messageStore.ts          # 新建
├── streamingStore.ts        # 新建
├── uiStateStore.ts          # 新建
└── conversationStore.ts    # 修改（简化）
```

### 9.2 依赖更新

```toml
# core/Cargo.toml 新增
lru = "0.12"

# agent/Cargo.toml 确认已有
thiserror = "2"
anyhow = "1"
tokio = { version = "1", features = ["full"] }
```

---

> 文档结束
