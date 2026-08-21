// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
import type { AttentionMetrics, Catalyst, ExitSignals, SerenityCandidate } from "@/stores/feature/serenityStore";
import { AimOutlined, AlertOutlined, FireOutlined, ThunderboltOutlined } from "@ant-design/icons";
import { Card, Progress, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Text, Title } = Typography;

/// 评分分级：≥80 高优（绿）/ 65-79 中等（蓝）/ 55-64 弱信号（橙）/ <55 排除（红）
function scoreTier(score: number): "high" | "mid" | "low" | "reject" {
  if (score >= 80) { return "high"; }
  if (score >= 65) { return "mid"; }
  if (score >= 55) { return "low"; }
  return "reject";
}

const TIER_COLOR: Record<string, string> = {
  high: "#52c41a",
  mid: "#1677ff",
  low: "#fa8c16",
  reject: "#ff4d4f",
};

/// relevance 标签颜色
function relevanceColor(rel?: string): string {
  if (rel === "direct") { return "green"; }
  if (rel === "indirect") { return "blue"; }
  return "default";
}

function relevanceLabel(t: (k: string) => string, rel?: string): string {
  if (rel === "direct") { return t("serenityPanel.directBenefit"); }
  if (rel === "indirect") { return t("serenityPanel.indirectBenefit"); }
  return t("serenityPanel.themeRelated");
}

/// 催化剂类型颜色
function catalystColor(type: string): string {
  if (type === "earnings") { return "green"; }
  if (type === "production_ramp") { return "blue"; }
  if (type === "policy") { return "orange"; }
  if (type === "supply_shock") { return "red"; }
  if (type === "capacity_release") { return "purple"; }
  if (type === "contract_award") { return "cyan"; }
  return "default";
}

function catalystLabel(t: (k: string) => string, type: string): string {
  const map: Record<string, string> = {
    earnings: t("serenityPanel.catalystEarnings"),
    production_ramp: t("serenityPanel.catalystProdRamp"),
    policy: t("serenityPanel.catalystPolicy"),
    supply_shock: t("serenityPanel.catalystSupplyShock"),
    capacity_release: t("serenityPanel.catalystCapacityRelease"),
    contract_award: t("serenityPanel.catalystContractAward"),
  };
  return map[type] ?? type;
}

function timeframeLabel(t: (k: string) => string, tf: string): string {
  if (tf === "short_term") { return t("serenityPanel.timeframeShort"); }
  if (tf === "mid_term") { return t("serenityPanel.timeframeMid"); }
  return t("serenityPanel.timeframeLong");
}

/// 退出紧迫度颜色
function exitUrgencyColor(urgency?: string): string {
  if (urgency === "exit_now") { return "red"; }
  if (urgency === "caution") { return "orange"; }
  if (urgency === "watch") { return "blue"; }
  return "default";
}

function exitUrgencyLabel(t: (k: string) => string, urgency?: string): string {
  if (urgency === "exit_now") { return t("serenityPanel.exitNow"); }
  if (urgency === "caution") { return t("serenityPanel.exitCaution"); }
  if (urgency === "watch") { return t("serenityPanel.exitWatch"); }
  return t("serenityPanel.exitNone");
}

/// 策略类型颜色映射
function strategyColor(strategy?: string): string {
  if (strategy === "bottleneck") { return "volcano"; }
  if (strategy === "policy") { return "blue"; }
  if (strategy === "earnings") { return "green"; }
  if (strategy === "capital") { return "purple"; }
  if (strategy === "event") { return "orange"; }
  if (strategy === "technical") { return "cyan"; }
  return "default";
}

/// 策略类型标签
function strategyLabel(t: (k: string) => string, strategy?: string): string {
  if (strategy === "bottleneck") { return t("serenityPanel.strategyBottleneck"); }
  if (strategy === "policy") { return t("serenityPanel.strategyPolicy"); }
  if (strategy === "earnings") { return t("serenityPanel.strategyEarnings"); }
  if (strategy === "capital") { return t("serenityPanel.strategyCapital"); }
  if (strategy === "event") { return t("serenityPanel.strategyEvent"); }
  if (strategy === "technical") { return t("serenityPanel.strategyTechnical"); }
  return strategy ?? t("serenityPanel.strategyUnknown");
}

/// 渲染策略标签
function renderStrategyTag(t: (k: string) => string, strategy?: string) {
  if (!strategy) { return null; }
  return <Tag color={strategyColor(strategy)} className="text-xs ml-1">{strategyLabel(t, strategy)}</Tag>;
}

interface Props {
  candidate: SerenityCandidate;
}

