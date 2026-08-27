// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { message } from "@/lib/toast";
import type { WorkflowTool, WorkflowToolInput, WorkflowToolStatus, WorkflowToolType } from "@/types";
import { App, Button, Form, Input, Modal, Popconfirm, Select, Space, Switch, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { Plus, RefreshCcw, Sparkles, Wrench } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface WorkflowToolsPanelProps {
  workflowId: string;
  workflowName: string;
  open: boolean;
  onClose: () => void;
}

const TYPE_OPTIONS: { value: WorkflowToolType; label: string }[] = [
  { value: "rhai_script", label: "Rhai Script" },
  { value: "workflow_dag", label: "Workflow DAG" },
  { value: "llm_function", label: "LLM Function" },
];

const STATUS_COLOR: Record<WorkflowToolStatus, string> = {
  pending: "gold",
  active: "green",
  disabled: "default",
};

/** 工作流运行时工具管理面板：查看/新增/启停/删除/统计 */
export function WorkflowToolsPanel({
  workflowId,
  workflowName,
  open,
  onClose,
}: WorkflowToolsPanelProps) {
  const { t } = useTranslation();
  const { message: appMessage } = App.useApp();
  const [form] = Form.useForm();
  const [genForm] = Form.useForm();
  const [tools, setTools] = useState<WorkflowTool[]>([]);
  const [loading, setLoading] = useState(false);
  const [editorVisible, setEditorVisible] = useState(false);
  const [editing, setEditing] = useState<WorkflowTool | null>(null);
  const [generateVisible, setGenerateVisible] = useState(false);
  const [generating, setGenerating] = useState(false);

  const loadTools = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<WorkflowTool[]>("list_workflow_tools", {
        workflowId,
      });
      setTools(Array.isArray(list) ? list : []);
    } catch (e) {
      logIpcError("list_workflow_tools")(e);
      appMessage.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [workflowId, appMessage]);

  useEffect(() => {
    if (open) {
      loadTools();
    }
  }, [open, loadTools]);

  const handleUpsert = async () => {
    try {
      const values = await form.validateFields();
      const input: WorkflowToolInput = {
        workflowId,
        toolName: values.toolName,
        toolType: values.toolType,
        description: values.description,
        code: values.code,
        inputSchema: values.inputSchema,
        source: editing?.source ?? "manual",
        status: values.status,
      };
      await invoke<WorkflowTool>("upsert_workflow_tool", { input });
      message.success(editing ? t("workflow.tools.updated") : t("workflow.tools.created"));
      setEditorVisible(false);
      setEditing(null);
      form.resetFields();
      loadTools();
    } catch (e) {
      logIpcError("upsert_workflow_tool")(e);
      message.error(String(e));
    }
  };

  /** AI 生成工具（发现闭环）：LLM 生成 Rhai 脚本 → 沙箱验证 → pending 待确认 */
  const handleGenerate = async () => {
    try {
      const values = await genForm.validateFields();
      setGenerating(true);
      const tool = await invoke<WorkflowTool>("generate_workflow_tool", {
        workflowId,
        description: values.description,
        availableTools: values.availableTools?.length ? values.availableTools : [],
      });
      message.success(t("workflow.tools.generated"));
      setGenerateVisible(false);
      genForm.resetFields();
      // 弹出编辑确认（pending → active）
      openEdit(tool);
      loadTools();
    } catch (e) {
      logIpcError("generate_workflow_tool")(e);
      message.error(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const handleToggleStatus = async (tool: WorkflowTool) => {
    const next: WorkflowToolStatus = tool.status === "active" ? "disabled" : "active";
    try {
      await invoke<boolean>("update_workflow_tool_status", { id: tool.id, status: next });
      message.success(t("workflow.tools.statusUpdated"));
      loadTools();
    } catch (e) {
      logIpcError("update_workflow_tool_status")(e);
      message.error(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke<boolean>("delete_workflow_tool", { id });
      message.success(t("workflow.tools.deleted"));
      loadTools();
    } catch (e) {
      logIpcError("delete_workflow_tool")(e);
      message.error(String(e));
    }
  };

  const openCreate = () => {
    setEditing(null);
    form.resetFields();
    form.setFieldsValue({ toolType: "rhai_script", status: "active" });
    setEditorVisible(true);
  };

  const openEdit = (tool: WorkflowTool) => {
    setEditing(tool);
    form.setFieldsValue({
      toolName: tool.toolName,
      toolType: tool.toolType,
      description: tool.description,
      code: tool.code,
      inputSchema: tool.inputSchema,
      status: tool.status,
    });
    setEditorVisible(true);
  };

  const columns = useMemo<ColumnsType<WorkflowTool>>(() => [
    {
      title: t("workflow.tools.toolName"),
      dataIndex: "toolName",
      key: "toolName",
      render: (name: string, tool) => (
        <Space size={4} direction="vertical" style={{ gap: 0 }}>
          <Text style={{ fontSize: 13 }}>{name}</Text>
          {tool.description && (
            <Text type="secondary" style={{ fontSize: 12, maxWidth: 220 }} ellipsis>
              {tool.description}
            </Text>
          )}
        </Space>
      ),
    },
    {
      title: t("workflow.tools.toolType"),
      dataIndex: "toolType",
      key: "toolType",
      width: 130,
      render: (v: WorkflowToolType) => <Tag>{v}</Tag>,
    },
    {
      title: t("workflow.tools.status"),
      dataIndex: "status",
      key: "status",
      width: 90,
      render: (v: WorkflowToolStatus) => <Tag color={STATUS_COLOR[v]}>{t(`workflow.tools.status.${v}`)}</Tag>,
    },
    {
      title: t("workflow.tools.source"),
      dataIndex: "source",
      key: "source",
      width: 140,
      render: (v: string) => <Text style={{ fontSize: 12 }}>{v}</Text>,
    },
    {
      title: t("workflow.tools.stats"),
      key: "stats",
      width: 120,
      render: (_, tool) => (
        <Space size={4} style={{ fontSize: 12 }} direction="vertical">
          <Text style={{ fontSize: 12 }}>
            {t("workflow.tools.usageCount")}: {tool.usageCount}
          </Text>
          <Text style={{ fontSize: 12 }}>
            {t("workflow.tools.successRate")}: {(tool.successRate * 100).toFixed(0)}%
          </Text>
        </Space>
      ),
    },
    {
      title: t("workflow.tools.actions"),
      key: "actions",
      width: 150,
      render: (_, tool) => (
        <Space size={4}>
          <Button type="link" size="small" onClick={() => openEdit(tool)}>
            {t("workflow.tools.edit")}
          </Button>
          <Switch
            size="small"
            checked={tool.status === "active"}
            onChange={() => handleToggleStatus(tool)}
            aria-label={t("workflow.tools.toggleStatus")}
          />
          <Popconfirm
            title={t("workflow.tools.confirmDelete")}
            onConfirm={() => handleDelete(tool.id)}
          >
            <Button type="link" size="small" danger>
              {t("workflow.tools.delete")}
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ], [t]);

  return (
    <Modal
      title={
        <Space>
          <Wrench size={16} />
          <span>{t("workflow.tools.title")}</span>
          <Text type="secondary" style={{ fontSize: 12 }}>{workflowName}</Text>
        </Space>
      }
      open={open}
      onCancel={onClose}
      width={820}
      footer={null}
      destroyOnHidden
    >
      <Space style={{ marginBottom: 12 }}>
        <Button
          type="primary"
          size="small"
          icon={<Plus size={14} />}
          onClick={openCreate}
        >
          {t("workflow.tools.create")}
        </Button>
        <Button
          size="small"
          icon={<Sparkles size={14} />}
          onClick={() => setGenerateVisible(true)}
        >
          {t("workflow.tools.aiGenerate")}
        </Button>
        <Button
          size="small"
          icon={<RefreshCcw size={14} />}
          onClick={loadTools}
          loading={loading}
        >
          {t("workflow.tools.refresh")}
        </Button>
      </Space>

      <Table<WorkflowTool>
        rowKey="id"
        size="small"
        loading={loading}
        columns={columns}
        dataSource={tools}
        pagination={false}
        locale={{ emptyText: t("workflow.tools.empty") }}
      />

      <Modal
        title={editing ? t("workflow.tools.editTitle") : t("workflow.tools.createTitle")}
        open={editorVisible}
        onOk={handleUpsert}
        onCancel={() => {
          setEditorVisible(false);
          setEditing(null);
        }}
        okText={t("workflow.tools.save")}
        cancelText={t("workflow.tools.cancel")}
        width={640}
        destroyOnHidden
      >
        <Form form={form} layout="vertical" size="small">
          <Form.Item
            name="toolName"
            label={t("workflow.tools.toolName")}
            rules={[{ required: true, message: t("workflow.tools.toolNameRequired") }]}
          >
            <Input disabled={!!editing} />
          </Form.Item>
          <Form.Item
            name="toolType"
            label={t("workflow.tools.toolType")}
            rules={[{ required: true }]}
          >
            <Select options={TYPE_OPTIONS} />
          </Form.Item>
          <Form.Item name="description" label={t("workflow.tools.description")}>
            <Input />
          </Form.Item>
          <Form.Item name="code" label={t("workflow.tools.code")}>
            <Input.TextArea
              rows={6}
              style={{ fontFamily: "var(--font-mono, monospace)", fontSize: 12 }}
              placeholder={t("workflow.tools.codePlaceholder")}
            />
          </Form.Item>
          <Form.Item name="inputSchema" label={t("workflow.tools.inputSchema")}>
            <Input.TextArea rows={3} style={{ fontFamily: "var(--font-mono, monospace)", fontSize: 12 }} />
          </Form.Item>
          <Form.Item name="status" label={t("workflow.tools.status")}>
            <Select
              options={[
                { value: "pending", label: t("workflow.tools.status.pending") },
                { value: "active", label: t("workflow.tools.status.active") },
                { value: "disabled", label: t("workflow.tools.status.disabled") },
              ]}
            />
          </Form.Item>
        </Form>
      </Modal>

      {/* AI 生成工具（发现闭环） */}
      <Modal
        title={t("workflow.tools.aiGenerateTitle")}
        open={generateVisible}
        onOk={handleGenerate}
        onCancel={() => {
          setGenerateVisible(false);
          genForm.resetFields();
        }}
        confirmLoading={generating}
        okText={t("workflow.tools.aiGenerate")}
        cancelText={t("workflow.tools.cancel")}
        width={560}
        destroyOnHidden
      >
        <Form form={genForm} layout="vertical" size="small">
          <Form.Item
            name="description"
            label={t("workflow.tools.aiGenerateDescription")}
            rules={[{ required: true, message: t("workflow.tools.aiGenerateRequired") }]}
          >
            <Input.TextArea
              rows={4}
              placeholder={t("workflow.tools.aiGeneratePlaceholder")}
            />
          </Form.Item>
          <Form.Item
            name="availableTools"
            label={t("workflow.tools.aiGenerateAvailableTools")}
            tooltip={t("workflow.tools.aiGenerateAvailableToolsHint")}
          >
            <Select
              mode="tags"
              open={false}
              suffixIcon={null}
              placeholder={t("workflow.tools.aiGenerateAvailableToolsPlaceholder")}
            />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("workflow.tools.aiGenerateHint")}
          </Text>
        </Form>
      </Modal>
    </Modal>
  );
}
