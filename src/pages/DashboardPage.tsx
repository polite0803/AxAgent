// SPDX-License-Identifier: AGPL-3.0-only

import { Icon } from "@/components/common/Icon";
import { invoke } from "@/lib/invoke";
import { useConversationStore, useGatewayStore, useProviderStore } from "@/stores";
import { DashboardStats } from "@/types";
import { Card, Col, Flex, Row, Spin, Statistic, theme } from "antd";
import { Bot, Cpu, Database, Globe, MessageSquare, TrendingUp, Zap } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

// ── Types ──

interface DashboardData {
  conversationCount: number;
  totalMessages: number;
  totalTokens: number;
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

export function DashboardPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const conversations = useConversationStore((s) => s.conversations);
  const fetchConversations = useConversationStore((s) => s.fetchConversations);
  const gatewayMetrics = useGatewayStore((s) => s.metrics);
  const fetchGatewayMetrics = useGatewayStore((s) => s.fetchMetrics);
  const providers = useProviderStore((s) => s.providers);
  const fetchProviders = useProviderStore((s) => s.fetchProviders);

  const [loading, setLoading] = useState(true);
  const [backendStats, setBackendStats] = useState<DashboardStats | null>(null);

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      const [stats] = await Promise.all([
        invoke<DashboardStats>("get_dashboard_stats").catch(() => null),
        fetchGatewayMetrics().catch(() => {}),
        fetchProviders().catch(() => {}),
        fetchConversations().catch(() => {}),
      ]);
      setBackendStats(stats);
      setLoading(false);
    };
    load();
  }, [fetchGatewayMetrics, fetchProviders, fetchConversations]);

  const dashboardData = useMemo<DashboardData>(() => {
    const g = gatewayMetrics;
    const stats = backendStats;
    return {
      conversationCount: stats?.total_conversations ?? conversations.length,
      totalMessages: stats?.total_messages ?? conversations.reduce((sum, c) => sum + (c.message_count || 0), 0),
      totalTokens: stats?.total_tokens ?? g?.total_tokens ?? 0,
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
        totalToolCalls: 0,
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
        padding: 24,
        height: "100%",
        overflow: "auto",
        display: "flex",
        flexDirection: "column",
        gap: 24,
      }}
    >
      {/* ── Page Title ── */}
      <div>
        <div
          style={{
            fontSize: 20,
            fontWeight: 700,
            color: token.colorText,
            display: "flex",
            alignItems: "center",
            gap: 10,
          }}
        >
          <Icon icon="fluent:grid-20-filled" size={22} />
          {t("nav.dashboard")}
        </div>
        <div style={{ fontSize: 12, color: token.colorTextQuaternary, marginTop: 4 }}>
          {t("appHeader.dashboardContext")}
        </div>
      </div>

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
                  value={formatNumber(data.gatewayMetrics?.totalTokens ?? data.totalTokens)}
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
                  value={formatNumber(data.gatewayMetrics?.todayRequests ?? 0)}
                  styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                />
              </Col>
              <Col span={12}>
                <Statistic
                  title={t("dashboard.todayTokenUsage")}
                  value={formatNumber(data.gatewayMetrics?.todayTokens ?? 0)}
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
            {data.gatewayMetrics
              ? (
                <Row gutter={[8, 8]}>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.totalRequests")}
                      value={formatNumber(data.gatewayMetrics.totalRequests)}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.todayRequests")}
                      value={formatNumber(data.gatewayMetrics.todayRequests)}
                      styles={{ content: { fontSize: 18, fontWeight: 600, color: token.colorText } }}
                    />
                  </Col>
                  <Col span={8}>
                    <Statistic
                      title={t("dashboard.activeConnections")}
                      value={data.gatewayMetrics.activeConnections}
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
    </div>
  );
}
