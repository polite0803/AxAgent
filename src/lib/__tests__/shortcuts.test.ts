import { describe, expect, it } from "vitest";

import type { AppSettings } from "@/types";
import {
  DEFAULT_SHORTCUT_BINDINGS,
  SHORTCUT_ACTIONS,
  detectShortcutConflicts,
  findExternalConflict,
  formatShortcutForDisplay,
  getShortcutBinding,
  isGlobalShortcutAction,
  matchesShortcutEvent,
  normalizeShortcutFromKeyboardEvent,
  toTauriAccelerator,
} from "../shortcuts";

// Minimal mock of AppSettings for tests — only fields actually accessed by shortcut functions
const mockSettings = (
  overrides: Partial<Record<string, string>> = {},
): AppSettings =>
  ({
    // Required shortcut fields (snake_case, match SHORTCUT_SETTING_KEYS)
    shortcut_toggle_current_window: "",
    shortcut_toggle_all_windows: "",
    shortcut_close_window: "",
    shortcut_new_conversation: "",
    shortcut_open_settings: "",
    shortcut_toggle_model_selector: "",
    shortcut_fill_last_message: "",
    shortcut_clear_context: "",
    shortcut_clear_conversation_messages: "",
    shortcut_toggle_gateway: "",
    shortcut_toggle_mode: "",
    shortcut_show_quick_bar: "",
    global_shortcut: "",
    ...overrides,
    // Stub required AppSettings fields so TS cast succeeds
    language: "zh-CN",
    theme_mode: "dark",
    theme_preset: "dark-elegance",
    primary_color: "#17A93D",
    border_radius: 8,
    auto_start: false,
    show_on_start: true,
    minimize_to_tray: true,
    font_size: 14,
    font_weight: 400,
    font_family: "",
    code_font_family: "",
    bubble_style: "minimal",
    code_theme: "poimandres",
    code_theme_light: "github-light",
    default_provider_id: null,
    default_model_id: null,
    default_temperature: null,
    default_max_tokens: null,
    default_top_p: null,
    default_frequency_penalty: null,
    default_context_count: null,
    title_summary_provider_id: null,
    title_summary_model_id: null,
    title_summary_temperature: null,
    title_summary_max_tokens: null,
    title_summary_top_p: null,
    title_summary_frequency_penalty: null,
    title_summary_context_count: null,
    title_summary_prompt: null,
    compression_provider_id: null,
    compression_model_id: null,
    compression_temperature: null,
    compression_max_tokens: null,
    compression_top_p: null,
    compression_frequency_penalty: null,
    compression_prompt: null,
    proxy_type: null,
    proxy_address: null,
    proxy_port: null,
    gateway_auto_start: false,
    gateway_listen_address: "127.0.0.1",
    gateway_port: 0,
    gateway_ssl_enabled: false,
    gateway_ssl_mode: "none",
    gateway_ssl_cert_path: null,
  }) as AppSettings;

describe("formatShortcutForDisplay", () => {
  it("formats CmdOrCtrl+Shift+A as display symbols", () => {
    const result = formatShortcutForDisplay("CmdOrCtrl+Shift+A");
    expect(result).toContain("⇧");
    expect(result).toContain("A");
  });

  it("formats CmdOrCtrl+N", () => {
    const result = formatShortcutForDisplay("CmdOrCtrl+N");
    expect(result).toContain("N");
  });

  it("formats arrow keys with display symbols", () => {
    const result = formatShortcutForDisplay("CmdOrCtrl+Shift+ArrowUp");
    expect(result).toContain("⇧");
    expect(result).toContain("↑");
  });

  it("formats function-style names like Escape", () => {
    const result = formatShortcutForDisplay("Escape");
    expect(result).toBe("Esc");
  });

  it("formats Space key", () => {
    const result = formatShortcutForDisplay("CmdOrCtrl+Shift+Space");
    expect(result).toContain("␣");
  });
});

describe("normalizeShortcutFromKeyboardEvent", () => {
  it("returns null for modifier-only events", () => {
    const event = {
      metaKey: true,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      key: "Meta",
    };
    expect(normalizeShortcutFromKeyboardEvent(event)).toBeNull();
  });

  it("normalizes cmd+shift+K", () => {
    const event = {
      metaKey: true,
      ctrlKey: false,
      shiftKey: true,
      altKey: false,
      key: "k",
    };
    expect(normalizeShortcutFromKeyboardEvent(event)).toBe("CmdOrCtrl+Shift+K");
  });

  it("normalizes ctrl+N (no meta)", () => {
    const event = {
      metaKey: false,
      ctrlKey: true,
      shiftKey: false,
      altKey: false,
      key: "n",
    };
    expect(normalizeShortcutFromKeyboardEvent(event)).toBe("CmdOrCtrl+N");
  });

  it("normalizes with Alt key", () => {
    const event = {
      metaKey: true,
      ctrlKey: false,
      shiftKey: false,
      altKey: true,
      key: "A",
    };
    expect(normalizeShortcutFromKeyboardEvent(event)).toBe("CmdOrCtrl+Alt+A");
  });

  it("returns null for Command key alone", () => {
    const event = {
      metaKey: true,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      key: "Command",
    };
    expect(normalizeShortcutFromKeyboardEvent(event)).toBeNull();
  });
});

