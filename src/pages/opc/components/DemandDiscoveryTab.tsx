// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import {
  AppstoreOutlined,
  BugOutlined,
  DeleteOutlined,
  EditOutlined,
  FireOutlined,
  GlobalOutlined,
  PlusOutlined,
  SearchOutlined,
  SettingOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Dropdown,
  Form,
  Input,
  List,
  message,
  Modal,
  Popconfirm,
  Progress,
  Row,
  Space,
  Switch,
  Table,
  Tabs,
  Tag,
  Timeline,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { DemandDiscoveryConfigPanel } from "@/components/settings/DemandDiscoveryConfigPanel";

import type {
  CapabilityEntry,
  CapabilityGap,
  CapabilityInventory,
  Delivery,
  DemandLead,
  MarketPlatform,
} from "../utils/constants";
import { DELIVERY_STATUS_COLOR_MAP, LEAD_STATUS_COLOR_MAP } from "../utils/constants";

const { TextArea } = Input;

// 后端 opc_demand_lead 实体返回的字段为 JSON 字符串（*_json）与 snake_case，
// 前端类型定义的是已解析的对象字段，这里做一次映射转换。
function mapLead(row: DemandLead): DemandLead {
  const toJson = (v: unknown): Record<string, unknown> => {
    if (typeof v === "string") {
      try {
        return JSON.parse(v);
      } catch {
        return {};
      }
    }
    return (v ?? {}) as Record<string, unknown>;
  };
  const toArray = (v: unknown): Array<unknown> => {
    if (typeof v === "string") {
      try {
        return JSON.parse(v);
      } catch {
        return [];
      }
    }
    return (v ?? []) as Array<unknown>;
  };
  return {
    ...row,
    confidence: (row.confidence ?? row.confidence_score) ?? 0,
    confidence_score: (row.confidence ?? row.confidence_score) ?? 0,
    raw_snapshot: (row.raw_snapshot ?? toJson(row.raw_snapshot_json)) as Record<string, unknown>,
    ai_analysis: (row.ai_analysis ?? toJson(row.ai_analysis_json)) as Record<string, unknown>,
    matched_capabilities: (row.matched_capabilities
      ?? toArray(row.matched_capabilities_json)) as Array<{
        id: string;
        name: string;
        source: string;
        score: number;
      }>,
    recommended_workflow: (row.recommended_workflow ?? row.recommended_workflow_id) as
      | string
      | null,
  };
}

export function DemandDiscoveryTab() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState("leads");

  return (
    <div className="space-y-4">
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: "leads",
            label: (
              <span>
                <FireOutlined /> {t("opc.demand.leads")}
              </span>
            ),
            children: <LeadsPanel />,
          },
          {
            key: "capabilities",
            label: (
              <span>
                <AppstoreOutlined /> {t("opc.demand.capabilities")}
              </span>
            ),
            children: <CapabilitiesPanel />,
          },
          {
            key: "platforms",
            label: (
              <span>
                <GlobalOutlined /> {t("opc.demand.platforms")}
              </span>
            ),
            children: <PlatformsPanel />,
          },
          {
            key: "gaps",
            label: (
              <span>
                <WarningOutlined /> {t("opc.demand.gaps")}
              </span>
            ),
            children: <CapabilityGapsPanel />,
          },
          {
            key: "deliveries",
            label: (
              <span>
                <ThunderboltOutlined /> {t("opc.demand.deliveries")}
              </span>
            ),
            children: <DeliveriesPanel />,
          },
          {
            key: "config",
            label: (
              <span>
                <SettingOutlined /> {t("opc.demand.config")}
              </span>
            ),
            children: <DemandDiscoveryConfigPanel />,
          },
        ]}
      />
    </div>
  );
}

// ── 需求线索面板 ──────────────────────────────────────────────

