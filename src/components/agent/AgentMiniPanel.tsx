// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import { Bot, Ellipsis, Expand, Plus } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

const MINI_WIDTH = 380;
const MINI_HEIGHT = 500;
const BUTTON_MARGIN = 20;
const STORAGE_KEY_POSITION = "axagent:agentMiniPanel:position";

function loadPersistedPosition(): { x: number; y: number } {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_POSITION);
    if (raw) {
      const parsed = JSON.parse(raw) as { x: number; y: number };
      if (typeof parsed.x === "number" && typeof parsed.y === "number") {
        return {
          x: Math.max(0, Math.min(window.innerWidth - MINI_WIDTH, parsed.x)),
          y: Math.max(0, Math.min(window.innerHeight - MINI_HEIGHT, parsed.y)),
        };
      }
    }
  } catch {
    // 忽略
  }
  return {
    x: window.innerWidth - MINI_WIDTH - BUTTON_MARGIN,
    y: window.innerHeight - MINI_HEIGHT - 80,
  };
}

function persistPosition(x: number, y: number): void {
  try {
    localStorage.setItem(STORAGE_KEY_POSITION, JSON.stringify({ x, y }));
  } catch {
    // 忽略
  }
}

/**
 * Agent 迷你面板
 *
 * - 右下角浮动（fixed 定位），可拖拽移动，位置持久化
 * - 未打开时显示浮动圆形按钮，点击展开迷你面板
 * - 响应窗口 resize，自动修正位置避免跑出屏幕
 * - 显示迷你对话输入框 + 最近消息预览
 * - 底部工具栏：展开全尺寸面板按钮 + 新建对话 + 更多操作
 */
export function AgentMiniPanel() {
  const { t } = useTranslation();
  const isOpen = useAgentPanelStore((s) => s.isOpen);
  const isMiniMode = useAgentPanelStore((s) => s.isMiniMode);
  const open = useAgentPanelStore((s) => s.open);

  const [position, setPosition] = useState(loadPersistedPosition);
  const [isMiniExpanded, setIsMiniExpanded] = useState(false);

  const draggingRef = useRef(false);
  const dragOffsetRef = useRef({ x: 0, y: 0 });

  const clampPosition = useCallback((x: number, y: number) => {
    return {
      x: Math.max(0, Math.min(window.innerWidth - MINI_WIDTH, x)),
      y: Math.max(0, Math.min(window.innerHeight - MINI_HEIGHT, y)),
    };
  }, []);

  useEffect(() => {
    const handleResize = () => {
      setPosition((prev) => clampPosition(prev.x, prev.y));
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [clampPosition]);

  const handleHeaderMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    draggingRef.current = true;
    dragOffsetRef.current = {
      x: e.clientX - position.x,
      y: e.clientY - position.y,
    };

    const handleMouseMove = (ev: MouseEvent) => {
      if (!draggingRef.current) { return; }
      const next = clampPosition(
        ev.clientX - dragOffsetRef.current.x,
        ev.clientY - dragOffsetRef.current.y,
      );
      setPosition(next);
    };

    const handleMouseUp = () => {
      draggingRef.current = false;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      persistPosition(position.x, position.y);
    };

    document.body.style.cursor = "move";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [position, clampPosition]);

  if (!isMiniMode || isOpen) {
    return null;
  }

  if (!isMiniExpanded) {
    return createPortal(
      <button
        type="button"
        className="fixed z-[999] w-12 h-12 rounded-full bg-[var(--color-primary)] text-white shadow-lg flex items-center justify-center hover:scale-110 transition-transform cursor-pointer"
        style={{
          right: BUTTON_MARGIN,
          bottom: 80,
        }}
        onClick={() => setIsMiniExpanded(true)}
        aria-label={t("agentPanel.openMiniPanel")}
      >
        <Bot size={22} />
      </button>,
      document.body,
    );
  }

  return createPortal(
    <div
      className="fixed z-[998] bg-[var(--color-bg-elevated)] rounded-lg shadow-2xl border border-[var(--border-color)] flex flex-col overflow-hidden"
      style={{
        left: position.x,
        top: position.y,
        width: MINI_WIDTH,
        height: MINI_HEIGHT,
      }}
    >
      <div
        className="flex items-center justify-between px-3 py-2 bg-[var(--color-bg-container)] border-b border-[var(--border-color)] cursor-move shrink-0"
        onMouseDown={handleHeaderMouseDown}
      >
        <div className="flex items-center gap-2">
          <Bot size={16} className="text-[var(--color-primary)]" />
          <span className="text-sm font-medium">AxAgent</span>
        </div>
        <button
          type="button"
          className="p-0.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
          onClick={() => setIsMiniExpanded(false)}
          aria-label={t("common.close")}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
            strokeLinecap="round"
          >
            <line x1={18} y1={6} x2={6} y2={18} />
            <line x1={6} y1={6} x2={18} y2={18} />
          </svg>
        </button>
      </div>

      <div className="flex-1 flex items-center justify-center p-4">
        <div className="text-center text-[var(--color-text-secondary)] text-sm">
          <Bot size={40} className="mx-auto mb-3 opacity-30" />
          <p>{t("agentPanel.miniPanelTitle")}</p>
          <p className="text-xs mt-1 opacity-70">{t("agentPanel.miniPanelComingSoon")}</p>
        </div>
      </div>

      <div className="flex items-center justify-between px-3 py-2 border-t border-[var(--border-color)] bg-[var(--color-bg-container)] shrink-0">
        <button
          type="button"
          className="p-1.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
          title={t("agentPanel.newChat")}
        >
          <Plus size={16} />
        </button>
        <div className="flex items-center gap-1">
          <button
            type="button"
            className="p-1.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
            title={t("common.more")}
          >
            <Ellipsis size={16} />
          </button>
          <button
            type="button"
            className="p-1.5 rounded hover:bg-[var(--color-primary)]/10 text-[var(--color-primary)]"
            title={t("agentPanel.expandFullPanel")}
            onClick={() => {
              setIsMiniExpanded(false);
              open();
            }}
          >
            <Expand size={16} />
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
