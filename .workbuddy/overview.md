# AxAgent 性能优化 — 第一阶段 (P0 核心链路)

## 已完成

### P0-1: 流式渲染批量化
- **文件**: `src/stores/domain/streamStore.ts`
- **变更**: `STREAM_UI_FLUSH_INTERVAL_MS` 16→50ms, `STREAM_MAX_CHUNK_SIZE` 100→500 字符
- **效果**: React setState 调用频率降低 ~67%，渲染更平滑

### P0-2: 消息列表 O(1) 局部更新
- **文件**: `src/stores/domain/streamStore.ts`, `src/stores/domain/conversationStore.ts`
- **变更**: 新增 `_messageIndex: Map<string, number>` 模块级索引，`flushPendingStreamChunk` 中三个分支均使用索引进行 O(1) 查找和更新，`conversationStore` 中通过 subscribe 自动重建索引
- **效果**: 流式更新从 O(n) messages.map() 降为 O(1) 直接数组槽位赋值，长对话性能提升显著

### P0-3: 级联回退管道 (前端 Q&A 路径)
- **文件**: `src/stores/domain/conversationStore.ts`, `src/types/agent.ts`
- **变更**: 
  - 新增 `buildFallbackChain()` 从所有可用 provider 构建回退链
  - 新增 `ModelFallbackEvent` 类型定义
  - `sendMessage` 错误处理增加自动回退逻辑：主模型失败→遍历回退链→自动切换 conversation 模型→重新发送
  - 排除不可重试错误 (认证、配額、上下文溢出等)
- **效果**: Q&A 路径可用性大幅提升，主模型失败自动切换备用模型

### P0-4: React 18 并发渲染 (useDeferredValue)
- **文件**: `src/components/chat/ChatView.tsx`
- **变更**: 对 `activeMessages`、`thinkingActiveMessageIds`、`userSearchContentById` 使用 `useDeferredValue`，bubbleItems useMemo 使用延迟值
- **效果**: 流式更新时 React 保持 UI 响应，避免气泡列表重新计算导致卡顿

## 后续事项
- P1: 智能模型路由 (SmartModelRouter)、语义缓存、Agent 事件分级
- P2: 上下文相关性裁剪、Virtual List、全链路追踪增强
- Rust 后端: 添加 model-fallback 事件 emit、context_manager 相关性裁剪
