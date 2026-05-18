import { invoke } from "@/lib/invoke";
import type { TrajectoryDetail, TrajectorySummary } from "@/types";
import { create } from "zustand";

interface TrajectoryStore {
  trajectoriesByConversation: Record<string, TrajectorySummary[]>;
  trajectoryDetails: Record<string, TrajectoryDetail | null>;
  loadingList: boolean;
  loadingDetail: Record<string, boolean>;

  fetchTrajectoryList: (conversationId: string) => Promise<void>;
  fetchTrajectoryDetail: (
    trajectoryId: string,
  ) => Promise<TrajectoryDetail | null>;
  clearConversation: (conversationId: string) => void;
}

export const useTrajectoryStore = create<TrajectoryStore>((set, get) => ({
  trajectoriesByConversation: {},
  trajectoryDetails: {},
  loadingList: false,
  loadingDetail: {},

  fetchTrajectoryList: async (conversationId: string) => {
    // 已有缓存则跳过
    if (get().trajectoriesByConversation[conversationId]) {
      return;
    }

    set({ loadingList: true });
    try {
      const result = await invoke<TrajectorySummary[]>("trajectory_list", {
        sessionId: conversationId,
        limit: 20,
      });
      set((s) => ({
        trajectoriesByConversation: {
          ...s.trajectoriesByConversation,
          [conversationId]: result,
        },
      }));
    } catch {
      // 轨迹服务可能未初始化，静默处理
    } finally {
      set({ loadingList: false });
    }
  },

  fetchTrajectoryDetail: async (trajectoryId: string) => {
    if (get().trajectoryDetails[trajectoryId] !== undefined) {
      return get().trajectoryDetails[trajectoryId];
    }

    set((s) => ({
      loadingDetail: { ...s.loadingDetail, [trajectoryId]: true },
    }));
    try {
      const result = await invoke<TrajectoryDetail>("get_trajectory_detail", {
        trajectoryId,
      });
      set((s) => ({
        trajectoryDetails: { ...s.trajectoryDetails, [trajectoryId]: result },
      }));
      return result;
    } catch {
      set((s) => ({
        trajectoryDetails: { ...s.trajectoryDetails, [trajectoryId]: null },
      }));
      return null;
    } finally {
      set((s) => ({
        loadingDetail: { ...s.loadingDetail, [trajectoryId]: false },
      }));
    }
  },

  clearConversation: (conversationId: string) => {
    set((s) => {
      const { [conversationId]: _, ...rest } = s.trajectoriesByConversation;
      return { trajectoriesByConversation: rest };
    });
  },
}));
