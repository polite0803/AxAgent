// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { DeleteOutlined, EditOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Card, Col, Form, Input, message, Modal, Popconfirm, Row, Select, Space, Table, Tag } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Customer } from "../utils/constants";
import { CUST_STATUS_COLOR_MAP, getCustomerStatusKey, getSourceKey } from "../utils/constants";

export function CustomersTab() {
  const { t } = useTranslation();
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<Customer | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>("");
  const [filterSource, setFilterSource] = useState<string>("");
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [form] = Form.useForm();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<Customer[]>("opc_list_customers", { filter: {} });
      setCustomers(data);
    } catch (e) {
      message.error(t("opc.common.loadFailed", { error: String(e) }));
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

  const filteredCustomers = customers.filter((c) => {
    if (filterStatus && c.status !== filterStatus) { return false; }
    if (filterSource && c.source !== filterSource) { return false; }
    return true;
  });

  const handleBatchDelete = async () => {
    try {
      await Promise.all(selectedRowKeys.map((id) => invoke("opc_delete_customer", { id })));
      message.success(t("opc.common.batchDeleteSuccess"));
      setSelectedRowKeys([]);
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
      render: (_: unknown, r: Customer) => (r.source ? t(getSourceKey(r.source)) : "-"),
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
      {/* 筛选工具栏 */}
      <Card size="small" style={{ marginBottom: 12 }}>
        <Row gutter={12} align="middle">
          <Col xs={24} sm={8} md={6}>
            <Select
              placeholder={t("opc.customer.filterStatus")}
              allowClear
              style={{ width: "100%" }}
              value={filterStatus || undefined}
              onChange={(val) => setFilterStatus(val || "")}
              options={[
                { value: "lead", label: t("opc.customerStatus.lead") },
                { value: "prospect", label: t("opc.customerStatus.prospect") },
                { value: "active", label: t("opc.customerStatus.active") },
                { value: "inactive", label: t("opc.customerStatus.inactive") },
                { value: "churned", label: t("opc.customerStatus.churned") },
              ]}
            />
          </Col>
          <Col xs={24} sm={8} md={6}>
            <Select
              placeholder={t("opc.customer.filterSource")}
              allowClear
              style={{ width: "100%" }}
              value={filterSource || undefined}
              onChange={(val) => setFilterSource(val || "")}
              options={[
                { value: "referral", label: t("opc.source.referral") },
                { value: "website", label: t("opc.source.website") },
                { value: "social_media", label: t("opc.source.social_media") },
                { value: "marketplace", label: t("opc.source.marketplace") },
                { value: "direct", label: t("opc.source.direct") },
              ]}
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
                  setFilterSource("");
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
            {t("opc.customer.newCustomer")}
          </Button>
        }
      >
        <Table
          dataSource={filteredCustomers}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={{ pageSize: 20 }}
          rowSelection={{
            selectedRowKeys,
            onChange: (keys) => setSelectedRowKeys(keys.map(String)),
          }}
          locale={{
            emptyText: loading ? t("opc.common.loading") : t("opc.customer.emptyTip"),
          }}
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
                { value: "referral", label: t("opc.source.referral") },
                { value: "website", label: t("opc.source.website") },
                { value: "social_media", label: t("opc.source.social_media") },
                { value: "marketplace", label: t("opc.source.marketplace") },
                { value: "direct", label: t("opc.source.direct") },
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
