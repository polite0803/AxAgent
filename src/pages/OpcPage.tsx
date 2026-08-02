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

// ── 状态标签映射 ──────────────────────────────────────────────────

const STATUS_MAP: Record<string, { color: string; label: string }> = {
  draft: { color: "default", label: "草稿" },
  sent: { color: "blue", label: "已发送" },
  paid: { color: "green", label: "已收款" },
  overdue: { color: "red", label: "逾期" },
  cancelled: { color: "default", label: "已取消" },
  refunded: { color: "orange", label: "已退款" },
};

const CUST_STATUS: Record<string, { color: string; label: string }> = {
  lead: { color: "default", label: "线索" },
  prospect: { color: "blue", label: "潜在" },
  active: { color: "green", label: "活跃" },
  inactive: { color: "default", label: "非活跃" },
  churned: { color: "red", label: "流失" },
};

const PROJ_STATUS: Record<string, { color: string; label: string }> = {
  planning: { color: "blue", label: "规划中" },
  active: { color: "green", label: "进行中" },
  paused: { color: "orange", label: "暂停" },
  completed: { color: "default", label: "已完成" },
  cancelled: { color: "red", label: "已取消" },
};

const SOURCE_MAP: Record<string, string> = {
  Referral: "推荐",
  Website: "网站",
  SocialMedia: "社交媒体",
  Marketplace: "市场",
  Direct: "直接",
};

// ── 主页面 ───────────────────────────────────────────────────────

