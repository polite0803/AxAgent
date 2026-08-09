// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业数据管理 Hook — 提供行业数据加载和操作
 *
 * 注意：所有 invoke 调用使用 camelCase 参数名（Tauri v2 IPC 默认 rename_all=camelCase）
 */

import { invoke } from "@/lib/invoke";
import type { IndustryLearningConfig } from "@/types";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  AutomationRuleInfo,
  IndustryDashboard,
  IndustryLearningMetrics,
  IndustryManifest,
  OpcIndustryDecision,
  WorkflowExecutionResult,
  WorkflowStepInfo,
} from "./types";

/** 行业数据 Hook 返回值 */
export interface UseIndustryDataReturn {
  // 状态
  loading: boolean;
  manifest: IndustryManifest | null;
  learningConfig: IndustryLearningConfig | null;
  learningLoading: boolean;
  dashboard: IndustryDashboard | null;
  dashboardLoading: boolean;
  workflowSteps: WorkflowStepInfo[];
  stepsLoading: boolean;
  automationRules: AutomationRuleInfo[];
  rulesLoading: boolean;
  rulesRunning: boolean;
  kpiTimeRange: "7" | "30" | "90";
  decision: OpcIndustryDecision | null;
  decisionLoading: boolean;
  decisionDays: number;
  workflowResult: WorkflowExecutionResult | null;
  workflowExecuting: boolean;
  learningMetrics: IndustryLearningMetrics | null;
  metricsLoading: boolean;

  // 操作
  setKpiTimeRange: (range: "7" | "30" | "90") => void;
  setDecisionDays: (days: number) => void;
  loadDashboard: () => Promise<void>;
  loadWorkflowSteps: () => Promise<void>;
  loadAutomationRules: () => Promise<void>;
  loadDecision: () => Promise<void>;
  loadLearningMetrics: () => Promise<void>;
  loadLearningConfig: () => Promise<void>;
  runAutomationRules: () => Promise<string[]>;
  executeWorkflow: (workflowId: string, userInput?: Record<string, unknown>) => Promise<WorkflowExecutionResult>;
  reflectOnWorkflow: (workflowId?: string) => Promise<void>;
  evolveWorkflow: (workflowId?: string, reason?: string) => Promise<void>;
  runSelfImprovement: (target?: string) => Promise<void>;
}

/**
 * 行业数据管理 Hook
 * @param industryId 行业 ID
 * @returns 行业数据和操作方法
 */
