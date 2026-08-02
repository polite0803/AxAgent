// SPDX-License-Identifier: AGPL-3.0-only

import { translateBackendError } from "@/lib/errorI18n";
import { invoke, isTauri } from "@/lib/invoke";
import type {
  AddMemberInput,
  CreateFleetInput,
  DirectMessageInput,
  DispatchChatMessage,
  DispatchEvent,
  DispatchInput,
  Fleet,
  FleetMember,
  FleetMemberStatus,
  FleetStatus,
} from "@/types";
import { create } from "zustand";

interface OfficeState {
  // ── 数据字段（state）──
  fleets: Fleet[];
  /** 当前选中的舰队 ID（用于 UI 高亮 / 像素办公室渲染） */
  activeFleetId: string | null;
  /** 按舰队 ID 索引的成员列表缓存 */
  membersByFleet: Record<string, FleetMember[]>;
  /** 当前 dispatcher 事件流（Channel 实时追加） */
  dispatchEvents: DispatchEvent[];
  loading: boolean;
  error: string | null;

  // ── Actions ──
  loadFleets: (statusFilter?: FleetStatus) => Promise<Fleet[]>;
  selectFleet: (fleetId: string | null) => void;
  createFleet: (input: CreateFleetInput) => Promise<Fleet | null>;
  updateFleetStatus: (fleetId: string, status: FleetStatus) => Promise<void>;
  deleteFleet: (fleetId: string) => Promise<void>;
  loadMembers: (fleetId: string, force?: boolean) => Promise<FleetMember[]>;
  addMember: (input: AddMemberInput) => Promise<FleetMember | null>;
  updateMemberStatus: (
    memberId: string,
    status: FleetMemberStatus,
  ) => Promise<void>;
  removeMember: (memberId: string, fleetId: string) => Promise<void>;
  resetDailyTokens: (fleetId: string) => Promise<void>;
  dispatch: (input: DispatchInput) => Promise<DispatchEvent[]>;
  directMessage: (
    input: DirectMessageInput,
  ) => Promise<DispatchEvent[]>;
  clearDispatchEvents: () => void;
  clearError: () => void;
}

/** 把 dispatch 事件追加到事件流，并同步成员状态 / token 用量。 */
function applyDispatchEvent(
  set: (fn: (s: OfficeState) => Partial<OfficeState>) => void,
  get: () => OfficeState,
  evt: DispatchEvent,
  fleetId: string,
): void {
  // 追加事件流
  set((s) => ({ dispatchEvents: [...s.dispatchEvents, evt] }));

  // 成员状态 / token 实时回写（驱动 Phaser 精灵动画）
  if (evt.type === "agent_status" || evt.type === "token_usage") {
    set((s) => {
      const members = s.membersByFleet[fleetId] ?? [];
      const next = members.map((m) => {
        if (m.agentSlug !== evt.agentSlug) {
          return m;
        }
        if (evt.type === "agent_status") {
          return { ...m, status: evt.status };
        }
        return {
          ...m,
          todayTokens: m.todayTokens + evt.inputTokens + evt.outputTokens,
          totalTokens: m.totalTokens + evt.inputTokens + evt.outputTokens,
        };
      });
      return { membersByFleet: { ...s.membersByFleet, [fleetId]: next } };
    });
  }
  void get;
}

