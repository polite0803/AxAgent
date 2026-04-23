import { useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useConversationStore, useSettingsStore, useTabStore } from '@/stores';
import {
  SHORTCUT_ACTIONS,
  getShortcutBinding,
  matchesShortcutEvent,
  type ShortcutAction,
} from '@/lib/shortcuts';
import { executeShortcutAction } from '@/lib/shortcutActions';

export function useKeyboardShortcuts() {
  const { t: _t } = useTranslation();
  const navigate = useNavigate();
  const settings = useSettingsStore((s) => s.settings);

  const handleKeyDown = useCallback(
    async (e: KeyboardEvent) => {
      // ── Tab navigation shortcuts (Ctrl+Tab / Ctrl+Shift+Tab) ──
      const isMod = e.metaKey || e.ctrlKey;
      if (isMod && e.key === 'Tab') {
        e.preventDefault();
        const { tabs, activeTabId, setActiveTab } = useTabStore.getState();
        if (tabs.length <= 1) return;
        const currentIdx = tabs.findIndex((t) => t.id === activeTabId);
        if (currentIdx === -1) return;
        const direction = e.shiftKey ? -1 : 1;
        const nextIdx = (currentIdx + direction + tabs.length) % tabs.length;
        setActiveTab(tabs[nextIdx].id);
        return;
      }

      for (const action of SHORTCUT_ACTIONS) {
        const binding = getShortcutBinding(settings, action);
        if (!binding) continue;
        if (!matchesShortcutEvent(e, binding)) continue;

        console.info('[shortcut-local-hit]', {
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

      if (!isMod) return;

      switch (e.key.toLowerCase()) {
        case 'f':
          e.preventDefault();
          navigate('/');
          setTimeout(() => {
            const searchInput = document.querySelector<HTMLInputElement>('.chat-sidebar-search input');
            searchInput?.focus();
          }, 50);
          return;
        case 'w':
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
    if (e.key === 'Escape') {
      if (window.location.pathname === '/settings' || window.location.pathname.startsWith('/settings/')) {
        navigate('/');
        return;
      }
      window.dispatchEvent(new CustomEvent('axagent:escape'));
    }
  }, [navigate]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keydown', handleKeyDownEsc);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keydown', handleKeyDownEsc);
    };
  }, [handleKeyDown, handleKeyDownEsc]);
}
