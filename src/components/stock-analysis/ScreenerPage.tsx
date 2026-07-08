import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { Collapse, Grid, Tabs } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "./_shared/PageHeader";
import { DragonTigerPanel } from "./DragonTigerPanel";
import { HotStocksPanel } from "./HotStocksPanel";
import { LimitUpPanel } from "./LimitUpPanel";
import { RecommendationPanel } from "./RecommendationPanel";
import { SerenityScreeningPanel } from "./SerenityScreeningPanel";
import { StockScreenerPanel } from "./StockScreenerPanel";

const { useBreakpoint } = Grid;

/**
 * ScreenerPage — 选股中心
 *
 * 顶部单卡 + Tabs:
 *   - 智能荐股:多周期 × 多风格的策略驱动推荐(原 RecommendationPanel)
 *   - 我的筛选:多因子条件筛选(原 StockScreenerPanel screen 模式)
 *
 * 底部折叠:HotStocks / LimitUp / DragonTiger — 默认收起的折叠面板
 *
 * 设计原则:系统被动推荐 与 用户主动筛选 走同一个入口,消除
 * "今日荐股 / 全市场发现" 这种重复暴露。
 */
export function ScreenerPage() {
  const { t } = useTranslation();
  const screens = useBreakpoint();
  const isMobile = !screens.md;
  const [activeTab, setActiveTab] = useState<string>("reco");
  const [activeKeys, setActiveKeys] = useState<string[]>([]);

  return (
    <PageErrorBoundary title="Screener">
      <div className="flex h-full flex-col">
        <PageHeader titleKey="screener.title" backTo="/stock-analysis" />
        <div className={["flex-1 overflow-auto space-y-4", isMobile ? "p-2" : "p-4"].join(" ")}>
          {/* 顶部:统一入口的"智能荐股 / 我的筛选"切换 */}
          <div>
            {
              /* Bug 13 修复: destroyOnHidden 让切走的 tab 立即 unmount,
                避免:
                  1. RecommendationPanel 内部 useEffect 中晚到的 invoke 在
                     hidden 时还去 setState,造成后续切回时显示过时/乱序数据
                  2. StockScreenerPanel 的因子勾选状态在两次 "我的筛选" 切换之间
                     残留,让用户对结果感到困惑
                切回时各 panel 重新 mount 并 invoke 自己的数据,语义最干净。 */
            }
            <Tabs
              activeKey={activeTab}
              onChange={setActiveTab}
              size={isMobile ? "small" : "middle"}
              destroyOnHidden
              items={[
                {
                  key: "reco",
                  label: (
                    <span className="text-sm font-medium">
                      {t("screener.tab.smartReco")}
                    </span>
                  ),
                  children: <RecommendationPanel />,
                },
                {
                  key: "serenity",
                  label: (
                    <span className="text-sm font-medium">
                      {t("screener.tab.serenity")}
                    </span>
                  ),
                  children: <SerenityScreeningPanel />,
                },
                {
                  key: "screen",
                  label: (
                    <span className="text-sm font-medium">
                      {t("screener.tab.myFilter")}
                    </span>
                  ),
                  children: <StockScreenerPanel />,
                },
              ]}
            />
          </div>

          {/* 底部:三个市场氛围面板(手风琴,默认全部收起) */}
          <Collapse
            accordion
            defaultActiveKey={[]}
            bordered={false}
            size={isMobile ? "small" : "middle"}
            activeKey={activeKeys}
            onChange={(keys) => setActiveKeys(keys as string[])}
            items={[
              {
                key: "hot",
                label: (
                  <span className="text-sm font-medium">
                    🔥 {t("stockAnalysis.settings.panels.hotStocks")}
                  </span>
                ),
                children: activeKeys.includes("hot") ? <HotStocksPanel bordered={false} /> : null,
              },
              {
                key: "limitup",
                label: (
                  <span className="text-sm font-medium">
                    🏆 {t("stockAnalysis.settings.panels.limitUp")}
                  </span>
                ),
                children: activeKeys.includes("limitup") ? <LimitUpPanel bordered={false} /> : null,
              },
              {
                key: "dragontiger",
                label: (
                  <span className="text-sm font-medium">
                    🐉 {t("stockAnalysis.settings.panels.dragonTiger")}
                  </span>
                ),
                children: activeKeys.includes("dragontiger") ? <DragonTigerPanel bordered={false} /> : null,
              },
            ]}
          />
        </div>
      </div>
    </PageErrorBoundary>
  );
}
