// SPDX-License-Identifier: AGPL-3.0-only

import { EmbeddingModelSelect } from "@/components/shared/EmbeddingModelSelect";
import { useEmbeddingProviderLabel } from "@/components/shared/ModelSelect";
import { invoke } from "@/lib/invoke";
import { useKnowledgeStore } from "@/stores";
import { useProviderStore, useSourceStore } from "@/stores";
import { useLlmWikiStore, type Wiki } from "@/stores/feature/llmWikiStore";
import { useMemoryStore } from "@/stores/feature/memoryStore";
import type { SourceConfig, UnifiedSource } from "@/stores/feature/sourceStore";
import type { KnowledgeBase } from "@/types";
import {
  App as AntdApp,
  Button,
  Card,
  Col,
  Descriptions,
  Divider,
  Empty,
  Form,
  Input,
  message,
  Modal,
  Popconfirm,
  Radio,
  Row,
  Select,
  Space,
  Spin,
  Statistic,
  Tabs,
  Tag,
  theme,
  Typography,
} from "antd";
import {
  ArrowLeft,
  BookOpen,
  Brain,
  Database,
  Eye,
  FolderPlus,
  GitGraph,
  Layers,
  Network,
  Plus,
  RefreshCw,
  Search,
  Settings,
  Sparkles,
  Trash2,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { KnowledgeBaseDocuments } from "./KnowledgeBaseDocuments";

const { Text, Paragraph } = Typography;

const TYPE_META: Record<
  string,
  {
    color: string;
    icon: React.ReactNode;
    labelKey: string;
    descKey: string;
    bgColor: string;
    fgColor: string;
  }
> = {
  knowledge: {
    color: "blue",
    icon: <Database size={16} />,
    labelKey: "sourceManager.type.knowledge",
    descKey: "sourceManager.typeDesc.knowledge",
    bgColor: "#e6f4ff",
    fgColor: "#1677ff",
  },
  memory: {
    color: "purple",
    icon: <Brain size={16} />,
    labelKey: "sourceManager.type.memory",
    descKey: "sourceManager.typeDesc.memory",
    bgColor: "#f9f0ff",
    fgColor: "#722ed1",
  },
  wiki: {
    color: "green",
    icon: <Network size={16} />,
    labelKey: "sourceManager.type.wiki",
    descKey: "sourceManager.typeDesc.wiki",
    bgColor: "#f6ffed",
    fgColor: "#52c41a",
  },
};

function TypeBadge({ containerType }: { containerType: string }) {
  const { t } = useTranslation();
  const meta = TYPE_META[containerType];
  if (!meta) {
    return <Tag>{containerType}</Tag>;
  }
  return (
    <Tag color={meta.color} icon={meta.icon}>
      {t(meta.labelKey, containerType)}
    </Tag>
  );
}

function SourceConfigModal({
  source,
  open,
  onClose,
}: {
  source: UnifiedSource | null;
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const getSourceConfig = useSourceStore((s) => s.getSourceConfig);
  const updateSourceEmbedding = useSourceStore((s) => s.updateSourceEmbedding);
  const rebuildSourceIndex = useSourceStore((s) => s.rebuildSourceIndex);
  const fetchSources = useSourceStore((s) => s.fetchSources);
  const formatProviderLabel = useEmbeddingProviderLabel();
  const [config, setConfig] = useState<SourceConfig | null>(null);
  const [loading, setLoading] = useState(false);
  const [editing, setEditing] = useState(false);
  const [draftProvider, setDraftProvider] = useState<string | undefined>(
    undefined,
  );
  const [saving, setSaving] = useState(false);
  const [rebuildConfirmOpen, setRebuildConfirmOpen] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [messageApi, contextHolder] = message.useMessage();

  useEffect(() => {
    if (!open || !source) {
      setTimeout(() => setConfig(null), 0);
      setEditing(false);
      setDraftProvider(undefined);
      return;
    }
    setTimeout(() => setLoading(true), 0);
    getSourceConfig(source.containerType, source.id)
      .then((cfg) => {
        setConfig(cfg);
        setDraftProvider(cfg.embeddingProvider ?? undefined);
      })
      .catch(() => setConfig(null))
      .finally(() => setLoading(false));
  }, [open, source, getSourceConfig]);

  const providerChanged = (draftProvider ?? "") !== (config?.embeddingProvider ?? "");

  const handleSave = async () => {
    if (!source) {
      return;
    }
    // 当 provider 真正变化时，弹出重建确认框
    if (providerChanged) {
      setRebuildConfirmOpen(true);
      return;
    }
    setEditing(false);
  };

  const handleConfirmRebuild = async () => {
    if (!source) {
      return;
    }
    setSaving(true);
    try {
      const { embeddingChanged } = await updateSourceEmbedding(
        source,
        draftProvider,
      );
      await fetchSources();
      setRebuildConfirmOpen(false);
      setEditing(false);
      setConfig((prev) => prev ? { ...prev, embeddingProvider: draftProvider } : prev);
      if (embeddingChanged) {
        // 触发后端重建索引（异步任务，不阻塞 UI）
        setRebuilding(true);
        rebuildSourceIndex(source.containerType, source.id)
          .then(() => messageApi.success(t("sourceManager.config.rebuildStarted")))
          .catch((e) => messageApi.error(String(e)))
          .finally(() => setRebuilding(false));
      } else {
        messageApi.success(t("sourceManager.config.saveSuccess"));
      }
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onCancel={onClose}
      footer={null}
      width={520}
      title={
        <span>
          <Settings
            size={16}
            style={{ marginRight: token.marginXS, verticalAlign: "middle" }}
          />
          {source?.name ?? ""}: {t("sourceManager.configTitle")}
        </span>
      }
    >
      {contextHolder}
      <Spin spinning={loading || saving || rebuilding}>
        {config
          ? (
            <div className="flex flex-col gap-3">
              <Descriptions column={1} size="small" bordered>
                <Descriptions.Item label={t("sourceManager.config.provider")}>
                  {editing
                    ? (
                      <EmbeddingModelSelect
                        value={draftProvider}
                        onChange={(val) => setDraftProvider(val || undefined)}
                        placeholder={t("settings.knowledge.embeddingModelPlaceholder")}
                        style={{ width: "100%" }}
                      />
                    )
                    : (
                      <span>
                        {config.embeddingProvider
                          ? formatProviderLabel(config.embeddingProvider)
                          : "—"}
                      </span>
                    )}
                </Descriptions.Item>
                <Descriptions.Item label={t("sourceManager.config.dimensions")}>
                  {config.embeddingDimensions ?? "—"}
                </Descriptions.Item>
                <Descriptions.Item label={t("sourceManager.config.threshold")}>
                  {config.retrievalThreshold ?? "—"}
                </Descriptions.Item>
                <Descriptions.Item label={t("sourceManager.config.topK")}>
                  {config.retrievalTopK ?? "—"}
                </Descriptions.Item>
              </Descriptions>
              <div className="flex justify-end gap-2">
                {editing
                  ? (
                    <>
                      <Button size="small" onClick={() => setEditing(false)} disabled={saving}>
                        {t("common.cancel")}
                      </Button>
                      <Button
                        size="small"
                        type="primary"
                        onClick={handleSave}
                        loading={saving}
                      >
                        {t("common.save")}
                      </Button>
                    </>
                  )
                  : (
                    <Button
                      size="small"
                      type="primary"
                      ghost
                      icon={<Settings size={12} />}
                      onClick={() => setEditing(true)}
                    >
                      {t("sourceManager.config.editEmbedding")}
                    </Button>
                  )}
              </div>
              {editing && providerChanged && (
                <Text type="warning" style={{ fontSize: 12 }}>
                  {t("sourceManager.config.changeWarning")}
                </Text>
              )}
            </div>
          )
          : (
            !loading && <Empty description={t("sourceManager.noConfig")} />
          )}
      </Spin>

      <Modal
        open={rebuildConfirmOpen}
        onCancel={() => setRebuildConfirmOpen(false)}
        onOk={handleConfirmRebuild}
        okButtonProps={{ danger: true, loading: saving }}
        okText={t("sourceManager.config.confirmRebuild")}
        cancelText={t("common.cancel")}
        title={t("sourceManager.config.changeEmbeddingTitle")}
        mask={{ enabled: true, blur: true }}
      >
        <p>{t("sourceManager.config.changeWarning")}</p>
      </Modal>
    </Modal>
  );
}

function CreateSourceModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const { fetchSources } = useSourceStore();
  const [form] = Form.useForm();
  const [creating, setCreating] = useState(false);
  const sourceType: string = Form.useWatch("sourceType", form) ?? "knowledge";

  const handleCreate = async () => {
    try {
      const values = await form.validateFields();
      setCreating(true);
      await invoke("create_source", {
        input: {
          name: values.name,
          sourceType: values.sourceType ?? "knowledge",
          description: values.description ?? null,
          embeddingProvider: values.embeddingProvider ?? null,
          scope: values.sourceType === "memory" ? "global" : undefined,
          rootPath: values.sourceType === "wiki" ? values.rootPath : undefined,
        },
      });
      form.resetFields();
      await fetchSources();
      onClose();
    } catch {
      // validation
    } finally {
      setCreating(false);
    }
  };

  return (
    <Modal
      title={t("sourceManager.createSource")}
      open={open}
      onOk={handleCreate}
      onCancel={() => {
        form.resetFields();
        onClose();
      }}
      confirmLoading={creating}
      width={480}
    >
      <Form form={form} layout="vertical">
        <Form.Item
          name="sourceType"
          label={t("sourceManager.typeLabel")}
          rules={[{ required: true }]}
          initialValue="knowledge"
        >
          <Select>
            <Select.Option value="knowledge">
              <Database size={14} /> {t("sourceManager.type.knowledge")}
            </Select.Option>
            <Select.Option value="memory">
              <Brain size={14} /> {t("sourceManager.type.memory")}
            </Select.Option>
            <Select.Option value="wiki">
              <Network size={14} /> {t("sourceManager.type.wiki")}
            </Select.Option>
          </Select>
        </Form.Item>
        <Form.Item name="name" label={t("sourceManager.sourceName")} rules={[{ required: true }]}>
          <Input />
        </Form.Item>
        <Form.Item name="description" label={t("sourceManager.description")}>
          <Input.TextArea rows={2} />
        </Form.Item>
        {sourceType === "wiki" && (
          <Form.Item name="rootPath" label={t("sourceManager.rootPath")} rules={[{ required: true }]}>
            <Input placeholder="/path/to/vault" />
          </Form.Item>
        )}
        <Form.Item
          name="embeddingProvider"
          label={t("sourceManager.embeddingModel")}
          rules={sourceType !== "wiki" ? [{ required: true, message: t("sourceManager.embeddingRequired") }] : []}
        >
          <EmbeddingModelSelect
            value={form.getFieldValue("embeddingProvider")}
            onChange={(val) => form.setFieldValue("embeddingProvider", val)}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
}

/// 导入/更新项目知识源的 Modal：支持自定义目录、知识源名称、模式（新增/更新）。
function ImportProjectSourcesModal({
  open,
  initialMode,
  onClose,
}: {
  open: boolean;
  initialMode: "create" | "update";
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const { fetchSources } = useSourceStore();
  const [form] = Form.useForm();
  const [importing, setImporting] = useState(false);
  const [messageApi, contextHolder] = message.useMessage();
  const mode: "create" | "update" = Form.useWatch("mode", form) ?? initialMode;

  // 打开时同步初始 mode 与默认名称
  useEffect(() => {
    if (open) {
      form.setFieldsValue({
        mode: initialMode,
        sourceName: t("sourceManager.importProjectModal.defaultSourceName"),
        sourcePath: "",
      });
    }
  }, [open, initialMode, form]);

  const handleSelectDirectory = useCallback(async () => {
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const selected = await openDialog({ directory: true, multiple: false });
      if (typeof selected === "string") {
        form.setFieldValue("sourcePath", selected);
      }
    } catch {
      // 用户取消或环境不支持
    }
  }, [form]);

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setImporting(true);
      const result = await invoke<{
        wikiId: string;
        wikiName: string;
        wikiImported: number;
        wikiFailed: number;
        wikiSkipped: number;
        kbId: string;
        kbName: string;
        entityCount: number;
        relationCount: number;
        embeddingProvider: string | null;
        embeddingChanged: boolean;
      }>("import_project_knowledge_sources", {
        sourcePath: values.sourcePath,
        sourceName: values.sourceName || undefined,
        mode: values.mode,
        embeddingProvider: values.embeddingProvider || undefined,
      });
      messageApi.success(t("sourceManager.importSuccess", {
        imported: result.wikiImported,
        skipped: result.wikiFailed + result.wikiSkipped,
        entities: result.entityCount,
        relations: result.relationCount,
        wikiName: result.wikiName,
        kbName: result.kbName,
      }));
      // 向量模型变更或新配置时提示用户重建索引
      if (result.embeddingChanged) {
        messageApi.warning(t("sourceManager.importProjectModal.embeddingChanged"));
      } else if (!result.embeddingProvider) {
        messageApi.info(t("sourceManager.importNoEmbedding"));
      }
      await fetchSources();
      form.resetFields();
      onClose();
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal
      title={t("sourceManager.importProjectModal.title")}
      open={open}
      onOk={handleSubmit}
      onCancel={() => {
        form.resetFields();
        onClose();
      }}
      okText={t("sourceManager.importProjectModal.confirm")}
      confirmLoading={importing}
      width={520}
    >
      {contextHolder}
      <Form form={form} layout="vertical">
        <Form.Item
          name="sourcePath"
          label={t("sourceManager.importProjectModal.directory")}
          rules={[{ required: true, message: t("sourceManager.importProjectModal.directoryRequired") }]}
        >
          <Space.Compact style={{ width: "100%" }}>
            <Input
              placeholder={t("sourceManager.importProjectModal.directoryPlaceholder")}
              readOnly
              style={{ width: "100%" }}
            />
            <Button
              onClick={handleSelectDirectory}
              icon={<FolderPlus size={12} />}
            >
              {t("sourceManager.importProjectModal.selectDirectory")}
            </Button>
          </Space.Compact>
        </Form.Item>
        <Form.Item
          name="sourceName"
          label={t("sourceManager.importProjectModal.sourceName")}
          rules={[{ required: true, message: t("sourceManager.importProjectModal.sourceNameRequired") }]}
          extra={t("sourceManager.importProjectModal.sourceNameHint")}
        >
          <Input placeholder={t("sourceManager.importProjectModal.sourceNamePlaceholder")} />
        </Form.Item>
        <Form.Item
          name="mode"
          label={t("sourceManager.importProjectModal.mode")}
          rules={[{ required: true }]}
        >
          <Radio.Group>
            <Radio value="create">
              <Text strong>{t("sourceManager.importProjectModal.modeCreate")}</Text>
              <br />
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("sourceManager.importProjectModal.modeCreateDesc")}
              </Text>
            </Radio>
            <Radio value="update" style={{ display: "block", marginLeft: 0, marginTop: 8 }}>
              <Text strong>{t("sourceManager.importProjectModal.modeUpdate")}</Text>
              <br />
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("sourceManager.importProjectModal.modeUpdateDesc")}
              </Text>
            </Radio>
          </Radio.Group>
        </Form.Item>
        <Form.Item
          name="embeddingProvider"
          label={t("sourceManager.importProjectModal.embeddingModel")}
          extra={t("sourceManager.importProjectModal.embeddingModelHint")}
        >
          <EmbeddingModelSelect
            value={form.getFieldValue("embeddingProvider")}
            onChange={(val) => form.setFieldValue("embeddingProvider", val)}
            placeholder={t("sourceManager.importProjectModal.embeddingModelPlaceholder")}
            style={{ width: "100%" }}
          />
        </Form.Item>
        {mode === "update" && (
          <Text type="warning" style={{ fontSize: 12 }}>
            {t("sourceManager.importProjectModal.updateWarning")}
          </Text>
        )}
      </Form>
    </Modal>
  );
}