export function OpcPage() {
  const [tab, setTab] = useState("dashboard");

  return (
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      <Title level={3} style={{ marginBottom: 16 }}>
        <FileTextOutlined style={{ marginRight: 8 }} />
        OPC — 一人公司管理面板
      </Title>
      <Tabs
        activeKey={tab}
        onChange={setTab}
        items={[
          {
            key: "dashboard",
            label: (
              <span>
                <RiseOutlined /> 仪表盘
              </span>
            ),
            children: <DashboardTab />,
          },
          {
            key: "invoices",
            label: (
              <span>
                <DollarOutlined /> 发票
              </span>
            ),
            children: <InvoicesTab />,
          },
          {
            key: "customers",
            label: (
              <span>
                <TeamOutlined /> 客户
              </span>
            ),
            children: <CustomersTab />,
          },
          {
            key: "projects",
            label: (
              <span>
                <ProjectOutlined /> 项目
              </span>
            ),
            children: <ProjectsTab />,
          },
          {
            key: "sites",
            label: (
              <span>
                <FileTextOutlined /> 站点
              </span>
            ),
            children: <SitesTab />,
          },
          {
            key: "office",
            label: (
              <span>
                <TeamOutlined /> 办公室
              </span>
            ),
            children: <OfficeTab />,
          },
          {
            key: "talent",
            label: (
              <span>
                <SearchOutlined /> 人才市场
              </span>
            ),
            children: <TalentMarketTab />,
          },
          {
            key: "market",
            label: (
              <span>
                <RiseOutlined /> 市场包
              </span>
            ),
            children: <MarketPackTab />,
          },
          {
            key: "kanban",
            label: (
              <span>
                <ProjectOutlined /> 看板
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
  if (!summary) { return <Empty description="无法加载仪表盘" />; }

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <Card size="small">
            <Statistic title="总收入" value={summary.total_revenue} prefix="¥" precision={2} />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic title="发票总数" value={summary.total_invoices} prefix={<FileTextOutlined />} />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic title="活跃客户" value={summary.total_customers} prefix={<TeamOutlined />} />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic title="活跃项目" value={summary.active_projects} prefix={<ProjectOutlined />} />
          </Card>
        </Col>
      </Row>
      <Row gutter={16}>
        <Col span={12}>
          <Card title="KPI 指标" size="small">
            {summary.recent_kpis.length === 0
              ? <Empty description="暂无 KPI" />
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
          <Card title="快速操作" size="small">
            <Space direction="vertical" style={{ width: "100%" }}>
              <Button
                type="primary"
                block
                icon={<DollarOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "invoices" }))}
              >
                管理发票
              </Button>
              <Button
                block
                icon={<TeamOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "customers" }))}
              >
                管理客户
              </Button>
              <Button
                block
                icon={<ProjectOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "projects" }))}
              >
                管理项目
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
      message.success("发票创建成功");
      setModalOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(`创建失败: ${e}`);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("opc_delete_invoice", { id });
      message.success("发票已删除");
      load();
    } catch (e) {
      message.error(`删除失败: ${e}`);
    }
  };

  const handleTransition = async (id: string, status: string) => {
    try {
      await invoke("opc_transition_invoice", { id, targetStatus: status });
      message.success("状态已更新");
      setTransitionOpen(false);
      setTransitionInvoice(null);
      load();
    } catch (e) {
      message.error(`状态变更失败: ${e}`);
    }
  };

  const nextStatuses = (status: string): Array<{ value: string; label: string }> => {
    const map: Record<string, Array<{ value: string; label: string }>> = {
      draft: [{ value: "sent", label: "发送" }, { value: "cancelled", label: "取消" }],
      sent: [{ value: "paid", label: "标记已收款" }, { value: "overdue", label: "标记逾期" }, {
        value: "cancelled",
        label: "取消",
      }],
      overdue: [{ value: "paid", label: "标记已收款" }, { value: "cancelled", label: "取消" }],
      paid: [{ value: "refunded", label: "退款" }],
    };
    return map[status] || [];
  };

  const columns = [
    { title: "编号", dataIndex: "invoice_number", key: "number", width: 180 },
    {
      title: "金额",
      key: "total",
      render: (_: unknown, r: Invoice) => `¥${r.total.toFixed(2)}`,
      sorter: (a: Invoice, b: Invoice) => a.total - b.total,
    },
    {
      title: "状态",
      key: "status",
      render: (_: unknown, r: Invoice) => {
        const s = STATUS_MAP[r.status] || { color: "default", label: r.status };
        return <Tag color={s.color}>{s.label}</Tag>;
      },
    },
    {
      title: "到期日",
      key: "due",
      render: (_: unknown, r: Invoice) => r.due_at ? new Date(r.due_at * 1000).toLocaleDateString() : "-",
    },
    {
      title: "创建时间",
      key: "created",
      render: (_: unknown, r: Invoice) => new Date(r.created_at * 1000).toLocaleString(),
    },
    {
      title: "操作",
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
              流转
            </Button>
          )}
          <Popconfirm title="确认删除此发票？" onConfirm={() => handleDelete(r.id)}>
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
            新建发票
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
        title="新建发票"
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          form.resetFields();
        }}
        width={640}
        okText="创建"
        cancelText="取消"
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="customer_id" label="客户" rules={[{ required: true, message: "请选择客户" }]}>
            <Select
              showSearch
              placeholder="选择客户"
              optionFilterProp="label"
              options={customers.map((c) => ({ value: c.id, label: `${c.name} (${c.email})` }))}
            />
          </Form.Item>
          <Form.List
            name="line_items"
            rules={[{
              validator: async (_, items) => {
                if (!items?.length) { throw new Error("至少需要一个行项目"); }
              },
            }]}
          >
            {(fields, { add, remove }) => (
              <>
                {fields.map(({ key, name, ...rest }) => (
                  <Space key={key} style={{ display: "flex", marginBottom: 8 }} align="baseline" {...rest}>
                    <Form.Item name={[name, "description"]} rules={[{ required: true, message: "描述" }]} noStyle>
                      <Input placeholder="描述" style={{ width: 180 }} />
                    </Form.Item>
                    <Form.Item name={[name, "quantity"]} rules={[{ required: true, message: "数量" }]} noStyle>
                      <InputNumber placeholder="数量" min={1} style={{ width: 80 }} />
                    </Form.Item>
                    <Form.Item name={[name, "unit_price"]} rules={[{ required: true, message: "单价" }]} noStyle>
                      <InputNumber placeholder="单价" min={0} precision={2} prefix="¥" style={{ width: 120 }} />
                    </Form.Item>
                    <Form.Item name={[name, "tax_rate"]} noStyle>
                      <Select
                        style={{ width: 80 }}
                        placeholder="税率"
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
                  添加行项目
                </Button>
              </>
            )}
          </Form.List>
          <Form.Item name="due_at" label="到期日">
            <DatePicker style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="notes" label="备注">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>

      {/* 状态流转 Modal */}
      <Modal
        title="发票状态流转"
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
              <Descriptions.Item label="编号">{transitionInvoice.invoice_number}</Descriptions.Item>
              <Descriptions.Item label="当前状态">
                <Tag color={STATUS_MAP[transitionInvoice.status]?.color}>
                  {STATUS_MAP[transitionInvoice.status]?.label}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="金额">¥{transitionInvoice.total.toFixed(2)}</Descriptions.Item>
            </Descriptions>
            <Divider />
            <Text strong>选择目标状态：</Text>
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
        message.success("客户已更新");
      } else {
        await invoke("opc_create_customer", { input: payload });
        message.success("客户创建成功");
      }
      setModalOpen(false);
      setEditing(null);
      form.resetFields();
      load();
    } catch (e) {
      message.error(`操作失败: ${e}`);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("opc_delete_customer", { id });
      message.success("客户已删除");
      load();
    } catch (e) {
      message.error(`删除失败: ${e}`);
    }
  };

  const columns = [
    { title: "名称", dataIndex: "name", key: "name" },
    { title: "邮箱", dataIndex: "email", key: "email" },
    { title: "公司", dataIndex: "company", key: "company", render: (v: string | null) => v || "-" },
    {
      title: "状态",
      key: "status",
      render: (_: unknown, r: Customer) => {
        const s = CUST_STATUS[r.status] || { color: "default", label: r.status };
        return <Tag color={s.color}>{s.label}</Tag>;
      },
    },
    {
      title: "来源",
      key: "source",
      render: (_: unknown, r: Customer) => r.source ? (SOURCE_MAP[r.source] || r.source) : "-",
    },
    {
      title: "累计消费",
      key: "revenue",
      render: (_: unknown, r: Customer) => `¥${r.total_revenue.toFixed(2)}`,
      sorter: (a: Customer, b: Customer) => a.total_revenue - b.total_revenue,
    },
    { title: "发票数", dataIndex: "invoice_count", key: "count", width: 80 },
    {
      title: "操作",
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
          <Popconfirm title="确认删除此客户？" onConfirm={() => handleDelete(r.id)}>
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
            新建客户
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
        title={editing ? "编辑客户" : "新建客户"}
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          setEditing(null);
          form.resetFields();
        }}
        okText={editing ? "更新" : "创建"}
        cancelText="取消"
      >
        <Form form={form} layout="vertical" onFinish={handleSave}>
          <Form.Item name="name" label="姓名" rules={[{ required: true, message: "请输入姓名" }]}>
            <Input />
          </Form.Item>
          <Form.Item name="email" label="邮箱" rules={[{ required: true, type: "email", message: "请输入有效邮箱" }]}>
            <Input />
          </Form.Item>
          <Form.Item name="phone" label="电话">
            <Input />
          </Form.Item>
          <Form.Item name="company" label="公司">
            <Input />
          </Form.Item>
          <Form.Item name="source" label="来源">
            <Select
              allowClear
              placeholder="选择来源"
              options={[
                { value: "Referral", label: "推荐" },
                { value: "Website", label: "网站" },
                { value: "SocialMedia", label: "社交媒体" },
                { value: "Marketplace", label: "市场" },
                { value: "Direct", label: "直接" },
              ]}
            />
          </Form.Item>
          <Form.Item name="notes" label="备注">
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
        message.success("项目已更新");
      } else {
        await invoke("opc_create_project", { input: payload });
        message.success("项目创建成功");
      }
      setModalOpen(false);
      setEditing(null);
      form.resetFields();
      load();
    } catch (e) {
      message.error(`操作失败: ${e}`);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("opc_delete_project", { id });
      message.success("项目已删除");
      load();
    } catch (e) {
      message.error(`删除失败: ${e}`);
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
      message.success("里程碑已添加");
      setMilestoneOpen(false);
      milestoneForm.resetFields();
      // 刷新项目详情
      const updated = await invoke<Project>("opc_get_project", { id: detailProject.id });
      setDetailProject(updated);
      load();
    } catch (e) {
      message.error(`添加失败: ${e}`);
    }
  };

  const handleCompleteMilestone = async (milestoneId: string) => {
    if (!detailProject) { return; }
    try {
      await invoke("opc_complete_milestone", { projectId: detailProject.id, milestoneId });
      message.success("里程碑已完成");
      const updated = await invoke<Project>("opc_get_project", { id: detailProject.id });
      setDetailProject(updated);
      load();
    } catch (e) {
      message.error(`操作失败: ${e}`);
    }
  };

  const columns = [
    { title: "项目名称", dataIndex: "title", key: "title" },
    {
      title: "状态",
      key: "status",
      render: (_: unknown, r: Project) => {
        const s = PROJ_STATUS[r.status] || { color: "default", label: r.status };
        return <Tag color={s.color}>{s.label}</Tag>;
      },
    },
    {
      title: "里程碑",
      key: "milestones",
      render: (_: unknown, r: Project) => {
        const done = r.milestones.filter((m) => m.status === "Completed").length;
        return r.milestones.length > 0 ? `${done}/${r.milestones.length}` : "-";
      },
    },
    {
      title: "预算",
      key: "budget",
      render: (_: unknown, r: Project) => r.budget ? `¥${r.budget.toFixed(2)}` : "-",
      sorter: (a: Project, b: Project) => (a.budget || 0) - (b.budget || 0),
    },
    {
      title: "截止日期",
      key: "deadline",
      render: (_: unknown, r: Project) => r.deadline ? new Date(r.deadline * 1000).toLocaleDateString() : "-",
    },
    {
      title: "操作",
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
            详情
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
          <Popconfirm title="确认删除此项目？" onConfirm={() => handleDelete(r.id)}>
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
            新建项目
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
        title={editing ? "编辑项目" : "新建项目"}
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          setEditing(null);
          form.resetFields();
        }}
        width={560}
        okText={editing ? "更新" : "创建"}
        cancelText="取消"
      >
        <Form form={form} layout="vertical" onFinish={handleSave}>
          <Form.Item name="title" label="项目名称" rules={[{ required: true, message: "请输入项目名称" }]}>
            <Input />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={3} />
          </Form.Item>
          <Form.Item name="customer_id" label="关联客户">
            <Select
              allowClear
              placeholder="选择客户（可选）"
              optionFilterProp="label"
              options={customers.map((c) => ({ value: c.id, label: `${c.name} (${c.email})` }))}
            />
          </Form.Item>
          <Form.Item name="budget" label="预算">
            <InputNumber min={0} precision={2} prefix="¥" style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="deadline" label="截止日期">
            <DatePicker style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="notes" label="备注">
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
              <Descriptions.Item label="状态">
                <Tag color={PROJ_STATUS[detailProject.status]?.color}>{PROJ_STATUS[detailProject.status]?.label}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label="描述">{detailProject.description || "-"}</Descriptions.Item>
              <Descriptions.Item label="预算">
                {detailProject.budget ? `¥${detailProject.budget.toFixed(2)}` : "-"}
              </Descriptions.Item>
              <Descriptions.Item label="截止日期">
                {detailProject.deadline ? new Date(detailProject.deadline * 1000).toLocaleDateString() : "-"}
              </Descriptions.Item>
            </Descriptions>
            <Divider />
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
              <Text strong>里程碑 ({detailProject.milestones.length})</Text>
              <Button
                size="small"
                icon={<PlusOutlined />}
                onClick={() => {
                  milestoneForm.resetFields();
                  setMilestoneOpen(true);
                }}
              >
                添加里程碑
              </Button>
            </div>
            {detailProject.milestones.length === 0
              ? <Empty description="暂无里程碑" />
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
                              完成
                            </Button>
                          )}
                        </div>
                        <div>
                          <Text type="secondary">{m.description}</Text>
                        </div>
                        <div>
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            {m.due_at ? `截止: ${new Date(m.due_at * 1000).toLocaleDateString()}` : ""}
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
        title="添加里程碑"
        open={milestoneOpen}
        onOk={() => milestoneForm.submit()}
        onCancel={() => {
          setMilestoneOpen(false);
          milestoneForm.resetFields();
        }}
        okText="添加"
        cancelText="取消"
      >
        <Form form={milestoneForm} layout="vertical" onFinish={handleAddMilestone}>
          <Form.Item name="title" label="标题" rules={[{ required: true, message: "请输入里程碑标题" }]}>
            <Input />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={2} />
          </Form.Item>
          <Form.Item name="due_at" label="截止日期">
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
  const [subTab, setSubTab] = useState("landing");
  return (
    <Tabs
      activeKey={subTab}
      onChange={setSubTab}
      size="small"
      items={[
        { key: "landing", label: "落地页", children: <_LandingPagesPanel /> },
        { key: "blog", label: "博客", children: <_BlogPostsPanel /> },
        { key: "contacts", label: "联系表单", children: <_ContactsPanel /> },
      ]}
    />
  );
}

