// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ActionResult } from "../actionRouter";

// Mock modules
const mockExecute = vi.fn().mockResolvedValue({ success: true } as ActionResult);
const mockInvoke = vi.fn();

vi.mock("../actionRouter", () => ({
  getActionRouter: () => ({
    execute: mockExecute,
  }),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
  logIpcError: (tag: string) => (e: unknown) => {
    console.error(tag, e);
  },
}));

// Mock store for triggerSkillReload
// skillLifecycle 直接静态导入 @/stores/feature/skillExtensionStore（不再经 @/stores barrel）
const mockRefreshSkill = vi.fn();
vi.mock("@/stores/feature/skillExtensionStore", () => ({
  useSkillExtensionStore: {
    getState: () => ({ refreshSkill: mockRefreshSkill }),
  },
}));

import type { SkillManifest } from "@/types";
import {
  getLifecycleCacheTtl,
  invalidateLifecycleCache,
  setLifecycleCacheTtl,
  triggerOnDisable,
  triggerOnEnable,
  triggerOnInstall,
  triggerOnUninstall,
  triggerSkillReload,
} from "../skillLifecycle";

const BASIC_MANIFEST: SkillManifest = {
  name: "test-skill",
  version: "1.0.0",
  description: "Test skill",
};

describe("skillLifecycle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invalidateLifecycleCache("test-skill");
    setLifecycleCacheTtl(5 * 60 * 1000);
  });

  describe("getLifecycleCacheTtl / setLifecycleCacheTtl", () => {
    it("默认 TTL 应为 5 分钟", () => {
      expect(getLifecycleCacheTtl()).toBe(5 * 60 * 1000);
    });

    it("应能设置自定义 TTL", () => {
      setLifecycleCacheTtl(1000);
      expect(getLifecycleCacheTtl()).toBe(1000);
    });

    it("设置 0 或负数可禁用缓存", () => {
      setLifecycleCacheTtl(0);
      expect(getLifecycleCacheTtl()).toBe(0);
    });
  });

  describe("triggerOnInstall", () => {
    it("skill 无 lifecycle 钩子时不应调用 execute", async () => {
      mockInvoke.mockResolvedValue({ manifest: BASIC_MANIFEST });

      await triggerOnInstall("test-skill");

      expect(mockExecute).not.toHaveBeenCalled();
    });

    it("skill 有 onInstall 钩子时应调用 execute", async () => {
      const lifecycleActions = [
        {
          mode: "declarative" as const,
          action: { type: "invoke" as const, command: "init_skill" },
        },
      ];
      const manifest: SkillManifest = {
        ...BASIC_MANIFEST,
        lifecycle: { onInstall: lifecycleActions },
      };
      mockInvoke.mockResolvedValue({ manifest });

      await triggerOnInstall("test-skill");

      expect(mockExecute).toHaveBeenCalledTimes(1);
      expect(mockExecute).toHaveBeenCalledWith(
        lifecycleActions[0],
        expect.objectContaining({ skillName: "test-skill" }),
      );
    });

    it("invoke 失败时应优雅处理", async () => {
      mockInvoke.mockRejectedValue(new Error("IPC error"));

      // 重试机制有 3 次退避延迟（1s + 2s + 4s），需要更长的超时
      await expect(triggerOnInstall("test-skill")).resolves.not.toThrow();
    }, 10000);
  });

  describe("triggerOnEnable", () => {
    it("skill 有 onEnable 钩子时应调用 execute", async () => {
      const lifecycleActions = [
        {
          mode: "declarative" as const,
          action: { type: "emit" as const, event: "skill:enabled" },
        },
      ];
      const manifest: SkillManifest = {
        ...BASIC_MANIFEST,
        lifecycle: { onEnable: lifecycleActions },
      };
      mockInvoke.mockResolvedValue({ manifest });

      await triggerOnEnable("test-skill");

      expect(mockExecute).toHaveBeenCalledTimes(1);
    });
  });

  describe("triggerOnDisable", () => {
    it("skill 有 onDisable 钩子时应顺序执行", async () => {
      const lifecycleActions = [
        {
          mode: "declarative" as const,
          action: { type: "emit" as const, event: "skill:disabled" },
        },
      ];
      const manifest: SkillManifest = {
        ...BASIC_MANIFEST,
        lifecycle: { onDisable: lifecycleActions },
      };
      mockInvoke.mockResolvedValue({ manifest });

      await triggerOnDisable("test-skill");

      expect(mockExecute).toHaveBeenCalledTimes(1);
    });

    it("多个钩子应顺序执行", async () => {
      const lifecycleActions = [
        {
          mode: "declarative" as const,
          action: { type: "emit" as const, event: "skill:step1" },
        },
        {
          mode: "declarative" as const,
          action: { type: "emit" as const, event: "skill:step2" },
        },
      ];
      const manifest: SkillManifest = {
        ...BASIC_MANIFEST,
        lifecycle: { onDisable: lifecycleActions },
      };
      mockInvoke.mockResolvedValue({ manifest });

      await triggerOnDisable("test-skill");

      expect(mockExecute).toHaveBeenCalledTimes(2);
    });
  });

  describe("triggerOnUninstall", () => {
    it("skill 有 onUninstall 钩子时应顺序执行", async () => {
      const lifecycleActions = [
        {
          mode: "declarative" as const,
          action: { type: "invoke" as const, command: "cleanup" },
        },
      ];
      const manifest: SkillManifest = {
        ...BASIC_MANIFEST,
        lifecycle: { onUninstall: lifecycleActions },
      };
      mockInvoke.mockResolvedValue({ manifest });

      await triggerOnUninstall("test-skill");

      expect(mockExecute).toHaveBeenCalledTimes(1);
    });
  });

  describe("triggerSkillReload", () => {
    it("应清除缓存并刷新 skill", async () => {
      await triggerSkillReload("test-skill");

      expect(mockRefreshSkill).toHaveBeenCalledWith("test-skill");
    });
  });

  describe("缓存行为", () => {
    it("TTL 内应使用缓存不重复调用 invoke", async () => {
      setLifecycleCacheTtl(60000); // 1 min
      mockInvoke.mockResolvedValue({ manifest: BASIC_MANIFEST });

      await triggerOnInstall("test-skill");
      await triggerOnInstall("test-skill");

      // invoke 只应被调用一次（第二次命中缓存）
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    it("disable 缓存后每次都应调用 invoke", async () => {
      setLifecycleCacheTtl(0); // disable cache
      mockInvoke.mockResolvedValue({ manifest: BASIC_MANIFEST });

      await triggerOnInstall("test-skill");
      await triggerOnInstall("test-skill");

      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });
  });
});
