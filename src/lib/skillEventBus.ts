// SPDX-License-Identifier: AGPL-3.0-only

/** Skill 事件总线，提供 Skill → App 通信的 namespace 隔离事件系统 */

type EventHandler = (payload: unknown) => void | Promise<void>;
const listeners = new Map<string, Set<EventHandler>>();
const MAX_LISTENER_KEYS = 200;

/**
 * 容量驱逐策略：FIFO（先进先出）。
 * 当注册的监听器 key 数量超过 MAX_LISTENER_KEYS 时，驱逐最早注册的监听器。
 * 对于 event bus 场景，FIFO 是可接受的——长期未活跃的 Skill 通常先注册。
 */
function evictIfNeeded() {
  if (listeners.size <= MAX_LISTENER_KEYS) {
    return;
  }
  const keys = listeners.keys();
  const excess = listeners.size - MAX_LISTENER_KEYS;
  for (let i = 0; i < excess; i++) {
    const key = keys.next().value;
    if (key !== undefined) {
      listeners.delete(key);
    }
  }
}

export const skillEventBus = {
  emit(skillName: string, event: string, payload: unknown): void {
    const key = `${skillName}:${event}`;
    const handlers = listeners.get(key);
    if (handlers) {
      for (const handler of handlers) {
        try {
          const result = handler(payload);
          if (result instanceof Promise) {
            result.catch((e) => console.error(`[skillEventBus] 异步 handler 错误 ${key}:`, e));
          }
        } catch (e) {
          console.error(`[skillEventBus] Handler 错误 ${key}:`, e);
        }
      }
    }
  },

  on(skillName: string, event: string, handler: EventHandler): () => void {
    const key = `${skillName}:${event}`;
    if (!listeners.has(key)) {
      listeners.set(key, new Set());
      evictIfNeeded();
    }
    listeners.get(key)!.add(handler);
    return () => {
      listeners.get(key)?.delete(handler);
    };
  },

  clear(skillName: string): void {
    for (const [key] of listeners) {
      if (key.startsWith(`${skillName}:`)) {
        listeners.delete(key);
      }
    }
  },

  destroy(): void {
    listeners.clear();
  },
};
