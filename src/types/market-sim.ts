// SPDX-License-Identifier: AGPL-3.0-only

export interface SimRunRequest {
  stockCode: string;
  referencePrice: number;
  maxSimTimeNs?: number;
  defaultLatencyNs?: number;
  seed?: number;
  agentConfig?: AgentConfig;
  trace?: boolean;
}

export interface AgentConfig {
  marketMakers?: number;
  momentumAgents?: number;
  valueAgents?: number;
  noiseAgents?: number;
}

export interface SimRunResult {
  stockCode: string;
  referencePrice: number;
  totalEvents: number;
  wallClockMs: number;
  simTimeNs: number;
  finalMidPrice: number | null;
  agentCount: number;
  stats: SimRunStats;
}

export interface SimRunStats {
  totalTrades: number;
  totalOrders: number;
  maxQueueDepth: number;
}

// ── 蒙特卡洛鲁棒性测试类型 ──

export interface McRunRequest {
  stockCode: string;
  referencePrice: number;
  maxSimTimeNs: number;
  scenarios: McScenarioSpec[];
}

export interface McScenarioSpec {
  scenario: string;
  paths: number;
}

export interface RobustnessResult {
  stockCode: string;
  referencePrice: number;
  totalPaths: number;
  survivalRate: number;
  consistencyScore: number;
  bestScenario: string;
  worstScenario: string;
  scenarioResults: McScenarioResultItem[];
}

export interface McScenarioResultItem {
  scenario: string;
  label: string;
  paths: number;
  avgTotalTrades: number;
  avgFinalMidPrice: number | null;
  priceChangePct: number | null;
}

// ── 量化策略模拟类型 ──

export interface QuantSimResult {
  totalEvents: number;
  totalTrades: number;
  finalMidPrice: number | null;
  wallClockMs: number;
  strategyName?: string;
}
