// SPDX-License-Identifier: AGPL-3.0-only

export type BenchmarkCategory =
  | "reasoning"
  | "code_generation"
  | "tool_usage"
  | "research"
  | "conversation"
  | "error_recovery";

export type Difficulty = "easy" | "medium" | "hard" | "expert";

export type EvaluationMetric =
  | "exact_match"
  | "contains"
  | "levenshtein_similarity"
  | "semantic_similarity"
  | "tool_correctness"
  | "output_format"
  | "performance";

export interface BenchmarkMetadata {
  version: string;
  author: string;
  createdAt: string;
  tags: string[];
}

export interface TaskInput {
  query: string;
  context?: unknown;
  constraints: string[];
}

export interface TaskOutput {
  content: string;
  format: string;
}

export interface EvaluationCriteria {
  name: string;
  metric: EvaluationMetric;
  weight: number;
  threshold?: number;
}

export interface BenchmarkTask {
  id: string;
  name: string;
  description: string;
  input: TaskInput;
  expectedOutput?: TaskOutput;
  evaluationCriteria: EvaluationCriteria[];
  difficulty: Difficulty;
  tags: string[];
}

export interface Benchmark {
  id: string;
  name: string;
  description: string;
  category: BenchmarkCategory;
  tasks: BenchmarkTask[];
  metadata: BenchmarkMetadata;
}

export interface RunnerConfig {
  maxConcurrency: number;
  timeoutMs: number;
  maxDifficulty?: Difficulty;
  includeTraces: boolean;
}

export interface ScoreResult {
  criteriaName: string;
  metric: EvaluationMetric;
  rawScore: number;
  weightedScore: number;
  passed: boolean;
}

export interface TaskResult {
  taskId: string;
  taskName: string;
  difficulty: Difficulty;
  success: boolean;
  durationMs: number;
  scores: ScoreResult[];
  overallScore: number;
  response?: string;
  error?: string;
  traceId?: string;
}

export interface AggregateMetrics {
  totalTasks: number;
  passedTasks: number;
  failedTasks: number;
  passRate: number;
  avgDurationMs: number;
  avgScore: number;
  scoreBreakdown: Record<string, number>;
  difficultyDistribution: Record<string, number>;
}

export interface BenchmarkResult {
  benchmarkId: string;
  benchmarkName: string;
  runAt: string;
  config: RunnerConfig;
  taskResults: TaskResult[];
  aggregate: AggregateMetrics;
  durationMs: number;
}

export interface ReportSummary {
  totalTasks: number;
  passedTasks: number;
  failedTasks: number;
  passRate: number;
  overallScore: number;
  totalDurationMs: number;
  avgTaskDurationMs: number;
}

export interface CriteriaScore {
  name: string;
  score: number;
  passed: boolean;
}

export interface TaskBreakdown {
  taskId: string;
  taskName: string;
  difficulty: string;
  success: boolean;
  score: number;
  durationMs: number;
  criteriaScores: CriteriaScore[];
}

export interface BenchmarkReport {
  benchmarkId: string;
  benchmarkName: string;
  generatedAt: string;
  summary: ReportSummary;
  taskBreakdown: TaskBreakdown[];
  categoryScores: Record<string, number>;
  recommendations: string[];
}

export interface Dataset {
  id: string;
  name: string;
  description: string;
  benchmarks: string[];
  version: string;
  metadata: {
    source: string;
    license: string;
    tags: string[];
  };
}

export function formatScore(score: number): string {
  return `${(score * 100).toFixed(2)}%`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${ms}ms`;
  }
  if (ms < 60000) {
    return `${(ms / 1000).toFixed(1)}s`;
  }
  return `${(ms / 60000).toFixed(1)}m`;
}

export function getDifficultyKey(difficulty: Difficulty): string {
  switch (difficulty) {
    case "easy":
      return "difficulty.easy";
    case "medium":
      return "difficulty.medium";
    case "hard":
      return "difficulty.hard";
    case "expert":
      return "difficulty.expert";
  }
}

export function getCategoryKey(category: BenchmarkCategory): string {
  switch (category) {
    case "reasoning":
      return "evalCategory.reasoning";
    case "code_generation":
      return "evalCategory.codeGeneration";
    case "tool_usage":
      return "evalCategory.toolUsage";
    case "research":
      return "evalCategory.research";
    case "conversation":
      return "evalCategory.conversation";
    case "error_recovery":
      return "evalCategory.errorRecovery";
  }
}
