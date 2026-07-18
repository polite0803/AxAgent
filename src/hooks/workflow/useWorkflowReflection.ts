// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 工作流反思 / 进化 / 优化 hooks(阶段 5 wiring 前端入口)
 *
 * 包装 `@/lib/invoke` 中 6 个 workflow_reflection 命令,提供:
 * - 加载 / 错误状态管理
 * - 错误处理走项目标准 `showBackendError`(自动 i18n 翻译 + 按 category 弹 toast)
 * - 防重入(进化运行中不允许再次触发)
 *
 * 设计原则:hooks 是 UI 与 store 之间的薄层,不维护业务状态;
 * 业务状态(模板、反思历史等)由 `useWorkflowEditorStore` 管理。
 */

import { showBackendError } from "@/lib/errorI18n";
import {
  workflowEvolutionIsRunning,
  type WorkflowEvolutionStats,
  workflowEvolutionStats,
  workflowEvolveTemplate,
  type WorkflowModification,
  workflowOptimizeApply,
  workflowOptimizeSuggest,
  type WorkflowReflection,
  workflowShouldAutoEvolve,
  type WorkflowSuggestion,
} from "@/lib/invoke";
import { App } from "antd";
import { useCallback, useState } from "react";

export interface UseWorkflowReflectionResult {
  /** 加载状态(任意命令执行中均为 true)。 */
  loading: boolean;
  /** 错误信息(已 i18n 翻译);null 表示无错误。 */
  error: string | null;
  /** 优化建议(suggest 命令的最近结果)。 */
  suggestions: WorkflowSuggestion[];
  /** 进化统计。 */
  evolutionStats: WorkflowEvolutionStats | null;
  /** 进化是否运行中。 */
  evolutionRunning: boolean;
  /** 是否应自动进化(shouldAutoEvolve 的最近结果)。 */
  shouldAutoEvolve: boolean;
  /** 生成优化建议。 */
  suggest: (template: unknown, reflection: WorkflowReflection) => Promise<WorkflowSuggestion[] | null>;
  /** 应用优化建议,返回新模板。 */
  apply: (template: unknown, suggestions: WorkflowSuggestion[]) => Promise<unknown | null>;
  /** 触发模板进化。 */
  evolve: (templateId: string, reflections: WorkflowReflection[]) => Promise<WorkflowModification | null>;
  /** 刷新进化统计。 */
  refreshStats: () => Promise<void>;
  /** 刷新"是否应自动进化"标志。 */
  refreshShouldAutoEvolve: (templateId: string) => Promise<void>;
  /** 清除错误状态。 */
  clearError: () => void;
}

/**
 * 工作流反思 / 进化 / 优化统一 hook。
 *
 * 用法:
 * ```tsx
 * const reflection = useWorkflowReflection();
 * await reflection.suggest(template, latestReflection);
 * ```
 */
export function useWorkflowReflection(): UseWorkflowReflectionResult {
  const { message } = App.useApp();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [suggestions, setSuggestions] = useState<WorkflowSuggestion[]>([]);
  const [evolutionStats, setEvolutionStats] = useState<WorkflowEvolutionStats | null>(null);
  const [evolutionRunning, setEvolutionRunning] = useState(false);
  const [shouldAutoEvolve, setShouldAutoEvolve] = useState(false);

  /** 包装命令错误:走项目标准 `showBackendError`(自动 i18n + 按 category 弹 toast)。 */
  const handleAndShowError = useCallback(
    (e: unknown, context: string) => {
      const text = showBackendError(message, e, { context });
      setError(text);
    },
    [message],
  );

  const suggest = useCallback(
    async (template: unknown, reflection: WorkflowReflection) => {
      setLoading(true);
      setError(null);
      try {
        const result = await workflowOptimizeSuggest(template, reflection);
        setSuggestions(result);
        return result;
      } catch (e) {
        handleAndShowError(e, "workflow_optimize_suggest");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [handleAndShowError],
  );

  const apply = useCallback(
    async (template: unknown, suggs: WorkflowSuggestion[]) => {
      setLoading(true);
      setError(null);
      try {
        return await workflowOptimizeApply(template, suggs);
      } catch (e) {
        handleAndShowError(e, "workflow_optimize_apply");
        return null;
      } finally {
        setLoading(false);
      }
    },
    [handleAndShowError],
  );

  const evolve = useCallback(
    async (templateId: string, reflections: WorkflowReflection[]) => {
      // 防重入:进化运行中不允许再次触发
      if (evolutionRunning) {
        return null;
      }
      setLoading(true);
      setError(null);
      setEvolutionRunning(true);
      try {
        return await workflowEvolveTemplate(templateId, reflections);
      } catch (e) {
        handleAndShowError(e, "workflow_evolve_template");
        return null;
      } finally {
        setLoading(false);
        setEvolutionRunning(false);
      }
    },
    [evolutionRunning, handleAndShowError],
  );

  const refreshStats = useCallback(async () => {
    setLoading(true);
    try {
      const [stats, running] = await Promise.all([
        workflowEvolutionStats(),
        workflowEvolutionIsRunning(),
      ]);
      setEvolutionStats(stats);
      setEvolutionRunning(running);
    } catch (e) {
      handleAndShowError(e, "workflow_evolution_stats");
    } finally {
      setLoading(false);
    }
  }, [handleAndShowError]);

  const refreshShouldAutoEvolve = useCallback(
    async (templateId: string) => {
      try {
        const result = await workflowShouldAutoEvolve(templateId);
        setShouldAutoEvolve(result);
      } catch (e) {
        handleAndShowError(e, "workflow_should_auto_evolve");
      }
    },
    [handleAndShowError],
  );

  const clearError = useCallback(() => setError(null), []);

  return {
    loading,
    error,
    suggestions,
    evolutionStats,
    evolutionRunning,
    shouldAutoEvolve,
    suggest,
    apply,
    evolve,
    refreshStats,
    refreshShouldAutoEvolve,
    clearError,
  };
}
