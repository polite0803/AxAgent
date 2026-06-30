// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { create } from "zustand";

// ── Types ──

export interface EngineLog {
  timestamp: number;
  level: "info" | "warn" | "error";
  message: string;
}

export interface EngineStatus {
  name: string;
  displayName: string;
  description: string;
  category: "core" | "learning" | "safety" | "experimental";
  running: boolean;
  config: Record<string, unknown>;
  stats: Record<string, unknown>;
  logs: EngineLog[];
  lastActive?: number;
}

export interface SkillVersion {
  version: number;
  timestamp: number;
  summary: string;
  metrics: Record<string, { before: number; after: number }>;
  promptDiff?: { old: string; new: string };
}

export interface ABTestResult {
  metric: string;
  valueA: number;
  valueB: number;
  change: number;
  winner: "A" | "B" | "tie";
}

export interface EvolutionEvent {
  engine: string;
  timestamp: number;
  type: "started" | "stopped" | "evolved" | "config_changed" | "error";
  detail: string;
}

// ── Mock Data ──

const ENGINE_DEFINITIONS: Omit<EngineStatus, "running" | "config" | "stats" | "logs" | "lastActive">[] = [
  {
    name: "skill_evolution",
    displayName: "技能进化引擎",
    description: "自动进化技能提示词和工具调用策略，基于执行反馈持续优化",
    category: "core",
  },
  {
    name: "auto_tool_creator",
    displayName: "自动工具创建器",
    description: "从频繁执行模式中学习，自动创建新的可复用工具",
    category: "core",
  },
  {
    name: "text_grad",
    displayName: "TextGrad 优化",
    description: "文本梯度优化引擎，类似梯度下降但用于文本提示词优化",
    category: "core",
  },
  {
    name: "constitution",
    displayName: "宪法规则引擎",
    description: "约束 Agent 行为的规则系统，确保安全合规",
    category: "safety",
  },
  {
    name: "intrinsic_motivation",
    displayName: "内在动机系统",
    description: "自主驱动学习系统，模拟人类好奇心驱动的探索行为",
    category: "learning",
  },
  {
    name: "coevolution",
    displayName: "协同进化环境",
    description: "多 Agent 协同进化环境，促进跨智能体的知识共享与竞争",
    category: "learning",
  },
  {
    name: "dream_consolidator",
    displayName: "梦境巩固器",
    description: "类似人类 REM 睡眠的记忆巩固机制，离线优化知识表示",
    category: "learning",
  },
  {
    name: "process_reward",
    displayName: "过程奖励模型",
    description: "评估中间步骤质量而非仅最终结果，提供细粒度反馈信号",
    category: "learning",
  },
  {
    name: "sandbox",
    displayName: "沙箱执行器",
    description: "安全执行不受信代码，隔离运行环境防止系统损害",
    category: "safety",
  },
];

function buildDefaultConfig(engineName: string): Record<string, unknown> {
  const configs: Record<string, Record<string, unknown>> = {
    skill_evolution: {
      evolutionRate: 0.01,
      minImprovement: 0.05,
      maxVersions: 10,
      populationSize: 20,
      generations: 50,
      mutationRate: 0.1,
      crossoverRate: 0.7,
      autoRollback: true,
      requireApproval: true,
    },
    auto_tool_creator: {
      minPatternFrequency: 3,
      similarityThreshold: 0.8,
      maxToolsPerSession: 5,
      requireConfirmation: true,
      toolComplexityLimit: "medium",
    },
    text_grad: {
      learningRate: 0.01,
      momentum: 0.9,
      maxIterations: 100,
      convergenceThreshold: 0.001,
      batchSize: 8,
      optimizer: "adam",
    },
    constitution: {
      strictMode: true,
      allowOverrides: false,
      rulePriority: "high",
      auditLog: true,
      maxRuleCount: 50,
    },
    intrinsic_motivation: {
      curiosityWeight: 0.3,
      noveltyThreshold: 0.5,
      explorationDecay: 0.99,
      maxExplorationBudget: 1000,
    },
    coevolution: {
      maxConcurrentAgents: 5,
      knowledgeShareInterval: 60000,
      competitionRatio: 0.3,
      elitismCount: 2,
    },
    dream_consolidator: {
      consolidationInterval: 3600000,
      batchSize: 32,
      memoryRetention: 0.9,
      replayRatio: 0.2,
    },
    process_reward: {
      discountFactor: 0.95,
      stepPenalty: 0.01,
      successBonus: 1.0,
      failurePenalty: -0.5,
    },
    sandbox: {
      timeoutMs: 30000,
      maxMemoryMB: 512,
      networkAccess: false,
      fileSystemAccess: "readonly",
      allowedLanguages: ["python", "javascript"],
    },
  };
  return configs[engineName] ?? {};
}

