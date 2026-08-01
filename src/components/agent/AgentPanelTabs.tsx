// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { AgentPanelTab } from "@/stores/shared/agentPanelStore";
import { Tabs } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

export function AgentPanelTabs() {
  const { t } = useTranslation();
  const activeTab = useAgentPanelStore((s) => s.activeTab);
  const setTab = useAgentPanelStore((s) => s.setTab);
  const agentUISchemaCount = useAgentPanelStore((s) => s.agentUISchemas.length);

  const tabItems = useMemo(() => [
    { key: "chat" as AgentPanelTab, label: t("agentPanel.tabs.chat") },
    { key: "execution" as AgentPanelTab, label: t("agentPanel.tabs.execution") },
    { key: "skill" as AgentPanelTab, label: t("agentPanel.tabs.skill") },
    {
      key: "ui" as AgentPanelTab,
      label: `${t("agentPanel.tabs.ui", { defaultValue: "UI" })}${
        agentUISchemaCount > 0 ? ` (${agentUISchemaCount})` : ""
      }`,
    },
    { key: "nl-generation" as AgentPanelTab, label: t("agentPanel.tabs.nlGeneration") },
  ], [t, agentUISchemaCount]);

  return (
    <div className="px-2 pt-1 shrink-0">
      <Tabs
        size="small"
        activeKey={activeTab}
        onChange={(key) => setTab(key as AgentPanelTab)}
        items={tabItems.map((item) => ({
          key: item.key,
          label: <span className="text-xs">{item.label}</span>,
        }))}
        tabBarStyle={{ marginBottom: 0 }}
      />
    </div>
  );
}