function SourceCard({
  source,
  onViewConfig,
  onViewDocument,
}: {
  source: UnifiedSource;
  onViewConfig: (s: UnifiedSource) => void;
  onViewDocument?: (s: UnifiedSource) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const meta = TYPE_META[source.containerType];
  const deleteSource = useSourceStore((s) => s.deleteSource);
  const fetchSources = useSourceStore((s) => s.fetchSources);
  const [messageApi, contextHolder] = message.useMessage();
  const { modal } = AntdApp.useApp();
  const formatProviderLabel = useEmbeddingProviderLabel();

  const handleView = useCallback(() => {
    if (onViewDocument && source.containerType === "knowledge") {
      onViewDocument(source);
      return;
    }
    switch (source.containerType) {
      case "wiki":
        navigate(`/wiki/${source.id}`);
        break;
      case "knowledge":
      case "memory":
        navigate(`/knowledge`);
        break;
      default:
        break;
    }
  }, [navigate, onViewDocument, source]);

  const handleDeleteClick = () => {
    modal.confirm({
      title: t("sourceManager.confirmDelete"),
      content: t("sourceManager.deleteWarning"),
      okText: t("sourceManager.confirmDeleteOk"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      okCancel: true,
      onOk: async () => {
        try {
          await deleteSource(source);
          // 刷新所有来源列表（knowledge / memory store 内部列表也需重载）
          await Promise.all([
            fetchSources(),
            useKnowledgeStore.getState().loadBases?.(),
            useMemoryStore.getState().loadNamespaces?.(),
          ]).catch(() => {
            // 单个 store 失败不阻塞
          });
          messageApi.success(t("sourceManager.deleteSuccess"));
        } catch (e) {
          messageApi.error(String(e));
          throw e;
        }
      },
    });
  };

  return (
    <Card
      hoverable
      size="small"
      style={{ borderRadius: token.borderRadiusLG, overflow: "hidden" }}
      styles={{
        body: { padding: `${token.paddingSM}px ${token.padding}px` },
      }}
    >
      {contextHolder}
      <div className="flex items-start gap-3">
        <div
          className="shrink-0 flex items-center justify-center"
          style={{
            width: 40,
            height: 40,
            borderRadius: token.borderRadius,
            backgroundColor: meta
              ? `${token[`${meta.color}6` as keyof typeof token]}`
              : token.colorFillQuaternary,
            color: meta
              ? (token[`${meta.color}1` as keyof typeof token] as string)
              : token.colorTextSecondary,
          }}
        >
          {meta?.icon ?? <Layers size={16} />}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <Text strong ellipsis style={{ fontSize: 14, flex: 1 }}>
              {source.name}
            </Text>
            {!source.enabled && (
              <Tag color="default" style={{ fontSize: 10 }}>
                {t("sourceManager.disabled")}
              </Tag>
            )}
          </div>
          <div className="flex items-center gap-2 mb-2">
            <TypeBadge containerType={source.containerType} />
            {source.embeddingProvider && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {formatProviderLabel(source.embeddingProvider)}
                {source.embeddingDimensions
                  ? ` · ${source.embeddingDimensions}d`
                  : ""}
              </Text>
            )}
          </div>
          {source.description && (
            <Paragraph
              type="secondary"
              ellipsis={{ rows: 2 }}
              style={{ fontSize: 12, marginBottom: 8 }}
            >
              {source.description}
            </Paragraph>
          )}
          <div className="flex items-center gap-1">
            <Button
              size="small"
              type="primary"
              ghost
              icon={<Eye size={12} />}
              onClick={handleView}
            >
              {t("sourceManager.view")}
            </Button>
            <Button
              size="small"
              type="text"
              icon={<Settings size={12} />}
              onClick={() => onViewConfig(source)}
            >
              {t("sourceManager.viewConfig")}
            </Button>
            <Button
              size="small"
              type="text"
              danger
              icon={<Trash2 size={12} />}
              onClick={handleDeleteClick}
            />
          </div>
        </div>
      </div>
    </Card>
  );
}

function KnowledgeTab({
  onViewConfig,
  onCreate,
}: {
  onViewConfig: (s: UnifiedSource) => void;
  onCreate?: () => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const { bases, loadBases, loading: knowledgeLoading } = useKnowledgeStore();
  const allSources = useSourceStore((s) => s.sources);
  const knowledgeSources = useMemo(
    () => allSources.filter((s) => s.containerType === "knowledge"),
    [allSources],
  );

  useEffect(() => {
    loadBases();
  }, [loadBases]);

  const [selectedBase, setSelectedBase] = useState<KnowledgeBase | null>(
    bases.length > 0 ? bases[0] : null,
  );

  const handleViewDocument = useCallback((source: UnifiedSource) => {
    const base = bases.find((b) => b.name === source.name);
    if (base) { setSelectedBase(base); }
  }, [bases]);

  const configuredCount = knowledgeSources.filter(
    (s) => s.embeddingProvider,
  ).length;

  return (
    <div>
      {selectedBase
        ? (
          <div>
            <Button
              type="text"
              icon={<ArrowLeft size={16} />}
              onClick={() => setSelectedBase(null)}
              style={{ marginBottom: token.marginSM }}
            >
              {t("sourceManager.backToList")}
            </Button>
            <KnowledgeBaseDocuments base={selectedBase} />
          </div>
        )
        : (
          <div>
            <Row gutter={[16, 16]} style={{ marginBottom: token.marginLG }}>
              <Col span={8}>
                <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
                  <Statistic
                    title={t("sourceManager.stats.knowledgeBases")}
                    value={bases.length}
                    prefix={<Database size={16} style={{ color: token.colorPrimary }} />}
                    styles={{ content: { fontSize: 24 } }}
                  />
                </Card>
              </Col>
              <Col span={8}>
                <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
                  <Statistic
                    title={t("sourceManager.stats.documents")}
                    value={bases.length}
                    prefix={<BookOpen size={16} style={{ color: token.colorInfo }} />}
                    styles={{ content: { fontSize: 24 } }}
                  />
                </Card>
              </Col>
              <Col span={8}>
                <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
                  <Statistic
                    title={t("sourceManager.stats.vectorReady")}
                    value={configuredCount}
                    suffix={`/ ${knowledgeSources.length}`}
                    prefix={<Zap size={16} style={{ color: token.colorSuccess }} />}
                    styles={{ content: { fontSize: 24 } }}
                  />
                </Card>
              </Col>
            </Row>

            <div
              className="flex items-center justify-between"
              style={{ marginBottom: token.marginMD }}
            >
              <Text strong style={{ fontSize: 15 }}>
                {t("sourceManager.knowledge.title")}
              </Text>
              <div className="flex items-center gap-2">
                <Button
                  size="small"
                  icon={<Plus size={14} />}
                  onClick={() => onCreate?.()}
                >
                  {t("settings.knowledge.add")}
                </Button>
              </div>
            </div>

            <Spin spinning={knowledgeLoading}>
              {knowledgeSources.length === 0
                ? (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description={t("sourceManager.empty")}
                    style={{ padding: 40 }}
                  />
                )
                : (
                  <Row gutter={[12, 12]}>
                    {knowledgeSources.map((source) => (
                      <Col key={source.id} xs={24} sm={12} lg={8}>
                        <SourceCard
                          source={source}
                          onViewConfig={onViewConfig}
                          onViewDocument={handleViewDocument}
                        />
                      </Col>
                    ))}
                  </Row>
                )}

              {bases.length > 0 && (
                <>
                  <Divider style={{ margin: `${token.marginLG}px 0` }} />
                  <div
                    className="flex items-center justify-between"
                    style={{ marginBottom: token.marginMD }}
                  >
                    <Text strong style={{ fontSize: 15 }}>
                      {t("sourceManager.knowledge.recentBases")}
                    </Text>
                    <Button
                      size="small"
                      type="link"
                      onClick={() => navigate("/knowledge")}
                    >
                      {t("sourceManager.viewAll")}
                    </Button>
                  </div>
                  <Row gutter={[12, 12]}>
                    {bases.slice(0, 6).map((base) => (
                      <Col key={base.id} xs={24} sm={12} lg={8}>
                        <Card
                          hoverable
                          size="small"
                          style={{ borderRadius: token.borderRadiusLG }}
                          onClick={() => setSelectedBase(base)}
                          styles={{ body: { padding: token.paddingSM } }}
                        >
                          <div className="flex items-center gap-3">
                            <div
                              className="shrink-0 flex items-center justify-center"
                              style={{
                                width: 36,
                                height: 36,
                                borderRadius: token.borderRadius,
                                backgroundColor: TYPE_META.knowledge.bgColor,
                                color: TYPE_META.knowledge.fgColor,
                              }}
                            >
                              <Database size={16} />
                            </div>
                            <div className="flex-1 min-w-0">
                              <Text strong ellipsis style={{ fontSize: 13 }}>
                                {base.name}
                              </Text>
                              <div className="flex items-center gap-2 mt-1">
                                <Tag
                                  color={base.embeddingProvider ? "green" : "default"}
                                  style={{ fontSize: 10, margin: 0 }}
                                >
                                  {base.embeddingProvider
                                    ? t("settings.knowledge.vectorReady")
                                    : t("settings.knowledge.vectorNotConfigured")}
                                </Tag>
                              </div>
                            </div>
                          </div>
                        </Card>
                      </Col>
                    ))}
                  </Row>
                </>
              )}
            </Spin>
          </div>
        )}
    </div>
  );
}

function MemoryTab({
  onViewConfig,
  onCreate,
}: {
  onViewConfig: (s: UnifiedSource) => void;
  onCreate?: () => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const {
    namespaces,
    loadNamespaces,
    loading: memoryLoading,
  } = useMemoryStore();
  const allSources = useSourceStore((s) => s.sources);
  const memorySources = useMemo(
    () => allSources.filter((s) => s.containerType === "memory"),
    [allSources],
  );

  useEffect(() => {
    loadNamespaces();
  }, [loadNamespaces]);

  const configuredCount = memorySources.filter(
    (s) => s.embeddingProvider,
  ).length;

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: token.marginLG }}>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.namespaces")}
              value={namespaces.length}
              prefix={<Brain size={16} style={{ color: token.colorPrimary }} />}
              styles={{ content: { fontSize: 24 } }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.memoryItems")}
              value={namespaces.length}
              prefix={<Sparkles size={16} style={{ color: token.colorPrimary }} />}
              styles={{ content: { fontSize: 24 } }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.vectorReady")}
              value={configuredCount}
              suffix={`/ ${memorySources.length}`}
              prefix={<Zap size={16} style={{ color: token.colorSuccess }} />}
              styles={{ content: { fontSize: 24 } }}
            />
          </Card>
        </Col>
      </Row>

      <div
        className="flex items-center justify-between"
        style={{ marginBottom: token.marginMD }}
      >
        <Text strong style={{ fontSize: 15 }}>
          {t("sourceManager.memory.title")}
        </Text>
        <div className="flex items-center gap-2">
          <Button
            size="small"
            icon={<Plus size={14} />}
            onClick={() => onCreate?.()}
          >
            {t("settings.memory.addNamespace")}
          </Button>
        </div>
      </div>

      <Spin spinning={memoryLoading}>
        {memorySources.length === 0
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("sourceManager.empty")}
              style={{ padding: 40 }}
            />
          )
          : (
            <Row gutter={[12, 12]}>
              {memorySources.map((source) => (
                <Col key={source.id} xs={24} sm={12} lg={8}>
                  <SourceCard source={source} onViewConfig={onViewConfig} />
                </Col>
              ))}
            </Row>
          )}

        {namespaces.length > 0 && (
          <>
            <Divider style={{ margin: `${token.marginLG}px 0` }} />
            <div
              className="flex items-center justify-between"
              style={{ marginBottom: token.marginMD }}
            >
              <Text strong style={{ fontSize: 15 }}>
                {t("sourceManager.memory.namespaces")}
              </Text>
              <Button
                size="small"
                type="link"
                onClick={() => navigate("/knowledge")}
              >
                {t("sourceManager.viewAll")}
              </Button>
            </div>
            <Row gutter={[12, 12]}>
              {namespaces.slice(0, 6).map((ns) => (
                <Col key={ns.id} xs={24} sm={12} lg={8}>
                  <Card
                    hoverable
                    size="small"
                    style={{ borderRadius: token.borderRadiusLG }}
                    onClick={() => navigate("/knowledge")}
                    styles={{ body: { padding: token.paddingSM } }}
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="shrink-0 flex items-center justify-center"
                        style={{
                          width: 36,
                          height: 36,
                          borderRadius: token.borderRadius,
                          backgroundColor: TYPE_META.memory.bgColor,
                          color: TYPE_META.memory.fgColor,
                        }}
                      >
                        <Brain size={16} />
                      </div>
                      <div className="flex-1 min-w-0">
                        <Text strong ellipsis style={{ fontSize: 13 }}>
                          {ns.name}
                        </Text>
                        <div className="flex items-center gap-2 mt-1">
                          <Tag
                            color={ns.embeddingProvider ? "green" : "default"}
                            style={{ fontSize: 10, margin: 0 }}
                          >
                            {ns.embeddingProvider
                              ? t("settings.memory.vectorReady")
                              : t("settings.memory.vectorNotConfigured")}
                          </Tag>
                        </div>
                      </div>
                    </div>
                  </Card>
                </Col>
              ))}
            </Row>
          </>
        )}
      </Spin>
    </div>
  );
}

