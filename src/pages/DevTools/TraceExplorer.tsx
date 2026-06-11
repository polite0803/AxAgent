// SPDX-License-Identifier: AGPL-3.0-only

import { TraceDetail } from "@/components/devtools/TraceDetail";
import { TraceFilters } from "@/components/devtools/TraceFilters";
import { TraceList } from "@/components/devtools/TraceList";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Empty, Spin, theme } from "antd";
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
        {isLoading
          ? (
            <div className="flex items-center justify-center h-full">
              <Spin size="large" />
            </div>
          )
          : selectedTrace
          ? <TraceDetail />
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
