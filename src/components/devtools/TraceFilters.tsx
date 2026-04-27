import { useState } from "react";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Input, Select, DatePicker, Button, Space } from "antd";
import type { TraceFilter } from "@/types/tracer";

const { RangePicker } = DatePicker;

export function TraceFilters() {
  const { filter, setFilter } = useTracerStore();
  const [localFilter, setLocalFilter] = useState<TraceFilter>(filter);

  const handleApply = () => {
    setFilter(localFilter);
  };

  const handleReset = () => {
    const emptyFilter: TraceFilter = {};
    setLocalFilter(emptyFilter);
    setFilter(emptyFilter);
  };

  return (
    <div className="p-3 border-b">
      <div className="space-y-3">
        <div>
          <label className="text-xs text-gray-500 mb-1 block">会话 ID</label>
          <Input
            placeholder="过滤会话"
            value={localFilter.session_id || ""}
            onChange={(e) =>
              setLocalFilter({ ...localFilter, session_id: e.target.value || undefined })
            }
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-gray-500 mb-1 block">时间范围</label>
          <RangePicker
            className="w-full"
            showTime
            onChange={(dates) => {
              if (dates && dates[0] && dates[1]) {
                setLocalFilter({
                  ...localFilter,
                  from_date: dates[0].toISOString(),
                  to_date: dates[1].toISOString(),
                });
              } else {
                setLocalFilter({
                  ...localFilter,
                  from_date: undefined,
                  to_date: undefined,
                });
              }
            }}
          />
        </div>

        <div>
          <label className="text-xs text-gray-500 mb-1 block">最小耗时 (ms)</label>
          <Input
            type="number"
            placeholder="最小耗时"
            value={localFilter.min_duration_ms || ""}
            onChange={(e) =>
              setLocalFilter({
                ...localFilter,
                min_duration_ms: e.target.value ? Number(e.target.value) : undefined,
              })
            }
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-gray-500 mb-1 block">最大耗时 (ms)</label>
          <Input
            type="number"
            placeholder="最大耗时"
            value={localFilter.max_duration_ms || ""}
            onChange={(e) =>
              setLocalFilter({
                ...localFilter,
                max_duration_ms: e.target.value ? Number(e.target.value) : undefined,
              })
            }
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-gray-500 mb-1 block">错误筛选</label>
          <Select
            className="w-full"
            placeholder="是否包含错误"
            value={localFilter.has_errors}
            onChange={(value) => setLocalFilter({ ...localFilter, has_errors: value })}
            allowClear
            options={[
              { value: true, label: "仅错误" },
              { value: false, label: "仅成功" },
            ]}
          />
        </div>

        <Space className="w-full">
          <Button type="primary" onClick={handleApply} className="flex-1">
            应用
          </Button>
          <Button onClick={handleReset}>重置</Button>
        </Space>
      </div>
    </div>
  );
}