function WikiTab({
  onViewConfig,
  onCreate,
}: {
  onViewConfig: (s: UnifiedSource) => void;
  onCreate?: () => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { wikis, loadWikis } = useLlmWikiStore();
  const fetchSources = useSourceStore((s) => s.fetchSources);
  const allSources = useSourceStore((s) => s.sources);
  const wikiSources = useMemo(
    () => allSources.filter((s) => s.containerType === "wiki"),
    [allSources],
  );

  useEffect(() => {
    loadWikis();
  }, [loadWikis]);

  // Wiki 删除后同步刷新 UnifiedSource 列表，清除残留的 SourceCard
  useEffect(() => {
    if (wikis.length === 0 && wikiSources.length > 0) {
      fetchSources();
    } else {
      // 比较 wikis 和 wikiSources 是否一致，不一致时刷新
      const wikiIds = new Set(wikis.map((w) => w.id));
      const staleSources = wikiSources.filter((s) => !wikiIds.has(s.id));
      if (staleSources.length > 0) {
        fetchSources();
      }
    }
  }, [wikis, wikiSources, fetchSources]);

  const totalNotes = wikis.reduce((sum, w) => sum + (w.noteCount ?? 0), 0);
  const totalSources = wikis.reduce((sum, w) => sum + (w.sourceCount ?? 0), 0);

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: token.marginLG }}>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.wikis")}
              value={wikis.length}
              prefix={<Network size={16} style={{ color: token.colorPrimary }} />}
              styles={{ content: { fontSize: 24 } }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.notes")}
              value={totalNotes}
              prefix={<BookOpen size={16} style={{ color: token.colorPrimary }} />}
              styles={{ content: { fontSize: 24 } }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.wikiSources")}
              value={totalSources}
              prefix={<FolderPlus size={16} style={{ color: token.colorWarning }} />}
              styles={{ content: { fontSize: 24 } }}
            />
          </Card>
        </Col>
      </Row>

      <div
        className="flex items-center justify-between"
        style={{ marginBottom: token.marginMD }}
      >
        <Text strong style={{ fontSize: 15 }}>
          {t("sourceManager.wiki.title")}
        </Text>
        <div className="flex items-center gap-2">
          <Button
            size="small"
            icon={<Plus size={14} />}
            onClick={() => onCreate?.()}
          >
            {t("wiki.llm.createWiki")}
          </Button>
        </div>
      </div>

      {wikiSources.length === 0 && wikis.length === 0
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("sourceManager.empty")}
            style={{ padding: 40 }}
          />
        )
        : (
          <>
            {wikiSources.length > 0 && (
              <Row
                gutter={[12, 12]}
                style={{
                  marginBottom: wikiSources.length > 0 && wikis.length > 0
                    ? token.marginMD
                    : 0,
                }}
              >
                {wikiSources.map((source) => (
                  <Col key={source.id} xs={24} sm={12} lg={8}>
                    <SourceCard source={source} onViewConfig={onViewConfig} />
                  </Col>
                ))}
              </Row>
            )}

            {wikis.length > 0 && (
              <>
                {wikiSources.length > 0 && <Divider style={{ margin: `${token.marginLG}px 0` }} />}
                <div
                  className="flex items-center justify-between"
                  style={{ marginBottom: token.marginMD }}
                >
                  <Text strong style={{ fontSize: 15 }}>
                    {t("sourceManager.wiki.wikiList")}
                  </Text>
                </div>
                <Row gutter={[12, 12]}>
                  {wikis.map((wiki) => (
                    <Col key={wiki.id} xs={24} sm={12} lg={8}>
                      <WikiCard wiki={wiki} />
                    </Col>
                  ))}
                </Row>
              </>
            )}
          </>
        )}
    </div>
  );
}

