import { TemplateList } from "@/components/workflow/Templates";
import type { WorkflowTemplateResponse } from "@/components/workflow/types";
import { Button, Card, Typography } from "antd";
import { GitBranch, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Title, Paragraph } = Typography;

interface WorkflowSettingsProps {
  onOpenEditor?: (templateId?: string) => void;
  onCreateNew?: () => void;
}

export function WorkflowSettings({ onOpenEditor, onCreateNew }: WorkflowSettingsProps) {
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
      console.log("Create new template");
      if (onOpenEditor) {
        onOpenEditor();
      }
    }
  };

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <Title level={4} className="mb-1">{t("settings.workflow.title")}</Title>
          <Paragraph type="secondary">{t("settings.workflow.description")}</Paragraph>
        </div>
        <Button type="primary" icon={<Plus size={16} />} onClick={handleCreateNew}>
          {t("settings.workflow.createNew")}
        </Button>
      </div>

      <Card className="mb-4">
        <div className="flex items-center gap-4">
          <div className="p-3 bg-purple-100 rounded-lg">
            <GitBranch size={24} className="text-purple-600" />
          </div>
          <div className="flex-1">
            <Title level={5} className="mb-1">{t("settings.workflow.visualEditor")}</Title>
            <Paragraph type="secondary" className="mb-0">
              {t("settings.workflow.visualEditorDesc")}
            </Paragraph>
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
}
