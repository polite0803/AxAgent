// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  executeActionChain,
  executeSkillAction,
  getCustomFunction,
  registerCustomHandler,
  unregisterCustomHandler,
} from "@/lib/skillActionExecutor";
import type { SkillCommandAction } from "@/types";

const mockNavigate = vi.fn();

const { actionRouterMock } = vi.hoisted(() => ({
  actionRouterMock: {
    execute: vi.fn(),
  },
}));

vi.mock("@/lib/actionRouter", () => ({
  getActionRouter: () => actionRouterMock,
}));

describe("skillActionExecutor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockNavigate.mockClear();
  });

  describe("executeSkillAction", () => {
    it("navigates for declarative navigate actions", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: {
          type: "navigate",
          path: "/settings",
        },
      };

      await executeSkillAction(action, mockNavigate);

      expect(mockNavigate).toHaveBeenCalledWith("/settings");
      expect(actionRouterMock.execute).not.toHaveBeenCalled();
    });

    it("delegates to action router for non-navigate declarative actions", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: {
          type: "invoke",
          command: "open-url",
          args: { url: "https://example.com" },
        },
      };

      actionRouterMock.execute.mockResolvedValueOnce(undefined);

      await executeSkillAction(action, mockNavigate);

      expect(mockNavigate).not.toHaveBeenCalled();
      expect(actionRouterMock.execute).toHaveBeenCalledWith(action, {
        skillName: "",
      });
    });

    it("delegates to action router for function actions", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: {
          type: "function",
          name: "custom_handler",
          args: [{ key: "value" }],
        },
      };

      actionRouterMock.execute.mockResolvedValueOnce(undefined);

      await executeSkillAction(action, mockNavigate);

      expect(actionRouterMock.execute).toHaveBeenCalledWith(action, {
        skillName: "",
      });
    });
  });

  describe("executeActionChain", () => {
    it("executes all actions in sequence", async () => {
      const actions: SkillCommandAction[] = [
        {
          mode: "declarative",
          action: { type: "navigate", path: "/page1" },
        },
        {
          mode: "declarative",
          action: { type: "navigate", path: "/page2" },
        },
      ];

      await executeActionChain(actions, mockNavigate);

      expect(mockNavigate).toHaveBeenNthCalledWith(1, "/page1");
      expect(mockNavigate).toHaveBeenNthCalledWith(2, "/page2");
    });

    it("handles empty action chain", async () => {
      await executeActionChain([], mockNavigate);
      expect(mockNavigate).not.toHaveBeenCalled();
      expect(actionRouterMock.execute).not.toHaveBeenCalled();
    });

    it("executes mixed declarative and function actions", async () => {
      const actions: SkillCommandAction[] = [
        {
          mode: "declarative",
          action: { type: "navigate", path: "/dashboard" },
        },
        {
          mode: "declarative",
          action: { type: "invoke", command: "call-api", args: { url: "https://api.example.com" } },
        },
        {
          mode: "declarative",
          action: { type: "function", name: "my_handler", args: [{ x: 1 }] },
        },
      ];

      actionRouterMock.execute.mockResolvedValue(undefined);

      await executeActionChain(actions, mockNavigate);

      expect(mockNavigate).toHaveBeenCalledOnce();
      expect(mockNavigate).toHaveBeenCalledWith("/dashboard");
      expect(actionRouterMock.execute).toHaveBeenCalledTimes(2);
    });
  });

  describe("custom handler registration", () => {
    it("registers and retrieves a custom handler", () => {
      const handler = vi.fn();
      registerCustomHandler("my_handler", handler);

      const retrieved = getCustomFunction("my_handler");
      expect(retrieved).toBe(handler);
    });

    it("returns undefined for unregistered handler", () => {
      expect(getCustomFunction("nonexistent")).toBeUndefined();
    });

    it("unregisters a custom handler", () => {
      const handler = vi.fn();
      registerCustomHandler("my_handler", handler);
      unregisterCustomHandler("my_handler");

      expect(getCustomFunction("my_handler")).toBeUndefined();
    });

    it("replaces existing handler with same id", () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      registerCustomHandler("my_handler", handler1);
      registerCustomHandler("my_handler", handler2);

      expect(getCustomFunction("my_handler")).toBe(handler2);
    });

    it("invokes handler with data and skillName", async () => {
      const handler = vi.fn().mockResolvedValue(undefined);
      registerCustomHandler("my_handler", handler);

      const retrieved = getCustomFunction("my_handler");
      await retrieved!({ foo: "bar" }, "test-skill");

      expect(handler).toHaveBeenCalledWith({ foo: "bar" }, "test-skill");
    });
  });
});
