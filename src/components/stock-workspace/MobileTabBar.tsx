// SPDX-License-Identifier: AGPL-3.0-only

import type { WorkspaceView } from "@/stores";
import { useWorkspaceStore } from "@/stores";
import { BarChart3, Briefcase, Coins, MoreHorizontal, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";

interface TabItem {
  key: WorkspaceView;
  labelKey: string;
  icon: React.ReactNode;
}

/** 移动端底部 Tab Bar：4 核心 + 更多 */
export function MobileTabBar() {
  const { t } = useTranslation();
  const currentView = useWorkspaceStore((s) => s.currentView);
  const setCurrentView = useWorkspaceStore((s) => s.setCurrentView);

  const tabs: TabItem[] = [
    { key: "analysis", labelKey: "workspace.view.analysis", icon: <BarChart3 size={20} /> },
    { key: "monitor", labelKey: "workspace.view.monitor", icon: <Briefcase size={20} /> },
    { key: "trade", labelKey: "workspace.view.trade", icon: <Coins size={20} /> },
    { key: "review", labelKey: "workspace.view.review", icon: <RotateCcw size={20} /> },
    { key: "more", labelKey: "workspace.view.more", icon: <MoreHorizontal size={20} /> },
  ];

  return (
    <div
      className="flex items-stretch"
      style={{
        borderTop: "1px solid var(--border)",
        background: "var(--surface)",
        height: 64,
        flexShrink: 0,
      }}
    >
      {tabs.map((tab) => {
        // "more" tab 在 backtest/compare 视图时也高亮（用户从更多菜单进入）
        const isActive = tab.key === "more"
          ? currentView === "more" || currentView === "backtest" || currentView === "compare"
          : currentView === tab.key;
        return (
          <button
            key={tab.key}
            type="button"
            onClick={() => setCurrentView(tab.key)}
            className="flex-1 flex flex-col items-center justify-center gap-0.5 transition-colors"
            style={{
              color: isActive ? "var(--accent)" : "var(--muted)",
            }}
          >
            {tab.icon}
            <span className="text-sm" style={{ fontSize: 11 }}>
              {t(tab.labelKey)}
            </span>
          </button>
        );
      })}
    </div>
  );
}
