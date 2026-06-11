// SPDX-License-Identifier: AGPL-3.0-only

import { countTerminalNodes, isDeadEndNode } from "@/components/workflow/DebugPanel/deadEnd";
import { describe, expect, it } from "vitest";

describe("isDeadEndNode - #6.6", () => {
  it("trigger nodes are never dead ends", () => {
    expect(isDeadEndNode({ id: "t1", nodeType: "trigger", hasIncoming: false, hasOutgoing: true }, 0)).toBe(false);
  });

  it("end nodes are never dead ends", () => {
    expect(isDeadEndNode({ id: "e1", nodeType: "end", hasIncoming: true, hasOutgoing: false }, 1)).toBe(false);
  });

  it("a node with no incoming edge is not a dead end", () => {
    expect(isDeadEndNode({ id: "n1", nodeType: "agent", hasIncoming: false, hasOutgoing: true }, 0)).toBe(false);
  });

  it("a node with outgoing edge is not a dead end", () => {
    expect(isDeadEndNode({ id: "n1", nodeType: "agent", hasIncoming: true, hasOutgoing: true }, 2)).toBe(false);
  });

  it("single terminal in workflow → not a dead end (sole legal exit)", () => {
    expect(isDeadEndNode({ id: "n1", nodeType: "agent", hasIncoming: true, hasOutgoing: false }, 1)).toBe(false);
  });

  it("multiple terminals in workflow → each terminal is a dead end (missing edge)", () => {
    expect(isDeadEndNode({ id: "n1", nodeType: "agent", hasIncoming: true, hasOutgoing: false }, 3)).toBe(true);
  });
});

describe("countTerminalNodes - #6.6", () => {
  it("counts nodes with incoming but no outgoing", () => {
    const n = countTerminalNodes([
      { id: "a", nodeType: "agent", hasIncoming: true, hasOutgoing: false },
      { id: "b", nodeType: "agent", hasIncoming: true, hasOutgoing: true },
      { id: "c", nodeType: "agent", hasIncoming: false, hasOutgoing: true },
    ]);
    expect(n).toBe(1);
  });

  it("returns 0 when no terminals", () => {
    expect(countTerminalNodes([
      { id: "a", nodeType: "agent", hasIncoming: true, hasOutgoing: true },
    ])).toBe(0);
  });
});
