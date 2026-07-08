import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { Tag, Tooltip } from "antd";
import { Clock } from "lucide-react";
import { useTranslation } from "react-i18next";

/**
 * ReplayBadge — L3 面板级 chip
 *
 * 视觉规则（参考 design spec §10.1 HCI 4 层视觉信号）：
 *   - live 模式 → 不渲染（null）
 *   - replay → 橙色 chip + 时钟 icon + as_of_date
 *   - backtest_sweep → 蓝色 chip + 闪烁动画 + "Sweep"
 *
 * 用途：嵌入 DecisionTimelinePanel、RecommendationPanel、BacktestPanel 等
 * 任何与数据相关的面板，让用户立即看到"我现在在 replay 模式"。
 */
export function ReplayBadge({ style }: { style?: React.CSSProperties }) {
  const { t } = useTranslation();
  const mode = useTimeAnchorStore((s) => s.mode);
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);

  if (mode === "live") { return null; }

  const isSweep = mode === "backtest_sweep";
  const color = isSweep ? "blue" : "orange";
  const label = isSweep
    ? t("timeTravel.replayBadge.sweep")
    : t("timeTravel.replayBadge.replay", { date: asOfDate ?? "" });

  return (
    <Tooltip title={t("timeTravel.replayBadge.tooltip", { date: asOfDate ?? "" })}>
      <Tag
        color={color}
        icon={<Clock size={11} />}
        style={{
          fontWeight: 600,
          fontSize: 11,
          padding: "0 6px",
          margin: 0,
          animation: isSweep ? "ax-pulse 1.8s ease-in-out infinite" : undefined,
          ...style,
        }}
        data-testid="replay-badge"
      >
        {label}
      </Tag>
    </Tooltip>
  );
}

/**
 * ReplayWatermark — L4 数据水印
 *
 * 用 position:absolute 覆盖在 K 线图/分析报告之上，
 * 用极淡的文字 + 旋转 20° 的形式表示"此数据基于历史快照"。
 */
export function ReplayWatermark() {
  const mode = useTimeAnchorStore((s) => s.mode);
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  if (mode === "live" || !asOfDate) { return null; }
  return (
    <div
      data-testid="replay-watermark"
      aria-hidden
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
        overflow: "hidden",
        zIndex: 1,
      }}
    >
      <div
        style={{
          position: "absolute",
          top: "40%",
          left: "20%",
          fontSize: 64,
          fontWeight: 800,
          color: "rgba(245, 158, 11, 0.08)",
          transform: "rotate(-20deg)",
          userSelect: "none",
        }}
      >
        {`as of ${asOfDate}`}
      </div>
    </div>
  );
}
