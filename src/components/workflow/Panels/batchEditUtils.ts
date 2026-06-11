// SPDX-License-Identifier: AGPL-3.0-only

import type { RetryConfig, WorkflowNode } from "@/components/workflow/types";

export interface BatchEditOptions {
  timeout?: number | null;
  retryEnabled?: boolean | null;
  enabled?: boolean | null;
}

/**
 * 构造单节点的批量更新 diff，保留原有 retry 字段。
 * 当 retryEnabled 为 null/undefined 时，不写入 retry 字段（保持不变）。
 */
export function buildBatchUpdate(
  node: WorkflowNode,
  options: BatchEditOptions,
): Partial<WorkflowNode> {
  const updates: Partial<WorkflowNode> & Record<string, unknown> = {};
  if (options.timeout != null) { updates.timeout = options.timeout; }
  if (options.retryEnabled != null) {
    const baseRetry: RetryConfig = {
      enabled: false,
      max_retries: 0,
      backoff_type: "Fixed",
      base_delay_ms: 0,
      max_delay_ms: 0,
    };
    const existing = (node as unknown as { retry?: RetryConfig }).retry;
    const merged: RetryConfig = existing
      ? { ...baseRetry, ...existing, enabled: options.retryEnabled }
      : { ...baseRetry, enabled: options.retryEnabled };
    updates.retry = merged;
  }
  if (options.enabled != null) { updates.enabled = options.enabled; }
  return updates;
}
