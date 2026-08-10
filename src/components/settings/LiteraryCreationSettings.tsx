import { Tabs } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LiteraryCreationConfigPanel } from "./LiteraryCreationConfigPanel";

export function LiteraryCreationSettings({ defaultTab }: { defaultTab?: string } = {}) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState(defaultTab ?? "params");

  return (
    <div className="p-6 pb-12">
      <Tabs
        size="small"
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: "params",
            label: t("literaryCreation.settings.tab.params"),
            children: <LiteraryCreationConfigPanel />,
          },
        ]}
      />
    </div>
  );
}
