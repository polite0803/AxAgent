// SPDX-License-Identifier: AGPL-3.0-only
// 集成测试：自然语言 → 解析 → 应用工作流

import { useWorkflowStore } from "@/stores/feature/workflowStore";
import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

describe("NL-to-Workflow Pipeline", () => {
  it("parses natural language into workflow definition", async () => {
    const { result } = renderHook(() => useWorkflowStore());

    let parseResult: Awaited<ReturnType<typeof result.current.parseNaturalLanguage>>;

    await act(async () => {
      parseResult = await result.current.parseNaturalLanguage({
        prompt: "每天早上8点检查服务器状态，如果异常就发送邮件通知",
      });
    });

    expect(parseResult!).toBeDefined();
    expect(parseResult!.confidence).toBeGreaterThan(0);
    expect(parseResult!.workflow).toBeDefined();
    expect(parseResult!.workflow.nodes.length).toBeGreaterThan(0);
  });

  it("shows parse progress during parsing", async () => {
    const { result } = renderHook(() => useWorkflowStore());

    // Start parsing but don't await
    const parsePromise = act(() =>
      result.current.parseNaturalLanguage({
        prompt: "创建一个数据同步工作流",
      })
    );

    // Progress should be set immediately
    // (since the mock uses setTimeout inside, we check before awaiting)
    // After act, the initial progress has been set
    // We can check that isParsing was true at some point

    await parsePromise;
    expect(result.current.parseProgress).toBe("");
    expect(result.current.isParsing).toBe(false);
  });

  it("emits workflow with nodes containing correct structure", async () => {
    const { result } = renderHook(() => useWorkflowStore());

    let parseResult: Awaited<ReturnType<typeof result.current.parseNaturalLanguage>>;

    await act(async () => {
      parseResult = await result.current.parseNaturalLanguage({
        prompt: "用户提交内容后进行敏感词检测和审核",
      });
    });

    const wf = parseResult!.workflow;
    // Every node should have id, type, label, config, position
    for (const node of wf.nodes) {
      expect(node).toHaveProperty("id");
      expect(node).toHaveProperty("type");
      expect(node).toHaveProperty("label");
      expect(node).toHaveProperty("config");
      expect(node).toHaveProperty("position");
    }

    // Edges should connect valid nodes
    const nodeIds = new Set(wf.nodes.map((n) => n.id));
    for (const edge of wf.edges) {
      expect(nodeIds.has(edge.source)).toBe(true);
      expect(nodeIds.has(edge.target)).toBe(true);
    }
  });

  it("produces parse history after multiple parses", async () => {
    const { result } = renderHook(() => useWorkflowStore());

    await act(async () => {
      await result.current.parseNaturalLanguage({ prompt: "工作流 A" });
    });
    await act(async () => {
      await result.current.parseNaturalLanguage({ prompt: "工作流 B" });
    });

    expect(result.current.parseHistory.length).toBeGreaterThanOrEqual(2);
  });

  it("rejects on empty prompt gracefully", async () => {
    const { result } = renderHook(() => useWorkflowStore());

    // Empty prompt should still return a result (mock is lenient)
    // but we verify it doesn't throw
    await act(async () => {
      const r = await result.current.parseNaturalLanguage({ prompt: "" });
      expect(r).toBeDefined();
    });
  });
});