export const useOfficeStore = create<OfficeState>((set, get) => ({
  fleets: [],
  activeFleetId: null,
  membersByFleet: {},
  dispatchEvents: [],
  loading: false,
  error: null,

  loadFleets: async (statusFilter?: FleetStatus) => {
    set({ loading: true, error: null });
    try {
      const fleets = await invoke<Fleet[]>("fleet_list", {
        statusFilter: statusFilter ?? null,
      });
      set({ fleets, loading: false });
      return fleets;
    } catch (e) {
      set({ error: String(e), loading: false });
      return [];
    }
  },

  selectFleet: (fleetId) => {
    set({ activeFleetId: fleetId });
  },

  createFleet: async (input) => {
    set({ error: null });
    try {
      const fleet = await invoke<Fleet>("fleet_create", { input });
      set((s) => ({ fleets: [...s.fleets, fleet] }));
      return fleet;
    } catch (e) {
      const msg = translateBackendError(e);
      set({ error: msg });
      console.warn(`[officeStore] createFleet failed: ${msg}`);
      return null;
    }
  },

  updateFleetStatus: async (fleetId, status) => {
    set({ error: null });
    try {
      await invoke<void>("fleet_update_status", { fleetId, status });
      set((s) => ({
        fleets: s.fleets.map((f) => f.id === fleetId ? { ...f, status, updatedAt: Date.now() } : f),
      }));
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  deleteFleet: async (fleetId) => {
    set({ error: null });
    try {
      await invoke<void>("fleet_delete", { fleetId });
      set((s) => {
        const fleets = s.fleets.filter((f) => f.id !== fleetId);
        const membersByFleet = { ...s.membersByFleet };
        delete membersByFleet[fleetId];
        const activeFleetId = s.activeFleetId === fleetId ? null : s.activeFleetId;
        return { fleets, membersByFleet, activeFleetId };
      });
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  loadMembers: async (fleetId, force = false) => {
    set({ error: null });
    // 缓存命中且非强制刷新时直接返回缓存
    if (!force) {
      const cached = get().membersByFleet[fleetId];
      if (cached) {
        return cached;
      }
    }
    try {
      const members = await invoke<FleetMember[]>("fleet_list_members", {
        fleetId,
      });
      set((s) => ({
        membersByFleet: { ...s.membersByFleet, [fleetId]: members },
      }));
      return members;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return [];
    }
  },

  addMember: async (input) => {
    set({ error: null });
    try {
      const member = await invoke<FleetMember>("fleet_add_member", { input });
      set((s) => {
        const existing = s.membersByFleet[member.fleetId] ?? [];
        return {
          membersByFleet: {
            ...s.membersByFleet,
            [member.fleetId]: [...existing, member],
          },
        };
      });
      return member;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  updateMemberStatus: async (memberId, status) => {
    set({ error: null });
    try {
      await invoke<void>("fleet_update_member_status", { memberId, status });
      set((s) => {
        const membersByFleet: Record<string, FleetMember[]> = {};
        for (const [fid, members] of Object.entries(s.membersByFleet)) {
          membersByFleet[fid] = members.map((m) => m.id === memberId ? { ...m, status } : m);
        }
        return { membersByFleet };
      });
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  removeMember: async (memberId, fleetId) => {
    set({ error: null });
    try {
      await invoke<void>("fleet_remove_member", { memberId });
      set((s) => {
        const existing = s.membersByFleet[fleetId] ?? [];
        return {
          membersByFleet: {
            ...s.membersByFleet,
            [fleetId]: existing.filter((m) => m.id !== memberId),
          },
        };
      });
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  resetDailyTokens: async (fleetId) => {
    set({ error: null });
    try {
      await invoke<void>("fleet_reset_daily_tokens", { fleetId });
      set((s) => {
        const existing = s.membersByFleet[fleetId] ?? [];
        return {
          membersByFleet: {
            ...s.membersByFleet,
            [fleetId]: existing.map((m) => ({ ...m, todayTokens: 0 })),
          },
        };
      });
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  dispatch: async (input) => {
    set({ error: null, dispatchEvents: [] });
    const fleetId = input.fleetId;
    try {
      // 事件回传：Tauri 用 Channel（流式）；浏览器模式用 MockChannel 普通对象
      // （Tauri Channel 构造依赖 __TAURI_INTERNALS__.transformCallback，浏览器模式会抛错）
      if (isTauri()) {
        const { Channel } = await import("@tauri-apps/api/core");
        const channel = new Channel<DispatchEvent>();
        channel.onmessage = (evt) => {
          applyDispatchEvent(set, get, evt, fleetId);
        };
        await invoke<void>("fleet_dispatch", { input, onEvent: channel });
      } else {
        const mockChannel: { onmessage: (evt: DispatchEvent) => void } = {
          onmessage: (evt) => {
            applyDispatchEvent(set, get, evt, fleetId);
          },
        };
        await invoke<void>("fleet_dispatch", { input, onEvent: mockChannel });
      }
      return get().dispatchEvents;
    } catch (e) {
      const errorEvent: DispatchEvent = {
        type: "error",
        message: translateBackendError(e),
      };
      applyDispatchEvent(set, get, errorEvent, fleetId);
      return get().dispatchEvents;
    }
  },

  directMessage: async (input) => {
    set({ error: null, dispatchEvents: [] });
    const fleetId = input.fleetId;
    try {
      if (isTauri()) {
        const { Channel } = await import("@tauri-apps/api/core");
        const channel = new Channel<DispatchEvent>();
        channel.onmessage = (evt) => {
          applyDispatchEvent(set, get, evt, fleetId);
        };
        await invoke<void>("fleet_direct_message", { input, onEvent: channel });
      } else {
        const mockChannel: { onmessage: (evt: DispatchEvent) => void } = {
          onmessage: (evt) => {
            applyDispatchEvent(set, get, evt, fleetId);
          },
        };
        await invoke<void>("fleet_direct_message", { input, onEvent: mockChannel });
      }
      return get().dispatchEvents;
    } catch (e) {
      const errorEvent: DispatchEvent = {
        type: "error",
        message: translateBackendError(e),
      };
      applyDispatchEvent(set, get, errorEvent, fleetId);
      return get().dispatchEvents;
    }
  },

  clearDispatchEvents: () => {
    set({ dispatchEvents: [] });
  },

  clearError: () => {
    set({ error: null });
  },
}));

/** 历史消息构建辅助函数（前端构造 DispatchChatMessage 列表） */
export function buildDispatchHistory(
  messages: Array<{ role: string; content: string; agentSlug?: string }>,
): DispatchChatMessage[] {
  return messages.map((m) => ({
    role: m.role,
    content: m.content,
    agentSlug: m.agentSlug,
  }));
}
