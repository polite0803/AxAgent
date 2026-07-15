// SPDX-License-Identifier: AGPL-3.0-only

import { PortfolioDashboard } from "@/components/stock-analysis/PortfolioDashboard";
import { WatchlistPage } from "@/components/stock-analysis/WatchlistPage";
import { useWorkspaceStore } from "@/stores";
import { Briefcase, Star } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * 监控视图 — 工作区中栏的"监控"视图。
 *
 * 组合自选股管理 + 持仓 dashboard，通过子 Tab 切换。
 * 阶段 3：直接复用现有组件；阶段 4 会整合为统一 dashboard。
 */
export function MonitorView() {
  const { t } = useTranslation();
  const currentStockCode = useWorkspaceStore((s) => s.currentStockCode);
  const [subTab, setSubTab] = useState<"watchlist" | "portfolio">("watchlist");

  return (
    <div className="flex flex-col h-full">
      {/* 子 Tab 切换器 */}
      <div
        className="flex items-center gap-1 px-3 py-1.5"
        style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}
      >
        <button
          type="button"
          onClick={() => setSubTab("watchlist")}
          className="flex items-center gap-1 px-3 py-1 rounded text-sm transition-colors"
          style={{
            background: subTab === "watchlist" ? "var(--accent)" : "transparent",
            color: subTab === "watchlist" ? "white" : "var(--muted)",
          }}
        >
          <Star size={14} />
          {t("workspace.view.watchlist")}
        </button>
        <button
          type="button"
          onClick={() => setSubTab("portfolio")}
          className="flex items-center gap-1 px-3 py-1 rounded text-sm transition-colors"
          style={{
            background: subTab === "portfolio" ? "var(--accent)" : "transparent",
            color: subTab === "portfolio" ? "white" : "var(--muted)",
          }}
        >
          <Briefcase size={14} />
          {t("workspace.view.portfolio")}
        </button>
        {currentStockCode && (
          <span className="ml-auto text-sm" style={{ color: "var(--muted)" }}>
            {t("workspace.view.currentStock")}: {currentStockCode}
          </span>
        )}
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto">
        {subTab === "watchlist" ? <WatchlistPage /> : <PortfolioDashboard />}
      </div>
    </div>
  );
}
