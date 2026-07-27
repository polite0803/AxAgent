// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
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
  /** 当前 dispatcher 事件流（前端 SSE 消费） */
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

export const useOfficeStore = create<OfficeState>((set, _get) => ({
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
      const msg = String(e);
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
      set({ error: String(e) });
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
      set({ error: String(e) });
    }
  },

  loadMembers: async (fleetId, force = false) => {
    set({ error: null });
    // 缓存命中且非强制刷新时直接返回缓存
    if (!force) {
      const cached = _get().membersByFleet[fleetId];
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
      set({ error: String(e) });
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
      set({ error: String(e) });
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
      set({ error: String(e) });
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
      set({ error: String(e) });
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
      set({ error: String(e) });
    }
  },

  dispatch: async (input) => {
    set({ error: null, dispatchEvents: [] });
    try {
      const events = await invoke<DispatchEvent[]>("fleet_dispatch", { input });
      set({ dispatchEvents: events });
      return events;
    } catch (e) {
      const errorEvent: DispatchEvent = {
        type: "error",
        message: String(e),
      };
      set({ dispatchEvents: [errorEvent], error: String(e) });
      return [errorEvent];
    }
  },

  directMessage: async (input) => {
    set({ error: null, dispatchEvents: [] });
    try {
      const events = await invoke<DispatchEvent[]>("fleet_direct_message", {
        input,
      });
      set({ dispatchEvents: events });
      return events;
    } catch (e) {
      const errorEvent: DispatchEvent = {
        type: "error",
        message: String(e),
      };
      set({ dispatchEvents: [errorEvent], error: String(e) });
      return [errorEvent];
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