function LeadsPanel() {
  const { t } = useTranslation();
  const [leads, setLeads] = useState<DemandLead[]>([]);
  const [loading, setLoading] = useState(false);
  const [discoverModalOpen, setDiscoverModalOpen] = useState(false);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [discoverQuery, setDiscoverQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<string | undefined>();
  const [platformFilter] = useState<string | undefined>();
  const [form] = Form.useForm();

  const loadLeads = useCallback(() => {
    setLoading(true);
    invoke<DemandLead[]>("opc_list_leads", { status: statusFilter, platform: platformFilter })
      .then((rows) => setLeads(rows.map(mapLead)))
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [statusFilter, platformFilter]);

  useEffect(() => {
    loadLeads();
  }, [loadLeads]);

  const handleDiscover = async () => {
    if (!discoverQuery.trim()) {
      message.warning(t("opc.demand.enterSearchQuery"));
      return;
    }
    setDiscovering(true);
    try {
      const found = await invoke<DemandLead[]>("opc_discover_leads", { query: discoverQuery });
      if (found && found.length > 0) {
        message.success(t("opc.demand.discoveredCount", { count: found.length }));
        setDiscoverModalOpen(false);
        setDiscoverQuery("");
        loadLeads();
      } else {
        message.info(t("opc.demand.noLeadsFound"));
      }
    } catch (e) {
      message.error(String(e));
    } finally {
      setDiscovering(false);
    }
  };

  const handleMatchCapabilities = async (id: string) => {
    try {
      await invoke("opc_match_lead_capabilities", { id });
      message.success(t("opc.demand.matchedCapabilities"));
      loadLeads();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleConfirmLead = async (id: string) => {
    try {
      await invoke("opc_confirm_lead", { id });
      message.success(t("opc.demand.leadConfirmed"));
      loadLeads();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleMarkStatus = async (id: string, status: string) => {
    try {
      await invoke("opc_mark_lead_status", { id, status });
      message.success(t("opc.demand.statusUpdated"));
      loadLeads();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleExecuteDelivery = async (leadId: string) => {
    try {
      const result = await invoke<Delivery>("opc_execute_demand_workflow", { lead_id: leadId });
      message.success(t("opc.demand.deliveryStarted"));
      loadLeads();
      return result;
    } catch (e) {
      message.error(String(e));
      return null;
    }
  };

  const handleCreateLead = async (values: Record<string, unknown>) => {
    try {
      await invoke("opc_create_lead", { input: values });
      message.success(t("opc.demand.leadCreated"));
      setCreateModalOpen(false);
      form.resetFields();
      loadLeads();
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns: ColumnsType<DemandLead> = [
    {
      title: t("opc.demand.colTitle"),
      dataIndex: "title",
      key: "title",
      width: 200,
      ellipsis: true,
    },
    {
      title: t("opc.demand.colDescription"),
      dataIndex: "description",
      key: "description",
      width: 200,
      ellipsis: true,
    },
    {
      title: t("opc.demand.colPlatform"),
      dataIndex: "platform",
      key: "platform",
      width: 100,
    },
    {
      title: t("opc.demand.colStatus"),
      dataIndex: "status",
      key: "status",
      width: 100,
      render: (v: string) => (
        <Tag color={LEAD_STATUS_COLOR_MAP[v] || "default"}>
          {t(`opc.demand.status.${v}`)}
        </Tag>
      ),
    },
    {
      title: t("opc.demand.colBudget"),
      key: "budget",
      width: 120,
      render: (_: unknown, r: DemandLead) => {
        if (r.budget_min && r.budget_max) {
          return `${r.budget_min}-${r.budget_max}`;
        } else if (r.budget_min) {
          return `≥${r.budget_min}`;
        }
        return "-";
      },
    },
    {
      title: t("opc.demand.colConfidence"),
      dataIndex: "confidence_score",
      key: "confidence",
      width: 100,
      render: (v: number) => <Progress percent={Math.round(v * 100)} size="small" />,
    },
    {
      title: t("opc.demand.colMatched"),
      key: "matched",
      width: 80,
      render: (_: unknown, r: DemandLead) => r.matched_capabilities?.length || 0,
    },
    {
      title: t("opc.demand.colActions"),
      key: "actions",
      width: 280,
      fixed: "right",
      render: (_: unknown, r: DemandLead) => (
        <Space size="small">
          {r.status === "new" && (
            <>
              <Button
                size="small"
                icon={<SearchOutlined />}
                onClick={() => handleMatchCapabilities(r.id)}
              >
                {t("opc.demand.actionMatch")}
              </Button>
              <Button
                size="small"
                type="primary"
                onClick={() => handleConfirmLead(r.id)}
              >
                {t("opc.demand.actionConfirm")}
              </Button>
            </>
          )}
          {r.status === "qualified" && (
            <Button
              size="small"
              type="primary"
              danger
              icon={<ThunderboltOutlined />}
              onClick={() => handleExecuteDelivery(r.id)}
            >
              {t("opc.demand.actionExecute")}
            </Button>
          )}
          {r.status === "executing" && <Tag color="orange">{t("opc.demand.status.executing")}</Tag>}
          {(r.status === "delivered" || r.status === "failed") && (
            <Tag color={r.status === "delivered" ? "green" : "red"}>
              {t(`opc.demand.status.${r.status}`)}
            </Tag>
          )}
          {!["delivered", "failed", "cancelled", "expired", "claimed"].includes(r.status) && (
            <Dropdown
              menu={{
                items: [
                  { key: "expired", label: t("opc.demand.markExpired") },
                  { key: "claimed", label: t("opc.demand.markClaimed") },
                  { key: "cancelled", label: t("opc.demand.markCancelled") },
                ],
                onClick: ({ key }) => handleMarkStatus(r.id, key),
              }}
            >
              <Button size="small" icon={<EditOutlined />}>
                {t("opc.demand.actionMark")}
              </Button>
            </Dropdown>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      <Card size="small">
        <Row gutter={[16, 16]} align="middle">
          <Col span={6}>
            <Button
              icon={<SearchOutlined />}
              onClick={() => setDiscoverModalOpen(true)}
            >
              {t("opc.demand.btnDiscover")}
            </Button>
          </Col>
          <Col span={6}>
            <Button
              icon={<PlusOutlined />}
              onClick={() => setCreateModalOpen(true)}
            >
              {t("opc.demand.btnCreate")}
            </Button>
          </Col>
          <Col span={6}>
            <Input.Search
              placeholder={t("opc.demand.searchPlaceholder")}
              allowClear
              onSearch={(v) => {
                setStatusFilter(v ? undefined : statusFilter);
                loadLeads();
              }}
              style={{ width: 200 }}
            />
          </Col>
          <Col span={6}>
            <Space>
              <Button onClick={loadLeads}>{t("opc.demand.btnRefresh")}</Button>
            </Space>
          </Col>
        </Row>
      </Card>

      <Table
        rowKey="id"
        loading={loading}
        dataSource={leads}
        columns={columns}
        pagination={{ pageSize: 10 }}
        scroll={{ x: 1200 }}
      />

      {/* 发现线索弹窗 */}
      <Modal
        title={t("opc.demand.discoverTitle")}
        open={discoverModalOpen}
        onCancel={() => setDiscoverModalOpen(false)}
        onOk={handleDiscover}
        confirmLoading={discovering}
      >
        <Form layout="vertical">
          <Form.Item label={t("opc.demand.searchQuery")}>
            <TextArea
              rows={3}
              value={discoverQuery}
              onChange={(e) => setDiscoverQuery(e.target.value)}
              placeholder={t("opc.demand.searchQueryPlaceholder")}
            />
          </Form.Item>
        </Form>
      </Modal>

      {/* 手动创建弹窗 */}
      <Modal
        title={t("opc.demand.createTitle")}
        open={createModalOpen}
        onCancel={() => setCreateModalOpen(false)}
        onOk={() => form.submit()}
      >
        <Form form={form} layout="vertical" onFinish={handleCreateLead}>
          <Form.Item
            name="title"
            label={t("opc.demand.formTitle")}
            rules={[{ required: true, message: t("opc.demand.formTitleRequired") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="description" label={t("opc.demand.formDescription")}>
            <TextArea rows={3} />
          </Form.Item>
          <Form.Item name="platform" label={t("opc.demand.formPlatform")}>
            <Input placeholder={t("opc.demand.formPlatformPlaceholder")} />
          </Form.Item>
          <Row gutter={12}>
            <Col span={12}>
              <Form.Item name="budget_min" label={t("opc.demand.formBudgetMin")}>
                <Input type="number" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="budget_max" label={t("opc.demand.formBudgetMax")}>
                <Input type="number" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="contact_name" label={t("opc.demand.formContactName")}>
            <Input />
          </Form.Item>
          <Form.Item name="contact_email" label={t("opc.demand.formContactEmail")}>
            <Input />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

// ── 能力库面板 ────────────────────────────────────────────────

function CapabilitiesPanel() {
  const { t } = useTranslation();
  const [inventory, setInventory] = useState<CapabilityInventory | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(() => {
    setLoading(true);
    invoke<CapabilityInventory>("opc_scan_capabilities")
      .then(setInventory)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const renderTable = (title: string, data: CapabilityEntry[]) => {
    const cols: ColumnsType<CapabilityEntry> = [
      { title: t("opc.demand.colName"), dataIndex: "name", key: "name", width: 150 },
      { title: t("opc.demand.colDescription"), dataIndex: "description", key: "description", ellipsis: true },
      {
        title: t("opc.demand.colSource"),
        dataIndex: "source",
        key: "source",
        width: 100,
        render: (v: string) => <Tag>{v}</Tag>,
      },
      { title: t("opc.demand.colType"), dataIndex: "capability_type", key: "type", width: 100 },
    ];
    return (
      <Card
        title={`${title} (${data.length})`}
        size="small"
        style={{ marginBottom: 16 }}
      >
        <Table
          rowKey="id"
          size="small"
          dataSource={data}
          columns={cols}
          pagination={{ pageSize: 5 }}
          scroll={{ x: 600 }}
        />
      </Card>
    );
  };

  return (
    <div className="space-y-4">
      <Card size="small">
        <Space>
          <Button icon={<BugOutlined />} onClick={load} loading={loading}>
            {t("opc.demand.btnScanCapabilities")}
          </Button>
          {inventory && (
            <Tag color="blue">
              {t("opc.demand.totalCapabilities", { count: inventory.total_count })}
            </Tag>
          )}
        </Space>
      </Card>

      {inventory && (
        <>
          {renderTable(t("opc.demand.categoryTools"), inventory.tools)}
          {renderTable(t("opc.demand.categorySkills"), inventory.skills)}
          {renderTable(t("opc.demand.categoryMcpTools"), inventory.mcp_tools)}
          {renderTable(t("opc.demand.categoryWorkflows"), inventory.workflows)}
        </>
      )}
    </div>
  );
}

// ── 交付记录面板 ──────────────────────────────────────────────

function DeliveriesPanel() {
  const { t } = useTranslation();
  const [deliveries, setDeliveries] = useState<Delivery[]>([]);
  const [loading, setLoading] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [currentDelivery, setCurrentDelivery] = useState<Delivery | null>(null);

  const load = useCallback(() => {
    setLoading(true);
    invoke<Delivery[]>("opc_list_deliveries", {})
      .then(setDeliveries)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleViewDetail = (d: Delivery) => {
    setCurrentDelivery(d);
    setDetailOpen(true);
  };

  const handleRetry = async (id: string) => {
    try {
      await invoke("opc_retry_delivery", { id });
      message.success(t("opc.demand.deliveryRetried"));
      load();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleCancel = async (id: string) => {
    try {
      await invoke("opc_cancel_delivery", { id });
      message.success(t("opc.demand.deliveryCancelled"));
      load();
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns: ColumnsType<Delivery> = [
    { title: t("opc.demand.colTitle"), dataIndex: "title", key: "title", width: 200, ellipsis: true },
    {
      title: t("opc.demand.colStatus"),
      dataIndex: "status",
      key: "status",
      width: 120,
      render: (v: string) => (
        <Tag color={DELIVERY_STATUS_COLOR_MAP[v] || "default"}>
          {t(`opc.delivery.status.${v}`)}
        </Tag>
      ),
    },
    {
      title: t("opc.demand.colProgress"),
      dataIndex: "progress",
      key: "progress",
      width: 150,
      render: (v: number) => <Progress percent={Math.round(v * 100)} size="small" />,
    },
    {
      title: t("opc.demand.colTemplate"),
      dataIndex: "workflow_template_id",
      key: "template",
      width: 180,
      ellipsis: true,
    },
    {
      title: t("opc.demand.colStartedAt"),
      dataIndex: "started_at",
      key: "started_at",
      width: 120,
      render: (v: number | null) => v ? new Date(v * 1000).toLocaleString() : "-",
    },
    {
      title: t("opc.demand.colCompletedAt"),
      dataIndex: "completed_at",
      key: "completed_at",
      width: 120,
      render: (v: number | null) => v ? new Date(v * 1000).toLocaleString() : "-",
    },
    {
      title: t("opc.demand.colResult"),
      dataIndex: "result_summary",
      key: "result",
      ellipsis: true,
    },
    {
      title: t("opc.demand.colActions"),
      key: "actions",
      width: 180,
      fixed: "right",
      render: (_: unknown, r: Delivery) => (
        <Space size="small">
          <Button size="small" onClick={() => handleViewDetail(r)}>
            {t("opc.demand.actionViewDetail")}
          </Button>
          {r.status === "failed" && (
            <Button size="small" type="primary" onClick={() => handleRetry(r.id)}>
              {t("opc.demand.actionRetry")}
            </Button>
          )}
          {["pending", "running"].includes(r.status) && (
            <Button size="small" danger onClick={() => handleCancel(r.id)}>
              {t("opc.demand.actionCancel")}
            </Button>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      <Card size="small">
        <Button onClick={load} loading={loading}>
          {t("opc.demand.btnRefresh")}
        </Button>
      </Card>

      <Table
        rowKey="id"
        loading={loading}
        dataSource={deliveries}
        columns={columns}
        pagination={{ pageSize: 10 }}
        scroll={{ x: 1200 }}
      />

      <DeliveryDetailModal
        open={detailOpen}
        delivery={currentDelivery}
        onClose={() => setDetailOpen(false)}
        onRetry={handleRetry}
        onCancel={handleCancel}
      />
    </div>
  );
}

// ── 交付详情模态框 ──────────────────────────────────────────

interface DeliveryDetailModalProps {
  open: boolean;
  delivery: Delivery | null;
  onClose: () => void;
  onRetry: (id: string) => Promise<void>;
  onCancel: (id: string) => Promise<void>;
}

function DeliveryDetailModal({
  open,
  delivery,
  onClose,
  onRetry,
  onCancel,
}: DeliveryDetailModalProps) {
  const { t } = useTranslation();

  if (!delivery) { return null; }

  const timelineSteps = [
    {
      title: t("opc.demand.deliverySteps.initiated"),
      time: delivery.started_at ? new Date(delivery.started_at * 1000).toLocaleString() : "-",
      status: "completed",
    },
    {
      title: t("opc.demand.deliverySteps.processing"),
      time: delivery.progress > 0 ? t("opc.demand.deliverySteps.inProgress") : "-",
      status: delivery.progress > 0 ? "active" : "pending",
    },
    {
      title: t("opc.demand.deliverySteps.completed"),
      time: delivery.completed_at ? new Date(delivery.completed_at * 1000).toLocaleString() : "-",
      status: delivery.status === "completed" || delivery.status === "delivered"
        ? "completed"
        : delivery.status === "failed"
        ? "error"
        : "pending",
    },
  ];

  return (
    <Modal
      title={`${t("opc.demand.deliveryDetail")}: ${delivery.title}`}
      open={open}
      onCancel={onClose}
      footer={[
        (delivery.status === "failed" || delivery.status === "cancelled") && (
          <Button
            key="retry"
            type="primary"
            onClick={() => onRetry(delivery.id)}
          >
            {t("opc.demand.actionRetry")}
          </Button>
        ),
        ["pending", "running"].includes(delivery.status) && (
          <Button
            key="cancel"
            danger
            onClick={() => onCancel(delivery.id)}
          >
            {t("opc.demand.actionCancel")}
          </Button>
        ),
        <Button key="close" onClick={onClose}>
          {t("opc.demand.actionClose")}
        </Button>,
      ].filter(Boolean)}
      width={720}
    >
      <div className="space-y-4">
        {/* 交付状态卡片 */}
        <Card size="small" title={t("opc.demand.deliveryInfo")}>
          <Descriptions column={2} size="small">
            <Descriptions.Item label={t("opc.demand.colStatus")}>
              <Tag color={DELIVERY_STATUS_COLOR_MAP[delivery.status] || "default"}>
                {t(`opc.delivery.status.${delivery.status}`)}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label={t("opc.demand.colProgress")}>
              <Progress percent={Math.round(delivery.progress * 100)} />
            </Descriptions.Item>
            <Descriptions.Item label={t("opc.demand.colTemplate")}>
              {delivery.workflow_template_id || "-"}
            </Descriptions.Item>
            <Descriptions.Item label={t("opc.demand.leadId")}>
              {delivery.lead_id}
            </Descriptions.Item>
            <Descriptions.Item label={t("opc.demand.colStartedAt")}>
              {delivery.started_at ? new Date(delivery.started_at * 1000).toLocaleString() : "-"}
            </Descriptions.Item>
            <Descriptions.Item label={t("opc.demand.colCompletedAt")}>
              {delivery.completed_at ? new Date(delivery.completed_at * 1000).toLocaleString() : "-"}
            </Descriptions.Item>
          </Descriptions>
        </Card>

        {/* 交付时间线 */}
        <Card size="small" title={t("opc.demand.deliveryTimeline")}>
          <Timeline
            items={timelineSteps.map((s) => ({
              color: s.status === "completed"
                ? "green"
                : s.status === "active"
                ? "blue"
                : s.status === "error"
                ? "red"
                : "gray",
              children: (
                <div>
                  <div className="font-medium">{s.title}</div>
                  <div className="text-xs text-gray-500">{s.time}</div>
                </div>
              ),
            }))}
          />
        </Card>

        {/* 交付结果 */}
        {delivery.result_summary && (
          <Card size="small" title={t("opc.demand.deliveryResult")}>
            <Typography.Paragraph
              ellipsis={{ rows: 3, expandable: true, symbol: t("opc.demand.expand") }}
            >
              {delivery.result_summary}
            </Typography.Paragraph>
          </Card>
        )}

        {/* 可交付物列表 */}
        {delivery.deliverables && delivery.deliverables.length > 0 && (
          <Card size="small" title={t("opc.demand.deliverables")}>
            <List
              size="small"
              dataSource={delivery.deliverables}
              renderItem={(d, idx) => (
                <List.Item key={idx}>
                  <Space>
                    <Tag color="blue">
                      {(d.name as string) || t("opc.demand.deliverable")}
                    </Tag>
                    <span>{(d.type as string) || ""}</span>
                  </Space>
                </List.Item>
              )}
            />
          </Card>
        )}

        {/* 错误列表 */}
        {delivery.errors && delivery.errors.length > 0 && (
          <Card size="small" title={t("opc.demand.errors")}>
            <Alert
              type="error"
              showIcon
              message={t("opc.demand.deliveryError")}
              description={
                <ul>
                  {delivery.errors.map((e, idx) => <li key={idx}>{(e.message as string) || JSON.stringify(e)}</li>)}
                </ul>
              }
            />
          </Card>
        )}

        {/* 元数据 */}
        {delivery.metadata && Object.keys(delivery.metadata).length > 0 && (
          <Card size="small" title={t("opc.demand.metadata")}>
            <pre className="text-xs bg-gray-50 p-2 rounded">
              {JSON.stringify(delivery.metadata, null, 2)}
            </pre>
          </Card>
        )}
      </div>
    </Modal>
  );
}

// ── 平台配置面板 ──────────────────────────────────────────────

function PlatformsPanel() {
  const { t } = useTranslation();
  const [platforms, setPlatforms] = useState<MarketPlatform[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<MarketPlatform | null>(null);
  const [form] = Form.useForm();

  const load = useCallback(() => {
    setLoading(true);
    invoke<MarketPlatform[]>("opc_list_platforms")
      .then(setPlatforms)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const openCreate = () => {
    setEditing(null);
    form.resetFields();
    setModalOpen(true);
  };

  const openEdit = (p: MarketPlatform) => {
    setEditing(p);
    form.setFieldsValue({
      id: p.id,
      name: p.name,
      platform_type: p.platform_type,
      enabled: p.enabled === 1,
      base_url: p.base_url || "",
      config: p.config ? JSON.stringify(p.config) : "{}",
    });
    setModalOpen(true);
  };

  const handleSave = async () => {
    const values = await form.validateFields();
    let config: Record<string, unknown> = {};
    try {
      config = values.config ? JSON.parse(values.config) : {};
    } catch {
      message.warning(t("opc.demand.invalidConfigJson"));
      return;
    }
    try {
      await invoke("opc_save_platform", {
        input: {
          id: editing?.id || "",
          name: values.name,
          platform_type: values.platform_type || "manual",
          enabled: values.enabled ? 1 : 0,
          base_url: values.base_url || null,
          config,
        },
      });
      message.success(t("opc.demand.platformSaved"));
      setModalOpen(false);
      load();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("opc_delete_platform", { id });
      message.success(t("opc.demand.platformDeleted"));
      load();
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns: ColumnsType<MarketPlatform> = [
    { title: t("opc.demand.colName"), dataIndex: "name", key: "name", width: 150 },
    {
      title: t("opc.demand.colPlatformType"),
      dataIndex: "platform_type",
      key: "platform_type",
      width: 120,
      render: (v: string) => <Tag>{v}</Tag>,
    },
    { title: t("opc.demand.colBaseUrl"), dataIndex: "base_url", key: "base_url", ellipsis: true },
    {
      title: t("opc.demand.colEnabled"),
      dataIndex: "enabled",
      key: "enabled",
      width: 90,
      render: (v: number) => (
        <Tag color={v === 1 ? "green" : "default"}>
          {v === 1 ? t("opc.demand.enabled") : t("opc.demand.disabled")}
        </Tag>
      ),
    },
    {
      title: t("opc.demand.colLastSync"),
      dataIndex: "last_sync_at",
      key: "last_sync_at",
      width: 140,
      render: (v: number | null) => (v ? new Date(v * 1000).toLocaleString() : "-"),
    },
    {
      title: t("opc.demand.colStatus"),
      dataIndex: "status",
      key: "status",
      width: 100,
      render: (v: string) => <Tag>{v}</Tag>,
    },
    {
      title: t("opc.demand.colActions"),
      key: "actions",
      width: 140,
      render: (_: unknown, r: MarketPlatform) => (
        <Space size="small">
          <Button size="small" icon={<EditOutlined />} onClick={() => openEdit(r)}>
            {t("opc.demand.actionEdit")}
          </Button>
          <Popconfirm title={t("opc.demand.confirmDelete")} onConfirm={() => handleDelete(r.id)}>
            <Button size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      <Card size="small">
        <Space>
          <Button icon={<PlusOutlined />} onClick={openCreate}>
            {t("opc.demand.actionAddPlatform")}
          </Button>
          <Button onClick={load} loading={loading}>
            {t("opc.demand.btnRefresh")}
          </Button>
        </Space>
      </Card>

      <Table
        rowKey="id"
        loading={loading}
        dataSource={platforms}
        columns={columns}
        pagination={{ pageSize: 10 }}
        scroll={{ x: 900 }}
      />

      <Modal
        title={editing ? t("opc.demand.editPlatform") : t("opc.demand.addPlatform")}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label={t("opc.demand.formName")} rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="platform_type" label={t("opc.demand.formPlatformType")}>
            <Input placeholder={t("opc.demand.formPlatformTypePlaceholder")} />
          </Form.Item>
          <Form.Item name="base_url" label={t("opc.demand.formBaseUrl")}>
            <Input placeholder="https://..." />
          </Form.Item>
          <Form.Item name="enabled" label={t("opc.demand.formEnabled")} valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="config" label={t("opc.demand.formConfig")}>
            <TextArea rows={4} placeholder="{}" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

// ── 能力缺口面板 ──────────────────────────────────────────────

function CapabilityGapsPanel() {
  const { t } = useTranslation();
  const [gaps, setGaps] = useState<CapabilityGap[]>([]);
  const [loading, setLoading] = useState(false);

  const load = useCallback(() => {
    setLoading(true);
    invoke<CapabilityGap[]>("opc_list_capability_gaps", {})
      .then(setGaps)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleClose = async (id: string) => {
    try {
      await invoke("opc_close_capability_gap", { id });
      message.success(t("opc.demand.gapClosed"));
      load();
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns: ColumnsType<CapabilityGap> = [
    { title: t("opc.demand.colTitle"), dataIndex: "title", key: "title", width: 200, ellipsis: true },
    {
      title: t("opc.demand.colDescription"),
      dataIndex: "description",
      key: "description",
      ellipsis: true,
    },
    {
      title: t("opc.demand.colGapType"),
      dataIndex: "gap_type",
      key: "gap_type",
      width: 110,
      render: (v: string) => <Tag color="orange">{v}</Tag>,
    },
    {
      title: t("opc.demand.colSuggestedAction"),
      dataIndex: "suggested_action",
      key: "suggested_action",
      ellipsis: true,
    },
    {
      title: t("opc.demand.colPriority"),
      dataIndex: "priority",
      key: "priority",
      width: 90,
      render: (v: number) => <Tag color={v <= 2 ? "red" : v === 3 ? "orange" : "default"}>{v}</Tag>,
    },
    {
      title: t("opc.demand.colStatus"),
      dataIndex: "status",
      key: "status",
      width: 100,
      render: (v: string) => <Tag color={v === "open" ? "orange" : "green"}>{t(`opc.demand.gapStatus.${v}`)}</Tag>,
    },
    {
      title: t("opc.demand.colActions"),
      key: "actions",
      width: 120,
      render: (_: unknown, r: CapabilityGap) =>
        r.status === "open"
          ? (
            <Button size="small" onClick={() => handleClose(r.id)}>
              {t("opc.demand.actionCloseGap")}
            </Button>
          )
          : null,
    },
  ];

  return (
    <div className="space-y-4">
      <Card size="small">
        <Button onClick={load} loading={loading}>
          {t("opc.demand.btnRefresh")}
        </Button>
      </Card>

      <Table
        rowKey="id"
        loading={loading}
        dataSource={gaps}
        columns={columns}
        pagination={{ pageSize: 10 }}
        scroll={{ x: 1100 }}
      />
    </div>
  );
}
