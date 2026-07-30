// SPDX-License-Identifier: AGPL-3.0-only

import { AnnouncementsPanel } from "@/components/stock-analysis/AnnouncementsPanel";
import { ClsFlashPanel } from "@/components/stock-analysis/ClsFlashPanel";
import { ConceptBlocksPanel } from "@/components/stock-analysis/ConceptBlocksPanel";
import { EventCalendarPanel } from "@/components/stock-analysis/EventCalendarPanel";
import { IndexQuotesPanel } from "@/components/stock-analysis/IndexQuotesPanel";
import { IndustryRankingPanel } from "@/components/stock-analysis/IndustryRankingPanel";
import { NorthBoundPanel } from "@/components/stock-analysis/NorthBoundPanel";
import { OptionPcrPanel } from "@/components/stock-analysis/OptionPcrPanel";
import { PositionsMiniPanel } from "@/components/stock-analysis/PositionsMiniPanel";
import { SectorHeatmapPanel } from "@/components/stock-analysis/SectorHeatmapPanel";
import { useStockAnalysisStore, useWorkspaceStore } from "@/stores";
import { Collapse } from "antd";
import { ChevronRight, PanelRight } from "lucide-react";
import { useTranslation } from "react-i18next";

/**
 * 右栏：上下文侧栏 — 持仓表 + 大盘指数 + 辅助面板。
 *
 * 所有面板都在此独立展示，不占用主分析区空间。
 * 持仓和大盘指数是交易决策的前置条件，始终可见。
 * 辅助面板（板块/北向/日历等）可折叠展开。
 */
export function ContextSidebar() {
  const { t } = useTranslation();
  const collapsed = useWorkspaceStore((s) => s.rightSidebarCollapsed);
  const toggle = useWorkspaceStore((s) => s.toggleRightSidebar);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);

  if (collapsed) {
    return (
      <button
        type="button"
        onClick={toggle}
        className="flex items-center justify-center rounded hover:opacity-70"
        style={{
          width: 24,
          borderLeft: "1px solid var(--border)",
          background: "var(--surface)",
          flexShrink: 0,
          padding: "4px 0",
        }}
        title={t("workspace.contextSidebar.expand")}
      >
        <PanelRight size={12} />
      </button>
    );
  }

  const sidePanels = [
    { key: "sectors", label: t("stockAnalysis.settings.sheet.sectors"), children: <SectorHeatmapPanel /> },
    { key: "north", label: t("stockAnalysis.settings.sheet.north"), children: <NorthBoundPanel /> },
    { key: "events", label: t("stockAnalysis.settings.sheet.events"), children: <EventCalendarPanel /> },
    {
      key: "announcements",
      label: t("stockAnalysis.announcements"),
      children: <AnnouncementsPanel stockCode={stockCode} />,
    },
    {
      key: "concepts",
      label: t("stockAnalysis.conceptBlocks"),
      children: <ConceptBlocksPanel stockCode={stockCode} />,
    },
    { key: "optionpcr", label: t("stockAnalysis.optionPcr"), children: <OptionPcrPanel stockCode={stockCode} /> },
    { key: "industry", label: t("stockAnalysis.industryRanking"), children: <IndustryRankingPanel /> },
    { key: "flash", label: t("stockAnalysis.clsFlash"), children: <ClsFlashPanel /> },
  ];

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
        style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}
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

      {/* 持仓面板 — 决策前置条件，始终可见 */}
      <div style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}>
        <PositionsMiniPanel />
      </div>

      {/* 大盘指数面板 — 市场环境判断，始终可见 */}
      <div style={{ borderBottom: "1px solid var(--border)", flexShrink: 0 }}>
        <IndexQuotesPanel />
      </div>

      {/* 辅助面板区域 — 可折叠，按需展开 */}
      <div className="flex-1 overflow-y-auto" style={{ minHeight: 0 }}>
        <Collapse
          size="small"
          ghost
          items={sidePanels.map((p) => ({
            key: p.key,
            label: <span className="text-xs font-medium">{p.label}</span>,
            children: <div style={{ padding: "4px 0" }}>{p.children}</div>,
          }))}
        />
      </div>
    </div>
  );
}
