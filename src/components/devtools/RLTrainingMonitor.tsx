// SPDX-License-Identifier: AGPL-3.0-only

import { useRlTrainingStore } from "@/stores/feature/rlTrainingStore";
import { Progress, Statistic, Typography } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

// Mini SVG chart — 声明在组件外部以避免在 render 期间创建组件
function MiniLineChart({ data, color }: { data: { step: number; value: number }[]; color: string }) {
  if (data.length < 2) { return <Text type="secondary">数据不足</Text>; }
  const w = 340, h = 100, pad = { t: 8, r: 8, b: 16, l: 40 };
  const xs = data.map((d) => d.step);
  const ys = data.map((d) => d.value);
  const xMin = Math.min(...xs), xMax = Math.max(...xs) || 1;
  const yMin = Math.min(...ys), yMax = Math.max(...ys) || 1;
  const yRange = yMax - yMin || 1;
  const sx = (x: number) => pad.l + ((x - xMin) / (xMax - xMin)) * (w - pad.l - pad.r);
  const sy = (y: number) => h - pad.b - ((y - yMin) / yRange) * (h - pad.t - pad.b);
  const pathD = data.map((d, i) => `${i === 0 ? "M" : "L"}${sx(d.step)},${sy(d.value)}`).join(" ");
  const yTicks = [yMin, yMin + yRange * 0.5, yMax];

  return (
    <svg viewBox={`0 0 ${w} ${h}`} style={{ width: "100%", maxWidth: w, height: "auto" }}>
      {/* Grid lines */}
      {yTicks.map((v, i) => (
        <line key={i} x1={pad.l} x2={w - pad.r} y1={sy(v)} y2={sy(v)} stroke="#f0f0f0" strokeWidth={1} />
      ))}
      {/* Y-axis labels */}
      {yTicks.map((v, i) => (
        <text key={i} x={pad.l - 6} y={sy(v) + 4} textAnchor="end" fontSize={9} fill="#999">
          {v.toFixed(3)}
        </text>
      ))}
      {/* Line */}
      <path d={pathD} fill="none" stroke={color} strokeWidth={1.5} />
    </svg>
  );
}

export default function RLTrainingMonitor() {
  const { t } = useTranslation();
  const status = useRlTrainingStore((s) => s.status);
  const currentMetrics = useRlTrainingStore((s) => s.currentMetrics);
  const metricsHistory = useRlTrainingStore((s) => s.metricsHistory);
  const config = useRlTrainingStore((s) => s.config);

  const chartData = useMemo(() => {
    const last50 = metricsHistory.slice(-50);
    return {
      loss: last50.map((m) => ({ step: m.step, value: m.loss })),
      reward: last50.map((m) => ({ step: m.step, value: m.reward })),
      policyLoss: last50.map((m) => ({ step: m.step, value: m.policyLoss })),
      valueLoss: last50.map((m) => ({ step: m.step, value: m.valueLoss })),
    };
  }, [metricsHistory]);

  const maxStep = config?.maxSteps ?? 10000;
  const currentStep = currentMetrics?.step ?? 0;
  const progress = Math.min(100, (currentStep / maxStep) * 100);

  const stepsPerSec = useMemo(() => {
    if (metricsHistory.length < 2) { return 0; }
    const last = metricsHistory[metricsHistory.length - 1];
    const prev = metricsHistory[Math.max(0, metricsHistory.length - 6)];
    const dt = (last.timestamp - prev.timestamp) / 1000;
    const ds = last.step - prev.step;
    return dt > 0 ? ds / dt : 0;
  }, [metricsHistory]);

  if (status === "idle") {
    return <Text type="secondary">{t("rl.monitor.idle", "训练未开始，请先配置并启动训练。")}</Text>;
  }

  return (
    <div>
      {/* Progress */}
      <Progress
        percent={Math.round(progress * 10) / 10}
        status={status === "failed" ? "exception" : status === "completed" ? "success" : "active"}
        style={{ marginBottom: 16 }}
      />

      {/* Info stats */}
      <div style={{ display: "flex", gap: 24, marginBottom: 16, flexWrap: "wrap" }}>
        <Statistic title="当前步数" value={currentStep} valueStyle={{ fontSize: 16 }} />
        <Statistic title="损失" value={currentMetrics?.loss?.toFixed(4) ?? "-"} valueStyle={{ fontSize: 16 }} />
        <Statistic title="奖励" value={currentMetrics?.reward?.toFixed(4) ?? "-"} valueStyle={{ fontSize: 16 }} />
        <Statistic title="速度" value={`${stepsPerSec.toFixed(1)} steps/s`} valueStyle={{ fontSize: 16 }} />
      </div>

      {/* Loss chart (3 lines) */}
      <Text strong style={{ display: "block", marginBottom: 4 }}>损失曲线</Text>
      <MiniLineChart data={chartData.loss} color="#1890ff" />

      {/* Reward chart */}
      <Text strong style={{ display: "block", marginBottom: 4, marginTop: 12 }}>奖励曲线</Text>
      <MiniLineChart data={chartData.reward} color="#52c41a" />
    </div>
  );
}