function _LandingPagesPanel() {
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
      message.success("落地页已创建");
      setModalOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(`创建失败: ${e}`);
    }
  };

  const handlePublish = async (id: string) => {
    try {
      await invoke("opc_publish_landing_page", { id });
      message.success("已发布");
      load();
    } catch (e) {
      message.error(`发布失败: ${e}`);
    }
  };

  const columns = [
    { title: "标题", dataIndex: "title", key: "title" },
    { title: "Slug", dataIndex: "slug", key: "slug" },
    {
      title: "状态",
      key: "status",
      render: (_: unknown, r: _LandingPage) => r.published ? <Tag color="green">已发布</Tag> : <Tag>草稿</Tag>,
    },
    {
      title: "创建时间",
      key: "created",
      render: (_: unknown, r: _LandingPage) => new Date(r.created_at * 1000).toLocaleDateString(),
    },
    {
      title: "操作",
      key: "actions",
      width: 100,
      render: (_: unknown, r: _LandingPage) =>
        !r.published && <Button size="small" onClick={() => handlePublish(r.id)}>发布</Button>,
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
          新建落地页
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
        title="新建落地页"
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          form.resetFields();
        }}
        okText="创建"
        cancelText="取消"
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="title" label="标题" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="slug" label="Slug" rules={[{ required: true }]}>
            <Input placeholder="my-page" />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  );
}

