import {
  GatewayDiagnostics,
  GatewayKeys,
  GatewayMetrics,
  GatewayOverview,
  GatewaySettings,
  GatewayTemplates,
  QuickConnectCycleIcon,
} from "@/components/gateway";
import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import { useGatewayStore } from "@/stores";
import { GatewayMonitor } from "@/components/gateway/GatewayMonitor";
import { Tabs } from "antd";
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

  const items = [
    {
      key: "overview",
      label: t("gateway.overview"),
      icon: <Gauge size={16} color={CHAT_ICON_COLORS.Gauge} />,
      children: <GatewayOverview onViewMoreLogs={handleViewMoreLogs} />,
    },
    {
      key: "keys",
      label: t("gateway.keys"),
      icon: <Key size={16} color={CHAT_ICON_COLORS.Key} />,
      children: <GatewayKeys />,
    },
    {
      key: "metrics",
      label: t("gateway.metrics"),
      icon: <BarChart3 size={16} color={CHAT_ICON_COLORS.BarChart3} />,
      children: <GatewayMetrics />,
    },
    {
      key: "diagnostics",
      label: t("gateway.logs"),
      icon: <ScrollText size={16} color={CHAT_ICON_COLORS.ScrollText} />,
      children: <GatewayDiagnostics />,
    },
    {
      key: "quickConnect",
      label: t("gateway.connectedTools"),
      icon: <QuickConnectCycleIcon size={16} />,
      children: <GatewayTemplates />,
    },
    {
      key: "settings",
      label: t("gateway.settings"),
      icon: <Settings size={16} color={CHAT_ICON_COLORS.Settings} />,
      children: <GatewaySettings />,
    },
    {
      key: "monitor",
      label: "监控",
      icon: <Activity size={16} color={CHAT_ICON_COLORS.Settings} />,
      children: <GatewayMonitor />,
    },
  ];

  return (
    <div className="h-full flex flex-col px-2" style={{ overflow: "hidden" }} data-testid="gateway-overview">
      <Tabs
        items={items}
        activeKey={activeKey}
        onChange={setActiveKey}
        className="flex-1"
        style={{ display: "flex", flexDirection: "column", minHeight: 0 }}
        tabBarStyle={{ flexShrink: 0 }}
      />
      <style>
        {`
        .h-full > .ant-tabs > .ant-tabs-content-holder {
          flex: 1;
          overflow-y: auto;
          overflow-x: hidden;
          min-height: 0;
        }
      `}
      </style>
    </div>
  );
}
