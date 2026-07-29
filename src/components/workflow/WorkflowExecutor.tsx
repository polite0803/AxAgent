// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: WorkflowExecutor — 工作流执行面板

import { WorkflowLogPanel } from "@/components/workflow/WorkflowLogPanel";
import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { WorkflowDefinition, WorkflowExecution } from "@/types";
import { App, Button, Descriptions, Empty, Form, Input, Modal, Space, Tag, Typography } from "antd";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface WorkflowExecutorProps {
  workflow: WorkflowDefinition;
  open: boolean;
  onClose: () => void;
}

const statusColor: Record<string, string> = {
  waiting: "default",
  pending: "default",
  ready: "default",
  running: "processing",
  in_progress: "processing",
  success: "success",
  completed: "success",
  failed: "error",
  error: "error",
  timeout: "error",
  skipped: "default",
  cancelled: "warning",
  partially_completed: "warning",
  paused: "warning",
};

export function WorkflowExecutor({ workflow, open, onClose }: WorkflowExecutorProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [form] = Form.useForm();
  const [execution, setExecution] = useState<WorkflowExecution | null>(null);
  const isExecuting = useWorkflowStore((s) => s.isExecuting);
  const executeWorkflow = useWorkflowStore((s) => s.executeWorkflow);

  const statusLabel: Record<string, string> = useMemo(() => ({
    waiting: t("rl.status.idle"),
    pending: t("rl.status.idle"),
    ready: t("rl.status.idle"),
    running: t("rl.status.running"),
    in_progress: t("rl.status.running"),
    success: t("rl.status.completed"),
    completed: t("rl.status.completed"),
    failed: t("rl.status.failed"),
    error: t("rl.status.failed"),
    timeout: t("rl.status.failed"),
    skipped: t("workflow.executor.skipped"),
    cancelled: t("workflow.executor.cancelled"),
    partially_completed: t("workflow.executor.partiallyCompleted"),
    paused: t("workflow.executor.paused"),
  }), [t]);

  const handleExecute = useCallback(async () => {
    try {
      const values = form.getFieldsValue();
      const exec = await executeWorkflow(workflow.id, values);
      setExecution(exec);
    } catch (e) {
      message.error(String(e));
    }
  }, [form, workflow.id, executeWorkflow, message]);

  const handleClose = useCallback(() => {
    setExecution(null);
    form.resetFields();
    onClose();
  }, [form, onClose]);

  const variableEntries = Object.entries(workflow.variables);

  return (
    <Modal
      title={`${t("workflow.executor.execute")}: ${workflow.name}`}
      open={open}
      onCancel={handleClose}
      width={700}
      footer={null}
      destroyOnClose
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {/* Input variables form */}
        {variableEntries.length > 0 && !execution && (
          <div>
            <Text strong style={{ display: "block", marginBottom: 8 }}>{t("workflow.executor.inputVariables")}</Text>
            <Form form={form} layout="vertical" size="small">
              {variableEntries.map(([key, val]) => (
                <Form.Item key={key} name={key} label={key} initialValue={typeof val === "string" ? val : ""}>
                  <Input />
                </Form.Item>
              ))}
            </Form>
          </div>
        )}

        {variableEntries.length === 0 && !execution && <Empty description={t("workflow.executor.noInputVariables")} />}

        {/* 执行按钮 */}
        {!execution && (
          <Button type="primary" onClick={handleExecute} loading={isExecuting} block>
            {isExecuting ? t("workflow.executor.executing") : t("workflow.executor.execute")}
          </Button>
        )}

        {/* 执行中状态 */}
        {isExecuting && (
          <div style={{ textAlign: "center", padding: 16 }}>
            <Text type="secondary">{t("workflow.executor.executing")}</Text>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 12, justifyContent: "center" }}>
              {workflow.nodes.map((node) => (
                <Tag key={node.id} color="processing">
                  {node.label}
                </Tag>
              ))}
            </div>
          </div>
        )}

        {/* 执行结果 */}
        {execution && !isExecuting && (
          <>
            <Descriptions size="small" column={2} bordered>
              <Descriptions.Item label={t("workflow.executor.status")}>
                <Tag color={execution.status === "completed" ? "success" : "error"}>
                  {execution.status === "completed"
                    ? t("workflow.executor.executionSuccess")
                    : t("workflow.executor.executionFailed")}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t("workflow.executor.duration")}>
                {execution.finishedAt && execution.startedAt
                  ? `${((execution.finishedAt - execution.startedAt) / 1000).toFixed(1)}s`
                  : "-"}
              </Descriptions.Item>
            </Descriptions>

            {/* 节点状态 */}
            <div>
              <Text strong style={{ display: "block", marginBottom: 8 }}>
                {t("workflow.executor.nodeExecutionStatus")}
              </Text>
              <Space wrap>
                {execution.nodeStates.map((ns) => (
                  <Tag key={ns.nodeId} color={statusColor[ns.status]}>
                    {workflow.nodes.find((n) => n.id === ns.nodeId)?.label ?? ns.nodeId}: {statusLabel[ns.status]}
                  </Tag>
                ))}
              </Space>
            </div>

            {/* 输出变量 */}
            {execution.outputs && Object.keys(execution.outputs).length > 0 && (
              <div>
                <Text strong style={{ display: "block", marginBottom: 8 }}>{t("workflow.executor.outputResult")}</Text>
                <pre
                  style={{
                    backgroundColor: "var(--color-fill-tertiary)",
                    padding: 8,
                    borderRadius: 4,
                    fontSize: 12,
                    maxHeight: 120,
                    overflow: "auto",
                  }}
                >
                  {JSON.stringify(execution.outputs, null, 2)}
                </pre>
              </div>
            )}

            {/* 日志 */}
            {execution.logs.length > 0 && (
              <div>
                <Text strong style={{ display: "block", marginBottom: 8 }}>{t("workflow.executor.executionLog")}</Text>
                <WorkflowLogPanel
                  logs={execution.logs}
                  maxHeight={200}
                />
              </div>
            )}

            <Button onClick={handleClose}>{t("workflow.executor.close")}</Button>
          </>
        )}
      </div>
    </Modal>
  );
}