function _BlogPostsPanel() {
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
      message.success("文章已创建");
      setModalOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(`创建失败: ${e}`);
    }
  };

  const handlePublish = async (id: string) => {
    try {
      await invoke("opc_publish_blog_post", { id });
      message.success("已发布");
      load();
    } catch (e) {
      message.error(`发布失败: ${e}`);
    }
  };

  const columns = [
    { title: "标题", dataIndex: "title", key: "title" },
    { title: "Slug", dataIndex: "slug", key: "slug" },
    {
      title: "状态",
      key: "status",
      render: (_: unknown, r: _BlogPost) => r.published ? <Tag color="green">已发布</Tag> : <Tag>草稿</Tag>,
    },
    { title: "阅读", dataIndex: "view_count", key: "views", width: 60 },
    { title: "标签", key: "tags", render: (_: unknown, r: _BlogPost) => r.tags.map((t) => <Tag key={t}>{t}</Tag>) },
    {
      title: "操作",
      key: "actions",
      width: 100,
      render: (_: unknown, r: _BlogPost) =>
        !r.published && <Button size="small" onClick={() => handlePublish(r.id)}>发布</Button>,
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
          新建文章
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
        title="新建文章"
        open={modalOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setModalOpen(false);
          form.resetFields();
        }}
        okText="创建"
        cancelText="取消"
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="title" label="标题" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="slug" label="Slug" rules={[{ required: true }]}>
            <Input placeholder="my-article" />
          </Form.Item>
          <Form.Item name="excerpt" label="摘要">
            <Input.TextArea rows={2} />
          </Form.Item>
          <Form.Item name="tags" label="标签（逗号分隔）">
            <Input placeholder="tech, rust, opc" />
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  );
}

