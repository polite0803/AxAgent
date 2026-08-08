// SPDX-License-Identifier: AGPL-3.0-only

import { WorkflowSettings } from "@/components/settings";
import { WorkflowEditor } from "@/components/workflow";
import { ReactFlowProvider } from "@xyflow/react";
import { useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";

/**
 * 工作流页面：包含「我的工作流」与「市场」两个 Tab（由 WorkflowSettings 内部提供）。
 * 编辑器全屏模式：创建新或编辑现有时隐藏列表，直接展示编辑器。
 */
export function WorkflowPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [editingTemplateId, setEditingTemplateId] = useState<
    string | undefined
  >(undefined);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const urlInitDoneRef = useRef(false);

  // URL query 参数初始化（仅页面挂载时执行一次）
  useEffect(() => {
    if (urlInitDoneRef.current) {
      return;
    }
    const template = searchParams.get("template");
    const industry = searchParams.get("industry");

    if (!template && !industry) {
      urlInitDoneRef.current = true;
      return;
    }

    urlInitDoneRef.current = true;

    if (template) {
      setEditingTemplateId(template);
    } else if (industry) {
      // 仅有 industry 参数时，进入创建模式
      setIsCreatingNew(true);
    }

    // 清理 URL 参数
    setSearchParams({}, { replace: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 编辑器全屏模式：创建新或编辑现有时隐藏 Tabs
  if (isCreatingNew || editingTemplateId) {
    return (
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
        <ReactFlowProvider>
          <WorkflowEditor
            templateId={isCreatingNew ? undefined : editingTemplateId}
            onClose={() => {
              setEditingTemplateId(undefined);
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
        onCreateNew={() => setIsCreatingNew(true)}
      />
    </div>
  );
}
