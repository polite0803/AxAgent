// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type {
  AutoLearningResult,
  EvolveWorkflowParams,
  ExperiencePoolStats,
  IndustryLearningConfig,
  IndustryLearningConfigSummary,
  ReflectOnWorkflowParams,
  RLPolicyUpdate,
  RunSelfImprovementParams,
  TriggerRLOptimizationParams,
} from "@/types";

/**
 * 获取指定行业的学习配置
 */
export async function getLearningConfig(
  industryId: string,
): Promise<IndustryLearningConfig> {
  return invoke<IndustryLearningConfig>("opc_get_learning_config", {
    industryId,
  });
}

/**
 * 获取所有行业的学习配置列表
 */
export async function listLearningConfigs(): Promise<IndustryLearningConfigSummary[]> {
  return invoke<IndustryLearningConfigSummary[]>("opc_list_learning_configs");
}

/**
 * 触发工作流反思
 * P1-7：返回类型对齐后端 ReflectionResult（snake_case）
 */
export async function reflectOnWorkflow(
  params: ReflectOnWorkflowParams,
): Promise<{
  success: boolean;
  industry_id: string;
  workflow_id: string;
  quality_score: number;
  suggestions: string[];
  summary: string;
}> {
  return invoke("opc_reflect_on_workflow", {
    industryId: params.industry_id,
    workflowId: params.workflow_id,
    workflowResult: params.workflow_result,
  });
}

/**
 * 触发工作流进化
 * P1-7：返回类型对齐后端 EvolutionResult
 */
export async function evolveWorkflow(
  params: EvolveWorkflowParams,
): Promise<{
  success: boolean;
  industry_id: string;
  workflow_id: string;
  status: string;
  suggested_optimizations: string[];
  message: string;
}> {
  return invoke("opc_evolve_workflow", {
    industryId: params.industry_id,
    workflowId: params.workflow_id,
    reason: params.reason,
  });
}

/**
 * 执行自我改进
 * P1-7：返回类型对齐后端 SelfImprovementResult
 */
export async function runSelfImprovement(
  params: RunSelfImprovementParams,
): Promise<{
  success: boolean;
  industry_id: string;
  target: string;
  status: string;
  improvements_applied: string[];
  message: string;
}> {
  return invoke("opc_run_self_improvement", {
    industryId: params.industry_id,
    target: params.target,
  });
}

/**
 * 获取 RL 经验池统计
 */
export async function getRLStats(industryId?: string): Promise<ExperiencePoolStats> {
  return invoke("opc_get_rl_stats", {
    industryId,
  });
}

/**
 * 记录 RL 经验
 */
export async function recordRLExperience(
  params: {
    industry_id: string;
    workflow_id: string;
    quality_score: number;
    workflow_result: Record<string, unknown>;
  },
): Promise<{ success: boolean; experienceId?: string; totalReward?: number; message?: string }> {
  return invoke("opc_record_rl_experience", {
    industryId: params.industry_id,
    workflowId: params.workflow_id,
    qualityScore: params.quality_score,
    workflowResult: params.workflow_result,
  });
}

/**
 * 触发 RL 策略优化
 */
export async function triggerRLOptimization(
  params: TriggerRLOptimizationParams,
): Promise<RLPolicyUpdate> {
  return invoke("opc_trigger_rl_optimization", {
    industryId: params.industry_id,
  });
}

/**
 * 触发自动学习闭环（反思→进化→自我改进→RL）
 */
export async function triggerAutoLearning(params: {
  industry_id: string;
  workflow_id: string;
  workflow_result: Record<string, unknown>;
}): Promise<AutoLearningResult> {
  return invoke("opc_trigger_industry_learning", {
    industryId: params.industry_id,
    workflowId: params.workflow_id,
    workflowResult: params.workflow_result,
  });
}
