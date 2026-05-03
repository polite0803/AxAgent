import { MemoryPage } from "@/pages/MemoryPage";
import { KnowledgePage } from "@/pages/KnowledgePage";
import { LlmWikiPage } from "@/pages/LlmWikiPage";
import { useActivePage } from "@/hooks/usePageRouting";
import { Tabs } from "antd";
import { Brain, BookOpen, Database } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function KnowledgeHubPage() {
  const { t } = useTranslation();
  const pageKey = useActivePage();
  const defaultKey = pageKey === "memory" ? "memory" : pageKey === "llm-wiki" ? "wiki" : "knowledge";
  const [activeKey, setActiveKey] = useState(defaultKey);

  const items = [
    {
      key: "knowledge",
      label: t("nav.knowledge"),
      icon: <Database size={16} />,
      children: <KnowledgePage />,
    },
    {
      key: "memory",
      label: t("nav.memory"),
      icon: <Brain size={16} />,
      children: <MemoryPage />,
    },
    {
      key: "wiki",
      label: t("nav.wiki"),
      icon: <BookOpen size={16} />,
      children: <LlmWikiPage />,
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
