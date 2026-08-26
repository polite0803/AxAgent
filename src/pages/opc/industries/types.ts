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

/** 工作流用户输入字段（前端渲染表单用） */
export interface WorkflowInputField {
  key: string;
  label: string;
  type: "string" | "number" | "textarea";
  required?: boolean;
  placeholder?: string;
  default?: string;
}

/** 向导步骤类型 */
export type WizardStepType = "form" | "confirm" | "execute" | "result" | "custom";

/** 向导上下文 — 跨步骤共享的状态与操作 */
export interface WizardContext {
  /** 当前所有表单值 */
  values: Record<string, unknown>;
  /** 设置单个字段值 */
  setValue: (key: string, value: unknown) => void;
  /** 批量设置字段值 */
  setValues: (values: Record<string, unknown>) => void;
  /** 当前步骤索引 */
  stepIndex: number;
  /** 工作流元数据 */
  workflow: IndustryWorkflow;
  /** 执行工作流 */
  execute: () => Promise<void>;
  /** 执行状态 */
  executing: boolean;
  /** 执行结果 */
  resultStatus: "success" | "failed" | null;
  /** 执行结果消息 */
  resultMessage: string;
  /** 关闭向导 */
  close: () => void;
}

/** 向导步骤定义 */
export interface WizardStep {
  /** 步骤唯一 ID */
  id: string;
  /** 步骤标题（i18n key 或直接文本） */
  title: string;
  /** 步骤标题的 i18n 描述（可选） */
  description?: string;
  /** 步骤类型 */
  type: WizardStepType;
  /** 该步骤关联的输入字段（仅 form 类型使用） */
  fields?: WorkflowInputField[];
  /** 自定义渲染函数（custom 类型使用） */
  render?: (ctx: WizardContext) => ReactNode;
  /** 校验函数，返回是否可进入下一步 */
  validate?: (ctx: WizardContext) => boolean;
  /** 跳过条件函数，返回 true 时跳过此步骤 */
  canSkip?: (ctx: WizardContext) => boolean;
  /** "下一步"按钮文案（i18n key） */
  nextLabel?: string;
  /** "上一步"按钮文案（i18n key） */
  prevLabel?: string;
  /** 是否显示"上一步"按钮 */
  showBack?: boolean;
}

/** 行业工作流 */
export interface IndustryWorkflow {
  id: string;
  name: string;
  description: string;
  version: string;
  /** 关联的工作流模板 ID（用于在编辑器中打开，不传则通过 id 查找） */
  template_id?: string;
  /** 用户输入字段（非空时前端渲染表单） */
  inputFields?: WorkflowInputField[];
  /**
   * 自定义向导步骤。未设置时自动生成：
   * - 有 inputFields → form → confirm → execute → result
   * - 无 inputFields → confirm → execute → result
   */
  wizardSteps?: WizardStep[];
}

/** 行业操作项 */
export interface ActionItem {
  key: string;
  icon: ReactNode;
  type: "conversation" | "workflow";
  label?: string;
  /** 工作流模板 ID（type=workflow 时指定要打开的模板，不传则查找关联 workflow 的 template_id） */
  template_id?: string;
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
