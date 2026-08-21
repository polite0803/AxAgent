// i18n-exempt: 配置映射表/业务数据字符串，非用户可见 UI 文案
import { useSettingsStore } from "@/stores";
import type { Catalyst, ChecklistItem, DashboardReport, RiskAlert } from "@/types";
import { AlertOutlined, BulbOutlined, CheckCircleOutlined, SafetyCertificateOutlined } from "@ant-design/icons";
import { Alert, Card, Checkbox, Progress, Space, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { ReportMarkdown } from "./ReportMarkdown";

const { Text, Title } = Typography;

/** 根据动作返回对应颜色 */
function actionColor(action: string): string {
  switch (action) {
    case "强烈买入":
      return "#f5222d";
    case "买入":
      return "#fa541c";
    case "增持":
      return "#fa8c16";
    case "持有":
      return "#8c8c8c";
    case "减持":
      return "#52c41a";
    case "卖出":
      return "#13c2c2";
    default:
      return "#8c8c8c";
  }
}

/** 根据趋势返回对应颜色 */
function trendColor(trend: string): string {
  switch (trend) {
    case "看多":
      return "#f5222d";
    case "看空":
      return "#52c41a";
    default:
      return "#8c8c8c";
  }
}

/** 根据风险等级返回对应颜色 */
function severityColor(severity: string): string {
  switch (severity) {
    case "高":
      return "red";
    case "中":
      return "orange";
    case "低":
      return "green";
    default:
      return "default";
  }
}

/** 格式化可选数字 */
function fmtNum(v?: number | null): string {
  if (v === null || v === undefined) {
    return "—";
  }
  return v.toFixed(2);
}

/** 风险警报区块 */
function RiskAlertsSection({ alerts }: { alerts: RiskAlert[] }) {
  const { t } = useTranslation();
  const themeMode = useSettingsStore((s) => s.settings.themeMode);
  const isDark = themeMode === "dark"
    || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  if (alerts.length === 0) {
    return null;
  }
  return (
    <Card
      size="small"
      title={
        <Space>
          <AlertOutlined />
          {t("stockAnalysis.dashboard.riskAlerts")}
        </Space>
      }
      style={{ marginTop: 12 }}
    >
      <Space orientation="vertical" style={{ width: "100%" }}>
        {alerts.map((alert, idx) => (
          <div key={idx}>
            <Tag color={severityColor(alert.severity)}>{alert.severity}</Tag>
            {alert.source && <Tag>{alert.source}</Tag>}
            <ReportMarkdown content={alert.description ?? ""} isDark={isDark} />
          </div>
        ))}
      </Space>
    </Card>
  );
}

/** 催化因素区块 */
function CatalystsSection({ catalysts }: { catalysts: Catalyst[] }) {
  const { t } = useTranslation();
  const themeMode = useSettingsStore((s) => s.settings.themeMode);
  const isDark = themeMode === "dark"
    || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  if (catalysts.length === 0) {
    return null;
  }
  return (
    <Card
      size="small"
      title={
        <Space>
          <BulbOutlined />
          {t("stockAnalysis.dashboard.catalysts")}
        </Space>
      }
      style={{ marginTop: 12 }}
    >
      <Space orientation="vertical" style={{ width: "100%" }}>
        {catalysts.map((cat, idx) => (
          <div key={idx}>
            <Tag color={cat.direction === "利好" ? "red" : "green"}>{cat.direction}</Tag>
            {cat.timeline && <Tag>{cat.timeline}</Tag>}
            <ReportMarkdown content={cat.description ?? ""} isDark={isDark} />
            {cat.confidenceScore !== null && cat.confidenceScore !== undefined && (
              <Text type="secondary" style={{ marginLeft: 8 }}>
                {t("stockAnalysis.dashboard.confidence")}: {cat.confidenceScore.toFixed(0)}%
              </Text>
            )}
          </div>
        ))}
      </Space>
    </Card>
  );
}

/** 操作检查清单区块 */
function ChecklistSection({ items }: { items: ChecklistItem[] }) {
  const { t } = useTranslation();
  const themeMode = useSettingsStore((s) => s.settings.themeMode);
  const isDark = themeMode === "dark"
    || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  if (items.length === 0) {
    return null;
  }
  return (
    <Card
      size="small"
      title={
        <Space>
          <CheckCircleOutlined />
          {t("stockAnalysis.dashboard.checklist")}
        </Space>
      }
      style={{ marginTop: 12 }}
    >
      <Space orientation="vertical" style={{ width: "100%" }}>
        {items.map((item, idx) => (
          <Checkbox key={idx} checked={item.checked} disabled>
            <Tag>{item.category}</Tag>
            <ReportMarkdown content={item.description ?? ""} isDark={isDark} />
          </Checkbox>
        ))}
      </Space>
    </Card>
  );
}

/** 决策仪表盘预览组件 */
export function DashboardReportPreview({ report }: { report: DashboardReport }) {
  const { t } = useTranslation();
  const themeMode = useSettingsStore((s) => s.settings.themeMode);
  const isDark = themeMode === "dark"
    || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  const scoreColor = report.score >= 60 ? "#52c41a" : report.score >= 30 ? "#faad14" : "#f5222d";

  return (
    <div style={{ padding: 16 }}>
      {/* 标题 */}
      <Title level={4}>
        {report.stockName}({report.stockCode}) {t("stockAnalysis.dashboard.title")}
      </Title>
      <Space size="middle" style={{ marginBottom: 12 }}>
        <Text type="secondary">📅 {report.analysisDate}</Text>
        <Text type="secondary">
          🤖 {report.llmModel ?? "—"}
        </Text>
        {!report.integrityPassed && (
          <Alert
            type="warning"
            showIcon
            title={t("stockAnalysis.dashboard.integrityWarning")}
            style={{ padding: "2px 8px" }}
          />
        )}
      </Space>

      {/* 1. 核心结论 */}
      <Card
        size="small"
        title={
          <Space>
            <SafetyCertificateOutlined />
            {t("stockAnalysis.dashboard.coreConclusion")}
          </Space>
        }
      >
        <Space wrap style={{ marginBottom: 8 }}>
          <Tag color={actionColor(report.action)} style={{ fontSize: 14, padding: "2px 8px" }}>
            {report.action}
          </Tag>
          <Tag color={trendColor(report.trend)}>{report.trend}</Tag>
          <Text>
            📊 {t("stockAnalysis.dashboard.score")}:{" "}
            <Text strong style={{ color: scoreColor }}>{report.score}/100</Text>
          </Text>
          <Text>
            🎯 {t("stockAnalysis.dashboard.confidence")}: <Text strong>{report.confidence.toFixed(0)}%</Text>
          </Text>
        </Space>
        <Progress
          percent={report.score}
          strokeColor={scoreColor}
          size="small"
          style={{ marginBottom: 8 }}
        />
        <ReportMarkdown content={report.coreConclusion} isDark={isDark} />
      </Card>

      {/* 2. 买卖点位 */}
      <Card size="small" title={t("stockAnalysis.dashboard.buySellPoints")} style={{ marginTop: 12 }}>
        <Space orientation="vertical" style={{ width: "100%" }}>
          {report.buyPointLow !== null && report.buyPointLow !== undefined
            && report.buyPointHigh !== null
            && report.buyPointHigh !== undefined && (
            <Text>
              {t("stockAnalysis.dashboard.buyRange")}:{" "}
              <Text strong>
                {fmtNum(report.buyPointLow)} - {fmtNum(report.buyPointHigh)}
              </Text>
            </Text>
          )}
          <Text>
            {t("stockAnalysis.dashboard.targetPrice")}:{" "}
            <Text strong style={{ color: "#f5222d" }}>{fmtNum(report.targetPrice)}</Text>
          </Text>
          <Text>
            {t("stockAnalysis.dashboard.stopLoss")}:{" "}
            <Text strong style={{ color: "#52c41a" }}>{fmtNum(report.stopLoss)}</Text>
          </Text>
          <Text>
            {t("stockAnalysis.dashboard.positionPct")}: <Text strong>{report.positionPct.toFixed(0)}%</Text>
          </Text>
        </Space>
      </Card>

      {/* 3. 风险警报 */}
      <RiskAlertsSection alerts={report.riskAlerts} />

      {/* 4. 催化因素 */}
      <CatalystsSection catalysts={report.catalysts} />

      {/* 5. 操作检查清单 */}
      <ChecklistSection items={report.checklist} />

      {/* 6. 最新动态 */}
      {report.latestNews && (
        <Card size="small" title={t("stockAnalysis.dashboard.latestNews")} style={{ marginTop: 12 }}>
          <ReportMarkdown content={report.latestNews} isDark={isDark} />
        </Card>
      )}

      {/* 7. 业绩预期 */}
      {report.earningsExpectation && (
        <Card
          size="small"
          title={t("stockAnalysis.dashboard.earningsExpectation")}
          style={{ marginTop: 12 }}
        >
          <ReportMarkdown content={report.earningsExpectation} isDark={isDark} />
        </Card>
      )}
    </div>
  );
}
