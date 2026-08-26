// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: WorkflowExecutor — 工作流执行面板（动态 UI 实时构建）

import type { JsonSchemaProperty, Variable, WorkflowTemplateResponse } from "@/components/workflow/types";
import { WorkflowLogPanel } from "@/components/workflow/WorkflowLogPanel";
import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { WorkflowDefinition, WorkflowExecution } from "@/types";
import {
  App,
  Button,
  Col,
  Descriptions,
  Divider,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
} from "antd";
import { Play, RotateCcw } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface WorkflowExecutorProps {
  workflow: WorkflowTemplateResponse;
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

/** 从 JsonSchemaProperty + Variable 推导动态表单字段 */
interface DynamicField {
  name: string;
  label: string;
  description?: string;
  type: "string" | "number" | "integer" | "boolean" | "enum" | "object" | "array";
  format?: string;
  required: boolean;
  default?: unknown;
  enumValues: unknown[];
  isSecret: boolean;
}

/** 解析 variables 的原始值类型（后端可能用字符串表达类型） */
function inferVarType(value: unknown): DynamicField["type"] {
  if (typeof value === "number") {
    return Number.isInteger(value) ? "integer" : "number";
  }
  if (typeof value === "boolean") { return "boolean"; }
  if (Array.isArray(value)) { return "array"; }
  if (value !== null && typeof value === "object") { return "object"; }
  return "string";
}

/**
 * 构建动态表单字段：
 * 优先用 inputSchema.properties（带完整类型/枚举/必填信息），
 * 再叠加 variables（提供默认值与秘密标记），schema 缺失时直接用 variables 推导。
 */
function buildDynamicFields(workflow: WorkflowTemplateResponse): DynamicField[] {
  const schemaProps = workflow.inputSchema?.properties ?? {};
  const required = new Set(workflow.inputSchema?.required ?? []);
  const varMap = new Map<string, Variable>();
  for (const v of workflow.variables ?? []) {
    varMap.set(v.name, v);
  }

  const fields: DynamicField[] = [];

  // 1) schema 定义的字段
  for (const [name, prop] of Object.entries(schemaProps as Record<string, JsonSchemaProperty>)) {
    const v = varMap.get(name);
    let type = prop.type as DynamicField["type"];
    if (prop.enumValues && prop.enumValues.length > 0) {
      type = "enum";
    }
    fields.push({
      name,
      label: name,
      description: prop.description,
      type,
      format: prop.format,
      required: required.has(name),
      default: v?.value ?? prop.default,
      enumValues: prop.enumValues ?? [],
      isSecret: v?.isSecret ?? false,
    });
  }

  // 2) 未被 schema 覆盖的 variables
  for (const v of workflow.variables ?? []) {
    if (schemaProps[v.name]) { continue; }
    fields.push({
      name: v.name,
      label: v.name,
      description: v.description,
      type: inferVarType(v.value),
      required: false,
      default: v.value,
      enumValues: [],
      isSecret: v.isSecret,
    });
  }

  return fields;
}

/** 按字段类型渲染动态控件 */
function renderFieldControl(field: DynamicField) {
  const { type, enumValues, isSecret, description } = field;

  if (type === "boolean") {
    return <Switch aria-label={field.name} />;
  }

  if (type === "number" || type === "integer") {
    return (
      <InputNumber
        style={{ width: "100%" }}
        precision={type === "integer" ? 0 : undefined}
        aria-label={field.name}
      />
    );
  }

  if (type === "enum" && enumValues.length > 0) {
    return (
      <Select
        aria-label={field.name}
        options={enumValues.map((v) => ({ value: v, label: String(v) }))}
      />
    );
  }

  if (type === "array") {
    return (
      <Select
        mode="tags"
        aria-label={field.name}
        placeholder={description}
        open={false}
        suffixIcon={null}
      />
    );
  }

  if (type === "object") {
    return (
      <Input.TextArea
        rows={4}
        aria-label={field.name}
        placeholder={description}
      />
    );
  }

  // string
  if (isSecret) {
    return <Input.Password aria-label={field.name} />;
  }
  if (field.format === "textarea" || (description?.length ?? 0) > 60) {
    return <Input.TextArea rows={3} aria-label={field.name} />;
  }
  return <Input aria-label={field.name} />;
}

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

  /** 工作流简化定义（用于节点展示） */
  const workflowDefinition: WorkflowDefinition = useMemo(() => {
    const nodes = (workflow.nodes ?? []).map((n) => ({
      id: n.id,
      type:
        ("type" in n && typeof n.type === "string" ? n.type : "action") as WorkflowDefinition["nodes"][number]["type"],
      label: n.title ?? n.id,
      config: ("config" in n ? n.config : {}) as Record<string, unknown>,
      position: n.position ?? { x: 0, y: 0 },
    }));
    const edges = (workflow.edges ?? []).map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      label: e.label,
    }));
    const variables: Record<string, unknown> = {};
    for (const v of workflow.variables ?? []) {
      variables[v.name] = v.value;
    }
    return {
      id: workflow.id,
      name: workflow.name,
      description: workflow.description ?? "",
      version: workflow.version,
      nodes,
      edges,
      variables,
      createdAt: workflow.createdAt,
      updatedAt: workflow.updatedAt,
      status: workflow.isPreset ? ("active" as const) : ("draft" as const),
    };
  }, [workflow]);

  const fields = useMemo(() => buildDynamicFields(workflow), [workflow]);

  const handleExecute = useCallback(async () => {
    try {
      const raw = form.getFieldsValue();
      // 类型归一化：number/integer → number；boolean → boolean；object 尝试 JSON.parse；空值忽略
      const inputs: Record<string, unknown> = {};
      for (const f of fields) {
        const val = raw[f.name];
        if (val === undefined || val === null || val === "") { continue; }
        if (f.type === "number" || f.type === "integer") {
          inputs[f.name] = typeof val === "number" ? val : Number(val);
        } else if (f.type === "boolean") {
          inputs[f.name] = Boolean(val);
        } else if (f.type === "object" && typeof val === "string") {
          try {
            inputs[f.name] = JSON.parse(val);
          } catch {
            inputs[f.name] = val;
          }
        } else if (f.type === "array" && Array.isArray(val)) {
          inputs[f.name] = val.map((x) => {
            if (typeof x !== "string") { return x; }
            try {
              return JSON.parse(x);
            } catch {
              return x;
            }
          });
        } else {
          inputs[f.name] = val;
        }
      }
      const exec = await executeWorkflow(workflow.id, inputs);
      setExecution(exec);
    } catch (e) {
      message.error(String(e));
    }
  }, [form, workflow.id, executeWorkflow, fields, message]);

  const handleClose = useCallback(() => {
    setExecution(null);
    form.resetFields();
    onClose();
  }, [form, onClose]);

  const handleReExecute = useCallback(() => {
    setExecution(null);
  }, []);

  return (
    <Modal
      title={`${t("workflow.executor.execute")}: ${workflow.name}`}
      open={open}
      onCancel={handleClose}
      width={720}
      footer={null}
      destroyOnHidden
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {/* 动态输入表单 */}
        {!execution && !isExecuting && (
          <>
            {fields.length > 0
              ? (
                <>
                  <Text strong style={{ display: "block" }}>
                    {t("workflow.executor.inputVariables")}
                  </Text>
                  <Form form={form} layout="vertical" size="small">
                    <Row gutter={16}>
                      {fields.map((f) => (
                        <Col span={f.type === "object" || f.type === "array" ? 24 : 12} key={f.name}>
                          <Form.Item
                            name={f.name}
                            label={
                              <Space size={4}>
                                <span>{f.label}</span>
                                {f.isSecret && (
                                  <Tag color="red" style={{ fontSize: 11, marginInlineEnd: 0 }}>
                                    {t("workflow.executor.secret")}
                                  </Tag>
                                )}
                                {f.required && (
                                  <Tag color="blue" style={{ fontSize: 11, marginInlineEnd: 0 }}>
                                    {t("workflow.executor.required")}
                                  </Tag>
                                )}
                              </Space>
                            }
                            tooltip={f.description}
                            initialValue={f.default}
                            valuePropName={f.type === "boolean" ? "checked" : "value"}
                            rules={f.required
                              ? [{ required: true, message: t("workflow.executor.requiredField", { name: f.label }) }]
                              : []}
                          >
                            {renderFieldControl(f)}
                          </Form.Item>
                        </Col>
                      ))}
                    </Row>
                  </Form>
                  <Button
                    type="primary"
                    icon={<Play size={14} />}
                    onClick={handleExecute}
                    loading={isExecuting}
                    block
                  >
                    {isExecuting ? t("workflow.executor.executing") : t("workflow.executor.execute")}
                  </Button>
                </>
              )
              : (
                <>
                  <Empty description={t("workflow.executor.noInputVariables")} />
                  <Button
                    type="primary"
                    icon={<Play size={14} />}
                    onClick={handleExecute}
                    loading={isExecuting}
                    block
                  >
                    {isExecuting ? t("workflow.executor.executing") : t("workflow.executor.execute")}
                  </Button>
                </>
              )}
          </>
        )}

        {/* 执行中状态 */}
        {isExecuting && (
          <div style={{ textAlign: "center", padding: 16 }}>
            <Text type="secondary">{t("workflow.executor.executing")}</Text>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 12, justifyContent: "center" }}>
              {workflowDefinition.nodes.map((node) => (
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
                    {workflowDefinition.nodes.find((n) => n.id === ns.nodeId)?.label ?? ns.nodeId}:{" "}
                    {statusLabel[ns.status]}
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

            <Divider style={{ margin: "4px 0" }} />

            <Space>
              <Button icon={<RotateCcw size={14} />} onClick={handleReExecute}>
                {t("workflow.executor.reExecute")}
              </Button>
              <Button type="primary" onClick={handleClose}>
                {t("workflow.executor.close")}
              </Button>
            </Space>
          </>
        )}
      </div>
    </Modal>
  );
}
