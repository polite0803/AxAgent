// SPDX-License-Identifier: AGPL-3.0-only

import {
  AppstoreOutlined,
  DollarOutlined,
  FileTextOutlined,
  ProjectOutlined,
  RiseOutlined,
  SearchOutlined,
  TeamOutlined,
  VideoCameraOutlined,
} from "@ant-design/icons";
import { Tabs, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";

import { ContentMediaTab } from "./opc/components/ContentMediaTab";
import { CustomersTab } from "./opc/components/CustomersTab";
import { DashboardTab } from "./opc/components/DashboardTab";
import { DemandDiscoveryTab } from "./opc/components/DemandDiscoveryTab";
import { InvoicesTab } from "./opc/components/InvoicesTab";
import { KanbanTab } from "./opc/components/KanbanTab";
import { MarketPackTab } from "./opc/components/MarketPackTab";
import { ProjectsTab } from "./opc/components/ProjectsTab";
import { SitesTab } from "./opc/components/SitesTab";
import { TalentMarketTab } from "./opc/components/TalentMarketTab";

const { Title } = Typography;

const OPC_TABS = [
  { key: "dashboard", labelKey: "opc.nav.dashboard", icon: <RiseOutlined />, component: DashboardTab },
  { key: "invoices", labelKey: "opc.nav.invoices", icon: <DollarOutlined />, component: InvoicesTab },
  { key: "customers", labelKey: "opc.nav.customers", icon: <TeamOutlined />, component: CustomersTab },
  { key: "projects", labelKey: "opc.nav.projects", icon: <ProjectOutlined />, component: ProjectsTab },
  { key: "sites", labelKey: "opc.nav.sites", icon: <FileTextOutlined />, component: SitesTab },
  { key: "content_media", labelKey: "opc.nav.contentMedia", icon: <VideoCameraOutlined />, component: ContentMediaTab },
  { key: "talent", labelKey: "opc.nav.talent", icon: <SearchOutlined />, component: TalentMarketTab },
  { key: "market", labelKey: "opc.nav.market", icon: <RiseOutlined />, component: MarketPackTab },
  { key: "demand", labelKey: "opc.nav.demand", icon: <AppstoreOutlined />, component: DemandDiscoveryTab },
  { key: "kanban", labelKey: "opc.nav.kanban", icon: <ProjectOutlined />, component: KanbanTab },
];

export function OpcPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const params = useParams();
  const [tab, setTab] = useState(params?.tab || "dashboard");

  useEffect(() => {
    const newTab = params?.tab || "dashboard";
    if (newTab !== tab) {
      setTab(newTab);
    }
  }, [params?.tab]);

  useEffect(() => {
    const handler = (e: Event) => {
      const key = (e as CustomEvent).detail as string;
      if (key) {
        setTab(key);
        navigate(`/opc/${key}`, { replace: true });
      }
    };
    window.addEventListener("opc-switch-tab", handler);
    return () => window.removeEventListener("opc-switch-tab", handler);
  }, [navigate]);

  const handleTabChange = useCallback((key: string) => {
    setTab(key);
    navigate(`/opc/${key}`, { replace: true });
  }, [navigate]);

  return (
    <div className="p-6 h-full overflow-auto">
      <Title level={3} style={{ marginBottom: 16 }}>
        <FileTextOutlined style={{ marginRight: 8 }} />
        {t("opc.title")}
      </Title>
      <Tabs
        activeKey={tab}
        onChange={handleTabChange}
        items={OPC_TABS.map((item) => ({
          key: item.key,
          label: (
            <span>
              {item.icon} {t(item.labelKey)}
            </span>
          ),
          children: <item.component />,
        }))}
      />
    </div>
  );
}

export function OpcSubPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const params = useParams();
  const [tab, setTab] = useState(params?.tab || "dashboard");

  useEffect(() => {
    const newTab = params?.tab || "dashboard";
    if (newTab !== tab) {
      setTab(newTab);
    }
  }, [params?.tab]);

  const handleTabChange = useCallback((key: string) => {
    setTab(key);
    navigate(`/opc/${key}`, { replace: true });
  }, [navigate]);

  return (
    <div className="p-6 h-full overflow-auto">
      <Title level={3} style={{ marginBottom: 16 }}>
        <FileTextOutlined style={{ marginRight: 8 }} />
        {t("opc.title")}
      </Title>
      <Tabs
        activeKey={tab}
        onChange={handleTabChange}
        items={OPC_TABS.map((item) => ({
          key: item.key,
          label: (
            <span>
              {item.icon} {t(item.labelKey)}
            </span>
          ),
          children: <item.component />,
        }))}
      />
    </div>
  );
}
