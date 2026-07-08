import type { TFunction } from "i18next";

/**
 * 工作流节点 ID → 用户可见标签
 * - 优先尝试 i18n key：`stockAnalysis.workflow.${nodeId}`
 * - 特殊节点（翻译文件 key 与 ID 不完全对齐）走显式映射
 * - 全部 miss 时返回 nodeId 本身
 */
const SPECIAL_MAP: Record<string, string> = {
  "cls-risk-level": "stockAnalysis.workflow.riskLevel",
  "risk-level": "stockAnalysis.workflow.riskLevel",
  "agg-risk": "stockAnalysis.workflow.aggRisk",
  "risk-agg": "stockAnalysis.workflow.riskAggregation",
  "risk-con": "stockAnalysis.workflow.riskConservative",
  "risk-neu": "stockAnalysis.workflow.riskNeutral",
  "risk-aggregated": "stockAnalysis.workflow.riskAggregation",
  "risk-convergence": "stockAnalysis.workflow.riskConvergence",
  "v-validate": "stockAnalysis.workflow.vValidate",
  "notify-result": "stockAnalysis.workflow.notifyResult",
};

export function getWorkflowNodeLabel(nodeId: string, t: TFunction): string {
  const key = SPECIAL_MAP[nodeId] ?? `stockAnalysis.workflow.${nodeId}`;
  const result = t(key, { defaultValue: nodeId });
  // Some keys (e.g. "analyst", "phase") resolve to i18n objects, not strings
  return typeof result === "string" ? result : nodeId;
}