describe("toTauriAccelerator", () => {
  it("converts CmdOrCtrl to CommandOrControl", () => {
    expect(toTauriAccelerator("CmdOrCtrl+Shift+A")).toBe("CommandOrControl+Shift+A");
  });

  it("converts comma shorthand", () => {
    expect(toTauriAccelerator("CmdOrCtrl+,")).toBe("CommandOrControl+Comma");
  });

  it("converts period shorthand", () => {
    expect(toTauriAccelerator("CmdOrCtrl+.")).toBe("CommandOrControl+Period");
  });

  it("converts slash shorthand", () => {
    expect(toTauriAccelerator("Shift+/")).toBe("Shift+Slash");
  });

  it("handles simple key without modifiers", () => {
    expect(toTauriAccelerator("Escape")).toBe("Escape");
  });
});

describe("matchesShortcutEvent", () => {
  it("matches cmd+shift+K event", () => {
    const event = new KeyboardEvent("keydown", {
      key: "k",
      metaKey: true,
      shiftKey: true,
      bubbles: true,
    });
    expect(matchesShortcutEvent(event, "CmdOrCtrl+Shift+K")).toBe(true);
  });

  it("does not match when shift is missing", () => {
    const event = new KeyboardEvent("keydown", {
      key: "k",
      metaKey: true,
      shiftKey: false,
    });
    expect(matchesShortcutEvent(event, "CmdOrCtrl+Shift+K")).toBe(false);
  });

  it("does not match when CmdOrCtrl is missing", () => {
    const event = new KeyboardEvent("keydown", {
      key: "k",
      metaKey: false,
      ctrlKey: false,
      shiftKey: true,
    });
    expect(matchesShortcutEvent(event, "CmdOrCtrl+Shift+K")).toBe(false);
  });

  it("returns false for empty binding", () => {
    const event = new KeyboardEvent("keydown", { key: "a" });
    expect(matchesShortcutEvent(event, "")).toBe(false);
  });

  it("matches Escape key", () => {
    const event = new KeyboardEvent("keydown", { key: "Escape" });
    expect(matchesShortcutEvent(event, "Escape")).toBe(true);
  });

  it("matches Tab key", () => {
    const event = new KeyboardEvent("keydown", { key: "Tab", shiftKey: true });
    expect(matchesShortcutEvent(event, "Shift+Tab")).toBe(true);
  });
});

describe("detectShortcutConflicts", () => {
  it("detects no conflicts when bindings are unique", () => {
    const conflicts = detectShortcutConflicts({
      toggleCurrentWindow: "CmdOrCtrl+Shift+A",
      newConversation: "CmdOrCtrl+N",
    });
    expect(Object.keys(conflicts)).toHaveLength(0);
  });

  it("detects conflicts when two actions share the same binding", () => {
    const conflicts = detectShortcutConflicts({
      newConversation: "CmdOrCtrl+N",
      openSettings: "CmdOrCtrl+N",
    });
    expect(conflicts.newConversation).toEqual(["openSettings"]);
    expect(conflicts.openSettings).toEqual(["newConversation"]);
  });

  it("ignores empty bindings in conflict detection", () => {
    const conflicts = detectShortcutConflicts({
      newConversation: "CmdOrCtrl+N",
      openSettings: "",
    });
    expect(Object.keys(conflicts)).toHaveLength(0);
  });
});

describe("findExternalConflict", () => {
  it("detects known external conflict for CmdOrCtrl+Shift+A", () => {
    const result = findExternalConflict("CommandOrControl+Shift+A");
    expect(result).toBeDefined();
    expect(result).toContain("飞书");
  });

  it("detects known external conflict for Ctrl+Alt+A", () => {
    const result = findExternalConflict("Control+Alt+A");
    expect(result).toBeDefined();
    expect(result).toContain("QQ");
  });

  it("returns undefined for non-conflicting shortcut", () => {
    const result = findExternalConflict("CommandOrControl+Shift+X");
    expect(result).toBeUndefined();
  });
});

describe("getShortcutBinding", () => {
  it("returns default binding when setting is empty", () => {
    const settings = mockSettings();
    const binding = getShortcutBinding(settings, "newConversation");
    expect(binding).toBe(DEFAULT_SHORTCUT_BINDINGS.newConversation);
  });

  it("returns custom binding when setting is provided", () => {
    const settings = mockSettings({
      shortcut_new_conversation: "CmdOrCtrl+M",
    });
    const binding = getShortcutBinding(settings, "newConversation");
    expect(binding).toBe("CmdOrCtrl+M");
  });
});

describe("isGlobalShortcutAction", () => {
  it("returns true for toggleCurrentWindow", () => {
    expect(isGlobalShortcutAction("toggleCurrentWindow")).toBe(true);
  });

  it("returns true for toggleAllWindows", () => {
    expect(isGlobalShortcutAction("toggleAllWindows")).toBe(true);
  });

  it("returns false for newConversation", () => {
    expect(isGlobalShortcutAction("newConversation")).toBe(false);
  });

  it("returns false for openSettings", () => {
    expect(isGlobalShortcutAction("openSettings")).toBe(false);
  });
});

describe("SHORTCUT_ACTIONS", () => {
  it("contains all expected actions", () => {
    expect(SHORTCUT_ACTIONS).toContain("toggleCurrentWindow");
    expect(SHORTCUT_ACTIONS).toContain("newConversation");
    expect(SHORTCUT_ACTIONS).toContain("openSettings");
    expect(SHORTCUT_ACTIONS).toContain("showQuickBar");
  });

  it("has exactly 12 actions", () => {
    expect(SHORTCUT_ACTIONS).toHaveLength(12);
  });
});
