// StrategyListTab — 策略元数据列表

import { Empty, Space, Table, Tag } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { useStrategyStore } from "@/stores/feature/quant";
import type { StrategyMeta } from "@/types";

export function StrategyListTab() {
  const { t } = useTranslation();
  const strategies = useStrategyStore((s) => s.strategies);
  const isLoading = useStrategyStore((s) => s.isLoading);
  const loadStrategies = useStrategyStore((s) => s.loadStrategies);
  const error = useStrategyStore((s) => s.error);

  useEffect(() => {
    void loadStrategies(true);
  }, [loadStrategies]);

  const columns = [
    {
      title: t("quant.strategy.name"),
      dataIndex: "name",
      key: "name",
    },
    {
      title: t("quant.strategy.version"),
      dataIndex: "version",
      key: "version",
    },
    {
      title: t("quant.strategy.type"),
      dataIndex: "strategyType",
      key: "strategyType",
      render: (v: StrategyMeta["strategyType"]) => (
        <Tag color={v === "rhai" ? "purple" : "blue"}>
          {v === "rhai" ? t("quant.strategy.rhai") : t("quant.strategy.builtin")}
        </Tag>
      ),
    },
    {
      title: t("quant.strategy.description"),
      dataIndex: "description",
      key: "description",
      render: (v: string | null) => v || "—",
    },
    {
      title: t("quant.strategy.params"),
      dataIndex: "params",
      key: "params",
      render: (_: unknown, record: StrategyMeta) => (
        <Space size={4} wrap>
          {Object.entries(record.params).map(([k, v]) => (
            <Tag key={k} color="default">
              {k}={String(v)}
            </Tag>
          ))}
        </Space>
      ),
    },
    {
      title: t("quant.strategy.lastUpdated"),
      dataIndex: "updatedAt",
      key: "updatedAt",
      render: (v: number) => new Date(v * 1000).toLocaleString(),
    },
  ];

  if (strategies.length === 0 && !isLoading) {
    return <Empty description={error || t("quant.common.empty")} />;
  }

  return (
    <Table<StrategyMeta>
      rowKey="id"
      dataSource={strategies}
      columns={columns}
      loading={isLoading}
      pagination={false}
      size="small"
    />
  );
}
