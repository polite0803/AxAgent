// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { create } from "zustand";

// ── Types ──

export interface RLTrainingConfig {
  algorithm: "ppo" | "grpo" | "dpo" | "rlhf";
  learningRate: number;
  batchSize: number;
  epochs: number;
  maxSteps: number;
}

export interface TrainingMetrics {
  step: number;
  loss: number;
  reward: number;
  policyLoss: number;
  valueLoss: number;
  timestamp: number;
}

export interface CheckpointInfo {
  id: string;
  name: string;
  step: number;
  loss: number;
  reward: number;
  timestamp: number;
}

type TrainingStatus = "idle" | "running" | "paused" | "completed" | "failed";

interface RlTrainingState {
  trainingId: string | null;
  status: TrainingStatus;
  config: RLTrainingConfig;
  currentMetrics: TrainingMetrics | null;
  metricsHistory: TrainingMetrics[];
  checkpoints: CheckpointInfo[];
  error: string | null;
  _intervalId: ReturnType<typeof setInterval> | null;

  startTraining: (config: RLTrainingConfig) => Promise<void>;
  stopTraining: () => Promise<void>;
  fetchMetrics: () => void;
  saveCheckpoint: (name: string) => Promise<void>;
  loadCheckpoint: (id: string) => Promise<void>;
  listCheckpoints: () => Promise<void>;
}

function generateMockMetrics(step: number): TrainingMetrics {
  const baseLoss = 2.5 * Math.exp(-step * 0.002);
  const noise = (Math.random() - 0.5) * 0.1;
  return {
    step,
    loss: Math.max(0.01, baseLoss + noise),
    reward: Math.min(1.0, 0.2 + 0.8 * (1 - Math.exp(-step * 0.001)) + (Math.random() - 0.5) * 0.05),
    policyLoss: Math.max(0.01, baseLoss * 0.6 + (Math.random() - 0.5) * 0.05),
    valueLoss: Math.max(0.01, baseLoss * 0.4 + (Math.random() - 0.5) * 0.05),
    timestamp: Date.now(),
  };
}

function generateMockCheckpoints(): CheckpointInfo[] {
  const now = Date.now();
  return [
    { id: "ckpt_001", name: "初始检查点", step: 0, loss: 2.51, reward: 0.20, timestamp: now - 3600000 },
    { id: "ckpt_002", name: "500步", step: 500, loss: 1.82, reward: 0.45, timestamp: now - 2400000 },
    { id: "ckpt_003", name: "1000步", step: 1000, loss: 1.21, reward: 0.62, timestamp: now - 1200000 },
    { id: "ckpt_004", name: "2000步-最佳", step: 2000, loss: 0.58, reward: 0.81, timestamp: now - 600000 },
  ];
}

export const useRlTrainingStore = create<RlTrainingState>((set, get) => ({
  trainingId: null,
  status: "idle",
  config: {
    algorithm: "ppo",
    learningRate: 1e-5,
    batchSize: 64,
    epochs: 10,
    maxSteps: 10000,
  },
  currentMetrics: null,
  metricsHistory: [],
  checkpoints: generateMockCheckpoints(),
  error: null,
  _intervalId: null,

  startTraining: async (config: RLTrainingConfig) => {
    set({ status: "running", config, metricsHistory: [], error: null, currentMetrics: null });

    // Clear any existing interval
    const existing = get()._intervalId;
    if (existing !== null) clearInterval(existing);

    let step = 0;
    const maxSteps = config.maxSteps;
    const fetchMetrics = () => {
      const state = get();
      if (state.status !== "running") return;

      if (step >= maxSteps) {
        const id = state._intervalId;
        if (id !== null) clearInterval(id);
        set({ status: "completed", _intervalId: null });
        return;
      }

      try {
        // Try real backend first
        invoke<TrainingMetrics>("get_training_metrics", { step })
          .then((metrics) => {
            set((s) => ({
              currentMetrics: metrics,
              metricsHistory: [...s.metricsHistory.slice(-499), metrics],
            }));
          })
          .catch(() => {
            // Fallback to mock
            const mockMetrics = generateMockMetrics(step);
            set((s) => ({
              currentMetrics: mockMetrics,
              metricsHistory: [...s.metricsHistory.slice(-499), mockMetrics],
            }));
          });
      } catch {
        const mockMetrics = generateMockMetrics(step);
        set((s) => ({
          currentMetrics: mockMetrics,
          metricsHistory: [...s.metricsHistory.slice(-499), mockMetrics],
        }));
      }

      step += 10;
    };

    // Try real backend first
    try {
      const trainingId = await invoke<string>("start_rl_training", { config });
      set({ trainingId });
    } catch (err) {
      console.warn("[rlTrainingStore] startTraining invoke failed, using mock simulation", err);
    }

    // Run first metrics fetch immediately
    fetchMetrics();

    const intervalId = setInterval(fetchMetrics, 2000);
    set({ _intervalId: intervalId });
  },

  stopTraining: async () => {
    const state = get();
    const id = state._intervalId;
    if (id !== null) {
      clearInterval(id);
    }

    try {
      if (state.trainingId) {
        await invoke("stop_rl_training", { trainingId: state.trainingId });
      }
    } catch (err) {
      console.warn("[rlTrainingStore] stopTraining invoke failed", err);
    }

    set({
      status: state.status === "running" ? "paused" : state.status,
      _intervalId: null,
    });
  },

  fetchMetrics: () => {
    // Called externally; internal interval handles this
    const state = get();
    if (state.status !== "running") return;

    const step = state.metricsHistory.length > 0
      ? state.metricsHistory[state.metricsHistory.length - 1].step + 10
      : 0;

    try {
      invoke<TrainingMetrics>("get_training_metrics", { step })
        .then((metrics) => {
          set((s) => ({
            currentMetrics: metrics,
            metricsHistory: [...s.metricsHistory.slice(-499), metrics],
          }));
        })
        .catch(() => {
          const mockMetrics = generateMockMetrics(step);
          set((s) => ({
            currentMetrics: mockMetrics,
            metricsHistory: [...s.metricsHistory.slice(-499), mockMetrics],
          }));
        });
    } catch {
      const mockMetrics = generateMockMetrics(step);
      set((s) => ({
        currentMetrics: mockMetrics,
        metricsHistory: [...s.metricsHistory.slice(-499), mockMetrics],
      }));
    }
  },

  saveCheckpoint: async (name: string) => {
    const state = get();
    const metrics = state.currentMetrics;
    if (!metrics) return;

    const newCheckpoint: CheckpointInfo = {
      id: `ckpt_${Date.now()}`,
      name,
      step: metrics.step,
      loss: metrics.loss,
      reward: metrics.reward,
      timestamp: Date.now(),
    };

    try {
      await invoke("save_checkpoint", { name, ...newCheckpoint });
    } catch (err) {
      console.warn("[rlTrainingStore] saveCheckpoint invoke failed, using mock", err);
    }

    set((s) => ({ checkpoints: [...s.checkpoints, newCheckpoint] }));
  },

  loadCheckpoint: async (id: string) => {
    try {
      await invoke("load_checkpoint", { checkpointId: id });
    } catch (err) {
      console.warn("[rlTrainingStore] loadCheckpoint invoke failed", err);
    }
  },

  listCheckpoints: async () => {
    try {
      const checkpoints = await invoke<CheckpointInfo[]>("list_checkpoints");
      set({ checkpoints });
    } catch (err) {
      console.warn("[rlTrainingStore] listCheckpoints failed, using mock", err);
    }
  },
}));
