// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { AgentPanelTab } from "@/stores/shared/agentPanelStore";
import { Tabs } from "antd";

const TAB_ITEMS: { key: AgentPanelTab; label: string }[] = [
  { key: "chat", label: "对话" },
  { key: "execution", label: "执行追踪" },
  { key: "skill", label: "技能" },
  { key: "nl-generation", label: "NL 生成" },
];

export function AgentPanelTabs() {
  const activeTab = useAgentPanelStore((s) => s.activeTab);
  const setTab = useAgentPanelStore((s) => s.setTab);

  return (
    <div className="px-2 pt-1 shrink-0">
      <Tabs
        size="small"
        activeKey={activeTab}
        onChange={(key) => setTab(key as AgentPanelTab)}
        items={TAB_ITEMS.map((item) => ({
          key: item.key,
          label: <span className="text-xs">{item.label}</span>,
        }))}
        tabBarStyle={{ marginBottom: 0 }}
      />
    </div>
  );
}
