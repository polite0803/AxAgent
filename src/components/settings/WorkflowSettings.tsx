import { TemplateList } from "@/components/workflow/Templates";
import type { WorkflowTemplateResponse } from "@/components/workflow/types";
import { WorkflowMarketplace } from "@/pages/WorkflowMarketplace";
import { Button, Card, Tabs, theme } from "antd";
import { GitBranch, Plus, Store } from "lucide-react";
import { useTranslation } from "react-i18next";

interface WorkflowSettingsProps {
  onOpenEditor?: (templateId?: string) => void;
  onCreateNew?: () => void;
}

export function WorkflowSettings({ onOpenEditor, onCreateNew }: WorkflowSettingsProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

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
      console.log("Create new template");
      if (onOpenEditor) {
        onOpenEditor();
      }
    }
  };

  const renderMyWorkflows = () => (
    <div style={{ padding: "0" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 24 }}>
        <div>
          <Button type="primary" icon={<Plus size={16} />} onClick={handleCreateNew}>
            {t("settings.workflow.createNew")}
          </Button>
        </div>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <div
            className="p-3 rounded-lg"
            style={{ backgroundColor: token.colorPrimaryBg }}
          >
            <GitBranch size={24} style={{ color: token.colorPrimary }} />
          </div>
          <div style={{ flex: 1 }}>
            <h5 style={{ margin: "0 0 4px 0", fontWeight: 500 }}>{t("settings.workflow.visualEditor")}</h5>
            <p style={{ margin: 0, color: token.colorTextSecondary, fontSize: 13 }}>
              {t("settings.workflow.visualEditorDesc")}
            </p>
          </div>
          <Button onClick={() => onOpenEditor?.()}>{t("settings.workflow.openEditor")}</Button>
        </div>
      </Card>

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
        style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}
        tabBarStyle={{ padding: "0 24px", marginBottom: 0, flexShrink: 0 }}
        items={[
          {
            key: "my-workflows",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <GitBranch size={14} />
                {t("settings.workflow.myWorkflows", "我的工作流")}
              </span>
            ),
            children: renderMyWorkflows(),
          },
          {
            key: "marketplace",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Store size={14} />
                {t("settings.workflow.marketplace", "工作流市场")}
              </span>
            ),
            children: <WorkflowMarketplace />,
          },
        ]}
      />
    </div>
  );
}
