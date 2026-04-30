import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ─── Mock 设置 ────────────────────────────────────────────────────

const mockIsTauri = vi.fn();
const mockInvoke = vi.fn();

vi.mock("@/lib/invoke", () => ({
  isTauri: mockIsTauri,
  invoke: mockInvoke,
}));

vi.mock("@/lib/shortcuts", async () => {
  const actual = await vi.importActual<typeof import("../shortcuts")>("../shortcuts");
  return {
    ...actual,
  };
});

const mockMessageInfo = vi.fn();
vi.mock("antd", () => ({
  message: { info: mockMessageInfo },
}));

const mockITranslate = vi.fn();
vi.mock("@/i18n", () => ({
  default: { t: mockITranslate },
}));

const mockSettingsStoreGetState = vi.fn();
vi.mock("@/stores", () => ({
  useSettingsStore: { getState: mockSettingsStoreGetState },
}));

const mockGetCurrentWindow = {
  isVisible: vi.fn(),
  show: vi.fn(),
  hide: vi.fn(),
  setFocus: vi.fn(),
  close: vi.fn(),
};

const mockGetAllWindows = vi.fn();

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => mockGetCurrentWindow,
  getAllWindows: mockGetAllWindows,
}));

import type { ShortcutAction } from "../shortcuts";
import { executeShortcutAction } from "../shortcutActions";

// ─── 辅助函数 ──────────────────────────────────────────────────────

/** 模拟禁用了 toast 通知的设置 */
function mockSettings(overrides: Record<string, unknown> = {}) {
  mockSettingsStoreGetState.mockReturnValue({
    settings: {
      shortcut_trigger_toast_enabled: false,
      ...overrides,
    },
  });
}

/** 捕获 dispatchEvent 调用 */
function setupDispatchSpy() {
  const spy = vi.spyOn(window, "dispatchEvent");
  return spy;
}

// ─── 测试套件 ──────────────────────────────────────────────────────

