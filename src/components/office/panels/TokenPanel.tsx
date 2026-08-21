// SPDX-License-Identifier: AGPL-3.0-only

/**
 * TokenPanel — Token 用量统计面板 + 模拟组合总览卡。
 *
 * 顶部展示「模拟组合总览」（portfolioDashboard）：
 * - 聚合所有 active paper portfolio 的 summary
 * - 显示持仓数、总市值、总浮动盈亏、总收益率
 * - 投研团队可直接看到当前观察组合的整体表现
 *
 * 下方展示当前 fleet 的成员 token 用量（today / total），按今日用量降序。
 */

import { useOfficeStore, usePaperPortfolioStore } from "@/stores";
import type { FleetMember } from "@/types";
import { Button, Empty, Spin, Table, Tag, theme, Typography } from "antd";
import type { TFunction } from "i18next";
import { Briefcase, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

function formatTokens(n: number) {
  if (n >= 1_000_000) { return `${(n / 1_000_000).toFixed(2)}M`; }
  if (n >= 1_000) { return `${(n / 1_000).toFixed(1)}K`; }
  return String(n);
}

function formatMoney(n: number, t: TFunction): string {
  const abs = Math.abs(n);
  const sign = n >= 0 ? "" : "-";
  if (abs >= 100_000_000) { return `${sign}${(abs / 100_000_000).toFixed(2)}${t("office.token.unitYi")}`; }
  if (abs >= 10_000) { return `${sign}${(abs / 10_000).toFixed(2)}${t("office.token.unitWan")}`; }
  if (abs >= 1_000) { return `${sign}${(abs / 1_000).toFixed(1)}${t("office.token.unitQian")}`; }
  return `${sign}${abs.toFixed(2)}`;
}

function formatPct(n: number): string {
  const sign = n >= 0 ? "+" : "";
  return `${sign}${n.toFixed(2)}%`;
}

export function TokenPanel({ fleetId }: { fleetId: string }) {
  const { t } = useTranslation();
  const { token: themeToken } = theme.useToken();
  const members = useOfficeStore((s) => s.membersByFleet[fleetId] ?? []);
  const loading = useOfficeStore((s) => s.loading);
  const resetDaily = useOfficeStore((s) => s.resetDailyTokens);
  const [resetting, setResetting] = useState(false);

  // 组合总览数据（聚合所有 active 组合）
  const activeDetails = usePaperPortfolioStore((s) => s.activeDetails);
  const fetchActiveDetails = usePaperPortfolioStore((s) => s.fetchActiveDetails);
  const loadingPortfolio = usePaperPortfolioStore((s) => s.loadingList);

  // 进入 Token tab 时拉取一次 active 组合详情（用于顶部总览卡）
  useEffect(() => {
    void fetchActiveDetails();
  }, [fetchActiveDetails]);

  // 聚合所有 active 组合的 summary（多组合时累加）
  const portfolioSummary = (() => {
    if (activeDetails.length === 0) { return null; }
    const aggregated = activeDetails.reduce(
      (acc, p) => {
        acc.positionCount += p.summary.positionCount;
        acc.openCount += p.summary.openCount;
        acc.closedCount += p.summary.closedCount;
        acc.totalCost += p.summary.totalCost;
        acc.totalMarketValue += p.summary.totalMarketValue;
        acc.totalUnrealizedPnl += p.summary.totalUnrealizedPnl;
        acc.totalRealizedPnl += p.summary.totalRealizedPnl;
        return acc;
      },
      {
        positionCount: 0,
        openCount: 0,
        closedCount: 0,
        totalCost: 0,
        totalMarketValue: 0,
        totalUnrealizedPnl: 0,
        totalRealizedPnl: 0,
      },
    );
    const totalReturnPct = aggregated.totalCost > 0
      ? ((aggregated.totalMarketValue - aggregated.totalCost) / aggregated.totalCost) * 100
      : 0;
    return { ...aggregated, totalReturnPct };
  })();

  const sorted = [...members].sort((a, b) => b.todayTokens - a.todayTokens);
  const totalToday = sorted.reduce((s, m) => s + m.todayTokens, 0);
  const totalAll = sorted.reduce((s, m) => s + m.totalTokens, 0);

  const handleReset = async () => {
    setResetting(true);
    try {
      await resetDaily(fleetId);
    } finally {
      setResetting(false);
    }
  };

  if (loading && members.length === 0) {
    return (
      <div style={{ padding: 24, textAlign: "center" }}>
        <Spin size="small" />
      </div>
    );
  }

  if (members.length === 0) {
    return (
      <div style={{ padding: 24 }}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("office.token.empty")}
          styles={{ description: { fontSize: 12, color: themeToken.colorTextQuaternary } }}
        />
      </div>
    );
  }

  return (
    <div style={{ padding: 12, height: "100%", display: "flex", flexDirection: "column", gap: 8 }}>
      {/* 模拟组合总览卡（portfolioDashboard） */}
      {portfolioSummary && (
        <div
          style={{
            padding: "10px 12px",
            background: themeToken.colorBgLayout,
            borderRadius: 6,
            border: `1px solid ${themeToken.colorBorderSecondary}`,
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              marginBottom: 8,
              fontSize: 12,
              color: themeToken.colorTextSecondary,
              fontWeight: 600,
            }}
          >
            <Briefcase size={12} />
            <span>{t("office.token.portfolioDashboardTitle")}</span>
            <Text type="secondary" style={{ fontSize: 10, fontWeight: 400 }}>
              {t("office.token.portfolioCount", { count: activeDetails.length })}
            </Text>
          </div>
          {/* 四宫格指标 */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
            <PortfolioMetric
              label={t("office.token.positionCount")}
              value={`${portfolioSummary.openCount}/${portfolioSummary.positionCount}`}
              hint={t("office.token.openClosedHint", {
                open: portfolioSummary.openCount,
                closed: portfolioSummary.closedCount,
              })}
            />
            <PortfolioMetric
              label={t("office.token.totalMarketValue")}
              value={formatMoney(portfolioSummary.totalMarketValue, t)}
              hint={`${t("office.token.totalCost")}：${formatMoney(portfolioSummary.totalCost, t)}`}
            />
            <PortfolioMetric
              label={t("office.token.unrealizedPnl")}
              value={formatMoney(portfolioSummary.totalUnrealizedPnl, t)}
              valueColor={portfolioSummary.totalUnrealizedPnl > 0
                ? "#52c41a"
                : portfolioSummary.totalUnrealizedPnl < 0
                ? "#ff4d4f"
                : themeToken.colorText}
            />
            <PortfolioMetric
              label={t("office.token.totalReturn")}
              value={formatPct(portfolioSummary.totalReturnPct)}
              valueColor={portfolioSummary.totalReturnPct > 0
                ? "#52c41a"
                : portfolioSummary.totalReturnPct < 0
                ? "#ff4d4f"
                : themeToken.colorText}
              hint={`${t("office.token.realizedPnl")}：${formatMoney(portfolioSummary.totalRealizedPnl, t)}`}
            />
          </div>
        </div>
      )}
      {loadingPortfolio && !portfolioSummary && (
        <div style={{ padding: 12, textAlign: "center" }}>
          <Spin size="small" />
        </div>
      )}

      {/* Token 总览统计 */}
      <div style={{ display: "flex", gap: 8 }}>
        <div
          style={{
            flex: 1,
            padding: "10px 12px",
            background: themeToken.colorBgLayout,
            borderRadius: 6,
            border: `1px solid ${themeToken.colorBorderSecondary}`,
          }}
        >
          <div style={{ fontSize: 11, color: themeToken.colorTextTertiary }}>
            {t("office.token.todayTotal")}
          </div>
          <div style={{ fontSize: 18, fontWeight: 600, color: themeToken.colorPrimary }}>
            {formatTokens(totalToday)}
          </div>
        </div>
        <div
          style={{
            flex: 1,
            padding: "10px 12px",
            background: themeToken.colorBgLayout,
            borderRadius: 6,
            border: `1px solid ${themeToken.colorBorderSecondary}`,
          }}
        >
          <div style={{ fontSize: 11, color: themeToken.colorTextTertiary }}>
            {t("office.token.allTotal")}
          </div>
          <div style={{ fontSize: 18, fontWeight: 600, color: themeToken.colorText }}>
            {formatTokens(totalAll)}
          </div>
        </div>
      </div>

      {/* 操作按钮 */}
      <div style={{ display: "flex", justifyContent: "flex-end" }}>
        <Button
          size="small"
          icon={<RotateCcw size={12} />}
          loading={resetting}
          onClick={handleReset}
        >
          {t("office.token.resetDaily")}
        </Button>
      </div>

      {/* 成员列表 */}
      <div style={{ flex: 1, overflow: "auto" }}>
        <Table<FleetMember>
          size="small"
          dataSource={sorted}
          rowKey="id"
          pagination={false}
          columns={[
            {
              title: t("office.token.colMember"),
              dataIndex: "displayName",
              render: (v, r) => (
                <div>
                  <div style={{ fontWeight: 500, fontSize: 12 }}>{v}</div>
                  <div style={{ fontSize: 10, color: themeToken.colorTextQuaternary, fontFamily: "monospace" }}>
                    {r.agentSlug}
                  </div>
                </div>
              ),
            },
            {
              title: t("office.token.colStatus"),
              dataIndex: "status",
              width: 90,
              render: (s: FleetMember["status"]) => (
                <Tag
                  color={statusColor(s)}
                  style={{ fontSize: 10, margin: 0, padding: "0 6px" }}
                >
                  {t(`office.memberStatus.${s}`)}
                </Tag>
              ),
            },
            {
              title: t("office.token.colToday"),
              dataIndex: "todayTokens",
              width: 100,
              align: "right",
              render: (v: number) => (
                <Text style={{ fontWeight: 600, color: themeToken.colorPrimary }}>
                  {formatTokens(v)}
                </Text>
              ),
            },
            {
              title: t("office.token.colTotal"),
              dataIndex: "totalTokens",
              width: 100,
              align: "right",
              render: (v: number) => (
                <Text style={{ fontSize: 11 }}>
                  {formatTokens(v)}
                </Text>
              ),
            },
          ]}
        />
      </div>
    </div>
  );
}

/** 组合指标单元 */
function PortfolioMetric({
  label,
  value,
  hint,
  valueColor,
}: {
  label: string;
  value: string;
  hint?: string;
  valueColor?: string;
}) {
  const { token: themeToken } = theme.useToken();
  return (
    <div
      style={{
        padding: "6px 8px",
        background: themeToken.colorBgContainer,
        borderRadius: 4,
        border: `1px solid ${themeToken.colorBorderSecondary}`,
      }}
    >
      <div style={{ fontSize: 10, color: themeToken.colorTextTertiary }}>{label}</div>
      <div style={{ fontSize: 14, fontWeight: 600, color: valueColor ?? themeToken.colorText }}>
        {value}
      </div>
      {hint && (
        <div style={{ fontSize: 10, color: themeToken.colorTextQuaternary, marginTop: 2 }}>
          {hint}
        </div>
      )}
    </div>
  );
}

function statusColor(status: FleetMember["status"]): string {
  switch (status) {
    case "idle":
      return "green";
    case "busy":
      return "blue";
    case "paused":
      return "orange";
    case "error":
      return "red";
    case "offline":
      return "default";
    default:
      return "default";
  }
}
