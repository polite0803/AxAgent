import { SyncOutlined } from "@ant-design/icons";
import { theme } from "antd";
import type { GlobalToken } from "antd/es/theme/interface";
import { Brain, CheckCircle2, ChevronDown, ChevronRight, ListChecks, Play, Search } from "lucide-react";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

// ── Types ────────────────────────────────────────────────────────────────

export type ThinkingPhase =
  | "analysis"
  | "planning"
  | "execution"
  | "verification";

interface ThinkingBlock {
  phase: ThinkingPhase;
  title: string;
  content: string;
}

interface StructuredThinkingProps {
  thinking: string;
  isStreaming: boolean;
  totalMs?: number | null;
  onExpandChange?: (expanded: boolean) => void;
}

// ── Phase detection patterns ─────────────────────────────────────────────

const PHASE_PATTERNS: { phase: ThinkingPhase; keywords: string[] }[] = [
  {
    phase: "analysis",
    keywords: [
      "分析",
      "理解",
      "了解",
      "查看",
      "检查一下",
      "让我看看",
      "先看",
      "当前",
      "现状",
      "现有的",
      "代码库",
      "项目结构",
      "目录结构",
      "让我读",
      "我先看",
      "先了解",
      "探索",
      "搜索",
      "查找",
      "analyze",
      "understand",
      "look at",
      "check",
      "examine",
      "explore",
      "search",
      "find",
      "read",
      "inspect",
    ],
  },
  {
    phase: "planning",
    keywords: [
      "计划",
      "方案",
      "步骤",
      "思路",
      "策略",
      "规划",
      "设计",
      "首先",
      "然后",
      "接着",
      "最后",
      "第.*步",
      "我会",
      "我将",
      "需要做",
      "接下来",
      "plan",
      "strategy",
      "approach",
      "steps",
      "first",
      "then",
      "I will",
      "I'll",
      "need to",
      "going to",
    ],
  },
  {
    phase: "execution",
    keywords: [
      "执行",
      "实现",
      "编写",
      "创建",
      "修改",
      "删除",
      "运行",
      "编辑",
      "写入",
      "调用",
      "构建",
      "生成",
      "开始",
      "execute",
      "implement",
      "create",
      "write",
      "modify",
      "delete",
      "run",
      "edit",
      "call",
      "build",
      "generate",
      "start",
    ],
  },
  {
    phase: "verification",
    keywords: [
      "验证",
      "检查",
      "确认",
      "测试",
      "确保",
      "通过",
      "完成",
      "正确",
      "无误",
      "成功",
      "结果",
      "总结",
      "verify",
      "check",
      "confirm",
      "test",
      "ensure",
      "done",
      "complete",
      "correct",
      "success",
      "result",
      "summary",
    ],
  },
];

function detectPhase(line: string): ThinkingPhase | null {
  const lower = line.toLowerCase().trim();
  // js-set-map-lookups: 子串匹配无法用 Set.has 替代
  for (const { phase, keywords } of PHASE_PATTERNS) {
    if (keywords.some((kw) => lower.includes(kw))) {
      return phase;
    }
  }
  return null;
}