function buildDefaultStats(engineName: string): Record<string, unknown> {
  const stats: Record<string, Record<string, unknown>> = {
    skill_evolution: { totalEvolutions: 42, activeSkills: 12, avgImprovement: "8.3%", lastEvolution: Date.now() - 3600000 },
    auto_tool_creator: { toolsCreated: 7, patternsDetected: 23, avgConfidence: "87%", lastCreated: Date.now() - 7200000 },
    text_grad: { nodes: 156, gradients: 1280, iterations: 5000, lossReduction: "34%" },
    constitution: { rules: 18, violations: 3, enforcementRate: "99.7%", lastViolation: Date.now() - 86400000 },
    intrinsic_motivation: { explorationScore: 0.72, noveltyCount: 45, activeDrives: 3, energyLevel: "85%" },
    coevolution: { activeTasks: 2, agentsInPool: 8, knowledgeTransfers: 156, avgFitness: 0.68 },
    dream_consolidator: { knowledgeEntries: 2048, lastConsolidation: Date.now() - 1800000, retentionRate: "94%" },
    process_reward: { accuracy: "82%", stepsEvaluated: 15000, avgStepScore: 0.65, activeModels: 2 },
    sandbox: { totalExecutions: 324, successRate: "96%", avgExecutionMs: 450, lastExecution: Date.now() - 600000 },
  };
  return stats[engineName] ?? {};
}

function buildDefaultLogs(engineName: string): EngineLog[] {
  const now = Date.now();
  return [
    { timestamp: now - 300000, level: "info", message: `[${engineName}] Engine initialized successfully` },
    { timestamp: now - 240000, level: "info", message: `[${engineName}] Configuration loaded` },
    { timestamp: now - 180000, level: "info", message: `[${engineName}] Starting background tasks` },
    { timestamp: now - 120000, level: "info", message: `[${engineName}] Health check passed` },
    { timestamp: now - 60000, level: "info", message: `[${engineName}] Idle, waiting for triggers` },
  ];
}

function buildMockEngines(): Record<string, EngineStatus> {
  const engines: Record<string, EngineStatus> = {};
  for (const def of ENGINE_DEFINITIONS) {
    engines[def.name] = {
      ...def,
      running: def.category === "core" || def.category === "safety",
      config: buildDefaultConfig(def.name),
      stats: buildDefaultStats(def.name),
      logs: buildDefaultLogs(def.name),
      lastActive: Date.now() - Math.floor(Math.random() * 3600000),
    };
  }
  return engines;
}

function buildMockSkillVersions(): SkillVersion[] {
  return [
    {
      version: 4,
      timestamp: Date.now() - 86400000,
      summary: "优化了推理链步骤顺序，减少了冗余工具调用",
      metrics: { successRate: { before: 78, after: 85 }, tokenUsage: { before: 3200, after: 2800 }, avgTime: { before: 12.5, after: 10.2 } },
      promptDiff: { old: "You are an expert assistant. Think step by step.", new: "You are an expert assistant. Analyze the problem, identify key constraints, then execute efficiently." },
    },
    {
      version: 3,
      timestamp: Date.now() - 172800000,
      summary: "增加了错误处理分支，提高了鲁棒性",
      metrics: { successRate: { before: 72, after: 78 }, errorRate: { before: 15, after: 8 }, avgTime: { before: 14.0, after: 12.5 } },
    },
    {
      version: 2,
      timestamp: Date.now() - 259200000,
      summary: "引入了并行工具调用策略",
      metrics: { successRate: { before: 65, after: 72 }, tokenUsage: { before: 4000, after: 3500 }, avgTime: { before: 18.0, after: 14.0 } },
      promptDiff: { old: "Call tools one at a time.", new: "When possible, call independent tools in parallel." },
    },
    {
      version: 1,
      timestamp: Date.now() - 345600000,
      summary: "初始版本，基础功能实现",
      metrics: { successRate: { before: 0, after: 65 }, tokenUsage: { before: 0, after: 4000 }, avgTime: { before: 0, after: 18.0 } },
    },
  ];
}

function buildMockABTestResults(): ABTestResult[] {
  return [
    { metric: "成功率", valueA: 85, valueB: 78, change: 8.97, winner: "A" },
    { metric: "平均 Token 消耗", valueA: 2800, valueB: 3200, change: -12.5, winner: "A" },
    { metric: "平均执行时间(s)", valueA: 10.2, valueB: 12.5, change: -18.4, winner: "A" },
    { metric: "用户满意度", valueA: 4.2, valueB: 3.8, change: 10.5, winner: "A" },
    { metric: "错误率", valueA: 5, valueB: 8, change: -37.5, winner: "A" },
  ];
}

// ── Store ──

