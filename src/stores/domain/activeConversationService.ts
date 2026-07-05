// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 跨 store 共享的活跃会话 ID 访问器。
 *
 * 用于打破 conversationStore ↔ preferenceStore 双向循环依赖：
 * - conversationStore 在切换会话时调用 setActiveConversationId()
 * - preferenceStore 通过 getActiveConversationId() 读取，无需直接 import conversationStore
 *
 * 原则：写入方（conversationStore）负责注入，读取方（preferenceStore）不依赖写入方的类型。
 */

let _activeConversationId: string | null = null;
let _listener: ((id: string | null) => void) | null = null;

export function setActiveConversationId(id: string | null): void {
  _activeConversationId = id;
  _listener?.(id);
}

export function getActiveConversationId(): string | null {
  return _activeConversationId;
}

/**
 * 注册一个当活跃会话变化时的回调（可选）。
 * 用于通知 preferenceStore 重新加载偏好。
 */
export function onActiveConversationChange(
  listener: (id: string | null) => void,
): () => void {
  _listener = listener;
  return () => {
    _listener = null;
  };
}
