// SPDX-License-Identifier: AGPL-3.0-only

import { OfficeTab } from "@/components/office/OfficeTab";
import { invoke } from "@/lib/invoke";
import {
  initGatewayStatusListener,
  useConversationStore,
  useFormatCny,
  useGatewayStore,
  useProviderStore,
} from "@/stores";
import { CostByProvider, DailyUsage, DashboardStats } from "@/types";
import { Card, Col, Flex, Row, Spin, Statistic, Tabs, theme } from "antd";
import { Bot, Building2, Cpu, Database, Globe, MessageSquare, TrendingUp, Zap } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

// ── Types ──

interface DashboardData {
  conversationCount: number;
  totalMessages: number;
  totalTokens: number;
  /** 今日（本地时区）消息数（messages 表） */
  todayMessages: number;
  /** 今日（本地时区）token 数（messages 表：prompt+completion） */
  todayTokens: number;
  gatewayMetrics: {
    totalRequests: number;
    totalTokens: number;
    todayRequests: number;
    todayTokens: number;
    activeConnections: number;
  } | null;
  agentStats: {
    totalSessions: number;
    completedSessions: number;
    failedSessions: number;
    totalToolCalls: number;
  };
  providerCount: number;
  modelCount: number;
}

// ── Helpers ──

function formatNumber(n: number): string {
  if (n >= 1_000_000) { return `${(n / 1_000_000).toFixed(1)}M`; }
  if (n >= 1_000) { return `${(n / 1_000).toFixed(1)}K`; }
  return n.toLocaleString();
}

// ── Stat Card ──

function StatCard({
  icon,
  title,
  value,
  suffix,
  color,
  loading,
}: {
  icon: React.ReactNode;
  title: string;
  value: number | string;
  suffix?: string;
  color?: string;
  loading?: boolean;
}) {
  const { token } = theme.useToken();
  return (
    <Card
      size="small"
      styles={{
        body: { padding: "16px 20px" },
      }}
      style={{
        borderColor: token.colorBorderSecondary,
        background: token.colorBgContainer,
      }}
    >
      {loading
        ? (
          <div style={{ display: "flex", justifyContent: "center", padding: 12 }}>
            <Spin size="small" />
          </div>
        )
        : (
          <Flex align="center" gap={16}>
            <div
              style={{
                width: 40,
                height: 40,
                borderRadius: 10,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                background: color ? `${color}18` : token.colorFillSecondary,
                color: color || token.colorTextSecondary,
                flexShrink: 0,
              }}
            >
              {icon}
            </div>
            <div style={{ flex: 1, minWidth: 0 }}>
              <Statistic
                title={
                  <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
                    {title}
                  </span>
                }
                value={value}
                suffix={suffix}
                styles={{ content: { fontSize: 22, fontWeight: 600, color: token.colorText } }}
              />
            </div>
          </Flex>
        )}
    </Card>
  );
}

// ── Section Header ──

function SectionHeader({ title, icon }: { title: string; icon?: React.ReactNode }) {
  const { token } = theme.useToken();
  return (
    <div
      style={{
        fontSize: 14,
        fontWeight: 600,
        color: token.colorText,
        display: "flex",
        alignItems: "center",
        gap: 8,
        marginBottom: 12,
      }}
    >
      {icon}
      {title}
    </div>
  );
}

// ── Main Component ──

