import { formatTokenCount } from "@/components/gateway/tokenFormat";
import { Space, theme, Tooltip } from "antd";
import { useTranslation } from "react-i18next";

export interface ContextSegment {
  key: string;
  labelKey: string;
  tokens: number;
  color: string;
}

export interface ContextClassificationBarProps {
  segments: ContextSegment[];
  maxTokens?: number;
}

/**
 * Context classification occupancy indicator.
 *
 * Renders a compact horizontal segmented bar showing per-category token
 * occupancy (Messages, System Prompt, Knowledge, Memory, Tools, Skills),
 * similar to Claude Code's context breakdown.
 *
 * Each segment is color-coded and proportional to its token share.
 * Hover for detailed tooltip with exact token count and percentage.
 */
export function ContextClassificationBar({
  segments,
  maxTokens,
}: ContextClassificationBarProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const visibleSegments = segments.filter((s) => s.tokens > 0);
  if (visibleSegments.length === 0) {
    return null;
  }

  const totalTokens = visibleSegments.reduce((sum, s) => sum + s.tokens, 0);
  const max = maxTokens ?? totalTokens;
  const overallRatio = max > 0 ? totalTokens / max : 0;
  const overallPercent = Math.min(100, Math.round(overallRatio * 100));

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "4px 16px 6px",
        borderBottom: "1px solid var(--border-color)",
        backgroundColor: token.colorBgContainer,
        overflowX: "auto",
        opacity: overallPercent < 5 ? 0.5 : 1,
      }}
    >
      {/* Segmented bar */}
      <div
        style={{
          display: "flex",
          height: 8,
          borderRadius: 4,
          overflow: "hidden",
          flex: 1,
          minWidth: 120,
          backgroundColor: token.colorFillSecondary,
        }}
      >
        {visibleSegments.map((seg) => {
          const width = totalTokens > 0 ? (seg.tokens / totalTokens) * 100 : 0;
          return (
            <Tooltip
              key={seg.key}
              title={`${t(seg.labelKey)}: ${seg.tokens.toLocaleString()} tokens (${
                totalTokens > 0
                  ? ((seg.tokens / totalTokens) * 100).toFixed(1)
                  : 0
              }%)`}
            >
              <div
                style={{
                  width: `${Math.max(width, 2)}%`,
                  height: "100%",
                  backgroundColor: seg.color,
                  cursor: "pointer",
                  transition: "width 0.3s ease",
                  flexShrink: 0,
                }}
              />
            </Tooltip>
          );
        })}
      </div>

      {/* Labels + Token counts */}
      <Space size={[6, 2]} wrap style={{ fontSize: 12, lineHeight: "16px" }}>
        {visibleSegments.map((seg, i) => {
          const pct = totalTokens > 0 ? (seg.tokens / totalTokens) * 100 : 0;
          return (
            <Tooltip
              key={seg.key}
              title={`${t(seg.labelKey)}: ${seg.tokens.toLocaleString()} tokens (${pct.toFixed(1)}%)`}
            >
              <span
                style={{
                  color: token.colorTextSecondary,
                  whiteSpace: "nowrap",
                  cursor: "default",
                }}
              >
                <span
                  style={{
                    display: "inline-block",
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    backgroundColor: seg.color,
                    marginRight: 3,
                    verticalAlign: "middle",
                  }}
                />
                {t(seg.labelKey)}{" "}
                <span style={{ fontWeight: 500, color: token.colorText }}>
                  {formatTokenCount(seg.tokens)}
                </span>
                {i < visibleSegments.length - 1 && (
                  <span
                    style={{ color: token.colorBorderSecondary, marginLeft: 6 }}
                  >
                    |
                  </span>
                )}
              </span>
            </Tooltip>
          );
        })}
      </Space>

      {/* Overall percentage */}
      {maxTokens != null && maxTokens > 0 && (
        <span
          style={{
            fontSize: 12,
            fontWeight: 500,
            color: overallRatio > 0.95
              ? token.colorError
              : overallRatio > 0.8
              ? token.colorWarning
              : token.colorTextSecondary,
            whiteSpace: "nowrap",
            flexShrink: 0,
          }}
        >
          {overallPercent}%
        </span>
      )}
    </div>
  );
}
