// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type {
  CapabilityDiscoveryResult,
  CapabilityIndexStats,
  CapabilityPassportDto,
  DiscoverRequestPayload,
  IndexResult,
} from "@/types";
import i18next from "i18next";
import { create } from "zustand";

/**
 * 解析后端 ErrorResponse 并返回 i18n 翻译后的错误消息
 *
 * 注意：此处不能调用 useTranslation()（React Hook 只能在组件/Hook 顶层调用），
 * 故直接使用 i18next 实例的 t 方法，与组件渲染走的是同一翻译资源。
 */
function translateCapabilityError(err: unknown): string {
  const message = err instanceof Error ? err.message : String(err);

  try {
    const parsed = JSON.parse(message);
    if (parsed?.code && typeof parsed.code === "string") {
      return String(i18next.t(`error.${parsed.code}`, parsed.params ?? {}));
    }
  } catch {
    // 非 JSON 格式，直接返回原始消息
  }

  return message;
}

interface CapabilityState {
  passports: CapabilityPassportDto[];
  discoveryResult: CapabilityDiscoveryResult | null;
  stats: CapabilityIndexStats | null;
  isLoading: boolean;
  isDiscovering: boolean;
  error: string | null;

  registerPassport: (
    passport: CapabilityPassportDto,
  ) => Promise<IndexResult>;
  registerBatch: (
    passports: CapabilityPassportDto[],
  ) => Promise<IndexResult[]>;
  removePassport: (capabilityId: string) => Promise<void>;
  listPassports: () => Promise<void>;
  getStats: () => Promise<void>;
  discover: (
    payload: DiscoverRequestPayload,
  ) => Promise<CapabilityDiscoveryResult>;
  setError: (error: string | null) => void;
  clearResult: () => void;
}

export const useCapabilityStore = create<CapabilityState>((set) => ({
  passports: [],
  discoveryResult: null,
  stats: null,
  isLoading: false,
  isDiscovering: false,
  error: null,

  registerPassport: async (passport) => {
    set({ isLoading: true, error: null });
    try {
      const result = await invoke<IndexResult>("capability_register_passport", {
        request: { passport },
      });
      return result;
    } catch (e) {
      const msg = translateCapabilityError(e);
      set({ error: msg });
      throw e;
    } finally {
      set({ isLoading: false });
    }
  },

  registerBatch: async (passports) => {
    set({ isLoading: true, error: null });
    try {
      const results = await invoke<IndexResult[]>(
        "capability_register_batch",
        { passports },
      );
      return results;
    } catch (e) {
      const msg = translateCapabilityError(e);
      set({ error: msg });
      throw e;
    } finally {
      set({ isLoading: false });
    }
  },

  removePassport: async (capabilityId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<void>("capability_remove_passport", {
        capabilityId,
      });
      set((state) => ({
        passports: state.passports.filter(
          (p) => p.capability_id !== capabilityId,
        ),
      }));
    } catch (e) {
      const msg = translateCapabilityError(e);
      set({ error: msg });
      throw e;
    } finally {
      set({ isLoading: false });
    }
  },

  listPassports: async () => {
    set({ isLoading: true, error: null });
    try {
      const passports = await invoke<CapabilityPassportDto[]>(
        "capability_list_passports",
      );
      set({ passports });
    } catch (e) {
      const msg = translateCapabilityError(e);
      set({ error: msg });
    } finally {
      set({ isLoading: false });
    }
  },

  getStats: async () => {
    set({ isLoading: true, error: null });
    try {
      const stats = await invoke<CapabilityIndexStats>(
        "capability_get_stats",
      );
      set({ stats });
    } catch (e) {
      const msg = translateCapabilityError(e);
      set({ error: msg });
    } finally {
      set({ isLoading: false });
    }
  },

  discover: async (payload) => {
    set({ isDiscovering: true, error: null });
    try {
      const result = await invoke<CapabilityDiscoveryResult>(
        "capability_discover",
        { request: payload },
      );
      set({ discoveryResult: result });
      return result;
    } catch (e) {
      const msg = translateCapabilityError(e);
      set({ error: msg });
      throw e;
    } finally {
      set({ isDiscovering: false });
    }
  },

  setError: (error) => set({ error }),
  clearResult: () => set({ discoveryResult: null }),
}));
