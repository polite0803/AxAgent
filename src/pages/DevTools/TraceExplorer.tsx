import { TraceDetail } from "@/components/devtools/TraceDetail";
import { TraceList } from "@/components/devtools/TraceList";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Empty, Spin } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

export function TraceExplorer() {
  const { selectedTrace, isLoading, loadTraces } = useTracerStore();
  const { t } = useTranslation();

  useEffect(() => {
    loadTraces();
  }, [loadTraces]);

  return (
    <div className="flex h-full">
      <div className="w-80 border-r border-gray-200 overflow-auto">
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
          : <Empty description={t("traceExplorer.selectTrace")} className="mt-20" />}
      </div>
    </div>
  );
}
