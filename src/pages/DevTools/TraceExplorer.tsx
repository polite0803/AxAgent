// SPDX-License-Identifier: AGPL-3.0-only

import { TraceDetail } from "@/components/devtools/TraceDetail";
import { TraceFilters } from "@/components/devtools/TraceFilters";
import { TraceList } from "@/components/devtools/TraceList";
import { BottleneckAnalyzer } from "@/components/trace/BottleneckAnalyzer";
import { FeedbackCollector } from "@/components/trace/FeedbackCollector";
import { ImprovementSuggestion } from "@/components/trace/ImprovementSuggestion";
import { TraceTimeline } from "@/components/trace/TraceTimeline";
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
      <div className="flex-1 flex flex-col overflow-hidden">
        {isLoading
          ? (
            <div className="flex items-center justify-center h-full">
              <Spin size="large" />
            </div>
          )
          : selectedTrace
          ? (
            <Tabs
              defaultActiveKey="detail"
              className="ax-fill-tabs"
              tabBarStyle={{ paddingLeft: 16, paddingTop: 4, flexShrink: 0, marginBottom: 0 }}
              items={[
                {
                  key: "detail",
                  label: t("traceExplorer.tab.detail"),
                  children: <TraceDetail />,
                },
                {
                  key: "timeline",
                  label: t("traceExplorer.tab.timeline"),
                  children: (
                    <div style={{ padding: 16, flex: 1, overflow: "auto" }}>
                      <TraceTimeline traceId={selectedTrace.trace.traceId} />
                    </div>
                  ),
                },
                {
                  key: "bottleneck",
                  label: t("traceExplorer.tab.bottleneck"),
                  children: (
                    <div style={{ padding: 16, flex: 1, overflow: "auto" }}>
                      <BottleneckAnalyzer traceId={selectedTrace.trace.traceId} />
                    </div>
                  ),
                },
                {
                  key: "suggestions",
                  label: t("traceExplorer.tab.suggestions"),
                  children: (
                    <div style={{ padding: 16, flex: 1, overflow: "auto" }}>
                      <ImprovementSuggestion traceId={selectedTrace.trace.traceId} />
                    </div>
                  ),
                },
                {
                  key: "feedback",
                  label: t("traceExplorer.tab.feedback"),
                  children: (
                    <div style={{ padding: 16, flex: 1, overflow: "auto" }}>
                      <FeedbackCollector traceId={selectedTrace.trace.traceId} />
                    </div>
                  ),
                },
              ]}
            />
          )
          : (
            <Empty
              description={t("traceExplorer.selectTrace")}
              className="mt-20"
            />
          )}
      </div>
    </div>
  );
}
