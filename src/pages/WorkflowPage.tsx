// SPDX-License-Identifier: AGPL-3.0-only

import { WorkflowSettings } from "@/components/settings";
import { WorkflowEditor, WorkflowExecutor } from "@/components/workflow";
import type { WorkflowTemplateResponse } from "@/components/workflow/types";
import { ReactFlowProvider } from "@xyflow/react";
import { useState } from "react";

/**
 * 工作流页面：包含「我的工作流」与「市场」两个 Tab（由 WorkflowSettings 内部提供）。
 * 编辑器全屏模式：创建新或编辑现有时隐藏列表，直接展示编辑器。
 * 运行模式：打开执行面板（动态 UI 表单 + 实时执行结果）。
 */
export function WorkflowPage() {
  const [editingTemplateId, setEditingTemplateId] = useState<
    string | undefined
  >(undefined);
  const [isEditingSystem, setIsEditingSystem] = useState(false);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [runningTemplate, setRunningTemplate] = useState<
    WorkflowTemplateResponse | null
  >(null);

  // 编辑器全屏模式：创建新或编辑现有时隐藏 Tabs
  if (isCreatingNew || editingTemplateId) {
    return (
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
        <ReactFlowProvider>
          <WorkflowEditor
            templateId={isCreatingNew ? undefined : editingTemplateId}
            isSystemTemplate={isEditingSystem}
            onClose={() => {
              setEditingTemplateId(undefined);
              setIsEditingSystem(false);
              setIsCreatingNew(false);
            }}
          />
        </ReactFlowProvider>
      </div>
    );
  }

  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
      <WorkflowSettings
        onOpenEditor={(templateId?: string) => setEditingTemplateId(templateId)}
        onOpenSystemEditor={(templateId: string) => {
          setEditingTemplateId(templateId);
          setIsEditingSystem(true);
        }}
        onCreateNew={() => setIsCreatingNew(true)}
        onRunWorkflow={(template) => setRunningTemplate(template)}
      />
      {runningTemplate && (
        <WorkflowExecutor
          workflow={runningTemplate}
          open
          onClose={() => setRunningTemplate(null)}
        />
      )}
    </div>
  );
}
