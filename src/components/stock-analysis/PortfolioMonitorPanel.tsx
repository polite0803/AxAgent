import {
  AlertOutlined,
  ClusterOutlined,
  ReloadOutlined,
  SafetyOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { Alert, Button, Col, Empty, Progress, Row, Skeleton, Space, Statistic, Tag, Tooltip } from "antd";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

import { type PortfolioCorrelationCell, useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";

function fmtMoney(n: number | undefined): string {
  if (n === undefined || n === null) { return "—"; }
  return `${n.toFixed(2)}`;
}

function fmtPct(n: number | undefined, withSign = true): string {
  if (n === undefined || n === null) { return "—"; }
  const sign = n > 0 && withSign ? "+" : "";
  return `${sign}${n.toFixed(2)}%`;
}

function correlationColor(c: number): string {
  if (c >= 0.7) { return "#cf1322"; }
  if (c >= 0.4) { return "#d46b08"; }
  if (c <= -0.3) { return "#389e0d"; }
  return "#8c8c8c";
}

function riskColor(level: string): string {
  if (level.includes("高")) { return "red"; }
  if (level.includes("中")) { return "orange"; }
  if (level.includes("低")) { return "green"; }
  return "default";
}

export function PortfolioMonitorPanel() {
  const { t } = useTranslation();
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  const mode = useTimeAnchorStore((s) => s.mode);
  const dashboard = useStockAnalysisStore((s) => s.portfolioDashboard);
  const correlations = useStockAnalysisStore((s) => s.portfolioCorrelations);
  const refreshing = useStockAnalysisStore((s) => s.portfolioRefreshing);
  const lastError = useStockAnalysisStore((s) => s.portfolioLastError);
  const fetchDashboard = useStockAnalysisStore((s) => s.fetchPortfolioDashboard);
  const fetchCorrelations = useStockAnalysisStore((s) => s.fetchPortfolioCorrelations);
  const refresh = useStockAnalysisStore((s) => s.refreshPortfolioMetrics);
  const lastAsOfRef = useRef<string | null>(null);

  useEffect(() => {
    // 首次加载或 asOfDate 变化时重新拉取数据
    if (lastAsOfRef.current !== asOfDate) {
      lastAsOfRef.current = asOfDate;
      void fetchDashboard(asOfDate);
      void fetchCorrelations(asOfDate);
    }
  }, [asOfDate, fetchDashboard, fetchCorrelations]);

  const isReplay = mode === "replay" && !!asOfDate;

  const hasDashboard = dashboard !== null && typeof dashboard === "object" && !Array.isArray(dashboard);

  const sortedSectors = useMemo(() => {
    if (!hasDashboard || !dashboard || !dashboard.sectorExposure) { return []; }
    return Object.entries(dashboard.sectorExposure)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8);
  }, [dashboard, hasDashboard]);

  const sortedCorrelations = useMemo(() => {
    if (!correlations || !Array.isArray(correlations)) { return []; }
    const cells = [...correlations];
    cells.sort((a, b) => Math.abs(b.correlation) - Math.abs(a.correlation));
    return cells.slice(0, 10);
  }, [correlations]);

  if (!hasDashboard && !refreshing) {
    return (
      <div className="p-4">
        <Empty description={t("stockAnalysis.portfolioMonitor.empty")} />
        {lastError && (
          <Alert
            type="error"
            showIcon
            className="mt-2"
            message={t("stockAnalysis.portfolioMonitor.loadError")}
            description={lastError}
          />
        )}
        <Button
          type="primary"
          icon={<ReloadOutlined />}
          onClick={() => refresh(asOfDate)}
          className="mt-3"
        >
          {t("stockAnalysis.portfolioMonitor.refresh")}
        </Button>
      </div>
    );
  }

  if (!hasDashboard) {
    return <Skeleton active paragraph={{ rows: 6 }} />;
  }

  if (!dashboard) {
    return <Skeleton active paragraph={{ rows: 6 }} />;
  }

  return (
    <div className="space-y-3 p-3">
      {isReplay && (
        <Alert
          type="info"
          showIcon
          message={t("stockAnalysis.portfolioMonitor.historicalTitle", { date: dashboard.asOfDate ?? asOfDate })}
          description={t("stockAnalysis.portfolioMonitor.historicalDesc")}
        />
      )}

      {dashboard.concentrationWarning && (
        <Alert
          type="warning"
          showIcon
          icon={<WarningOutlined />}
          message={t("stockAnalysis.portfolioMonitor.warningTitle")}
          description={dashboard.concentrationWarning}
        />
      )}

      {/* 1) Metric Cards */}
      <Row gutter={[12, 12]}>
        <Col xs={12} sm={8} md={4}>
          <Statistic
            title={t("stockAnalysis.portfolioMonitor.totalMv")}
            value={fmtMoney(dashboard.totalMarketValue)}
            prefix="¥"
            valueStyle={{ fontSize: 18 }}
          />
        </Col>
        <Col xs={12} sm={8} md={4}>
          <Statistic
            title={t("stockAnalysis.portfolioMonitor.totalPnl")}
            value={fmtMoney(dashboard.totalPnl)}
            valueStyle={{
              fontSize: 18,
              color: dashboard.totalPnl >= 0 ? "#cf1322" : "#389e0d",
            }}
            prefix={dashboard.totalPnl >= 0 ? "+" : ""}
          />
        </Col>
        <Col xs={12} sm={8} md={4}>
          <Statistic
            title={t("stockAnalysis.portfolioMonitor.totalPnlPct")}
            value={fmtPct(dashboard.totalPnlPct, false)}
            valueStyle={{
              fontSize: 18,
              color: dashboard.totalPnlPct >= 0 ? "#cf1322" : "#389e0d",
            }}
          />
        </Col>
        <Col xs={12} sm={8} md={4}>
          <Statistic
            title={t("stockAnalysis.portfolioMonitor.topConcentration")}
            value={fmtPct(dashboard.topConcentrationPct, false)}
            valueStyle={{ fontSize: 18 }}
            suffix={
              <Tooltip title={t("stockAnalysis.portfolioMonitor.topConcentrationTip")}>
                <Tag color="red" className="ml-1">
                  {dashboard.positions.length}
                </Tag>
              </Tooltip>
            }
          />
        </Col>
        <Col xs={12} sm={8} md={4}>
          <Tooltip title={t("stockAnalysis.portfolioMonitor.betaTip")}>
            <Statistic
              title={t("stockAnalysis.portfolioMonitor.beta")}
              value={dashboard.beta?.toFixed(2) ?? "—"}
              valueStyle={{ fontSize: 18 }}
            />
          </Tooltip>
        </Col>
        <Col xs={12} sm={8} md={4}>
          <Tooltip title={t("stockAnalysis.portfolioMonitor.diversificationTip")}>
            <Statistic
              title={t("stockAnalysis.portfolioMonitor.diversificationScore")}
              value={dashboard.diversificationScore}
              suffix="/ 100"
              valueStyle={{
                fontSize: 18,
                color: dashboard.diversificationScore >= 70 ? "#389e0d" : "#d46b08",
              }}
            />
          </Tooltip>
        </Col>
      </Row>

      <Row gutter={[12, 12]} className="items-center">
        <Col flex="auto">
          <Space>
            <Tag color={riskColor(dashboard.riskLevel)} icon={<SafetyOutlined />}>
              {t("stockAnalysis.portfolioMonitor.riskLevel")}: {dashboard.riskLevel}
            </Tag>
            {dashboard.correlationAvg != null && (
              <Tag color="purple" icon={<ClusterOutlined />}>
                {t("stockAnalysis.portfolioMonitor.correlationAvg")}: {dashboard.correlationAvg.toFixed(2)}
              </Tag>
            )}
            {dashboard.sharpe30d != null && (
              <Tag color="blue">
                {t("stockAnalysis.portfolioMonitor.sharpe30d")}: {dashboard.sharpe30d.toFixed(2)}
              </Tag>
            )}
            <span className="text-xs text-gray-500">
              {t("stockAnalysis.portfolioMonitor.snapshotAt")}: {dashboard.snapshotAt
                ? new Date(dashboard.snapshotAt).toLocaleString()
                : "—"}
            </span>
          </Space>
        </Col>
        <Col>
          <Button
            icon={<ReloadOutlined />}
            onClick={() => refresh(asOfDate)}
            loading={refreshing}
            type="primary"
          >
            {t("stockAnalysis.portfolioMonitor.refresh")}
          </Button>
        </Col>
      </Row>

      {/* 2) 行业集中度 + 相关性高亮 */}
      <Row gutter={[12, 12]}>
        <Col xs={24} md={12}>
          <div className="rounded border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-900">
            <div className="mb-2 flex items-center gap-2">
              <ClusterOutlined />
              <span className="font-semibold">{t("stockAnalysis.portfolioMonitor.sectorExposure")}</span>
            </div>
            {sortedSectors.length === 0 ? <Empty description={t("stockAnalysis.portfolioMonitor.noSector")} /> : (
              sortedSectors.map(([sector, pct]) => (
                <div key={sector} className="mb-2">
                  <div className="flex justify-between text-xs">
                    <span>{sector}</span>
                    <span className="text-gray-500">{pct.toFixed(1)}%</span>
                  </div>
                  <Progress
                    percent={Math.min(100, pct)}
                    showInfo={false}
                    strokeColor={pct > 40 ? "#cf1322" : pct > 25 ? "#d46b08" : "#1677ff"}
                    size="small"
                  />
                </div>
              ))
            )}
          </div>
        </Col>

        <Col xs={24} md={12}>
          <div className="rounded border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-900">
            <div className="mb-2 flex items-center gap-2">
              <AlertOutlined />
              <span className="font-semibold">{t("stockAnalysis.portfolioMonitor.correlationHighlights")}</span>
            </div>
            {sortedCorrelations.length === 0
              ? <Empty description={t("stockAnalysis.portfolioMonitor.noCorrelation")} />
              : (
                <ul className="space-y-1 text-sm">
                  {sortedCorrelations.map((c: PortfolioCorrelationCell) => (
                    <li
                      key={`${c.codeA}-${c.codeB}`}
                      className="flex items-center justify-between"
                    >
                      <span>
                        <span className="font-mono">{c.codeA}</span>
                        <span className="mx-1 text-gray-400">↔</span>
                        <span className="font-mono">{c.codeB}</span>
                      </span>
                      <span
                        className="font-mono font-semibold"
                        style={{ color: correlationColor(c.correlation) }}
                      >
                        {c.correlation.toFixed(2)}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
          </div>
        </Col>
      </Row>

      {/* 3) 压测三场景 */}
      <div className="rounded border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-900">
        <div className="mb-2 flex items-center gap-2">
          <ThunderboltOutlined />
          <span className="font-semibold">{t("stockAnalysis.portfolioMonitor.stressTest")}</span>
        </div>
        <Row gutter={[12, 12]}>
          {[dashboard.stressTest.m10, dashboard.stressTest.m20, dashboard.stressTest.blackSwan]
            .filter(Boolean)
            .map((s) => {
              if (!s) { return null; }
              return (
                <Col key={s.scenario} xs={24} sm={8}>
                  <div className="rounded border border-gray-200 p-2 dark:border-gray-700">
                    <div className="text-sm font-semibold">{s.label}</div>
                    <div
                      className="text-2xl font-bold"
                      style={{ color: s.portfolioPnl < 0 ? "#cf1322" : "#389e0d" }}
                    >
                      {fmtMoney(s.portfolioPnl)} 元
                    </div>
                    <div className="text-xs text-gray-500">
                      {fmtPct(s.portfolioPnlPct, false)} · {t("stockAnalysis.portfolioMonitor.worstHit")}:{" "}
                      {s.topHit?.stockCode ?? "—"}
                      {s.topHit ? ` (${fmtPct(s.topHit.pnlPct, false)})` : ""}
                    </div>
                    <div className="mt-1 text-xs text-gray-400">{s.note}</div>
                  </div>
                </Col>
              );
            })}
        </Row>
      </div>
    </div>
  );
}
