// SPDX-License-Identifier: AGPL-3.0-only

import BottleneckAnalyzer from "@/components/trace/BottleneckAnalyzer";
import FeedbackCollector from "@/components/trace/FeedbackCollector";
import ImprovementSuggestion from "@/components/trace/ImprovementSuggestion";
import TraceTimeline from "@/components/trace/TraceTimeline";
import { TraceDetail } from "@/components/devtools/TraceDetail";
import { TraceFilters } from "@/components/devtools/TraceFilters";
import { TraceList } from "@/components/devtools/TraceList";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Empty, Spin, Tabs, theme } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

export function TraceExplorer() {
  const { selectedTrace, isLoading, loadTraces } = useTracerStore();
  const { t } = useTranslation();
  const { token } = theme.useToken();

  useEffect(() => {
    loadTraces();
  }, [loadTraces]);

  return (
    <div className="flex h-full">
      <div
        className="w-80 border-r overflow-auto flex flex-col"
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <TraceFilters />
        <TraceList />
      </div>
      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <Spin size="large" />
          </div>
        ) : selectedTrace ? (
          <Tabs
            defaultActiveKey="detail"
            tabBarStyle={{ paddingLeft: 16, paddingTop: 4 }}
            style={{ height: "100%" }}
            items={[
              {
                key: "detail",
                label: t("traceExplorer.tab.detail", "详情"),
                children: <TraceDetail />,
              },
              {
                key: "timeline",
                label: t("traceExplorer.tab.timeline", "时间线"),
                children: (
                  <div style={{ padding: 16 }}>
                    <TraceTimeline traceId={selectedTrace.trace.trace_id} />
                  </div>
                ),
              },
              {
                key: "bottleneck",
                label: t("traceExplorer.tab.bottleneck", "瓶颈分析"),
                children: (
                  <div style={{ padding: 16 }}>
                    <BottleneckAnalyzer traceId={selectedTrace.trace.trace_id} />
                  </div>
                ),
              },
              {
                key: "suggestions",
                label: t("traceExplorer.tab.suggestions", "改进建议"),
                children: (
                  <div style={{ padding: 16 }}>
                    <ImprovementSuggestion traceId={selectedTrace.trace.trace_id} />
                  </div>
                ),
              },
              {
                key: "feedback",
                label: t("traceExplorer.tab.feedback", "反馈"),
                children: (
                  <div style={{ padding: 16 }}>
                    <FeedbackCollector traceId={selectedTrace.trace.trace_id} />
                  </div>
                ),
              },
            ]}
          />
        ) : (
          <Empty
            description={t("traceExplorer.selectTrace")}
            className="mt-20"
          />
        )}
      </div>
    </div>
  );
}