function WikiCard({ wiki }: { wiki: Wiki }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const deleteWiki = useLlmWikiStore((s) => s.deleteWiki);
  const [messageApi, contextHolder] = message.useMessage();

  const handleDelete = async () => {
    try {
      await deleteWiki(wiki.id);
      messageApi.success(t("wiki.llm.deleteSuccess"));
    } catch (e) {
      messageApi.error(String(e));
    }
  };

  return (
    <>
      {contextHolder}
      <Card
        hoverable
        size="small"
        style={{ borderRadius: token.borderRadiusLG }}
        styles={{ body: { padding: token.paddingSM } }}
      >
        <div className="flex items-start gap-3">
          <div
            className="shrink-0 flex items-center justify-center"
            style={{
              width: 40,
              height: 40,
              borderRadius: token.borderRadius,
              backgroundColor: TYPE_META.wiki.bgColor,
              color: TYPE_META.wiki.fgColor,
            }}
          >
            <Network size={16} />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <Text strong ellipsis style={{ fontSize: 14, flex: 1 }}>
                {wiki.name}
              </Text>
              <Tag color="blue" style={{ fontSize: 10 }}>
                v{wiki.schemaVersion}
              </Tag>
            </div>
            <div className="flex items-center gap-3 mb-2">
              <Text type="secondary" style={{ fontSize: 12 }}>
                {wiki.noteCount ?? 0} {t("sourceManager.stats.notes")}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {wiki.sourceCount ?? 0} {t("sourceManager.stats.wikiSources")}
              </Text>
            </div>
            {wiki.description && (
              <Paragraph
                type="secondary"
                ellipsis={{ rows: 1 }}
                style={{ fontSize: 12, marginBottom: 8 }}
              >
                {wiki.description}
              </Paragraph>
            )}
            <div className="flex items-center gap-1">
              <Button
                size="small"
                type="primary"
                ghost
                icon={<Eye size={12} />}
                onClick={() => navigate(`/wiki/${wiki.id}`)}
              >
                {t("sourceManager.view")}
              </Button>
              <Button
                size="small"
                type="text"
                icon={<GitGraph size={12} />}
                onClick={() => navigate(`/wiki/${wiki.id}`)}
              >
                {t("wiki.graph.title")}
              </Button>
              <Popconfirm
                title={t("wiki.llm.confirmDelete")}
                onConfirm={handleDelete}
              >
                <Button
                  size="small"
                  type="text"
                  danger
                  icon={<Trash2 size={12} />}
                />
              </Popconfirm>
            </div>
          </div>
        </div>
      </Card>
    </>
  );
}

