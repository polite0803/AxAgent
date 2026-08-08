// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业页面共享类型定义
 */

import type { ReactNode } from "react";

/** 行业清单 */
export interface IndustryManifest {
  id: string;
  name: string;
  icon: string;
  description: string;
  version: number;
  enabled: boolean;
}

/** 行业工作流 */
export interface IndustryWorkflow {
  id: string;
  name: string;
  description: string;
  version: string;
}

/** 行业操作项 */
export interface ActionItem {
  key: string;
  icon: ReactNode;
  type: "conversation" | "workflow";
  label?: string;
}

/** Tab 业务阶段配置 */
export interface IndustryTab {
  key: string;
  label: string;
  icon?: ReactNode;
  description?: string;
  actions: ActionItem[];
  workflows: IndustryWorkflow[];
}

/** 行业配置（支持 Tab 业务流程） */
export interface IndustryConfig {
  // 兼容旧格式（无 Tab）
  actions?: ActionItem[];
  workflows?: IndustryWorkflow[];
  // 新格式：Tab 业务流程
  tabs?: IndustryTab[];
}

/** KPI 值 */
export interface KpiValue {
  id: string;
  name: string;
  value: number;
  unit: string;
  period: string;
  target?: number;
  trend?: "improving" | "stable" | "declining";
  last_updated: number;
}

/** 行业仪表盘 */
export interface IndustryDashboard {
  id: string;
  industry_id: string;
  period_days: number;
  kpis: KpiValue[];
  generated_at: number;
}

/** 工作流步骤信息 */
export interface WorkflowStepInfo {
  id: string;
  workflow_id: string;
  step_order: number;
  step_type: string;
  name: string;
  description: string;
  avg_duration_ms: number;
  success_rate: number;
  execution_count: number;
}

/** 自动化规则信息 */
export interface AutomationRuleInfo {
  id: string;
  name: string;
  description: string;
  trigger_event: string;
  condition: string;
  action: string;
  enabled: boolean;
  last_triggered: number | null;
  trigger_count: number;
}

/** 行业分析决策 */
export interface OpcIndustryDecision {
  id: string;
  industry_id: string;
  days: number;
  generated_at: number;
  summary: string;
  risk_level: "high" | "medium" | "low";
  confidence: number;
  decision_type: string;
  key_metrics: Array<{
    name: string;
    value: number;
    unit: string;
    trend: string;
  }>;
  recommendations: Array<{
    type: string;
    priority: string;
    description: string;
  }>;
}

/** 工作流执行结果 */
export interface WorkflowExecutionResult {
  workflow_id: string;
  status: "success" | "failed" | "running" | "completed";
  steps_completed: number;
  steps_total: number;
  output?: Record<string, unknown>;
  error?: string;
  duration_ms: number;
  node_results?: Array<{
    id: string;
    name: string;
    status: string;
    duration_ms: number;
    output?: Record<string, unknown>;
  }>;
}

/** 行业学习指标 */
export interface IndustryLearningMetrics {
  total_samples: number;
  decision_accuracy: number;
  risk_prediction_accuracy: number;
  avg_feedback_score: number;
  improvement_trend: "improving" | "stable" | "declining";
  reflection_count: number;
  evolution_count: number;
  improvement_count: number;
  avg_improvement_score: number;
  last_reflection_at: number | null;
  last_evolution_at: number | null;
  last_improvement_at: number | null;
}
