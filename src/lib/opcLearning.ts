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
    industry_id: industryId,
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
 */
export async function reflectOnWorkflow(
  params: ReflectOnWorkflowParams,
): Promise<{
  success: boolean;
  industryId: string;
  workflowId: string;
  reflectionStatus: string;
  message: string;
}> {
  return invoke("opc_reflect_on_workflow", {
    industry_id: params.industry_id,
    workflow_id: params.workflow_id,
    workflow_result: params.workflow_result,
  });
}

/**
 * 触发工作流进化
 */
export async function evolveWorkflow(
  params: EvolveWorkflowParams,
): Promise<{
  success: boolean;
  industryId: string;
  workflowId: string;
  evolutionStatus: string;
  reason: string;
  message: string;
}> {
  return invoke("opc_evolve_workflow", {
    industry_id: params.industry_id,
    workflow_id: params.workflow_id,
    reason: params.reason,
  });
}

/**
 * 执行自我改进
 */
export async function runSelfImprovement(
  params: RunSelfImprovementParams,
): Promise<{
  success: boolean;
  industryId: string;
  target: string;
  improvementStatus: string;
  message: string;
}> {
  return invoke("opc_run_self_improvement", {
    industry_id: params.industry_id,
    target: params.target,
  });
}

/**
 * 获取 RL 经验池统计
 */
export async function getRLStats(industryId?: string): Promise<ExperiencePoolStats> {
  return invoke("opc_get_rl_stats", {
    industry_id: industryId,
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
    industry_id: params.industry_id,
    workflow_id: params.workflow_id,
    quality_score: params.quality_score,
    workflow_result: params.workflow_result,
  });
}

/**
 * 触发 RL 策略优化
 */
export async function triggerRLOptimization(
  params: TriggerRLOptimizationParams,
): Promise<RLPolicyUpdate> {
  return invoke("opc_trigger_rl_optimization", {
    industry_id: params.industry_id,
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
    industry_id: params.industry_id,
    workflow_id: params.workflow_id,
    workflow_result: params.workflow_result,
  });
}
