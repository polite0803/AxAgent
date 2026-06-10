import { buildBatchUpdate } from "@/components/workflow/Panels/batchEditUtils";
import type { WorkflowNode } from "@/components/workflow/types";
import { describe, expect, it } from "vitest";

function makeNode(retry?: Record<string, unknown>): WorkflowNode {
  return {
    id: "n-1",
    type: "agent",
    title: "n",
    description: "",
    position: { x: 0, y: 0 },
    config: {},
    retry: retry as never,
    enabled: true,
  } as unknown as WorkflowNode;
}

describe("buildBatchUpdate - #6.7", () => {
  it("returns empty updates when all options are null/undefined", () => {
    const node = makeNode({ enabled: false, max_retries: 3 });
    expect(buildBatchUpdate(node, {})).toEqual({});
    expect(buildBatchUpdate(node, { timeout: null, retryEnabled: null, enabled: null })).toEqual({});
  });

  it("preserves existing retry fields when only toggling enabled", () => {
    const node = makeNode({
      enabled: false,
      max_retries: 5,
      backoff_type: "Exponential",
      base_delay_ms: 1000,
      max_delay_ms: 30000,
    });
    const update = buildBatchUpdate(node, { retryEnabled: true });
    expect(update.retry).toEqual({
      enabled: true,
      max_retries: 5,
      backoff_type: "Exponential",
      base_delay_ms: 1000,
      max_delay_ms: 30000,
    });
  });

  it("fills in safe defaults when retry was previously undefined", () => {
    const node = makeNode(undefined);
    const update = buildBatchUpdate(node, { retryEnabled: true });
    expect((update.retry as any).enabled).toBe(true);
    expect((update.retry as any).max_retries).toBe(0);
    expect((update.retry as any).backoff_type).toBe("Fixed");
    expect((update.retry as any).base_delay_ms).toBe(0);
    expect((update.retry as any).max_delay_ms).toBe(0);
  });

  it("writes timeout when provided", () => {
    const node = makeNode({ enabled: false });
    const update = buildBatchUpdate(node, { timeout: 30 });
    expect(update.timeout).toBe(30);
  });

  it("writes enabled when provided", () => {
    const node = makeNode({ enabled: false });
    const update = buildBatchUpdate(node, { enabled: false });
    expect(update.enabled).toBe(false);
  });

  it("does not touch retry when retryEnabled is null", () => {
    const node = makeNode({ enabled: false, max_retries: 5 });
    const update = buildBatchUpdate(node, { retryEnabled: null });
    expect(update.retry).toBeUndefined();
  });
});
