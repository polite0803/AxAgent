/**
 * 证据引用审计溯源面板
 *
 * 展示决策理由 → 分析师报告 → 数据源的完整引用链。
 * 数据来自 extract_evidence_citations 后端命令。
 */

import { invoke } from "@/lib/invoke";
import { Alert, Button, Collapse, Spin, Tag, Tooltip } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** 单条证据引用（对齐后端 EvidenceCitation） */
interface EvidenceCitation {
  claim: string;
  sourceAnalystId: string;
  sourceAnalystName: string;
  matchConfidence: number;
  sourceSnippet: string;
  hasDataSupport: boolean;
  dataSource: string | null;
}

/** 引用报告（对齐后端 CitationReport） */
interface CitationReport {
  stockCode: string;
  stockName: string;
  analysisDate: string;
  decisionAction: string;
  decisionConfidence: number;
  citations: EvidenceCitation[];
  supportedClaims: number;
  totalClaims: number;
  supportRate: number;
  analystCount: number;
}

interface Props {
  analysisId: string;
  visible?: boolean;
}

export function EvidenceCitationPanel({ analysisId, visible = true }: Props) {
  const { t } = useTranslation();
  const [report, setReport] = useState<CitationReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadCitations = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<CitationReport>("extract_evidence_citations", {
        analysisId,
      });
      setReport(result);
    } catch (e: unknown) {
      setError(
        typeof e === "string" ? e : e instanceof Error ? e.message : t("stockAnalysis.evidenceCitation.extractFailed"),
      );
    } finally {
      setLoading(false);
    }
  }, [analysisId, t]);

  useEffect(() => {
    if (visible && analysisId) {
      loadCitations();
    }
  }, [visible, analysisId, loadCitations]);

  if (!visible) { return null; }

  if (loading) {
    return (
      <div className="flex justify-center py-8">
        <Spin description={t("stockAnalysis.evidenceCitation.loading")} />
      </div>
    );
  }

  if (error) {
    return (
      <Alert
        type="error"
        title={t("stockAnalysis.evidenceCitation.error")}
        description={error}
        showIcon
      />
    );
  }

  if (!report || report.citations.length === 0) {
    return (
      <div className="text-gray-400 text-sm text-center py-6">
        {t("stockAnalysis.evidenceCitation.noData")}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {/* 概览头部 */}
      <div className="flex items-center justify-between">
        <div>
          <span className="text-sm font-medium text-gray-200">
            {t("stockAnalysis.evidenceCitation.title")}
          </span>
          <span className="text-xs text-gray-500 ml-2">
            {report.stockCode} · {report.analysisDate}
          </span>
        </div>
        <Button size="small" onClick={loadCitations}>
          {t("common.refresh")}
        </Button>
      </div>

      {/* 统计卡 */}
      <div className="grid grid-cols-3 gap-2">
        <div className="bg-gray-800/60 rounded p-2 text-center">
          <div className="text-lg font-semibold text-green-400">
            {Math.round(report.supportRate * 100)}%
          </div>
          <div className="text-[10px] text-gray-400">
            {t("stockAnalysis.evidenceCitation.supportRate")}
          </div>
        </div>
        <div className="bg-gray-800/60 rounded p-2 text-center">
          <div className="text-lg font-semibold text-blue-400">{report.analystCount}</div>
          <div className="text-[10px] text-gray-400">
            {t("stockAnalysis.evidenceCitation.analystCount")}
          </div>
        </div>
        <div className="bg-gray-800/60 rounded p-2 text-center">
          <div className="text-lg font-semibold text-yellow-400">{report.totalClaims}</div>
          <div className="text-[10px] text-gray-400">
            {t("stockAnalysis.evidenceCitation.claimCount")}
          </div>
        </div>
      </div>

      {/* 理由列表 */}
      <Collapse
        size="small"
        items={report.citations.map((citation, i) => ({
          key: String(i),
          label: (
            <div
              className="flex items-center gap-2 text-sm"
              style={{ maxWidth: "100%", minWidth: 0 }}
            >
              <span className="text-gray-400 font-mono text-xs shrink-0">#{i + 1}</span>
              <span
                className="text-gray-200 flex-1"
                style={{
                  overflowWrap: "anywhere",
                  wordBreak: "break-all",
                  whiteSpace: "normal",
                }}
              >
                {citation.claim}
              </span>
              <Tag
                className="text-[10px] leading-none px-1 py-0 shrink-0"
                color={citation.hasDataSupport ? "green" : "orange"}
              >
                {citation.hasDataSupport
                  ? t("stockAnalysis.evidenceCitation.supported")
                  : t("stockAnalysis.evidenceCitation.unsupported")}
              </Tag>
            </div>
          ),
          children: (
            <div className="text-xs space-y-2">
              <div className="flex items-center gap-2">
                <span className="text-gray-400">
                  {t("stockAnalysis.evidenceCitation.source")}:
                </span>
                <Tag className="text-xs">{citation.sourceAnalystName}</Tag>
                <Tooltip
                  title={t("stockAnalysis.evidenceCitation.matchConfidence", {
                    percent: (citation.matchConfidence * 100).toFixed(0),
                  })}
                >
                  <div className="h-1.5 w-16 bg-gray-700 rounded-full overflow-hidden">
                    <div
                      className="h-full rounded-full"
                      style={{
                        width: `${Math.min(citation.matchConfidence * 100, 100)}%`,
                        backgroundColor: citation.matchConfidence > 0.5 ? "#22c55e" : "#eab308",
                      }}
                    />
                  </div>
                </Tooltip>
              </div>
              {citation.sourceSnippet && (
                <div className="bg-gray-900/60 rounded p-1.5 text-gray-400 italic border-l-2 border-gray-600">
                  {citation.sourceSnippet}
                </div>
              )}
              {citation.dataSource && (
                <div className="text-green-400/80">
                  📊 {citation.dataSource}
                </div>
              )}
            </div>
          ),
        }))}
      />
    </div>
  );
}
