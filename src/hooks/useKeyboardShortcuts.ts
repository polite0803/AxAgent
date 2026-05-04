import { executeShortcutAction } from "@/lib/shortcutActions";
import { getShortcutBinding, matchesShortcutEvent, SHORTCUT_ACTIONS, type ShortcutAction } from "@/lib/shortcuts";
import { useConversationStore, useSettingsStore, useTabStore } from "@/stores";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

export function useKeyboardShortcuts() {
  const { t: _t } = useTranslation();
  const navigate = useNavigate();
  const settings = useSettingsStore((s) => s.settings);

  const handleKeyDown = useCallback(
    async (e: KeyboardEvent) => {
      // 在输入框或工作流画布中不触发全局快捷键，避免与 useWorkflowShortcuts 冲突
      const isInputField = e.target instanceof HTMLInputElement
        || e.target instanceof HTMLTextAreaElement
        || e.target instanceof HTMLSelectElement;
      const isWorkflowCanvas = (e.target as HTMLElement)?.closest?.(".react-flow") != null;
      if (isInputField || isWorkflowCanvas) { return; }

      // ── Tab navigation shortcuts (Ctrl+Tab / Ctrl+Shift+Tab) ──
      const isMod = e.metaKey || e.ctrlKey;
      if (isMod && e.key === "Tab") {
        e.preventDefault();
        const { tabs, activeTabId, setActiveTab } = useTabStore.getState();
        if (tabs.length <= 1) { return; }
        const currentIdx = tabs.findIndex((t) => t.id === activeTabId);
        if (currentIdx === -1) { return; }
        const direction = e.shiftKey ? -1 : 1;
        const nextIdx = (currentIdx + direction + tabs.length) % tabs.length;
        setActiveTab(tabs[nextIdx].id);
        return;
      }

      for (const action of SHORTCUT_ACTIONS) {
        const binding = getShortcutBinding(settings, action);
        if (!binding) { continue; }
        if (!matchesShortcutEvent(e, binding)) { continue; }

        console.info("[shortcut-local-hit]", {
          action,
          binding,
          key: e.key,
          metaKey: e.metaKey,
          ctrlKey: e.ctrlKey,
          shiftKey: e.shiftKey,
          altKey: e.altKey,
        });
        e.preventDefault();
        await executeShortcutAction(action as ShortcutAction);
        return;
      }

      if (!isMod) { return; }

      switch (e.key.toLowerCase()) {
        case "f":
          e.preventDefault();
          navigate("/");
          setTimeout(() => {
            const searchInput = document.querySelector<HTMLInputElement>(".chat-sidebar-search input");
            searchInput?.focus();
          }, 50);
          return;
        case "w":
          e.preventDefault();
          // Close the active tab instead of just clearing the conversation
          {
            const { activeTabId, closeTab } = useTabStore.getState();
            if (activeTabId) {
              closeTab(activeTabId);
            } else {
              useConversationStore.getState().setActiveConversation(null);
            }
          }
          return;
        default:
          return;
      }
    },
    [navigate, settings],
  );

  const handleKeyDownEsc = useCallback((e: KeyboardEvent) => {
    if (e.key === "Escape") {
      if (window.location.pathname === "/settings" || window.location.pathname.startsWith("/settings/")) {
        navigate("/");
        return;
      }
      window.dispatchEvent(new CustomEvent("axagent:escape"));
    }
  }, [navigate]);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keydown", handleKeyDownEsc);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keydown", handleKeyDownEsc);
    };
  }, [handleKeyDown, handleKeyDownEsc]);
}