function _ContactsPanel() {
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
      message.error(`操作失败: ${e}`);
    }
  };

  const columns = [
    { title: "姓名", dataIndex: "name", key: "name" },
    { title: "邮箱", dataIndex: "email", key: "email" },
    { title: "消息", dataIndex: "message", key: "message", ellipsis: true, width: 300 },
    { title: "来源", dataIndex: "source", key: "source" },
    {
      title: "状态",
      key: "status",
      render: (_: unknown, r: _ContactSubmission) => r.read ? <Tag>已读</Tag> : <Tag color="orange">未读</Tag>,
    },
    {
      title: "时间",
      key: "created",
      render: (_: unknown, r: _ContactSubmission) => new Date(r.created_at * 1000).toLocaleString(),
    },
    {
      title: "操作",
      key: "actions",
      width: 80,
      render: (_: unknown, r: _ContactSubmission) =>
        !r.read && <Button size="small" onClick={() => handleMarkRead(r.id)}>标已读</Button>,
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

// ══════════════════════════════════════════════════════════════════
// 可视化办公室
// ══════════════════════════════════════════════════════════════════

interface OfficeWorker {
  id: string;
  role: string;
  name: string;
  icon: string;
  color: string;
  status: "working" | "idle" | "busy" | "offline";
  currentTask: string;
  completedToday: number;
  stats: { label: string; value: string }[];
}

const OFFICE_WORKERS: OfficeWorker[] = [
  {
    id: "financial-clerk",
    role: "opc_financial_clerk",
    name: "财务专员",
    icon: "💰",
    color: "#52c41a",
    status: "working",
    currentTask: "处理待开发票",
    completedToday: 3,
    stats: [{ label: "本月开票", value: "12 张" }, { label: "催款", value: "2 笔" }],
  },
  {
    id: "operations-manager",
    role: "opc_operations_manager",
    name: "运营经理",
    icon: "📋",
    color: "#1890ff",
    status: "working",
    currentTask: "项目里程碑检查",
    completedToday: 2,
    stats: [{ label: "活跃项目", value: "5 个" }, { label: "里程碑", value: "8 个" }],
  },
  {
    id: "sales-rep",
    role: "opc_sales_rep",
    name: "销售代表",
    icon: "🤝",
    color: "#722ed1",
    status: "idle",
    currentTask: "等待新线索分配",
    completedToday: 1,
    stats: [{ label: "本月客户", value: "3 位" }, { label: "线索", value: "2 条" }],
  },
  {
    id: "business-analyst",
    role: "opc_business_analyst",
    name: "业务分析师",
    icon: "📊",
    color: "#fa8c16",
    status: "busy",
    currentTask: "生成季度运营报告",
    completedToday: 0,
    stats: [{ label: "KPI 数", value: "6 个" }, { label: "报告", value: "2 份" }],
  },
];

function OfficeTab() {
  const [hovered, setHovered] = useState<string | null>(null);

  const statusLabel: Record<string, { color: string; label: string }> = {
    working: { color: "#52c41a", label: "工作中" },
    idle: { color: "#faad14", label: "待命中" },
    busy: { color: "#f5222d", label: "忙碌" },
    offline: { color: "#d9d9d9", label: "离线" },
  };

  return (
    <div>
      {/* 办公室布局 */}
      <div
        style={{
          background: "linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)",
          borderRadius: 12,
          padding: 24,
          minHeight: 420,
          position: "relative",
          overflow: "hidden",
        }}
      >
        {/* 地板网格 */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            backgroundImage:
              "linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px)",
            backgroundSize: "40px 40px",
            opacity: 0.5,
          }}
        />
        {/* 部门区域 */}
        <div style={{ position: "relative", zIndex: 1 }}>
          <Text style={{ color: "rgba(255,255,255,0.5)", fontSize: 12, letterSpacing: 2, textTransform: "uppercase" }}>
            OPC 办公区 · {new Date().toLocaleDateString("zh-CN", {
              weekday: "long",
              year: "numeric",
              month: "long",
              day: "numeric",
            })}
          </Text>
          <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
            {OFFICE_WORKERS.map((worker) => (
              <Col span={6} key={worker.id}>
                <Card
                  size="small"
                  hoverable
                  onMouseEnter={() => setHovered(worker.id)}
                  onMouseLeave={() => setHovered(null)}
                  style={{
                    background: hovered === worker.id ? "rgba(255,255,255,0.12)" : "rgba(255,255,255,0.06)",
                    border: `1px solid ${worker.color}40`,
                    borderRadius: 12,
                    backdropFilter: "blur(8px)",
                    transition: "all 0.3s",
                    transform: hovered === worker.id ? "translateY(-4px)" : "none",
                    cursor: "pointer",
                  }}
                >
                  {/* 工位头部 */}
                  <div style={{ textAlign: "center", marginBottom: 12 }}>
                    {/* 桌面图标 */}
                    <div
                      style={{
                        width: 48,
                        height: 48,
                        borderRadius: 12,
                        background: `${worker.color}20`,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        fontSize: 24,
                        margin: "0 auto 8px",
                        border: `2px solid ${worker.color}`,
                      }}
                    >
                      {worker.icon}
                    </div>
                    {/* 状态指示灯 */}
                    <div
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        padding: "2px 8px",
                        borderRadius: 10,
                        background: `${statusLabel[worker.status].color}20`,
                        fontSize: 11,
                      }}
                    >
                      <div
                        style={{
                          width: 6,
                          height: 6,
                          borderRadius: "50%",
                          background: statusLabel[worker.status].color,
                          animation: worker.status === "working" ? "pulse 2s infinite" : "none",
                        } as React.CSSProperties}
                      />
                      <span style={{ color: statusLabel[worker.status].color }}>
                        {statusLabel[worker.status].label}
                      </span>
                    </div>
                  </div>
                  {/* 姓名 */}
                  <div style={{ textAlign: "center", color: "#fff", fontWeight: 600, fontSize: 14, marginBottom: 2 }}>
                    {worker.name}
                  </div>
                  {/* 当前任务 */}
                  <div
                    style={{
                      textAlign: "center",
                      fontSize: 11,
                      color: "rgba(255,255,255,0.5)",
                      marginBottom: 8,
                    }}
                  >
                    {worker.currentTask}
                  </div>
                  {/* 统计 */}
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-around",
                      borderTop: "1px solid rgba(255,255,255,0.08)",
                      paddingTop: 8,
                    }}
                  >
                    {worker.stats.map((s, i) => (
                      <div key={i} style={{ textAlign: "center" }}>
                        <div style={{ fontSize: 13, fontWeight: 600, color: "#fff" }}>{s.value}</div>
                        <div style={{ fontSize: 10, color: "rgba(255,255,255,0.4)" }}>{s.label}</div>
                      </div>
                    ))}
                  </div>
                </Card>
              </Col>
            ))}
          </Row>
        </div>
      </div>

      {/* 底部快速操作 */}
      <Row gutter={16} style={{ marginTop: 16 }}>
        <Col span={12}>
          <Card size="small" title="团队动态" style={{ fontSize: 13 }}>
            <Timeline
              items={[
                { color: "green", children: "财务专员 创建了一份发票 (INV-20260717-0003)" },
                { color: "blue", children: "运营经理 完成了项目里程碑 #2" },
                { color: "orange", children: "业务分析师 记录了 KPI：月收入 ¥25,000" },
                { color: "default", children: "销售代表 添加了新客户：张三" },
              ]}
            />
          </Card>
        </Col>
        <Col span={12}>
          <Card size="small" title="快捷操作" style={{ fontSize: 13 }}>
            <Space direction="vertical" style={{ width: "100%" }}>
              <Button
                type="primary"
                block
                ghost
                icon={<DollarOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "invoices" }))}
              >
                去发票模块 — 财务专员
              </Button>
              <Button
                block
                ghost
                icon={<TeamOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "customers" }))}
              >
                去客户模块 — 销售代表
              </Button>
              <Button
                block
                ghost
                icon={<ProjectOutlined />}
                onClick={() => window.dispatchEvent(new CustomEvent("opc-switch-tab", { detail: "projects" }))}
              >
                去项目模块 — 运营经理
              </Button>
            </Space>
          </Card>
        </Col>
      </Row>
    </div>
  );
}

