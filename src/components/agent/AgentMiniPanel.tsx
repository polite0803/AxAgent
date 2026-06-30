// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import { Bot, Expand, Plus, Ellipsis } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { createPortal } from "react-dom";

/** 迷你面板尺寸 */
const MINI_WIDTH = 380;
const MINI_HEIGHT = 500;

/** 浮动按钮到屏幕边缘的间距 */
const BUTTON_MARGIN = 20;

/**
 * Agent 迷你面板 — 升级版 BuddyWidget
 *
 * - 右下角浮动（fixed 定位），可拖拽移动
 * - 未打开时显示浮动圆形按钮，点击展开迷你面板
 * - 显示迷你对话输入框 + 最近消息预览
 * - 底部工具栏：展开全尺寸面板按钮 + 新建对话 + 更多操作
 */
export function AgentMiniPanel() {
  const isOpen = useAgentPanelStore((s) => s.isOpen);
  const isMiniMode = useAgentPanelStore((s) => s.isMiniMode);
  const open = useAgentPanelStore((s) => s.open);
  const toggleMiniMode = useAgentPanelStore((s) => s.toggleMiniMode);
  void toggleMiniMode; // Reserved for Phase 1 mini-mode expand button

  // 迷你窗口的位置（可拖拽）
  const [position, setPosition] = useState({
    x: window.innerWidth - MINI_WIDTH - BUTTON_MARGIN,
    y: window.innerHeight - MINI_HEIGHT - 80,
  });
  const [isMiniExpanded, setIsMiniExpanded] = useState(false);

  // 拖拽状态
  const draggingRef = useRef(false);
  const dragOffsetRef = useRef({ x: 0, y: 0 });

  const handleHeaderMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    draggingRef.current = true;
    dragOffsetRef.current = {
      x: e.clientX - position.x,
      y: e.clientY - position.y,
    };

    const handleMouseMove = (ev: MouseEvent) => {
      if (!draggingRef.current) return;
      setPosition({
        x: Math.max(0, Math.min(window.innerWidth - MINI_WIDTH, ev.clientX - dragOffsetRef.current.x)),
        y: Math.max(0, Math.min(window.innerHeight - MINI_HEIGHT, ev.clientY - dragOffsetRef.current.y)),
      });
    };

    const handleMouseUp = () => {
      draggingRef.current = false;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [position]);

  // 如果全尺寸面板已经打开或不在迷你模式，只显示浮动按钮
  if (!isMiniMode || isOpen) {
    return null;
  }

  // 浮动圆形按钮（未展开迷你面板时）
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
        aria-label="打开 Agent 迷你面板"
      >
        <Bot size={22} />
      </button>,
      document.body,
    );
  }

  // 迷你面板展开
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
      {/* 可拖拽标题栏 */}
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
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round">
            <line x1={18} y1={6} x2={6} y2={18} />
            <line x1={6} y1={6} x2={18} y2={18} />
          </svg>
        </button>
      </div>

      {/* 内容区 — 占位 */}
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="text-center text-[var(--color-text-secondary)] text-sm">
          <Bot size={40} className="mx-auto mb-3 opacity-30" />
          <p>Agent 迷你面板</p>
          <p className="text-xs mt-1 opacity-70">对话功能将在后续版本中开放</p>
        </div>
      </div>

      {/* 底部工具栏 */}
      <div className="flex items-center justify-between px-3 py-2 border-t border-[var(--border-color)] bg-[var(--color-bg-container)] shrink-0">
        <button
          type="button"
          className="p-1.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
          title="新建对话"
        >
          <Plus size={16} />
        </button>
        <div className="flex items-center gap-1">
          <button
            type="button"
            className="p-1.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
            title="更多操作"
          >
            <Ellipsis size={16} />
          </button>
          <button
            type="button"
            className="p-1.5 rounded hover:bg-[var(--color-primary)]/10 text-[var(--color-primary)]"
            title="展开全尺寸面板"
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
