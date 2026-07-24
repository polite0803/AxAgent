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
      setError(typeof e === "string" ? e : e instanceof Error ? e.message : "提取引用失败");
    } finally {
      setLoading(false);
    }
  }, [analysisId]);

  useEffect(() => {
    if (visible && analysisId) {
      loadCitations();
    }
  }, [visible, analysisId, loadCitations]);

  if (!visible) { return null; }

  if (loading) {
    return (
      <div className="flex justify-center py-8">
        <Spin tip={t("stockAnalysis.evidenceCitation.loading", "提取证据引用中...")} />
      </div>
    );
  }

  if (error) {
    return (
      <Alert
        type="error"
        message={t("stockAnalysis.evidenceCitation.error", "提取失败")}
        description={error}
        showIcon
      />
    );
  }

  if (!report || report.citations.length === 0) {
    return (
      <div className="text-gray-400 text-sm text-center py-6">
        {t("stockAnalysis.evidenceCitation.noData", "无可用证据引用数据")}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {/* 概览头部 */}
      <div className="flex items-center justify-between">
        <div>
          <span className="text-sm font-medium text-gray-200">
            {t("stockAnalysis.evidenceCitation.title", "证据引用审计")}
          </span>
          <span className="text-xs text-gray-500 ml-2">
            {report.stockCode} · {report.analysisDate}
          </span>
        </div>
        <Button size="small" onClick={loadCitations}>
          {t("common.refresh", "刷新")}
        </Button>
      </div>

      {/* 统计卡 */}
      <div className="grid grid-cols-3 gap-2">
        <div className="bg-gray-800/60 rounded p-2 text-center">
          <div className="text-lg font-semibold text-green-400">
            {Math.round(report.supportRate * 100)}%
          </div>
          <div className="text-[10px] text-gray-400">
            {t("stockAnalysis.evidenceCitation.supportRate", "数据支撑率")}
          </div>
        </div>
        <div className="bg-gray-800/60 rounded p-2 text-center">
          <div className="text-lg font-semibold text-blue-400">{report.analystCount}</div>
          <div className="text-[10px] text-gray-400">
            {t("stockAnalysis.evidenceCitation.analystCount", "来源分析师")}
          </div>
        </div>
        <div className="bg-gray-800/60 rounded p-2 text-center">
          <div className="text-lg font-semibold text-yellow-400">{report.totalClaims}</div>
          <div className="text-[10px] text-gray-400">
            {t("stockAnalysis.evidenceCitation.claimCount", "决策理由")}
          </div>
        </div>
      </div>

      {/* 理由列表 */}
      <Collapse
        size="small"
        items={report.citations.map((citation, i) => ({
          key: String(i),
          label: (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-gray-400 font-mono text-xs">#{i + 1}</span>
              <span className="text-gray-200 truncate flex-1">{citation.claim}</span>
              <Tag
                className="text-[10px] leading-none px-1 py-0"
                color={citation.hasDataSupport ? "green" : "orange"}
              >
                {citation.hasDataSupport
                  ? t("stockAnalysis.evidenceCitation.supported", "有数据")
                  : t("stockAnalysis.evidenceCitation.unsupported", "无数据")}
              </Tag>
            </div>
          ),
          children: (
            <div className="text-xs space-y-2">
              <div className="flex items-center gap-2">
                <span className="text-gray-400">
                  {t("stockAnalysis.evidenceCitation.source", "来源")}:
                </span>
                <Tag className="text-xs">{citation.sourceAnalystName}</Tag>
                <Tooltip title={`匹配度 ${(citation.matchConfidence * 100).toFixed(0)}%`}>
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
