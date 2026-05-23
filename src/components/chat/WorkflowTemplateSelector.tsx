import type { WorkflowTemplateResponse } from "@/components/workflow/types";
import { invoke } from "@/lib/invoke";
import { Card, Input, Modal, Spin, Tag } from "antd";
import { ArrowRight, LayoutTemplate, MessageCircle } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** 工作流步骤定义（从后端 nodes 提取） */
interface WorkflowStepDef {
  id: string;
  goal: string;
  role: string;
  needs: string[];
  agentProfileId?: string;
}

/** 选择器中使用的模板类型 */
export interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  tags: string[];
  /** 从后端 nodes 提取的步骤 */
  steps?: WorkflowStepDef[];
  /** 从后端 edges 复用的连线 */
  edges?: { source: string; target: string }[];
  /** 是否场景限定模板 */
  scenarios?: string[];
}

/**
 * 后端模板并非为聊天选择器设计——没有 systemPrompt / initialMessage 等聊天专用字段。
 * 此处仅列出模板供选择，选择后由调用方决定如何将模板注入对话上下文。
 * 有 steps 的模板会走 workflow_create → workflow_execute 流程。
 */
function mapBackendTemplate(
  bt: WorkflowTemplateResponse,
): WorkflowTemplate {
  const steps: WorkflowStepDef[] = (bt.nodes || [])
    .filter((n) => n.type === "agent")
    .map((n) => ({
      id: n.id,
      goal: n.title || n.description || n.id,
      role: (n as unknown as Record<string, unknown>).role as string || "coder",
      needs: (bt.edges || [])
        .filter((e) => e.target === n.id)
        .map((e) => e.source),
    }));

  // 从 edges 提取简化的连线信息
  const edges = (bt.edges || []).map((e) => ({
    source: e.source,
    target: e.target,
  }));

  return {
    id: bt.id,
    name: bt.name,
    description: bt.description || "",
    tags: bt.tags || [],
    steps: steps.length > 0 ? steps : undefined,
    edges: edges.length > 0 ? edges : undefined,
  };
}

interface WorkflowTemplateSelectorProps {
  open: boolean;
  onClose: () => void;
  onSelect: (template: WorkflowTemplate, workflowId?: string) => void;
  scenario?: string | null;
  expertCategory?: string | null;
}

/**
 * 保留向后兼容的空导出——旧代码可能引用此函数。
 * 模板数据现在统一从后端 `list_workflow_templates` 获取。
 */
export const getWorkflowTemplates = (): WorkflowTemplate[] => [];

export const WorkflowTemplateSelector: React.FC<
  WorkflowTemplateSelectorProps
