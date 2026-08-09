import { invoke } from "@/lib/invoke";
import { CheckCircleFilled, CloseCircleFilled, ExclamationCircleFilled, ThunderboltFilled } from "@ant-design/icons";
import { Button, Col, Modal, Progress, Row, Table, Tag, Tooltip, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

// ── 单个检查项目 ──────────────────────────────────────────────
interface QualityCheck {
  category: string; // i18n key suffix
  field: string; // 字段名
  status: "good" | "warning" | "issue";
  detail: string; // 人类可读详情
}

const STATUS_ICON: Record<string, React.ReactNode> = {
  good: <CheckCircleFilled style={{ color: "#52c41a" }} />,
  warning: <ExclamationCircleFilled style={{ color: "#faad14" }} />,
  issue: <CloseCircleFilled style={{ color: "#f5222d" }} />,
};

// ── 节点类型检测 ──────────────────────────────────────────────
type NodeType = "analyst" | "debate" | "decision" | "tool" | "valuation" | "risk" | "other";

/** 根据 expertId/nodeId 推断节点类型 */
function detectNodeType(nodeId: string): NodeType {
  if (nodeId.startsWith("a-")) { return "analyst"; }
  if (nodeId.startsWith("bull-") || nodeId.startsWith("bear-")) { return "debate"; }
  if (nodeId.includes("decision") || nodeId.includes("manager")) { return "decision"; }
  if (nodeId.startsWith("t-") || nodeId.startsWith("u-")) { return "tool"; }
  if (nodeId.includes("valuation")) { return "valuation"; }
  if (nodeId.includes("risk")) { return "risk"; }
  return "other";
}

/** 获取节点类型的中文名称 */
function getNodeTypeName(nodeType: NodeType): string {
  const names: Record<NodeType, string> = {
    analyst: "分析师",
    debate: "辩论节点",
    decision: "决策节点",
    tool: "工具/算法节点",
    valuation: "估值节点",
    risk: "风险节点",
    other: "通用节点",
  };
  return names[nodeType];
}

// ── 分析师类型 → 预期 VERDICT 字段 Schema ──────────────────
// 不同分析师输出的 VERDICT JSON 字段不同，定义按类型的预期核心字段集。
// 不在核心集内的字段视为扩展字段（不算缺失）。
interface VerdictFieldSchema {
  name: string;
  type: "string" | "number";
}

/** 获取指定 expertId 的预期 VERDICT 核心字段 */
function getExpectedVerdictFields(expertId: string): VerdictFieldSchema[] {
  // 10 位标准分析师：verdict / bull_score / bear_score / confidence
  const STANDARD_ANALYST_PREFIXES = [
    "a-market-analyst",
    "a-sentiment",
    "a-news",
    "a-fundamentals",
    "a-policy",
    "a-hot-money",
    "a-lockup",
    "a-research",
    "a-sector",
    "a-catalyst",
  ];
  const isStandard = STANDARD_ANALYST_PREFIXES.some((p) => expertId.startsWith(p));

  if (isStandard) {
    return [
      { name: "verdict", type: "string" },
      { name: "bull_score", type: "number" },
      { name: "bear_score", type: "number" },
      { name: "confidence", type: "number" },
    ];
  }

  // 辩论节点：stance / strength_score / confidence
  if (expertId.startsWith("bull") || expertId.startsWith("bear")) {
    return [
      { name: "stance", type: "string" },
      { name: "strength_score", type: "number" },
      { name: "confidence", type: "number" },
    ];
  }

  // 其他（value-investor, data-quality-inspector 等）：通用兜底
  return [
    { name: "verdict", type: "string" },
    { name: "bull_score", type: "number" },
    { name: "bear_score", type: "number" },
    { name: "confidence", type: "number" },
  ];
}

// ── 数据质量分析器 ────────────────────────────────────────────
interface ParsedReport {
  type?: string;
  summary?: string;
  signals?: string[];
  risk_flags?: string[];
  argument?: string;
  key_points?: string[];
  confidence?: number;
  core_arguments?: string[];
  resonance_points?: string[];
  preempted_counter_attacks?: string[];
  bull_strength_score?: number;
  bear_strength_score?: number;
  data_gaps?: string[];
  main_flow_state?: string;
  active_player?: string;
  dragon_tiger_signal?: string;
  limit_up_sustainability?: string;
  bull_score?: number;
  bear_score?: number;
  trigger_bull?: string;
  trigger_bear?: string;
  evidence?: Array<{ point?: string; data?: string; weight?: number }>;
  expert?: string;
  business_model?: string;
  moat_rating?: string;
  moat_reasoning?: string;
  financial_health?: string;
  intrinsic_value_range?: string | null;
  margin_of_safety?: string;
  buffett_verdict?: string;
  ideal_buy_price?: string | null;
  catalyst_detail?: string;
  catalyst_level?: string;
  narrative_completeness?: string;
  narrative_missing?: string[];
  institutional_trace?: string;
  concept_risk?: string;
  key_events?: Array<{ event?: string; source?: string; stance?: string; weight?: number }>;
  analysis?: string;
  assessment?: string;
  verdict?: string;
  reasoning?: string;
  stance?: string;
  positionPct?: number;
  action?: string;
  [key: string]: unknown;
}

interface QualityResult {
  score: number;
  grade: "A" | "B" | "C" | "D" | "F";
  checks: QualityCheck[];
  goodCount: number;
  warningCount: number;
  issueCount: number;
}

function analyzeDataQuality(parsed: ParsedReport | null, report: string, expertId: string): QualityResult {
  const checks: QualityCheck[] = [];
  let score = 100;
  let goodCount = 0;
  let warningCount = 0;
  let issueCount = 0;

  if (!parsed) {
    // 无解析结果 → 检查原始文本是否包含有效 JSON
    const hasJson = report.trim().startsWith("{") || report.trim().startsWith("[");
    checks.push({
      category: "dataQualityOverall",
      field: "parsed_json",
      status: "issue",
      detail: hasJson ? "JSON 格式存在但前端解析失败" : "非结构化文本，无法执行结构化数据质量分析",
    });
    issueCount++;
    score = hasJson ? 30 : 10;
    // 不进一步分析
    return { score, grade: scoreToGrade(score), checks, goodCount, warningCount, issueCount };
  }

  // ── 0. VERDICT 格式分析（原始报告字符串级）─────────────
  // AnalystReportCard 有两种解析路径：
  //   A) <!-- VERDICT: {...} --> 格式：自由文本 + HTML 注释包裹的 JSON verdict 标签
  //   B) 纯 JSON 格式：整段纯 JSON 字符串
  // 这里从原始 report 字符串层面检测 VERDICT 格式质量
  const verdictTagIdx = report.indexOf("<!-- VERDICT:");
  if (verdictTagIdx !== -1) {
    // 有 VERDICT 标签
    const jsonPart = report.slice(verdictTagIdx + "<!-- VERDICT:".length);
    const jsonEnd = jsonPart.indexOf("-->");
    if (jsonEnd === -1) {
      checks.push({
        category: "dataQualityFieldCompleteness",
        field: "VERDICT 格式",
        status: "issue",
        detail: "VERDICT 标签存在但缺少 --> 闭合标记，JSON 不完整",
      });
      issueCount++;
      score -= 15;
    } else {
      // JSON 部分基本完整，尝试解析
      const jsonStr = jsonPart.slice(0, jsonEnd).trim();
      try {
        const verdictData = JSON.parse(jsonStr);
        checks.push({
          category: "dataQualityFieldCompleteness",
          field: "VERDICT JSON",
          status: "good",
          detail: "VERDICT 标签格式完整，JSON 解析成功",
        });
        goodCount++;

        // 逐字段列出预期字段的存在/缺失（每个字段一行）
        const expectedFields = getExpectedVerdictFields(expertId);
        const expectedNames = expectedFields.map((f) => f.name);
        const verdictKeys = Object.keys(verdictData);
        const extraVerdictKeys = verdictKeys.filter((k) => !expectedNames.includes(k));

        for (const f of expectedFields) {
          const isPresent = f.name in verdictData;
          if (isPresent) {
            const rawVal = (verdictData as Record<string, unknown>)[f.name];
            const valStr = typeof rawVal === "string" ? `"${rawVal}"` : String(rawVal);
            checks.push({
              category: "dataQualityFieldCompleteness",
              field: `VERDICT.${f.name}`,
              status: "good",
              detail: `${valStr}`,
            });
            goodCount++;
          } else {
            checks.push({
              category: "dataQualityFieldCompleteness",
              field: `VERDICT.${f.name}`,
              status: "issue",
              detail: "字段缺失",
            });
            issueCount++;
            score -= 8;
          }
        }
        if (extraVerdictKeys.length > 0) {
          checks.push({
            category: "dataQualityFieldCompleteness",
            field: "VERDICT 扩展字段",
            status: "good",
            detail: `VERDICT 含 ${extraVerdictKeys.length} 个扩展字段：${extraVerdictKeys.join("、")}`,
          });
          goodCount++;
        }

        // VERDICT 值类型检查（按 analyst type 动态）
        const typeIssues: string[] = [];
        for (const f of expectedFields) {
          if (f.name in verdictData) {
            const actualType = typeof (verdictData as Record<string, unknown>)[f.name];
            if (actualType !== f.type) {
              typeIssues.push(`${f.name} 应为 ${f.type}，实际为 ${actualType}`);
            }
          }
        }
        if (typeIssues.length > 0) {
          checks.push({
            category: "dataQualityValueQuality",
            field: "VERDICT 类型",
            status: "warning",
            detail: typeIssues.join("；"),
          });
          warningCount++;
          score -= 3;
        } else {
          checks.push({
            category: "dataQualityValueQuality",
            field: "VERDICT 类型",
            status: "good",
            detail: "VERDICT 各字段类型正确",
          });
          goodCount++;
        }
      } catch {
        checks.push({
          category: "dataQualityFieldCompleteness",
          field: "VERDICT JSON",
          status: "issue",
          detail: "VERDICT 标签内的 JSON 解析失败，可能含格式错误",
        });
        issueCount++;
        score -= 10;
      }

      // 检查自由文本 vs VERDICT 数据量比例
      const freeTextLen = report.slice(0, verdictTagIdx).trim().length;
      if (freeTextLen > 50) {
        checks.push({
          category: "dataQualityContentQuality",
          field: "VERDICT 文本",
          status: "good",
          detail: `VERDICT 前自由文本 ${freeTextLen} 字符，分析内容充实`,
        });
        goodCount++;
      } else if (freeTextLen > 10) {
        checks.push({
          category: "dataQualityContentQuality",
          field: "VERDICT 文本",
          status: "warning",
          detail: `VERDICT 前自由文本仅 ${freeTextLen} 字符，分析偏简略`,
        });
        warningCount++;
        score -= 3;
      } else {
        checks.push({
          category: "dataQualityContentQuality",
          field: "VERDICT 文本",
          status: "warning",
          detail: "VERDICT 前无实质自由文本，纯 JSON 输出",
        });
        warningCount++;
        score -= 3;
      }
    }
  } else {
    // 无 VERDICT 标签 → 纯 JSON 格式
    const isJson = report.trim().startsWith("{") || report.trim().startsWith("[");
    if (isJson) {
      checks.push({
        category: "dataQualityFieldCompleteness",
        field: "输出格式",
        status: "good",
        detail: "纯 JSON 格式（非 VERDICT 标签）",
      });
      goodCount++;
    } else {
      checks.push({
        category: "dataQualityFieldCompleteness",
        field: "输出格式",
        status: "warning",
        detail: "非标准格式（既非 VERDICT 标签也非纯 JSON）",
      });
      warningCount++;
      score -= 3;
    }
  }

  // ── 1. 字段完整性 ─────────────────────────────────────────
  const hasSummary = !!(parsed.summary || parsed.analysis || parsed.argument || parsed.assessment
    || parsed.report || parsed.reasoning || parsed.buffett_verdict || parsed.catalyst_detail);
  const hasConfidence = typeof parsed.confidence === "number";
  const hasVerdict = !!(parsed.verdict || parsed.stance);
  const hasBullScore = typeof parsed.bull_score === "number";
  const hasBearScore = typeof parsed.bear_score === "number";

  // 关键字段检查
  if (!hasSummary) {
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "summary/analysis",
      status: "issue",
      detail: "无摘要或分析文本",
    });
    issueCount++;
    score -= 15;
  } else if ((parsed.summary?.length ?? 0) > 0) {
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "summary",
      status: "good",
      detail: `摘要存在（${(parsed.summary?.length ?? 0)} 字符）`,
    });
    goodCount++;
  }

  if (!hasConfidence) {
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "confidence",
      status: "warning",
      detail: "缺失置信度评分",
    });
    warningCount++;
    score -= 5;
  } else {
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "confidence",
      status: "good",
      detail: `置信度=${parsed.confidence}`,
    });
    goodCount++;
  }

  if (!hasVerdict) {
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "verdict/stance",
      status: "warning",
      detail: "缺失看多/看空判断",
    });
    warningCount++;
    score -= 5;
  } else {
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "verdict/stance",
      status: "good",
      detail: `判断=${parsed.verdict ?? parsed.stance}`,
    });
    goodCount++;
  }

  // 额外字段：key_points / evidence 等
  const hasKeyPoints = !!(parsed.key_points?.length || parsed.core_arguments?.length
    || parsed.resonance_points?.length || parsed.evidence?.length || parsed.key_events?.length);
  if (!hasKeyPoints) {
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "key_points/evidence",
      status: "warning",
      detail: "缺失要点或证据列表",
    });
    warningCount++;
    score -= 5;
  } else {
    const ptCount = parsed.key_points?.length ?? parsed.core_arguments?.length ?? 0;
    checks.push({
      category: "dataQualityFieldCompleteness",
      field: "key_points/evidence",
      status: "good",
      detail: `存在 ${ptCount} 条要点`,
    });
    goodCount++;
  }

  // ── 2. 数值质量 ───────────────────────────────────────────
  if (hasBullScore) {
    const bs = parsed.bull_score!;
    if (bs >= 0 && bs <= 100) {
      checks.push({
        category: "dataQualityValueQuality",
        field: "bull_score",
        status: "good",
        detail: `看多评分=${bs}（合法范围 0-100）`,
      });
      goodCount++;
    } else {
      checks.push({
        category: "dataQualityValueQuality",
        field: "bull_score",
        status: "warning",
        detail: `看多评分=${bs}，超出常见范围`,
      });
      warningCount++;
      score -= 3;
    }
  } else {
    checks.push({ category: "dataQualityValueQuality", field: "bull_score", status: "warning", detail: "无看多评分" });
    warningCount++;
    score -= 3;
  }

  if (hasBearScore) {
    const bs = parsed.bear_score!;
    if (bs >= 0 && bs <= 100) {
      checks.push({
        category: "dataQualityValueQuality",
        field: "bear_score",
        status: "good",
        detail: `看空评分=${bs}（合法范围 0-100）`,
      });
      goodCount++;
    } else {
      checks.push({
        category: "dataQualityValueQuality",
        field: "bear_score",
        status: "warning",
        detail: `看空评分=${bs}，超出常见范围`,
      });
      warningCount++;
      score -= 3;
    }
  } else {
    checks.push({ category: "dataQualityValueQuality", field: "bear_score", status: "warning", detail: "无看空评分" });
    warningCount++;
    score -= 3;
  }

  // ── 2.1 评分自洽性检查（bull_score + bear_score 应约为 100） ──
  if (hasBullScore && hasBearScore) {
    const scoreSum = (parsed.bull_score || 0) + (parsed.bear_score || 0);
    if (scoreSum >= 90 && scoreSum <= 110) {
      checks.push({
        category: "dataQualityConsistency",
        field: "score_sum",
        status: "good",
        detail: `评分总和 ${scoreSum}，符合 100 基准（±10 容差）`,
      });
      goodCount++;
    } else {
      checks.push({
        category: "dataQualityConsistency",
        field: "score_sum",
        status: "issue",
        detail: `评分总和 ${scoreSum}，偏离 100 基准（±10 容差外），存在评分不自洽问题`,
      });
      issueCount++;
      score -= 10;
    }
  }

  // ── 2.2 confidence 范围检查 ──
  if (hasConfidence && parsed.confidence !== undefined) {
    const conf = parsed.confidence;
    if (conf < 0 || conf > 100) {
      checks.push({
        category: "dataQualityValueQuality",
        field: "confidence_range",
        status: "issue",
        detail: `置信度 ${conf} 超出合法范围 0-100`,
      });
      issueCount++;
      score -= 5;
    } else if (conf > 0 && conf < 10) {
      checks.push({
        category: "dataQualityValueQuality",
        field: "confidence_range",
        status: "warning",
        detail: `置信度 ${conf} 极低（< 10），数据可靠性存疑`,
      });
      warningCount++;
      score -= 3;
    }
  }

  // ── 3. 内容质量 ───────────────────────────────────────────
  const summaryText = parsed.summary || parsed.analysis || parsed.argument || "";
  if (summaryText.length > 100) {
    checks.push({
      category: "dataQualityContentQuality",
      field: "summary_length",
      status: "good",
      detail: `分析文本长度=${summaryText.length} 字符，内容充实`,
    });
    goodCount++;
  } else if (summaryText.length > 20) {
    checks.push({
      category: "dataQualityContentQuality",
      field: "summary_length",
      status: "warning",
      detail: `分析文本较短（${summaryText.length} 字符）`,
    });
    warningCount++;
    score -= 5;
  } else {
    checks.push({
      category: "dataQualityContentQuality",
      field: "summary_length",
      status: "issue",
      detail: "分析文本过短或无实质内容",
    });
    issueCount++;
    score -= 10;
  }

  // ── 4. 证据质量 ───────────────────────────────────────────
  if (parsed.evidence && parsed.evidence.length > 0) {
    const withData = parsed.evidence.filter((e) => e?.data && e.data.length > 0).length;
    const total = parsed.evidence.length;
    if (withData >= total / 2) {
      checks.push({
        category: "dataQualityEvidence",
        field: "evidence",
        status: "good",
        detail: `共 ${total} 条证据，${withData} 条含数据支撑`,
      });
      goodCount++;
    } else {
      checks.push({
        category: "dataQualityEvidence",
        field: "evidence",
        status: "warning",
        detail: `共 ${total} 条证据，仅 ${withData} 条含数据支撑`,
      });
      warningCount++;
      score -= 5;
    }
  } else if (parsed.key_events && parsed.key_events.length > 0) {
    checks.push({
      category: "dataQualityEvidence",
      field: "key_events",
      status: "good",
      detail: `共 ${parsed.key_events.length} 条关键事件`,
    });
    goodCount++;
  } else {
    // 没有 evidence/key_events 不一定有问题，部分分析师不用这个字段
  }

  // ── 5. 数据缺口 ───────────────────────────────────────────
  const hasDataGaps = parsed.data_gaps && parsed.data_gaps.length > 0;
  const hasNarrativeMissing = parsed.narrative_missing && parsed.narrative_missing.length > 0;
  if (hasDataGaps || hasNarrativeMissing) {
    const gapCount = (parsed.data_gaps?.length ?? 0) + (parsed.narrative_missing?.length ?? 0);
    let detail = `${gapCount} 个数据缺口`;
    if (parsed.data_gaps?.length) {
      detail += `：${parsed.data_gaps.slice(0, 3).join("、")}`;
      if (parsed.data_gaps.length > 3) { detail += "…"; }
    }
    checks.push({ category: "dataQualityDataGaps", field: "data_gaps", status: "warning", detail });
    warningCount++;
    score -= gapCount * 3;
  } else {
    checks.push({ category: "dataQualityDataGaps", field: "data_gaps", status: "good", detail: "无数据缺口" });
    goodCount++;
  }

  // ── 6. 逻辑一致性 ─────────────────────────────────────────
  if (hasVerdict && hasBullScore && hasBearScore) {
    const verdict = (parsed.verdict ?? parsed.stance ?? "").toLowerCase();
    const isBull = verdict.includes("看多") || verdict.includes("bull");
    const isBear = verdict.includes("看空") || verdict.includes("bear");
    if (isBull && parsed.bull_score! > parsed.bear_score!) {
      checks.push({
        category: "dataQualityConsistency",
        field: "verdict_vs_scores",
        status: "good",
        detail: "看多判断与评分一致",
      });
      goodCount++;
    } else if (isBear && parsed.bear_score! > parsed.bull_score!) {
      checks.push({
        category: "dataQualityConsistency",
        field: "verdict_vs_scores",
        status: "good",
        detail: "看空判断与评分一致",
      });
      goodCount++;
    } else if (isBull && parsed.bull_score! <= parsed.bear_score!) {
      checks.push({
        category: "dataQualityConsistency",
        field: "verdict_vs_scores",
        status: "warning",
        detail: `看多判断但 bull_score(${parsed.bull_score}) ≤ bear_score(${parsed.bear_score})`,
      });
      warningCount++;
      score -= 5;
    } else if (isBear && parsed.bear_score! <= parsed.bull_score!) {
      checks.push({
        category: "dataQualityConsistency",
        field: "verdict_vs_scores",
        status: "warning",
        detail: `看空判断但 bear_score(${parsed.bear_score}) ≤ bull_score(${parsed.bull_score})`,
      });
      warningCount++;
      score -= 5;
    } else {
      checks.push({
        category: "dataQualityConsistency",
        field: "verdict_vs_scores",
        status: "good",
        detail: "中性判断，无冲突",
      });
      goodCount++;
    }
  } else {
    checks.push({
      category: "dataQualityConsistency",
      field: "verdict_vs_scores",
      status: "warning",
      detail: "缺少 verdict 或评分，无法做一致性检查",
    });
    warningCount++;
    score -= 3;
  }

  // 裁剪分数到 0-100
  score = Math.max(0, Math.min(100, score));
  const grade = scoreToGrade(score);

  return { score, grade, checks, goodCount, warningCount, issueCount };
}

