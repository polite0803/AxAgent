// SPDX-License-Identifier: AGPL-3.0-only

import { useFormatCny } from "@/stores";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import type { TraceSummary } from "@/types";
import { Button, Card, DatePicker, Input, Space, Tag, Typography } from "antd";
import dayjs from "dayjs";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

function formatDuration(ms?: number): string {
  if (!ms) {
    return "-";
  }
  if (ms < 1000) {
    return `${ms}ms`;
  }
  if (ms < 60000) {
    return `${(ms / 1000).toFixed(1)}s`;
  }
  return `${(ms / 60000).toFixed(1)}m`;
}

function getStatusColor(errorCount: number): "green" | "red" | "default" {
  if (errorCount > 0) {
    return "red";
  }
  return "green";
}

interface TraceItemProps {
  trace: TraceSummary;
  isSelected: boolean;
  onClick: () => void;
}

function TraceItem({ trace, isSelected, onClick }: TraceItemProps) {
  // 成本以人民币展示
  const formatCny = useFormatCny();
  return (
    <Card
      size="small"
      className={`mb-2 cursor-pointer transition-colors ${
        isSelected ? "border-blue-500 bg-blue-50" : "hover:bg-zinc-50"
      }`}
      onClick={onClick}
    >
      <div className="flex justify-between items-start">
        <div className="flex-1 min-w-0">
          <Text strong className="block truncate">
            {trace.traceId.slice(0, 8)}...
          </Text>
          <Text type="secondary" className="text-xs block">
            {dayjs(trace.startedAt).format("MM-DD HH:mm:ss")}
          </Text>
        </div>
        <Tag color={getStatusColor(trace.errorCount)} className="ml-2">
          {trace.errorCount > 0 ? `${trace.errorCount} errors` : "OK"}
        </Tag>
      </div>
      <div className="mt-2 flex gap-4">
        <Text type="secondary" className="text-xs">
          <span className="font-medium">{trace.spanCount}</span> spans
        </Text>
        <Text type="secondary" className="text-xs">
          <span className="font-medium">
            {formatDuration(trace.durationMs)}
          </span>
        </Text>
        <Text type="secondary" className="text-xs">
          <span className="font-medium">
            {formatCny(trace.totalCostUsd, 4)}
          </span>
        </Text>
      </div>
    </Card>
  );
}

export function TraceList() {
  const { t } = useTranslation();
  const { traces, selectedTrace, selectTrace, loadTraces, filter, setFilter } = useTracerStore();

  useEffect(() => {
    loadTraces();
  }, [loadTraces]);

  const handleSelect = (trace: TraceSummary) => {
    selectTrace(trace.traceId);
  };

  const selectedTraceId = selectedTrace?.trace.traceId;

  return (
    <div className="p-3">
      <Space orientation="vertical" className="w-full mb-4">
        <Space.Compact className="w-full">
          <Input
            placeholder={t("devtools.searchTraceId")}
            onPressEnter={(e) => {
              const value = (e.target as HTMLInputElement).value;
              const next = { ...filter, traceId: value || undefined };
              setFilter(next);
              loadTraces(next);
            }}
            onClear={() => {
              const next = { ...filter, traceId: undefined };
              setFilter(next);
              loadTraces(next);
            }}
            allowClear
          />
          <Button
            type="primary"
            onClick={() => {
              const next = { ...filter, traceId: undefined };
              setFilter(next);
              loadTraces(next);
            }}
          >
            {t("common.search")}
          </Button>
        </Space.Compact>
        <DatePicker.RangePicker
          className="w-full"
          onChange={(dates) => {
            if (dates && dates[0] && dates[1]) {
              const newFilter = {
                ...filter,
                fromDate: dates[0].toISOString(),
                toDate: dates[1].toISOString(),
              };
              setFilter(newFilter);
              loadTraces(newFilter);
            }
          }}
        />
      </Space>
      <div className="text-xs text-zinc-500 mb-2">
        {t("devtools.traceCount", { count: traces.length })}
      </div>
      <div className="divide-y divide-gray-100">
        {traces.map((trace) => (
          <TraceItem
            key={trace.traceId}
            trace={trace}
            isSelected={trace.traceId === selectedTraceId}
            onClick={() => handleSelect(trace)}
          />
        ))}
      </div>
    </div>
  );
}
