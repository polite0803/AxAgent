// QuantMetricsCard — 单卡展示 1 个核心指标
import { Card, Statistic } from "antd";

interface QuantMetricsCardProps {
  title: string;
  value: number;
  precision?: number;
  suffix?: string;
  prefix?: React.ReactNode;
  formatter?: (v: number | string) => React.ReactNode;
  positiveIsGood?: boolean;
  hint?: string;
}

function colorForValue(v: number, positiveIsGood: boolean | undefined): string | undefined {
  if (v === 0) { return undefined; }
  if (positiveIsGood === undefined) { return undefined; }
  // 修复 H11: A 股约定涨=红跌=绿（与 TradesTable/BacktestPanel 一致）
  // 原代码正收益显绿色(#389e0d) 误导判断，调换为涨红跌绿
  const isPositive = v > 0;
  const good = (positiveIsGood && isPositive) || (!positiveIsGood && !isPositive);
  return good ? "#cf1322" : "#389e0d";
}

export function QuantMetricsCard({
  title,
  value,
  precision = 2,
  suffix,
  prefix,
  formatter,
  positiveIsGood,
  hint,
}: QuantMetricsCardProps) {
  const color = colorForValue(value, positiveIsGood);
  return (
    <Card size="small" hoverable>
      <Statistic
        title={title}
        value={value}
        precision={precision}
        suffix={suffix}
        prefix={prefix}
        styles={{ content: color ? { color } : undefined }}
        formatter={formatter ?? ((v) => v)}
      />
      {hint && <div style={{ fontSize: 12, color: "var(--text-3)", marginTop: 4 }}>{hint}</div>}
    </Card>
  );
}
