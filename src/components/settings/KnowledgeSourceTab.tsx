// SPDX-License-Identifier: AGPL-3.0-only

import { useKnowledgeSourceStore } from "@/stores";
import type { CreateKnowledgeSourceInput, KnowledgeSource } from "@/types";
import { App as AntdApp, Button, Form, Input, Modal, Popconfirm, Select, Space, Table, Tag, Tooltip } from "antd";
import { ColumnsType } from "antd/es/table";
import { GitFork, Globe, Import, Play, RefreshCw, Rss, Trash2, Zap } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const SOURCE_TYPE_META: Record<string, { icon: React.ReactNode; color: string }> = {
  url: { icon: <Globe size={14} />, color: "blue" },
  rss: { icon: <Rss size={14} />, color: "orange" },
};

/**
 * 知识库增长更新入口（docs/knowledge-source-ingest-plan.md）
 * - URL 快速抓取：输入 URL → 生成 Wiki 页面 + RAG 索引
 * - 知识源管理：url/rss 源 CRUD + 手动抓取 + 批量抓取
 */
export function KnowledgeSourceTab() {
  const { t } = useTranslation();
  const { message } = AntdApp.useApp();
  const {
    sources,
    loading,
    loadSources,
    createSource,
    updateSource,
    deleteSource,
    fetchNow,
    fetchAll,
    fetchUrlToWiki,
    scheduleSync,
    githubRepoImport,
    sitemapCrawl,
  } = useKnowledgeSourceStore();

  const [createOpen, setCreateOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [editSource, setEditSource] = useState<KnowledgeSource | null>(null);
  const [editing, setEditing] = useState(false);
  const [fetchingAll, setFetchingAll] = useState(false);
  const [fetchingId, setFetchingId] = useState<string | null>(null);
  const [githubOpen, setGithubOpen] = useState(false);
  const [importingGithub, setImportingGithub] = useState(false);
  const [sitemapOpen, setSitemapOpen] = useState(false);
  const [importingSitemap, setImportingSitemap] = useState(false);
  const [scheduling, setScheduling] = useState(false);
  const [form] = Form.useForm<CreateKnowledgeSourceInput>();
  const [quickForm] = Form.useForm<{ url: string; title?: string }>();
  const [githubForm] = Form.useForm<{ repo: string; pathFilter?: string }>();
  const [sitemapForm] = Form.useForm<{ baseUrl: string }>();
  const [cronForm] = Form.useForm<{ cron: string }>();
  const [editForm] = Form.useForm<{ title: string; status: string; scheduleCron?: string }>();

  useEffect(() => {
    void loadSources();
  }, [loadSources]);

  const handleQuickFetch = async (values: { url: string; title?: string }) => {
    const result = await fetchUrlToWiki(values.url, values.title || undefined);
    if (!result) {
      message.error(t("sourceManager.knowledgeSource.fetchFailed"));
      return;
    }
    const actionLabel = result.action === "skipped"
      ? t("sourceManager.knowledgeSource.skipped")
      : result.action === "updated"
      ? t("sourceManager.knowledgeSource.updated")
      : t("sourceManager.knowledgeSource.created");
    message.success(`${result.title} — ${actionLabel}`);
    quickForm.resetFields();
    void loadSources();
  };

  const handleCreate = async (values: CreateKnowledgeSourceInput) => {
    setCreating(true);
    try {
      const created = await createSource(values);
      if (created) {
        message.success(t("sourceManager.knowledgeSource.createSuccess"));
        setCreateOpen(false);
        form.resetFields();
      } else {
        message.error(t("sourceManager.knowledgeSource.createFailed"));
      }
    } finally {
      setCreating(false);
    }
  };

  const handleFetchNow = async (id: string) => {
    setFetchingId(id);
    try {
      const result = await fetchNow(id);
      if (!result) {
        message.error(t("sourceManager.knowledgeSource.fetchFailed"));
      } else if (result.action === "error") {
        message.error(`${result.sourceTitle}: ${result.detail}`);
      } else {
        message.success(`${result.sourceTitle} — ${result.detail}`);
      }
    } finally {
      setFetchingId(null);
    }
  };

  const handleEdit = async (values: { title: string; status: string; scheduleCron?: string }) => {
    if (!editSource) {
      return;
    }
    setEditing(true);
    try {
      const updated = await updateSource({
        id: editSource.id,
        title: values.title,
        status: values.status,
        scheduleCron: values.scheduleCron || undefined,
        clearSchedule: values.scheduleCron === undefined || values.scheduleCron === "" ? true : undefined,
      });
      if (updated) {
        message.success(t("sourceManager.knowledgeSource.updateSuccess"));
        setEditSource(null);
      } else {
        message.error(t("sourceManager.knowledgeSource.updateFailed"));
      }
    } finally {
      setEditing(false);
    }
  };

  const handleFetchAll = async () => {
    setFetchingAll(true);
    try {
      const results = await fetchAll();
      const errors = results.filter((r) => r.action === "error");
      if (errors.length > 0) {
        message.warning(
          t("sourceManager.knowledgeSource.fetchAllPartial", {
            total: results.length,
            errors: errors.length,
          }),
        );
      } else {
        message.success(
          t("sourceManager.knowledgeSource.fetchAllDone", { total: results.length }),
        );
      }
    } finally {
      setFetchingAll(false);
    }
  };

  const handleGithubImport = async (values: { repo: string; pathFilter?: string }) => {
    setImportingGithub(true);
    try {
      const result = await githubRepoImport(values.repo, values.pathFilter || undefined);
      if (!result) {
        message.error(t("sourceManager.knowledgeSource.importFailed"));
      } else if (result.action === "error") {
        message.error(result.detail);
      } else {
        message.success(`${values.repo} — ${result.detail}`);
        setGithubOpen(false);
        githubForm.resetFields();
        void loadSources();
      }
    } finally {
      setImportingGithub(false);
    }
  };

  const handleSchedule = async (values: { cron: string }) => {
    setScheduling(true);
    try {
      const id = await scheduleSync(values.cron);
      if (id) {
        message.success(t("sourceManager.knowledgeSource.scheduleSuccess", { cron: values.cron }));
      } else {
        message.error(t("sourceManager.knowledgeSource.scheduleFailed"));
      }
    } finally {
      setScheduling(false);
    }
  };

  const handleSitemap = async (values: { baseUrl: string }) => {
    setImportingSitemap(true);
    try {
      const results = await sitemapCrawl(values.baseUrl);
      if (!results) {
        message.error(t("sourceManager.knowledgeSource.sitemapFailed"));
      } else {
        message.success(
          t("sourceManager.knowledgeSource.sitemapDone", { count: results.length }),
        );
        setSitemapOpen(false);
        sitemapForm.resetFields();
        void loadSources();
      }
    } finally {
      setImportingSitemap(false);
    }
  };

  const columns = useMemo<ColumnsType<KnowledgeSource>>(
    () => [
      {
        title: t("sourceManager.knowledgeSource.title"),
        dataIndex: "title",
        key: "title",
        render: (title: string, record) => {
          const meta = SOURCE_TYPE_META[record.sourceType] ?? SOURCE_TYPE_META.url;
          return (
            <Space>
              <Tag color={meta.color} style={{ marginRight: 0 }} icon={meta.icon} />
              <span>{title}</span>
            </Space>
          );
        },
      },
      {
        title: t("sourceManager.knowledgeSource.sourcePath"),
        dataIndex: "sourcePath",
        key: "sourcePath",
        ellipsis: true,
        render: (path: string) => (
          <Tooltip title={path}>
            <span className="font-mono text-xs">{path}</span>
          </Tooltip>
        ),
      },
      {
        title: t("sourceManager.knowledgeSource.status"),
        dataIndex: "status",
        key: "status",
        width: 90,
        render: (status: string) => (
          <Tag color={status === "active" ? "green" : "default"}>
            {status === "active"
              ? t("sourceManager.knowledgeSource.statusActive")
              : t("sourceManager.knowledgeSource.statusPaused")}
          </Tag>
        ),
      },
      {
        title: t("sourceManager.knowledgeSource.schedule"),
        dataIndex: "scheduleCron",
        key: "scheduleCron",
        width: 110,
        render: (cron?: string) =>
          cron ? <span className="font-mono text-xs">{cron}</span> : <span className="text-xs opacity-50">—</span>,
      },
      {
        title: t("sourceManager.knowledgeSource.lastFetched"),
        dataIndex: "lastFetchedAt",
        key: "lastFetchedAt",
        width: 150,
        render: (ts?: number) => ts ? new Date(ts).toLocaleString() : <span className="text-xs opacity-50">—</span>,
      },
      {
        title: t("sourceManager.knowledgeSource.actions"),
        key: "actions",
        width: 130,
        render: (_, record) => (
          <Space size={4}>
            <Tooltip title={t("sourceManager.knowledgeSource.fetchNow")}>
              <Button
                size="small"
                type="text"
                icon={<Play size={14} />}
                loading={fetchingId === record.id}
                onClick={() => void handleFetchNow(record.id)}
              />
            </Tooltip>
            <Tooltip title={t("sourceManager.knowledgeSource.configure")}>
              <Button
                size="small"
                type="text"
                icon={<Zap size={14} />}
                onClick={() => {
                  setEditSource(record);
                  editForm.setFieldsValue({
                    title: record.title,
                    status: record.status,
                    scheduleCron: record.scheduleCron ?? "",
                  });
                }}
              />
            </Tooltip>
            <Popconfirm
              title={t("sourceManager.knowledgeSource.deleteConfirm")}
              onConfirm={() => void deleteSource(record.id)}
            >
              <Button size="small" type="text" danger icon={<Trash2 size={14} />} />
            </Popconfirm>
          </Space>
        ),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [t, fetchingId],
  );

  return (
    <div className="flex flex-col gap-4">
      {/* URL 快速抓取 */}
      <Form
        form={quickForm}
        layout="inline"
        onFinish={handleQuickFetch}
        className="flex-wrap gap-2"
      >
        <Form.Item
          name="url"
          rules={[
            { required: true, message: t("sourceManager.knowledgeSource.urlRequired") },
            { type: "url", message: t("sourceManager.knowledgeSource.urlInvalid") },
          ]}
          style={{ minWidth: 320, flex: 1 }}
        >
          <Input
            prefix={<Globe size={14} className="opacity-50" />}
            placeholder={t("sourceManager.knowledgeSource.urlPlaceholder")}
            allowClear
          />
        </Form.Item>
        <Form.Item name="title" style={{ minWidth: 180 }}>
          <Input placeholder={t("sourceManager.knowledgeSource.titlePlaceholder")} allowClear />
        </Form.Item>
        <Form.Item>
          <Space>
            <Button type="primary" htmlType="submit" icon={<Zap size={14} />}>
              {t("sourceManager.knowledgeSource.fetchToWiki")}
            </Button>
            <Button
              icon={<RefreshCw size={14} />}
              loading={fetchingAll}
              onClick={() => void handleFetchAll()}
            >
              {t("sourceManager.knowledgeSource.fetchAll")}
            </Button>
            <Button icon={<GitFork size={14} />} onClick={() => setGithubOpen(true)}>
              {t("sourceManager.knowledgeSource.githubImport")}
            </Button>
            <Button icon={<Import size={14} />} onClick={() => setSitemapOpen(true)}>
              {t("sourceManager.knowledgeSource.sitemapImport")}
            </Button>
          </Space>
        </Form.Item>
      </Form>

      {/* 定时刷新 */}
      <Form form={cronForm} layout="inline" onFinish={handleSchedule} className="flex-wrap gap-2">
        <Form.Item
          name="cron"
          rules={[{ required: true, message: t("sourceManager.knowledgeSource.cronRequired") }]}
        >
          <Input
            prefix={<RefreshCw size={14} className="opacity-50" />}
            placeholder="0 3 * * *"
            style={{ width: 160 }}
            allowClear
          />
        </Form.Item>
        <Form.Item>
          <Button loading={scheduling} htmlType="submit" size="small">
            {t("sourceManager.knowledgeSource.scheduleSync")}
          </Button>
        </Form.Item>
      </Form>

      {/* GitHub 仓库导入 */}
      <Modal
        title={t("sourceManager.knowledgeSource.githubImportTitle")}
        open={githubOpen}
        onCancel={() => setGithubOpen(false)}
        footer={null}
        destroyOnHidden
      >
        <Form form={githubForm} layout="vertical" onFinish={handleGithubImport}>
          <Form.Item
            name="repo"
            label={t("sourceManager.knowledgeSource.githubRepo")}
            rules={[{ required: true, message: t("sourceManager.knowledgeSource.githubRepoRequired") }]}
          >
            <Input placeholder={t("sourceManager.knowledgeSource.githubRepoPlaceholder")} />
          </Form.Item>
          <Form.Item
            name="pathFilter"
            label={t("sourceManager.knowledgeSource.githubPath")}
            tooltip={t("sourceManager.knowledgeSource.githubPathHint")}
          >
            <Input placeholder={t("sourceManager.knowledgeSource.githubPathPlaceholder")} />
          </Form.Item>
          <div className="flex justify-end">
            <Button type="primary" htmlType="submit" loading={importingGithub}>
              {t("sourceManager.knowledgeSource.importSubmit")}
            </Button>
          </div>
        </Form>
      </Modal>

      {/* sitemap 批量抓取 */}
      <Modal
        title={t("sourceManager.knowledgeSource.sitemapTitle")}
        open={sitemapOpen}
        onCancel={() => setSitemapOpen(false)}
        footer={null}
        destroyOnHidden
      >
        <Form form={sitemapForm} layout="vertical" onFinish={handleSitemap}>
          <Form.Item
            name="baseUrl"
            label={t("sourceManager.knowledgeSource.sitemapUrl")}
            rules={[{ required: true, message: t("sourceManager.knowledgeSource.sitemapUrlRequired") }]}
          >
            <Input placeholder={t("sourceManager.knowledgeSource.sitemapUrlPlaceholder")} />
          </Form.Item>
          <div className="flex justify-end">
            <Button type="primary" htmlType="submit" loading={importingSitemap}>
              {t("sourceManager.knowledgeSource.sitemapSubmit")}
            </Button>
          </div>
        </Form>
      </Modal>

      {/* 编辑知识源 */}
      <Modal
        title={t("sourceManager.knowledgeSource.editTitle")}
        open={editSource !== null}
        onCancel={() => setEditSource(null)}
        footer={null}
        destroyOnHidden
      >
        <Form form={editForm} layout="vertical" onFinish={handleEdit} initialValues={{ status: "active" }}>
          <Form.Item
            name="title"
            label={t("sourceManager.knowledgeSource.title")}
            rules={[{ required: true, message: t("sourceManager.knowledgeSource.titleRequired") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="status"
            label={t("sourceManager.knowledgeSource.status")}
            rules={[{ required: true }]}
          >
            <Select
              options={[
                { label: t("sourceManager.knowledgeSource.statusActive"), value: "active" },
                { label: t("sourceManager.knowledgeSource.statusPaused"), value: "paused" },
              ]}
            />
          </Form.Item>
          <Form.Item
            name="scheduleCron"
            label={t("sourceManager.knowledgeSource.schedule")}
            tooltip={t("sourceManager.knowledgeSource.scheduleHint")}
          >
            <Input placeholder="0 3 * * *" allowClear />
          </Form.Item>
          <div className="flex justify-end">
            <Button type="primary" htmlType="submit" loading={editing}>
              {t("sourceManager.knowledgeSource.save")}
            </Button>
          </div>
        </Form>
      </Modal>

      {/* 知识源列表 */}
      <Table<KnowledgeSource>
        rowKey="id"
        size="small"
        loading={loading}
        columns={columns}
        dataSource={sources}
        pagination={{ pageSize: 10, showSizeChanger: false }}
        locale={{ emptyText: t("sourceManager.knowledgeSource.empty") }}
      />

      {/* 新增知识源 */}
      <Modal
        title={t("sourceManager.knowledgeSource.createTitle")}
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        footer={null}
        destroyOnHidden
      >
        <Form<CreateKnowledgeSourceInput>
          form={form}
          layout="vertical"
          onFinish={handleCreate}
          initialValues={{ sourceType: "url", status: "active" }}
        >
          <Form.Item
            name="sourceType"
            label={t("sourceManager.knowledgeSource.sourceType")}
            rules={[{ required: true }]}
          >
            <Select
              options={[
                { label: "URL", value: "url" },
                { label: "RSS", value: "rss" },
              ]}
            />
          </Form.Item>
          <Form.Item
            name="sourcePath"
            label={t("sourceManager.knowledgeSource.sourcePath")}
            rules={[{ required: true, message: t("sourceManager.knowledgeSource.pathRequired") }]}
          >
            <Input placeholder={t("sourceManager.knowledgeSource.pathPlaceholder")} />
          </Form.Item>
          <Form.Item
            name="title"
            label={t("sourceManager.knowledgeSource.title")}
            rules={[{ required: true, message: t("sourceManager.knowledgeSource.titleRequired") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="scheduleCron"
            label={t("sourceManager.knowledgeSource.schedule")}
            tooltip={t("sourceManager.knowledgeSource.scheduleHint")}
          >
            <Input placeholder="0 3 * * *" />
          </Form.Item>
          <Form.Item name="wikiId" hidden>
            <Input />
          </Form.Item>
          <div className="flex justify-end">
            <Button type="primary" htmlType="submit" loading={creating}>
              {t("sourceManager.knowledgeSource.createSubmit")}
            </Button>
          </div>
        </Form>
      </Modal>
    </div>
  );
}
