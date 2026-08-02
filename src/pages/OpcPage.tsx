import { OfficeTab as FleetOfficeTab } from "@/components/office/OfficeTab";
import { invoke } from "@/lib/invoke";
import {
  DeleteOutlined,
  DollarOutlined,
  EditOutlined,
  FileTextOutlined,
  FlagOutlined,
  PlusOutlined,
  ProjectOutlined,
  RiseOutlined,
  SearchOutlined,
  SwapOutlined,
  TeamOutlined,
} from "@ant-design/icons";
import {
  Alert,
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
  Spin,
  Statistic,
  Table,
  Tabs,
  Tag,
  Timeline,
  Typography,
} from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Text } = Typography;

// ── 类型 ─────────────────────────────────────────────────────────

interface InvoiceLineItem {
  description: string;
  quantity: number;
  unit_price: number;
  tax_rate: number;
  total: number;
}

interface Invoice {
  id: string;
  customer_id: string;
  invoice_number: string;
  status: string;
  line_items?: InvoiceLineItem[];
  subtotal: number;
  tax_total: number;
  total: number;
  currency: string;
  notes: string;
  due_at: number | null;
  paid_at: number | null;
  issued_at: number | null;
  created_at: number;
  updated_at: number;
}

interface Customer {
  id: string;
  name: string;
  email: string;
  phone: string | null;
  company: string | null;
  status: string;
  source: string | null;
  tags: string[];
  notes: string;
  total_revenue: number;
  invoice_count: number;
  created_at: number;
  updated_at: number;
}

interface Milestone {
  id: string;
  title: string;
  description: string;
  due_at: number | null;
  completed_at: number | null;
  status: string;
}

interface Project {
  id: string;
  customer_id: string | null;
  title: string;
  description: string;
  status: string;
  milestones: Milestone[];
  budget: number | null;
  currency: string;
  started_at: number | null;
  deadline: number | null;
  completed_at: number | null;
  notes: string;
  created_at: number;
  updated_at: number;
}

// ── 状态标签翻译函数 ─────────────────────────────────────────────

function getInvoiceStatusKey(status: string): string {
  return `opc.invoiceStatus.${status}`;
}

function getCustomerStatusKey(status: string): string {
  return `opc.customerStatus.${status}`;
}

function getProjectStatusKey(status: string): string {
  return `opc.projectStatus.${status}`;
}

function getSourceKey(source: string): string {
  return `opc.source.${source}`;
}

const STATUS_COLOR_MAP: Record<string, string> = {
  draft: "default",
  sent: "blue",
  paid: "green",
  overdue: "red",
  cancelled: "default",
  refunded: "orange",
};

const CUST_STATUS_COLOR_MAP: Record<string, string> = {
  lead: "default",
  prospect: "blue",
  active: "green",
  inactive: "default",
  churned: "red",
};

const PROJ_STATUS_COLOR_MAP: Record<string, string> = {
  planning: "blue",
  active: "green",
  paused: "orange",
  completed: "default",
  cancelled: "red",
};

// ── 主页面 ───────────────────────────────────────────────────────

export function OpcPage() {
  const { t } = useTranslation();
  const [tab, setTab] = useState("dashboard");

  return (
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      <Title level={3} style={{ marginBottom: 16 }}>
        <FileTextOutlined style={{ marginRight: 8 }} />
        {t("opc.title")}
      </Title>
      <Tabs
        activeKey={tab}
        onChange={setTab}
        items={[
          {
            key: "dashboard",
            label: (
              <span>
                <RiseOutlined /> {t("opc.nav.dashboard")}
              </span>
            ),
            children: <DashboardTab />,
          },
          {
            key: "invoices",
            label: (
              <span>
                <DollarOutlined /> {t("opc.nav.invoices")}
              </span>
            ),
            children: <InvoicesTab />,
          },
          {
            key: "customers",
            label: (
              <span>
                <TeamOutlined /> {t("opc.nav.customers")}
              </span>
            ),
            children: <CustomersTab />,
          },
          {
            key: "projects",
            label: (
              <span>
                <ProjectOutlined /> {t("opc.nav.projects")}
              </span>
            ),
            children: <ProjectsTab />,
          },
          {
            key: "sites",
            label: (
              <span>
                <FileTextOutlined /> {t("opc.nav.sites")}
              </span>
            ),
            children: <SitesTab />,
          },
          {
            key: "office",
            label: (
              <span>
                <TeamOutlined /> {t("opc.nav.office")}
              </span>
            ),
            children: <OpcOfficeTab />,
          },
          {
            key: "talent",
            label: (
              <span>
                <SearchOutlined /> {t("opc.nav.talent")}
              </span>
            ),
            children: <TalentMarketTab />,
          },
          {
            key: "market",
            label: (
              <span>
                <RiseOutlined /> {t("opc.nav.market")}
              </span>
            ),
            children: <MarketPackTab />,
          },
          {
            key: "kanban",
            label: (
              <span>
                <ProjectOutlined /> {t("opc.nav.kanban")}
              </span>
            ),
            children: <KanbanTab />,
          },
        ]}
      />
    </div>
  );
}

// ══════════════════════════════════════════════════════════════════
// 仪表盘
// ══════════════════════════════════════════════════════════════════

