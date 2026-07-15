// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkspaceStore } from "@/stores";
import { ChevronRight, PanelRight } from "lucide-react";
import { useTranslation } from "react-i18next";

/**
 * 右栏：上下文侧栏（阶段 1 占位）。
 *
 * 阶段 4 会迁入 K线/行情/新闻/资金/板块等面板。
 * 当前为占位组件，只显示折叠/展开按钮。
 */
export function ContextSidebar() {
  const { t } = useTranslation();
  const collapsed = useWorkspaceStore((s) => s.rightSidebarCollapsed);
  const toggle = useWorkspaceStore((s) => s.toggleRightSidebar);

  if (collapsed) {
    return (
      <button
        type="button"
        onClick={toggle}
        className="flex items-center justify-center p-1.5 rounded hover:opacity-70"
        style={{
          width: 36,
          borderLeft: "1px solid var(--border)",
          background: "var(--surface)",
          flexShrink: 0,
        }}
        title={t("workspace.contextSidebar.expand")}
      >
        <PanelRight size={16} />
      </button>
    );
  }

  return (
    <div
      className="flex flex-col"
      style={{
        width: 320,
        borderLeft: "1px solid var(--border)",
        background: "var(--surface)",
        flexShrink: 0,
      }}
    >
      {/* 标题栏 */}
      <div
        className="flex items-center justify-between px-2 py-1.5"
        style={{ borderBottom: "1px solid var(--border)" }}
      >
        <span className="text-sm font-semibold">{t("workspace.contextSidebar.title")}</span>
        <button
          type="button"
          onClick={toggle}
          className="p-1 rounded hover:opacity-70"
          title={t("workspace.contextSidebar.collapse")}
        >
          <ChevronRight size={14} />
        </button>
      </div>

      {/* 占位内容 */}
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="text-sm text-center" style={{ color: "var(--muted)" }}>
          {t("workspace.contextSidebar.placeholder")}
        </div>
      </div>
    </div>
  );
}
