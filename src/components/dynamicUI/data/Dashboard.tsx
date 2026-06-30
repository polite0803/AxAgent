// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { ArrowDownOutlined, ArrowUpOutlined } from "@ant-design/icons";
import { Card, Statistic } from "antd";

interface DashboardItem {
  label: string;
  value: string | number;
  unit?: string;
  trend?: "up" | "down";
  color?: string;
}

/**
 * 仪表盘组件。
 * 渲染数值卡片网格，基于 Ant Design Statistic。
 */
export const Dashboard: React.FC<DynamicUIProps> = ({ schema, dataContext }) => {
  const rawItems = schema.props.items as DashboardItem[] | undefined;

  const items: DashboardItem[] = rawItems
      || (dataContext
        && Array.isArray(
          (dataContext as Record<string, unknown>)[schema.id],
        ))
    ? (
      (dataContext as Record<string, unknown>)[schema.id] as DashboardItem[]
    )
    : [];

  const { columns = 3, gap = 16 } = schema.props as {
    columns?: number;
    gap?: number;
  };

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: `repeat(${columns}, 1fr)`,
        gap: `${gap}px`,
        ...(schema.style as React.CSSProperties),
      }}
    >
      {items.map((item, index) => (
        <Card key={index} size="small">
          <Statistic
            title={item.label}
            value={item.value}
            suffix={item.unit}
            valueStyle={item.color ? { color: item.color } : undefined}
            prefix={item.trend === "up"
              ? <ArrowUpOutlined style={{ color: "#52c41a" }} />
              : item.trend === "down"
              ? <ArrowDownOutlined style={{ color: "#ff4d4f" }} />
              : undefined}
          />
        </Card>
      ))}
    </div>
  );
};

export default Dashboard;
