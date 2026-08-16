// SPDX-License-Identifier: AGPL-3.0-only

import { useTracerStore } from "@/stores/devtools/tracerStore";
import type { TraceFilter } from "@/types";
import { Button, DatePicker, Input, Select, Space } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { RangePicker } = DatePicker;

export function TraceFilters() {
  const { t } = useTranslation();
  const { filter, setFilter, loadTraces } = useTracerStore();
  const [localFilter, setLocalFilter] = useState<TraceFilter>(filter);

  // DT-P0-1: 应用筛选时必须触发数据加载,否则功能闭环断裂
  const handleApply = () => {
    setFilter(localFilter);
    loadTraces(localFilter);
  };

  const handleReset = () => {
    const emptyFilter: TraceFilter = {};
    setLocalFilter(emptyFilter);
    setFilter(emptyFilter);
    loadTraces(emptyFilter);
  };

  return (
    <div className="p-3 border-b">
      <div className="space-y-3">
        <div>
          <label className="text-xs text-zinc-500 mb-1 block">
            {t("devtools.sessionId")}
          </label>
          <Input
            id="trace-filters-input-36"
            placeholder={t("devtools.filterSession")}
            value={localFilter.sessionId || ""}
            onChange={(e) =>
              setLocalFilter((prev) => ({
                ...prev,
                sessionId: e.target.value || undefined,
              }))}
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-zinc-500 mb-1 block">
            {t("devtools.timeRange")}
          </label>
          <RangePicker
            className="w-full"
            showTime
            onChange={(dates) => {
              if (dates?.[0] && dates?.[1]) {
                setLocalFilter((prev) => ({
                  ...prev,
                  fromDate: dates[0]!.toISOString(),
                  toDate: dates[1]!.toISOString(),
                }));
              } else {
                setLocalFilter((prev) => ({
                  ...prev,
                  fromDate: undefined,
                  toDate: undefined,
                }));
              }
            }}
          />
        </div>

        <div>
          <label className="text-xs text-zinc-500 mb-1 block">
            {t("devtools.minDuration")}
          </label>
          <Input
            id="trace-filters-input-37"
            type="number"
            placeholder={t("devtools.minDuration")}
            value={localFilter.minDurationMs || ""}
            onChange={(e) =>
              setLocalFilter((prev) => ({
                ...prev,
                minDurationMs: e.target.value
                  ? Number(e.target.value)
                  : undefined,
              }))}
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-zinc-500 mb-1 block">
            {t("devtools.maxDuration")}
          </label>
          <Input
            id="trace-filters-input-38"
            type="number"
            placeholder={t("devtools.maxDuration")}
            value={localFilter.maxDurationMs || ""}
            onChange={(e) =>
              setLocalFilter((prev) => ({
                ...prev,
                maxDurationMs: e.target.value
                  ? Number(e.target.value)
                  : undefined,
              }))}
            allowClear
          />
        </div>

        <div>
          <label className="text-xs text-zinc-500 mb-1 block">
            {t("devtools.errorFilter")}
          </label>
          <Select
            className="w-full"
            placeholder={t("devtools.includeErrors")}
            value={localFilter.hasErrors}
            onChange={(value) => setLocalFilter((prev) => ({ ...prev, hasErrors: value }))}
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
