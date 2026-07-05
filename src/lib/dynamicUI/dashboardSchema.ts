// SPDX-License-Identifier: AGPL-3.0-only

/**
 * DashboardPage → DynamicUI Schema 工厂。
 *
 * 将运行时 DashboardData 转换为 DynamicUIRenderer 可消费的 UISchema。
 * 保持与原始 DashboardPage.tsx 完全相同的视觉布局与功能。
 */

import type { UISchema } from "@/types";
import type { TFunction } from "i18next";

// ── Helpers ──

function formatNumber(n: number): string {
  if (n >= 1_000_000) { return `${(n / 1_000_000).toFixed(1)}M`; }
  if (n >= 1_000) { return `${(n / 1_000).toFixed(1)}K`; }
  return n.toLocaleString();
}

// ── Types ──

export interface DashboardData {
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
  messageTokenSummary: {
    total_tokens: number;
    today_tokens: number;
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

// ── Schema Factory ──

export function createDashboardSchema(
  data: DashboardData,
  isLoading: boolean,
  t: TFunction,
): UISchema {
  const g = data.gatewayMetrics;
  const m = data.messageTokenSummary;

  // ── Overview stat cards ──
  const overviewItems: Record<string, unknown>[] = [
    {
      label: t("dashboard.totalConversations"),
      value: formatNumber(data.conversationCount),
      icon: "MessageSquare",
      color: "#1677ff",
      loading: isLoading,
    },
    {
      label: t("dashboard.totalMessages"),
      value: formatNumber(data.totalMessages),
      icon: "Bot",
      color: "#52c41a",
      loading: isLoading,
    },
    {
      label: t("dashboard.totalTokens"),
      value: formatNumber(data.totalTokens),
      icon: "Zap",
      color: "#fa8c16",
      loading: isLoading,
    },
    {
      label: t("dashboard.providers"),
      value: `${data.providerCount}`,
      unit: t("dashboard.providersUnit", { count: data.modelCount }),
      icon: "Cpu",
      color: "#722ed1",
      loading: isLoading,
    },
  ];

  // ── Token consumption items (inside Card) ──
  const tokenItems: Record<string, unknown>[] = [
    {
      label: t("dashboard.totalTokens"),
      value: formatNumber(m?.total_tokens ?? data.totalTokens),
    },
    {
      label: t("dashboard.todayTokenUsage"),
      value: formatNumber(m?.today_tokens ?? 0),
    },
  ];

  // ── Gateway items (inside Card) or placeholder ──
  const hasGateway = g !== null;
  const gatewayItems: Record<string, unknown>[] = hasGateway
    ? [
      { label: t("dashboard.totalRequests"), value: formatNumber(g!.totalRequests) },
      { label: t("dashboard.todayRequests"), value: formatNumber(g!.todayRequests) },
      { label: t("dashboard.activeConnections"), value: g!.activeConnections },
    ]
    : [];

  // ── Agent activity items ──
  const agentItems: Record<string, unknown>[] = isLoading
    ? [
      { label: t("dashboard.agentTotalSessions"), value: "-", loading: true },
      { label: t("dashboard.agentCompleted"), value: "-", loading: true },
      { label: t("dashboard.agentFailed"), value: "-", loading: true },
    ]
    : [
      { label: t("dashboard.agentTotalSessions"), value: data.agentStats.totalSessions },
      {
        label: t("dashboard.agentCompleted"),
        value: data.agentStats.completedSessions,
        color: "#52c41a",
      },
      {
        label: t("dashboard.agentFailed"),
        value: data.agentStats.failedSessions,
        color: data.agentStats.failedSessions > 0 ? "#ff4d4f" : undefined,
      },
    ];

  // ── Model/Provider summary items ──
  const modelItems: Record<string, unknown>[] = isLoading
    ? [
      { label: t("dashboard.totalProviders"), value: "-", loading: true },
      { label: t("dashboard.totalModels"), value: "-", loading: true },
    ]
    : [
      { label: t("dashboard.totalProviders"), value: data.providerCount },
      { label: t("dashboard.totalModels"), value: data.modelCount },
    ];

  // ── Build children ──
  const children: UISchema[] = [
    // ── Page Title ──
    {
      version: "1.0",
      id: "dashboard-title-block",
      type: "Container",
      props: { display: "flex" },
      style: { flexDirection: "column", gap: "4px" },
      children: [
        {
          version: "1.0",
          id: "dashboard-title",
          type: "Text",
          props: {
            content: t("nav.dashboard"),
            level: undefined,
            strong: true,
          },
          style: { fontSize: "20px", fontWeight: 700 },
        },
        {
          version: "1.0",
          id: "dashboard-subtitle",
          type: "Text",
          props: {
            content: t("appHeader.dashboardContext"),
            type: "secondary",
          },
          style: { fontSize: "12px", marginTop: "4px" },
        },
      ],
    },

    // ── Overview Section ──
    {
      version: "1.0",
      id: "dashboard-overview-header",
      type: "Text",
      props: {
        content: t("dashboard.overview"),
        strong: true,
      },
      style: { fontSize: "14px", fontWeight: 600, marginTop: "0px" },
    },
    {
      version: "1.0",
      id: "dashboard-overview-cards",
      type: "Dashboard",
      props: {
        columns: 4,
        gap: 16,
        variant: "cards",
        items: overviewItems,
      },
    },

    // ── Token & Gateway Row ──
    {
      version: "1.0",
      id: "dashboard-token-gateway-row",
      type: "Row",
      props: { gap: 16, align: "stretch" },
      style: { width: "100%" },
      children: [
        // Token Consumption card
        {
          version: "1.0",
          id: "dashboard-token-card",
          type: "Card",
          props: {
            title: t("dashboard.tokenConsumption"),
            size: "small",
          },
          style: { flex: 1 },
          children: [
            {
              version: "1.0",
              id: "dashboard-token-stats",
              type: "Dashboard",
              props: {
                columns: 2,
                gap: 8,
                variant: "inline",
                items: tokenItems,
              },
            },
          ],
        },
        // Gateway Status card
        {
          version: "1.0",
          id: "dashboard-gateway-card",
          type: "Card",
          props: {
            title: t("dashboard.gatewayStatus"),
            size: "small",
          },
          style: { flex: 1 },
          children: hasGateway
            ? [
              {
                version: "1.0",
                id: "dashboard-gateway-stats",
                type: "Dashboard",
                props: {
                  columns: 3,
                  gap: 8,
                  variant: "inline",
                  items: gatewayItems,
                },
              },
            ]
            : [
              {
                version: "1.0",
                id: "dashboard-gateway-empty",
                type: "Text",
                props: {
                  content: t("dashboard.gatewayNotRunning"),
                  type: "secondary",
                },
                style: { fontSize: "13px", padding: "12px 0" },
              },
            ],
        },
      ],
    },

    // ── Agent & Model Row ──
    {
      version: "1.0",
      id: "dashboard-agent-model-row",
      type: "Row",
      props: { gap: 16, align: "stretch" },
      style: { width: "100%" },
      children: [
        // Agent Activity card
        {
          version: "1.0",
          id: "dashboard-agent-card",
          type: "Card",
          props: {
            title: t("dashboard.agentActivity"),
            size: "small",
          },
          style: { flex: 1 },
          children: [
            {
              version: "1.0",
              id: "dashboard-agent-stats",
              type: "Dashboard",
              props: {
                columns: 3,
                gap: 8,
                variant: "inline",
                items: agentItems,
              },
            },
          ],
        },
        // Model Provider Summary card
        {
          version: "1.0",
          id: "dashboard-model-card",
          type: "Card",
          props: {
            title: t("dashboard.modelProviderSummary"),
            size: "small",
          },
          style: { flex: 1 },
          children: [
            {
              version: "1.0",
              id: "dashboard-model-stats",
              type: "Dashboard",
              props: {
                columns: 2,
                gap: 8,
                variant: "inline",
                items: modelItems,
              },
            },
          ],
        },
      ],
    },
  ];

  return {
    version: "1.0",
    id: "dashboard-page-root",
    type: "Container",
    props: {
      display: "flex",
      padding: 24,
    },
    style: {
      height: "100%",
      overflow: "auto",
      flexDirection: "column",
      gap: "24px",
    },
    children,
  };
}
