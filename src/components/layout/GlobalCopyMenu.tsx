import { useState, useEffect, useLayoutEffect, useCallback, useRef } from 'react';
import { Copy, TextCursorInput, Bug, Scissors, ClipboardPaste, BoxSelect } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { theme, message } from 'antd';
import { invoke } from '@/lib/invoke';
import { useConversationStore } from '@/stores';

/**
 * Global right-click context menu.
 * - On textarea/input → Cut + Copy + Paste + Select All + (DevTools in dev)
 * - Text selected → Copy + (Fill to Input in chat) + (DevTools in dev)
 * - No text, dev mode → DevTools only
 * - No text, prod mode → suppress native menu
 * - Skips when a component-specific context menu already handled the event.
 */
export function GlobalCopyMenu() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [menuPos, setMenuPos] = useState<{ x: number; y: number } | null>(null);
  const [hasSelection, setHasSelection] = useState(false);
  const [inChatMessages, setInChatMessages] = useState(false);
  const [isTextInput, setIsTextInput] = useState(false);
  const selectedTextRef = useRef('');
  const menuRef = useRef<HTMLDivElement>(null);
  const targetInputRef = useRef<HTMLTextAreaElement | HTMLInputElement | null>(null);
  const selectionRangeRef = useRef<{ start: number; end: number }>({ start: 0, end: 0 });
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const isDev = import.meta.env.DEV;

  useEffect(() => {
    // Save textarea selection before Chromium's auto word-selection on right-click
    const savedSelRef = { start: -1, end: -1, el: null as HTMLTextAreaElement | HTMLInputElement | null };

    const saveSelectionOnRightClick = (e: MouseEvent) => {
      if (e.button !== 2) return;
      const t = e.target as HTMLElement;
      if (t.tagName === 'TEXTAREA' || (t.tagName === 'INPUT' && (t as HTMLInputElement).type === 'text')) {
        const input = t as HTMLTextAreaElement | HTMLInputElement;
        savedSelRef.start = input.selectionStart ?? 0;
        savedSelRef.end = input.selectionEnd ?? 0;
        savedSelRef.el = input;
      } else {
        savedSelRef.el = null;
      }
    };

    const handler = (e: MouseEvent) => {
      if (e.defaultPrevented) return;
      if (menuRef.current?.contains(e.target as Node)) return;
      e.preventDefault();

      const targetEl = e.target as HTMLElement;
      const isInput = targetEl.tagName === 'TEXTAREA' || (targetEl.tagName === 'INPUT' && (targetEl as HTMLInputElement).type === 'text');

      if (isInput) {
        const inputEl = targetEl as HTMLTextAreaElement | HTMLInputElement;
        targetInputRef.current = inputEl;

        // Use saved selection (before auto word-select) if available
        const start = savedSelRef.el === inputEl ? savedSelRef.start : (inputEl.selectionStart ?? 0);
        const end = savedSelRef.el === inputEl ? savedSelRef.end : (inputEl.selectionEnd ?? 0);
        selectionRangeRef.current = { start, end };

        // Restore original selection to undo Chromium's auto word-select (deferred)
        inputEl.setSelectionRange(start, end);
        requestAnimationFrame(() => {
          inputEl.setSelectionRange(start, end);
        });

        const sel = inputEl.value.substring(start, end);
        selectedTextRef.current = sel;
        setHasSelection(!!sel);
        setIsTextInput(true);
        setInChatMessages(false);
        setMenuPos({ x: e.clientX, y: e.clientY });
        return;
      }

      setIsTextInput(false);
      targetInputRef.current = null;

      const sel = window.getSelection()?.toString().trim() || '';
      selectedTextRef.current = sel;

      // "Fill to input" only when right-clicking inside chat message area
      const inMessageArea = !!(e.target as HTMLElement).closest?.('[data-message-area]');
      setInChatMessages(inMessageArea);

      if (sel) {
        setHasSelection(true);
        setMenuPos({ x: e.clientX, y: e.clientY });
      } else if (isDev) {
        setHasSelection(false);
        setMenuPos({ x: e.clientX, y: e.clientY });
      } else {
        setMenuPos(null);
      }
    };

    const dismissHandler = () => setMenuPos(null);

    document.addEventListener('mousedown', saveSelectionOnRightClick, true);
    document.addEventListener('contextmenu', handler);
    document.addEventListener('click', dismissHandler);
    document.addEventListener('scroll', dismissHandler, true);
    return () => {
      document.removeEventListener('mousedown', saveSelectionOnRightClick, true);
      document.removeEventListener('contextmenu', handler);
      document.removeEventListener('click', dismissHandler);
      document.removeEventListener('scroll', dismissHandler, true);
    };
  }, [isDev]);

  const handleCopy = useCallback(() => {
    const text = selectedTextRef.current;
    if (text) {
      void navigator.clipboard.writeText(text);
      message.success(t('common.copySuccess'));
    }
    setMenuPos(null);
  }, [t]);

  const handleCut = useCallback(() => {
    const input = targetInputRef.current;
    if (!input) { setMenuPos(null); return; }
    const { start, end } = selectionRangeRef.current;
    const selectedText = input.value.substring(start, end);
    if (selectedText) {
      void navigator.clipboard.writeText(selectedText);
      input.focus();
      input.setSelectionRange(start, end);
      document.execCommand('delete');
    }
    setMenuPos(null);
  }, []);

  const handlePaste = useCallback(async () => {
    const input = targetInputRef.current;
    if (!input) { setMenuPos(null); return; }
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        input.focus();
        const { start, end } = selectionRangeRef.current;
        input.setSelectionRange(start, end);
        document.execCommand('insertText', false, text);
      }
    } catch (err) {
      console.error('Paste failed:', err);
    }
    setMenuPos(null);
  }, []);

  const handleSelectAll = useCallback(() => {
    const input = targetInputRef.current;
    if (input) {
      input.focus();
      input.select();
    }
    setMenuPos(null);
  }, []);

  const handleFillInput = useCallback(() => {
    const text = selectedTextRef.current;
    if (text) {
      window.dispatchEvent(new CustomEvent('axagent:fill-input', { detail: text }));
    }
    setMenuPos(null);
  }, []);

  const handleOpenDevtools = useCallback(() => {
    void invoke('open_devtools');
    setMenuPos(null);
  }, []);

  // Clamp menu position to stay within viewport
  useLayoutEffect(() => {
    if (!menuRef.current || !menuPos) return;
    const el = menuRef.current;
    const rect = el.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    let x = menuPos.x;
    let y = menuPos.y;

    if (x + rect.width > vw) x = vw - rect.width - 4;
    if (y + rect.height > vh) y = vh - rect.height - 4;
    if (x < 4) x = 4;
    if (y < 4) y = 4;

    el.style.left = x + 'px';
    el.style.top = y + 'px';
  }, [menuPos, isTextInput, hasSelection]);

  if (!menuPos) return null;

  interface MenuItem {
    key: string;
    icon: React.ReactNode;
    label: string;
    onClick: () => void;
    disabled?: boolean;
    divider?: boolean;
  }

  const items: MenuItem[] = [];

  if (isTextInput) {
    items.push(
      { key: 'cut', icon: <Scissors size={14} />, label: t('common.cut'), onClick: handleCut, disabled: !hasSelection },
      { key: 'copy', icon: <Copy size={14} />, label: t('common.copy'), onClick: handleCopy, disabled: !hasSelection },
      { key: 'paste', icon: <ClipboardPaste size={14} />, label: t('common.paste'), onClick: handlePaste },
      { key: 'selectAll', icon: <BoxSelect size={14} />, label: t('common.selectAll'), onClick: handleSelectAll, divider: true },
    );
  } else {
    if (hasSelection) {
      items.push(
        { key: 'copy', icon: <Copy size={14} />, label: t('common.copy'), onClick: handleCopy },
      );
      if (activeConversationId && inChatMessages) {
        items.push({
          key: 'fill',
          icon: <TextCursorInput size={14} />,
          label: t('common.fillToInput'),
          onClick: handleFillInput,
        });
      }
    }
  }

  if (isDev) {
    items.push({
      key: 'devtools',
      icon: <Bug size={14} />,
      label: t('common.openDevtools'),
      onClick: handleOpenDevtools,
    });
  }

  return (
    <div
      ref={menuRef}
      style={{
        position: 'fixed',
        left: menuPos.x,
        top: menuPos.y,
        zIndex: 9999,
        backgroundColor: token.colorBgElevated,
        borderRadius: 8,
        boxShadow: token.boxShadowSecondary,
        padding: '4px',
        minWidth: 120,
      }}
    >
      {items.map((item) => (
        <div key={item.key}>
          {item.divider && (
            <div style={{ height: 1, backgroundColor: token.colorBorderSecondary, margin: '4px 8px' }} />
          )}
          <div
            className="flex items-center gap-2"
            style={{
              padding: '6px 12px',
              borderRadius: 4,
              fontSize: 13,
              color: item.disabled ? token.colorTextDisabled : token.colorText,
              cursor: item.disabled ? 'default' : 'pointer',
              transition: 'background-color 0.15s',
            }}
            onClick={item.disabled ? undefined : item.onClick}
            onMouseEnter={(e) => { if (!item.disabled) (e.currentTarget as HTMLElement).style.backgroundColor = token.colorFillSecondary; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.backgroundColor = 'transparent'; }}
          >
            {item.icon}
            <span>{item.label}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
