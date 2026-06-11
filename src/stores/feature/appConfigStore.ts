// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import type { SystemPermissionMode } from "@/types";
import { create } from "zustand";

export interface FeatureFlags {
  forkSubagent: boolean;
  coordinatorMode: boolean;
  proactiveMode: boolean;
  swarmMode: boolean;
  toolConcurrency: boolean;
  verificationAgent: boolean;
  dreamTask: boolean;
}

export type ModelTier = "opus" | "sonnet" | "haiku";
export type PermissionMode = SystemPermissionMode;

const DEFAULT_FEATURE_FLAGS: FeatureFlags = {
  forkSubagent: false,
  coordinatorMode: false,
  proactiveMode: true,
  swarmMode: false,
  toolConcurrency: true,
  verificationAgent: false,
  dreamTask: true,
};

interface AppConfigState {
  model: ModelTier;
  permissionMode: PermissionMode;
  maxIterations: number;
  features: FeatureFlags;
  loading: boolean;
  error: string | null;

  setModel: (model: ModelTier) => void;
  setPermissionMode: (mode: PermissionMode) => void;
  setMaxIterations: (n: number) => void;
  toggleFeature: (name: keyof FeatureFlags) => void;
  loadConfig: () => Promise<void>;
  saveConfig: () => Promise<void>;
}

export const useAppConfigStore = create<AppConfigState>((set, get) => ({
  model: "sonnet",
  permissionMode: "workspace-write",
  maxIterations: 50,
  features: { ...DEFAULT_FEATURE_FLAGS },
  loading: false,
  error: null,

  setModel: (model) => {
    set({ model });
  },

  setPermissionMode: (mode) => {
    set({ permissionMode: mode });
  },

  setMaxIterations: (n) => {
    set({ maxIterations: Math.max(1, Math.min(100, Math.round(n))) });
  },

  toggleFeature: (name) =>
    set((state) => {
      const newValue = !state.features[name];
      if (name === "proactiveMode") {
        invoke("proactive_set_enabled", { enabled: newValue }).catch(logIpcError("proactive_set_enabled"));
      }
      return {
        features: { ...state.features, [name]: newValue },
      };
    }),

  loadConfig: async () => {
    set({ loading: true, error: null });
    try {
      const data = await invoke<Partial<AppConfigState>>("get_app_config");
      if (data) {
        set((state) => ({
          model: (data.model as ModelTier) ?? state.model,
          permissionMode: (data.permissionMode as PermissionMode) ?? state.permissionMode,
          maxIterations: data.maxIterations ?? state.maxIterations,
          features: data.features
            ? { ...DEFAULT_FEATURE_FLAGS, ...data.features }
            : state.features,
          loading: false,
        }));
      } else {
        set({ loading: false });
      }
    } catch (e) {
      logIpcError("appConfigStore: 加载配置失败")(e);
      set({ loading: false, error: String(e) });
    }
  },

  saveConfig: async () => {
    const state = get();
    try {
      await invoke("save_app_config", {
        config: {
          model: state.model,
          permissionMode: state.permissionMode,
          maxIterations: state.maxIterations,
          features: state.features,
        },
      });
    } catch (e) {
      logIpcError("appConfigStore: 保存配置失败")(e);
      set({ error: String(e) });
    }
  },
}));