function scoreToGrade(score: number): "A" | "B" | "C" | "D" | "F" {
  if (score >= 90) { return "A"; }
  if (score >= 70) { return "B"; }
  if (score >= 50) { return "C"; }
  if (score >= 30) { return "D"; }
  return "F";
}

const GRADE_COLOR: Record<string, string> = {
  A: "#52c41a",
  B: "#73d13d",
  C: "#faad14",
  D: "#fa8c16",
  F: "#f5222d",
};

// ── 组件 ──────────────────────────────────────────────────
interface Props {
  name: string;
  expertId: string;
  parsed: object | null;
  report: string;
  open: boolean;
  onClose: () => void;
  /** 可选：股票代码，用于关联分析 */
  stockCode?: string;
  /** 可选：执行 ID，用于关联工作流执行 */
  executionId?: string;
}

export function AnalystDataQualityModal({
  name,
  expertId,
  parsed,
  report,
  open,
  onClose,
  stockCode = "",
  executionId = "",
}: Props) {
  const { t } = useTranslation();
  const result = analyzeDataQuality(parsed as ParsedReport | null, report, expertId);

  // 检测节点类型
  const nodeType = detectNodeType(expertId);
  const nodeTypeName = getNodeTypeName(nodeType);

  // 当 Modal 打开且有解析结果时，自动上报反馈给后端用于自我进化
  useEffect(() => {
    if (!open || !parsed) { return; }

    const p = parsed as ParsedReport;
    const checksJson = JSON.stringify(result.checks);

    // 构建通用质量指标
    const qualityMetrics: Record<string, unknown> = {};

    if (nodeType === "analyst") {
      // 分析师指标
      const scoreSum = (p.bull_score || 0) + (p.bear_score || 0);
      qualityMetrics.bull_score = p.bull_score ?? null;
      qualityMetrics.bear_score = p.bear_score ?? null;
      qualityMetrics.confidence = p.confidence ?? null;
      qualityMetrics.score_consistent = scoreSum >= 90 && scoreSum <= 110;

      const verdict = (p.verdict ?? p.stance ?? "").toLowerCase();
      const isBull = verdict.includes("看多") || verdict.includes("bull");
      const isBear = verdict.includes("看空") || verdict.includes("bear");
      let directionConsistent = true;
      if (isBull && p.bull_score !== undefined && p.bear_score !== undefined) {
        directionConsistent = p.bull_score > p.bear_score;
      } else if (isBear && p.bull_score !== undefined && p.bear_score !== undefined) {
        directionConsistent = p.bear_score > p.bull_score;
      }
      qualityMetrics.direction_consistent = directionConsistent;
    } else if (nodeType === "debate") {
      // 辩论节点指标
      qualityMetrics.stance = p.stance ?? null;
      qualityMetrics.strength_score = p.bull_strength_score ?? p.bear_strength_score ?? null;
      qualityMetrics.confidence = p.confidence ?? null;
      qualityMetrics.logic_consistent = true; // 默认true，后续可增强
    } else if (nodeType === "decision") {
      // 决策节点指标
      qualityMetrics.action = p.verdict ?? null;
      qualityMetrics.confidence = p.confidence ?? null;
      qualityMetrics.risk_assessed = true; // 默认true
      qualityMetrics.criteria_met = result.issueCount === 0;
    } else {
      // 工具/估值/风险节点指标
      qualityMetrics.completeness = result.issueCount === 0;
      qualityMetrics.accuracy = result.score / 100;
    }

    invoke("save_node_feedback", {
      request: {
        nodeType,
        nodeId: expertId,
        reportId: `report-${Date.now()}`,
        stockCode,
        executionId,
        qualityScore: result.score,
        grade: result.grade,
        issueCount: result.issueCount,
        warningCount: result.warningCount,
        goodCount: result.goodCount,
        checksJson,
        qualityMetricsJson: JSON.stringify(qualityMetrics),
      },
    }).catch((err) => {
      console.warn(`Failed to save ${nodeTypeName} feedback for self-evolution:`, err);
    });
  }, [open, parsed, result.score, nodeType, nodeTypeName]);

  // 节点自我进化状态
  const [evolving, setEvolving] = useState(false);
  const [evolutionStatus, setEvolutionStatus] = useState<string | null>(null);
  const [evolutionSuggestions, setEvolutionSuggestions] = useState<string[]>([]);

  const handleEvolve = async () => {
    setEvolving(true);
    setEvolutionStatus(null);
    setEvolutionSuggestions([]);
    try {
      const status = await invoke<{
        node_type: string;
        node_id: string;
        total_feedbacks: number;
        status: string;
        suggestions: string[];
      }>("evolve_node_command", {
        request: {
          nodeType,
          nodeId: expertId,
        },
      });
      setEvolutionStatus(status.status);
      setEvolutionSuggestions(status.suggestions || []);
    } catch (err) {
      console.error(`Failed to evolve ${nodeTypeName}:`, err);
      setEvolutionStatus("error");
    } finally {
      setEvolving(false);
    }
  };

  const columns: ColumnsType<QualityCheck> = [
    {
      title: "",
      dataIndex: "status",
      key: "icon",
      width: 32,
      render: (s: string) => STATUS_ICON[s] ?? null,
    },
    {
      title: t("stockAnalysis.analystReport.dataQualityFieldCompleteness"),
      dataIndex: "field",
      key: "field",
      width: 180,
      render: (val: string) => <code>{val}</code>,
    },
    {
      title: t("stockAnalysis.analystReport.dataQualityOverall"),
      dataIndex: "detail",
      key: "detail",
      render: (val: string) => <Text style={{ fontSize: 12 }}>{val}</Text>,
    },
  ];

  return (
    <Modal
      title={
        <span>
          {name} — {t("stockAnalysis.analystReport.dataQuality")}
        </span>
      }
      open={open}
      onCancel={onClose}
      footer={
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          {/* 左侧：进化状态 */}
          <div style={{ flex: 1 }}>
            {evolutionStatus === "healthy" && <Tag color="success">✓ {nodeTypeName}状态良好</Tag>}
            {evolutionStatus === "needs_attention" && <Tag color="warning">⚠ {nodeTypeName}需要优化</Tag>}
            {evolutionStatus === "collecting_data" && <Tag color="blue">收集{nodeTypeName}数据中...</Tag>}
            {evolutionStatus === "no_data" && <Tag color="default">{nodeTypeName}暂无反馈数据</Tag>}
            {evolutionStatus === "error" && <Tag color="error">{nodeTypeName}进化失败</Tag>}
          </div>
          {/* 右侧：操作按钮 */}
          <div style={{ display: "flex", gap: 8 }}>
            <Button onClick={onClose}>关闭</Button>
            <Button
              type="primary"
              icon={<ThunderboltFilled />}
              loading={evolving}
              onClick={handleEvolve}
              disabled={!parsed}
            >
              触发{nodeTypeName}自我进化
            </Button>
          </div>
        </div>
      }
      width={640}
      style={{ top: 40 }}
      styles={{ body: { maxHeight: "70vh", overflow: "auto" } }}
    >
      {result.score <= 10 && !parsed
        ? (
          <div style={{ padding: 24, textAlign: "center" }}>
            <Text type="secondary">{t("stockAnalysis.analystReport.dataQualityEmpty")}</Text>
          </div>
        )
        : (
          <>
            {/* 总分 + 评级 */}
            <Row gutter={24} style={{ marginBottom: 20 }}>
              <Col span={8} style={{ textAlign: "center" }}>
                <Progress
                  type="circle"
                  percent={result.score}
                  size={80}
                  strokeColor={GRADE_COLOR[result.grade]}
                  format={(pct) => `${Math.round(pct ?? 0)}`}
                />
                <div style={{ marginTop: 4 }}>
                  <Text style={{ fontSize: 12, color: "var(--muted)" }}>
                    {t("stockAnalysis.analystReport.dataQualityScore")}
                  </Text>
                </div>
              </Col>
              <Col span={8} style={{ textAlign: "center" }}>
                <div
                  style={{
                    fontSize: 48,
                    fontWeight: 700,
                    color: GRADE_COLOR[result.grade],
                    lineHeight: 1,
                    marginTop: 16,
                  }}
                >
                  {result.grade}
                </div>
                <div style={{ marginTop: 4 }}>
                  <Text style={{ fontSize: 12, color: "var(--muted)" }}>
                    {t("stockAnalysis.analystReport.dataQualityOverall")}
                  </Text>
                </div>
              </Col>
              <Col span={8} style={{ textAlign: "center", paddingTop: 20 }}>
                <div style={{ display: "flex", justifyContent: "center", gap: 12 }}>
                  <Tooltip title={t("stockAnalysis.analystReport.dataQualityGood")}>
                    <Tag color="success">{result.goodCount}</Tag>
                  </Tooltip>
                  <Tooltip title={t("stockAnalysis.analystReport.dataQualityWarning")}>
                    <Tag color="warning">{result.warningCount}</Tag>
                  </Tooltip>
                  <Tooltip title={t("stockAnalysis.analystReport.dataQualityIssue")}>
                    <Tag color="error">{result.issueCount}</Tag>
                  </Tooltip>
                </div>
                <div style={{ marginTop: 4 }}>
                  <Text style={{ fontSize: 12, color: "var(--muted)" }}>
                    {t("stockAnalysis.analystReport.dataQualityCheckCount", { count: result.checks.length })}
                  </Text>
                </div>
              </Col>
            </Row>

            {/* 检查明细表 */}
            <Table
              dataSource={result.checks}
              columns={columns}
              rowKey={(_, i) => String(i ?? 0)}
              pagination={false}
              size="small"
              bordered
              style={{ fontSize: 12 }}
              onHeaderRow={() => ({ style: { fontSize: 12 } })}
            />

            {/* 自我进化建议 */}
            {evolutionSuggestions.length > 0 && (
              <div style={{ marginTop: 16, padding: 12, background: "var(--ant-color-info-bg)", borderRadius: 8 }}>
                <Text strong>🧬 {nodeTypeName}自我进化建议</Text>
                <ul style={{ margin: "8px 0 0 0", paddingLeft: 20 }}>
                  {evolutionSuggestions.map((s, i) => (
                    <li key={i}>
                      <Text type="secondary" style={{ fontSize: 12 }}>{s}</Text>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </>
        )}
    </Modal>
  );
}