> = ({ open, onClose, onSelect, scenario, expertCategory }) => {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [creatingWorkflow, setCreatingWorkflow] = useState<string | null>(null);
  const [allTemplates, setAllTemplates] = useState<WorkflowTemplate[]>([]);
  const [loading, setLoading] = useState(false);

  // 从后端加载模板列表（与设置页「我的工作流」同一数据源）
  const loadTemplates = useCallback(async () => {
    setLoading(true);
    try {
      const backend = await invoke<WorkflowTemplateResponse[]>(
        "list_workflow_templates",
        {},
      );
      setAllTemplates(backend.map(mapBackendTemplate));
    } catch {
      setAllTemplates([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) {
      loadTemplates();
    }
  }, [open, loadTemplates]);

  // Expert category → scenario 映射
  const EXPERT_TO_SCENARIO: Record<string, string> = {
    development: "coding",
    security: "coding",
    data: "analysis",
    devops: "coding",
    design: "coding",
    writing: "writing",
    business: "analysis",
  };

  const resolvedScenario = scenario
    || (expertCategory ? EXPERT_TO_SCENARIO[expertCategory] : null);

  const filteredTemplates = allTemplates.filter((template) => {
    const matchesSearch = template.name
      .toLowerCase()
      .includes(searchQuery.toLowerCase())
      || template.description
        .toLowerCase()
        .includes(searchQuery.toLowerCase())
      || template.tags.some((tag) => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesScenario = !resolvedScenario
      || !template.scenarios
      || template.scenarios.length === 0
      || template.scenarios.includes(resolvedScenario);
    return matchesSearch && matchesScenario;
  });

  // 按场景匹配优先级排序
  const sortedTemplates = [...filteredTemplates].sort((a, b) => {
    if (resolvedScenario) {
      const aMatch = a.scenarios?.includes(resolvedScenario);
      const bMatch = b.scenarios?.includes(resolvedScenario);
      if (aMatch && !bMatch) { return -1; }
      if (!aMatch && bMatch) { return 1; }
    }
    return 0;
  });

  const handleSelect = async (template: WorkflowTemplate) => {
    if (template.steps && template.steps.length > 0) {
      setCreatingWorkflow(template.id);
      try {
        // 使用后端模板已有的 nodes + edges 创建工作流实例
        const backend = await invoke<WorkflowTemplateResponse>(
          "get_workflow_template",
          { id: template.id },
        );

        const nodes = (backend.nodes || []).map((n) => ({
          type: n.type,
          id: n.id,
          title: n.title || n.description || n.id,
          description: n.description || "",
          position: n.position || { x: 0, y: 0 },
          retry: (n as unknown as Record<string, unknown>).retry || {
            enabled: true,
            max_retries: 2,
            backoff_type: "exponential",
            base_delay_ms: 1000,
            max_delay_ms: 30000,
          },
          timeout: null,
          enabled: true,
        }));

        const edges = (backend.edges || []).map((e) => ({
          id: e.id,
          source: e.source,
          target: e.target,
          edge_type: e.edge_type || "direct",
        }));

        const result = await invoke<{
          workflowId: string;
          name: string;
          stepCount: number;
        }>("workflow_create", {
          request: { name: template.name, nodes, edges },
        });

        try {
          await invoke("workflow_execute", { workflowId: result.workflowId });
        } catch (execErr) {
          console.error(
            "[WorkflowTemplateSelector] Workflow created but execution failed:",
            execErr,
          );
        }

        onSelect(template, result.workflowId);
      } catch (e) {
        console.error(
          "[WorkflowTemplateSelector] Failed to create workflow:",
          e,
        );
        onSelect(template);
      } finally {
        setCreatingWorkflow(null);
      }
    } else {
      onSelect(template);
    }
  };

  return (
    <Modal
      title={t("chat.workflow.title")}
      open={open}
      onCancel={onClose}
      footer={null}
      width={720}
    >
      <Input
        placeholder={t("chat.workflow.searchPlaceholder")}
        value={searchQuery}
        onChange={(e) => setSearchQuery(e.target.value)}
        style={{ marginBottom: 16 }}
        allowClear
      />

      {loading
        ? (
          <div className="flex justify-center py-12">
            <Spin />
          </div>
        )
        : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {/* 对话模式 — 默认选项 */}
            <Card
              key="conversation-mode"
              size="small"
              hoverable
              onClick={() => {
                onSelect({
                  id: "",
                  name: t("chat.workflow.conversationMode"),
                  description: t("chat.workflow.conversationModeDesc"),
                  tags: [],
                });
              }}
              className="cursor-pointer border-dashed"
              style={{ borderStyle: "dashed" }}
            >
              <div className="flex items-start gap-3">
                <div className="shrink-0 text-zinc-400 mt-0.5">
                  <MessageCircle size={20} />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-sm text-zinc-500">
                    {t("chat.workflow.conversationMode")}
                  </div>
                  <div className="text-xs text-zinc-400 mt-1">
                    {t("chat.workflow.conversationModeDesc")}
                  </div>
                </div>
              </div>
            </Card>
            {sortedTemplates.map((template) => (
              <Card
                key={template.id}
                size="small"
                hoverable
                onClick={() => handleSelect(template)}
                className="cursor-pointer"
                loading={creatingWorkflow === template.id}
              >
                <div className="flex items-start gap-3">
                  <div className="shrink-0 text-blue-500 mt-0.5">
                    <LayoutTemplate size={20} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm">
                      {template.name}
                    </div>
                    <div className="text-xs text-zinc-500 mt-1 line-clamp-2">
                      {template.description}
                    </div>
                    <div className="flex flex-wrap gap-1 mt-2">
                      {template.tags.map((tag) => (
                        <Tag key={tag} className="text-xs py-0 leading-tight">
                          {tag}
                        </Tag>
                      ))}
                    </div>
                  </div>
                  <ArrowRight size={14} className="text-zinc-400 shrink-0 mt-1" />
                </div>
              </Card>
            ))}
          </div>
        )}
    </Modal>
  );
};
