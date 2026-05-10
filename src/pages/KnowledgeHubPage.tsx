import SourceManager from "@/components/settings/SourceManager";
import { useActivePage } from "@/hooks/usePageRouting";
import { KnowledgePage } from "@/pages/KnowledgePage";
import { MemoryPage } from "@/pages/MemoryPage";
import { Tabs } from "antd";
import { BookOpen, Brain, Database, Layers } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

export function KnowledgeHubPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const pageKey = useActivePage();
  const defaultKey = pageKey === "memory" ? "memory" : pageKey === "llm-wiki" ? "wiki" : "sources";
  const [activeKey, setActiveKey] = useState(defaultKey);

  useEffect(() => {
    if (activeKey === "wiki") {
      navigate("/wiki");
    }
  }, [activeKey, navigate]);

  const items = [
    {
      key: "sources",
      label: t("sourceManager.tab.all"),
      icon: <Layers size={16} />,
      children: <SourceManager />,
    },
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
      children: null,
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
