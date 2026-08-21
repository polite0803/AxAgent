// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { DeleteOutlined, EditOutlined, PlusOutlined, ScheduleOutlined, SearchOutlined } from "@ant-design/icons";
import {
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  message,
  Modal,
  Row,
  Select,
  Space,
  Steps,
  Table,
  Tabs,
  Tag,
} from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ContentAsset {
  id: string;
  title: string;
  content_type: string;
  body: string;
  tags_json: string;
  status: string;
  created_at: number;
  updated_at: number;
}

interface PublishSchedule {
  id: string;
  content_ref_type: string;
  content_ref_id: string;
  scheduled_at: number;
  status: string;
  published_at: number | null;
  created_at: number;
  updated_at: number;
}

const CONTENT_TYPES = ["article", "video", "image", "social_post", "newsletter"] as const;

const STATUS_COLORS: Record<string, string> = {
  draft: "default",
  published: "green",
  pending: "orange",
  failed: "red",
};

export function ContentMediaTab() {
  const { t } = useTranslation();
  const [subTab, setSubTab] = useState("assets");
  return (
    <Tabs
      activeKey={subTab}
      onChange={setSubTab}
      size="small"
      items={[
        { key: "assets", label: t("opc.contentMedia.tabAssets"), children: <ContentAssetsPanel /> },
        { key: "schedules", label: t("opc.contentMedia.tabSchedules"), children: <PublishSchedulesPanel /> },
        { key: "wizard", label: t("opc.contentMedia.tabWizard"), children: <ContentCreationWizard /> },
      ]}
    />
  );
}

