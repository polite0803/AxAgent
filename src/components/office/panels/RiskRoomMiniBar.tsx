// SPDX-License-Identifier: AGPL-3.0-only

/**
 * RiskRoomMiniBar — 风控室压测结果 mini 条。
 *
 * 仅在「投研办公室」场景（investment_office）下显示，置于 Phaser 画布顶部、
 * 其他房间 mini 条之上（堆叠时按渲染顺序排列）。
 *
 * 数据源：stockAnalysisStore.portfolioDashboard.stressTest — 由
 * `get_portfolio_dashboard` 命令返回的 StressTestBundle。
 *
 * 展示三个压测场景：
 *   - m10        ：-10% 普跌
 *   - m20        ：-20% 急跌
 *   - black_swan ：黑天鹅
 * 每个场景显示：portfolioPnl / portfolioPnlPct / topHit / note。
 *
 * 交互：
 *   - 手动刷新按钮（调用 fetchPortfolioDashboard）
 *   - 折叠 / 展开场景列表（默认展开）
 */

import { useStockAnalysisStore } from "@/stores";
import type { PortfolioStressResult } from "@/stores";
import { Button, Empty, Spin, Tag, theme, Tooltip, Typography } from "antd";
import type { TFunction } from "i18next";
import { AlertTriangle, ChevronRight, RefreshCw, ShieldAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 场景 → 标签颜色 */
function scenarioTagColor(scenario: string): string {
  switch (scenario) {
    case "m10":
      return "gold";
    case "m20":
      return "orange";
    case "black_swan":
      return "red";
    default:
      return "default";
  }
}

/** P&L 百分比 → 文本色 */
function pnlPctColor(pct: number, token: ReturnType<typeof theme.useToken>["token"]): string {
  if (pct <= -10) { return token.colorError; }
  if (pct < 0) { return token.colorWarning; }
  return token.colorTextSecondary;
}

/** 格式化金额（元） */
function formatYuan(v: number, t: TFunction): string {
  const abs = Math.abs(v);
  const sign = v < 0 ? "-" : "";
  if (abs >= 1e8) { return `${sign}${(abs / 1e8).toFixed(2)} ${t("office.roomMiniBar.risk.unitYi")}`; }
  if (abs >= 1e4) { return `${sign}${(abs / 1e4).toFixed(2)} ${t("office.roomMiniBar.risk.unitWan")}`; }
  return `${sign}${abs.toFixed(0)}`;
}

/** 格式化百分比 */
function formatPct(v: number): string {
  const sign = v > 0 ? "+" : "";
  return `${sign}${v.toFixed(2)}%`;
}

export interface RiskRoomMiniBarProps {
  sceneTemplateSlug?: string;
}

export function RiskRoomMiniBar({ sceneTemplateSlug }: RiskRoomMiniBarProps) {
  // === 所有 hook 前先做条件返回，遵守 Rules of Hooks ===
  if (sceneTemplateSlug !== "investment_office") {
    return null;
  }

  const { t } = useTranslation();
  const { token } = theme.useToken();

  const portfolioDashboard = useStockAnalysisStore((s) => s.portfolioDashboard);
  const portfolioRefreshing = useStockAnalysisStore((s) => s.portfolioRefreshing);
  const portfolioLastError = useStockAnalysisStore((s) => s.portfolioLastError);
  const fetchPortfolioDashboard = useStockAnalysisStore(
    (s) => s.fetchPortfolioDashboard,
  );

  const [collapsed, setCollapsed] = useState(false);

  // 首次挂载拉取
  useEffect(() => {
    void fetchPortfolioDashboard(null);
  }, [fetchPortfolioDashboard]);

  const handleRefresh = () => {
    void fetchPortfolioDashboard(null);
  };

  const stress = portfolioDashboard?.stressTest;
  const scenarios: PortfolioStressResult[] = [];
  if (stress?.m10) { scenarios.push(stress.m10); }
  if (stress?.m20) { scenarios.push(stress.m20); }
  if (stress?.blackSwan) { scenarios.push(stress.blackSwan); }

  // 风险等级判定：取最差场景的 pct
  const worstPct = scenarios.length
    ? Math.min(...scenarios.map((s) => s.portfolioPnlPct))
    : 0;
  const riskLevel = worstPct <= -15
    ? "high"
    : worstPct <= -5
    ? "medium"
    : "low";

  const riskLevelColor = riskLevel === "high"
    ? token.colorError
    : riskLevel === "medium"
    ? token.colorWarning
    : token.colorSuccess;

  const riskLevelLabel = t(`office.roomMiniBar.risk.level.${riskLevel}`);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 4,
        padding: "6px 8px",
        background: token.colorBgLayout,
        borderRadius: 6,
        border: `1px solid ${token.colorBorderSecondary}`,
        fontSize: 12,
      }}
    >
      {/* 标题栏 */}
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <ShieldAlert size={14} color={riskLevelColor} />
        <Text strong style={{ fontSize: 12 }}>
          {t("office.roomMiniBar.risk.title")}
        </Text>
        <Tag
          color={riskLevel === "high" ? "red" : riskLevel === "medium" ? "orange" : "green"}
          style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
        >
          {riskLevelLabel}
        </Tag>
        {portfolioDashboard && (
          <Text type="secondary" style={{ fontSize: 10 }}>
            {t("office.roomMiniBar.risk.snapshotAt", {
              time: new Date(portfolioDashboard.snapshotAt).toLocaleString(),
            })}
          </Text>
        )}

        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 4 }}>
          <Tooltip title={t("office.roomMiniBar.risk.refresh")}>
            <Button
              size="small"
              type="text"
              icon={<RefreshCw size={12} />}
              loading={portfolioRefreshing}
              onClick={handleRefresh}
              style={{ padding: "0 4px" }}
            />
          </Tooltip>
          <Tooltip
            title={collapsed
              ? t("office.roomMiniBar.risk.expand")
              : t("office.roomMiniBar.risk.collapse")}
          >
            <Button
              size="small"
              type="text"
              onClick={() => setCollapsed((v) => !v)}
              style={{ padding: "0 4px", fontSize: 11 }}
            >
              {collapsed ? "▾" : "▴"}
            </Button>
          </Tooltip>
        </div>
      </div>

      {/* 内容区 */}
      {!collapsed && (
        <>
          {portfolioRefreshing && !portfolioDashboard
            ? (
              <div style={{ textAlign: "center", padding: 12 }}>
                <Spin size="small" />
              </div>
            )
            : scenarios.length === 0
            ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={portfolioLastError ?? t("office.roomMiniBar.risk.empty")}
                styles={{ description: { fontSize: 11, color: token.colorTextQuaternary } }}
                style={{ margin: "8px 0" }}
              />
            )
            : (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 3,
                  maxHeight: 200,
                  overflowY: "auto",
                  paddingRight: 4,
                }}
              >
                {scenarios.map((s) => <ScenarioItem key={s.scenario} scenario={s} />)}
                {worstPct <= -15 && (
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 4,
                      padding: "4px 8px",
                      background: token.colorErrorBg,
                      borderRadius: 4,
                      fontSize: 10,
                      color: token.colorError,
                    }}
                  >
                    <AlertTriangle size={11} />
                    <Text type="danger" style={{ fontSize: 10 }}>
                      {t("office.roomMiniBar.risk.highRiskHint")}
                    </Text>
                  </div>
                )}
              </div>
            )}
        </>
      )}
    </div>
  );
}