function DashboardTab() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [summary, setSummary] = useState<
    {
      total_revenue: number;
      total_invoices: number;
      active_projects: number;
      total_customers: number;
      recent_kpis: Array<{ name: string; value: number; unit: string; period: string }>;
    } | null
  >(null);

  useEffect(() => {
    invoke<{
      total_revenue: number;
      total_invoices: number;
      active_projects: number;
      total_customers: number;
      recent_kpis: Array<{ name: string; value: number; unit: string; period: string }>;
    }>("opc_get_dashboard_summary")
      .then(setSummary).catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  if (loading) { return <Spin size="large" style={{ display: "block", margin: "80px auto" }} />; }
  if (!summary) { return <Empty description={t("opc.dashboard.loadFailed")} />; }

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <Card size="small">
            <Statistic title={t("opc.dashboard.totalRevenue")} value={summary.total_revenue} prefix="¥" precision={2} />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("opc.dashboard.totalInvoices")}
              value={summary.total_invoices}
              prefix={<FileTextOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("opc.dashboard.activeCustomers")}
              value={summary.total_customers}
              prefix={<TeamOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("opc.dashboard.activeProjects")}
              value={summary.active_projects}
              prefix={<ProjectOutlined />}
            />
          </Card>
        </Col>
      </Row>
      <Row gutter={16}>
        <Col span={12}>
          <Card title={t("opc.dashboard.kpiTitle")} size="small">
            {summary.recent_kpis.length === 0
              ? <Empty description={t("opc.dashboard.noKpi")} />
              : (
                <Timeline
                  items={summary.recent_kpis.slice(0, 5).map((kpi) => ({
                    color: "blue",
                    children: (
                      <>
                        <strong>{kpi.name}</strong>: {kpi.value} {kpi.unit} <Tag>{kpi.period}</Tag>
                      </>
                    ),
                  }))}
                />
              )}
          </Card>
        </Col>
        <Col span={12}>
          <Card title={t("opc.dashboard.quickActionsTitle")} size="small">
            <Space direction="vertical" style={{ width: "100%" }}>
              <Button
                type="primary"
                block
                icon={<DollarOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "invoices" }))}
              >
                {t("opc.dashboard.manageInvoices")}
              </Button>
              <Button
                block
                icon={<TeamOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "customers" }))}
              >
                {t("opc.dashboard.manageCustomers")}
              </Button>
              <Button
                block
                icon={<ProjectOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "projects" }))}
              >
                {t("opc.dashboard.manageProjects")}
              </Button>
            </Space>
          </Card>
        </Col>
      </Row>
    </div>
  );
}

// ══════════════════════════════════════════════════════════════════
// 发票管理
// ══════════════════════════════════════════════════════════════════

