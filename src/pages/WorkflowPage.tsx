// SPDX-License-Identifier: AGPL-3.0-only

import { WorkflowSettings } from "@/components/settings";
import { WorkflowEditor } from "@/components/workflow";
import { ReactFlowProvider } from "@xyflow/react";
import { useState } from "react";

/**
 * 工作流页面：包含「我的工作流」与「市场」两个 Tab（由 WorkflowSettings 内部提供）。
 * 编辑器全屏模式：创建新或编辑现有时隐藏列表，直接展示编辑器。
 */
export function WorkflowPage() {
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

  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
      <WorkflowSettings
        onOpenEditor={(templateId?: string) => setEditingTemplateId(templateId)}
        onCreateNew={() => setIsCreatingNew(true)}
      />
    </div>
  );
}