export function SerenityCandidateCard({ candidate }: Props) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const code = candidate.stock_code ?? candidate.stockCode ?? "";
  const name = candidate.stockName ?? candidate.stock_name ?? "";
  const score = candidate.serenityScore ?? candidate.serenity_score ?? 0;
  const confidence = candidate.confidence ?? 0;
  const bottleneckProduct = candidate.bottleneckProduct ?? candidate.bottleneck_product;
  const primaryRisk = candidate.primaryRisk ?? candidate.primary_risk;
  const catalysts: Catalyst[] = candidate.catalysts ?? [];
  const exitSignals: ExitSignals | undefined = candidate.exit_signals ?? candidate.exitSignals;
  const attention: AttentionMetrics | undefined = candidate.attention_metrics ?? candidate.attentionMetrics;

  const tier = scoreTier(score);
  const tierColor = TIER_COLOR[tier];

  return (
    <Card
      size="small"
      hoverable
      className="w-full overflow-hidden cursor-pointer"
      styles={{ body: { padding: 0 } }}
      onClick={() => navigate(`/stock-analysis?code=${code}`, { replace: true })}
    >
      {/* ── 顶部色带 + 头部 ── */}
      <div
        className="px-3 py-2 flex items-center justify-between"
        style={{ borderLeft: `4px solid ${tierColor}`, backgroundColor: "rgba(255,255,255,0.02)" }}
      >
        <div className="flex items-center gap-2 min-w-0">
          <Text strong className="text-base truncate">{name}</Text>
          <Text type="secondary" className="text-xs font-mono">{code}</Text>
          {candidate.relevance && (
            <Tag color={relevanceColor(candidate.relevance)} className="text-xs">
              {relevanceLabel(t, candidate.relevance)}
            </Tag>
          )}
          {renderStrategyTag(t, candidate.strategy_type ?? candidate.strategyType)}
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <div className="flex items-baseline gap-1">
            <Title level={4} className="m-0" style={{ color: tierColor }}>{score}</Title>
            <Text type="secondary" className="text-xs">{t("serenityPanel.scoreSuffix")}</Text>
          </div>
        </div>
      </div>

      <div className="px-3 py-2 flex flex-col gap-2">
        {/* ── 瓶颈产品 + 主要风险 ── */}
        {(bottleneckProduct || primaryRisk) && (
          <div className="grid grid-cols-2 gap-2 text-xs">
            {bottleneckProduct && (
              <div className="flex items-start gap-1 min-w-0">
                <AimOutlined className="mt-0.5 shrink-0" style={{ color: "#722ed1" }} />
                <div className="min-w-0">
                  <Text type="secondary" className="text-[10px] block">
                    {t("serenityPanel.bottleneckProduct")}
                  </Text>
                  <Text className="text-xs break-words">{bottleneckProduct}</Text>
                </div>
              </div>
            )}
            {primaryRisk && (
              <div className="flex items-start gap-1 min-w-0">
                <AlertOutlined className="mt-0.5 shrink-0" style={{ color: "#ff4d4f" }} />
                <div className="min-w-0">
                  <Text type="secondary" className="text-[10px] block">
                    {t("serenityPanel.riskPrefix")}
                  </Text>
                  <Text type="danger" className="text-xs break-words">{primaryRisk}</Text>
                </div>
              </div>
            )}
          </div>
        )}

        {/* ── 催化剂 ── */}
        {catalysts.length > 0 && (
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-1 text-[10px] text-gray-400">
              <ThunderboltOutlined />
              <span>{t("serenityPanel.catalystLabelPrefix")}</span>
            </div>
            <div className="flex flex-wrap gap-1">
              {catalysts.map((cat, ci) => (
                <Tag
                  key={ci}
                  color={catalystColor(cat.type)}
                  className="text-xs"
                  title={cat.description}
                >
                  {catalystLabel(t, cat.type)} · {timeframeLabel(t, cat.expected_timeframe)} · {cat.confidence}%
                </Tag>
              ))}
            </div>
          </div>
        )}

        {/* ── 退出信号 ── */}
        {exitSignals?.overall_exit_urgency && (
          <div className="flex items-center gap-1">
            <AlertOutlined className="text-[10px] text-gray-400" />
            <span className="text-[10px] text-gray-400">{t("serenityPanel.exitLabelPrefix")}</span>
            <Tag color={exitUrgencyColor(exitSignals.overall_exit_urgency)} className="text-xs font-bold">
              {exitUrgencyLabel(t, exitSignals.overall_exit_urgency)}
            </Tag>
          </div>
        )}

        {/* ── 关注度 ── */}
        {attention && (
          <div className="flex flex-col gap-1">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1 text-[10px] text-gray-400">
                <FireOutlined />
                <span>{t("serenityPanel.attentionLabelPrefix")}</span>
              </div>
              {attention.attention_score != null && (
                <Progress
                  percent={attention.attention_score}
                  size="small"
                  strokeColor={attention.attention_score <= 30
                    ? "#52c41a"
                    : attention.attention_score <= 60
                    ? "#1677ff"
                    : "#ff4d4f"}
                  format={() => String(attention.attention_score ?? 0)}
                  className="w-24"
                />
              )}
            </div>
            <div className="flex flex-wrap gap-1">
              {attention.search_heat && (
                <Tag
                  color={attention.search_heat === "冷门"
                    ? "green"
                    : attention.search_heat === "热门"
                    ? "red"
                    : "default"}
                  className="text-[10px]"
                >
                  {t("serenityPanel.heatLabelPrefix")}
                  {attention.search_heat}
                </Tag>
              )}
              {attention.consensus_gap && (
                <Tag
                  color={attention.consensus_gap === "明显低估"
                    ? "green"
                    : attention.consensus_gap === "高估"
                    ? "red"
                    : "default"}
                  className="text-[10px]"
                >
                  {attention.consensus_gap}
                </Tag>
              )}
            </div>
          </div>
        )}

        {/* ── 底部置信度 ── */}
        <div className="flex items-center justify-between border-t border-white/5 pt-1.5">
          <Text type="secondary" className="text-[10px]">
            {t("serenityPanel.confidencePrefix")}
            {confidence}%
          </Text>
        </div>
      </div>
    </Card>
  );
}
