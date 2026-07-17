// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import { invoke, logAndNotify } from "@/lib/invoke";
import { create } from "zustand";

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
  deleteCheckpoint: (id: string) => Promise<void>;
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
  checkpoints: [],
  error: null,
  _intervalId: null,

  startTraining: async (config: RLTrainingConfig) => {
    set({ status: "running", config, metricsHistory: [], error: null, currentMetrics: null });

    const existing = get()._intervalId;
    if (existing !== null) { clearInterval(existing); }

    let step = 0;
    const maxSteps = config.maxSteps;
    const fetchMetrics = () => {
      const state = get();
      if (state.status !== "running") { return; }

      if (step >= maxSteps) {
        const id = state._intervalId;
        if (id !== null) { clearInterval(id); }
        set({ status: "completed", _intervalId: null });
        return;
      }

      invoke<TrainingMetrics>("get_training_metrics", { step })
        .then((metrics) => {
          set((s) => ({
            currentMetrics: metrics,
            metricsHistory: [...s.metricsHistory.slice(-499), metrics],
            error: null,
          }));
        })
        .catch((err) => {
          set({ error: String(err), status: "failed" });
        });

      step += 10;
    };

    try {
      const trainingId = await invoke<string>("start_rl_training", { config });
      set({ trainingId });
    } catch (err) {
      set({ error: String(err), status: "failed" });
      return;
    }

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
      set({ error: String(err) });
    }

    set({
      status: state.status === "running" ? "paused" : state.status,
      _intervalId: null,
    });
  },

  fetchMetrics: () => {
    const state = get();
    if (state.status !== "running") { return; }

    const step = state.metricsHistory.length > 0
      ? state.metricsHistory[state.metricsHistory.length - 1].step + 10
      : 0;

    invoke<TrainingMetrics>("get_training_metrics", { step })
      .then((metrics) => {
        set((s) => ({
          currentMetrics: metrics,
          metricsHistory: [...s.metricsHistory.slice(-499), metrics],
          error: null,
        }));
      })
      .catch((err) => {
        set({ error: String(err), status: "failed" });
      });
  },

  saveCheckpoint: async (name: string) => {
    const state = get();
    const metrics = state.currentMetrics;
    if (!metrics) { return; }

    const newCheckpoint: CheckpointInfo = {
      id: `ckpt_${Date.now()}`,
      name,
      step: metrics.step,
      loss: metrics.loss,
      reward: metrics.reward,
      timestamp: Date.now(),
    };

    try {
      await invoke("save_checkpoint", { ...newCheckpoint });
      set((s) => ({ checkpoints: [...s.checkpoints, newCheckpoint] }));
    } catch (err) {
      logAndNotify(i18n.t("rlTrainingStore.saveCheckpoint"))(err);
    }
  },

  loadCheckpoint: async (id: string) => {
    try {
      await invoke("load_checkpoint", { checkpointId: id });
    } catch (err) {
      logAndNotify(i18n.t("rlTrainingStore.loadCheckpoint"))(err);
    }
  },

  listCheckpoints: async () => {
    try {
      const checkpoints = await invoke<CheckpointInfo[]>("list_checkpoints");
      set({ checkpoints });
    } catch (err) {
      logAndNotify(i18n.t("rlTrainingStore.listCheckpoints"))(err);
    }
  },

  deleteCheckpoint: async (id: string) => {
    set((s) => ({ checkpoints: s.checkpoints.filter((c) => c.id !== id) }));
    try {
      await invoke("delete_checkpoint", { checkpointId: id });
    } catch (err) {
      logAndNotify(i18n.t("rlTrainingStore.deleteCheckpoint"))(err);
    }
  },
}));
