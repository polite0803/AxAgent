// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import { ACTIVE_PHASES, type ExecutionPhase, PHASE_TRANSITIONS, TERMINAL_PHASES } from "../executionPhaseMachine";

describe("executionPhaseMachine", () => {
  describe("PHASE_TRANSITIONS", () => {
    it("idle can transition to planning, executing, completed, failed, cancelled", () => {
      expect(PHASE_TRANSITIONS.idle).toEqual([
        "planning",
        "executing",
        "completed",
        "failed",
        "cancelled",
      ]);
    });

    it("idle cannot transition to waiting_permission", () => {
      expect(PHASE_TRANSITIONS.idle).not.toContain("waiting_permission");
    });

    it("planning can transition to executing, waiting_permission, failed, cancelled", () => {
      expect(PHASE_TRANSITIONS.planning).toContain("executing");
      expect(PHASE_TRANSITIONS.planning).toContain("waiting_permission");
      expect(PHASE_TRANSITIONS.planning).toContain("failed");
      expect(PHASE_TRANSITIONS.planning).toContain("cancelled");
    });

    it("planning cannot transition to idle or completed", () => {
      expect(PHASE_TRANSITIONS.planning).not.toContain("idle");
      expect(PHASE_TRANSITIONS.planning).not.toContain("completed");
    });

    it("executing can self-transition (re-entry)", () => {
      expect(PHASE_TRANSITIONS.executing).toContain("executing");
    });

    it("executing can transition to waiting_permission, completed, failed, cancelled", () => {
      expect(PHASE_TRANSITIONS.executing).toContain("waiting_permission");
      expect(PHASE_TRANSITIONS.executing).toContain("completed");
      expect(PHASE_TRANSITIONS.executing).toContain("failed");
      expect(PHASE_TRANSITIONS.executing).toContain("cancelled");
    });

    it("waiting_permission can transition to executing or cancelled", () => {
      expect(PHASE_TRANSITIONS.waiting_permission).toEqual([
        "executing",
        "cancelled",
      ]);
    });

    it("waiting_permission cannot transition to idle", () => {
      expect(PHASE_TRANSITIONS.waiting_permission).not.toContain("idle");
    });

    it("completed can transition to idle or executing (restart)", () => {
      expect(PHASE_TRANSITIONS.completed).toContain("idle");
      expect(PHASE_TRANSITIONS.completed).toContain("executing");
    });

    it("failed can transition to idle or executing (retry)", () => {
      expect(PHASE_TRANSITIONS.failed).toContain("idle");
      expect(PHASE_TRANSITIONS.failed).toContain("executing");
    });

    it("cancelled can transition to idle or executing (restart)", () => {
      expect(PHASE_TRANSITIONS.cancelled).toContain("idle");
      expect(PHASE_TRANSITIONS.cancelled).toContain("executing");
    });

    it("every phase has at least one outgoing transition", () => {
      const phases: ExecutionPhase[] = [
        "idle",
        "planning",
        "executing",
        "waiting_permission",
        "completed",
        "failed",
        "cancelled",
      ];
      for (const phase of phases) {
        expect(PHASE_TRANSITIONS[phase].length).toBeGreaterThan(0);
      }
    });
  });

  describe("ACTIVE_PHASES", () => {
    it("planning, executing, waiting_permission are active", () => {
      expect(ACTIVE_PHASES.has("planning")).toBe(true);
      expect(ACTIVE_PHASES.has("executing")).toBe(true);
      expect(ACTIVE_PHASES.has("waiting_permission")).toBe(true);
    });

    it("idle, completed, failed, cancelled are NOT active", () => {
      expect(ACTIVE_PHASES.has("idle")).toBe(false);
      expect(ACTIVE_PHASES.has("completed")).toBe(false);
      expect(ACTIVE_PHASES.has("failed")).toBe(false);
      expect(ACTIVE_PHASES.has("cancelled")).toBe(false);
    });
  });

  describe("TERMINAL_PHASES", () => {
    it("completed, failed, cancelled are terminal", () => {
      expect(TERMINAL_PHASES.has("completed")).toBe(true);
      expect(TERMINAL_PHASES.has("failed")).toBe(true);
      expect(TERMINAL_PHASES.has("cancelled")).toBe(true);
    });

    it("idle, planning, executing, waiting_permission are NOT terminal", () => {
      expect(TERMINAL_PHASES.has("idle")).toBe(false);
      expect(TERMINAL_PHASES.has("planning")).toBe(false);
      expect(TERMINAL_PHASES.has("executing")).toBe(false);
      expect(TERMINAL_PHASES.has("waiting_permission")).toBe(false);
    });

    it("active and terminal sets are disjoint", () => {
      for (const phase of ACTIVE_PHASES) {
        expect(TERMINAL_PHASES.has(phase)).toBe(false);
      }
    });
  });
});