function InvoicesTab() {
  const { t } = useTranslation();
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [transitionOpen, setTransitionOpen] = useState(false);
  const [transitionInvoice, setTransitionInvoice] = useState<Invoice | null>(null);
  const [form] = Form.useForm();

  const load = useCallback(() => {
    setLoading(true);
    Promise.all([
      invoke<Invoice[]>("opc_list_invoices", { filter: {} }),
      invoke<Customer[]>("opc_list_customers", { filter: {} }),
    ]).then(([inv, cust]) => {
      setInvoices(inv);
      setCustomers(cust);
    }).catch(console.error).finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleCreate = async (values: Record<string, unknown>) => {
    try {
      const lineItems = (values.line_items as Array<Record<string, unknown>>).map((item: Record<string, unknown>) => ({
        description: item.description as string,
        quantity: Number(item.quantity),
        unit_price: Number(item.unit_price),
        tax_rate: Number(item.tax_rate || 0),
        total: Number(item.quantity) * Number(item.unit_price) * (1 + Number(item.tax_rate || 0)),
      }));
      await invoke("opc_create_invoice", {
        input: {
          customer_id: values.customer_id,
          line_items: lineItems,
          currency: "CNY",
          due_at: values.due_at ? Math.floor(new Date(values.due_at as string).getTime() / 1000) : null,
          notes: values.notes || "",
        },
      });
      message.success(t("opc.invoice.created"));
      setModalOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(t("opc.common.createFailed", { error: String(e) }));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("opc_delete_invoice", { id });
      message.success(t("opc.invoice.deleted"));
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const handleTransition = async (id: string, status: string) => {
    try {
      await invoke("opc_transition_invoice", { id, targetStatus: status });
      message.success(t("opc.invoice.statusUpdated"));
      setTransitionOpen(false);
      setTransitionInvoice(null);
      load();
    } catch (e) {
      message.error(t("opc.invoice.statusUpdateFailed", { error: String(e) }));
    }
  };

  const nextStatuses = (status: string): Array<{ value: string; label: string }> => {
    const map: Record<string, Array<{ value: string; label: string }>> = {
      draft: [{ value: "sent", label: t("opc.invoice.actionSend") }, {
        value: "cancelled",
        label: t("opc.invoice.actionCancel"),
      }],
      sent: [{ value: "paid", label: t("opc.invoice.actionMarkPaid") }, {
        value: "overdue",
        label: t("opc.invoice.actionMarkOverdue"),
      }, { value: "cancelled", label: t("opc.invoice.actionCancel") }],
      overdue: [{ value: "paid", label: t("opc.invoice.actionMarkPaid") }, {
        value: "cancelled",
        label: t("opc.invoice.actionCancel"),
      }],
      paid: [{ value: "refunded", label: t("opc.invoice.actionRefund") }],
    };
    return map[status] || [];
  };

  const columns = [
    { title: t("opc.invoice.columnNumber"), dataIndex: "invoice_number", key: "number", width: 180 },
    {
      title: t("opc.invoice.columnAmount"),
      key: "total",
      render: (_: unknown, r: Invoice) => `¥${r.total.toFixed(2)}`,
      sorter: (a: Invoice, b: Invoice) => a.total - b.total,
    },
    {
      title: t("opc.invoice.columnStatus"),
      key: "status",
      render: (_: unknown, r: Invoice) => {
        const color = STATUS_COLOR_MAP[r.status] || "default";
        return <Tag color={color}>{t(getInvoiceStatusKey(r.status))}</Tag>;
      },
    },
    {
      title: t("opc.invoice.columnDue"),
      key: "due",
      render: (_: unknown, r: Invoice) => r.due_at ? new Date(r.due_at * 1000).toLocaleDateString() : "-",
    },
    {
      title: t("opc.invoice.columnCreated"),
      key: "created",
      render: (_: unknown, r: Invoice) => new Date(r.created_at * 1000).toLocaleString(),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 200,
      render: (_: unknown, r: Invoice) => (
        <Space size="small">
          {nextStatuses(r.status).length > 0 && (
            <Button
              size="small"
              icon={<SwapOutlined />}
              onClick={() => {
                setTransitionInvoice(r);
                setTransitionOpen(true);
              }}
            >
              {t("opc.invoice.transition")}
            </Button>
          )}
          <Popconfirm title={t("opc.invoice.confirmDelete")} onConfirm={() => handleDelete(r.id)}>
            <Button size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <>
      <Card
        extra={
          <Button
            type="primary"
            size="small"
            icon={<PlusOutlined />}
            onClick={() => {
              form.resetFields();
              setModalOpen(true);
            }}
          >
            {t("opc.invoice.newInvoice")}
          </Button>
        }
      >
        <Table
          dataSource={invoices}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={{ pageSize: 20 }}
        />
      </Card>

      {/* 新建发票 Modal */}
      <Modal
        title={t("opc.invoice.newInvoice")}
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          form.resetFields();
        }}
        width={640}
        okText={t("opc.common.create")}
        cancelText={t("opc.common.cancel")}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item
            name="customer_id"
            label={t("opc.invoice.customerLabel")}
            rules={[{ required: true, message: t("opc.invoice.customerRequired") }]}
          >
            <Select
              showSearch
              placeholder={t("opc.invoice.customerPlaceholder")}
              optionFilterProp="label"
              options={customers.map((c) => ({ value: c.id, label: `${c.name} (${c.email})` }))}
            />
          </Form.Item>
          <Form.List
            name="line_items"
            rules={[{
              validator: async (_, items) => {
                if (!items?.length) { throw new Error(t("opc.invoice.needItem")); }
              },
            }]}
          >
            {(fields, { add, remove }) => (
              <>
                {fields.map(({ key, name, ...rest }) => (
                  <Space key={key} style={{ display: "flex", marginBottom: 8 }} align="baseline" {...rest}>
                    <Form.Item
                      name={[name, "description"]}
                      rules={[{ required: true, message: t("opc.common.description") }]}
                      noStyle
                    >
                      <Input placeholder={t("opc.common.description")} style={{ width: 180 }} />
                    </Form.Item>
                    <Form.Item
                      name={[name, "quantity"]}
                      rules={[{ required: true, message: t("opc.invoice.quantity") }]}
                      noStyle
                    >
                      <InputNumber placeholder={t("opc.invoice.quantity")} min={1} style={{ width: 80 }} />
                    </Form.Item>
                    <Form.Item
                      name={[name, "unit_price"]}
                      rules={[{ required: true, message: t("opc.invoice.unitPrice") }]}
                      noStyle
                    >
                      <InputNumber
                        placeholder={t("opc.invoice.unitPrice")}
                        min={0}
                        precision={2}
                        prefix="¥"
                        style={{ width: 120 }}
                      />
                    </Form.Item>
                    <Form.Item name={[name, "tax_rate"]} noStyle>
                      <Select
                        style={{ width: 80 }}
                        placeholder={t("opc.invoice.taxRate")}
                        options={[
                          { value: 0, label: "0%" },
                          { value: 0.03, label: "3%" },
                          { value: 0.06, label: "6%" },
                          { value: 0.13, label: "13%" },
                        ]}
                      />
                    </Form.Item>
                    <Button
                      type="link"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={() =>
                        remove(name)}
                    />
                  </Space>
                ))}
                <Button
                  type="dashed"
                  onClick={() => add({ description: "", quantity: 1, unit_price: 0, tax_rate: 0 })}
                  icon={<PlusOutlined />}
                >
                  {t("opc.invoice.addLineItem")}
                </Button>
              </>
            )}
          </Form.List>
          <Form.Item name="due_at" label={t("opc.invoice.dueAtLabel")}>
            <DatePicker style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="notes" label={t("opc.common.notes")}>
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>

      {/* 状态流转 Modal */}
      <Modal
        title={t("opc.invoice.transitionTitle")}
        open={transitionOpen}
        onCancel={() => {
          setTransitionOpen(false);
          setTransitionInvoice(null);
        }}
        footer={null}
        width={400}
      >
        {transitionInvoice && (
          <div>
            <Descriptions size="small" column={1}>
              <Descriptions.Item label={t("opc.invoice.numberLabel")}>
                {transitionInvoice.invoice_number}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.invoice.currentStatus")}>
                <Tag color={STATUS_COLOR_MAP[transitionInvoice.status] || "default"}>
                  {t(getInvoiceStatusKey(transitionInvoice.status))}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.invoice.amountLabel")}>
                ¥{transitionInvoice.total.toFixed(2)}
              </Descriptions.Item>
            </Descriptions>
            <Divider />
            <Text strong>{t("opc.invoice.selectTargetStatus")}</Text>
            <div style={{ marginTop: 12 }}>
              {nextStatuses(transitionInvoice.status).map((ns) => (
                <Button
                  key={ns.value}
                  style={{ marginRight: 8, marginBottom: 8 }}
                  onClick={() => handleTransition(transitionInvoice.id, ns.value)}
                >
                  {ns.label}
                </Button>
              ))}
            </div>
          </div>
        )}
      </Modal>
    </>
  );
}

// ══════════════════════════════════════════════════════════════════
// 客户管理
// ══════════════════════════════════════════════════════════════════

function CustomersTab() {
  const { t } = useTranslation();
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<Customer | null>(null);
  const [form] = Form.useForm();

  const load = useCallback(() => {
    setLoading(true);
    invoke<Customer[]>("opc_list_customers", { filter: {} })
      .then(setCustomers).catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleSave = async (values: Record<string, unknown>) => {
    try {
      const payload = {
        name: values.name as string,
        email: values.email as string,
        phone: (values.phone as string) || null,
        company: (values.company as string) || null,
        source: (values.source as string) || null,
        tags: [] as string[],
        notes: (values.notes as string) || "",
      };
      if (editing) {
        await invoke("opc_update_customer", { id: editing.id, input: payload });
        message.success(t("opc.customer.updated"));
      } else {
        await invoke("opc_create_customer", { input: payload });
        message.success(t("opc.customer.created"));
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
      await invoke("opc_delete_customer", { id });
      message.success(t("opc.customer.deleted"));
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const columns = [
    { title: t("opc.customer.columnName"), dataIndex: "name", key: "name" },
    { title: t("opc.customer.columnEmail"), dataIndex: "email", key: "email" },
    {
      title: t("opc.customer.columnCompany"),
      dataIndex: "company",
      key: "company",
      render: (v: string | null) => v || "-",
    },
    {
      title: t("opc.customer.columnStatus"),
      key: "status",
      render: (_: unknown, r: Customer) => {
        const color = CUST_STATUS_COLOR_MAP[r.status] || "default";
        return <Tag color={color}>{t(getCustomerStatusKey(r.status))}</Tag>;
      },
    },
    {
      title: t("opc.customer.columnSource"),
      key: "source",
      render: (_: unknown, r: Customer) => r.source ? t(getSourceKey(r.source)) : "-",
    },
    {
      title: t("opc.customer.columnRevenue"),
      key: "revenue",
      render: (_: unknown, r: Customer) => `¥${r.total_revenue.toFixed(2)}`,
      sorter: (a: Customer, b: Customer) => a.total_revenue - b.total_revenue,
    },
    { title: t("opc.customer.columnInvoiceCount"), dataIndex: "invoice_count", key: "count", width: 80 },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 120,
      render: (_: unknown, r: Customer) => (
        <Space size="small">
          <Button
            size="small"
            icon={<EditOutlined />}
            onClick={() => {
              setEditing(r);
              form.setFieldsValue({
                name: r.name,
                email: r.email,
                phone: r.phone,
                company: r.company,
                source: r.source,
                notes: r.notes,
              });
              setModalOpen(true);
            }}
          />
          <Popconfirm title={t("opc.customer.confirmDelete")} onConfirm={() => handleDelete(r.id)}>
            <Button size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <>
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
            {t("opc.customer.newCustomer")}
          </Button>
        }
      >
        <Table
          dataSource={customers}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={{ pageSize: 20 }}
        />
      </Card>

      <Modal
        title={editing ? t("opc.customer.editTitle") : t("opc.customer.newCustomer")}
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          setEditing(null);
          form.resetFields();
        }}
        okText={editing ? t("opc.common.update") : t("opc.common.create")}
        cancelText={t("opc.common.cancel")}
      >
        <Form form={form} layout="vertical" onFinish={handleSave}>
          <Form.Item
            name="name"
            label={t("opc.customer.nameLabel")}
            rules={[{ required: true, message: t("opc.customer.nameRequired") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="email"
            label={t("opc.customer.emailLabel")}
            rules={[{ required: true, type: "email", message: t("opc.customer.emailRequired") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="phone" label={t("opc.customer.phoneLabel")}>
            <Input />
          </Form.Item>
          <Form.Item name="company" label={t("opc.customer.companyLabel")}>
            <Input />
          </Form.Item>
          <Form.Item name="source" label={t("opc.customer.sourceLabel")}>
            <Select
              allowClear
              placeholder={t("opc.customer.sourcePlaceholder")}
              options={[
                { value: "Referral", label: t("opc.source.Referral") },
                { value: "Website", label: t("opc.source.Website") },
                { value: "SocialMedia", label: t("opc.source.SocialMedia") },
                { value: "Marketplace", label: t("opc.source.Marketplace") },
                { value: "Direct", label: t("opc.source.Direct") },
              ]}
            />
          </Form.Item>
          <Form.Item name="notes" label={t("opc.common.notes")}>
            <Input.TextArea rows={3} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}

// ══════════════════════════════════════════════════════════════════
// 项目管理
// ══════════════════════════════════════════════════════════════════

function ProjectsTab() {
  const { t } = useTranslation();
  const [projects, setProjects] = useState<Project[]>([]);
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<Project | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailProject, setDetailProject] = useState<Project | null>(null);
  const [milestoneOpen, setMilestoneOpen] = useState(false);
  const [milestoneForm] = Form.useForm();
  const [form] = Form.useForm();

  const load = useCallback(() => {
    setLoading(true);
    Promise.all([
      invoke<Project[]>("opc_list_projects", { filter: {} }),
      invoke<Customer[]>("opc_list_customers", { filter: {} }),
    ]).then(([proj, cust]) => {
      setProjects(proj);
      setCustomers(cust);
    }).catch(console.error).finally(() => setLoading(false));
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
          status: "Pending",
        },
      });
      message.success(t("opc.project.milestoneAdded"));
      setMilestoneOpen(false);
      milestoneForm.resetFields();
      // 刷新项目详情
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
        const done = r.milestones.filter((m) => m.status === "Completed").length;
        return r.milestones.length > 0 ? `${done}/${r.milestones.length}` : "-";
      },
    },
    {
      title: t("opc.project.columnBudget"),
      key: "budget",
      render: (_: unknown, r: Project) => r.budget ? `¥${r.budget.toFixed(2)}` : "-",
      sorter: (a: Project, b: Project) => (a.budget || 0) - (b.budget || 0),
    },
    {
      title: t("opc.project.columnDeadline"),
      key: "deadline",
      render: (_: unknown, r: Project) => r.deadline ? new Date(r.deadline * 1000).toLocaleDateString() : "-",
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
          dataSource={projects}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={{ pageSize: 20 }}
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
            {detailProject.milestones.length === 0
              ? <Empty description={t("opc.project.noMilestones")} />
              : (
                <Timeline
                  items={detailProject.milestones.map((m) => ({
                    color: m.status === "Completed" ? "green" : m.status === "InProgress" ? "blue" : "gray",
                    children: (
                      <div>
                        <div style={{ display: "flex", justifyContent: "space-between" }}>
                          <Text strong>{m.title}</Text>
                          {m.status !== "Completed" && (
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

// ══════════════════════════════════════════════════════════════════
// 站点管理
// ══════════════════════════════════════════════════════════════════

interface _LandingPage {
  id: string;
  title: string;
  slug: string;
  description: string;
  published: boolean;
  published_at: number | null;
  created_at: number;
}
interface _BlogPost {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  tags: string[];
  published: boolean;
  view_count: number;
  created_at: number;
}
interface _ContactSubmission {
  id: string;
  name: string;
  email: string;
  message: string;
  source: string;
  read: boolean;
  created_at: number;
}

function SitesTab() {
  const { t } = useTranslation();
  const [subTab, setSubTab] = useState("landing");
  return (
    <Tabs
      activeKey={subTab}
      onChange={setSubTab}
      size="small"
      items={[
        { key: "landing", label: t("opc.site.tabLanding"), children: <_LandingPagesPanel /> },
        { key: "blog", label: t("opc.site.tabBlog"), children: <_BlogPostsPanel /> },
        { key: "contacts", label: t("opc.site.tabContacts"), children: <_ContactsPanel /> },
      ]}
    />
  );
}

function _LandingPagesPanel() {
  const { t } = useTranslation();
  const [pages, setPages] = useState<_LandingPage[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [form] = Form.useForm();

  const load = () => {
    setLoading(true);
    invoke<_LandingPage[]>("opc_list_landing_pages").then(setPages).catch(console.error).finally(() =>
      setLoading(false)
    );
  };
  useEffect(() => {
    load();
  }, []);

  const handleCreate = async (values: Record<string, unknown>) => {
    try {
      await invoke("opc_create_landing_page", {
        input: {
          title: values.title,
          slug: values.slug,
          description: (values.description as string) || "",
          content: "",
        },
      });
      message.success(t("opc.site.landingCreated"));
      setModalOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(t("opc.common.createFailed", { error: String(e) }));
    }
  };

  const handlePublish = async (id: string) => {
    try {
      await invoke("opc_publish_landing_page", { id });
      message.success(t("opc.site.published"));
      load();
    } catch (e) {
      message.error(t("opc.site.publishFailed", { error: String(e) }));
    }
  };

  const columns = [
    { title: t("opc.site.columnTitle"), dataIndex: "title", key: "title" },
    { title: t("opc.site.columnSlug"), dataIndex: "slug", key: "slug" },
    {
      title: t("opc.site.columnStatus"),
      key: "status",
      render: (_: unknown, r: _LandingPage) =>
        r.published ? <Tag color="green">{t("opc.site.published")}</Tag> : <Tag>{t("opc.site.draftTag")}</Tag>,
    },
    {
      title: t("opc.site.columnCreated"),
      key: "created",
      render: (_: unknown, r: _LandingPage) => new Date(r.created_at * 1000).toLocaleDateString(),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 100,
      render: (_: unknown, r: _LandingPage) =>
        !r.published && <Button size="small" onClick={() => handlePublish(r.id)}>{t("opc.site.publish")}</Button>,
    },
  ];

  return (
    <Card
      extra={
        <Button
          type="primary"
          size="small"
          icon={<PlusOutlined />}
          onClick={() => {
            form.resetFields();
            setModalOpen(true);
          }}
        >
          {t("opc.site.newLanding")}
        </Button>
      }
    >
      <Table
        dataSource={pages}
        columns={columns}
        rowKey="id"
        loading={loading}
        size="small"
        pagination={{ pageSize: 20 }}
      />
      <Modal
        title={t("opc.site.landingModalTitle")}
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          form.resetFields();
        }}
        okText={t("opc.common.create")}
        cancelText={t("opc.common.cancel")}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="title" label={t("opc.site.titleLabel")} rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="slug" label={t("opc.site.slugLabel")} rules={[{ required: true }]}>
            <Input placeholder={t("opc.site.landingSlugPlaceholder")} />
          </Form.Item>
          <Form.Item name="description" label={t("opc.common.description")}>
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  );
}

function _BlogPostsPanel() {
  const { t } = useTranslation();
  const [posts, setPosts] = useState<_BlogPost[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [form] = Form.useForm();

  const load = () => {
    setLoading(true);
    invoke<_BlogPost[]>("opc_list_blog_posts").then(setPosts).catch(console.error).finally(() => setLoading(false));
  };
  useEffect(() => {
    load();
  }, []);

  const handleCreate = async (values: Record<string, unknown>) => {
    try {
      await invoke("opc_create_blog_post", {
        input: {
          title: values.title,
          slug: values.slug,
          excerpt: (values.excerpt as string) || "",
          content: "",
          tags: values.tags ? ((values.tags as string).split(",").map((s) => s.trim()).filter(Boolean)) : [],
        },
      });
      message.success(t("opc.site.postCreated"));
      setModalOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(t("opc.common.createFailed", { error: String(e) }));
    }
  };

  const handlePublish = async (id: string) => {
    try {
      await invoke("opc_publish_blog_post", { id });
      message.success(t("opc.site.published"));
      load();
    } catch (e) {
      message.error(t("opc.site.publishFailed", { error: String(e) }));
    }
  };

  const columns = [
    { title: t("opc.site.columnTitle"), dataIndex: "title", key: "title" },
    { title: t("opc.site.columnSlug"), dataIndex: "slug", key: "slug" },
    {
      title: t("opc.site.columnStatus"),
      key: "status",
      render: (_: unknown, r: _BlogPost) =>
        r.published ? <Tag color="green">{t("opc.site.published")}</Tag> : <Tag>{t("opc.site.draftTag")}</Tag>,
    },
    { title: t("opc.site.columnViews"), dataIndex: "view_count", key: "views", width: 60 },
    {
      title: t("opc.site.columnTags"),
      key: "tags",
      render: (_: unknown, r: _BlogPost) => r.tags.map((t) => <Tag key={t}>{t}</Tag>),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 100,
      render: (_: unknown, r: _BlogPost) =>
        !r.published && <Button size="small" onClick={() => handlePublish(r.id)}>{t("opc.site.publish")}</Button>,
    },
  ];

  return (
    <Card
      extra={
        <Button
          type="primary"
          size="small"
          icon={<PlusOutlined />}
          onClick={() => {
            form.resetFields();
            setModalOpen(true);
          }}
        >
          {t("opc.site.newPost")}
        </Button>
      }
    >
      <Table
        dataSource={posts}
        columns={columns}
        rowKey="id"
        loading={loading}
        size="small"
        pagination={{ pageSize: 20 }}
      />
      <Modal
        title={t("opc.site.postModalTitle")}
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          form.resetFields();
        }}
        okText={t("opc.common.create")}
        cancelText={t("opc.common.cancel")}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="title" label={t("opc.site.titleLabel")} rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="slug" label={t("opc.site.slugLabel")} rules={[{ required: true }]}>
            <Input placeholder={t("opc.site.postSlugPlaceholder")} />
          </Form.Item>
          <Form.Item name="excerpt" label={t("opc.site.excerptLabel")}>
            <Input.TextArea rows={2} />
          </Form.Item>
          <Form.Item name="tags" label={t("opc.site.tagsLabel")}>
            <Input placeholder={t("opc.site.tagsPlaceholder")} />
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  );
}

function _ContactsPanel() {
  const { t } = useTranslation();
  const [contacts, setContacts] = useState<_ContactSubmission[]>([]);
  const [loading, setLoading] = useState(true);

  const load = () => {
    setLoading(true);
    invoke<_ContactSubmission[]>("opc_list_contacts").then(setContacts).catch(console.error).finally(() =>
      setLoading(false)
    );
  };
  useEffect(() => {
    load();
  }, []);

  const handleMarkRead = async (id: string) => {
    try {
      await invoke("opc_mark_contact_read", { id });
      load();
    } catch (e) {
      message.error(t("opc.common.opFailed", { error: String(e) }));
    }
  };

  const columns = [
    { title: t("opc.site.contactColumnName"), dataIndex: "name", key: "name" },
    { title: t("opc.site.contactColumnEmail"), dataIndex: "email", key: "email" },
    { title: t("opc.site.contactColumnMessage"), dataIndex: "message", key: "message", ellipsis: true, width: 300 },
    { title: t("opc.site.contactColumnSource"), dataIndex: "source", key: "source" },
    {
      title: t("opc.site.contactColumnStatus"),
      key: "status",
      render: (_: unknown, r: _ContactSubmission) =>
        r.read ? <Tag>{t("opc.site.readTag")}</Tag> : <Tag color="orange">{t("opc.site.unreadTag")}</Tag>,
    },
    {
      title: t("opc.site.contactColumnTime"),
      key: "created",
      render: (_: unknown, r: _ContactSubmission) => new Date(r.created_at * 1000).toLocaleString(),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 80,
      render: (_: unknown, r: _ContactSubmission) =>
        !r.read && <Button size="small" onClick={() => handleMarkRead(r.id)}>{t("opc.site.markRead")}</Button>,
    },
  ];

  return (
    <Card>
      <Table
        dataSource={contacts}
        columns={columns}
        rowKey="id"
        loading={loading}
        size="small"
        pagination={{ pageSize: 20 }}
      />
    </Card>
  );
}

interface TalentRole {
  id: string;
  nameKey: string;
  descriptionKey: string;
  category: string;
  icon: string;
}

const TALENT_CATEGORIES: Record<string, { labelKey: string; icon: string }> = {
  engineering: { labelKey: "opc.talent.catEngineering", icon: "💻" },
  design: { labelKey: "opc.talent.catDesign", icon: "🎨" },
  finance: { labelKey: "opc.talent.catFinance", icon: "💰" },
  marketing: { labelKey: "opc.talent.catMarketing", icon: "📢" },
  sales: { labelKey: "opc.talent.catSales", icon: "🤝" },
  product: { labelKey: "opc.talent.catProduct", icon: "📋" },
  security: { labelKey: "opc.talent.catSecurity", icon: "🔒" },
  data: { labelKey: "opc.talent.catData", icon: "📊" },
  devops: { labelKey: "opc.talent.catDevops", icon: "🚀" },
  testing: { labelKey: "opc.talent.catTesting", icon: "🧪" },
  support: { labelKey: "opc.talent.catSupport", icon: "🎧" },
  academic: { labelKey: "opc.talent.catAcademic", icon: "🎓" },
};

const TALENT_ROLES: TalentRole[] = [
  {
    id: "ai-engineer",
    nameKey: "opc.talent.roleAiEngineer",
    descriptionKey: "opc.talent.roleAiEngineerDesc",
    category: "engineering",
    icon: "🤖",
  },
  {
    id: "backend-architect",
    nameKey: "opc.talent.roleBackendArchitect",
    descriptionKey: "opc.talent.roleBackendArchitectDesc",
    category: "engineering",
    icon: "🏗️",
  },
  {
    id: "frontend-developer",
    nameKey: "opc.talent.roleFrontendDev",
    descriptionKey: "opc.talent.roleFrontendDevDesc",
    category: "engineering",
    icon: "🖥️",
  },
  {
    id: "devops-engineer",
    nameKey: "opc.talent.roleDevops",
    descriptionKey: "opc.talent.roleDevopsDesc",
    category: "engineering",
    icon: "🚀",
  },
  {
    id: "code-reviewer",
    nameKey: "opc.talent.roleCodeReviewer",
    descriptionKey: "opc.talent.roleCodeReviewerDesc",
    category: "engineering",
    icon: "👀",
  },
  {
    id: "financial-analyst",
    nameKey: "opc.talent.roleFinancialAnalyst",
    descriptionKey: "opc.talent.roleFinancialAnalystDesc",
    category: "finance",
    icon: "📈",
  },
  {
    id: "accountant",
    nameKey: "opc.talent.roleAccountant",
    descriptionKey: "opc.talent.roleAccountantDesc",
    category: "finance",
    icon: "🧾",
  },
  {
    id: "security-expert",
    nameKey: "opc.talent.roleSecurityExpert",
    descriptionKey: "opc.talent.roleSecurityExpertDesc",
    category: "security",
    icon: "🛡️",
  },
  {
    id: "data-scientist",
    nameKey: "opc.talent.roleDataScientist",
    descriptionKey: "opc.talent.roleDataScientistDesc",
    category: "data",
    icon: "📊",
  },
  {
    id: "seo-specialist",
    nameKey: "opc.talent.roleSeoSpecialist",
    descriptionKey: "opc.talent.roleSeoSpecialistDesc",
    category: "marketing",
    icon: "🔍",
  },
  {
    id: "sales-engineer",
    nameKey: "opc.talent.roleSalesEngineer",
    descriptionKey: "opc.talent.roleSalesEngineerDesc",
    category: "sales",
    icon: "🤝",
  },
  {
    id: "product-manager",
    nameKey: "opc.talent.roleProductManager",
    descriptionKey: "opc.talent.roleProductManagerDesc",
    category: "product",
    icon: "📋",
  },
  {
    id: "ux-designer",
    nameKey: "opc.talent.roleUxDesigner",
    descriptionKey: "opc.talent.roleUxDesignerDesc",
    category: "design",
    icon: "🎨",
  },
  {
    id: "qa-engineer",
    nameKey: "opc.talent.roleQaEngineer",
    descriptionKey: "opc.talent.roleQaEngineerDesc",
    category: "testing",
    icon: "🧪",
  },
  {
    id: "tech-support",
    nameKey: "opc.talent.roleTechSupport",
    descriptionKey: "opc.talent.roleTechSupportDesc",
    category: "support",
    icon: "🎧",
  },
];

function TalentMarketTab() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [importedIds, setImportedIds] = useState<Set<string>>(new Set());
  const [importing, setImporting] = useState<string | null>(null);
  const [loading] = useState(true);

  // 预置人才列表（扫描 agency-agents 的 fallback）
  const allRoles: TalentRole[] = TALENT_ROLES;

  // 加载已导入的角色
  useEffect(() => {
    invoke<Array<{ id: string }>>("list_agency_experts")
      .then((rows) => setImportedIds(new Set(rows.map((r) => r.id))))
      .catch(() => {});
  }, []);

  const handleHire = async (roleId: string) => {
    setImporting(roleId);
    try {
      await invoke("import_agency_experts", { request: { path: "agency-agents-src" } });
      message.success(t("opc.talent.hireSuccess", { name: t(allRoles.find((r) => r.id === roleId)?.nameKey || "") }));
      setImportedIds((prev) => new Set(prev).add(roleId));
    } catch (e) {
      message.error(t("opc.talent.hireFailed", { error: String(e) }));
    } finally {
      setImporting(null);
    }
  };

  const filtered = allRoles.filter((r) => {
    if (category && r.category !== category) { return false; }
    if (search && !t(r.nameKey).includes(search) && !t(r.descriptionKey).includes(search)) { return false; }
    return true;
  });

  const categories = [...new Set(allRoles.map((r) => r.category))].sort();

  return (
    <div>
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={8}>
          <Input.Search
            placeholder={t("opc.talent.searchPlaceholder")}
            allowClear
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </Col>
        <Col span={16}>
          <Space wrap>
            <Button size="small" type={category === null ? "primary" : "default"} onClick={() => setCategory(null)}>
              {t("opc.talent.all")}
            </Button>
            {categories.map((cat) => (
              <Button
                key={cat}
                size="small"
                type={category === cat ? "primary" : "default"}
                onClick={() => setCategory(cat)}
              >
                {TALENT_CATEGORIES[cat]?.icon} {TALENT_CATEGORIES[cat] ? t(TALENT_CATEGORIES[cat].labelKey) : cat}
              </Button>
            ))}
          </Space>
        </Col>
      </Row>
      {loading ? <Spin style={{ display: "block", margin: "80px auto" }} /> : (
        <Row gutter={[12, 12]}>
          {filtered.length === 0
            ? (
              <Col span={24}>
                <Empty description={t("opc.talent.noMatch")} />
              </Col>
            )
            : (
              filtered.map((role) => {
                const isImported = importedIds.has(role.id);
                return (
                  <Col span={6} key={role.id}>
                    <Card
                      size="small"
                      hoverable
                      style={{ height: "100%" }}
                      actions={[
                        isImported
                          ? <Tag color="green">{t("opc.talent.onboarded")}</Tag>
                          : (
                            <Button
                              type="primary"
                              size="small"
                              loading={importing === role.id}
                              onClick={() => handleHire(role.id)}
                            >
                              {t("opc.talent.hire")}
                            </Button>
                          ),
                      ]}
                    >
                      <Card.Meta
                        avatar={<div style={{ fontSize: 28 }}>{role.icon}</div>}
                        title={<span style={{ fontSize: 13 }}>{t(role.nameKey)}</span>}
                        description={
                          <div>
                            <Tag>
                              {TALENT_CATEGORIES[role.category]?.icon} {TALENT_CATEGORIES[role.category]
                                ? t(TALENT_CATEGORIES[role.category].labelKey)
                                : role.category}
                            </Tag>
                            <div style={{ fontSize: 12, color: "rgba(255,255,255,0.6)", marginTop: 4 }}>
                              {t(role.descriptionKey)}
                            </div>
                          </div>
                        }
                      />
                    </Card>
                  </Col>
                );
              })
            )}
        </Row>
      )}
    </div>
  );
}

// ══════════════════════════════════════════════════════════════════
// 看板（P3：WorkItem 状态机投影）
// ══════════════════════════════════════════════════════════════════

interface KanbanItem {
  id: string;
  title: string;
  phase: string;
  owner_role_id: string | null;
  assignee_agent_id: string | null;
  manager_role_id: string | null;
  last_error: string | null;
  deps: string[];
  created_at: number;
  updated_at: number;
}

type KanbanBoard = Record<string, KanbanItem[]>;

/** 自改进循环结果（run_self_improving_opc_work_item 返回） */
interface SirResult {
  text: string;
  totalRounds: number;
  finalScore: number;
  confidence: number;
  strengths: string[];
  gaps: string[];
}

const KANBAN_COLUMNS = [
  "opc.kanban.colTodo",
  "opc.kanban.colInProgress",
  "opc.kanban.colBlocked",
  "opc.kanban.colReview",
  "opc.kanban.colDone",
  "opc.kanban.colCancelled",
];

function KanbanTab() {
  const { t } = useTranslation();
  const [board, setBoard] = useState<KanbanBoard>({});
  const [loading, setLoading] = useState(false);
  const [acting, setActing] = useState<string | null>(null);
  const [sirRunning, setSirRunning] = useState(false);
  const [sirResult, setSirResult] = useState<SirResult | null>(null);
  const [sirModalOpen, setSirModalOpen] = useState(false);

  const refresh = useCallback(() => {
    setLoading(true);
    invoke<KanbanBoard>("opc_kanban_board")
      .then(setBoard)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const act = async (id: string, cmd: string, extra?: Record<string, unknown>) => {
    setActing(id);
    try {
      await invoke(cmd, { id, ...extra });
      message.success(t("opc.kanban.opSuccess", { cmd }));
      refresh();
    } catch (e) {
      message.error(t("opc.kanban.opFailed", { cmd, error: String(e) }));
    } finally {
      setActing(null);
    }
  };

  /** 运行自改进循环（对接上游 Loop Engineering） */
  const runSIR = async (id: string) => {
    setActing(id);
    setSirRunning(true);
    try {
      const result = await invoke<SirResult>("run_self_improving_opc_work_item", {
        task: id,
        maxRounds: 3,
      });
      setSirResult(result);
      setSirModalOpen(true);
    } catch (e) {
      message.error(t("opc.kanban.sirRunFailed", { error: String(e) }));
    } finally {
      setActing(null);
      setSirRunning(false);
    }
  };

  return (
    <div>
      <Space style={{ marginBottom: 12 }}>
        <Button size="small" type="primary" icon={<ProjectOutlined />} onClick={refresh} loading={loading}>
          {t("opc.kanban.refresh")}
        </Button>
        <Typography.Text type="secondary">
          {t("opc.kanban.machineDesc")}
        </Typography.Text>
      </Space>
      <Row gutter={[12, 12]}>
        {KANBAN_COLUMNS.map((col) => {
          const items = board[col] ?? [];
          const colLabel = t(col);
          return (
            <Col key={col} span={4}>
              <Card
                size="small"
                title={
                  <Space>
                    {colLabel}
                    <Tag
                      color={colLabel === t("opc.kanban.colBlocked")
                        ? "red"
                        : colLabel === t("opc.kanban.colDone")
                        ? "green"
                        : "blue"}
                    >
                      {items.length}
                    </Tag>
                  </Space>
                }
                style={{ minHeight: 200, background: colLabel === t("opc.kanban.colBlocked") ? "#fff2f0" : undefined }}
              >
                {items.length === 0
                  ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("opc.kanban.empty")} />
                  : (
                    <Space direction="vertical" style={{ width: "100%" }}>
                      {items.map((it) => (
                        <Card key={it.id} size="small" styles={{ body: { padding: 8 } }}>
                          <Typography.Text strong style={{ fontSize: 12 }}>
                            {it.title}
                          </Typography.Text>
                          <div style={{ fontSize: 11, color: "#888", marginTop: 4 }}>
                            <div>ID: {it.id}</div>
                            <div>{t("opc.kanban.owner", { id: it.owner_role_id ?? "-" })}</div>
                            {it.deps.length > 0 && <div>{t("opc.kanban.deps", { deps: it.deps.join(", ") })}</div>}
                            {it.last_error && <div style={{ color: "#cf1322" }}>⚠ {it.last_error}</div>}
                          </div>
                          <Space wrap style={{ marginTop: 6 }}>
                            <Button
                              size="small"
                              icon={<ProjectOutlined />}
                              loading={acting === it.id && sirRunning}
                              disabled={sirRunning && acting !== it.id}
                              onClick={() => runSIR(it.id)}
                              title={t("opc.kanban.sir")}
                            >
                              {sirRunning && acting === it.id ? t("opc.kanban.sirRunning") : t("opc.kanban.sir")}
                            </Button>
                            {it.phase === "QUEUED" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_start")}
                              >
                                {t("opc.kanban.claim")}
                              </Button>
                            )}
                            {it.phase === "IN_PROGRESS" && (
                              <Button
                                size="small"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_review")}
                              >
                                {t("opc.kanban.submitReview")}
                              </Button>
                            )}
                            {it.phase === "REVIEW" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_start")}
                              >
                                {t("opc.kanban.approveDone")}
                              </Button>
                            )}
                            {it.phase !== "BLOCKED" && it.phase !== "DONE" && it.phase !== "APPROVED"
                              && it.phase !== "FAILED" && it.phase !== "CANCELLED" && (
                              <Button
                                size="small"
                                danger
                                loading={acting === it.id}
                                onClick={() => {
                                  const reason = window.prompt(
                                    t("opc.kanban.escalateReason"),
                                    t("opc.kanban.escalateDefault"),
                                  );
                                  if (reason !== null) { act(it.id, "opc_escalate_work_item", { reason }); }
                                }}
                              >
                                {t("opc.kanban.escalate")}
                              </Button>
                            )}
                            {it.phase === "BLOCKED" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_unblock")}
                              >
                                {t("opc.kanban.unblock")}
                              </Button>
                            )}
                          </Space>
                        </Card>
                      ))}
                    </Space>
                  )}
              </Card>
            </Col>
          );
        })}
      </Row>

      {/* 自改进循环结果 Modal */}
      <Modal
        open={sirModalOpen}
        title={t("opc.kanban.sirTitle")}
        onCancel={() => setSirModalOpen(false)}
        footer={null}
        width={720}
      >
        {sirResult && (
          <div>
            <Descriptions size="small" column={3} bordered style={{ marginBottom: 12 }}>
              <Descriptions.Item label={t("opc.kanban.sirScore")}>
                {(sirResult.finalScore * 100).toFixed(1)}%
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirRounds")}>
                {sirResult.totalRounds}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirAccept")}>
                {sirResult.finalScore >= 0.85 ? "✅" : "⏳"}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirStrengths")} span={3}>
                {sirResult.strengths.length > 0 ? sirResult.strengths.join("；") : "-"}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirGaps")} span={3}>
                {sirResult.gaps.length > 0 ? sirResult.gaps.join("；") : "-"}
              </Descriptions.Item>
            </Descriptions>
            <pre
              style={{
                maxHeight: 320,
                overflow: "auto",
                fontSize: 12,
                background: "rgba(0,0,0,0.03)",
                padding: 12,
                borderRadius: 6,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
              }}
            >
              {sirResult.text}
            </pre>
          </div>
        )}
      </Modal>
    </div>
  );
}

// ══════════════════════════════════════════════════════════════════
// 市场包（P4-1：.opcip 行业包市场）
// ══════════════════════════════════════════════════════════════════

interface MarketPack {
  id: string;
  name: string;
  icon: string;
  version: number;
  enabled: boolean;
  installed: boolean;
  path: string;
}

function MarketPackTab() {
  const { t } = useTranslation();
  const [packs, setPacks] = useState<MarketPack[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(() => {
    setLoading(true);
    invoke<MarketPack[]>("opc_market_list")
      .then(setPacks)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div>
      <Space style={{ marginBottom: 12 }}>
        <Button size="small" type="primary" onClick={refresh} loading={loading}>
          {t("opc.market.refresh")}
        </Button>
        <Typography.Text type="secondary">
          {t("opc.market.subtitle")}
        </Typography.Text>
      </Space>
      <Row gutter={[12, 12]}>
        {packs.map((p) => (
          <Col key={p.id} span={8}>
            <Card
              size="small"
              title={
                <Space>
                  <span>{p.icon}</span>
                  {p.name}
                  <Tag color={p.installed ? "green" : "blue"}>
                    {p.installed ? t("opc.market.installed") : t("opc.market.notInstalled")}
                  </Tag>
                </Space>
              }
            >
              <div style={{ fontSize: 12, color: "#888" }}>
                <div>ID: {p.id}</div>
                <div>{t("opc.market.version", { version: p.version })}</div>
                <div>{t("opc.market.enabled", { value: p.enabled ? t("opc.market.yes") : t("opc.market.no") })}</div>
              </div>
              <Space style={{ marginTop: 8 }}>
                <Button
                  size="small"
                  type={p.installed ? "default" : "primary"}
                  disabled={p.installed}
                  onClick={async () => {
                    try {
                      await invoke("opc_import_industry_pack", {
                        archivePath: p.path,
                      });
                      message.success(t("opc.market.installSuccess", { name: p.name }));
                      refresh();
                    } catch (e) {
                      message.error(t("opc.market.installFailed", { error: String(e) }));
                    }
                  }}
                >
                  {t("opc.market.install")}
                </Button>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>
    </div>
  );
}

// ══════════════════════════════════════════════════════════════════
// 办公室 tab（方案 B）：挂载时同步 OPC 角色进 Fleet，渲染上游舰队办公室
// ══════════════════════════════════════════════════════════════════

/** 上游舰队办公室（真实 Agent 状态 + Phaser 场景）。AxOPC 静态版已退役。 */
function OpcOfficeTab() {
  const { t } = useTranslation();
  const [ready, setReady] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    invoke("opc_sync_fleet")
      .then(() => setReady(true))
      .catch((e) => {
        console.error("[opc-sync-fleet] failed", e);
        setErr(String(e));
      });
  }, []);

  if (err) {
    return <Alert type="warning" message={t("opc.office.syncFailed", { error: String(err) })} />;
  }
  if (!ready) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Spin tip={t("opc.office.syncTip")} />
      </div>
    );
  }
  return <FleetOfficeTab />;
}