function ScenarioItem({ scenario }: { scenario: PortfolioStressResult }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 2,
        padding: "4px 8px",
        background: token.colorBgContainer,
        borderRadius: 4,
        border: `1px solid ${token.colorBorderSecondary}`,
        fontSize: 11,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Tag
          color={scenarioTagColor(scenario.scenario)}
          style={{ fontSize: 9, margin: 0, padding: "0 4px", lineHeight: "16px" }}
        >
          {scenario.label || scenario.scenario}
        </Tag>
        <Text
          strong
          style={{
            fontSize: 12,
            color: pnlPctColor(scenario.portfolioPnlPct, token),
          }}
        >
          {formatPct(scenario.portfolioPnlPct)}
        </Text>
        <Text type="secondary" style={{ fontSize: 10 }}>
          {formatYuan(scenario.portfolioPnl, t)}
        </Text>
      </div>
      {scenario.topHit && (
        <div style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 10 }}>
          <ChevronRight size={10} color={token.colorTextTertiary} />
          <Text type="secondary" style={{ fontSize: 10 }}>
            {t("office.roomMiniBar.risk.topHit")}:
          </Text>
          <Text style={{ fontSize: 10 }}>
            {scenario.topHit.stockName || scenario.topHit.stockCode}
          </Text>
          <Text
            style={{
              fontSize: 10,
              color: pnlPctColor(scenario.topHit.pnlPct, token),
            }}
          >
            {formatPct(scenario.topHit.pnlPct)}
          </Text>
        </div>
      )}
      {scenario.note && (
        <Text type="secondary" style={{ fontSize: 10 }}>
          {scenario.note}
        </Text>
      )}
    </div>
  );
}
