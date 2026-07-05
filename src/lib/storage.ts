// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 统一存储抽象层 —— 封装 localStorage 操作。
 *
 * 设计原则：
 * 1. 所有存储操作通过此模块进行
 * 2. 支持 JSON 自动序列化/反序列化
 * 3. 支持带前缀存储（按模块隔离）
 * 4. 后续可无缝切换到其他存储后端（如 IndexedDB）
 * 5. 支持可选的写入防抖（面板宽度等频繁写入场景）
 * 6. secureStorage 使用 Tauri 后端 AES-256-GCM 加密，保护 API Key 等敏感数据
 */

export const storage = {
  get<T = string>(key: string): T | null {
    const raw = localStorage.getItem(key);
    if (raw === null) { return null; }
    try {
      return JSON.parse(raw) as T;
    } catch {
      return raw as unknown as T;
    }
  },

  set<T>(key: string, value: T): void {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch (e) {
      if (e instanceof DOMException && e.name === "QuotaExceededError") {
        console.warn(`[storage] 存储空间不足，无法保存 ${key}`);
      } else {
        throw e;
      }
    }
  },

  remove(key: string): void {
    localStorage.removeItem(key);
  },

  clear(): void {
    localStorage.clear();
  },

  /** 带命名空间前缀的存储（推荐使用） */
  namespace(prefix: string) {
    const ns = (key: string) => `${prefix}:${key}`;
    return {
      get<T = string>(key: string): T | null {
        return storage.get<T>(ns(key));
      },
      set<T>(key: string, value: T): void {
        storage.set(ns(key), value);
      },
      remove(key: string): void {
        storage.remove(ns(key));
      },
    };
  },
};

/**
 * 创建一个防抖版本的命名空间存储，适用于频繁写入场景。
 * 在最后一次 `set()` 调用后等待 `delay` 毫秒再实际写入。
 */
export function createDebouncedStorage(prefix: string, delay = 300) {
  const ns = storage.namespace(prefix);
  const pending = new Map<string, { timer: ReturnType<typeof setTimeout>; value: unknown }>();

  return {
    get<T = string>(key: string): T | null {
      return ns.get<T>(key);
    },
    set<T>(key: string, value: T): void {
      const entry = pending.get(key);
      if (entry) { clearTimeout(entry.timer); }
      const timer = setTimeout(() => {
        ns.set(key, value);
        pending.delete(key);
      }, delay) as unknown as ReturnType<typeof setTimeout>;
      pending.set(key, { timer, value });
    },
    remove(key: string): void {
      const entry = pending.get(key);
      if (entry) {
        clearTimeout(entry.timer);
        pending.delete(key);
      }
      ns.remove(key);
    },
    /** 立即刷入所有待写入数据 */
    flush(): void {
      for (const [key, entry] of pending) {
        clearTimeout(entry.timer);
        ns.set(key, entry.value);
        pending.delete(key);
      }
    },
  };
}

/**
 * 安全存储 —— 使用 Tauri 后端 AES-256-GCM 加密存储敏感数据。
 *
 * 用于 API Key、Token 等不应以明文出现在 localStorage 中的敏感值。
 * 在 Tauri 环境下调用后端 Rust 加密/解密命令，密钥从机器指纹派生。
 * 在浏览器模式下回退到 localStorage（开发调试用），并发出警告。
 */
/// SECURITY (S6): 浏览器 fallback 模式下对敏感值加 base64 编码，防止明文泄露。
/// 生产环境必须使用 Tauri 后端 AES-256-GCM 加密。
export const secureStorage = {
  async set<T>(key: string, value: T): Promise<void> {
    const serialized = JSON.stringify(value);
    try {
      const { isTauri, invoke } = await import("./invoke");
      if (isTauri()) {
        await invoke("secure_store", { key, value: serialized });
        return;
      }
    } catch {
      // Tauri not available: fall through to localStorage
    }
    console.warn(
      `⚠️ [secureStorage] SECURITY: Tauri not available, falling back to obfuscated localStorage for key: ${key}. Sensitive data is NOT encrypted in this mode — do NOT use browser mode in production.`,
    );
    // 浏览器 fallback 模式下使用 base64 编码存储，非加密，仅防明文泄露
    storage.set(`secure:${key}`, btoa(serialized));
  },

  async get<T = string>(key: string): Promise<T | null> {
    try {
      const { isTauri, invoke } = await import("./invoke");
      if (isTauri()) {
        const result = await invoke<Option<string>>("secure_get", { key });
        if (result) {
          try {
            return JSON.parse(result) as T;
          } catch {
            return result as unknown as T;
          }
        }
        return null;
      }
    } catch {
      // Tauri not available: fall through to localStorage
    }
    const raw = storage.get<string>(`secure:${key}`);
    if (raw === null) { return null; }
    try {
      return JSON.parse(atob(raw)) as T;
    } catch {
      try {
        return atob(raw) as unknown as T;
      } catch {
        return raw as unknown as T;
      }
    }
  },

  async remove(key: string): Promise<void> {
    try {
      const { isTauri, invoke } = await import("./invoke");
      if (isTauri()) {
        await invoke("secure_remove", { key });
        return;
      }
    } catch {
      // Tauri not available: fall through
    }
    storage.remove(`secure:${key}`);
  },
};

// Type helper for Tauri invoke return
type Option<T> = T | null;