interface EvolutionState {
  engines: Record<string, EngineStatus>;
  evolutionHistory: EvolutionEvent[];
  loading: boolean;
  error: string | null;

  fetchAllEngineStatus: () => Promise<void>;
  startEngine: (name: string) => Promise<void>;
  stopEngine: (name: string) => Promise<void>;
  updateEngineConfig: (name: string, config: Record<string, unknown>) => Promise<void>;
  fetchEngineLogs: (name: string) => Promise<void>;
  getSkillEvolutionHistory: (skillId: string) => SkillVersion[];
  getABTestResults: (skillId: string) => ABTestResult[];
  triggerSkillEvolution: (skillId: string) => Promise<void>;
  addEvolutionEvent: (event: EvolutionEvent) => void;
}

export const useEvolutionStore = create<EvolutionState>((set, get) => ({
  engines: buildMockEngines(),
  evolutionHistory: [],
  loading: false,
  error: null,

  fetchAllEngineStatus: async () => {
    set({ loading: true, error: null });
    try {
      const statuses = await invoke<Record<string, EngineStatus>>("get_all_engine_status");
      set({ engines: statuses, loading: false });
    } catch (err) {
      console.warn("[evolutionStore] fetchAllEngineStatus failed, using mock data", err);
      // Keep existing mock data, just mark loading done
      set({ loading: false });
    }
  },

  startEngine: async (name: string) => {
    try {
      await invoke("start_engine", { engineName: name });
      set((state) => ({
        engines: {
          ...state.engines,
          [name]: { ...state.engines[name], running: true, lastActive: Date.now() },
        },
      }));
      get().addEvolutionEvent({ engine: name, timestamp: Date.now(), type: "started", detail: `Engine ${name} started` });
    } catch (err) {
      console.warn("[evolutionStore] startEngine failed, using mock", err);
      set((state) => ({
        engines: {
          ...state.engines,
          [name]: { ...state.engines[name], running: true, lastActive: Date.now() },
        },
      }));
      get().addEvolutionEvent({ engine: name, timestamp: Date.now(), type: "started", detail: `Engine ${name} started (mock)` });
    }
  },

  stopEngine: async (name: string) => {
    try {
      await invoke("stop_engine", { engineName: name });
      set((state) => ({
        engines: {
          ...state.engines,
          [name]: { ...state.engines[name], running: false },
        },
      }));
      get().addEvolutionEvent({ engine: name, timestamp: Date.now(), type: "stopped", detail: `Engine ${name} stopped` });
    } catch (err) {
      console.warn("[evolutionStore] stopEngine failed, using mock", err);
      set((state) => ({
        engines: {
          ...state.engines,
          [name]: { ...state.engines[name], running: false },
        },
      }));
      get().addEvolutionEvent({ engine: name, timestamp: Date.now(), type: "stopped", detail: `Engine ${name} stopped (mock)` });
    }
  },

  updateEngineConfig: async (name: string, config: Record<string, unknown>) => {
    try {
      await invoke("update_engine_config", { engineName: name, config });
      set((state) => ({
        engines: {
          ...state.engines,
          [name]: { ...state.engines[name], config: { ...state.engines[name].config, ...config } },
        },
      }));
      get().addEvolutionEvent({ engine: name, timestamp: Date.now(), type: "config_changed", detail: "Configuration updated" });
    } catch (err) {
      console.warn("[evolutionStore] updateEngineConfig failed, using mock", err);
      set((state) => ({
        engines: {
          ...state.engines,
          [name]: { ...state.engines[name], config: { ...state.engines[name].config, ...config } },
        },
      }));
    }
  },

  fetchEngineLogs: async (name: string) => {
    try {
      const logs = await invoke<EngineLog[]>("get_engine_logs", { engineName: name, limit: 50 });
      set((state) => ({
        engines: { ...state.engines, [name]: { ...state.engines[name], logs } },
      }));
    } catch (err) {
      console.warn("[evolutionStore] fetchEngineLogs failed, using mock", err);
      // Keep existing mock logs
    }
  },

  getSkillEvolutionHistory: (_skillId: string) => {
    return buildMockSkillVersions();
  },

  getABTestResults: (_skillId: string) => {
    return buildMockABTestResults();
  },

  triggerSkillEvolution: async (_skillId: string) => {
    try {
      await invoke("trigger_skill_evolution", { skillId: _skillId });
    } catch (err) {
      console.warn("[evolutionStore] triggerSkillEvolution failed, using mock", err);
    }
    get().addEvolutionEvent({
      engine: "skill_evolution",
      timestamp: Date.now(),
      type: "evolved",
      detail: `Skill ${_skillId} evolution triggered`,
    });
  },

  addEvolutionEvent: (event: EvolutionEvent) => {
    set((state) => ({
      evolutionHistory: [...state.evolutionHistory.slice(-199), event],
    }));
  },
}));
