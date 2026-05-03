import { useActivePage } from "@/hooks/usePageRouting";
import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import { GatewayPage } from "@/pages/GatewayPage";
import { LinkPage } from "@/pages/LinkPage";
import { Tabs } from "antd";
import { Link2, Router } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function GatewayLinkPage() {
  const { t } = useTranslation();
  const pageKey = useActivePage();

  // 根据当前路由确定默认 tab
  const defaultKey = pageKey === "gateway" ? "gateway" : "link";
  const [activeKey, setActiveKey] = useState(defaultKey);

  const items = [
    {
      key: "gateway",
      label: t("gateway.title"),
      icon: <Router size={16} color={CHAT_ICON_COLORS.Settings} />,
      children: <GatewayPage />,
    },
    {
      key: "link",
      label: t("nav.link"),
      icon: <Link2 size={16} color={CHAT_ICON_COLORS.Settings} />,
      children: <LinkPage />,
    },
  ];

  return (
    <div className="h-full flex flex-col" style={{ overflow: "hidden" }}>
      <Tabs
        items={items}
        activeKey={activeKey}
        onChange={setActiveKey}
        className="flex-1"
        style={{ display: "flex", flexDirection: "column", minHeight: 0 }}
        tabBarStyle={{ flexShrink: 0, paddingLeft: 4 }}
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
