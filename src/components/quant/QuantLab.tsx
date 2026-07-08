// QuantLab — 量化实验台主入口
// 4 个 tab：回测运行 / 策略管理 / Rhai 编辑器 / 指标对比

import { Tabs } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { BacktestTab } from "@/components/quant/tabs/BacktestTab";
import { CompareTab } from "@/components/quant/tabs/CompareTab";
import { RhaiEditorTab } from "@/components/quant/tabs/RhaiEditorTab";
import { StrategyListTab } from "@/components/quant/tabs/StrategyListTab";
import { PageHeader } from "@/components/stock-analysis/_shared/PageHeader";

type TabKey = "backtest" | "strategies" | "rhai" | "compare";

export function QuantLab() {
  const { t } = useTranslation();
  const [active, setActive] = useState<TabKey>("backtest");

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0 }}>
      <PageHeader
        titleKey="quant.title"
        backTo="/"
        meta={t("quant.subtitle")}
      />
      <div className="flex-1 overflow-auto p-4">
        <Tabs
          activeKey={active}
          onChange={(k) => setActive(k as TabKey)}
          items={[
            {
              key: "backtest",
              label: t("quant.tabs.backtest"),
              children: <BacktestTab />,
            },
            {
              key: "strategies",
              label: t("quant.tabs.strategies"),
              children: <StrategyListTab />,
            },
            {
              key: "rhai",
              label: t("quant.tabs.rhai"),
              children: <RhaiEditorTab />,
            },
            {
              key: "compare",
              label: t("quant.tabs.compare"),
              children: <CompareTab />,
            },
          ]}
        />
      </div>
    </div>
  );
}
