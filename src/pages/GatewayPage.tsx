// SPDX-License-Identifier: AGPL-3.0-only

import {
  GatewayDiagnostics,
  GatewayKeys,
  GatewayMetrics,
  GatewayOverview,
  GatewaySettings,
  GatewayTemplates,
  QuickConnectCycleIcon,
} from "@/components/gateway";
import { GatewayMonitor } from "@/components/gateway/GatewayMonitor";
import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import { useGatewayStore } from "@/stores";
import { Activity, BarChart3, Gauge, Key, ScrollText, Settings } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

export function GatewayPage() {
  const { t } = useTranslation();
  const { fetchRequestLogs } = useGatewayStore();
  const [activeKey, setActiveKey] = useState("overview");

  const handleViewMoreLogs = useCallback(() => {
    setActiveKey("diagnostics");
    void fetchRequestLogs();
  }, [fetchRequestLogs]);

  const tabs = [
    {
      key: "overview",
      label: t("gateway.overview"),
      icon: <Gauge size={14} color={CHAT_ICON_COLORS.Gauge} />,
      children: <GatewayOverview onViewMoreLogs={handleViewMoreLogs} />,
    },
    {
      key: "keys",
      label: t("gateway.keys"),
      icon: <Key size={14} color={CHAT_ICON_COLORS.Key} />,
      children: <GatewayKeys />,
    },
    {
      key: "metrics",
      label: t("gateway.metrics"),
      icon: <BarChart3 size={14} color={CHAT_ICON_COLORS.BarChart3} />,
      children: <GatewayMetrics />,
    },
    {
      key: "diagnostics",
      label: t("gateway.logs"),
      icon: <ScrollText size={14} color={CHAT_ICON_COLORS.ScrollText} />,
      children: <GatewayDiagnostics />,
    },
    {
      key: "quickConnect",
      label: t("gateway.connectedTools"),
      icon: <QuickConnectCycleIcon size={14} />,
      children: <GatewayTemplates />,
    },
    {
      key: "settings",
      label: t("gateway.settings"),
      icon: <Settings size={14} color={CHAT_ICON_COLORS.Settings} />,
      children: <GatewaySettings />,
    },
    {
      key: "monitor",
      label: t("gateway.tab.monitor"),
      icon: <Activity size={14} color={CHAT_ICON_COLORS.Settings} />,
      children: <GatewayMonitor />,
    },
  ];

  const activeTab = tabs.find((t) => t.key === activeKey) ?? tabs[0];

  return (
    <div className="gw-layout">
      <div className="gw-tabs">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            className={`gw-tab${tab.key === activeKey ? " active" : ""}`}
            onClick={() => setActiveKey(tab.key)}
          >
            {tab.icon}
            {tab.label}
          </button>
        ))}
      </div>
      <div className="gw-body">
        {activeTab.children}
      </div>
    </div>
  );
}
