import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface RLPolicy {
  id: string;
  name: string;
  policy_type: string;
  total_experiences: number;
  avg_reward: number;
}

export interface RLStats {
  total_policies: number;
  total_experiences: number;
  avg_reward: number;
  policies: RLPolicy[];
}

interface TrainingProgress {
  policy_id: string;
  status: "idle" | "training" | "completed" | "failed";
  episodes_completed: number;
  total_episodes: number;
  avg_reward: number;
}

export const useRLStore = create<{
  policies: RLPolicy[];
  stats: RLStats | null;
  selectedPolicy: RLPolicy | null;
  isLoading: boolean;
  error: string | null;
  trainingProgress: TrainingProgress | null;
  fetchPolicies: () => Promise<void>;
  fetchStats: () => Promise<void>;
  selectPolicy: (policyId: string) => Promise<void>;
  createPolicy: (name: string, policyType: string, modelId: string) => Promise<RLPolicy | null>;
  deletePolicy: (policyId: string) => Promise<void>;
  trainPolicy: (policyId: string) => Promise<void>;
  recordExperience: (
    taskId: string,
    taskType: string,
    toolId: string,
    toolName: string,
    reward: number
  ) => Promise<void>;
  exportModel: (policyId: string, path: string) => Promise<string | null>;
  importModel: (path: string) => Promise<RLPolicy | null>;
}>((set) => ({
  policies: [],
  stats: null,
  selectedPolicy: null,
  isLoading: false,
  error: null,
  trainingProgress: null,

  fetchPolicies: async () => {
    set({ isLoading: true, error: null });
    try {
      const policies = await invoke<RLPolicy[]>("rl_list_policies");
      set({ policies, isLoading: false });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch policies", isLoading: false });
    }
  },

  fetchStats: async () => {
    try {
      const stats = await invoke<RLStats>("rl_get_stats");
      set({ stats });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch stats" });
    }
  },

  selectPolicy: async (policyId: string) => {
    set({ isLoading: true });
    try {
      const policy = await invoke<RLPolicy | null>("rl_get_policy", { policyId });
      set({ selectedPolicy: policy, isLoading: false });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to select policy", isLoading: false });
    }
  },

  createPolicy: async (name: string, policyType: string, modelId: string) => {
    set({ isLoading: true, error: null });
    try {
      const policy = await invoke<RLPolicy>("rl_create_policy", { name, policyType, modelId });
      set((state) => ({ policies: [...state.policies, policy], isLoading: false }));
      return policy;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to create policy", isLoading: false });
      return null;
    }
  },

  deletePolicy: async (policyId: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("rl_delete_policy", { policyId });
      set((state) => ({
        policies: state.policies.filter((p) => p.id !== policyId),
        selectedPolicy: state.selectedPolicy?.id === policyId ? null : state.selectedPolicy,
        isLoading: false,
      }));
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to delete policy", isLoading: false });
    }
  },

  trainPolicy: async (policyId: string) => {
    set({
      isLoading: true,
      error: null,
      trainingProgress: {
        policy_id: policyId,
        status: "training",
        episodes_completed: 0,
        total_episodes: 100,
        avg_reward: 0,
      },
    });
    try {
      await invoke("rl_train_policy", { policyId });
      set((state) => ({
        trainingProgress: state.trainingProgress ? { ...state.trainingProgress, status: "completed" } : null,
        isLoading: false,
      }));
    } catch (error) {
      set((state) => ({
        error: error instanceof Error ? error.message : "Training failed",
        isLoading: false,
        trainingProgress: state.trainingProgress ? { ...state.trainingProgress, status: "failed" as const } : null,
      }));
    }
  },

  recordExperience: async (
    taskId: string,
    taskType: string,
    toolId: string,
    toolName: string,
    reward: number
  ) => {
    try {
      await invoke("rl_record_experience", { taskId, taskType, toolId, toolName, reward });
    } catch (error) {
      console.error("Failed to record experience:", error);
    }
  },

  exportModel: async (policyId: string, path: string) => {
    try {
      return await invoke<string>("rl_export_model", { policyId, path });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to export model" });
      return null;
    }
  },

  importModel: async (path: string) => {
    try {
      const policy = await invoke<RLPolicy>("rl_import_model", { path });
      set((state) => ({ policies: [...state.policies, policy] }));
      return policy;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to import model" });
      return null;
    }
  },
}));
