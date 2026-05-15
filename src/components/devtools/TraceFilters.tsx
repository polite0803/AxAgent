import { useTracerStore } from "@/stores/devtools/tracerStore";
import type { TraceFilter } from "@/types";
import { Button, DatePicker, Input, Select, Space } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { RangePicker } = DatePicker;

export function TraceFilters() {
  const { t } = useTranslation();
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
          <label className="text-xs text-gray-500 mb-1 block">{t("devtools.sessionId")}</label>
          <Input
            id="trace-filters-input-36"
            placeholder={t("devtools.filterSession")}
            value={localFilter.session_id || ""}
            onChange={(e) => setLocalFilter({ ...localFilter, session_id: e.target.value || undefined })}
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-gray-500 mb-1 block">{t("devtools.timeRange")}</label>
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
          <label className="text-xs text-gray-500 mb-1 block">{t("devtools.minDuration")}</label>
          <Input
            id="trace-filters-input-37"
            type="number"
            placeholder={t("devtools.minDuration")}
            value={localFilter.min_duration_ms || ""}
            onChange={(e) =>
              setLocalFilter({
                ...localFilter,
                min_duration_ms: e.target.value ? Number(e.target.value) : undefined,
              })}
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-gray-500 mb-1 block">{t("devtools.maxDuration")}</label>
          <Input
            id="trace-filters-input-38"
            type="number"
            placeholder={t("devtools.maxDuration")}
            value={localFilter.max_duration_ms || ""}
            onChange={(e) =>
              setLocalFilter({
                ...localFilter,
                max_duration_ms: e.target.value ? Number(e.target.value) : undefined,
              })}
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-gray-500 mb-1 block">{t("devtools.errorFilter")}</label>
          <Select
            className="w-full"
            placeholder={t("devtools.includeErrors")}
            value={localFilter.has_errors}
            onChange={(value) => setLocalFilter({ ...localFilter, has_errors: value })}
            allowClear
            options={[
              { value: true, label: t("devtools.errorOnly") },
              { value: false, label: t("devtools.successOnly") },
            ]}
          />
        </div>

        <Space className="w-full">
          <Button type="primary" onClick={handleApply} className="flex-1">
            {t("devtools.apply")}
          </Button>
          <Button onClick={handleReset}>{t("common.reset")}</Button>
        </Space>
      </div>
    </div>
  );
}
