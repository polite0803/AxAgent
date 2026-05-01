import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface DatasetInfo {
  id: string;
  name: string;
  description: string;
  num_samples: number;
  created_at: string;
}

export interface TrainingJobInfo {
  id: string;
  status: string;
  dataset_id: string;
  base_model: string;
  progress_percent: number;
  current_loss: number;
  output_lora: string | null;
}

export interface TrainingStats {
  total_jobs: number;
  completed_jobs: number;
  running_jobs: number;
  failed_jobs: number;
}

export interface BaseModelInfo {
  model_id: string;
  name: string;
  path: string;
  size_gb: number;
  context_length: number;
  supports_lora: boolean;
}

export interface LoRAAdapterInfo {
  adapter_id: string;
  name: string;
  base_model: string;
  lora_path: string;
  rank: number;
  alpha: number;
  training_date: string;
  performance_score: number;
}

export interface LoRAConfig {
  rank: number;
  alpha: number;
  learning_rate: number;
  batch_size: number;
  epochs: number;
}

export const useFineTuneStore = create<{
  datasets: DatasetInfo[];
  trainingJobs: TrainingJobInfo[];
  stats: TrainingStats | null;
  baseModels: BaseModelInfo[];
  loraAdapters: LoRAAdapterInfo[];
  selectedDataset: DatasetInfo | null;
  selectedJob: TrainingJobInfo | null;
  isLoading: boolean;
  error: string | null;
  fetchDatasets: () => Promise<void>;
  fetchDataset: (id: string) => Promise<DatasetInfo | null>;
  createDataset: (name: string, description: string) => Promise<DatasetInfo | null>;
  deleteDataset: (id: string) => Promise<void>;
  addSample: (datasetId: string, input: string, output: string, systemPrompt?: string) => Promise<void>;
  fetchTrainingJobs: () => Promise<void>;
  fetchTrainingJob: (id: string) => Promise<TrainingJobInfo | null>;
  createTrainingJob: (
    datasetId: string,
    baseModel: string,
    config: LoRAConfig
  ) => Promise<TrainingJobInfo | null>;
  startTrainingJob: (id: string) => Promise<void>;
  cancelTrainingJob: (id: string) => Promise<void>;
  deleteTrainingJob: (id: string) => Promise<void>;
  fetchTrainingStats: () => Promise<void>;
  fetchBaseModels: () => Promise<void>;
  fetchLoRAAdapters: () => Promise<void>;
}>((set, get) => ({
  datasets: [],
  trainingJobs: [],
  stats: null,
  baseModels: [],
  loraAdapters: [],
  selectedDataset: null,
  selectedJob: null,
  isLoading: false,
  error: null,

  fetchDatasets: async () => {
    set({ isLoading: true, error: null });
    try {
      const datasets = await invoke<DatasetInfo[]>("list_datasets");
      set({ datasets, isLoading: false });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch datasets", isLoading: false });
    }
  },

  fetchDataset: async (id: string) => {
    try {
      const dataset = await invoke<DatasetInfo>("get_dataset", { datasetId: id });
      set({ selectedDataset: dataset });
      return dataset;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch dataset" });
      return null;
    }
  },

  createDataset: async (name: string, description: string) => {
    set({ isLoading: true, error: null });
    try {
      const dataset = await invoke<DatasetInfo>("create_dataset", { name, description });
      set((state) => ({ datasets: [...state.datasets, dataset], isLoading: false }));
      return dataset;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to create dataset", isLoading: false });
      return null;
    }
  },

  deleteDataset: async (id: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("delete_dataset", { datasetId: id });
      set((state) => ({
        datasets: state.datasets.filter((d) => d.id !== id),
        selectedDataset: state.selectedDataset?.id === id ? null : state.selectedDataset,
        isLoading: false,
      }));
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to delete dataset", isLoading: false });
    }
  },

  addSample: async (datasetId: string, input: string, output: string, systemPrompt?: string) => {
    try {
      await invoke("add_sample", { datasetId, input, output, systemPrompt });
      await get().fetchDatasets();
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to add sample" });
    }
  },

  fetchTrainingJobs: async () => {
    set({ isLoading: true, error: null });
    try {
      const jobs = await invoke<TrainingJobInfo[]>("list_training_jobs");
      set({ trainingJobs: jobs, isLoading: false });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch training jobs", isLoading: false });
    }
  },

  fetchTrainingJob: async (id: string) => {
    try {
      const job = await invoke<TrainingJobInfo>("get_training_job", { jobId: id });
      set({ selectedJob: job });
      return job;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch training job" });
      return null;
    }
  },

  createTrainingJob: async (
    datasetId: string,
    baseModel: string,
    config: LoRAConfig
  ) => {
    set({ isLoading: true, error: null });
    try {
      const job = await invoke<TrainingJobInfo>("create_training_job", {
        datasetId,
        baseModel,
        rank: config.rank,
        alpha: config.alpha,
        learningRate: config.learning_rate,
        batchSize: config.batch_size,
        epochs: config.epochs,
      });
      set((state) => ({ trainingJobs: [...state.trainingJobs, job], isLoading: false }));
      return job;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to create training job", isLoading: false });
      return null;
    }
  },

  startTrainingJob: async (id: string) => {
    try {
      await invoke("start_training_job", { jobId: id });
      await get().fetchTrainingJobs();
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to start training job" });
    }
  },

  cancelTrainingJob: async (id: string) => {
    try {
      await invoke("cancel_training_job", { jobId: id });
      await get().fetchTrainingJobs();
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to cancel training job" });
    }
  },

  deleteTrainingJob: async (id: string) => {
    try {
      await invoke("delete_training_job", { jobId: id });
      set((state) => ({
        trainingJobs: state.trainingJobs.filter((j) => j.id !== id),
        selectedJob: state.selectedJob?.id === id ? null : state.selectedJob,
      }));
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to delete training job" });
    }
  },

  fetchTrainingStats: async () => {
    try {
      const stats = await invoke<TrainingStats>("get_training_stats");
      set({ stats });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch training stats" });
    }
  },

  fetchBaseModels: async () => {
    try {
      const baseModels = await invoke<BaseModelInfo[]>("list_base_models");
      set({ baseModels });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch base models" });
    }
  },

  fetchLoRAAdapters: async () => {
    try {
      const loraAdapters = await invoke<LoRAAdapterInfo[]>("list_lora_adapters");
      set({ loraAdapters });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch LoRA adapters" });
    }
  },

  setActiveModel: async (modelId: string, adapterId?: string) => {
    try {
      await invoke("set_active_model", { modelId, adapterId });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to set active model" });
    }
  },

  getActiveModel: async () => {
    try {
      const info = await invoke<{ modelId: string; adapterId?: string }>("get_active_model");
      return info;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to get active model" });
      return null;
    }
  },
}));
