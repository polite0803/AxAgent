// SPDX-License-Identifier: AGPL-3.0-only

import { SystemTemplateList, TemplateList } from "@/components/workflow/Templates";
import type { WorkflowTemplateResponse } from "@/components/workflow/types";
import { WorkflowMarketplace } from "@/pages/WorkflowMarketplace";
import { Tabs } from "antd";
import { BrainCircuit, GitBranch, Store } from "lucide-react";
import { useTranslation } from "react-i18next";

interface WorkflowSettingsProps {
  onOpenEditor?: (templateId?: string) => void;
  /** 打开系统模板（认知编排器等）到工作流编辑器 */
  onOpenSystemEditor?: (templateId: string) => void;
  onCreateNew?: () => void;
  /** 运行工作流（打开执行面板） */
  onRunWorkflow?: (template: WorkflowTemplateResponse) => void;
}

export function WorkflowSettings({
  onOpenEditor,
  onOpenSystemEditor,
  onCreateNew,
  onRunWorkflow,
}: WorkflowSettingsProps) {
  const { t } = useTranslation();

  const handleSelectTemplate = (template: WorkflowTemplateResponse) => {
    if (onOpenEditor) {
      onOpenEditor(template.id);
    }
  };

  const handleEditTemplate = (template: WorkflowTemplateResponse) => {
    if (onOpenEditor) {
      onOpenEditor(template.id);
    }
  };

  const handleCreateNew = () => {
    if (onCreateNew) {
      onCreateNew();
    } else {
      if (onOpenEditor) {
        onOpenEditor();
      }
    }
  };

  const renderMyWorkflows = () => (
    <div style={{ padding: "16px 0", flex: 1, minHeight: 0, overflowY: "auto" }}>
      <TemplateList
        onSelectTemplate={handleSelectTemplate}
        onCreateNew={handleCreateNew}
        onEditTemplate={handleEditTemplate}
        onRunTemplate={onRunWorkflow}
      />
    </div>
  );

  const renderSystemTemplates = () => (
    <div style={{ padding: "16px 0", flex: 1, minHeight: 0, overflowY: "auto" }}>
      {onOpenSystemEditor ? <SystemTemplateList onOpenEditor={onOpenSystemEditor} /> : (
        <TemplateList
          onSelectTemplate={handleSelectTemplate}
          onCreateNew={handleCreateNew}
          onEditTemplate={handleEditTemplate}
        />
      )}
    </div>
  );

  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
      <Tabs
        className="ax-fill-tabs"
        style={{ padding: "0 16px", flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
        tabBarStyle={{ flexShrink: 0, marginBottom: 0 }}
        items={[
          {
            key: "my-workflows",
            label: (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
              >
                <GitBranch size={14} />
                {t("settings.workflow.myWorkflows")}
              </span>
            ),
            children: renderMyWorkflows(),
          },
          {
            key: "system-templates",
            label: (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
              >
                <BrainCircuit size={14} />
                {t("settings.workflow.systemTemplates")}
              </span>
            ),
            children: renderSystemTemplates(),
          },
          {
            key: "marketplace",
            label: (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
              >
                <Store size={14} />
                {t("settings.workflow.marketplace")}
              </span>
            ),
            children: <WorkflowMarketplace />,
          },
        ]}
      />
    </div>
  );
}