export function useIndustryData(industryId: string | null): UseIndustryDataReturn {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [manifest, setManifest] = useState<IndustryManifest | null>(null);
  const [learningConfig, setLearningConfig] = useState<IndustryLearningConfig | null>(null);
  const [learningLoading, setLearningLoading] = useState(false);
  const [dashboard, setDashboard] = useState<IndustryDashboard | null>(null);
  const [dashboardLoading, setDashboardLoading] = useState(false);
  const [workflowSteps, setWorkflowSteps] = useState<WorkflowStepInfo[]>([]);
  const [stepsLoading, setStepsLoading] = useState(false);
  const [automationRules, setAutomationRules] = useState<AutomationRuleInfo[]>([]);
  const [rulesLoading, setRulesLoading] = useState(false);
  const [rulesRunning, setRulesRunning] = useState(false);
  const [kpiTimeRange, setKpiTimeRange] = useState<"7" | "30" | "90">("30");
  const [decision, setDecision] = useState<OpcIndustryDecision | null>(null);
  const [decisionLoading, setDecisionLoading] = useState(false);
  const [decisionDays, setDecisionDays] = useState(30);
  const [workflowResult, setWorkflowResult] = useState<WorkflowExecutionResult | null>(null);
  const [workflowExecuting, setWorkflowExecuting] = useState(false);
  const [learningMetrics, setLearningMetrics] = useState<IndustryLearningMetrics | null>(null);
  const [metricsLoading, setMetricsLoading] = useState(false);

  // 加载行业清单
  useEffect(() => {
    if (!industryId) {
      setLoading(false);
      return;
    }

    const loadIndustry = async () => {
      setLoading(true);
      try {
        const result = await invoke<{ manifest: IndustryManifest }>(
          "opc_get_industry_pack",
          { industryId },
        );
        setManifest(result.manifest);
      } catch (e) {
        console.error("[useIndustryData] load failed:", e);
      } finally {
        setLoading(false);
      }
    };

    loadIndustry();
  }, [industryId]);

  // 加载仪表盘
  const loadDashboard = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setDashboardLoading(true);
    try {
      const days = Number(kpiTimeRange);
      const result = await invoke<IndustryDashboard>(
        "opc_get_industry_dashboard",
        { industryId, days },
      );
      setDashboard(result);
    } catch (e) {
      console.error("[useIndustryData] load dashboard failed:", e);
    } finally {
      setDashboardLoading(false);
    }
  }, [industryId, kpiTimeRange]);

  // 加载工作流步骤
  const loadWorkflowSteps = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setStepsLoading(true);
    try {
      const result = await invoke<{ steps: WorkflowStepInfo[] }>(
        "opc_get_industry_workflow_steps",
        { industryId },
      );
      setWorkflowSteps(result.steps || []);
    } catch (e) {
      console.error("[useIndustryData] load workflow steps failed:", e);
      setWorkflowSteps([]);
    } finally {
      setStepsLoading(false);
    }
  }, [industryId]);

  // 加载自动化规则
  const loadAutomationRules = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setRulesLoading(true);
    try {
      const result = await invoke<{ rules: AutomationRuleInfo[] }>(
        "opc_get_industry_automation_rules",
        { industryId },
      );
      setAutomationRules(result.rules || []);
    } catch (e) {
      console.error("[useIndustryData] load automation rules failed:", e);
      setAutomationRules([]);
    } finally {
      setRulesLoading(false);
    }
  }, [industryId]);

  // 加载决策（使用 opc_execute_analysis 命令）
  const loadDecision = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setDecisionLoading(true);
    try {
      const result = await invoke<OpcIndustryDecision>("opc_execute_analysis", {
        industryId,
        days: decisionDays,
      });
      setDecision(result);
    } catch (e) {
      console.error("[useIndustryData] load decision failed:", e);
    } finally {
      setDecisionLoading(false);
    }
  }, [industryId, decisionDays]);

  // 加载学习指标（使用 opc_get_learning_metrics 命令）
  const loadLearningMetrics = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setMetricsLoading(true);
    try {
      const result = await invoke<IndustryLearningMetrics>(
        "opc_get_learning_metrics",
        { industryId },
      );
      setLearningMetrics(result);
    } catch (e) {
      console.error("[useIndustryData] load learning metrics failed:", e);
    } finally {
      setMetricsLoading(false);
    }
  }, [industryId]);

  // 加载学习配置（使用 opc_get_learning_config 命令）
  const loadLearningConfig = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setLearningLoading(true);
    try {
      const result = await invoke<IndustryLearningConfig>(
        "opc_get_learning_config",
        { industryId },
      );
      setLearningConfig(result);
    } catch (e) {
      console.error("[useIndustryData] load learning config failed:", e);
    } finally {
      setLearningLoading(false);
    }
  }, [industryId]);

  // 执行自动化规则
  const runAutomationRules = useCallback(async (): Promise<string[]> => {
    if (!industryId) {
      return [];
    }
    setRulesRunning(true);
    try {
      const triggered = await invoke<string[]>("opc_run_automation_rules", {
        industryId,
        entityType: "customer",
        entityId: "manual_trigger",
      });
      return triggered;
    } catch (e) {
      console.error("[useIndustryData] run automation rules failed:", e);
      return [];
    } finally {
      setRulesRunning(false);
    }
  }, [industryId]);

  // 执行工作流（使用 opc_execute_workflow 命令，传递 workflow_id + industry_id + days + userInput）
  const executeWorkflow = useCallback(
    async (workflowId: string, userInput?: Record<string, unknown>): Promise<WorkflowExecutionResult> => {
      setWorkflowExecuting(true);
      try {
        const result = await invoke<WorkflowExecutionResult>("opc_execute_workflow", {
          industryId,
          workflowId,
          days: 30,
          userInput: userInput ?? null,
        });
        setWorkflowResult(result);
        return result;
      } catch (e) {
        const errorResult: WorkflowExecutionResult = {
          workflow_id: workflowId,
          status: "failed",
          steps_completed: 0,
          steps_total: 0,
          error: String(e),
          duration_ms: 0,
        };
        setWorkflowResult(errorResult);
        return errorResult;
      } finally {
        setWorkflowExecuting(false);
      }
    },
    [industryId],
  );

  // 反思（使用 opc_reflect_on_workflow 命令，需要 workflow_id + workflow_result）
  const reflectOnWorkflow = useCallback(
    async (workflowId?: string) => {
      if (!industryId) {
        return;
      }
      try {
        const wfId = workflowId || `default_${industryId}`;
        const wfResult = workflowResult || { status: "completed", steps_completed: 0, steps_total: 0 };
        await invoke("opc_reflect_on_workflow", {
          industryId,
          workflowId: wfId,
          workflowResult: wfResult,
        });
        await loadLearningMetrics();
      } catch (e) {
        console.error("[useIndustryData] reflect failed:", e);
      }
    },
    [industryId, workflowResult, loadLearningMetrics],
  );

  // 进化（使用 opc_evolve_workflow 命令，需要 workflow_id + reason）
  const evolveWorkflow = useCallback(
    async (workflowId?: string, reason?: string) => {
      if (!industryId) {
        return;
      }
      try {
        const wfId = workflowId || `default_${industryId}`;
        const reasonText = reason || t("opc.industry.learning.evolution.defaultReason");
        await invoke("opc_evolve_workflow", {
          industryId,
          workflowId: wfId,
          reason: reasonText,
        });
        await loadLearningMetrics();
      } catch (e) {
        console.error("[useIndustryData] evolve failed:", e);
      }
    },
    [industryId, loadLearningMetrics, t],
  );

  // 自我改进（使用 opc_run_self_improvement 命令，需要 target）
  const runSelfImprovement = useCallback(
    async (target?: string) => {
      if (!industryId) {
        return;
      }
      try {
        const targetText = target || "all";
        await invoke("opc_run_self_improvement", {
          industryId,
          target: targetText,
        });
        await loadLearningMetrics();
      } catch (e) {
        console.error("[useIndustryData] self improve failed:", e);
      }
    },
    [industryId, loadLearningMetrics],
  );

  // 初始化加载
  useEffect(() => {
    if (!industryId) {
      return;
    }
    loadDashboard();
    loadWorkflowSteps();
    loadAutomationRules();
    loadLearningMetrics();
    loadLearningConfig();
  }, [industryId, loadDashboard, loadWorkflowSteps, loadAutomationRules, loadLearningMetrics, loadLearningConfig]);

  // KPI 时间范围变化时刷新
  useEffect(() => {
    if (!industryId) {
      return;
    }
    loadDashboard();
  }, [industryId, kpiTimeRange, loadDashboard]);

  return {
    loading,
    manifest,
    learningConfig,
    learningLoading,
    dashboard,
    dashboardLoading,
    workflowSteps,
    stepsLoading,
    automationRules,
    rulesLoading,
    rulesRunning,
    kpiTimeRange,
    setKpiTimeRange,
    decision,
    decisionLoading,
    decisionDays,
    setDecisionDays,
    workflowResult,
    workflowExecuting,
    learningMetrics,
    metricsLoading,
    loadDashboard,
    loadWorkflowSteps,
    loadAutomationRules,
    loadDecision,
    loadLearningMetrics,
    loadLearningConfig,
    runAutomationRules,
    executeWorkflow,
    reflectOnWorkflow,
    evolveWorkflow,
    runSelfImprovement,
  };
}
