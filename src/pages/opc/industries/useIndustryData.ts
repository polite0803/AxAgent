// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业数据管理 Hook — 提供行业数据加载和操作
 */

import { invoke } from "@/lib/invoke";
import type { IndustryLearningConfig } from "@/types";
import { useCallback, useEffect, useState } from "react";
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
  executeWorkflow: (workflowId: string) => Promise<WorkflowExecutionResult>;
  reflectOnWorkflow: () => Promise<void>;
  evolveWorkflow: () => Promise<void>;
  runSelfImprovement: () => Promise<void>;
}

/**
 * 行业数据管理 Hook
 * @param industryId 行业 ID
 * @returns 行业数据和操作方法
 */
export function useIndustryData(industryId: string | null): UseIndustryDataReturn {
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

  // 加载决策
  const loadDecision = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setDecisionLoading(true);
    try {
      const result = await invoke<OpcIndustryDecision>("opc_get_industry_decision", {
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

  // 加载学习指标
  const loadLearningMetrics = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setMetricsLoading(true);
    try {
      const result = await invoke<IndustryLearningMetrics>(
        "opc_get_industry_learning_metrics",
        { industryId },
      );
      setLearningMetrics(result);
    } catch (e) {
      console.error("[useIndustryData] load learning metrics failed:", e);
    } finally {
      setMetricsLoading(false);
    }
  }, [industryId]);

  // 加载学习配置
  const loadLearningConfig = useCallback(async () => {
    if (!industryId) {
      return;
    }
    setLearningLoading(true);
    try {
      const result = await invoke<IndustryLearningConfig>(
        "opc_get_industry_learning_config",
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

  // 执行工作流
  const executeWorkflow = useCallback(
    async (workflowId: string): Promise<WorkflowExecutionResult> => {
      setWorkflowExecuting(true);
      try {
        const result = await invoke<WorkflowExecutionResult>("opc_execute_workflow", {
          workflowId,
          industryId,
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

  // 反思
  const reflectOnWorkflow = useCallback(async () => {
    if (!industryId) {
      return;
    }
    try {
      await invoke("opc_reflect_workflow", { industryId });
      await loadLearningMetrics();
    } catch (e) {
      console.error("[useIndustryData] reflect failed:", e);
    }
  }, [industryId, loadLearningMetrics]);

  // 进化
  const evolveWorkflow = useCallback(async () => {
    if (!industryId) {
      return;
    }
    try {
      await invoke("opc_evolve_workflow", { industryId });
      await loadLearningMetrics();
    } catch (e) {
      console.error("[useIndustryData] evolve failed:", e);
    }
  }, [industryId, loadLearningMetrics]);

  // 自我改进
  const runSelfImprovement = useCallback(async () => {
    if (!industryId) {
      return;
    }
    try {
      await invoke("opc_self_improve", { industryId });
      await loadLearningMetrics();
    } catch (e) {
      console.error("[useIndustryData] self improve failed:", e);
    }
  }, [industryId, loadLearningMetrics]);

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
