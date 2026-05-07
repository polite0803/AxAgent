/** Skill 事件总线，提供 Skill → App 通信的 namespace 隔离事件系统 */

type EventHandler = (payload: unknown) => void | Promise<void>;
const listeners = new Map<string, Set<EventHandler>>();

export const skillEventBus = {
  /** 发送事件（指定 skill namespace） */
  emit(skillName: string, event: string, payload: unknown): void {
    const key = `${skillName}:${event}`;
    const handlers = listeners.get(key);
    if (handlers) {
      for (const handler of handlers) {
        try {
          // 用 Promise.resolve 包裹以捕获同步和异步错误
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

  /** 监听事件，返回取消监听的函数 */
  on(skillName: string, event: string, handler: EventHandler): () => void {
    const key = `${skillName}:${event}`;
    if (!listeners.has(key)) {
      listeners.set(key, new Set());
    }
    listeners.get(key)!.add(handler);
    return () => {
      listeners.get(key)?.delete(handler);
    };
  },

  /** 清除指定 skill 的所有监听 */
  clear(skillName: string): void {
    for (const [key] of listeners) {
      if (key.startsWith(`${skillName}:`)) {
        listeners.delete(key);
      }
    }
  },
};