function ContentAssetsPanel() {
  const { t } = useTranslation();
  const [assets, setAssets] = useState<ContentAsset[]>([]);
  const [loading, setLoading] = useState(true);
  const [createOpen, setCreateOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [editingAsset, setEditingAsset] = useState<ContentAsset | null>(null);
  const [searchText, setSearchText] = useState("");
  const [form] = Form.useForm();

  const load = () => {
    setLoading(true);
    (async () => {
      try {
        const data = await invoke<ContentAsset[]>("opc_list_content_assets");
        setAssets(data);
      } catch (e) {
        message.error(t("opc.common.loadFailed", { error: String(e) }));
        setAssets([]);
      } finally {
        setLoading(false);
      }
    })();
  };

  useEffect(() => {
    load();
  }, []);

  const handleCreate = async (values: Record<string, unknown>) => {
    try {
      await invoke("opc_create_content_asset", {
        input: {
          title: values.title,
          content_type: values.content_type,
          body: values.body,
          tags: values.tags ? String(values.tags).split(",").map((s) => s.trim()).filter(Boolean) : [],
          status: values.status || "draft",
        },
      });
      message.success(t("opc.contentMedia.assetCreated"));
      setCreateOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(t("opc.common.createFailed", { error: String(e) }));
    }
  };

  const handleUpdate = async (values: Record<string, unknown>) => {
    if (!editingAsset) { return; }
    try {
      await invoke("opc_update_content_asset", {
        id: editingAsset.id,
        input: {
          title: values.title,
          content_type: values.content_type,
          body: values.body,
          tags: values.tags ? String(values.tags).split(",").map((s) => s.trim()).filter(Boolean) : [],
          status: values.status,
        },
      });
      message.success(t("opc.contentMedia.assetUpdated"));
      setEditOpen(false);
      setEditingAsset(null);
      form.resetFields();
      load();
    } catch (e) {
      message.error(t("opc.common.updateFailed", { error: String(e) }));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("opc_delete_content_asset", { id });
      message.success(t("opc.contentMedia.assetDeleted"));
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const openEdit = (asset: ContentAsset) => {
    setEditingAsset(asset);
    form.setFieldsValue({
      title: asset.title,
      content_type: asset.content_type,
      body: asset.body,
      tags: asset.tags_json ? JSON.parse(asset.tags_json).join(",") : "",
      status: asset.status,
    });
    setEditOpen(true);
  };

  const filteredAssets = searchText
    ? assets.filter(
      (a) =>
        a.title.toLowerCase().includes(searchText.toLowerCase())
        || a.body.toLowerCase().includes(searchText.toLowerCase()),
    )
    : assets;

  const columns = [
    { title: t("opc.contentMedia.columnTitle"), dataIndex: "title", key: "title", width: 200 },
    {
      title: t("opc.contentMedia.columnType"),
      dataIndex: "content_type",
      key: "content_type",
      width: 100,
      render: (type: string) => t(`opc.contentMedia.type_${type}`),
    },
    {
      title: t("opc.contentMedia.columnStatus"),
      dataIndex: "status",
      key: "status",
      width: 90,
      render: (status: string) => (
        <Tag color={STATUS_COLORS[status] || "default"}>{t(`opc.contentMedia.status_${status}`)}</Tag>
      ),
    },
    {
      title: t("opc.contentMedia.columnTags"),
      key: "tags",
      width: 200,
      render: (_: unknown, r: ContentAsset) => {
        try {
          const tags: string[] = r.tags_json ? JSON.parse(r.tags_json) : [];
          return tags.map((tag) => <Tag key={tag}>{tag}</Tag>);
        } catch {
          return null;
        }
      },
    },
    {
      title: t("opc.contentMedia.columnCreated"),
      key: "created_at",
      width: 120,
      render: (_: unknown, r: ContentAsset) => new Date(r.created_at * 1000).toLocaleDateString(),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 120,
      render: (_: unknown, r: ContentAsset) => (
        <Space size="small">
          <Button size="small" icon={<EditOutlined />} onClick={() => openEdit(r)}>
            {t("opc.common.update")}
          </Button>
          <Button size="small" danger icon={<DeleteOutlined />} onClick={() => handleDelete(r.id)}>
            {t("opc.common.actions")}
          </Button>
        </Space>
      ),
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
            form.setFieldsValue({ status: "draft", content_type: "article" });
            setCreateOpen(true);
          }}
        >
          {t("opc.contentMedia.newAsset")}
        </Button>
      }
    >
      <Row gutter={12} style={{ marginBottom: 12 }} align="middle">
        <Col xs={24} sm={12} md={8}>
          <Input
            placeholder={t("opc.common.search")}
            prefix={<SearchOutlined />}
            allowClear
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
          />
        </Col>
      </Row>
      <Table
        dataSource={filteredAssets}
        columns={columns}
        rowKey="id"
        loading={loading}
        size="small"
        pagination={{ pageSize: 20 }}
        scroll={{ x: 900 }}
      />
      <Modal
        title={t("opc.contentMedia.assetModalTitle")}
        open={createOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setCreateOpen(false);
          form.resetFields();
        }}
        okText={t("opc.common.create")}
        cancelText={t("opc.common.cancel")}
        width={560}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="title" label={t("opc.contentMedia.columnTitle")} rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="content_type" label={t("opc.contentMedia.columnType")} rules={[{ required: true }]}>
            <Select
              options={CONTENT_TYPES.map((c) => ({ value: c, label: t(`opc.contentMedia.type_${c}`) }))}
            />
          </Form.Item>
          <Form.Item name="body" label={t("opc.contentMedia.columnBody")}>
            <Input.TextArea rows={4} />
          </Form.Item>
          <Form.Item name="tags" label={t("opc.contentMedia.columnTags")}>
            <Input placeholder={t("opc.contentMedia.tagsPlaceholder")} />
          </Form.Item>
          <Form.Item name="status" label={t("opc.contentMedia.columnStatus")}>
            <Select
              options={[
                { value: "draft", label: t("opc.contentMedia.status_draft") },
                { value: "published", label: t("opc.contentMedia.status_published") },
              ]}
            />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title={t("opc.contentMedia.assetEditTitle")}
        open={editOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setEditOpen(false);
          setEditingAsset(null);
          form.resetFields();
        }}
        okText={t("opc.common.update")}
        cancelText={t("opc.common.cancel")}
        width={560}
      >
        <Form form={form} layout="vertical" onFinish={handleUpdate}>
          <Form.Item name="title" label={t("opc.contentMedia.columnTitle")} rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="content_type" label={t("opc.contentMedia.columnType")} rules={[{ required: true }]}>
            <Select
              options={CONTENT_TYPES.map((c) => ({ value: c, label: t(`opc.contentMedia.type_${c}`) }))}
            />
          </Form.Item>
          <Form.Item name="body" label={t("opc.contentMedia.columnBody")}>
            <Input.TextArea rows={4} />
          </Form.Item>
          <Form.Item name="tags" label={t("opc.contentMedia.columnTags")}>
            <Input placeholder={t("opc.contentMedia.tagsPlaceholder")} />
          </Form.Item>
          <Form.Item name="status" label={t("opc.contentMedia.columnStatus")}>
            <Select
              options={[
                { value: "draft", label: t("opc.contentMedia.status_draft") },
                { value: "published", label: t("opc.contentMedia.status_published") },
              ]}
            />
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  );
}

