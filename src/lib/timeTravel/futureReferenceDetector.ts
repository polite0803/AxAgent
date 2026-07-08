/**
 * 3 阶段 LLM 未来引用检测
 *
 * spec §6.2: 扫描所有 agent 输出中的日期字符串与未来指向短语,
 * 命中 ≥ 1 处 → 节点输出 `violations: [...]` 列表。
 *
 * 3 阶段:
 *   1. 阶段 A:绝对日期(20YY-MM-DD 格式)→ 与 as_of 比较
 *   2. 阶段 B:相对时态短语("tomorrow"、"next week" 等)→ 视为违规
 *   3. 阶段 C:模糊未来指向("soon"、"later"、"future" 等)→ 视为违规
 *
 * 注意:
 *   - 全部小写匹配,只对"agent output 文本"做检测
 *   - 不阻断流程(spec §6.2 末段:不阻断避免 false positive 致 workflow 失败)
 *   - 同 snippet 只返回一次(去重)
 */
export type FutureReferenceRule = "absolute-date" | "tense-phrase" | "vague-future";

export interface FutureReferenceHit {
  snippet: string;
  ruleHit: FutureReferenceRule;
}

const ABSOLUTE_DATE_RE = /\b(20\d{2})-(\d{2})-(\d{2})\b/g;

/**
 * spec §6.2 阶段 B:相对时态短语
 * 中英文全覆盖。
 */
const TENSE_PHRASES: string[] = [
  // 英文
  "tomorrow",
  "next quarter",
  "next month",
  "next week",
  "next year",
  "in the future",
  "later this year",
  "by year end",
  "by year-end",
  "in the coming",
  "down the road",
  "ahead of",
  // 中文 — 明确未来时态
  "明天",
  "下周",
  "下月",
  "下个季度",
  "下个季",
  "下季度",
  "明年",
  "下一年",
  "未来几天",
  "未来几周",
  "未来几个月",
  "未来几个季度",
  "未来几年",
  "即将",
  "将要",
  "会涨到",
  "会跌到",
  "有望突破",
  "预期将",
  "预计将",
  "有望在",
  "短期内",
  "中长期",
];

/**
 * spec §6.2 阶段 C:模糊未来指向
 * 中英文全覆盖。
 */
const VAGUE_FUTURE_PHRASES: string[] = [
  // 英文
  "soon",
  "later",
  "future",
  "upcoming",
  "shortly",
  "eventually",
  "in time",
  "in days ahead",
  "in weeks ahead",
  "in months ahead",
  // 中文 — 模糊未来指向
  "未来",
  "随后",
  "随后将",
  "后期",
  "后续",
  "接下来",
  "下一步",
  "展望",
  "前景",
  "趋势向好",
  "值得期待",
  "看涨",
  "看跌",
  "反弹",
  "回调",
  "突破",
];

/**
 * 解析 YYYY-MM-DD 字符串为可比较的时间戳(失败返回 null)。
 */
function parseAsOf(asOf: string): number | null {
  if (!asOf) { return null; }
  const m = asOf.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (!m) { return null; }
  const ts = Date.parse(`${m[1]}-${m[2]}-${m[3]}T00:00:00Z`);
  return Number.isNaN(ts) ? null : ts;
}

/**
 * 检测文本中所有"未来引用"。
 *
 * @param text   agent 原始输出
 * @param asOf   当前回放的截止日(YYYY-MM-DD);为 null 时不检测阶段 A,
 *               仍可命中 B/C(但 B/C 仅在 as_of 模式下激活,见 `enabled` 语义)
 * @returns 命中片段(去重,按出现顺序)
 */
export function detectFutureReferences(
  text: string,
  asOf: string | null,
): FutureReferenceHit[] {
  if (!text || text.length === 0) { return []; }
  const out: FutureReferenceHit[] = [];
  const seen = new Set<string>();

  // ── 阶段 A:绝对日期(> as_of) ──
  if (asOf) {
    const asOfTs = parseAsOf(asOf);
    if (asOfTs != null) {
      const matches = text.matchAll(ABSOLUTE_DATE_RE);
      for (const m of matches) {
        const snippet = m[0];
        const ts = Date.parse(`${m[1]}-${m[2]}-${m[3]}T00:00:00Z`);
        if (!Number.isNaN(ts) && ts > asOfTs) {
          if (!seen.has(snippet)) {
            seen.add(snippet);
            out.push({ snippet, ruleHit: "absolute-date" });
          }
        }
      }
    }
  }

  // 阶段 B / C 仅在 as-of 模式下激活
  if (!asOf) { return out; }

  // ── 阶段 B:相对时态短语 ──
  const lower = text.toLowerCase();
  for (const phrase of TENSE_PHRASES) {
    let from = 0;
    while (true) {
      const idx = lower.indexOf(phrase, from);
      if (idx < 0) { break; }
      if (!seen.has(phrase)) {
        seen.add(phrase);
        out.push({ snippet: phrase, ruleHit: "tense-phrase" });
      }
      from = idx + phrase.length;
    }
  }

  // ── 阶段 C:模糊未来指向 ──
  for (const phrase of VAGUE_FUTURE_PHRASES) {
    if (lower.includes(phrase) && !seen.has(phrase)) {
      seen.add(phrase);
      out.push({ snippet: phrase, ruleHit: "vague-future" });
    }
  }

  return out;
}

/**
 * 为整棵树(nodeId + summary)产出 violations[]。
 * spec §6.2:`violations: [{ nodeId, snippet, ruleHit }, ...]`
 */
export function detectFutureReferencesForNode(
  nodeId: string,
  summary: string,
  asOf: string | null,
): Array<{ nodeId: string; snippet: string; ruleHit: string }> {
  return detectFutureReferences(summary, asOf).map((h) => ({
    nodeId,
    snippet: h.snippet,
    ruleHit: h.ruleHit,
  }));
}
