// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock i18n
vi.mock("@/i18n", () => ({
  default: {
    t: (key: string, params?: Record<string, string>) => {
      if (params) {
        return `${key}: ${JSON.stringify(params)}`;
      }
      return key;
    },
  },
}));

// Mock invoke
const mockInvoke = vi.fn();
vi.mock("@/lib/invoke", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
  isTauri: () => false,
  // agentStore 在模块初始化阶段调用 listen 订阅计划确认事件，
  // 此处补齐导出以避免 "No listen export" 错误。
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

// Mock skillPermissions
vi.mock("@/lib/skillPermissions", () => ({
  isStoreReadCovered: (store: string, _field: string | undefined, perms: string[]) =>
    perms.includes(store) || perms.includes("*"),
  isStoreWriteCovered: (store: string, _field: string | undefined, perms: string[]) =>
    perms.includes(store) || perms.includes("*"),
  isWildcardMatch: (target: string, patterns: string[]) =>
    patterns.some((p: string) => {
      if (p.endsWith("*")) { return target.startsWith(p.slice(0, -1)); }
      return target === p;
    }),
}));

// Mock storeRegistry
const mockStoreRegistry = new Map<string, { get: () => unknown; set: () => unknown; update: () => unknown }>();
vi.mock("@/lib/storeRegistry", () => ({
  getStoreRegistry: () => mockStoreRegistry,
}));

// Mock skillActionExecutor
vi.mock("@/lib/skillActionExecutor", () => ({
  getCustomFunction: vi.fn().mockReturnValue(vi.fn()),
}));

// Mock stores — actionRouter 直接静态导入具体 store 文件（不再经 @/stores barrel），
// 因此按实际导入路径 mock，避免加载真实 store 及其重型依赖树。
const mockCreateConversation = vi.fn();
const mockSendMessage = vi.fn();
const mockGetHandler = vi.fn();

vi.mock("@/stores/domain/conversationStore", () => ({
  useConversationStore: {
    getState: () => ({
      createConversation: mockCreateConversation,
      sendMessage: mockSendMessage,
    }),
  },
}));

vi.mock("@/stores/feature/providerStore", () => ({
  useProviderStore: {
    getState: () => ({
      providers: [],
    }),
  },
}));

vi.mock("@/stores/feature/settingsStore", () => ({
  useSettingsStore: {
    getState: () => ({
      settings: {},
    }),
  },
}));

vi.mock("@/stores/feature/skillExtensionStore", () => ({
  useSkillExtensionStore: {
    getState: () => ({
      getHandler: mockGetHandler,
      skills: [],
    }),
  },
}));

import type { SkillCommandAction } from "@/types";
import { ActionRouter, getActionRouter, getDefaultNavigate, setDefaultNavigate } from "../actionRouter";

describe("actionRouter", () => {
  let router: ActionRouter;

  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    mockStoreRegistry.clear();
    (getActionRouter as () => ActionRouter)();
    router = getActionRouter();
  });

  describe("getActionRouter", () => {
    it("应返回单例", () => {
      const a = getActionRouter();
      const b = getActionRouter();
      expect(a).toBe(b);
    });
  });

  describe("setDefaultNavigate / getDefaultNavigate", () => {
    it("应能设置和获取默认导航函数", () => {
      const nav = vi.fn();
      setDefaultNavigate(nav);
      expect(getDefaultNavigate()).toBe(nav);
    });
  });

  describe("validateAction (via executeDeclarative)", () => {
    it("未知 action type 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "unknown" as "invoke", command: "" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("unknownType");
    });

    it("invoke 缺少 command 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "invoke", command: "" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("missingField");
    });

    it("navigate 缺少 path 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "navigate", path: "" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("missingField");
    });

    it("emit 缺少 event 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "emit", event: "" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("missingField");
    });

    it("store 缺少 storeName 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "store", storeName: "", operation: "get" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("missingField");
    });

    it("store 缺少 operation 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "store", storeName: "myStore", operation: "" as "get" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("missingField");
    });

    it("field type 不匹配应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "invoke", command: 123 as unknown as string },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
    });
  });

  describe("execute: invoke", () => {
    it("应调用 invoke 并返回结果", async () => {
      mockInvoke.mockResolvedValue({ data: "result" });

      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "invoke", command: "read_file", args: { path: "/test" } },
      };
      const result = await router.execute(action, { skillName: "test" });

      expect(result.success).toBe(true);
      expect(mockInvoke).toHaveBeenCalledWith("read_file", { path: "/test" });
    });

    it("权限不足时应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "invoke", command: "read_file" },
      };
      const result = await router.execute(action, {
        skillName: "test",
        permissions: { commands: ["write_file"] },
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("commandPermissionDenied");
    });
  });

  describe("execute: navigate", () => {
    it("路径遍历检测应拒绝 ../ 路径", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "navigate", path: "../admin" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("navigatePathTraversal");
    });

    it("路径遍历检测应拒绝 // 路径", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "navigate", path: "//secret" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
    });

    it("应使用注入的 navigate 函数", async () => {
      const nav = vi.fn();
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "navigate", path: "/settings" },
      };
      const result = await router.execute(action, {
        skillName: "test",
        navigate: nav,
      });

      expect(result.success).toBe(true);
      expect(nav).toHaveBeenCalledWith("/settings");
    });

    it("无 navigate 注入且无默认导航时应降级到 hash", async () => {
      // Ensure no default navigate set
      setDefaultNavigate(null as unknown as (path: string) => void);

      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "navigate", path: "/fallback" },
      };
      const result = await router.execute(action, { skillName: "test" });

      expect(result.success).toBe(true);
      expect(window.location.hash).toBe("#/fallback");
    });

    it("权限不足时应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "navigate", path: "/admin" },
      };
      const result = await router.execute(action, {
        skillName: "test",
        permissions: { navigate: ["/settings"] },
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("navigatePermissionDenied");
    });
  });

  describe("execute: emit", () => {
    it("缺少命名空间前缀应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "emit", event: "no-namespace" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("emitMissingNamespace");
    });

    it("保留 DOM 事件名应被拒绝", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "emit", event: "click" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
    });

    it("应派发 CustomEvent", async () => {
      const handler = vi.fn();
      window.addEventListener("skill:test", handler);

      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "emit", event: "skill:test", payload: { data: 42 } },
      };
      const result = await router.execute(action, { skillName: "test" });

      expect(result.success).toBe(true);
      expect(handler).toHaveBeenCalledTimes(1);
      expect(handler.mock.calls[0][0].detail).toEqual({ data: 42 });

      window.removeEventListener("skill:test", handler);
    });

    it("权限不足时应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "emit", event: "skill:secret" },
      };
      const result = await router.execute(action, {
        skillName: "test",
        permissions: { events: ["skill:public"] },
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("emitPermissionDenied");
    });
  });

  describe("execute: store", () => {
    beforeEach(() => {
      mockStoreRegistry.set("myStore", {
        get: vi.fn().mockReturnValue({ name: "test" }),
        set: vi.fn(),
        update: vi.fn(),
      });
    });

    it("未知 operation 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "store", storeName: "myStore", operation: "delete" as "get" },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("unknownStoreOp");
    });

    it("set 操作 payload 应为对象类型", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: {
          type: "store",
          storeName: "myStore",
          operation: "set",
          payload: "invalid",
        },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      // field type validation 先于 payload structure validation 执行
      expect(result.error).toContain("fieldTypeMismatch");
    });

    it("未注册的 store 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: {
          type: "store",
          storeName: "nonexistent",
          operation: "get",
          payload: {},
        },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("storeNotRegistered");
    });

    it("get 操作应返回 store 数据", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: {
          type: "store",
          storeName: "myStore",
          operation: "get",
          payload: {},
        },
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(true);
    });
  });

  describe("execute: agentic", () => {
    it("prompt 为空时应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "agentic",
        prompt: "   ",
      };
      const result = await router.execute(action, { skillName: "test" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("agenticMissingPrompt");
    });
  });

  describe("executeChain", () => {
    it("空链应返回 success", async () => {
      const result = await router.executeChain([], { skillName: "test" });
      expect(result.success).toBe(true);
    });

    it("链中某步失败应中断后续步骤", async () => {
      const step1: SkillCommandAction = {
        mode: "declarative",
        action: { type: "invoke", command: "" },
      };
      const step2: SkillCommandAction = {
        mode: "declarative",
        action: { type: "invoke", command: "should_not_run" },
      };
      const result = await router.executeChain([step1, step2], {
        skillName: "test",
      });

      expect(result.success).toBe(false);
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it("深度超过 20 应返回错误", async () => {
      const action: SkillCommandAction = {
        mode: "declarative",
        action: { type: "navigate", path: "/" },
      };
      const result = await router.executeChain([action], { skillName: "test" }, 21);
      expect(result.success).toBe(false);
      expect(result.error).toContain("chainDepthExceeded");
    });
  });

  describe("execute: update-schema", () => {
    it("应派发 CustomEvent 通知 DynamicUIRenderer", async () => {
      const handler = vi.fn();
      window.addEventListener("dynamic-ui:schema-update", handler);

      const action: SkillCommandAction = {
        mode: "declarative",
        action: {
          type: "update-schema",
          schemaId: "dashboard",
          operation: "replace",
          path: "/items",
          newSchema: { type: "list" },
        },
      };
      const result = await router.execute(action, { skillName: "test" });

      expect(result.success).toBe(true);
      expect(handler).toHaveBeenCalledTimes(1);

      window.removeEventListener("dynamic-ui:schema-update", handler);
    });
  });

  describe("registerDeclarativeExecutor", () => {
    it("注册 custom: 前缀的 executor 不会发出警告", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      router.registerDeclarativeExecutor(
        "custom:myAction",
        vi.fn().mockResolvedValue({
          success: true,
          data: "custom",
        }),
      );

      // custom: 前缀的类型不会触发警告
      expect(warn).not.toHaveBeenCalled();
      warn.mockRestore();
    });
  });
});
