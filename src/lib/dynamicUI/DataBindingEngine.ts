// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { DataSourceConfig } from "@/types";

/**
 * 数据绑定引擎：解析 DataSourceConfig 并返回实际数据。
 *
 * 支持四种数据源类型：
 * - store：读取 Zustand Store 数据
 * - api：调用 Tauri invoke 或 fetch
 * - static：直接返回静态数据
 * - agent-generated：从 Agent 生成数据中获取
 */

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
      const state = store.getState() as Record<string, unknown>;
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
      const generation = (executionState as Record<string, unknown>)[
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
