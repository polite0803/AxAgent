// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

// Mock i18n to return Chinese text
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

import type { SkillPermissions } from "@/types";
import {
  __NO_PERMISSIONS_DECLARED__,
  checkPermissionIntegrity,
  clearPermissionHash,
  extractRequiredCommands,
  isStoreReadCovered,
  isWildcardMatch,
  validateSkillPermissions,
} from "../skillPermissions";

describe("skillPermissions", () => {
  describe("isWildcardMatch", () => {
    it("精确匹配应返回 true", () => {
      expect(isWildcardMatch("read_file", ["read_file"])).toBe(true);
    });

    it("后缀通配符 * 应匹配前缀", () => {
      expect(isWildcardMatch("read_file", ["read_*"])).toBe(true);
    });

    it("后缀通配符不应匹配不相关的前缀", () => {
      expect(isWildcardMatch("write_file", ["read_*"])).toBe(false);
    });

    it("空模式列表应返回 false", () => {
      expect(isWildcardMatch("anything", [])).toBe(false);
    });

    it("多个模式中任一匹配即返回 true", () => {
      expect(isWildcardMatch("read_file", ["write_*", "read_*"])).toBe(true);
    });

    it("无通配符的不匹配应返回 false", () => {
      expect(isWildcardMatch("read_file", ["write_file"])).toBe(false);
    });
  });

  describe("extractRequiredCommands", () => {
    it("capabilities 为 undefined 时应返回空数组", () => {
      expect(extractRequiredCommands(undefined)).toEqual([]);
    });

    it("应提取 type=invoke 的 command", () => {
      const capabilities = [
        { type: "invoke", command: "read_file" },
        { type: "invoke", command: "write_file" },
      ];
      expect(extractRequiredCommands(capabilities)).toEqual([
        "read_file",
        "write_file",
      ]);
    });

    it("应提取 dynamicText 轮询命令", () => {
      const capabilities = [
        { command: "get_status", refreshIntervalMs: 5000 },
      ];
      expect(extractRequiredCommands(capabilities)).toEqual(["get_status"]);
    });

    it("应去重", () => {
      const capabilities = [
        { type: "invoke", command: "read_file" },
        { type: "invoke", command: "read_file" },
      ];
      expect(extractRequiredCommands(capabilities)).toEqual(["read_file"]);
    });

    it("应递归提取嵌套结构中的命令", () => {
      const capabilities = [
        {
          actions: [
            { type: "invoke", command: "nested_cmd" },
          ],
        },
      ];
      expect(extractRequiredCommands(capabilities)).toEqual(["nested_cmd"]);
    });
  });

  describe("validateSkillPermissions", () => {
    it("permissions 为 undefined 时应返回无效结果", () => {
      const result = validateSkillPermissions(undefined, ["read_file"]);
      expect(result.valid).toBe(false);
      expect(result.violations.some((v) => v.startsWith("UNAUTH:"))).toBe(true);
    });

    it("所需命令在权限白名单中时应返回有效", () => {
      const perms: SkillPermissions = { commands: ["read_file", "write_file"] };
      const result = validateSkillPermissions(perms, ["read_file"]);
      expect(result.valid).toBe(true);
    });

    it("所需命令不在白名单中时应返回无效", () => {
      const perms: SkillPermissions = { commands: ["read_file"] };
      const result = validateSkillPermissions(perms, ["write_file"]);
      expect(result.valid).toBe(false);
    });

    it("应拒绝访问 forbidden store（skill store）", () => {
      const perms: SkillPermissions = {
        storeRead: ["skill:name"],
        storeWrite: ["skill:state"],
      };
      const result = validateSkillPermissions(perms, []);
      expect(result.valid).toBe(false);
      expect(result.violations.some((v) => v.includes("forbidden"))).toBe(true);
    });

    it("有写权限无读权限时应给出提示", () => {
      const perms: SkillPermissions = {
        storeWrite: ["myStore:field"],
      };
      const result = validateSkillPermissions(perms, []);
      expect(result.violations.some((v) => v.includes("writeWithoutRead"))).toBe(
        true,
      );
    });

    it("声明 network 权限时应给出提示", () => {
      const perms: SkillPermissions = {
        network: ["https://api.example.com"],
      };
      const result = validateSkillPermissions(perms, []);
      expect(result.violations.some((v) => v.includes("networkDisabled"))).toBe(
        true,
      );
    });

    it("通过通配符匹配命令", () => {
      const perms: SkillPermissions = { commands: ["read_*"] };
      const result = validateSkillPermissions(perms, ["read_file", "read_config"]);
      expect(result.valid).toBe(true);
    });
  });

  describe("checkPermissionIntegrity", () => {
    it("首次调用应返回哈希值", async () => {
      const hash = await checkPermissionIntegrity("test-skill", {
        commands: ["read_file"],
      });
      expect(hash).toBeDefined();
      expect(hash).toHaveLength(16);
    });

    it("相同权限应返回相同哈希", async () => {
      const perms = { commands: ["read_file"] };
      const hash1 = await checkPermissionIntegrity("skill-a", perms);
      const hash2 = await checkPermissionIntegrity("skill-a", perms);
      expect(hash1).toBe(hash2);
    });

    it("权限变更时应发出 console.warn", async () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

      // 需要先清除之前的哈希缓存
      clearPermissionHash("skill-b");
      await checkPermissionIntegrity("skill-b", { commands: ["a"] });
      await checkPermissionIntegrity("skill-b", { commands: ["b"] });

      expect(warn).toHaveBeenCalledWith(
        expect.stringContaining("Permission manifest changed"),
      );

      warn.mockRestore();
    });
  });

  describe("clearPermissionHash", () => {
    it("应清除哈希缓存", async () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

      clearPermissionHash("skill-c");
      await checkPermissionIntegrity("skill-c", { commands: ["a"] });
      clearPermissionHash("skill-c");
      await checkPermissionIntegrity("skill-c", { commands: ["b"] });

      // 清除后不应触发变更警告
      const changeWarnings = warn.mock.calls.filter((call) => String(call[0]).includes("Permission manifest changed"));
      expect(changeWarnings).toHaveLength(0);

      warn.mockRestore();
    });
  });

  describe("isStoreReadCovered / isStoreWriteCovered", () => {
    it("精确匹配 store 名应返回 true", () => {
      expect(isStoreReadCovered("myStore", undefined, ["myStore"])).toBe(true);
    });

    it("通配 store 名无 fieldPath 时应返回 true", () => {
      expect(isStoreReadCovered("myStore", "field", ["myStore"])).toBe(true);
    });

    it("带 fieldPath 的精确匹配应返回 true", () => {
      expect(isStoreReadCovered("myStore", "name", ["myStore:name"])).toBe(true);
    });

    it("子字段匹配应返回 true", () => {
      expect(isStoreReadCovered("myStore", "name.first", ["myStore:name"])).toBe(
        true,
      );
    });

    it("不相干的 store 应返回 false", () => {
      expect(isStoreReadCovered("myStore", "field", ["otherStore"])).toBe(false);
    });

    it("不相干的 fieldPath 应返回 false", () => {
      expect(isStoreReadCovered("myStore", "age", ["myStore:name"])).toBe(false);
    });
  });
});
