// SPDX-License-Identifier: AGPL-3.0-only

/** Zustand Store 注册表，供声明式 Action 的 store 类型访问 */

type StoreAccessor = {
  get: (payload?: unknown) => unknown;
  set: (payload?: unknown) => void;
  update: (payload?: unknown) => void;
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

  const stores = await import("@/stores");

  const registry: Array<{
    name: string;
    store: { getState: () => unknown; setState: (partial: unknown) => void };
  }> = [
    {
      name: "preference",
      store: stores.usePreferenceStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "conversation",
      store: stores.useConversationStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "ui",
      store: stores.useUIStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "skill",
      store: stores.useSkillStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "artifact",
      store: stores.useArtifactStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "chatWorkspace",
      store: stores.useChatWorkspaceStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "settings",
      store: stores.useSettingsStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "provider",
      store: stores.useProviderStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "knowledge",
      store: stores.useKnowledgeStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "agent",
      store: stores.useAgentStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "tab",
      store: stores.useTabStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
    {
      name: "stream",
      store: stores.useStreamStore as unknown as {
        getState: () => unknown;
        setState: (partial: unknown) => void;
      },
    },
  ];

  for (const { name, store } of registry) {
    registerStore(name, {
      get: (payload?: unknown) => {
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
        store.setState(payload as Parameters<typeof store.setState>[0]);
      },
      update: (payload?: unknown) => {
        if (payload && typeof payload === "object" && !Array.isArray(payload)) {
          store.setState(payload as Parameters<typeof store.setState>[0]);
        } else if (payload !== undefined) {
          console.warn(
            `[storeRegistry] update() expected a plain object, received: ${typeof payload}`,
          );
        }
      },
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