function DailyUsageChart({ data = [], loading }: { data: DailyUsage[]; loading: boolean }) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  // 防御：mock 默认对 get_* 命令返回 {}，导致 recharts 收到非数组
  const safeData = Array.isArray(data) ? data : [];

  if (loading) {
    return (
      <div style={{ display: "flex", justifyContent: "center", padding: 32 }}>
        <Spin size="small" />
      </div>
    );
  }

  if (safeData.length === 0) {
    return (
      <div style={{ padding: "24px 0", textAlign: "center", color: token.colorTextQuaternary, fontSize: 13 }}>
        {t("dashboard.noUsageData")}
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={240}>
      <BarChart data={safeData} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" stroke={token.colorBorderSecondary} />
        <XAxis
          dataKey="date"
          tick={{ fontSize: 11, fill: token.colorTextSecondary }}
          tickLine={false}
          axisLine={{ stroke: token.colorBorderSecondary }}
        />
        <YAxis
          tick={{ fontSize: 11, fill: token.colorTextSecondary }}
          tickLine={false}
          axisLine={false}
          tickFormatter={(v: number) => v >= 1000 ? `${(v / 1000).toFixed(0)}K` : `${v}`}
        />
        <Tooltip
          contentStyle={{
            background: token.colorBgElevated,
            border: `1px solid ${token.colorBorder}`,
            borderRadius: 8,
            fontSize: 12,
          }}
          formatter={(value, name) => (
            <span>
              {Number(value ?? 0).toLocaleString()} {name === "total_prompt_tokens"
                ? t("dashboard.inputTokens")
                : name === "total_completion_tokens"
                ? t("dashboard.outputTokens")
                : t("dashboard.totalTokens")}
            </span>
          )}
        />
        <Bar dataKey="total_prompt_tokens" stackId="a" fill="#1677ff" radius={[2, 2, 0, 0]} />
        <Bar dataKey="total_completion_tokens" stackId="a" fill="#52c41a" radius={[2, 2, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  );
}

export function DashboardPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"overview" | "office">("overview");

  return (
    <div
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <Tabs
        activeKey={activeTab}
        onChange={(k) => setActiveTab(k as "overview" | "office")}
        items={[
          {
            key: "overview",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Database size={14} />
                {t("dashboard.tabs.overview")}
              </span>
            ),
            children: <OverviewTab />,
          },
          {
            key: "office",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Building2 size={14} />
                {t("dashboard.tabs.office")}
              </span>
            ),
            children: (
              <div style={{ height: "calc(100vh - 140px)", minHeight: 600 }}>
                <OfficeTab />
              </div>
            ),
          },
        ]}
        style={{ flex: 1, minHeight: 0, padding: "0 16px" }}
        tabBarStyle={{ flexShrink: 0, marginBottom: 0 }}
        destroyInactiveTabPane
      />
    </div>
  );
}

// ── OverviewTab：原 DashboardPage 主体 ──