// ══════════════════════════════════════════════════════════════════
// 人才市场
// ══════════════════════════════════════════════════════════════════

interface TalentRole {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: string;
}

const TALENT_CATEGORIES: Record<string, { label: string; icon: string }> = {
  engineering: { label: "工程开发", icon: "💻" },
  design: { label: "设计", icon: "🎨" },
  finance: { label: "金融", icon: "💰" },
  marketing: { label: "营销", icon: "📢" },
  sales: { label: "销售", icon: "🤝" },
  product: { label: "产品", icon: "📋" },
  security: { label: "安全", icon: "🔒" },
  data: { label: "数据", icon: "📊" },
  devops: { label: "运维", icon: "🚀" },
  testing: { label: "测试", icon: "🧪" },
  support: { label: "支持", icon: "🎧" },
  academic: { label: "学术", icon: "🎓" },
};

function TalentMarketTab() {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [importedIds, setImportedIds] = useState<Set<string>>(new Set());
  const [importing, setImporting] = useState<string | null>(null);
  const [loading] = useState(true);

  // 预置人才列表（扫描 agency-agents 的 fallback）
  const allRoles: TalentRole[] = [
    {
      id: "ai-engineer",
      name: "AI 工程师",
      description: "AI/LLM 应用开发、模型微调、RAG 系统",
      category: "engineering",
      icon: "🤖",
    },
    {
      id: "backend-architect",
      name: "后端架构师",
      description: "系统架构、API 设计、数据库优化",
      category: "engineering",
      icon: "🏗️",
    },
    {
      id: "frontend-developer",
      name: "前端开发",
      description: "React/Vue 前端、UI 实现",
      category: "engineering",
      icon: "🖥️",
    },
    {
      id: "devops-engineer",
      name: "DevOps",
      description: "CI/CD、容器化、云基础设施",
      category: "engineering",
      icon: "🚀",
    },
    { id: "code-reviewer", name: "代码审查官", description: "代码质量、安全审计", category: "engineering", icon: "👀" },
    { id: "financial-analyst", name: "金融分析师", description: "财务报表、投资评估", category: "finance", icon: "📈" },
    { id: "accountant", name: "会计师", description: "记账、税务、合规", category: "finance", icon: "🧾" },
    { id: "security-expert", name: "安全专家", description: "渗透测试、安全加固", category: "security", icon: "🛡️" },
    { id: "data-scientist", name: "数据科学家", description: "数据分析、机器学习", category: "data", icon: "📊" },
    { id: "seo-specialist", name: "SEO 专家", description: "SEO、内容策略", category: "marketing", icon: "🔍" },
    { id: "sales-engineer", name: "销售工程师", description: "方案演示、客户关系", category: "sales", icon: "🤝" },
    { id: "product-manager", name: "产品经理", description: "需求分析、PRD", category: "product", icon: "📋" },
    { id: "ux-designer", name: "UX 设计师", description: "交互设计、用户研究", category: "design", icon: "🎨" },
    { id: "qa-engineer", name: "QA 工程师", description: "自动化测试、集成测试", category: "testing", icon: "🧪" },
    { id: "tech-support", name: "技术支持", description: "客户支持、故障排查", category: "support", icon: "🎧" },
  ];

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
      message.success(`已招聘: ${allRoles.find((r) => r.id === roleId)?.name}`);
      setImportedIds((prev) => new Set(prev).add(roleId));
    } catch (e) {
      message.error(`招聘失败: ${e}`);
    } finally {
      setImporting(null);
    }
  };

  const filtered = allRoles.filter((r) => {
    if (category && r.category !== category) { return false; }
    if (search && !r.name.includes(search) && !r.description.includes(search)) { return false; }
    return true;
  });

  const categories = [...new Set(allRoles.map((r) => r.category))].sort();

  return (
    <div>
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={8}>
          <Input.Search
            placeholder="搜索人才..."
            allowClear
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </Col>
        <Col span={16}>
          <Space wrap>
            <Button size="small" type={category === null ? "primary" : "default"} onClick={() => setCategory(null)}>
              全部
            </Button>
            {categories.map((cat) => (
              <Button
                key={cat}
                size="small"
                type={category === cat ? "primary" : "default"}
                onClick={() => setCategory(cat)}
              >
                {TALENT_CATEGORIES[cat]?.icon} {TALENT_CATEGORIES[cat]?.label}
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
                <Empty description="未找到匹配的人才" />
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
                          ? <Tag color="green">✅ 已入职</Tag>
                          : (
                            <Button
                              type="primary"
                              size="small"
                              loading={importing === role.id}
                              onClick={() => handleHire(role.id)}
                            >
                              招聘
                            </Button>
                          ),
                      ]}
                    >
                      <Card.Meta
                        avatar={<div style={{ fontSize: 28 }}>{role.icon}</div>}
                        title={<span style={{ fontSize: 13 }}>{role.name}</span>}
                        description={
                          <div>
                            <Tag>
                              {TALENT_CATEGORIES[role.category]?.icon} {TALENT_CATEGORIES[role.category]?.label}
                            </Tag>
                            <div style={{ fontSize: 12, color: "rgba(255,255,255,0.6)", marginTop: 4 }}>
                              {role.description}
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

const KANBAN_COLUMNS = ["待办", "进行中", "阻塞", "评审", "已完成", "终止"];

function KanbanTab() {
  const [board, setBoard] = useState<KanbanBoard>({});
  const [loading, setLoading] = useState(false);
  const [acting, setActing] = useState<string | null>(null);

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
      message.success(`操作成功: ${cmd}`);
      refresh();
    } catch (e) {
      message.error(`${cmd} 失败: ${e}`);
    } finally {
      setActing(null);
    }
  };

  return (
    <div>
      <Space style={{ marginBottom: 12 }}>
        <Button size="small" type="primary" icon={<ProjectOutlined />} onClick={refresh} loading={loading}>
          刷新
        </Button>
        <Typography.Text type="secondary">
          WorkItem 状态机：QUEUED → IN_PROGRESS ⇄ BLOCKED → REVIEW → APPROVED → DONE
        </Typography.Text>
      </Space>
      <Row gutter={[12, 12]}>
        {KANBAN_COLUMNS.map((col) => {
          const items = board[col] ?? [];
          return (
            <Col key={col} span={4}>
              <Card
                size="small"
                title={
                  <Space>
                    {col}
                    <Tag color={col === "阻塞" ? "red" : col === "已完成" ? "green" : "blue"}>
                      {items.length}
                    </Tag>
                  </Space>
                }
                style={{ minHeight: 200, background: col === "阻塞" ? "#fff2f0" : undefined }}
              >
                {items.length === 0
                  ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="空" />
                  : (
                    <Space direction="vertical" style={{ width: "100%" }}>
                      {items.map((it) => (
                        <Card key={it.id} size="small" styles={{ body: { padding: 8 } }}>
                          <Typography.Text strong style={{ fontSize: 12 }}>
                            {it.title}
                          </Typography.Text>
                          <div style={{ fontSize: 11, color: "#888", marginTop: 4 }}>
                            <div>ID: {it.id}</div>
                            <div>负责人: {it.owner_role_id ?? "-"}</div>
                            {it.deps.length > 0 && <div>依赖: {it.deps.join(", ")}</div>}
                            {it.last_error && <div style={{ color: "#cf1322" }}>⚠ {it.last_error}</div>}
                          </div>
                          <Space wrap style={{ marginTop: 6 }}>
                            {it.phase === "QUEUED" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_start")}
                              >
                                认领
                              </Button>
                            )}
                            {it.phase === "IN_PROGRESS" && (
                              <Button
                                size="small"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_review")}
                              >
                                提交评审
                              </Button>
                            )}
                            {it.phase === "REVIEW" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_start")}
                              >
                                批准完成
                              </Button>
                            )}
                            {it.phase !== "BLOCKED" && it.phase !== "DONE" && it.phase !== "APPROVED"
                              && it.phase !== "FAILED" && it.phase !== "CANCELLED" && (
                              <Button
                                size="small"
                                danger
                                loading={acting === it.id}
                                onClick={() => {
                                  const reason = window.prompt("升级原因：", "需要人工介入");
                                  if (reason !== null) { act(it.id, "opc_escalate_work_item", { reason }); }
                                }}
                              >
                                升级
                              </Button>
                            )}
                            {it.phase === "BLOCKED" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_unblock")}
                              >
                                解除阻塞
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
          刷新市场
        </Button>
        <Typography.Text type="secondary">
          行业数据资产包市场：每个行业独立安装/启用，安装后 seed 对应工作流模板
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
                    {p.installed ? "已安装" : "未安装"}
                  </Tag>
                </Space>
              }
            >
              <div style={{ fontSize: 12, color: "#888" }}>
                <div>ID: {p.id}</div>
                <div>版本: v{p.version}</div>
                <div>启用: {p.enabled ? "是" : "否"}</div>
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
                      message.success(`已安装: ${p.name}`);
                      refresh();
                    } catch (e) {
                      message.error(`安装失败: ${e}`);
                    }
                  }}
                >
                  安装
                </Button>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>
    </div>
  );
}
