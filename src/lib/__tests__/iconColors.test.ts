import { describe, expect, it } from "vitest";

import {
  CHAT_ICON_COLORS,
  getIconColor,
  NAV_ICON_COLORS,
  SETTINGS_ICON_COLORS,
  TITLEBAR_ICON_COLORS,
} from "../iconColors";

describe("iconColors", () => {
  // ═══════════════════════════════════════════════════════════════
  // NAV_ICON_COLORS
  // ═══════════════════════════════════════════════════════════════
  describe("NAV_ICON_COLORS", () => {
    it("应包含导航栏所需的图标颜色映射", () => {
      expect(NAV_ICON_COLORS.MessageSquare).toBe("#3b82f6");
      expect(NAV_ICON_COLORS.Sparkles).toBe("#f59e0b");
      expect(NAV_ICON_COLORS.BookOpen).toBe("#10b981");
      expect(NAV_ICON_COLORS.Brain).toBe("#8b5cf6");
      expect(NAV_ICON_COLORS.Link2).toBe("#06b6d4");
      expect(NAV_ICON_COLORS.Router).toBe("#f97316");
      expect(NAV_ICON_COLORS.FolderOpen).toBe("#64748b");
    });

    it("所有颜色值应为有效的 hex 颜色", () => {
      for (const color of Object.values(NAV_ICON_COLORS)) {
        expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // SETTINGS_ICON_COLORS
  // ═══════════════════════════════════════════════════════════════
  describe("SETTINGS_ICON_COLORS", () => {
    it("应包含设置页图标颜色映射", () => {
      expect(SETTINGS_ICON_COLORS.Cloud).toBe("#3b82f6");
      expect(SETTINGS_ICON_COLORS.Bot).toBe("#22c55e");
      expect(SETTINGS_ICON_COLORS.Palette).toBe("#ec4899");
      expect(SETTINGS_ICON_COLORS.Zap).toBe("#eab308");
      expect(SETTINGS_ICON_COLORS.Database).toBe("#8b5cf6");
      expect(SETTINGS_ICON_COLORS.Workflow).toBe("#8b5cf6");
    });

    it("所有颜色值为有效 hex", () => {
      for (const color of Object.values(SETTINGS_ICON_COLORS)) {
        expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // TITLEBAR_ICON_COLORS
  // ═══════════════════════════════════════════════════════════════
  describe("TITLEBAR_ICON_COLORS", () => {
    it("应包含标题栏图标颜色", () => {
      expect(TITLEBAR_ICON_COLORS.Pin).toBe("#3b82f6");
      expect(TITLEBAR_ICON_COLORS.Sun).toBe("#f59e0b");
      expect(TITLEBAR_ICON_COLORS.Moon).toBe("#6366f1");
      expect(TITLEBAR_ICON_COLORS.Settings).toBe("#64748b");
      expect(TITLEBAR_ICON_COLORS.XCircle).toBe("#ef4444");
    });

    it("所有颜色值为有效 hex", () => {
      for (const color of Object.values(TITLEBAR_ICON_COLORS)) {
        expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // CHAT_ICON_COLORS
  // ═══════════════════════════════════════════════════════════════
  describe("CHAT_ICON_COLORS", () => {
    it("应包含常用聊天图标颜色", () => {
      expect(CHAT_ICON_COLORS.Pencil).toBe("#3b82f6");
      expect(CHAT_ICON_COLORS.Copy).toBe("#64748b");
      expect(CHAT_ICON_COLORS.Check).toBe("#22c55e");
      expect(CHAT_ICON_COLORS.Trash2).toBe("#ef4444");
      expect(CHAT_ICON_COLORS.Bot).toBe("#22c55e");
      expect(CHAT_ICON_COLORS.Code).toBe("#6366f1");
      expect(CHAT_ICON_COLORS.Mic).toBe("#ef4444");
      expect(CHAT_ICON_COLORS.Heart).toBe("#ef4444");
    });

    it("应有超过 80 个图标映射（丰富覆盖）", () => {
      const count = Object.keys(CHAT_ICON_COLORS).length;
      expect(count).toBeGreaterThan(80);
    });

    it("所有颜色值为有效 hex", () => {
      for (const color of Object.values(CHAT_ICON_COLORS)) {
        expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
      }
    });

    it("不同上下文间颜色应无冲突（各自独立）", () => {
      // 验证每个 context 的 map 是独立的
      expect(NAV_ICON_COLORS).not.toBe(SETTINGS_ICON_COLORS);
      expect(SETTINGS_ICON_COLORS).not.toBe(TITLEBAR_ICON_COLORS);
      expect(TITLEBAR_ICON_COLORS).not.toBe(CHAT_ICON_COLORS);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // getIconColor
  // ═══════════════════════════════════════════════════════════════
  describe("getIconColor", () => {
    it('context="nav" 时应从 NAV_ICON_COLORS 查找', () => {
      const color = getIconColor("Sparkles", "nav");
      expect(color).toBe("#f59e0b");
    });

    it('context="settings" 时应从 SETTINGS_ICON_COLORS 查找', () => {
      const color = getIconColor("Palette", "settings");
      expect(color).toBe("#ec4899");
    });

    it('context="titlebar" 时应从 TITLEBAR_ICON_COLORS 查找', () => {
      const color = getIconColor("Sun", "titlebar");
      expect(color).toBe("#f59e0b");
    });

    it("无 context 时应回退到 CHAT_ICON_COLORS", () => {
      const color = getIconColor("Pencil");
      expect(color).toBe("#3b82f6");
    });

    it("undefined context 应回退到 CHAT_ICON_COLORS", () => {
      const color = getIconColor("Bot", undefined);
      expect(color).toBe("#22c55e");
    });

    it("指定 context 中找不到时应返回 undefined", () => {
      // "Pencil" 在 NAV_ICON_COLORS 中不存在
      const color = getIconColor("Pencil", "nav");
      expect(color).toBeUndefined();
    });

    it("CHAT_ICON_COLORS 中也找不到时应返回 undefined", () => {
      const color = getIconColor("NonExistentIcon");
      expect(color).toBeUndefined();
    });
  });
});
