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
  const isPositive = v > 0;
  const good = (positiveIsGood && isPositive) || (!positiveIsGood && !isPositive);
  return good ? "#389e0d" : "#cf1322";
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
        valueStyle={color ? { color } : undefined}
        formatter={formatter ?? ((v) => v)}
      />
      {hint && <div style={{ fontSize: 12, color: "var(--text-3)", marginTop: 4 }}>{hint}</div>}
    </Card>
  );
}
