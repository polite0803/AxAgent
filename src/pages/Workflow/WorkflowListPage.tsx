// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: WorkflowListPage — 工作流列表页

import { useWorkflowStore } from "@/stores/feature/workflowStore";
import {
  CopyOutlined,
  DeleteOutlined,
  EditOutlined,
  FileAddOutlined,
  SearchOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import { Button, Card, Col, Empty, Input, Popconfirm, Row, Select, Space, Tag, Typography } from "antd";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Text, Title } = Typography;

export function WorkflowListPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchText, setSearchText] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const workflows = useWorkflowStore((s) => s.workflows);
  const createWorkflow = useWorkflowStore((s) => s.createWorkflow);
  const deleteWorkflow = useWorkflowStore((s) => s.deleteWorkflow);
  const duplicateWorkflow = useWorkflowStore((s) => s.duplicateWorkflow);

  const statusConfig: Record<string, { color: string; label: string }> = useMemo(() => ({
    draft: { color: "default", label: t("workflow.list.draft") },
    active: { color: "success", label: t("workflow.list.active") },
    archived: { color: "warning", label: t("workflow.list.archived") },
  }), [t]);

  const filtered = useMemo(() => {
    return workflows.filter((wf) => {
      if (statusFilter !== "all" && wf.status !== statusFilter) { return false; }
      if (searchText) {
        const q = searchText.toLowerCase();
        if (!wf.name.toLowerCase().includes(q) && !wf.description.toLowerCase().includes(q)) { return false; }
      }
      return true;
    });
  }, [workflows, searchText, statusFilter]);

  const handleCreate = useCallback(async () => {
    const wf = await createWorkflow({ name: t("workflow.list.createNew") });
    navigate(`/workflows/${wf.id}/edit`);
  }, [createWorkflow, navigate, t]);

  const handleEdit = useCallback(
    (id: string) => {
      navigate(`/workflows/${id}/edit`);
    },
    [navigate],
  );

  const handleDuplicate = useCallback(
    async (id: string) => {
      await duplicateWorkflow(id);
    },
    [duplicateWorkflow],
  );

  const handleDelete = useCallback(
    async (id: string) => {
      await deleteWorkflow(id);
    },
    [deleteWorkflow],
  );

  const handleTemplates = useCallback(() => {
    navigate("/workflows/templates");
  }, [navigate]);

  return (
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      {/* 工具栏 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 20,
          flexWrap: "wrap",
          gap: 12,
        }}
      >
        <Title level={4} style={{ margin: 0 }}>{t("workflow.list.title")}</Title>
        <Space wrap>
          <Input
            placeholder={t("workflow.list.searchPlaceholder")}
            prefix={<SearchOutlined />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 220 }}
            allowClear
          />
          <Select
            value={statusFilter}
            onChange={setStatusFilter}
            style={{ width: 110 }}
            options={[
              { value: "all", label: t("workflow.list.all") },
              { value: "draft", label: t("workflow.list.draft") },
              { value: "active", label: t("workflow.list.active") },
              { value: "archived", label: t("workflow.list.archived") },
            ]}
          />
          <Button icon={<ThunderboltOutlined />} onClick={handleTemplates}>
            {t("workflow.list.templates")}
          </Button>
          <Button type="primary" icon={<FileAddOutlined />} onClick={handleCreate}>
            {t("workflow.list.createNew")}
          </Button>
        </Space>
      </div>

      {/* 统计 */}
      <div style={{ marginBottom: 16, display: "flex", gap: 16 }}>
        {(["all", "draft", "active", "archived"] as const).map((s) => {
          const count = s === "all" ? workflows.length : workflows.filter((w) => w.status === s).length;
          return (
            <Card key={s} size="small" style={{ flex: 1, textAlign: "center" }} bodyStyle={{ padding: "10px 16px" }}>
              <Text type="secondary" style={{ fontSize: 11 }}>
                {s === "all" ? t("workflow.list.all") : statusConfig[s]?.label}
              </Text>
              <div style={{ fontSize: 22, fontWeight: 700 }}>{count}</div>
            </Card>
          );
        })}
      </div>

      {/* 列表 */}
      {filtered.length === 0 ? <Empty description={t("workflow.list.empty")} /> : (
        <Row gutter={[16, 16]}>
          {filtered.map((wf) => (
            <Col key={wf.id} xs={24} sm={12} lg={8} xl={6}>
              <Card
                hoverable
                onClick={() => handleEdit(wf.id)}
                style={{ cursor: "pointer", height: "100%" }}
                bodyStyle={{ padding: 16 }}
              >
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                    marginBottom: 8,
                  }}
                >
                  <Text strong ellipsis style={{ maxWidth: 180 }}>{wf.name}</Text>
                  <Tag color={statusConfig[wf.status]?.color}>{statusConfig[wf.status]?.label}</Tag>
                </div>
                <Text
                  type="secondary"
                  ellipsis
                  style={{ display: "block", marginBottom: 12, fontSize: 12, minHeight: 36 }}
                >
                  {wf.description || t("workflow.list.noDescription")}
                </Text>
                <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
                  <Tag>{t("workflow.list.nodeCount", { count: wf.nodes.length })}</Tag>
                  <Tag>{t("workflow.list.edgeCount", { count: wf.edges.length })}</Tag>
                  <Tag color="blue">v{wf.version}</Tag>
                </div>
                <Text type="secondary" style={{ fontSize: 11, display: "block", marginBottom: 8 }}>
                  {new Date(wf.updatedAt).toLocaleString()}
                </Text>
                <div onClick={(e) => e.stopPropagation()} style={{ display: "flex", gap: 4 }}>
                  <Button size="small" icon={<EditOutlined />} onClick={() => handleEdit(wf.id)}>
                    {t("workflow.list.edit")}
                  </Button>
                  <Button size="small" icon={<CopyOutlined />} onClick={() => handleDuplicate(wf.id)}>
                    {t("workflow.list.duplicate")}
                  </Button>
                  <Popconfirm
                    title={t("workflow.list.deleteConfirm")}
                    onConfirm={() => handleDelete(wf.id)}
                  >
                    <Button size="small" danger icon={<DeleteOutlined />} />
                  </Popconfirm>
                </div>
              </Card>
            </Col>
          ))}
        </Row>
      )}
    </div>
  );
}
