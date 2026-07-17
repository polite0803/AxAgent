// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { ExecutionPhase } from "../executionPhaseMachine";
import type { CurrentToolCall } from "../executionToolCallUtils";
import { shouldClearToolCall } from "../executionToolCallUtils";

const makeToolCall = (
  overrides?: Partial<CurrentToolCall>,
): CurrentToolCall => ({
  toolName: "echo",
  toolUseId: "tool-1",
  conversationId: "conv-1",
  startedAt: Date.now(),
  ...overrides,
});

const makePhases = (
  phases: Record<string, ExecutionPhase>,
): Record<string, ExecutionPhase> => phases;

describe("shouldClearToolCall", () => {
  it("returns false when currentToolCall is null", () => {
    expect(shouldClearToolCall(null, {}, "conv-1")).toBe(false);
  });

  it("returns true when currentToolCall conversationId matches doneConversationId", () => {
    const tc = makeToolCall({ conversationId: "conv-1" });
    expect(shouldClearToolCall(tc, {}, "conv-1")).toBe(true);
  });

  it("returns false when currentToolCall belongs to a different conversation that is still active", () => {
    const tc = makeToolCall({ conversationId: "conv-1" });
    const phases = makePhases({ "conv-1": "executing" });
    expect(shouldClearToolCall(tc, phases, "conv-2")).toBe(false);
  });

  it("returns true when currentToolCall belongs to a different conversation that is in terminal phase", () => {
    const tc = makeToolCall({ conversationId: "conv-1" });
    const phases = makePhases({ "conv-1": "completed" });
    expect(shouldClearToolCall(tc, phases, "conv-2")).toBe(true);
  });

  it("returns true when currentToolCall belongs to a conversation in failed phase (cross-dialogue residual)", () => {
    const tc = makeToolCall({ conversationId: "conv-1" });
    const phases = makePhases({ "conv-1": "failed" });
    expect(shouldClearToolCall(tc, phases, "conv-2")).toBe(true);
  });

  it("returns true when currentToolCall belongs to a conversation in cancelled phase", () => {
    const tc = makeToolCall({ conversationId: "conv-1" });
    const phases = makePhases({ "conv-1": "cancelled" });
    expect(shouldClearToolCall(tc, phases, "conv-2")).toBe(true);
  });

  it("returns false when currentToolCall belongs to a conversation in planning phase (active)", () => {
    const tc = makeToolCall({ conversationId: "conv-1" });
    const phases = makePhases({ "conv-1": "planning" });
    expect(shouldClearToolCall(tc, phases, "conv-2")).toBe(false);
  });

  it("returns false when currentToolCall belongs to a conversation in waiting_permission phase (active)", () => {
    const tc = makeToolCall({ conversationId: "conv-1" });
    const phases = makePhases({ "conv-1": "waiting_permission" });
    expect(shouldClearToolCall(tc, phases, "conv-2")).toBe(false);
  });

  it("defaults to idle phase when owner conversation has no phase record", () => {
    const tc = makeToolCall({ conversationId: "conv-unknown" });
    expect(shouldClearToolCall(tc, {}, "conv-2")).toBe(false);
  });
});