function PublishSchedulesPanel() {
  const { t } = useTranslation();
  const [schedules, setSchedules] = useState<PublishSchedule[]>([]);
  const [loading, setLoading] = useState(true);
  const [createOpen, setCreateOpen] = useState(false);
  const [assets, setAssets] = useState<ContentAsset[]>([]);
  const [form] = Form.useForm();

  const load = () => {
    setLoading(true);
    (async () => {
      try {
        const data = await invoke<PublishSchedule[]>("opc_list_publish_schedules");
        setSchedules(data);
      } catch (e) {
        message.error(t("opc.common.loadFailed", { error: String(e) }));
        setSchedules([]);
      } finally {
        setLoading(false);
      }
    })();
  };

  const loadAssets = async () => {
    try {
      const data = await invoke<ContentAsset[]>("opc_list_content_assets");
      setAssets(data);
    } catch (e) {
      message.error(t("opc.common.loadFailed", { error: String(e) }));
      setAssets([]);
    }
  };

  useEffect(() => {
    load();
    loadAssets();
  }, []);

  const handleCreate = async (values: Record<string, unknown>) => {
    try {
      await invoke("opc_create_publish_schedule", {
        input: {
          content_ref_type: "content_asset",
          content_ref_id: values.content_ref_id,
          scheduled_at: Math.floor((values.scheduled_at as Date).getTime() / 1000),
        },
      });
      message.success(t("opc.contentMedia.scheduleCreated"));
      setCreateOpen(false);
      form.resetFields();
      load();
    } catch (e) {
      message.error(t("opc.common.createFailed", { error: String(e) }));
    }
  };

  const handleCancel = async (id: string) => {
    try {
      await invoke("opc_delete_publish_schedule", { id });
      message.success(t("opc.contentMedia.scheduleCancelled"));
      load();
    } catch (e) {
      message.error(t("opc.common.opFailed", { error: String(e) }));
    }
  };

  const handleProcessDue = async () => {
    try {
      await invoke("opc_process_due_schedules");
      message.success(t("opc.contentMedia.processDueSuccess"));
      load();
    } catch (e) {
      message.error(t("opc.common.opFailed", { error: String(e) }));
    }
  };

  const assetOptions = assets.map((a) => ({
    value: a.id,
    label: `${a.title} (${a.content_type})`,
  }));

  const columns = [
    {
      title: t("opc.contentMedia.columnContentRef"),
      dataIndex: "content_ref_id",
      key: "content_ref_id",
      render: (id: string) => {
        const asset = assets.find((a) => a.id === id);
        return asset ? asset.title : id;
      },
    },
    {
      title: t("opc.contentMedia.columnRefType"),
      dataIndex: "content_ref_type",
      key: "content_ref_type",
      width: 120,
    },
    {
      title: t("opc.contentMedia.columnScheduledAt"),
      dataIndex: "scheduled_at",
      key: "scheduled_at",
      width: 160,
      render: (ts: number) => new Date(ts * 1000).toLocaleString(),
    },
    {
      title: t("opc.contentMedia.columnStatus"),
      dataIndex: "status",
      key: "status",
      width: 100,
      render: (status: string) => (
        <Tag color={STATUS_COLORS[status] || "default"}>{t(`opc.contentMedia.schedule_${status}`)}</Tag>
      ),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 100,
      render: (_: unknown, r: PublishSchedule) =>
        r.status === "pending" && (
          <Button size="small" danger onClick={() => handleCancel(r.id)}>
            {t("opc.contentMedia.cancelSchedule")}
          </Button>
        ),
    },
  ];

  return (
    <Card
      extra={
        <Space>
          <Button icon={<ScheduleOutlined />} onClick={handleProcessDue}>
            {t("opc.contentMedia.processDue")}
          </Button>
          <Button
            type="primary"
            size="small"
            icon={<PlusOutlined />}
            onClick={() => {
              form.resetFields();
              setCreateOpen(true);
            }}
          >
            {t("opc.contentMedia.newSchedule")}
          </Button>
        </Space>
      }
    >
      <Table
        dataSource={schedules}
        columns={columns}
        rowKey="id"
        loading={loading}
        size="small"
        pagination={{ pageSize: 20 }}
      />
      <Modal
        title={t("opc.contentMedia.scheduleModalTitle")}
        open={createOpen}
        onOk={() => form.submit()}
        onCancel={() => {
          setCreateOpen(false);
          form.resetFields();
        }}
        okText={t("opc.common.create")}
        cancelText={t("opc.common.cancel")}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item
            name="content_ref_id"
            label={t("opc.contentMedia.selectContent")}
            rules={[{ required: true }]}
          >
            <Select
              showSearch
              placeholder={t("opc.contentMedia.selectContentPlaceholder")}
              options={assetOptions}
              filterOption={(input, option) => String(option?.label ?? "").toLowerCase().includes(input.toLowerCase())}
            />
          </Form.Item>
          <Form.Item
            name="scheduled_at"
            label={t("opc.contentMedia.scheduledAt")}
            rules={[{ required: true }]}
          >
            <DatePicker showTime style={{ width: "100%" }} />
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  );
}