function AllSourcesTab({
  onViewConfig,
  onNavigateToTab,
  onCreateClick,
  onOpenImportModal,
}: {
  onViewConfig: (s: UnifiedSource) => void;
  onNavigateToTab: (tab: string) => void;
  onCreateClick: () => void;
  onOpenImportModal: (mode: "create" | "update") => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { sources, loading, searchAllSources } = useSourceStore();
  const [searchQuery, setSearchQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<UnifiedSource[] | null>(
    null,
  );

  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) {
      setSearchResults(null);
      return;
    }
    setSearching(true);
    try {
      const result = await searchAllSources(searchQuery.trim());
      const matchedIds = new Set(result.sources.map((s) => s.containerId));
      setSearchResults(sources.filter((s) => matchedIds.has(s.id)));
    } catch {
      setSearchResults(null);
    } finally {
      setSearching(false);
    }
  }, [searchQuery, searchAllSources, sources]);

  const displaySources = searchResults ?? sources;

  const knowledgeCount = sources.filter(
    (s) => s.containerType === "knowledge",
  ).length;
  const memoryCount = sources.filter(
    (s) => s.containerType === "memory",
  ).length;
  const wikiCount = sources.filter((s) => s.containerType === "wiki").length;

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: token.marginLG }}>
        <Col span={8}>
          <Card
            hoverable
            size="small"
            onClick={() => onNavigateToTab("knowledge")}
            style={{
              borderRadius: token.borderRadiusLG,
              borderColor: token.colorBorder,
              cursor: "pointer",
            }}
            styles={{ body: { padding: token.paddingSM } }}
          >
            <div className="flex items-center gap-3">
              <div
                className="flex items-center justify-center"
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: token.borderRadius,
                  backgroundColor: TYPE_META.knowledge.bgColor,
                  color: TYPE_META.knowledge.fgColor,
                }}
              >
                <Database size={18} />
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("sourceManager.type.knowledge")}
                </Text>
                <div>
                  <Text strong style={{ fontSize: 22 }}>
                    {knowledgeCount}
                  </Text>
                </div>
              </div>
            </div>
          </Card>
        </Col>
        <Col span={8}>
          <Card
            hoverable
            size="small"
            onClick={() => onNavigateToTab("memory")}
            style={{
              borderRadius: token.borderRadiusLG,
              borderColor: token.colorBorder,
              cursor: "pointer",
            }}
            styles={{ body: { padding: token.paddingSM } }}
          >
            <div className="flex items-center gap-3">
              <div
                className="flex items-center justify-center"
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: token.borderRadius,
                  backgroundColor: TYPE_META.memory.bgColor,
                  color: TYPE_META.memory.fgColor,
                }}
              >
                <Brain size={18} />
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("sourceManager.type.memory")}
                </Text>
                <div>
                  <Text strong style={{ fontSize: 22 }}>
                    {memoryCount}
                  </Text>
                </div>
              </div>
            </div>
          </Card>
        </Col>
        <Col span={8}>
          <Card
            hoverable
            size="small"
            onClick={() => onNavigateToTab("wiki")}
            style={{
              borderRadius: token.borderRadiusLG,
              borderColor: token.colorBorder,
              cursor: "pointer",
            }}
            styles={{ body: { padding: token.paddingSM } }}
          >
            <div className="flex items-center gap-3">
              <div
                className="flex items-center justify-center"
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: token.borderRadius,
                  backgroundColor: TYPE_META.wiki.bgColor,
                  color: TYPE_META.wiki.fgColor,
                }}
              >
                <Network size={18} />
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("sourceManager.type.wiki")}
                </Text>
                <div>
                  <Text strong style={{ fontSize: 22 }}>
                    {wikiCount}
                  </Text>
                </div>
              </div>
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[12, 12]} style={{ marginBottom: token.marginMD }}>
        <Col flex="auto">
          <Input
            id="source-manager-input-176"
            prefix={<Search size={14} />}
            placeholder={t("sourceManager.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onPressEnter={handleSearch}
            allowClear
            onClear={() => setSearchResults(null)}
          />
        </Col>
        <Col>
          <Button
            type="primary"
            icon={<Search size={14} />}
            loading={searching}
            onClick={handleSearch}
          >
            {t("sourceManager.search")}
          </Button>
        </Col>
        <Col>
          <Button icon={<Plus size={14} />} onClick={onCreateClick}>
            {t("sourceManager.createSource")}
          </Button>
        </Col>
        <Col>
          <Button
            icon={<FolderPlus size={14} />}
            onClick={() => onOpenImportModal("create")}
          >
            {t("sourceManager.importProjectSources")}
          </Button>
        </Col>
        <Col>
          <Button
            icon={<RefreshCw size={14} />}
            onClick={() => onOpenImportModal("update")}
          >
            {t("sourceManager.syncProjectSources")}
          </Button>
        </Col>
      </Row>

      <Spin spinning={loading}>
        {displaySources.length === 0
          ? (
            <Empty
              description={t("sourceManager.empty")}
              style={{ padding: 40 }}
            />
          )
          : (
            <Row gutter={[12, 12]}>
              {displaySources.map((source) => (
                <Col key={source.id} xs={24} sm={12} lg={8}>
                  <SourceCard source={source} onViewConfig={onViewConfig} />
                </Col>
              ))}
            </Row>
          )}
      </Spin>
    </div>
  );
}

