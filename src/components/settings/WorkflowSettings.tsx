// SPDX-License-Identifier: AGPL-3.0-only

import { TemplateList } from "@/components/workflow/Templates";
import type { WorkflowTemplateResponse } from "@/components/workflow/types";
import { WorkflowMarketplace } from "@/pages/WorkflowMarketplace";
import { Button, Tabs } from "antd";
import { GitBranch, Plus, Store } from "lucide-react";
import { useTranslation } from "react-i18next";

interface WorkflowSettingsProps {
  onOpenEditor?: (templateId?: string) => void;
  onCreateNew?: () => void;
}

export function WorkflowSettings({
  onOpenEditor,
  onCreateNew,
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
    <div style={{ padding: "0" }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 24,
        }}
      >
        <div>
          <Button
            type="primary"
            data-testid="workflow-create-new-btn"
            icon={<Plus size={16} />}
            onClick={handleCreateNew}
          >
            {t("settings.workflow.createNew")}
          </Button>
        </div>
      </div>

      <TemplateList
        onSelectTemplate={handleSelectTemplate}
        onCreateNew={handleCreateNew}
        onEditTemplate={handleEditTemplate}
      />
    </div>
  );

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <Tabs
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          minHeight: 0,
        }}
        tabBarStyle={{ padding: "0 24px", marginBottom: 0, flexShrink: 0 }}
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
