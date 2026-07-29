// SPDX-License-Identifier: AGPL-3.0-only

import { WorkflowSettings } from "@/components/settings";
import { WorkflowEditor } from "@/components/workflow";
import { ReactFlowProvider } from "@xyflow/react";
import { Tabs, theme } from "antd";
import { Store, Workflow } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { WorkflowMarketplace } from "./WorkflowMarketplace";

/**
 * 工作流页面：合并了「我的工作流」与「市场」两个 Tab。
 * 市场原为独立侧栏导航项，现作为工作流页内的二级 Tab，减少导航层级。
 */
export function WorkflowPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [editingTemplateId, setEditingTemplateId] = useState<
    string | undefined
  >(undefined);
  const [isCreatingNew, setIsCreatingNew] = useState(false);

  // 编辑器全屏模式：创建新或编辑现有时隐藏 Tabs
  if (isCreatingNew || editingTemplateId) {
    return (
      <ReactFlowProvider>
        <WorkflowEditor
          templateId={isCreatingNew ? undefined : editingTemplateId}
          onClose={() => {
            setEditingTemplateId(undefined);
            setIsCreatingNew(false);
          }}
        />
      </ReactFlowProvider>
    );
  }

  const tabItems = [
    {
      key: "editor",
      label: (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <Workflow size={14} /> {t("nav.workflow")}
        </span>
      ),
      children: (
        <div
          style={{
            backgroundColor: token.colorBgElevated,
            height: "100%",
            overflowY: "auto",
          }}
        >
          <WorkflowSettings
            onOpenEditor={(templateId?: string) => setEditingTemplateId(templateId)}
            onCreateNew={() => setIsCreatingNew(true)}
          />
        </div>
      ),
    },
    {
      key: "marketplace",
      label: (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <Store size={14} /> {t("nav.marketplace")}
        </span>
      ),
      children: <WorkflowMarketplace />,
    },
  ];

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <Tabs
        defaultActiveKey="editor"
        items={tabItems}
        className="ax-fill-tabs"
        style={{ padding: "0 16px" }}
        tabBarStyle={{ flexShrink: 0, marginBottom: 0 }}
        destroyInactiveTabPane
      />
    </div>
  );
}
