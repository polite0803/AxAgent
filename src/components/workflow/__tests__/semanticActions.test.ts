import {
  clearSemanticAction,
  flattenSemanticActions,
  isActionSelected,
  type SemanticActionMap,
  setSemanticAction,
} from "@/components/workflow/semanticActions";
import { describe, expect, it } from "vitest";

describe("SemanticActionMap - #6.2", () => {
  it("records action per (nodeId, skillId) pair", () => {
    const after = setSemanticAction({}, "n-1", "s-A", "replace");
    expect(after).toEqual({ "n-1": { "s-A": "replace" } });
  });

  it("supports multiple skills for the same node", () => {
    const a = setSemanticAction({}, "n-1", "s-A", "replace");
    const b = setSemanticAction(a, "n-1", "s-B", "keep");
    expect(b).toEqual({ "n-1": { "s-A": "replace", "s-B": "keep" } });
    expect(isActionSelected(b, "n-1", "s-A", "replace")).toBe(true);
    expect(isActionSelected(b, "n-1", "s-B", "keep")).toBe(true);
    expect(isActionSelected(b, "n-1", "s-A", "keep")).toBe(false);
  });

  it("overwrites the action for the same pair without affecting other pairs", () => {
    const a = setSemanticAction({}, "n-1", "s-A", "replace");
    const b = setSemanticAction(a, "n-1", "s-B", "keep");
    const c = setSemanticAction(b, "n-1", "s-A", "keep");
    expect(c["n-1"]["s-A"]).toBe("keep");
    expect(c["n-1"]["s-B"]).toBe("keep");
  });

  it("isolates actions between different nodes", () => {
    const a = setSemanticAction({}, "n-1", "s-A", "replace");
    const b = setSemanticAction(a, "n-2", "s-A", "keep");
    expect(b).toEqual({
      "n-1": { "s-A": "replace" },
      "n-2": { "s-A": "keep" },
    });
  });

  it("clearSemanticAction removes a single pair", () => {
    const a = setSemanticAction({}, "n-1", "s-A", "replace");
    const b = setSemanticAction(a, "n-1", "s-B", "keep");
    const c = clearSemanticAction(b, "n-1", "s-A");
    expect(c).toEqual({ "n-1": { "s-B": "keep" } });
  });

  it("clearSemanticAction removes the node entry when its last skill is cleared", () => {
    const a = setSemanticAction({}, "n-1", "s-A", "replace");
    const b = clearSemanticAction(a, "n-1", "s-A");
    expect(b).toEqual({});
  });

  it("flattenSemanticActions preserves all pairs", () => {
    const a = setSemanticAction({}, "n-1", "s-A", "replace");
    const b = setSemanticAction(a, "n-1", "s-B", "keep");
    const c = setSemanticAction(b, "n-2", "s-C", "replace");
    const flat = flattenSemanticActions(c);
    expect(flat).toHaveLength(3);
    expect(flat).toEqual(expect.arrayContaining([
      { nodeId: "n-1", skillId: "s-A", action: "replace" },
      { nodeId: "n-1", skillId: "s-B", action: "keep" },
      { nodeId: "n-2", skillId: "s-C", action: "replace" },
    ]));
  });

  it("isActionSelected returns false for missing node or skill", () => {
    const empty: SemanticActionMap = {};
    expect(isActionSelected(empty, "n-1", "s-A", "replace")).toBe(false);
    const a = setSemanticAction({}, "n-1", "s-A", "replace");
    expect(isActionSelected(a, "n-1", "s-MISSING", "replace")).toBe(false);
    expect(isActionSelected(a, "n-MISSING", "s-A", "replace")).toBe(false);
  });

  it("does not mutate the previous state", () => {
    const original: SemanticActionMap = {};
    const after = setSemanticAction(original, "n-1", "s-A", "replace");
    expect(original).toEqual({});
    expect(after).not.toBe(original);
  });
});
