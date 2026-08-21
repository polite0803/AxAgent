// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { DeleteOutlined, PlusOutlined, SwapOutlined } from "@ant-design/icons";
import {
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Divider,
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
  Typography,
} from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Customer, Invoice } from "../utils/constants";
import { getInvoiceStatusKey, STATUS_COLOR_MAP } from "../utils/constants";

const { Text } = Typography;

export function InvoicesTab() {
  const { t } = useTranslation();
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [transitionOpen, setTransitionOpen] = useState(false);
  const [transitionInvoice, setTransitionInvoice] = useState<Invoice | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>("");
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [form] = Form.useForm();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [inv, cust] = await Promise.all([
        invoke<Invoice[]>("opc_list_invoices", { filter: {} }),
        invoke<Customer[]>("opc_list_customers", { filter: {} }),
      ]);
      setInvoices(inv);
      setCustomers(cust);
    } catch (e) {
      message.error(t("opc.common.loadFailed", { error: String(e) }));
      setInvoices([]);
      setCustomers([]);
    } finally {
      setLoading(false);
    }
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
      draft: [
        { value: "sent", label: t("opc.invoice.actionSend") },
        { value: "cancelled", label: t("opc.invoice.actionCancel") },
      ],
      sent: [
        { value: "paid", label: t("opc.invoice.actionMarkPaid") },
        { value: "overdue", label: t("opc.invoice.actionMarkOverdue") },
        { value: "cancelled", label: t("opc.invoice.actionCancel") },
      ],
      overdue: [
        { value: "paid", label: t("opc.invoice.actionMarkPaid") },
        { value: "cancelled", label: t("opc.invoice.actionCancel") },
      ],
      paid: [{ value: "refunded", label: t("opc.invoice.actionRefund") }],
    };
    return map[status] || [];
  };

  const filteredInvoices = filterStatus
    ? invoices.filter((inv) => inv.status === filterStatus)
    : invoices;

  const handleBatchDelete = async () => {
    try {
      await Promise.all(selectedRowKeys.map((id) => invoke("opc_delete_invoice", { id })));
      message.success(t("opc.common.batchDeleteSuccess"));
      setSelectedRowKeys([]);
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const handleExportSelected = () => {
    const data = invoices.filter((inv) => selectedRowKeys.includes(inv.id));
    console.log("Exported invoices:", data);
    message.success(t("opc.common.exportSuccess", { count: data.length }));
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
      render: (_: unknown, r: Invoice) => (r.due_at ? new Date(r.due_at * 1000).toLocaleDateString() : "-"),
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
      {/* 筛选工具栏 */}
      <Card size="small" style={{ marginBottom: 12 }}>
        <Row gutter={12} align="middle">
          <Col xs={24} sm={8} md={6}>
            <Select
              placeholder={t("opc.invoice.filterStatus")}
              allowClear
              style={{ width: "100%" }}
              value={filterStatus || undefined}
              onChange={(val) => setFilterStatus(val || "")}
              options={[
                { value: "draft", label: t("opc.invoiceStatus.draft") },
                { value: "sent", label: t("opc.invoiceStatus.sent") },
                { value: "paid", label: t("opc.invoiceStatus.paid") },
                { value: "overdue", label: t("opc.invoiceStatus.overdue") },
                { value: "cancelled", label: t("opc.invoiceStatus.cancelled") },
              ]}
            />
          </Col>
          <Col>
            <Space>
              {selectedRowKeys.length > 0 && (
                <>
                  <Button danger size="small" onClick={handleBatchDelete}>
                    {t("opc.common.batchDelete", { count: selectedRowKeys.length })}
                  </Button>
                  <Button size="small" onClick={handleExportSelected}>
                    {t("opc.common.export")}
                  </Button>
                </>
              )}
              <Button
                size="small"
                onClick={() => {
                  setFilterStatus("");
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
              form.resetFields();
              setModalOpen(true);
            }}
          >
            {t("opc.invoice.newInvoice")}
          </Button>
        }
      >
        <Table
          dataSource={filteredInvoices}
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
