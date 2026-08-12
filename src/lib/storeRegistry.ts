// SPDX-License-Identifier: AGPL-3.0-only

import { useConversationStore } from "@/stores/domain/conversationStore";
import { usePreferenceStore } from "@/stores/domain/preferenceStore";
import { useStreamStore } from "@/stores/domain/streamStore";
import { useAgentStore } from "@/stores/feature/agentStore";
import { useExecutionStore } from "@/stores/feature/executionStore";
import { useKnowledgeStore } from "@/stores/feature/knowledgeStore";
import { useProviderStore } from "@/stores/feature/providerStore";
import { useSettingsStore } from "@/stores/feature/settingsStore";
import { useArtifactStore } from "@/stores/shared/artifactStore";
import { useTabStore } from "@/stores/shared/tabStore";
import { useUIStore } from "@/stores/shared/uiStore";

/** Zustand Store 注册表，供声明式 Action 的 store 类型访问 */

type StoreAccessor = {
  get: (payload?: unknown) => unknown;
  set: (payload?: unknown) => void;
  update: (payload?: unknown) => void;
  subscribe: (listener: () => void) => () => void;
};

const storeRegistry = new Map<string, StoreAccessor>();

let _initialized = false;

/**
 * 初始化 Store 注册表（App 启动时调用一次）。
 * 注册所有可被 Skill 声明式动作访问的 Zustand Store。
 */
export async function initStoreRegistry(): Promise<void> {
  if (_initialized) {
    return;
  }
  _initialized = true;

  const registry: Array<{
    name: string;
    store: {
      getState: () => unknown;
      setState: (partial: unknown) => void;
      subscribe: (listener: () => void) => () => void;
    };
  }> = [
    {
      name: "preference",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: usePreferenceStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "conversation",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useConversationStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "ui",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useUIStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    // P1 #16: skill store 已从白名单移除 — Skill 不应通过声明式 action
    // 修改技能系统自身状态。Skill 间通信使用 skillEventBus 的 emit/on 机制。
    {
      name: "artifact",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useArtifactStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "chatWorkspace",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useUIStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "settings",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useSettingsStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "provider",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useProviderStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "knowledge",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useKnowledgeStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "agent",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useAgentStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "tab",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useTabStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "stream",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useStreamStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
    {
      name: "execution",
      // SAFE: uniform store registry — all Zustand stores expose getState/setState
      store: useExecutionStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
        subscribe: (listener: () => void) => () => void;
      },
    },
  ];

  for (const { name, store } of registry) {
    registerStore(name, {
      get: (payload?: unknown) => {
        // SAFE: store state is treated as generic Record for registry access
        const state = store.getState() as Record<string, unknown>;
        const key = typeof payload === "string" ? payload : undefined;
        return key ? state[key] : state;
      },
      set: (payload?: unknown) => {
        if (
          payload !== undefined
          && (typeof payload !== "object"
            || payload === null
            || Array.isArray(payload))
        ) {
          console.warn(
            `[storeRegistry] set() expected a plain object, received: ${typeof payload}`,
          );
          return;
        }
        // SAFE: runtime validation above ensures payload is a plain object compatible with setState
        store.setState(payload as Parameters<typeof store.setState>[0]);
      },
      update: (payload?: unknown) => {
        if (payload && typeof payload === "object" && !Array.isArray(payload)) {
          // SAFE: runtime validation above ensures payload is a plain object compatible with setState
          store.setState(payload as Parameters<typeof store.setState>[0]);
        } else if (payload !== undefined) {
          console.warn(
            `[storeRegistry] update() expected a plain object, received: ${typeof payload}`,
          );
        }
      },
      subscribe: (listener) => store.subscribe(listener),
    });
  }
}

export function getStoreRegistry(): Map<string, StoreAccessor> {
  return storeRegistry;
}

export function registerStore(name: string, accessor: StoreAccessor): void {
  storeRegistry.set(name, accessor);
}

export function unregisterStore(name: string): void {
  storeRegistry.delete(name);
}
