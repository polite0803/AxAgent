import { useWorkflowEditorStore } from "@/stores";
import type { App } from "antd";
import { App as AntApp, Button, Empty, Modal, Tag, Tooltip, Typography } from "antd";
import { Check, RotateCcw, X } from "lucide-react";
import React, { useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import type { AiChatAction, WorkflowEdge, WorkflowNode } from "../types/workflow.types";

const { Text } = Typography;

interface ActionDiffPreviewProps {
  /** null = Modal 未打开；非空 = 显示在 Modal 中 */
  actions: AiChatAction[] | null;
  /** 当前画布节点（用于查询修改/删除前的状态） */
  currentNodes: WorkflowNode[];
  /** 当前画布边（用于查询修改/删除前的状态） */
  currentEdges: WorkflowEdge[];
  /** 应用单个 action（用 store.applyAiChatAction） */
  onApply: (action: AiChatAction) => void;
  /** 应用全部（按顺序执行 onApply） */
  onApplyAll: () => void;
  /** 关闭弹窗 */
  onCancel: () => void;
  /** AntD message（用于应用成功/失败提示） */
  messageApi?: ReturnType<typeof App.useApp>["message"];
}

/**
 * AI 聊天产出工作流变更动作时的 Diff 预览弹窗。
 * 显示每条 action 的 before/after 视图，用户确认后才写入 store。
 *
 * 设计原则：
 * 1. 不可见的破坏性操作：仅在 Modal 中显示，强制用户点击"应用"
 * 2. 多 action 队列：每条 action 可独立 apply / skip
 * 3. 失败可见：apply 失败时弹窗不关闭，用户可调整
 */
export const ActionDiffPreview: React.FC<ActionDiffPreviewProps> = ({
  actions,
  currentNodes,
  currentEdges,
  onApply,
  onApplyAll,
  onCancel,
}) => {
  const { t } = useTranslation();
  const { message } = AntApp.useApp();
  const [applied, setApplied] = useState<Set<number>>(new Set());
  const [batchTxId, setBatchTxId] = useState<string | null>(null);
  const beginAiActionTransaction = useWorkflowEditorStore((s) => s.beginAiActionTransaction);
  const applyInTx = useWorkflowEditorStore((s) => s.applyAiChatActionInTransaction);
  const commitTx = useWorkflowEditorStore((s) => s.commitAiActionTransaction);
  const rollbackLast = useWorkflowEditorStore((s) => s.rollbackLastAiActionTransaction);

  const visible = actions !== null;

  const handleApplySingle = (action: AiChatAction, idx: number) => {
    try {
      onApply(action);
      setApplied(prev => new Set(prev).add(idx));
    } catch (err) {
      message.error(t("workflow.aiPanel.diffPreview.applyFailed") + ": " + String(err));
    }
  };

  const handleApplyAll = () => {
    try {
      // 事务性批量应用：拍快照 → 逐个应用 → 提交。
      // 后续任一时刻可通过 "撤销整批" 按钮一键回滚。
      const txId = beginAiActionTransaction();
      setBatchTxId(txId);
      if (actions) {
        for (const action of actions) {
          applyInTx(txId, action);
        }
        commitTx(txId);
      }
      onApplyAll();
      setApplied(new Set(actions?.map((_, i) => i) ?? []));
    } catch (err) {
      message.error(t("workflow.aiPanel.diffPreview.applyAllFailed") + ": " + String(err));
    }
  };

  const handleRollbackBatch = () => {
    rollbackLast();
    setApplied(new Set());
    setBatchTxId(null);
    message.success(t("workflow.aiPanel.diffPreview.rollbackSuccess"));
  };

  const allApplied = useMemo(
    () => actions !== null && applied.size === actions.length,
    [actions, applied],
  );

  const showRollback = batchTxId !== null && applied.size > 0;

  return (
    <Modal
      open={visible}
      title={t("workflow.aiPanel.diffPreview.title")}
      width={640}
      footer={[
        <Button key="cancel" onClick={onCancel} icon={<X size={14} />}>
          {t("workflow.aiPanel.diffPreview.cancel")}
        </Button>,
        showRollback
          ? (
            <Tooltip key="rollback" title={t("workflow.aiPanel.diffPreview.rollbackTip")}>
              <Button
                onClick={handleRollbackBatch}
                icon={<RotateCcw size={14} />}
                danger
              >
                {t("workflow.aiPanel.diffPreview.rollbackBatch")}
              </Button>
            </Tooltip>
          )
          : null,
        <Button
          key="applyAll"
          type="primary"
          onClick={handleApplyAll}
          disabled={allApplied || !actions || actions.length === 0}
          icon={<Check size={14} />}
        >
          {t("workflow.aiPanel.diffPreview.applyAll")}
        </Button>,
      ]}
      onCancel={onCancel}
      destroyOnClose
    >
      {actions && actions.length > 0
        ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 12, maxHeight: "60vh", overflowY: "auto" }}>
            {actions.map((action, idx) => (
              <ActionDiffItem
                key={idx}
                action={action}
                currentNodes={currentNodes}
                currentEdges={currentEdges}
                applied={applied.has(idx)}
                onApply={() => handleApplySingle(action, idx)}
                t={t}
              />
            ))}
          </div>
        )
        : <Empty description={t("workflow.aiPanel.diffPreview.empty")} />}
    </Modal>
  );
};

