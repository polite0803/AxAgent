// SPDX-License-Identifier: AGPL-3.0-only

import { Table, Tabs, Typography } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

// ── Types ──

export interface BottleneckData {
  timeDistribution: TimeDistributionItem[];
  tokenDistribution: TokenConsumptionItem[];
  failureModes: FailurePatternItem[];
}

export interface TimeDistributionItem {
  name: string;
  value: number;
  color: string;
}

export interface TokenConsumptionItem {
  name: string;
  tokens: number;
}

export interface FailurePatternItem {
  reason: string;
  count: number;
  pct: number;
}

// ── Mock ──

function buildMockBottlenecks(): BottleneckData {
  return {
    timeDistribution: [
      { name: "LLM 推理", value: 45, color: "#1890ff" },
      { name: "工具调用", value: 25, color: "#fa8c16" },
      { name: "等待权限", value: 15, color: "#fadb14" },
      { name: "网络延迟", value: 10, color: "#722ed1" },
      { name: "其他", value: 5, color: "#d9d9d9" },
    ],
    tokenDistribution: [
      { name: "系统提示词", tokens: 1200 },
      { name: "工具定义", tokens: 800 },
      { name: "对话历史", tokens: 3200 },
      { name: "工具结果", tokens: 1500 },
      { name: "用户输入", tokens: 400 },
    ],
    failureModes: [
      { reason: "工具执行超时", count: 12, pct: 40 },
      { reason: "权限不足", count: 8, pct: 26.7 },
      { reason: "参数格式错误", count: 5, pct: 16.7 },
      { reason: "网络错误", count: 3, pct: 10 },
      { reason: "LLM 输出解析失败", count: 2, pct: 6.6 },
    ],
  };
}

// ── Simple chart components ──

function PieChart({ data }: { data: TimeDistributionItem[] }) {
  const total = data.reduce((s, d) => s + d.value, 0);

  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 16, padding: 16 }}>
      <div style={{ position: "relative", width: 200, height: 200 }}>
        <svg viewBox="0 0 36 36" style={{ width: "100%", height: "100%" }}>
          {(() => {
            const segments: { d: string; color: string }[] = [];
            let cumulative = 0;
            for (const d of data) {
              const pct = d.value / total;
              const startAngle = cumulative * 360;
              const endAngle = (cumulative + pct) * 360;
              cumulative += pct;
              const largeArc = pct > 0.5 ? 1 : 0;
              const startRad = ((startAngle - 90) * Math.PI) / 180;
              const endRad = ((endAngle - 90) * Math.PI) / 180;
              const r = 15.9;
              const cx = 18, cy = 18;
              const x1 = cx + r * Math.cos(startRad);
              const y1 = cy + r * Math.sin(startRad);
              const x2 = cx + r * Math.cos(endRad);
              const y2 = cy + r * Math.sin(endRad);
              segments.push({
                d: `M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2} Z`,
                color: d.color,
              });
            }
            return segments.map((seg, i) => (
              <path
                key={i}
                d={seg.d}
                fill={seg.color}
                stroke="#fff"
                strokeWidth={0.5}
              />
            ));
          })()}
        </svg>
      </div>
      <div>
        {data.map((d) => (
          <div key={d.name} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
            <span style={{ display: "inline-block", width: 12, height: 12, borderRadius: 2, background: d.color }} />
            <Text style={{ fontSize: 12 }}>{d.name}</Text>
            <Text strong style={{ fontSize: 12 }}>{((d.value / total) * 100).toFixed(0)}%</Text>
          </div>
        ))}
      </div>
    </div>
  );
}

function BarChart({ data }: { data: TokenConsumptionItem[] }) {
  const maxVal = Math.max(...data.map((d) => d.tokens), 1);

  return (
    <div style={{ padding: 16 }}>
      {data.map((d) => (
        <div key={d.name} style={{ marginBottom: 8 }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
            <Text style={{ fontSize: 12 }}>{d.name}</Text>
            <Text style={{ fontSize: 12 }}>{d.tokens.toLocaleString()}</Text>
          </div>
          <div style={{ height: 20, background: "#f5f5f5", borderRadius: 4, overflow: "hidden" }}>
            <div
              style={{
                height: "100%",
                width: `${(d.tokens / maxVal) * 100}%`,
                background: "linear-gradient(90deg, #1890ff, #69c0ff)",
                borderRadius: 4,
                transition: "width 0.4s",
              }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}

// ── Main Component ──

interface BottleneckAnalyzerProps {
  traceId: string;
}

export function BottleneckAnalyzer({ traceId: _traceId }: BottleneckAnalyzerProps) {
  const { t } = useTranslation();
  const data = useMemo(() => buildMockBottlenecks(), []);

  const failureColumns = [
    { title: "失败原因", dataIndex: "reason", key: "reason" },
    { title: "次数", dataIndex: "count", key: "count", width: 80, align: "right" as const },
    {
      title: "占比",
      dataIndex: "pct",
      key: "pct",
      width: 80,
      align: "right" as const,
      render: (v: number) => `${v}%`,
    },
  ];

  return (
    <Tabs
      defaultActiveKey="time"
      size="small"
      items={[
        {
          key: "time",
          label: t("trace.bottleneck.timeDistribution", "时间分布"),
          children: <PieChart data={data.timeDistribution} />,
        },
        {
          key: "token",
          label: t("trace.bottleneck.tokenConsumption", "Token 消耗"),
          children: <BarChart data={data.tokenDistribution} />,
        },
        {
          key: "failure",
          label: t("trace.bottleneck.failurePatterns", "失败模式"),
          children: (
            <Table
              dataSource={data.failureModes.map((r, i) => ({ ...r, key: i }))}
              columns={failureColumns}
              pagination={false}
              size="small"
            />
          ),
        },
      ]}
    />
  );
}
