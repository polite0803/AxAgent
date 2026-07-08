/**
 * CompactRecommendation — RecommendationPanel 在 chat 中的紧凑版本
 * 输入:推荐响应 { period, picks: { style: Pick[] } }
 * 输出:Top 3 最高信心度股票 + 风格标签
 */
import { useMemo } from "react";

interface RecoPick {
  stockCode: string;
  stockName: string;
  style: string;
  confidence: number;
  price: number;
  targetPrice: number;
  /** true = 兜底合成 pick（系统初筛 / 数据稀疏），false = 主策略真实命中 */
  synthetic?: boolean;
}

interface RecoResponseShape {
  period?: string;
  picks?: Record<string, RecoPick[]>;
}

const STYLE_LABEL: Record<string, string> = {
  trend: "趋势",
  value: "价值",
  capital: "资金",
  reversion: "反转",
  serenity: "瓶颈",
};

const STYLE_COLOR: Record<string, string> = {
  trend: "blue",
  value: "gold",
  capital: "magenta",
  reversion: "green",
  serenity: "purple",
};

interface CompactRecommendationProps {
  data: RecoResponseShape | unknown;
}

function normalize(data: CompactRecommendationProps["data"]): RecoResponseShape {
  if (data && typeof data === "object") {
    return data as RecoResponseShape;
  }
  return {};
}

export function CompactRecommendation({ data }: CompactRecommendationProps) {
  const response = useMemo(() => normalize(data), [data]);
  const picks = useMemo(() => {
    if (!response.picks) { return []; }
    const all: RecoPick[] = [];
    for (const arr of Object.values(response.picks)) {
      if (Array.isArray(arr)) { all.push(...arr); }
    }
    return all.sort((a, b) => (b.confidence ?? 0) - (a.confidence ?? 0)).slice(0, 3);
  }, [response]);

  if (picks.length === 0) {
    return (
      <div className="text-[12px] italic" style={{ color: "var(--muted)" }}>
        暂无推荐
      </div>
    );
  }

  return (
    <div className="space-y-1 text-[12px]">
      <div className="flex items-baseline gap-2 flex-wrap">
        <span style={{ color: "var(--muted)" }}>Top {picks.length}</span>
        {response.period && (
          <span style={{ color: "var(--muted)" }}>
            {response.period === "short" ? "短线" : response.period === "mid" ? "中线" : "长线"}
          </span>
        )}
      </div>
      <div className="space-y-0.5">
        {picks.map((p, i) => {
          const styleLabel = STYLE_LABEL[p.style] ?? p.style;
          const styleColor = STYLE_COLOR[p.style] ?? "default";
          return (
            <div
              key={`${p.stockCode}-${i}`}
              className="flex items-center gap-1.5 text-[11px]"
            >
              <span
                className="px-1 rounded text-[9px] font-medium"
                style={{
                  background: `var(--sa-${styleColor}-bg, #dbeafe)`,
                  color: `var(--sa-${styleColor}, #2563eb)`,
                }}
              >
                {styleLabel}
              </span>
              <span className="font-mono text-[10px]" style={{ color: "var(--muted)" }}>
                {p.stockCode}
              </span>
              <span className="flex-1 truncate" style={{ color: "var(--color-text-secondary)" }}>
                {p.stockName}
              </span>
              {typeof p.confidence === "number" && (
                <span
                  className="text-[10px] font-mono shrink-0"
                  style={{ color: "var(--accent, #7c3aed)" }}
                >
                  {Math.round(p.confidence)}
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
