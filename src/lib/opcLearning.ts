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
 * P1-7：返回类型对齐后端 ReflectionResult（camelCase）
 */
export async function reflectOnWorkflow(
  params: ReflectOnWorkflowParams,
): Promise<{
  success: boolean;
  industryId: string;
  workflowId: string;
  qualityScore: number;
  suggestions: string[];
  summary: string;
}> {
  return invoke("opc_reflect_on_workflow", {
    industryId: params.industryId,
    workflowId: params.workflowId,
    workflowResult: params.workflowResult,
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
  industryId: string;
  workflowId: string;
  status: string;
  suggestedOptimizations: string[];
  message: string;
}> {
  return invoke("opc_evolve_workflow", {
    industryId: params.industryId,
    workflowId: params.workflowId,
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
  industryId: string;
  target: string;
  status: string;
  improvementsApplied: string[];
  message: string;
}> {
  return invoke("opc_run_self_improvement", {
    industryId: params.industryId,
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
    industryId: string;
    workflowId: string;
    qualityScore: number;
    workflowResult: Record<string, unknown>;
  },
): Promise<{ success: boolean; experienceId?: string; totalReward?: number; message?: string }> {
  return invoke("opc_record_rl_experience", {
    industryId: params.industryId,
    workflowId: params.workflowId,
    qualityScore: params.qualityScore,
    workflowResult: params.workflowResult,
  });
}

/**
 * 触发 RL 策略优化
 */
export async function triggerRLOptimization(
  params: TriggerRLOptimizationParams,
): Promise<RLPolicyUpdate> {
  return invoke("opc_trigger_rl_optimization", {
    industryId: params.industryId,
  });
}

/**
 * 触发自动学习闭环（反思→进化→自我改进→RL）
 */
export async function triggerAutoLearning(params: {
  industryId: string;
  workflowId: string;
  workflowResult: Record<string, unknown>;
}): Promise<AutoLearningResult> {
  return invoke("opc_trigger_industry_learning", {
    industryId: params.industryId,
    workflowId: params.workflowId,
    workflowResult: params.workflowResult,
  });
}
