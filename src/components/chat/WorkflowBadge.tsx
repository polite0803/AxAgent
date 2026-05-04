import { Button, Spin, Tag, theme } from "antd";
import { CheckCircle, Loader2, Workflow } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import WorkflowTemplateSelector from "./WorkflowTemplateSelector";
import type { WorkflowTemplate } from "./WorkflowTemplateSelector";

export interface WorkflowBadgeProps {
  sessionType: "conversation" | "workflow";
  workflowTemplateId: string | null | undefined;
  workflowStatus: string | null | undefined;
  onSelectWorkflow: (templateId: string) => void;
  onRemoveWorkflow: () => void;
  disabled?: boolean;
}

export function WorkflowBadge({
  sessionType,
  workflowTemplateId,
  workflowStatus,
  onSelectWorkflow,
  onRemoveWorkflow,
  disabled,
}: WorkflowBadgeProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [selectorOpen, setSelectorOpen] = useState(false);

  const isRunning = workflowStatus === "running";
  const isCompleted = workflowStatus === "completed";
  const isWorkflow = sessionType === "workflow" && workflowTemplateId;

  const handleSelect = (template: WorkflowTemplate) => {
    setSelectorOpen(false);
    onSelectWorkflow(template.id);
  };

  const style = useMemo(() => {
    if (isCompleted) {
      return {
        color: token.colorSuccess,
        borderColor: token.colorSuccessBorder,
        bg: token.colorSuccessBg,
      };
    }
    if (isRunning) {
      return {
        color: token.colorPrimary,
        borderColor: token.colorPrimaryBorder,
        bg: token.colorPrimaryBg,
      };
    }
    if (isWorkflow) {
      return {
        color: token.colorWarning,
        borderColor: token.colorWarningBorder,
        bg: token.colorWarningBg,
      };
    }
    return {
      color: token.colorTextSecondary,
      borderColor: token.colorBorder,
      bg: "transparent",
    };
  }, [isRunning, isCompleted, isWorkflow, token]);

  // 对话型会话：显示轻量选择器
  if (sessionType === "conversation" && !isWorkflow) {
    return (
      <>
        <Button
          size="small"
          type="text"
          disabled={disabled}
          onClick={() => setSelectorOpen(true)}
          style={{
            fontSize: 12,
            color: token.colorTextSecondary,
            padding: "0 8px",
            height: 28,
            border: `1px dashed ${token.colorBorder}`,
            borderRadius: token.borderRadiusSM,
          }}
        >
          <Workflow size={12} style={{ marginRight: 4 }} />
          {t("chat.workflow.selectHint", "选择工作流")}
        </Button>
        <WorkflowTemplateSelector
          open={selectorOpen}
          onClose={() => setSelectorOpen(false)}
          onSelect={(template) => handleSelect(template.id)}
        />
      </>
    );
  }

  // 工作流型会话：显示工作流名称 + 状态
  return (
    <Tag
      style={{
        margin: 0,
        cursor: isRunning || isCompleted ? "default" : "pointer",
        color: style.color,
        borderColor: style.borderColor,
        background: style.bg,
        fontSize: 12,
        display: "inline-flex",
        alignItems: "center",
        gap: 4,
        opacity: disabled ? 0.6 : 1,
      }}
      closable={!isRunning && !isCompleted}
      onClose={(e) => {
        e.preventDefault();
        onRemoveWorkflow();
      }}
      icon={isRunning
        ? <Spin size="small" indicator={<Loader2 size={12} />} />
        : isCompleted
        ? <CheckCircle size={12} />
        : <Workflow size={12} />}
    >
      {workflowTemplateId || t("chat.workflow.unnamed", "工作流")}
    </Tag>
  );
}
