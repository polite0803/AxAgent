// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { DataSourceConfig } from "@/types";

/**
 * 数据绑定引擎：解析 DataSourceConfig 并返回实际数据。
 *
 * 支持四种数据源类型：
 * - store：读取 Zustand Store 数据（支持响应式订阅）
 * - api：调用 Tauri invoke 或 fetch
 * - static：直接返回静态数据
 * - agent-generated：从 Agent 生成数据中获取
 */

export interface DataSourceSubscriber {
  unsubscribe: () => void;
}

/**
 * 解析数据源配置，返回实际数据（非 Hook 版本，用于一次性获取）。
 */
export async function resolveDataSource(
  config: DataSourceConfig,
): Promise<unknown> {
  switch (config.type) {
    case "static":
      return (config.config as Record<string, unknown>).value;

    case "store": {
      const { storeName, selector } = config.config as {
        storeName: string;
        selector?: string;
      };
      const { getStoreRegistry } = await import("@/lib/storeRegistry");
      const store = getStoreRegistry().get(storeName);
      if (!store) {
        throw new Error(`Store "${storeName}" not registered`);
      }
      const state = store.get() as Record<string, unknown>;
      if (selector) {
        return getNestedValue(state, selector);
      }
      return state;
    }

    case "api": {
      const { endpoint, method, params } = config.config as {
        endpoint: string;
        method: "invoke" | "fetch";
        params?: unknown;
      };
      if (method === "invoke") {
        return invoke<unknown>(endpoint, params as Record<string, unknown>);
      }
      // fetch 模式
      const response = await fetch(endpoint, params as RequestInit);
      if (!response.ok) {
        throw new Error(`API request failed: ${response.statusText}`);
      }
      return response.json();
    }

    case "agent-generated": {
      const { generationId } = config.config as { generationId: string };
      const { useExecutionStore } = await import("@/stores");
      const executionState = useExecutionStore.getState();
      const generation =
        (executionState as unknown as Record<string, unknown>) /* SAFE: dynamic UI data binding from execution state */[
          generationId
        ];
      if (!generation) {
        throw new Error(
          `Agent generated data "${generationId}" not found in execution store`,
        );
      }
      return generation;
    }

    default:
      throw new Error(`Unknown data source type: ${config.type}`);
  }
}

/**
 * 订阅数据源变化，支持store响应式和轮询。
 * @param config 数据源配置
 * @param onData 数据更新回调
 * @returns 取消订阅函数
 */
export async function subscribeDataSource(
  config: DataSourceConfig,
  onData: (data: unknown) => void,
  onError?: (error: Error) => void,
): Promise<DataSourceSubscriber> {
  // 先执行一次初始加载
  try {
    const initialData = await resolveDataSource(config);
    onData(initialData);
  } catch (err) {
    if (onError) {
      onError(err instanceof Error ? err : new Error(String(err)));
    }
  }

  const cleanupFns: Array<() => void> = [];

  // 处理轮询
  if (config.polling && config.polling > 0) {
    const timer = window.setInterval(async () => {
      try {
        const data = await resolveDataSource(config);
        onData(data);
      } catch (err) {
        if (onError) {
          onError(err instanceof Error ? err : new Error(String(err)));
        }
      }
    }, config.polling);
    cleanupFns.push(() => window.clearInterval(timer));
  }

  // Store 类型数据源 — 通过 StoreRegistry 获取 store 并执行响应式订阅
  if (config.type === "store") {
    const { storeName, selector } = config.config as {
      storeName: string;
      selector?: string;
    };
    const { getStoreRegistry } = await import("@/lib/storeRegistry");
    const store = getStoreRegistry().get(storeName);
    if (store && typeof store.subscribe === "function") {
      const getNested = getNestedValue;
      const unsubscribe = store.subscribe(() => {
        const state = store.get() as Record<string, unknown>;
        const data = selector ? getNested(state, selector) : state;
        onData(data);
      });
      cleanupFns.push(unsubscribe);
    }
  }

  return {
    unsubscribe: () => {
      for (const fn of cleanupFns) {
        fn();
      }
    },
  };
}

/**
 * 使用点号分隔的路径获取嵌套对象值。
 * 如 "user.profile.name" -> obj.user.profile.name
 */
function getNestedValue(
  obj: Record<string, unknown>,
  path: string,
): unknown {
  const keys = path.split(".");
  let current: unknown = obj;
  for (const key of keys) {
    if (current === null || current === undefined) {
      return undefined;
    }
    if (typeof current !== "object") {
      return undefined;
    }
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}