function SourceManager() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { fetchSources } = useSourceStore();
  const providers = useProviderStore((s) => s.providers);
  const fetchProviders = useProviderStore((s) => s.fetchProviders);
  const [activeTab, setActiveTab] = useState("all");
  const [configSource, setConfigSource] = useState<UnifiedSource | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [importMode, setImportMode] = useState<"create" | "update">("create");

  useEffect(() => {
    fetchSources();
    // 确保 provider 列表已加载，供 EmbeddingModelSelect / useEmbeddingProviderLabel 解析名称
    if (providers.length === 0) {
      void fetchProviders();
    }
  }, [fetchSources, fetchProviders, providers.length]);

  const openImportModal = useCallback((mode: "create" | "update") => {
    setImportMode(mode);
    setImportOpen(true);
  }, []);

  const tabItems = [
    {
      key: "all",
      label: (
        <span className="flex items-center gap-1">
          <Layers size={14} />
          {t("sourceManager.tab.all")}
        </span>
      ),
    },
    {
      key: "knowledge",
      label: (
        <span className="flex items-center gap-1">
          <Database size={14} />
          {t("sourceManager.tab.knowledge")}
        </span>
      ),
    },
    {
      key: "memory",
      label: (
        <span className="flex items-center gap-1">
          <Brain size={14} />
          {t("sourceManager.tab.memory")}
        </span>
      ),
    },
    {
      key: "wiki",
      label: (
        <span className="flex items-center gap-1">
          <Network size={14} />
          {t("sourceManager.tab.wiki")}
        </span>
      ),
    },
  ];

  return (
    <div style={{ padding: token.paddingLG }}>
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={tabItems.map((tab) => ({
          ...tab,
          children: (
            <>
              {tab.key === "all" && (
                <AllSourcesTab
                  onViewConfig={setConfigSource}
                  onNavigateToTab={setActiveTab}
                  onCreateClick={() => setCreateOpen(true)}
                  onOpenImportModal={openImportModal}
                />
              )}
              {tab.key === "knowledge" && (
                <KnowledgeTab
                  onViewConfig={setConfigSource}
                  onCreate={() => setCreateOpen(true)}
                />
              )}
              {tab.key === "memory" && (
                <MemoryTab
                  onViewConfig={setConfigSource}
                  onCreate={() => setCreateOpen(true)}
                />
              )}
              {tab.key === "wiki" && <WikiTab onViewConfig={setConfigSource} onCreate={() => setCreateOpen(true)} />}
            </>
          ),
        }))}
      />

      <SourceConfigModal
        source={configSource}
        open={configSource !== null}
        onClose={() => setConfigSource(null)}
      />

      <CreateSourceModal
        open={createOpen}
        onClose={() => setCreateOpen(false)}
      />

      <ImportProjectSourcesModal
        open={importOpen}
        initialMode={importMode}
        onClose={() => setImportOpen(false)}
      />
    </div>
  );
}

export { SourceManager };
