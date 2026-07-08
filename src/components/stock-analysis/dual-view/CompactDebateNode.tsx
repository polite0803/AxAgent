/**
 * CompactDebateNode — DebatePanel 在 chat 中的紧凑版本
 * 输入:debate 数据(多空双方观点)
 * 输出:情绪比例 + 1 条最强看多 + 1 条最强看空
 */
import { useMemo } from "react";

interface DebateRound {
  round: number;
  bull: string;
  bear: string;
}

interface CompactDebateNodeProps {
  data: DebateRound[] | { rounds: DebateRound[] } | unknown;
}

function normalizeRounds(data: CompactDebateNodeProps["data"]): DebateRound[] {
  if (Array.isArray(data)) { return data; }
  if (data && typeof data === "object" && "rounds" in data) {
    const r = (data as { rounds: DebateRound[] }).rounds;
    return Array.isArray(r) ? r : [];
  }
  return [];
}

interface ParsedClaim {
  claim: string;
  strength?: number;
  category?: string;
}

function tryExtractClaims(text: string, isBull: boolean): ParsedClaim | null {
  if (!text) { return null; }
  // 提取 ```json 块
  const m = text.match(/```(?:json)?\s*([\s\S]+?)```/);
  const candidate = m ? m[1].trim() : text.trim();
  let parsed: Record<string, unknown> | null = null;
  try {
    parsed = JSON.parse(candidate);
  } catch { /* not json */ }
  if (!parsed) { return null; }

  if (isBull) {
    const arr = parsed.core_arguments;
    if (Array.isArray(arr) && arr.length > 0) {
      // 找 strength 最高的论点
      const sorted = [...arr].sort((a, b) => (b?.strength ?? 0) - (a?.strength ?? 0));
      const top = sorted[0];
      return {
        claim: String(top?.claim ?? "").slice(0, 100),
        strength: typeof top?.strength === "number" ? top.strength : undefined,
        category: top?.category,
      };
    }
    const resonance = parsed.resonance_points;
    if (Array.isArray(resonance) && resonance.length > 0) {
      const top = resonance[0];
      return { claim: String(top?.point ?? "").slice(0, 100), strength: top?.weight };
    }
  } else {
    // bear: 共用同一 JSON 结构,看多看空都从 core_arguments 提取
    // 但 bear 角色通常也会输出 core_arguments(多空都列)
    const arr = parsed.core_arguments;
    if (Array.isArray(arr) && arr.length > 0) {
      const sorted = [...arr].sort((a, b) => (b?.strength ?? 0) - (a?.strength ?? 0));
      const top = sorted[0];
      return {
        claim: String(top?.claim ?? "").slice(0, 100),
        strength: typeof top?.strength === "number" ? top.strength : undefined,
        category: top?.category,
      };
    }
  }
  return null;
}

export function CompactDebateNode({ data }: CompactDebateNodeProps) {
  const rounds = useMemo(() => normalizeRounds(data), [data]);

  const summary = useMemo(() => {
    if (rounds.length === 0) { return null; }
    // 情绪:对比所有 round 的 bull/bear 文本长度
    const bullLen = rounds.reduce((acc, r) => acc + (r.bull?.length ?? 0), 0);
    const bearLen = rounds.reduce((acc, r) => acc + (r.bear?.length ?? 0), 0);
    const total = bullLen + bearLen;
    const ratio = total > 0 ? bullLen / total : 0.5;

    // 取最新一轮的最强论点
    const lastRound = rounds[rounds.length - 1];
    const bullTop = tryExtractClaims(lastRound?.bull ?? "", true);
    const bearTop = tryExtractClaims(lastRound?.bear ?? "", false);

    return { ratio, bullTop, bearTop, totalRounds: rounds.length };
  }, [rounds]);

  if (!summary) {
    return (
      <div className="text-[12px] italic" style={{ color: "var(--muted)" }}>
        暂无辩论数据
      </div>
    );
  }

  const pct = Math.round(summary.ratio * 100);
  const sentiment = summary.ratio > 0.55 ? "看多" : summary.ratio < 0.45 ? "看空" : "中性";

  return (
    <div className="space-y-1 text-[12px]">
      <div className="flex items-baseline gap-2 flex-wrap">
        <span
          className="px-1.5 py-0.5 rounded text-[10px] font-medium"
          style={{
            background: summary.ratio > 0.55
              ? "var(--sa-red-bg, #fee2e2)"
              : summary.ratio < 0.45
              ? "var(--sa-green-bg, #dcfce7)"
              : "var(--muted-bg, #e5e7eb)",
            color: summary.ratio > 0.55
              ? "var(--sa-red, #dc2626)"
              : summary.ratio < 0.45
              ? "var(--sa-green, #16a34a)"
              : "var(--muted, #6b7280)",
          }}
        >
          {sentiment} · {pct}%
        </span>
        <span style={{ color: "var(--muted)" }}>{summary.totalRounds} 轮</span>
      </div>

      {summary.bullTop?.claim && (
        <div className="text-[11px] leading-snug flex gap-1">
          <span style={{ color: "var(--sa-red, #dc2626)", fontWeight: 600 }}>多:</span>
          <span style={{ color: "var(--color-text-secondary)" }}>{summary.bullTop.claim}</span>
        </div>
      )}
      {summary.bearTop?.claim && (
        <div className="text-[11px] leading-snug flex gap-1">
          <span style={{ color: "var(--sa-green, #16a34a)", fontWeight: 600 }}>空:</span>
          <span style={{ color: "var(--color-text-secondary)" }}>{summary.bearTop.claim}</span>
        </div>
      )}
    </div>
  );
}
