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
  created_at: string;
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
  expected_output?: TaskOutput;
  evaluation_criteria: EvaluationCriteria[];
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
  max_concurrency: number;
  timeout_ms: number;
  max_difficulty?: Difficulty;
  include_traces: boolean;
}

export interface ScoreResult {
  criteria_name: string;
  metric: EvaluationMetric;
  raw_score: number;
  weighted_score: number;
  passed: boolean;
}

export interface TaskResult {
  task_id: string;
  task_name: string;
  difficulty: Difficulty;
  success: boolean;
  duration_ms: number;
  scores: ScoreResult[];
  overall_score: number;
  response?: string;
  error?: string;
  trace_id?: string;
}

export interface AggregateMetrics {
  total_tasks: number;
  passed_tasks: number;
  failed_tasks: number;
  pass_rate: number;
  avg_duration_ms: number;
  avg_score: number;
  score_breakdown: Record<string, number>;
  difficulty_distribution: Record<string, number>;
}

export interface BenchmarkResult {
  benchmark_id: string;
  benchmark_name: string;
  run_at: string;
  config: RunnerConfig;
  task_results: TaskResult[];
  aggregate: AggregateMetrics;
  duration_ms: number;
}

export interface ReportSummary {
  total_tasks: number;
  passed_tasks: number;
  failed_tasks: number;
  pass_rate: number;
  overall_score: number;
  total_duration_ms: number;
  avg_task_duration_ms: number;
}

export interface CriteriaScore {
  name: string;
  score: number;
  passed: boolean;
}

export interface TaskBreakdown {
  task_id: string;
  task_name: string;
  difficulty: string;
  success: boolean;
  score: number;
  duration_ms: number;
  criteria_scores: CriteriaScore[];
}

export interface BenchmarkReport {
  benchmark_id: string;
  benchmark_name: string;
  generated_at: string;
  summary: ReportSummary;
  task_breakdown: TaskBreakdown[];
  category_scores: Record<string, number>;
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
  if (ms < 1000) { return `${ms}ms`; }
  if (ms < 60000) { return `${(ms / 1000).toFixed(1)}s`; }
  return `${(ms / 60000).toFixed(1)}m`;
}

export function getDifficultyLabel(difficulty: Difficulty): string {
  switch (difficulty) {
    case "easy":
      return "简单";
    case "medium":
      return "中等";
    case "hard":
      return "困难";
    case "expert":
      return "专家";
  }
}

export function getCategoryLabel(category: BenchmarkCategory): string {
  switch (category) {
    case "reasoning":
      return "推理";
    case "code_generation":
      return "代码生成";
    case "tool_usage":
      return "工具使用";
    case "research":
      return "研究";
    case "conversation":
      return "对话";
    case "error_recovery":
      return "错误恢复";
  }
}
