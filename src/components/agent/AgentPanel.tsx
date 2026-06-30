// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-refresh/only-export-components */

import { AgentChatTab } from "@/components/agent/AgentChatTab";
import { AgentExecutionTab } from "@/components/agent/AgentExecutionTab";
import { AgentPanelHeader } from "@/components/agent/AgentPanelHeader";
import { AgentPanelTabs } from "@/components/agent/AgentPanelTabs";
import { AgentSkillTab } from "@/components/agent/AgentSkillTab";
import { AgentWfTab } from "@/components/agent/AgentWfTab";
import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { AgentPanelTab } from "@/stores/shared/agentPanelStore";
import { useCallback, useEffect, useRef } from "react";

/** 面板可拖拽最小宽度 */
const MIN_WIDTH = 320;
/** 面板可拖拽最大宽度 */
const MAX_WIDTH = 600;

/** 内容区容器样式：根据面板展开状态动态调整右侧 margin */
function getContentStyle(isOpen: boolean, panelWidth: number): React.CSSProperties {
  if (!isOpen) { return {}; }
  return { marginRight: panelWidth };
}

/**
 * 全局 Agent Panel 壳层组件
 *
 * - 位于 App 右侧，与 ContentArea 平级
 * - 可折叠（点击按钮或遮罩）
 * - 左侧边缘可拖拽调整宽度
 * - 展开/折叠动画 300ms ease-in-out
 * - 当 isOpen=false 时完全不渲染（display:none）
 */
export function AgentPanel() {
  const isOpen = useAgentPanelStore((s) => s.isOpen);
  const activeTab = useAgentPanelStore((s) => s.activeTab);
  const panelWidth = useAgentPanelStore((s) => s.panelWidth);
  const setWidth = useAgentPanelStore((s) => s.setWidth);
  const close = useAgentPanelStore((s) => s.close);

  const draggingRef = useRef(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  // 拖拽调整宽度
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      draggingRef.current = true;
      startXRef.current = e.clientX;
      startWidthRef.current = panelWidth;

      const handleMouseMove = (ev: MouseEvent) => {
        if (!draggingRef.current) { return; }
        const delta = startXRef.current - ev.clientX;
        const newWidth = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidthRef.current + delta));
        setWidth(newWidth);
      };

      const handleMouseUp = () => {
        draggingRef.current = false;
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      };

      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    },
    [panelWidth, setWidth],
  );

  // 按 Escape 关闭面板
  useEffect(() => {
    if (!isOpen) { return; }
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        close();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, close]);

  const contentMap: Record<AgentPanelTab, React.ReactNode> = {
    chat: <AgentChatTab />,
    execution: <AgentExecutionTab />,
    skill: <AgentSkillTab />,
    "nl-generation": <AgentWfTab />,
  };

  if (!isOpen) {
    return null;
  }

  return (
    <div
      className="relative shrink-0 border-l border-[var(--border-color)] bg-[var(--color-bg-container)] flex flex-col"
      style={{
        width: panelWidth,
        transition: "width 300ms ease-in-out",
      }}
    >
      {/* 左侧拖拽手柄 */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1.5 cursor-col-resize z-10 hover:bg-[var(--color-primary)]/20 transition-colors"
        onMouseDown={handleMouseDown}
      />

      {/* 标题栏 */}
      <AgentPanelHeader />

      {/* 标签页切换 */}
      <AgentPanelTabs />

      {/* 内容区 */}
      <div className="flex-1 overflow-auto">
        {contentMap[activeTab]}
      </div>
    </div>
  );
}

export { getContentStyle };
