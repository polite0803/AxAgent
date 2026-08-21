// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { PlusOutlined, SearchOutlined } from "@ant-design/icons";
import { Button, Card, Col, Form, Input, message, Modal, Row, Space, Table, Tabs, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface LandingPage {
  id: string;
  title: string;
  slug: string;
  description: string;
  published: boolean;
  published_at: number | null;
  created_at: number;
}

interface BlogPost {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  tags: string[];
  published: boolean;
  view_count: number;
  created_at: number;
}

interface ContactSubmission {
  id: string;
  name: string;
  email: string;
  message: string;
  source: string;
  read: boolean;
  created_at: number;
}

export function SitesTab() {
  const { t } = useTranslation();
  const [subTab, setSubTab] = useState("landing");
  return (
    <Tabs
      activeKey={subTab}
      onChange={setSubTab}
      size="small"
      items={[
        { key: "landing", label: t("opc.site.tabLanding"), children: <LandingPagesPanel /> },
        { key: "blog", label: t("opc.site.tabBlog"), children: <BlogPostsPanel /> },
        { key: "contacts", label: t("opc.site.tabContacts"), children: <ContactsPanel /> },
      ]}
    />
  );
}

function LandingPagesPanel() {
  const { t } = useTranslation();
  const [pages, setPages] = useState<LandingPage[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [searchText, setSearchText] = useState("");
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [form] = Form.useForm();

  const load = () => {
    setLoading(true);
    (async () => {
      try {
        const data = await invoke<LandingPage[]>("opc_list_landing_pages");
        setPages(data);
      } catch (e) {
        message.error(t("opc.common.loadFailed", { error: String(e) }));
        setPages([]);
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

  const filteredPages = searchText
    ? pages.filter((p) =>
      p.title.toLowerCase().includes(searchText.toLowerCase())
      || p.slug.toLowerCase().includes(searchText.toLowerCase())
    )
    : pages;

  const handleBatchDelete = async () => {
    try {
      await Promise.all(selectedRowKeys.map((id) => invoke("opc_delete_landing_page", { id })));
      message.success(t("opc.common.batchDeleteSuccess"));
      setSelectedRowKeys([]);
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const columns = [
    { title: t("opc.site.columnTitle"), dataIndex: "title", key: "title" },
    { title: t("opc.site.columnSlug"), dataIndex: "slug", key: "slug" },
    {
      title: t("opc.site.columnStatus"),
      key: "status",
      render: (_: unknown, r: LandingPage) =>
        r.published ? <Tag color="green">{t("opc.site.published")}</Tag> : <Tag>{t("opc.site.draftTag")}</Tag>,
    },
    {
      title: t("opc.site.columnCreated"),
      key: "created",
      render: (_: unknown, r: LandingPage) => new Date(r.created_at * 1000).toLocaleDateString(),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 100,
      render: (_: unknown, r: LandingPage) =>
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
        <Col>
          <Space>
            {selectedRowKeys.length > 0 && (
              <Button danger size="small" onClick={handleBatchDelete}>
                {t("opc.common.batchDelete", { count: selectedRowKeys.length })}
              </Button>
            )}
          </Space>
        </Col>
      </Row>
      <Table
        dataSource={filteredPages}
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

function BlogPostsPanel() {
  const { t } = useTranslation();
  const [posts, setPosts] = useState<BlogPost[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [searchText, setSearchText] = useState("");
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [form] = Form.useForm();

  const load = () => {
    setLoading(true);
    (async () => {
      try {
        const data = await invoke<BlogPost[]>("opc_list_blog_posts");
        setPosts(data);
      } catch (e) {
        message.error(t("opc.common.loadFailed", { error: String(e) }));
        setPosts([]);
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
      await invoke("opc_create_blog_post", {
        input: {
          title: values.title,
          slug: values.slug,
          excerpt: (values.excerpt as string) || "",
          content: "",
          tags: values.tags ? (values.tags as string).split(",").map((s) => s.trim()).filter(Boolean) : [],
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

  const filteredPosts = searchText
    ? posts.filter((p) =>
      p.title.toLowerCase().includes(searchText.toLowerCase())
      || p.slug.toLowerCase().includes(searchText.toLowerCase())
    )
    : posts;

  const handleBatchDelete = async () => {
    try {
      await Promise.all(selectedRowKeys.map((id) => invoke("opc_delete_blog_post", { id })));
      message.success(t("opc.common.batchDeleteSuccess"));
      setSelectedRowKeys([]);
      load();
    } catch (e) {
      message.error(t("opc.common.deleteFailed", { error: String(e) }));
    }
  };

  const columns = [
    { title: t("opc.site.columnTitle"), dataIndex: "title", key: "title" },
    { title: t("opc.site.columnSlug"), dataIndex: "slug", key: "slug" },
    {
      title: t("opc.site.columnStatus"),
      key: "status",
      render: (_: unknown, r: BlogPost) =>
        r.published ? <Tag color="green">{t("opc.site.published")}</Tag> : <Tag>{t("opc.site.draftTag")}</Tag>,
    },
    { title: t("opc.site.columnViews"), dataIndex: "view_count", key: "views", width: 60 },
    {
      title: t("opc.site.columnTags"),
      key: "tags",
      render: (_: unknown, r: BlogPost) => r.tags.map((tag) => <Tag key={tag}>{tag}</Tag>),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 100,
      render: (_: unknown, r: BlogPost) =>
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
        <Col>
          <Space>
            {selectedRowKeys.length > 0 && (
              <Button danger size="small" onClick={handleBatchDelete}>
                {t("opc.common.batchDelete", { count: selectedRowKeys.length })}
              </Button>
            )}
          </Space>
        </Col>
      </Row>
      <Table
        dataSource={filteredPosts}
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

function ContactsPanel() {
  const { t } = useTranslation();
  const [contacts, setContacts] = useState<ContactSubmission[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchText, setSearchText] = useState("");
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);

  const load = () => {
    setLoading(true);
    (async () => {
      try {
        const data = await invoke<ContactSubmission[]>("opc_list_contacts");
        setContacts(data);
      } catch (e) {
        message.error(t("opc.common.loadFailed", { error: String(e) }));
        setContacts([]);
      } finally {
        setLoading(false);
      }
    })();
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

  const filteredContacts = searchText
    ? contacts.filter((c) =>
      c.name.toLowerCase().includes(searchText.toLowerCase())
      || c.email.toLowerCase().includes(searchText.toLowerCase())
    )
    : contacts;

  const handleBatchMarkRead = async () => {
    try {
      await Promise.all(selectedRowKeys.map((id) => invoke("opc_mark_contact_read", { id })));
      message.success(t("opc.common.batchSuccess"));
      setSelectedRowKeys([]);
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
      render: (_: unknown, r: ContactSubmission) =>
        r.read ? <Tag>{t("opc.site.readTag")}</Tag> : <Tag color="orange">{t("opc.site.unreadTag")}</Tag>,
    },
    {
      title: t("opc.site.contactColumnTime"),
      key: "created",
      render: (_: unknown, r: ContactSubmission) => new Date(r.created_at * 1000).toLocaleString(),
    },
    {
      title: t("opc.common.actions"),
      key: "actions",
      width: 80,
      render: (_: unknown, r: ContactSubmission) =>
        !r.read && <Button size="small" onClick={() => handleMarkRead(r.id)}>{t("opc.site.markRead")}</Button>,
    },
  ];

  return (
    <Card>
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
        <Col>
          <Space>
            {selectedRowKeys.length > 0 && (
              <Button size="small" onClick={handleBatchMarkRead}>
                {t("opc.common.batchMarkRead", { count: selectedRowKeys.length })}
              </Button>
            )}
          </Space>
        </Col>
      </Row>
      <Table
        dataSource={filteredContacts}
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
  );
}
