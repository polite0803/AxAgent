// SPDX-License-Identifier: AGPL-3.0-only

/** Skill 事件总线，提供 Skill → App 通信的 namespace 隔离事件系统 */

type EventHandler = (payload: unknown) => void | Promise<void>;

interface ListenerEntry {
  handlers: Set<EventHandler>;
  lastAccess: number;
}

const listeners = new Map<string, ListenerEntry>();
const MAX_LISTENER_KEYS = 200;

/**
 * P3-2.20: LRU 驱逐策略。
 * 当注册的监听器 key 数量超过 MAX_LISTENER_KEYS 时，驱逐最久未访问的 key。
 * emit 和 on 均会更新 lastAccess 时间戳，确保活跃 key 不被误驱逐。
 * 驱逐时选择 handler 数量最少的 key（当多个 key 访问时间接近时），
 * 最小化对活跃 Skill 的影响。
 */
function evictIfNeeded() {
  if (listeners.size <= MAX_LISTENER_KEYS) {
    return;
  }
  const excess = listeners.size - MAX_LISTENER_KEYS;
  const entries = Array.from(listeners.entries());
  // 按 lastAccess 升序（最久未访问在前），同时间按 handler 数量升序
  entries.sort((a, b) => {
    const timeDiff = a[1].lastAccess - b[1].lastAccess;
    if (timeDiff !== 0) return timeDiff;
    return a[1].handlers.size - b[1].handlers.size;
  });
  for (let i = 0; i < excess && i < entries.length; i++) {
    listeners.delete(entries[i][0]);
  }
}

function touchEntry(entry: ListenerEntry): void {
  entry.lastAccess = Date.now();
}

export const skillEventBus = {
  emit(skillName: string, event: string, payload: unknown): void {
    const key = `${skillName}:${event}`;
    const entry = listeners.get(key);
    if (entry) {
      touchEntry(entry);
      for (const handler of entry.handlers) {
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
    let entry = listeners.get(key);
    if (!entry) {
      entry = { handlers: new Set(), lastAccess: Date.now() };
      listeners.set(key, entry);
      evictIfNeeded();
    } else {
      touchEntry(entry);
    }
    entry.handlers.add(handler);
    return () => {
      entry?.handlers.delete(handler);
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
