import { useEffect } from "react";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import { TraceList } from "@/components/devtools/TraceList";
import { TraceDetail } from "@/components/devtools/TraceDetail";
import { Empty, Spin } from "antd";

export function TraceExplorer() {
  const { selectedTrace, isLoading, loadTraces } = useTracerStore();

  useEffect(() => {
    loadTraces();
  }, [loadTraces]);

  return (
    <div className="flex h-full">
      <div className="w-80 border-r border-gray-200 overflow-auto">
        <TraceList />
      </div>
      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <Spin size="large" />
          </div>
        ) : selectedTrace ? (
          <TraceDetail />
        ) : (
          <Empty description="选择一个追踪记录查看详情" className="mt-20" />
        )}
      </div>
    </div>
  );
}
