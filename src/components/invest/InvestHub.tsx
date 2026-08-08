// SPDX-License-Identifier: AGPL-3.0-only

import { Tabs, type TabsProps, Typography } from "antd";
import { LineChart } from "lucide-react";
import { lazy, Suspense, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";

import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";

const { Title } = Typography;

// 各业务子页面懒加载（复用现有页面入口）
const LazyMarketMainline = lazy(() =>
  import("@/pages/MarketMainlinePage").then((m) => ({ default: m.MarketMainlinePage }))
);
const LazyScreener = lazy(() => import("@/pages/ScreenerPage").then((m) => ({ default: m.ScreenerPage })));
const LazyStockWorkspace = lazy(() =>
  import("@/pages/StockWorkspacePage").then((m) => ({ default: m.StockWorkspacePage }))
);
const LazyScreenshotDiagnosis = lazy(() =>
  import("@/pages/ScreenshotDiagnosisPage").then((m) => ({ default: m.ScreenshotDiagnosisPage }))
);
const LazyPaperPortfolio = lazy(() =>
  import("@/pages/PaperPortfolioPage").then((m) => ({ default: m.PaperPortfolioPage }))
);
const LazyQuantLab = lazy(() => import("@/pages/QuantLabPage").then((m) => ({ default: m.QuantLabPage })));
const LazyPipeline = lazy(() => import("@/pages/PipelinePage").then((m) => ({ default: m.PipelinePage })));

/** 投资业务 tab key — 按操作逻辑排序：全局视角 → 发现 → 单股深度 → 外部导入 → 持仓跟踪 → 策略验证 → 流程编排 */
export type InvestTabKey =
  | "market-mainline"
  | "screener"
  | "workspace"
  | "screenshot-diagnosis"
  | "paper-portfolio"
  | "quant"
  | "pipeline";

/** tab 默认值 */
const DEFAULT_TAB: InvestTabKey = "market-mainline";

/** 合法 tab key 集合（用于校验 URL 参数） */
const VALID_TABS: Set<InvestTabKey> = new Set([
  "market-mainline",
  "screener",
  "workspace",
  "screenshot-diagnosis",
  "paper-portfolio",
  "quant",
  "pipeline",
]);

function TabLoader() {
  const { t } = useTranslation();
  return (
    <div className="flex items-center justify-center h-full w-full" style={{ minHeight: 200 }}>
      {t("common.loading")}
    </div>
  );
}

function SafeTab({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation();
  return (
    <PageErrorBoundary title={t("error.page")}>
      <Suspense fallback={<TabLoader />}>{children}</Suspense>
    </PageErrorBoundary>
  );
}

/**
 * InvestHub — 投资业务统一入口。
 *
 * 将 7 个 AxInvest 独有的股票业务页面集成到一个页面内的 Tab 中，按业务操作逻辑排序：
 *   1. 市场主线（全局视角） → 2. 选股（发现候选） → 3. 工作区（单股深度）
 *   → 4. 截图诊断（外部导入） → 5. 模拟观察（持仓跟踪）
 *   → 6. 量化（策略验证） → 7. 管道（流程编排）
 *
 * URL 参数：
 *   - ?tab=xxx — 当前激活的 tab（刷新保持）
 *   - ?stockCode=xxx — workspace tab 专用，驱动当前股票
 *   - ?view=xxx — workspace tab 专用，驱动当前视图
 */
export function InvestHub() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();

  // 从 URL 读取当前 tab（非法值回退到默认）
  const currentTab = useMemo<InvestTabKey>(() => {
    const raw = searchParams.get("tab");
    if (raw && VALID_TABS.has(raw as InvestTabKey)) {
      return raw as InvestTabKey;
    }
    return DEFAULT_TAB;
  }, [searchParams]);

  // tab 切换 → 更新 URL（保留 stockCode/view 等其他参数）
  const handleTabChange = (key: string) => {
    const next = new URLSearchParams(searchParams);
    next.set("tab", key);
    // 切换到非 workspace tab 时清理 workspace 专属参数
    if (key !== "workspace") {
      next.delete("stockCode");
      next.delete("view");
    }
    setSearchParams(next, { replace: true });
  };

  const items: TabsProps["items"] = useMemo(
    () => [
      {
        key: "market-mainline",
        label: t("invest.tab.marketMainline"),
        children: (
          <SafeTab>
            <LazyMarketMainline />
          </SafeTab>
        ),
      },
      {
        key: "screener",
        label: t("invest.tab.screener"),
        children: (
          <SafeTab>
            <LazyScreener />
          </SafeTab>
        ),
      },
      {
        key: "workspace",
        label: t("invest.tab.workspace"),
        children: (
          <SafeTab>
            <LazyStockWorkspace />
          </SafeTab>
        ),
      },
      {
        key: "screenshot-diagnosis",
        label: t("invest.tab.screenshotDiagnosis"),
        children: (
          <SafeTab>
            <LazyScreenshotDiagnosis />
          </SafeTab>
        ),
      },
      {
        key: "paper-portfolio",
        label: t("invest.tab.paperPortfolio"),
        children: (
          <SafeTab>
            <LazyPaperPortfolio />
          </SafeTab>
        ),
      },
      {
        key: "quant",
        label: t("invest.tab.quant"),
        children: (
          <SafeTab>
            <LazyQuantLab />
          </SafeTab>
        ),
      },
      {
        key: "pipeline",
        label: t("invest.tab.pipeline"),
        children: (
          <SafeTab>
            <LazyPipeline />
          </SafeTab>
        ),
      },
    ],
    [t],
  );

  return (
    <div className="flex flex-col h-full w-full min-h-0">
      <div style={{ padding: "12px 16px 0", background: "var(--color-bg-container)" }}>
        <Title level={3} style={{ margin: 0 }}>
          <LineChart size={20} style={{ marginRight: 8, verticalAlign: "middle" }} />
          {t("invest.title")}
        </Title>
      </div>
      <Tabs
        activeKey={currentTab}
        onChange={handleTabChange}
        items={items}
        className="invest-hub-tabs ax-fill-tabs"
        tabBarStyle={{
          margin: 0,
          padding: "0 16px",
          background: "var(--color-bg-container)",
          borderBottom: "1px solid var(--color-border-secondary)",
        }}
        tabBarGutter={24}
        size="small"
        destroyOnHidden={false}
      />
    </div>
  );
}
