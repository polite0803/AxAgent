import { invoke } from "@/lib/invoke";
import type { Benchmark, BenchmarkReport, BenchmarkResult, Dataset, RunnerConfig } from "@/types/evaluator";
import { create } from "zustand";

interface EvaluatorState {
  benchmarks: Benchmark[];
  datasets: Dataset[];
  selectedBenchmark: Benchmark | null;
  currentResult: BenchmarkResult | null;
  currentReport: BenchmarkReport | null;
  history: BenchmarkResult[];
  isLoading: boolean;
  isRunning: boolean;
  error: string | null;
  config: RunnerConfig;

  loadBenchmarks: () => Promise<void>;
  loadDatasets: () => Promise<void>;
  selectBenchmark: (id: string) => void;
  runBenchmark: (benchmarkId: string, config?: RunnerConfig) => Promise<void>;
  generateReport: () => Promise<void>;
  exportReport: (format: "json" | "markdown") => Promise<void>;
  clearResult: () => void;
  clearHistory: () => void;
  setConfig: (config: Partial<RunnerConfig>) => void;
}

export const useEvaluatorStore = create<EvaluatorState>((set, get) => ({
  benchmarks: [],
  datasets: [],
  selectedBenchmark: null,
  currentResult: null,
  currentReport: null,
  history: [],
  isLoading: false,
  isRunning: false,
  error: null,
  config: {
    max_concurrency: 3,
    timeout_ms: 60000,
    include_traces: true,
  },

  loadBenchmarks: async () => {
    set({ isLoading: true, error: null });
    try {
      const benchmarks = await invoke<Benchmark[]>("evaluator_list_benchmarks");
      set({ benchmarks, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to load benchmarks",
        isLoading: false,
      });
    }
  },

  loadDatasets: async () => {
    set({ isLoading: true, error: null });
    try {
      const datasets = await invoke<Dataset[]>("evaluator_list_datasets");
      set({ datasets, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to load datasets",
        isLoading: false,
      });
    }
  },

  selectBenchmark: (id: string) => {
    const { benchmarks } = get();
    const benchmark = benchmarks.find((b) => b.id === id) || null;
    set({ selectedBenchmark: benchmark });
  },

  runBenchmark: async (benchmarkId: string, config?: RunnerConfig) => {
    set({ isRunning: true, error: null });
    try {
      const runnerConfig = config || get().config;
      const result = await invoke<BenchmarkResult>("evaluator_run_benchmark", {
        benchmarkId,
        config: runnerConfig,
      });
      const { history } = get();
      set({
        currentResult: result,
        history: [...history, result],
        isRunning: false,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to run benchmark",
        isRunning: false,
      });
    }
  },

  generateReport: async () => {
    const { currentResult } = get();
    if (!currentResult) {
      set({ error: "No result to generate report from" });
      return;
    }

    set({ isLoading: true, error: null });
    try {
      const report = await invoke<BenchmarkReport>("evaluator_generate_report", {
        result: currentResult,
      });
      set({ currentReport: report, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to generate report",
        isLoading: false,
      });
    }
  },

  exportReport: async (format: "json" | "markdown") => {
    const { currentReport } = get();
    if (!currentReport) {
      set({ error: "No report to export" });
      return;
    }

    try {
      await invoke("evaluator_export_report", {
        report: currentReport,
        format,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to export report",
      });
    }
  },

  clearResult: () => {
    set({ currentResult: null, currentReport: null });
  },

  clearHistory: () => {
    set({ history: [], currentResult: null, currentReport: null });
  },

  setConfig: (config: Partial<RunnerConfig>) => {
    const { config: currentConfig } = get();
    set({ config: { ...currentConfig, ...config } });
  },
}));
