// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkspaceStore } from "@/stores";
import { BarChart3, Briefcase, ChevronLeft, Search, Star, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * 左栏：股票切换器。
 *
 * 显示最近访问的股票 + 自选股 + 持仓的快捷列表。
 * 切换股票时保持当前视图 Tab 不变。
 *
 * 阶段 1 为基础版本，阶段 4 会完善搜索 + 拖拽排序。
 */
export function StockSwitcher() {
  const { t } = useTranslation();
  const collapsed = useWorkspaceStore((s) => s.leftSidebarCollapsed);
  const toggle = useWorkspaceStore((s) => s.toggleLeftSidebar);
  const recentStocks = useWorkspaceStore((s) => s.recentStocks);
  const currentStockCode = useWorkspaceStore((s) => s.currentStockCode);
  const setCurrentStock = useWorkspaceStore((s) => s.setCurrentStock);

  const [search, setSearch] = useState("");

  const filteredRecent = useMemo(() => {
    if (!search.trim()) { return recentStocks; }
    const q = search.toLowerCase();
    return recentStocks.filter(
      (s) => s.code.toLowerCase().includes(q) || s.name.toLowerCase().includes(q),
    );
  }, [recentStocks, search]);

  // 折叠态：只显示图标列
  if (collapsed) {
    return (
      <div
        className="flex flex-col items-center gap-3 py-2 px-1"
        style={{
          width: 48,
          borderRight: "1px solid var(--border)",
          background: "var(--surface)",
          flexShrink: 0,
        }}
      >
        <button
          type="button"
          onClick={toggle}
          className="p-1.5 rounded hover:opacity-70 transition-opacity"
          title={t("workspace.stockSwitcher.expand")}
        >
          <ChevronLeft size={16} className="rotate-180" />
        </button>
        <button type="button" className="p-1.5 rounded hover:opacity-70" title={t("workspace.stockSwitcher.search")}>
          <Search size={16} />
        </button>
        <button type="button" className="p-1.5 rounded hover:opacity-70" title={t("workspace.stockSwitcher.watchlist")}>
          <Star size={16} />
        </button>
        <button type="button" className="p-1.5 rounded hover:opacity-70" title={t("workspace.stockSwitcher.holdings")}>
          <Briefcase size={16} />
        </button>
      </div>
    );
  }

  // 展开态：股票列表
  return (
    <div
      className="flex flex-col"
      style={{
        width: 240,
        borderRight: "1px solid var(--border)",
        background: "var(--surface)",
        flexShrink: 0,
      }}
    >
      {/* 标题栏 */}
      <div
        className="flex items-center justify-between px-2 py-1.5"
        style={{ borderBottom: "1px solid var(--border)" }}
      >
        <span className="text-sm font-semibold">{t("workspace.stockSwitcher.title")}</span>
        <button
          type="button"
          onClick={toggle}
          className="p-1 rounded hover:opacity-70"
          title={t("workspace.stockSwitcher.collapse")}
        >
          <ChevronLeft size={14} />
        </button>
      </div>

      {/* 搜索框 */}
      <div className="px-2 py-1.5">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t("workspace.stockSwitcher.search")}
          className="w-full text-sm px-2 py-1 rounded"
          style={{
            border: "1px solid var(--color-border-tertiary)",
            background: "transparent",
            color: "var(--color-text-primary)",
          }}
        />
      </div>

      {/* 最近访问列表 */}
      <div className="flex-1 overflow-auto px-1">
        {filteredRecent.length === 0
          ? (
            <div className="text-sm text-center py-4" style={{ color: "var(--muted)" }}>
              {t("workspace.stockSwitcher.empty")}
            </div>
          )
          : (
            <div className="space-y-0.5">
              {filteredRecent.map((stock) => {
                const isActive = stock.code === currentStockCode;
                return (
                  <button
                    key={stock.code}
                    type="button"
                    onClick={() => setCurrentStock(stock.code, stock.name)}
                    className="w-full flex items-center gap-2 px-2 py-1.5 rounded text-left transition-colors"
                    style={{
                      background: isActive ? "var(--accent-bg, rgba(59,130,246,0.10))" : "transparent",
                    }}
                  >
                    <BarChart3 size={14} style={{ color: isActive ? "var(--accent)" : "var(--muted)" }} />
                    <div className="flex-1 min-w-0">
                      <div
                        className="text-sm truncate"
                        style={{ color: isActive ? "var(--accent)" : "var(--color-text-primary)" }}
                      >
                        {stock.name}
                      </div>
                      <div className="text-sm font-mono" style={{ color: "var(--muted)" }}>
                        {stock.code}
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          )}
      </div>
    </div>
  );
}

/** 折叠态的股票切换器浮动按钮（移动端用） */
export function StockSwitcherFloating() {
  const { t } = useTranslation();
  const toggle = useWorkspaceStore((s) => s.toggleLeftSidebar);
  const collapsed = useWorkspaceStore((s) => s.leftSidebarCollapsed);

  if (!collapsed) { return null; }

  return (
    <button
      type="button"
      onClick={toggle}
      className="fixed left-2 top-20 z-50 p-2 rounded-full shadow-lg"
      style={{ background: "var(--accent)", color: "white" }}
      title={t("workspace.stockSwitcher.expand")}
    >
      <X size={16} className="rotate-45" />
    </button>
  );
}
