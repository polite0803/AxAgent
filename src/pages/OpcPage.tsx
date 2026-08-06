// SPDX-License-Identifier: AGPL-3.0-only

import {
  DollarOutlined,
  FileTextOutlined,
  ProjectOutlined,
  RiseOutlined,
  SearchOutlined,
  TeamOutlined,
} from "@ant-design/icons";
import { Tabs, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";

import { CustomersTab } from "./opc/components/CustomersTab";
import { DashboardTab } from "./opc/components/DashboardTab";
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
  { key: "talent", labelKey: "opc.nav.talent", icon: <SearchOutlined />, component: TalentMarketTab },
  { key: "market", labelKey: "opc.nav.market", icon: <RiseOutlined />, component: MarketPackTab },
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
  const params = useParams();
  const tab = params?.tab || "dashboard";
  const Component = OPC_TABS.find((item) => item.key === tab)?.component || DashboardTab;

  return (
    <div className="p-6 h-full overflow-auto">
      <Component />
    </div>
  );
}
