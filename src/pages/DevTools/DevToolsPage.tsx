// SPDX-License-Identifier: AGPL-3.0-only

import { RLTrainingPanel } from "@/components/devtools/RLTrainingPanel";
import { BenchmarkRunner } from "@/pages/DevTools/BenchmarkRunner";
import { ToolRecommender } from "@/pages/DevTools/ToolRecommender";
import { TraceExplorer } from "@/pages/DevTools/TraceExplorer";
import { FineTunePage } from "@/pages/FineTunePage";
import { Tabs } from "antd";
import { BrainCircuit, Gauge, Search, Trophy, Wand2 } from "lucide-react";
import { useTranslation } from "react-i18next";

/**
 * 开发者工具统一页面。
 * 合并原 5 个独立侧栏导航项为 1 项，内部 Tab 切换：
 * 追踪浏览器 / 基准测试 / 工具推荐 / 模型微调 / 强化学习训练。
 */
export function DevToolsPage() {
  const { t } = useTranslation();

  const tabLabel = (icon: React.ReactNode, text: string) => (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
      {icon}
      {text}
    </span>
  );

  const tabItems = [
    {
      key: "trace-explorer",
      label: tabLabel(<Search size={14} />, t("nav.devtools.traceExplorer")),
      children: <TraceExplorer />,
    },
    {
      key: "benchmark",
      label: tabLabel(<Gauge size={14} />, t("nav.devtools.benchmark")),
      children: <BenchmarkRunner />,
    },
    {
      key: "tool-recommender",
      label: tabLabel(<Wand2 size={14} />, t("nav.devtools.toolRecommender")),
      children: <ToolRecommender />,
    },
    {
      key: "fine-tune",
      label: tabLabel(<BrainCircuit size={14} />, t("nav.devtools.fineTune")),
      children: <FineTunePage />,
    },
    {
      key: "rl-training",
      label: tabLabel(<Trophy size={14} />, t("nav.devtools.rlTraining")),
      children: <RLTrainingPanel />,
    },
  ];

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <Tabs
        defaultActiveKey="trace-explorer"
        items={tabItems}
        style={{ flex: 1, minHeight: 0, padding: "0 16px" }}
        tabBarStyle={{ flexShrink: 0, marginBottom: 0 }}
        destroyInactiveTabPane
      />
    </div>
  );
}
