/** Skill 事件总线，提供 Skill -> App 通信的 namespace 隔离事件系统 */

type EventHandler = (payload: unknown) => void;
const listeners = new Map<string, Set<EventHandler>>();

export const skillEventBus = {
  /** 发送事件（指定 skill namespace） */
  emit(skillName: string, event: string, payload: unknown): void {
    const key = `${skillName}:${event}`;
    const handlers = listeners.get(key);
    if (handlers) {
      for (const handler of handlers) {
        try {
          handler(payload);
        } catch (e) {
          console.error(`[skillEventBus] Handler error for ${key}:`, e);
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