function ContentCreationWizard() {
  const { t } = useTranslation();
  const [current, setCurrent] = useState(0);
  const [wizardData, setWizardData] = useState({
    topic: "",
    angle: "",
    title: "",
    body: "",
    tags: "",
    meta_description: "",
    publish_type: "immediate" as "immediate" | "scheduled",
    scheduled_at: null as number | null,
    asset_id: "",
  });
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const steps = [
    { title: t("opc.contentMedia.wizard.step1Title"), desc: t("opc.contentMedia.wizard.step1Desc") },
    { title: t("opc.contentMedia.wizard.step2Title"), desc: t("opc.contentMedia.wizard.step2Desc") },
    { title: t("opc.contentMedia.wizard.step3Title"), desc: t("opc.contentMedia.wizard.step3Desc") },
    { title: t("opc.contentMedia.wizard.step4Title"), desc: t("opc.contentMedia.wizard.step4Desc") },
  ];

  const next = () => {
    form
      .validateFields()
      .then((values) => {
        setWizardData((prev) => ({ ...prev, ...values }));
        setCurrent((c) => c + 1);
      })
      .catch(() => {});
  };

  const prev = () => setCurrent((c) => c - 1);

  const handleFinish = async () => {
    setLoading(true);
    try {
      const assetId = wizardData.asset_id || crypto.randomUUID();
      await invoke("opc_create_content_asset", {
        input: {
          title: wizardData.title,
          content_type: "article",
          body: wizardData.body,
          tags: wizardData.tags ? wizardData.tags.split(",").map((s) => s.trim()).filter(Boolean) : [],
          status: "draft",
        },
      });

      if (wizardData.publish_type === "scheduled" && wizardData.scheduled_at) {
        await invoke("opc_create_publish_schedule", {
          input: {
            content_ref_type: "content_asset",
            content_ref_id: assetId,
            scheduled_at: wizardData.scheduled_at,
          },
        });
        message.success(t("opc.contentMedia.wizard.scheduleCreated"));
      } else {
        await invoke("opc_update_content_asset", {
          id: assetId,
          input: { status: "published" },
        });
        message.success(t("opc.contentMedia.wizard.publishedNow"));
      }

      message.success(t("opc.contentMedia.wizard.complete"));
      setCurrent(0);
      setWizardData({
        topic: "",
        angle: "",
        title: "",
        body: "",
        tags: "",
        meta_description: "",
        publish_type: "immediate",
        scheduled_at: null,
        asset_id: "",
      });
      form.resetFields();
    } catch (e) {
      message.error(t("opc.common.opFailed", { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card>
      <Steps current={current} items={steps} style={{ marginBottom: 32 }} />
      <div style={{ minHeight: 240 }}>
        {current === 0 && (
          <Form
            form={form}
            layout="vertical"
            initialValues={{ topic: wizardData.topic, angle: wizardData.angle }}
          >
            <Form.Item
              name="topic"
              label={t("opc.contentMedia.wizard.topicLabel")}
              rules={[{ required: true, message: t("opc.contentMedia.wizard.topicRequired") }]}
            >
              <Input placeholder={t("opc.contentMedia.wizard.topicPlaceholder")} />
            </Form.Item>
            <Form.Item name="angle" label={t("opc.contentMedia.wizard.angleLabel")}>
              <Input.TextArea rows={2} placeholder={t("opc.contentMedia.wizard.anglePlaceholder")} />
            </Form.Item>
          </Form>
        )}
        {current === 1 && (
          <Form
            form={form}
            layout="vertical"
            initialValues={{
              title: wizardData.title,
              body: wizardData.body,
              tags: wizardData.tags,
            }}
          >
            <Form.Item
              name="title"
              label={t("opc.contentMedia.wizard.titleLabel")}
              rules={[{ required: true, message: t("opc.contentMedia.wizard.titleRequired") }]}
            >
              <Input />
            </Form.Item>
            <Form.Item
              name="body"
              label={t("opc.contentMedia.wizard.bodyLabel")}
              rules={[{ required: true, message: t("opc.contentMedia.wizard.bodyRequired") }]}
            >
              <Input.TextArea rows={8} placeholder={t("opc.contentMedia.wizard.bodyPlaceholder")} />
            </Form.Item>
            <Form.Item name="tags" label={t("opc.contentMedia.columnTags")}>
              <Input placeholder={t("opc.contentMedia.tagsPlaceholder")} />
            </Form.Item>
          </Form>
        )}
        {current === 2 && (
          <Form
            form={form}
            layout="vertical"
            initialValues={{ meta_description: wizardData.meta_description }}
          >
            <Form.Item name="meta_description" label={t("opc.contentMedia.wizard.seoLabel")}>
              <Input.TextArea rows={3} placeholder={t("opc.contentMedia.wizard.seoPlaceholder")} />
            </Form.Item>
          </Form>
        )}
        {current === 3 && (
          <Form
            form={form}
            layout="vertical"
            initialValues={{
              publish_type: wizardData.publish_type,
              scheduled_at: wizardData.scheduled_at,
            }}
          >
            <Form.Item
              name="publish_type"
              label={t("opc.contentMedia.wizard.publishType")}
              rules={[{ required: true }]}
            >
              <Select
                options={[
                  { value: "immediate", label: t("opc.contentMedia.wizard.publishImmediate") },
                  { value: "scheduled", label: t("opc.contentMedia.wizard.publishScheduled") },
                ]}
              />
            </Form.Item>
            <Form.Item
              noStyle
              shouldUpdate={(prev, cur) => prev.publish_type !== cur.publish_type}
            >
              {({ getFieldValue }) =>
                getFieldValue("publish_type") === "scheduled"
                  ? (
                    <Form.Item
                      name="scheduled_at"
                      label={t("opc.contentMedia.wizard.scheduleTime")}
                      rules={[{ required: true, message: t("opc.contentMedia.wizard.scheduleRequired") }]}
                    >
                      <DatePicker showTime style={{ width: "100%" }} />
                    </Form.Item>
                  )
                  : null}
            </Form.Item>
          </Form>
        )}
      </div>
      <div style={{ textAlign: "right", marginTop: 24 }}>
        <Space>
          {current > 0 && <Button onClick={prev}>{t("opc.common.cancel")}</Button>}
          {current < steps.length - 1 && (
            <Button type="primary" onClick={next}>
              {t("opc.common.create")}
            </Button>
          )}
          {current === steps.length - 1 && (
            <Button type="primary" loading={loading} onClick={handleFinish}>
              {t("opc.contentMedia.wizard.finish")}
            </Button>
          )}
        </Space>
      </div>
    </Card>
  );
}
