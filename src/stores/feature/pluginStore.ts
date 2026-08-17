// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { InstallOutcomeDto, PluginManifestDto, PluginSummaryDto, UpdateOutcomeDto } from "@/types";
import { create } from "zustand";

interface PluginState {
  plugins: PluginSummaryDto[];
  loading: boolean;
  error: string | null;
  installing: string | null;
  validating: boolean;

  loadPlugins: () => Promise<void>;
  validateSource: (source: string) => Promise<PluginManifestDto | null>;
  installPlugin: (source: string) => Promise<InstallOutcomeDto | null>;
  enablePlugin: (pluginId: string) => Promise<boolean>;
  disablePlugin: (pluginId: string) => Promise<boolean>;
  uninstallPlugin: (pluginId: string) => Promise<boolean>;
  updatePlugin: (pluginId: string) => Promise<UpdateOutcomeDto | null>;
}

export const usePluginStore = create<PluginState>((set, get) => ({
  plugins: [],
  loading: false,
  error: null,
  installing: null,
  validating: false,

  loadPlugins: async () => {
    set({ loading: true, error: null });
    try {
      const plugins = await invoke<PluginSummaryDto[]>("plugin_list");
      set({ plugins, loading: false });
    } catch (e) {
      const errorMsg = String(e);
      set({ error: errorMsg, loading: false });
      if (import.meta.env.DEV) {
        console.error("Failed to load plugins:", e);
      }
    }
  },

  validateSource: async (source) => {
    set({ validating: true, error: null });
    try {
      const manifest = await invoke<PluginManifestDto>("plugin_validate_source", {
        source,
      });
      set({ validating: false });
      return manifest;
    } catch (e) {
      const errorMsg = String(e);
      set({ error: errorMsg, validating: false });
      if (import.meta.env.DEV) {
        console.error("Failed to validate plugin source:", e);
      }
      return null;
    }
  },

  installPlugin: async (source) => {
    set({ installing: source, error: null });
    try {
      const outcome = await invoke<InstallOutcomeDto>("plugin_install", { source });
      set((s) => ({
        installing: null,
        plugins: [
          ...s.plugins,
          {
            id: outcome.pluginId,
            name: outcome.pluginId.split("@")[0] || outcome.pluginId,
            version: outcome.version,
            description: "",
            kind: source.startsWith("openclaw:") ? "openclaw" as const : "external" as const,
            enabled: true,
            tools: [],
            mcpServers: [],
            skills: [],
          },
        ],
      }));
      await get().loadPlugins();
      return outcome;
    } catch (e) {
      const errorMsg = String(e);
      set({ error: errorMsg, installing: null });
      if (import.meta.env.DEV) {
        console.error("Failed to install plugin:", e);
      }
      return null;
    }
  },

  enablePlugin: async (pluginId) => {
    try {
      await invoke("plugin_enable", { pluginId });
      set((s) => ({
        plugins: s.plugins.map((p) => p.id === pluginId ? { ...p, enabled: true } : p),
        error: null,
      }));
      return true;
    } catch (e) {
      const errorMsg = String(e);
      set({ error: errorMsg });
      if (import.meta.env.DEV) {
        console.error("Failed to enable plugin:", e);
      }
      return false;
    }
  },

  disablePlugin: async (pluginId) => {
    try {
      await invoke("plugin_disable", { pluginId });
      set((s) => ({
        plugins: s.plugins.map((p) => p.id === pluginId ? { ...p, enabled: false } : p),
        error: null,
      }));
      return true;
    } catch (e) {
      const errorMsg = String(e);
      set({ error: errorMsg });
      if (import.meta.env.DEV) {
        console.error("Failed to disable plugin:", e);
      }
      return false;
    }
  },

  uninstallPlugin: async (pluginId) => {
    try {
      await invoke("plugin_uninstall", { pluginId });
      set((s) => ({
        plugins: s.plugins.filter((p) => p.id !== pluginId),
        error: null,
      }));
      return true;
    } catch (e) {
      const errorMsg = String(e);
      set({ error: errorMsg });
      if (import.meta.env.DEV) {
        console.error("Failed to uninstall plugin:", e);
      }
      return false;
    }
  },

  updatePlugin: async (pluginId) => {
    try {
      const outcome = await invoke<UpdateOutcomeDto>("plugin_update", { pluginId });
      set((s) => ({
        plugins: s.plugins.map((p) => p.id === pluginId ? { ...p, version: outcome.newVersion } : p),
        error: null,
      }));
      return outcome;
    } catch (e) {
      const errorMsg = String(e);
      set({ error: errorMsg });
      if (import.meta.env.DEV) {
        console.error("Failed to update plugin:", e);
      }
      return null;
    }
  },
}));