function OverviewTab() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  // 成本以人民币展示：后端返回 USD，按偏好汇率换算为 CNY
  const formatCny = useFormatCny();

  const conversations = useConversationStore((s) => s.conversations);
  const fetchConversations = useConversationStore((s) => s.fetchConversations);
  const gatewayMetrics = useGatewayStore((s) => s.metrics);
  const gatewayStatus = useGatewayStore((s) => s.status);
  const fetchGatewayMetrics = useGatewayStore((s) => s.fetchMetrics);
  const fetchGatewayStatus = useGatewayStore((s) => s.fetchStatus);
  const providers = useProviderStore((s) => s.providers);
  const fetchProviders = useProviderStore((s) => s.fetchProviders);

  const [loading, setLoading] = useState(true);
  const [backendStats, setBackendStats] = useState<DashboardStats | null>(null);
  const [dailyUsage, setDailyUsage] = useState<DailyUsage[]>([]);
  const [costByProvider, setCostByProvider] = useState<CostByProvider[]>([]);
  const [usageLoading, setUsageLoading] = useState(true);

  // 初次加载 + 5s 轮询 + 事件订阅
  // 事件由后端 start_gateway/stop_gateway 命令 emit "gateway-status-changed"
  useEffect(() => {
    const load = async () => {
      setLoading(true);
      setUsageLoading(true);
      const [stats, usage, cost] = await Promise.all([
        invoke<DashboardStats>("get_dashboard_stats").catch(() => null),
        invoke<DailyUsage[]>("get_usage_trend", { days: 30 }).catch(() => []),
        invoke<CostByProvider[]>("get_cost_by_provider").catch(() => []),
      ]);
      // 独立加载 store 数据，不阻塞主数据渲染
      fetchGatewayStatus().catch(() => {});
      fetchGatewayMetrics().catch(() => {});
      fetchProviders().catch(() => {});
      fetchConversations().catch(() => {});
      setBackendStats(stats);
      setDailyUsage(usage);
      setCostByProvider(cost);
      setLoading(false);
      setUsageLoading(false);
    };
    load();

    // 5s 轮询 status + metrics，参考 GatewayOverview 的实现
    const interval = setInterval(() => {
      fetchGatewayStatus().catch(() => {});
      fetchGatewayMetrics().catch(() => {});
    }, 5000);

    // 监听后端事件：网关启停时立即刷新（覆盖用户在别的页面操作网关的场景）
    const cleanupEventListener = initGatewayStatusListener();

    return () => {
      clearInterval(interval);
      cleanupEventListener();
    };
  }, [fetchGatewayMetrics, fetchGatewayStatus, fetchProviders, fetchConversations]);

  const dashboardData = useMemo<DashboardData>(() => {
    const g = gatewayMetrics;
    const stats = backendStats;
    return {
      conversationCount: stats?.total_conversations ?? conversations.length,
      totalMessages: stats?.total_messages ?? conversations.reduce((sum, c) => sum + (c.message_count || 0), 0),
      // 总 token：明确语义为"AxAgent 内聊天产生的 token"，不与 Gateway 转发的 token 混用 fallback
      totalTokens: stats?.total_tokens ?? 0,
      // 今日 token（messages 表）：AxAgent 内聊天今日消耗
      todayMessages: stats?.today_messages ?? 0,
      todayTokens: stats?.today_tokens ?? 0,
      gatewayMetrics: g
        ? {
          totalRequests: g.total_requests,
          totalTokens: g.total_tokens,
          todayRequests: g.today_requests,
          todayTokens: g.today_tokens,
          activeConnections: g.active_connections,
        }
        : null,
      agentStats: {
        totalSessions: stats?.total_agent_sessions
          ?? conversations.filter((c) => c.mode === "agent" || c.mode === "gateway").length,
        completedSessions: stats?.completed_agent_sessions
          ?? conversations.filter((c) => c.mode === "agent" || c.mode === "gateway").filter((c) =>
            c.workflow_status === "completed"
          ).length,
        failedSessions: stats?.failed_agent_sessions
          ?? conversations.filter((c) => c.mode === "agent" || c.mode === "gateway").filter((c) =>
            c.workflow_status === "failed"
          ).length,
        totalToolCalls: stats?.total_tool_calls ?? 0,
      },
      providerCount: providers.filter((p) => p.enabled).length,
      modelCount: providers
        .filter((p) => p.enabled)
        .reduce((sum, p) => sum + (p.models?.filter((m) => m.enabled).length ?? 0), 0),
    };
  }, [conversations, gatewayMetrics, providers, backendStats]);

  const data = dashboardData;
  const isLoading = loading;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 24,
        padding: "16px 0",
        overflow: "auto",
      }}
    >
      {/* ── Overview Cards ── */}
      <SectionHeader
        title={t("dashboard.overview")}
        icon={<Database size={14} color={token.colorTextSecondary} />}
      />
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} md={8} lg={6}>
          <StatCard
            icon={<MessageSquare size={18} />}
            title={t("dashboard.totalConversations")}
            value={formatNumber(data.conversationCount)}
            color={token.colorPrimary}
            loading={isLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={8} lg={6}>
          <StatCard
            icon={<Bot size={18} />}
            title={t("dashboard.totalMessages")}
            value={formatNumber(data.totalMessages)}
            color="#52c41a"
            loading={isLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={8} lg={6}>
          <StatCard
            icon={<Zap size={18} />}
            title={t("dashboard.totalTokens")}
            value={formatNumber(data.totalTokens)}
            suffix={data.totalTokens >= 1_000 ? "" : ""}
            color="#fa8c16"
            loading={isLoading}
          />
        </Col>
        <Col xs={24} sm={12} md={8} lg={6}>
          <StatCard
            icon={<Cpu size={18} />}
            title={t("dashboard.providers")}
            value={`${data.providerCount}`}
            suffix={t("dashboard.providersUnit", { count: data.modelCount })}
            color="#722ed1"
            loading={isLoading}
          />
        </Col>
      </Row>

      {/* ── Token & Gateway Section ── */}
      <Row gutter={[16, 16]}>
        {/* Token Consumption */}
        <Col xs={24} md={12}>
          <Card
            size="small"
            title={
              <SectionHeader
                title={t("dashboard.tokenConsumption")}
                icon={<TrendingUp size={14} />}
              />
            }
            styles={{ header: { paddingBottom: 0, borderBottom: "none" } }}
            style={{
              borderColor: token.colorBorderSecondary,
              background: token.colorBgContainer,
            }}
          >
            <Row gutter={[8, 8]}>
              <Col span={12}>
                <Statistic
                  title={t("dashboard.totalTokens")}
                  // 总 Token = AxAgent 内聊天 token + Gateway 转发 token（两源相加，避免 fallback 语义混淆）
                  value={formatNumber(
                    data.totalTokens + (data.gatewayMetrics?.totalTokens ?? 0),
                  )}
                  styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                />
              </Col>
              <Col span={12}>
                <Statistic
                  title={t("dashboard.gatewayActiveConnections")}
                  value={data.gatewayMetrics?.activeConnections ?? 0}
                  styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                />
              </Col>
              <Col span={12}>
                <Statistic
                  title={t("dashboard.todayRequests")}
                  // 今日请求 = AxAgent 内今日消息数 + Gateway 今日请求数
                  value={formatNumber(
                    data.todayMessages + (data.gatewayMetrics?.todayRequests ?? 0),
                  )}
                  styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                />
              </Col>
              <Col span={12}>
                <Statistic
                  title={t("dashboard.todayTokenUsage")}
                  // 今日 token = AxAgent 内聊天今日 token + Gateway 转发今日 token
                  value={formatNumber(
                    data.todayTokens + (data.gatewayMetrics?.todayTokens ?? 0),
                  )}
                  styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                />
              </Col>
            </Row>
          </Card>
        </Col>

        {/* Gateway Status */}
        <Col xs={24} md={12}>
          <Card
            size="small"
            title={
              <SectionHeader
                title={t("dashboard.gatewayStatus")}
                icon={<Globe size={14} />}
              />
            }
            styles={{ header: { paddingBottom: 0, borderBottom: "none" } }}
            style={{
              borderColor: token.colorBorderSecondary,
              background: token.colorBgContainer,
            }}
          >
            {gatewayStatus.is_running
              ? (
                <Row gutter={[8, 8]}>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.totalRequests")}
                      value={formatNumber(data.gatewayMetrics?.totalRequests ?? 0)}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.todayRequests")}
                      value={formatNumber(data.gatewayMetrics?.todayRequests ?? 0)}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.activeConnections")}
                      value={data.gatewayMetrics?.activeConnections ?? 0}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                </Row>
              )
              : (
                <div
                  style={{
                    color: token.colorTextQuaternary,
                    fontSize: 13,
                    padding: "12px 0",
                  }}
                >
                  {t("dashboard.gatewayNotRunning")}
                </div>
              )}
          </Card>
        </Col>
      </Row>

      {/* ── Agent Stats ── */}
      <Row gutter={[16, 16]}>
        <Col xs={24} md={12}>
          <Card
            size="small"
            title={
              <SectionHeader
                title={t("dashboard.agentActivity")}
                icon={<Bot size={14} />}
              />
            }
            styles={{ header: { paddingBottom: 0, borderBottom: "none" } }}
            style={{
              borderColor: token.colorBorderSecondary,
              background: token.colorBgContainer,
            }}
          >
            {isLoading
              ? (
                <div style={{ textAlign: "center", padding: 16 }}>
                  <Spin size="small" />
                </div>
              )
              : (
                <Row gutter={[8, 8]}>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.agentTotalSessions")}
                      value={data.agentStats.totalSessions}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.agentCompleted")}
                      value={data.agentStats.completedSessions}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                      valueRender={(v) => (
                        <span style={{ color: "#52c41a", fontWeight: 600, fontSize: 18 }}>
                          {v}
                        </span>
                      )}
                    />
                  </Col>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.agentFailed")}
                      value={data.agentStats.failedSessions}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                      valueRender={(v) => (
                        <span
                          style={{
                            color: data.agentStats.failedSessions > 0 ? "#ff4d4f" : token.colorText,
                            fontWeight: 600,
                            fontSize: 18,
                          }}
                        >
                          {v}
                        </span>
                      )}
                    />
                  </Col>
                </Row>
              )}
          </Card>
        </Col>

        {/* Model & Provider summary */}
        <Col xs={24} md={12}>
          <Card
            size="small"
            title={
              <SectionHeader
                title={t("dashboard.modelProviderSummary")}
                icon={<Cpu size={14} />}
              />
            }
            styles={{ header: { paddingBottom: 0, borderBottom: "none" } }}
            style={{
              borderColor: token.colorBorderSecondary,
              background: token.colorBgContainer,
            }}
          >
            {isLoading
              ? (
                <div style={{ textAlign: "center", padding: 16 }}>
                  <Spin size="small" />
                </div>
              )
              : (
                <Row gutter={[8, 8]}>
                  <Col span={12}>
                    <Statistic
                      title={t("dashboard.totalProviders")}
                      value={data.providerCount}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                  <Col span={12}>
                    <Statistic
                      title={t("dashboard.totalModels")}
                      value={data.modelCount}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                </Row>
              )}
          </Card>
        </Col>
      </Row>

      {/* ── Daily Usage Trend ── */}
      <div>
        <SectionHeader
          title={t("dashboard.usageTrend")}
          icon={<TrendingUp size={14} color={token.colorTextSecondary} />}
        />
        <Card
          size="small"
          styles={{ body: { padding: "16px 20px" } }}
          style={{
            borderColor: token.colorBorderSecondary,
            background: token.colorBgContainer,
          }}
        >
          <DailyUsageChart data={dailyUsage} loading={usageLoading} />
        </Card>
      </div>

      {/* ── Cost Overview ── */}
      <div>
        <SectionHeader
          title={t("dashboard.costOverview")}
          icon={<Zap size={14} color={token.colorTextSecondary} />}
        />
        <Row gutter={[16, 16]}>
          <Col xs={24} sm={12} md={8} lg={6}>
            <StatCard
              icon={<Zap size={18} />}
              title={t("dashboard.totalCost")}
              value={formatCny(backendStats?.total_cost_usd ?? 0)}
              color="#ff4d4f"
              loading={isLoading}
            />
          </Col>
          <Col xs={24} sm={12} md={8} lg={6}>
            <StatCard
              icon={<TrendingUp size={18} />}
              title={t("dashboard.avgCostPerSession")}
              value={backendStats && backendStats.total_agent_sessions > 0
                ? formatCny(backendStats.total_cost_usd / backendStats.total_agent_sessions, 4)
                : formatCny(0)}
              color="#1677ff"
              loading={isLoading}
            />
          </Col>
          <Col xs={24} sm={12} md={8} lg={6}>
            <StatCard
              icon={<Database size={18} />}
              title={t("dashboard.totalAgentTokens")}
              value={formatNumber(backendStats?.total_agent_tokens ?? 0)}
              color="#722ed1"
              loading={isLoading}
            />
          </Col>
          <Col xs={24} sm={12} md={8} lg={6}>
            <StatCard
              icon={<MessageSquare size={18} />}
              title={t("dashboard.dailyAvgTokens")}
              // 日均 token = 30 天总 token / 30（不是除以"有数据的天数"）
              value={dailyUsage.length > 0
                ? formatNumber(
                  Math.round(dailyUsage.reduce((s, d) => s + d.total_tokens, 0) / 30),
                )
                : "0"}
              color="#52c41a"
              loading={usageLoading}
            />
          </Col>
        </Row>
      </div>

      {/* ── Cost by Provider ── */}
      {Array.isArray(costByProvider) && costByProvider.length > 0
        ? (
          <div>
            <SectionHeader
              title={t("dashboard.costByProvider")}
              icon={<Cpu size={14} color={token.colorTextSecondary} />}
            />
            <Card
              size="small"
              styles={{ body: { padding: "16px 20px" } }}
              style={{
                borderColor: token.colorBorderSecondary,
                background: token.colorBgContainer,
              }}
            >
              <ResponsiveContainer width="100%" height={220}>
                <PieChart>
                  <Pie
                    data={Array.isArray(costByProvider) ? costByProvider : []}
                    dataKey="token_count"
                    nameKey="provider_id"
                    cx="50%"
                    cy="50%"
                    outerRadius={80}
                    label={(props) => `${props.payload.provider_id}: ${formatNumber(props.payload.token_count)}`}
                    labelLine
                  >
                    {(Array.isArray(costByProvider) ? costByProvider : []).map((_, i) => (
                      <Cell
                        key={i}
                        fill={[
                          "#1677ff",
                          "#52c41a",
                          "#faad14",
                          "#ff4d4f",
                          "#722ed1",
                          "#13c2c2",
                          "#eb2f96",
                          "#fa8c16",
                        ][i % 8]}
                      />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value, name) => (
                      <span>
                        {formatNumber(Number(value ?? 0))} {name}
                      </span>
                    )}
                    contentStyle={{
                      background: token.colorBgElevated,
                      border: `1px solid ${token.colorBorder}`,
                      borderRadius: 8,
                      fontSize: 12,
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </Card>
          </div>
        )
        : (
          <div>
            <SectionHeader
              title={t("dashboard.costByProvider")}
              icon={<Cpu size={14} color={token.colorTextSecondary} />}
            />
            <Card
              size="small"
              styles={{ body: { padding: "16px 20px" } }}
              style={{
                borderColor: token.colorBorderSecondary,
                background: token.colorBgContainer,
              }}
            >
              <div style={{ padding: "24px 0", textAlign: "center", color: token.colorTextQuaternary, fontSize: 13 }}>
                {t("dashboard.noUsageData")}
              </div>
            </Card>
          </div>
        )}
    </div>
  );
}
