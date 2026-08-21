// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { DeleteOutlined, EditOutlined, FlagOutlined, PlusOutlined } from "@ant-design/icons";
import {
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Divider,
  Empty,
  Form,
  Input,
  InputNumber,
  message,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Table,
  Tag,
  Timeline,
  Typography,
} from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Customer, Project } from "../utils/constants";
import { getProjectStatusKey, PROJ_STATUS_COLOR_MAP } from "../utils/constants";

const { Text } = Typography;

export function ProjectsTab() {
  const { t } = useTranslation();
  const [projects, setProjects] = useState<Project[]>([]);
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<Project | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailProject, setDetailProject] = useState<Project | null>(null);
  const [milestoneOpen, setMilestoneOpen] = useState(false);
  const [filterStatus, setFilterStatus] = useState<string>("");
  const [filterCustomer, setFilterCustomer] = useState<string>("");
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [milestoneForm] = Form.useForm();
  const [form] = Form.useForm();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [proj, cust] = await Promise.all([
        invoke<Project[]>("opc_list_projects", { filter: {} }),
        invoke<Customer[]>("opc_list_customers", { filter: {} }),
      ]);
      setProjects(proj);
      setCustomers(cust);
    } catch (e) {
      message.error(t("opc.common.loadFailed", { error: String(e) }));
      setProjects([]);
      setCustomers([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleSave = async (values: Record<string, unknown>) => {
    try {
      const payload = {
        title: values.title as string,
        description: (values.description as string) || "",
        customer_id: (values.customer_id as string) || null,
        budget: (values.budget as number) || null,
        currency: "CNY",
        deadline: values.deadline ? Math.floor(new Date(values.deadline as string).getTime() / 1000) : null,
        notes: (values.notes as string) || "",
      };
      if (editing) {
        await invoke("opc_update_project", { id: editing.id, input: payload });
        message.success(t("opc.project.updated"));
      } else {
        await invoke("opc_create_project", { input: payload });
        message.success(t("opc.project.created"));
      }
      setModalOpen(false);
      setEditing(null);
      form.resetFields();
      load();
    } catch (e) {
      message.error(t("opc.common.opFailed", { error: String(e) }));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("opc_delete_project", { id });
      message.success(t("opc.project.deleted"));
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const handleAddMilestone = async (values: Record<string, unknown>) => {
    if (!detailProject) { return; }
    try {
      await invoke("opc_add_milestone", {
        projectId: detailProject.id,
        milestone: {
          id: crypto.randomUUID(),
          title: values.title as string,
          description: (values.description as string) || "",
          due_at: values.due_at ? Math.floor(new Date(values.due_at as string).getTime() / 1000) : null,
          completed_at: null,
          status: "pending",
        },
      });
      message.success(t("opc.project.milestoneAdded"));
      setMilestoneOpen(false);
      milestoneForm.resetFields();
      const updated = await invoke<Project>("opc_get_project", { id: detailProject.id });
      setDetailProject(updated);
      load();
    } catch (e) {
      message.error(t("opc.project.milestoneAddFailed", { error: String(e) }));
    }
  };

  const handleCompleteMilestone = async (milestoneId: string) => {
    if (!detailProject) { return; }
    try {
      await invoke("opc_complete_milestone", { projectId: detailProject.id, milestoneId });
      message.success(t("opc.project.milestoneCompleted"));
      const updated = await invoke<Project>("opc_get_project", { id: detailProject.id });
      setDetailProject(updated);
      load();
    } catch (e) {
      message.error(t("opc.common.opFailed", { error: String(e) }));
    }
  };

  const filteredProjects = projects.filter((p) => {
    if (filterStatus && p.status !== filterStatus) { return false; }
    if (filterCustomer && p.customer_id !== filterCustomer) { return false; }
    return true;
  });

  const handleBatchDelete = async () => {
    try {
      await Promise.all(selectedRowKeys.map((id) => invoke("opc_delete_project", { id })));
      message.success(t("opc.common.batchDeleteSuccess"));
      setSelectedRowKeys([]);
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const columns = [
    { title: t("opc.project.columnTitle"), dataIndex: "title", key: "title" },
    {
      title: t("opc.project.columnStatus"),
      key: "status",
      render: (_: unknown, r: Project) => {
        const color = PROJ_STATUS_COLOR_MAP[r.status] || "default";
        return <Tag color={color}>{t(getProjectStatusKey(r.status))}</Tag>;
      },
    },
    {
      title: t("opc.project.columnMilestones"),
      key: "milestones",
      render: (_: unknown, r: Project) => {
        const done = r.milestones.filter((m) => m.status === "completed").length;
        return r.milestones.length > 0 ? `${done}/${r.milestones.length}` : "-";
      },
    },
    {
      title: t("opc.project.columnBudget"),
      key: "budget",
      render: (_: unknown, r: Project) => (r.budget ? `¥${r.budget.toFixed(2)}` : "-"),
      sorter: (a: Project, b: Project) => (a.budget || 0) - (b.budget || 0),
    },
    {
      title: t("opc.project.columnDeadline"),
      key: "deadline",
      render: (_: unknown, r: Project) => (r.deadline ? new Date(r.deadline * 1000).toLocaleDateString() : "-"),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 160,
      render: (_: unknown, r: Project) => (
        <Space size="small">
          <Button
            size="small"
            icon={<FlagOutlined />}
            onClick={() => {
              setDetailProject(r);
              setDetailOpen(true);
            }}
          >
            {t("opc.project.details")}
          </Button>
          <Button
            size="small"
            icon={<EditOutlined />}
            onClick={() => {
              setEditing(r);
              form.setFieldsValue({
                title: r.title,
                description: r.description,
                customer_id: r.customer_id,
                budget: r.budget,
                deadline: r.deadline ? new Date(r.deadline * 1000) : null,
                notes: r.notes,
              });
              setModalOpen(true);
            }}
          />
          <Popconfirm title={t("opc.project.confirmDelete")} onConfirm={() => handleDelete(r.id)}>
            <Button size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <>
      {/* 筛选工具栏 */}
      <Card size="small" style={{ marginBottom: 12 }}>
        <Row gutter={12} align="middle">
          <Col xs={24} sm={8} md={6}>
            <Select
              placeholder={t("opc.project.filterStatus")}
              allowClear
              style={{ width: "100%" }}
              value={filterStatus || undefined}
              onChange={(val) => setFilterStatus(val || "")}
              options={[
                { value: "planning", label: t("opc.projectStatus.planning") },
                { value: "active", label: t("opc.projectStatus.active") },
                { value: "paused", label: t("opc.projectStatus.paused") },
                { value: "completed", label: t("opc.projectStatus.completed") },
                { value: "cancelled", label: t("opc.projectStatus.cancelled") },
              ]}
            />
          </Col>
          <Col xs={24} sm={8} md={6}>
            <Select
              placeholder={t("opc.project.filterCustomer")}
              allowClear
              style={{ width: "100%" }}
              value={filterCustomer || undefined}
              onChange={(val) => setFilterCustomer(val || "")}
              optionFilterProp="label"
              options={customers.map((c) => ({ value: c.id, label: `${c.name} (${c.email})` }))}
            />
          </Col>
          <Col>
            <Space>
              {selectedRowKeys.length > 0 && (
                <Button danger size="small" onClick={handleBatchDelete}>
                  {t("opc.common.batchDelete", { count: selectedRowKeys.length })}
                </Button>
              )}
              <Button
                size="small"
                onClick={() => {
                  setFilterStatus("");
                  setFilterCustomer("");
                  setSelectedRowKeys([]);
                }}
              >
                {t("opc.common.reset")}
              </Button>
            </Space>
          </Col>
        </Row>
      </Card>

      <Card
        extra={
          <Button
            type="primary"
            size="small"
            icon={<PlusOutlined />}
            onClick={() => {
              setEditing(null);
              form.resetFields();
              setModalOpen(true);
            }}
          >
            {t("opc.project.newProject")}
          </Button>
        }
      >
        <Table
          dataSource={filteredProjects}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={{ pageSize: 20 }}
          rowSelection={{
            selectedRowKeys,
            onChange: (keys) => setSelectedRowKeys(keys.map(String)),
          }}
        />
      </Card>

      {/* 新建/编辑项目 Modal */}
      <Modal
        title={editing ? t("opc.project.editTitle") : t("opc.project.newProject")}
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          setEditing(null);
          form.resetFields();
        }}
        width={560}
        okText={editing ? t("opc.common.update") : t("opc.common.create")}
        cancelText={t("opc.common.cancel")}
      >
        <Form form={form} layout="vertical" onFinish={handleSave}>
          <Form.Item
            name="title"
            label={t("opc.project.titleLabel")}
            rules={[{ required: true, message: t("opc.project.titleRequired") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="description" label={t("opc.common.description")}>
            <Input.TextArea rows={3} />
          </Form.Item>
          <Form.Item name="customer_id" label={t("opc.project.customerLabel")}>
            <Select
              allowClear
              placeholder={t("opc.project.customerPlaceholder")}
              optionFilterProp="label"
              options={customers.map((c) => ({ value: c.id, label: `${c.name} (${c.email})` }))}
            />
          </Form.Item>
          <Form.Item name="budget" label={t("opc.project.budgetLabel")}>
            <InputNumber min={0} precision={2} prefix="¥" style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="deadline" label={t("opc.common.dueDate")}>
            <DatePicker style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="notes" label={t("opc.common.notes")}>
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>

      {/* 项目详情 Modal */}
      <Modal
        title={detailProject?.title}
        open={detailOpen}
        onCancel={() => {
          setDetailOpen(false);
          setDetailProject(null);
        }}
        footer={null}
        width={520}
      >
        {detailProject && (
          <div>
            <Descriptions size="small" column={1}>
              <Descriptions.Item label={t("opc.project.columnStatus")}>
                <Tag color={PROJ_STATUS_COLOR_MAP[detailProject.status] || "default"}>
                  {t(getProjectStatusKey(detailProject.status))}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.common.description")}>
                {detailProject.description || "-"}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.project.budgetLabel")}>
                {detailProject.budget ? `¥${detailProject.budget.toFixed(2)}` : "-"}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.common.dueDate")}>
                {detailProject.deadline ? new Date(detailProject.deadline * 1000).toLocaleDateString() : "-"}
              </Descriptions.Item>
            </Descriptions>
            <Divider />
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
              <Text strong>{t("opc.project.milestonesTitle", { count: detailProject.milestones.length })}</Text>
              <Button
                size="small"
                icon={<PlusOutlined />}
                onClick={() => {
                  milestoneForm.resetFields();
                  setMilestoneOpen(true);
                }}
              >
                {t("opc.project.addMilestone")}
              </Button>
            </div>
            {detailProject.milestones.length === 0 ? <Empty description={t("opc.project.noMilestones")} /> : (
              <Timeline
                items={detailProject.milestones.map((m) => ({
                  color: m.status === "completed" ? "green" : m.status === "in_progress" ? "blue" : "gray",
                  children: (
                    <div>
                      <div style={{ display: "flex", justifyContent: "space-between" }}>
                        <Text strong>{m.title}</Text>
                        {m.status !== "completed" && (
                          <Button
                            size="small"
                            type="link"
                            onClick={() => handleCompleteMilestone(m.id)}
                          >
                            {t("opc.project.complete")}
                          </Button>
                        )}
                      </div>
                      <div>
                        <Text type="secondary">{m.description}</Text>
                      </div>
                      <div>
                        <Text type="secondary" style={{ fontSize: 12 }}>
                          {m.due_at
                            ? t("opc.project.dueBy", { date: new Date(m.due_at * 1000).toLocaleDateString() })
                            : ""}
                        </Text>
                      </div>
                    </div>
                  ),
                }))}
              />
            )}
          </div>
        )}
      </Modal>

      {/* 添加里程碑 Modal */}
      <Modal
        title={t("opc.project.milestoneModalTitle")}
        open={milestoneOpen}
        onOk={() => milestoneForm.submit()}
        onCancel={() => {
          setMilestoneOpen(false);
          milestoneForm.resetFields();
        }}
        okText={t("opc.project.milestoneOkAdd")}
        cancelText={t("opc.common.cancel")}
      >
        <Form form={milestoneForm} layout="vertical" onFinish={handleAddMilestone}>
          <Form.Item
            name="title"
            label={t("opc.project.milestoneTitleLabel")}
            rules={[{ required: true, message: t("opc.project.milestoneTitleRequired") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="description" label={t("opc.common.description")}>
            <Input.TextArea rows={2} />
          </Form.Item>
          <Form.Item name="due_at" label={t("opc.common.dueDate")}>
            <DatePicker style={{ width: "100%" }} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}
