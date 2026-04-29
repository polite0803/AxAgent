import { WorkflowSettings } from "@/components/settings";
import { WorkflowEditor } from "@/components/workflow";
import { theme } from "antd";
import { useState } from "react";
import { ReactFlowProvider } from "reactflow";

/**
 * Standalone workflow page accessible at /workflow route.
 * Shows template list by default, opens editor inline when a template is selected.
 */
export function WorkflowPage() {
  const { token } = theme.useToken();
  const [editingTemplateId, setEditingTemplateId] = useState<string | undefined>(undefined);
  const [isCreatingNew, setIsCreatingNew] = useState(false);

  // Show editor when creating new (no templateId) or editing existing
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

  return (
    <div style={{ backgroundColor: token.colorBgElevated, height: "100%", overflowY: "auto" }}>
      <WorkflowSettings
        onOpenEditor={(templateId?: string) => setEditingTemplateId(templateId)}
        onCreateNew={() => setIsCreatingNew(true)}
      />
    </div>
  );
}