interface ActionDiffItemProps {
  action: AiChatAction;
  currentNodes: WorkflowNode[];
  currentEdges: WorkflowEdge[];
  applied: boolean;
  onApply: () => void;
  t: ReturnType<typeof useTranslation>["t"];
}

/**
 * 单条 action 的 Diff 视图（before / after 并列）。
 */
const ActionDiffItem: React.FC<ActionDiffItemProps> = ({
  action,
  currentNodes,
  currentEdges,
  applied,
  onApply,
  t,
}) => {
  const { label, color, beforeRender, afterRender } = useActionVisual(action, currentNodes, currentEdges, t);

  return (
    <div
      style={{
        border: "1px solid rgba(0,0,0,0.08)",
        borderRadius: 6,
        padding: 12,
        background: applied ? "rgba(82, 196, 26, 0.04)" : "transparent",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Tag color={color}>{label}</Tag>
          {applied && <Tag color="success">{t("workflow.aiPanel.diffPreview.applied")}</Tag>}
        </div>
        <Button
          size="small"
          type="primary"
          onClick={onApply}
          disabled={applied}
          icon={<Check size={12} />}
        >
          {t("workflow.aiPanel.diffPreview.apply")}
        </Button>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
        <div>
          <Text type="secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
            {t("workflow.aiPanel.diffPreview.before")}
          </Text>
          <div
            style={{
              padding: 8,
              background: "rgba(255, 77, 79, 0.06)",
              border: "1px solid rgba(255, 77, 79, 0.2)",
              borderRadius: 4,
              minHeight: 40,
              fontSize: 12,
              fontFamily: "monospace",
              whiteSpace: "pre-wrap",
              wordBreak: "break-all",
            }}
          >
            {beforeRender}
          </div>
        </div>
        <div>
          <Text type="secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
            {t("workflow.aiPanel.diffPreview.after")}
          </Text>
          <div
            style={{
              padding: 8,
              background: "rgba(82, 196, 26, 0.06)",
              border: "1px solid rgba(82, 196, 26, 0.2)",
              borderRadius: 4,
              minHeight: 40,
              fontSize: 12,
              fontFamily: "monospace",
              whiteSpace: "pre-wrap",
              wordBreak: "break-all",
            }}
          >
            {afterRender}
          </div>
        </div>
      </div>
    </div>
  );
};

interface ActionVisual {
  label: string;
  color: string;
  beforeRender: React.ReactNode;
  afterRender: React.ReactNode;
}

const NODE_OP_COLOR: Record<string, string> = {
  generate_workflow: "purple",
  add_node: "green",
  add_nodes: "green",
  update_node: "blue",
  modify_node: "blue",
  delete_node: "red",
  delete_nodes: "red",
};

const EDGE_OP_COLOR: Record<string, string> = {
  add_edge: "cyan",
  update_edge: "gold",
  delete_edge: "volcano",
  optimize_prompt: "geekblue",
};

function useActionVisual(
  action: AiChatAction,
  currentNodes: WorkflowNode[],
  currentEdges: WorkflowEdge[],
  t: ReturnType<typeof useTranslation>["t"],
): ActionVisual {
  const labelKey = `workflow.aiPanel.action${
    action.action_type.split("_").map(s => s.charAt(0).toUpperCase() + s.slice(1)).join("")
  }`;
  const label = t(labelKey);

  switch (action.action_type) {
    case "generate_workflow": {
      return {
        label,
        color: NODE_OP_COLOR.generate_workflow,
        beforeRender: <Text type="secondary">{t("workflow.aiPanel.diffPreview.nodesEdgesLabel", { nodes: currentNodes.length, edges: currentEdges.length })}</Text>,
        afterRender: <Text>{t("workflow.aiPanel.diffPreview.nodesEdgesLabel", { nodes: action.data.nodes.length, edges: action.data.edges.length })}</Text>,
      };
    }
    case "add_node": {
      return {
        label,
        color: NODE_OP_COLOR.add_node,
        beforeRender: <Text type="secondary">—</Text>,
        afterRender: <Text>{action.data.node.title} ({action.data.node.type})</Text>,
      };
    }
    case "add_nodes": {
      return {
        label,
        color: NODE_OP_COLOR.add_nodes,
        beforeRender: <Text type="secondary">—</Text>,
        afterRender: <Text>{t("workflow.aiPanel.diffPreview.newNodes", { count: action.data.nodes.length })}</Text>,
      };
    }
    case "update_node":
    case "modify_node": {
      const existing = currentNodes.find(n => n.id === action.data.node_id);
      const changes = action.data.changes;
      return {
        label,
        color: NODE_OP_COLOR[action.action_type],
        beforeRender: existing
          ? <Text code>{existing.title} ({existing.type})</Text>
          : <Text type="secondary">{t("workflow.aiPanel.diffPreview.nodeNotFound")}</Text>,
        afterRender: <Text code>{JSON.stringify(changes, null, 2)}</Text>,
      };
    }
    case "delete_node": {
      const existing = currentNodes.find(n => n.id === action.data.node_id);
      return {
        label,
        color: NODE_OP_COLOR.delete_node,
        beforeRender: existing
          ? <Text code>{existing.title} ({existing.type})</Text>
          : <Text type="secondary">{t("workflow.aiPanel.diffPreview.nodeNotFound")}</Text>,
        afterRender: <Text type="secondary">{t("workflow.aiPanel.diffPreview.deleted")}</Text>,
      };
    }
    case "delete_nodes": {
      const ids = action.data.node_ids;
      const existing = currentNodes.filter(n => ids.includes(n.id));
      return {
        label,
        color: NODE_OP_COLOR.delete_nodes,
        beforeRender: <Text code>{existing.map(n => n.title).join(", ")}</Text>,
        afterRender: <Text type="secondary">{t("workflow.aiPanel.diffPreview.deletedCount", { count: ids.length })}</Text>,
      };
    }
    case "add_edge": {
      const e = action.data.edge;
      return {
        label,
        color: EDGE_OP_COLOR.add_edge,
        beforeRender: <Text type="secondary">—</Text>,
        afterRender: <Text code>{e.source} → {e.target} ({e.edge_type})</Text>,
      };
    }
    case "update_edge": {
      const existing = currentEdges.find(e => e.id === action.data.edge_id);
      return {
        label,
        color: EDGE_OP_COLOR.update_edge,
        beforeRender: existing
          ? <Text code>{existing.source} → {existing.target} ({existing.edge_type})</Text>
          : <Text type="secondary">{t("workflow.aiPanel.diffPreview.edgeNotFound")}</Text>,
        afterRender: <Text code>{JSON.stringify(action.data.changes, null, 2)}</Text>,
      };
    }
    case "delete_edge": {
      const existing = currentEdges.find(e => e.id === action.data.edge_id);
      return {
        label,
        color: EDGE_OP_COLOR.delete_edge,
        beforeRender: existing
          ? <Text code>{existing.source} → {existing.target}</Text>
          : <Text type="secondary">{t("workflow.aiPanel.diffPreview.edgeNotFound")}</Text>,
        afterRender: <Text type="secondary">{t("workflow.aiPanel.diffPreview.deleted")}</Text>,
      };
    }
    case "optimize_prompt": {
      const existing = currentNodes.find(n => n.id === action.data.node_id);
      const beforeLen = existing
        ? (existing.config as Record<string, unknown>)?.system_prompt?.toString().length ?? 0
        : 0;
      const afterLen = action.data.optimized_prompt.length;
      return {
        label,
        color: EDGE_OP_COLOR.optimize_prompt,
        beforeRender: existing
          ? (
            <Text style={{ fontSize: 11 }}>
              {t("workflow.aiPanel.diffPreview.promptLength", { from: beforeLen, to: afterLen })}
            </Text>
          )
          : <Text type="secondary">{t("workflow.aiPanel.diffPreview.noBefore")}</Text>,
        afterRender: <Text style={{ fontSize: 11 }}>{action.data.optimized_prompt}</Text>,
      };
    }
  }
}

/**
 * 顶层挂载到 AIPanel 内部的 portal（避免 Modal 嵌套在 Card 中出现的层级问题）
 */
export const ActionDiffPreviewPortal: React.FC<ActionDiffPreviewProps> = (props) => {
  if (typeof document === "undefined") { return null; }
  return createPortal(<ActionDiffPreview {...props} />, document.body);
};
