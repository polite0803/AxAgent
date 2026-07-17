// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

import { resolveIconComponent } from "../skillIcons";

describe("skillIcons", () => {
  describe("resolveIconComponent", () => {
    it("应解析 lucide: 前缀的图标", () => {
      const icon = resolveIconComponent("lucide:FolderOpen");
      expect(icon).toBeDefined();
      // LucideIcon 是 ForwardRefExoticComponent，typeof 为 "object"
      expect(typeof icon).toBe("object");
    });

    it("应解析无前缀的图标", () => {
      const icon = resolveIconComponent("Package");
      expect(icon).toBeDefined();
      expect(typeof icon).toBe("object");
    });

    it("未识别的图标应回退到 Package", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const icon = resolveIconComponent("lucide:NonExistent");
      const pkg = resolveIconComponent("Package");
      expect(icon).toBe(pkg);
      expect(warn).toHaveBeenCalledWith(expect.stringContaining("未识别的图标"));
      warn.mockRestore();
    });

    it("应能解析常见图标", () => {
      const icons = ["Bell", "Settings", "Search", "User", "Play", "Check"];
      for (const name of icons) {
        expect(typeof resolveIconComponent(name)).toBe("object");
      }
    });
  });
});
