import { useEffect } from "react";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import { List, Card, Tag, Typography, Space, DatePicker, Input } from "antd";
import type { TraceSummary } from "@/types/tracer";
import dayjs from "dayjs";

const { Text } = Typography;

function formatDuration(ms?: number): string {
  if (!ms) return "-";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

function formatCost(cost: number): string {
  return `$${cost.toFixed(4)}`;
}

function getStatusColor(errorCount: number): "green" | "red" | "default" {
  if (errorCount > 0) return "red";
  return "green";
}

interface TraceItemProps {
  trace: TraceSummary;
  isSelected: boolean;
  onClick: () => void;
}

function TraceItem({ trace, isSelected, onClick }: TraceItemProps) {
  return (
    <Card
      size="small"
      className={`mb-2 cursor-pointer transition-colors ${
        isSelected ? "border-blue-500 bg-blue-50" : "hover:bg-gray-50"
      }`}
      onClick={onClick}
    >
      <div className="flex justify-between items-start">
        <div className="flex-1 min-w-0">
          <Text strong className="block truncate">
            {trace.trace_id.slice(0, 8)}...
          </Text>
          <Text type="secondary" className="text-xs block">
            {dayjs(trace.started_at).format("MM-DD HH:mm:ss")}
          </Text>
        </div>
        <Tag color={getStatusColor(trace.error_count)} className="ml-2">
          {trace.error_count > 0 ? `${trace.error_count} errors` : "OK"}
        </Tag>
      </div>
      <div className="mt-2 flex gap-4">
        <Text type="secondary" className="text-xs">
          <span className="font-medium">{trace.span_count}</span> spans
        </Text>
        <Text type="secondary" className="text-xs">
          <span className="font-medium">{formatDuration(trace.duration_ms)}</span>
        </Text>
        <Text type="secondary" className="text-xs">
          <span className="font-medium">{formatCost(trace.total_cost_usd)}</span>
        </Text>
      </div>
    </Card>
  );
}

export function TraceList() {
  const { traces, selectedTrace, selectTrace, loadTraces, filter, setFilter } =
    useTracerStore();

  useEffect(() => {
    loadTraces();
  }, [loadTraces]);

  const handleSelect = (trace: TraceSummary) => {
    selectTrace(trace.trace_id);
  };

  const selectedTraceId = selectedTrace?.trace.trace_id;

  return (
    <div className="p-3">
      <Space direction="vertical" className="w-full mb-4">
        <Input.Search
          placeholder="搜索 trace ID..."
          onSearch={(value) => {
            setFilter({ ...filter, trace_id: value || undefined });
            loadTraces({ ...filter, trace_id: value || undefined });
          }}
          allowClear
        />
        <DatePicker.RangePicker
          className="w-full"
          onChange={(dates) => {
            if (dates && dates[0] && dates[1]) {
              const newFilter = {
                ...filter,
                from_date: dates[0].toISOString(),
                to_date: dates[1].toISOString(),
              };
              setFilter(newFilter);
              loadTraces(newFilter);
            }
          }}
        />
      </Space>
      <div className="text-xs text-gray-500 mb-2">
        {traces.length} 个追踪记录
      </div>
      <List
        dataSource={traces}
        renderItem={(trace) => (
          <TraceItem
            trace={trace}
            isSelected={trace.trace_id === selectedTraceId}
            onClick={() => handleSelect(trace)}
          />
        )}
        locale={{ emptyText: "暂无追踪记录" }}
      />
    </div>
  );
}