describe("shortcutActions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSettings();
    mockIsTauri.mockReturnValue(false);
    // 重置 location
    delete (window as any).location;
    (window as any).location = { pathname: "/", href: "/" };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ═══════════════════════════════════════════════════════════════
  // 通用：toast 通知
  // ═══════════════════════════════════════════════════════════════
  describe("toast 通知", () => {
    it("shortcut_trigger_toast_enabled=false 时不弹出 toast", async () => {
      mockSettings({ shortcut_trigger_toast_enabled: false });

      await executeShortcutAction("newConversation");

      expect(mockMessageInfo).not.toHaveBeenCalled();
    });

    it("shortcut_trigger_toast_enabled=true 时应弹出 toast", async () => {
      mockSettings({ shortcut_trigger_toast_enabled: true });
      mockITranslate.mockImplementation((key: string) => key);

      await executeShortcutAction("newConversation");

      expect(mockMessageInfo).toHaveBeenCalled();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // 窗口控制类快捷键
  // ═══════════════════════════════════════════════════════════════
  describe("窗口控制快捷键", () => {
    it("toggleCurrentWindow: 非 Tauri 环境应直接返回", async () => {
      mockIsTauri.mockReturnValue(false);

      await executeShortcutAction("toggleCurrentWindow");

      expect(mockGetCurrentWindow.isVisible).not.toHaveBeenCalled();
    });

    it("toggleCurrentWindow: 窗口可见时应隐藏", async () => {
      mockIsTauri.mockReturnValue(true);
      mockGetCurrentWindow.isVisible.mockResolvedValue(true);

      await executeShortcutAction("toggleCurrentWindow");

      expect(mockGetCurrentWindow.hide).toHaveBeenCalled();
      expect(mockGetCurrentWindow.show).not.toHaveBeenCalled();
    });

    it("toggleCurrentWindow: 窗口不可见时应显示并聚焦", async () => {
      mockIsTauri.mockReturnValue(true);
      mockGetCurrentWindow.isVisible.mockResolvedValue(false);

      await executeShortcutAction("toggleCurrentWindow");

      expect(mockGetCurrentWindow.show).toHaveBeenCalled();
      expect(mockGetCurrentWindow.setFocus).toHaveBeenCalled();
      expect(mockGetCurrentWindow.hide).not.toHaveBeenCalled();
    });

    it("toggleAllWindows: 任一窗口可见时应全部隐藏", async () => {
      mockIsTauri.mockReturnValue(true);
      const win1 = { isVisible: vi.fn().mockResolvedValue(true), show: vi.fn(), hide: vi.fn(), setFocus: vi.fn() };
      const win2 = { isVisible: vi.fn().mockResolvedValue(false), show: vi.fn(), hide: vi.fn(), setFocus: vi.fn() };
      mockGetAllWindows.mockResolvedValue([win1, win2]);

      await executeShortcutAction("toggleAllWindows");

      expect(win1.hide).toHaveBeenCalled();
      expect(win2.hide).toHaveBeenCalled();
      expect(win1.show).not.toHaveBeenCalled();
    });

    it("toggleAllWindows: 全部不可见时应全部显示", async () => {
      mockIsTauri.mockReturnValue(true);
      const win1 = { isVisible: vi.fn().mockResolvedValue(false), show: vi.fn(), hide: vi.fn(), setFocus: vi.fn() };
      const win2 = { isVisible: vi.fn().mockResolvedValue(false), show: vi.fn(), hide: vi.fn(), setFocus: vi.fn() };
      mockGetAllWindows.mockResolvedValue([win1, win2]);

      await executeShortcutAction("toggleAllWindows");

      expect(win1.show).toHaveBeenCalled();
      expect(win2.show).toHaveBeenCalled();
      expect(win1.hide).not.toHaveBeenCalled();
    });

    it("toggleAllWindows: 无窗口时应直接返回", async () => {
      mockIsTauri.mockReturnValue(true);
      mockGetAllWindows.mockResolvedValue([]);

      await executeShortcutAction("toggleAllWindows");

      // 不应抛异常，正常返回即可
    });

    it("closeWindow: 应关闭当前窗口", async () => {
      mockIsTauri.mockReturnValue(true);

      await executeShortcutAction("closeWindow");

      expect(mockGetCurrentWindow.close).toHaveBeenCalled();
    });

    it("closeWindow: 非 Tauri 环境应直接返回", async () => {
      mockIsTauri.mockReturnValue(false);

      await executeShortcutAction("closeWindow");

      expect(mockGetCurrentWindow.close).not.toHaveBeenCalled();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // 聊天事件派发类快捷键
  // ═══════════════════════════════════════════════════════════════
  describe("事件派发快捷键", () => {
    it("newConversation: 应派发 axagent:new-conversation 事件", async () => {
      const spy = setupDispatchSpy();

      await executeShortcutAction("newConversation");

      expect(spy).toHaveBeenCalled();
      const event = spy.mock.calls.find((c) => (c[0] as CustomEvent).type === "axagent:new-conversation");
      expect(event).toBeDefined();
    });

    it("toggleModelSelector: 应派发 axagent:toggle-model-selector 事件", async () => {
      const spy = setupDispatchSpy();

      await executeShortcutAction("toggleModelSelector");

      const event = spy.mock.calls.find(
        (c) => (c[0] as CustomEvent).type === "axagent:toggle-model-selector",
      );
      expect(event).toBeDefined();
    });

    it("fillLastMessage: 应派发 axagent:fill-last-message 事件", async () => {
      const spy = setupDispatchSpy();

      await executeShortcutAction("fillLastMessage");

      const event = spy.mock.calls.find(
        (c) => (c[0] as CustomEvent).type === "axagent:fill-last-message",
      );
      expect(event).toBeDefined();
    });

    it("clearContext: 应派发 axagent:clear-context 事件", async () => {
      const spy = setupDispatchSpy();

      await executeShortcutAction("clearContext");

      const event = spy.mock.calls.find(
        (c) => (c[0] as CustomEvent).type === "axagent:clear-context",
      );
      expect(event).toBeDefined();
    });

    it("clearConversationMessages: 应派发 axagent:clear-conversation-messages 事件", async () => {
      const spy = setupDispatchSpy();

      await executeShortcutAction("clearConversationMessages");

      const event = spy.mock.calls.find(
        (c) => (c[0] as CustomEvent).type === "axagent:clear-conversation-messages",
      );
      expect(event).toBeDefined();
    });

    it("toggleMode: 应派发 axagent:toggle-mode 事件", async () => {
      const spy = setupDispatchSpy();

      await executeShortcutAction("toggleMode");

      const event = spy.mock.calls.find(
        (c) => (c[0] as CustomEvent).type === "axagent:toggle-mode",
      );
      expect(event).toBeDefined();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // openSettings — 页面导航
  // ═══════════════════════════════════════════════════════════════
  describe("openSettings", () => {
    it("当前不在设置页时应导航到 /settings", async () => {
      (window as any).location = { pathname: "/", href: "/" };

      await executeShortcutAction("openSettings");

      expect(window.location.href).toBe("/settings");
    });

    it("当前在设置页时应导航回 /", async () => {
      (window as any).location = { pathname: "/settings", href: "/settings" };

      await executeShortcutAction("openSettings");

      expect(window.location.href).toBe("/");
    });

    it("当前在设置子页面时应导航回 /", async () => {
      (window as any).location = { pathname: "/settings/providers", href: "/settings/providers" };

      await executeShortcutAction("openSettings");

      expect(window.location.href).toBe("/");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // toggleGateway — 网关控制
  // ═══════════════════════════════════════════════════════════════
  describe("toggleGateway", () => {
    it("网关运行中应调用 stop_gateway", async () => {
      mockInvoke.mockResolvedValueOnce({ is_running: true });

      await executeShortcutAction("toggleGateway");

      expect(mockInvoke).toHaveBeenCalledWith("get_gateway_status");
      expect(mockInvoke).toHaveBeenCalledWith("stop_gateway");
      expect(mockInvoke).not.toHaveBeenCalledWith("start_gateway");
    });

    it("网关未运行时应调用 start_gateway", async () => {
      mockInvoke.mockResolvedValueOnce({ is_running: false });

      await executeShortcutAction("toggleGateway");

      expect(mockInvoke).toHaveBeenCalledWith("get_gateway_status");
      expect(mockInvoke).toHaveBeenCalledWith("start_gateway");
      expect(mockInvoke).not.toHaveBeenCalledWith("stop_gateway");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // showQuickBar — 快速栏
  // ═══════════════════════════════════════════════════════════════
  describe("showQuickBar", () => {
    it("应调用 invoke('show_quickbar')", async () => {
      await executeShortcutAction("showQuickBar");

      expect(mockInvoke).toHaveBeenCalledWith("show_quickbar");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // 覆盖率：所有 12 个快捷键动作
  // ═══════════════════════════════════════════════════════════════
  describe("完整动作覆盖", () => {
    const allActions: ShortcutAction[] = [
      "toggleCurrentWindow",
      "toggleAllWindows",
      "closeWindow",
      "newConversation",
      "openSettings",
      "toggleModelSelector",
      "fillLastMessage",
      "clearContext",
      "clearConversationMessages",
      "toggleGateway",
      "toggleMode",
      "showQuickBar",
    ];

    it.each(allActions)("executeShortcutAction('%s') 不应抛出异常", async (action) => {
      mockIsTauri.mockReturnValue(true);
      mockGetCurrentWindow.isVisible.mockResolvedValue(false);

      // toggleGateway 特殊处理
      if (action === "toggleGateway") {
        mockInvoke.mockResolvedValue({ is_running: false });
      }

      await expect(executeShortcutAction(action)).resolves.toBeUndefined();
    });
  });
});