function parseThinkingBlocks(
  thinking: string,
  t: (key: string) => string,
): ThinkingBlock[] {
  const lines = thinking.split("\n");
  const blocks: ThinkingBlock[] = [];
  let currentPhase: ThinkingPhase | null = null;
  let currentTitle = "";
  let currentLines: string[] = [];

  for (const line of lines) {
    const detected = detectPhase(line);

    if (detected && detected !== currentPhase) {
      // Save current block
      if (currentLines.length > 0 && currentPhase) {
        blocks.push({
          phase: currentPhase,
          title: currentTitle || getDefaultTitle(currentPhase, t),
          content: currentLines.join("\n").trim(),
        });
      }

      // Start new block
      currentPhase = detected;
      currentTitle = line
        .trim()
        .replace(/^[#*\-\d.]+\s*/, "")
        .slice(0, 60);
      currentLines = [line];
    } else {
      if (!currentPhase) {
        // First block, default to analysis
        currentPhase = "analysis";
        currentTitle = getDefaultTitle("analysis", t);
      }
      currentLines.push(line);
    }
  }

  // Save last block
  if (currentLines.length > 0 && currentPhase) {
    blocks.push({
      phase: currentPhase,
      title: currentTitle || getDefaultTitle(currentPhase, t),
      content: currentLines.join("\n").trim(),
    });
  }

  // If no blocks detected, create a single "analysis" block
  if (blocks.length === 0 && thinking.trim()) {
    blocks.push({
      phase: "analysis",
      title: getDefaultTitle("analysis", t),
      content: thinking.trim(),
    });
  }

  // Merge adjacent same-phase blocks
  const merged: ThinkingBlock[] = [];
  for (const block of blocks) {
    const last = merged[merged.length - 1];
    if (last && last.phase === block.phase) {
      last.content += "\n" + block.content;
    } else {
      merged.push(block);
    }
  }

  return merged;
}

function getDefaultTitle(
  phase: ThinkingPhase,
  t: (key: string) => string,
): string {
  const titles: Record<ThinkingPhase, string> = {
    analysis: t("thinking.phase.analysis"),
    planning: t("thinking.phase.planning"),
    execution: t("thinking.phase.execution"),
    verification: t("thinking.phase.verification"),
  };
  return titles[phase];
}

const phaseIcons: Record<ThinkingPhase, React.ReactNode> = {
  analysis: <Search size={14} />,
  planning: <ListChecks size={14} />,
  execution: <Play size={14} />,
  verification: <CheckCircle2 size={14} />,
};

function getPhaseColors(token: GlobalToken): Record<
  ThinkingPhase,
  { bg: string; border: string; text: string }
> {
  return {
    analysis: { bg: token.colorPrimaryBg, border: token.colorPrimary, text: token.colorPrimary },
    planning: { bg: token.colorWarningBg, border: token.colorWarning, text: token.colorWarning },
    execution: { bg: token.colorSuccessBg, border: token.colorSuccess, text: token.colorSuccess },
    verification: {
      bg: "rgba(114,46,209,0.06)",
      border: "#722ed1",
      text: "#722ed1",
    },
  };
}

// ── Component ────────────────────────────────────────────────────────────

export const StructuredThinking = React.memo(function StructuredThinking({
  thinking,
  isStreaming,
  totalMs,
  onExpandChange,
}: StructuredThinkingProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [expanded, setExpanded] = useState(isStreaming);
  const [expandedBlocks, setExpandedBlocks] = useState<Set<number>>(new Set());

  useEffect(() => {
    setExpanded(isStreaming);
  }, [isStreaming]);

  useEffect(() => {
    if (!isStreaming) {
      // Auto-collapse after streaming done
      setExpanded(false);
    }
  }, [isStreaming]);

  const handleToggle = () => {
    const next = !expanded;
    setExpanded(next);
    onExpandChange?.(next);
  };

  const blocks = useMemo(() => parseThinkingBlocks(thinking, t), [thinking, t]);

  const toggleBlock = (idx: number) => {
    setExpandedBlocks((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) {
        next.delete(idx);
      } else {
        next.add(idx);
      }
      return next;
    });
  };

  const title = isStreaming
    ? t("chat.thinkingInProgress")
    : totalMs && !isNaN(totalMs)
    ? `${t("chat.thinkingComplete")} (${(totalMs / 1000).toFixed(1)}s)`
    : t("chat.thinkingComplete");

  return (
    <div
      style={{
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: token.borderRadius,
        marginBottom: 8,
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div
        onClick={handleToggle}
        role="button"
        tabIndex={0}
        aria-expanded={expanded}
        aria-label={title}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleToggle();
          }
        }}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          cursor: "pointer",
          userSelect: "none",
          backgroundColor: token.colorFillQuaternary,
          borderBottom: expanded
            ? `1px solid ${token.colorBorderSecondary}`
            : "none",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Brain size={14} style={{ color: token.colorPrimary }} />
          <span style={{ fontSize: 13, fontWeight: 500 }}>{title}</span>
          {isStreaming && (
            <SyncOutlined
              style={{
                fontSize: 12,
                color: token.colorPrimary,
                animation: "axagent-think-spin 1s linear infinite",
              }}
            />
          )}
          {!isStreaming && blocks.length > 0 && (
            <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
              {t("thinking.phaseCount", { count: blocks.length })}
            </span>
          )}
        </div>
        {expanded
          ? <ChevronDown size={14} style={{ color: token.colorTextSecondary }} />
          : <ChevronRight size={14} style={{ color: token.colorTextSecondary }} />}
      </div>

      {/* Phase blocks */}
      {expanded && (
        <div style={{ padding: "8px 12px" }}>
          {blocks.map((block, idx) => {
            const colors = getPhaseColors(token)[block.phase];
            const isBlockExpanded = expandedBlocks.has(idx) || blocks.length === 1;
            return (
              <div
                key={`${block.phase}-${block.title}`}
                style={{
                  marginBottom: idx < blocks.length - 1 ? 8 : 0,
                  border: `1px solid ${colors.border}20`,
                  borderLeft: `3px solid ${colors.border}`,
                  borderRadius: token.borderRadiusSM,
                  backgroundColor: colors.bg,
                  overflow: "hidden",
                }}
              >
                <div
                  onClick={() => toggleBlock(idx)}
                  role="button"
                  tabIndex={0}
                  aria-expanded={isBlockExpanded}
                  aria-label={block.title}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      toggleBlock(idx);
                    }
                  }}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "6px 10px",
                    cursor: "pointer",
                    userSelect: "none",
                  }}
                >
                  <div
                    style={{ display: "flex", alignItems: "center", gap: 6 }}
                  >
                    <span style={{ color: colors.text, display: "flex" }}>
                      {phaseIcons[block.phase]}
                    </span>
                    <span
                      style={{
                        fontSize: 12,
                        fontWeight: 500,
                        color: colors.text,
                      }}
                    >
                      {block.title}
                    </span>
                  </div>
                  {blocks.length > 1
                    && (isBlockExpanded
                      ? <ChevronDown size={12} style={{ color: colors.text }} />
                      : <ChevronRight size={12} style={{ color: colors.text }} />)}
                </div>
                {isBlockExpanded && (
                  <div
                    style={{
                      padding: "8px 10px",
                      fontSize: 12,
                      lineHeight: 1.7,
                      color: token.colorTextSecondary,
                      whiteSpace: "pre-wrap",
                      wordBreak: "break-word",
                      borderTop: `1px solid ${colors.border}15`,
                    }}
                  >
                    {block.content}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
});

export { parseThinkingBlocks };
