// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: WorkflowExecutor — 工作流执行面板

import { WorkflowLogPanel } from "@/components/workflow/WorkflowLogPanel";
import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { WorkflowDefinition, WorkflowExecution } from "@/types/workflow";
import { Button, Descriptions, Form, Input, Modal, Space, Tag, Typography } from "antd";
import { useCallback, useState } from "react";

const { Text, Title } = Typography;

interface WorkflowExecutorProps {
  workflow: WorkflowDefinition;
  open: boolean;
  onClose: () => void;
}

const statusColor: Record<string, string> = {
  waiting: "default",
  running: "processing",
  success: "success",
  failed: "error",
};

const statusLabel: Record<string, string> = {
  waiting: "等待",
  running: "执行中",
  success: "成功",
  failed: "失败",
};

export function WorkflowExecutor({ workflow, open, onClose }: WorkflowExecutorProps) {
  const [form] = Form.useForm();
  const [execution, setExecution] = useState<WorkflowExecution | null>(null);
  const isExecuting = useWorkflowStore((s) => s.isExecuting);
  const executeWorkflow = useWorkflowStore((s) => s.executeWorkflow);

  const handleExecute = useCallback(async () => {
    const values = form.getFieldsValue();
    const exec = await executeWorkflow(workflow.id, values);
    setExecution(exec);
  }, [form, workflow.id, executeWorkflow]);

  const handleClose = useCallback(() => {
    setExecution(null);
    form.resetFields();
    onClose();
  }, [form, onClose]);

  const variableEntries = Object.entries(workflow.variables);

  return (
    <Modal
      title={`执行工作流: ${workflow.name}`}
      open={open}
      onCancel={handleClose}
      width={700}
      footer={null}
      destroyOnClose
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {/* 输入变量表单 */}
        {variableEntries.length > 0 && !execution && (
          <div>
            <Text strong style={{ display: "block", marginBottom: 8 }}>输入变量</Text>
            <Form form={form} layout="vertical" size="small">
              {variableEntries.map(([key, val]) => (
                <Form.Item key={key} name={key} label={key} initialValue={typeof val === "string" ? val : ""}>
                  <Input />
                </Form.Item>
              ))}
            </Form>
          </div>
        )}

        {/* 执行按钮 */}
        {!execution && (
          <Button type="primary" onClick={handleExecute} loading={isExecuting} block>
            {isExecuting ? "执行中..." : "执行工作流"}
          </Button>
        )}

        {/* 执行中状态 */}
        {isExecuting && (
          <div style={{ textAlign: "center", padding: 16 }}>
            <Text type="secondary">正在执行工作流节点...</Text>
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
              <Descriptions.Item label="状态">
                <Tag color={execution.status === "completed" ? "success" : "error"}>
                  {execution.status === "completed" ? "执行成功" : "执行失败"}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="耗时">
                {execution.finishedAt && execution.startedAt
                  ? `${((execution.finishedAt - execution.startedAt) / 1000).toFixed(1)}s`
                  : "-"}
              </Descriptions.Item>
            </Descriptions>

            {/* 节点状态 */}
            <div>
              <Text strong style={{ display: "block", marginBottom: 8 }}>节点执行状态</Text>
              <Space wrap>
                {execution.nodeStates.map((ns) => (
                  <Tag key={ns.nodeId} color={statusColor[ns.status]}>
                    {workflow.nodes.find((n) => n.id === ns.nodeId)?.label ?? ns.nodeId}:{" "}
                    {statusLabel[ns.status]}
                  </Tag>
                ))}
              </Space>
            </div>

            {/* 输出变量 */}
            {execution.outputs && Object.keys(execution.outputs).length > 0 && (
              <div>
                <Text strong style={{ display: "block", marginBottom: 8 }}>输出结果</Text>
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
                <Text strong style={{ display: "block", marginBottom: 8 }}>执行日志</Text>
                <WorkflowLogPanel
                  logs={execution.logs}
                  onClear={() => {}}
                  onExport={() => {}}
                  maxHeight={200}
                />
              </div>
            )}

            <Button onClick={handleClose}>关闭</Button>
          </>
        )}
      </div>
    </Modal>
  );
}
