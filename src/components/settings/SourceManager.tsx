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
  Vault as VaultIcon,
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
  obsidian_vault: {
    color: "geekblue",
    icon: <VaultIcon size={16} />,
    labelKey: "sourceManager.type.obsidianVault",
    descKey: "sourceManager.typeDesc.obsidianVault",
    bgColor: "#f0f5ff",
    fgColor: "#2f54eb",
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
      const st = values.sourceType ?? "knowledge";
      await invoke("create_source", {
        input: {
          name: values.name,
          sourceType: st,
          description: values.description ?? null,
          embeddingProvider: st === "obsidian_vault" ? null : (values.embeddingProvider ?? null),
          scope: st === "memory" ? "global" : undefined,
          rootPath: st === "wiki" ? values.rootPath : undefined,
          vaultPath: st === "obsidian_vault" ? values.vaultPath : undefined,
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

  // obsidian_vault 与 wiki 类似，不需要 embedding
  const needsEmbedding = sourceType !== "wiki" && sourceType !== "obsidian_vault";

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
            <Select.Option value="obsidian_vault">
              <VaultIcon size={14} /> {t("sourceManager.type.obsidianVault")}
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
        {sourceType === "obsidian_vault" && (
          <Form.Item
            name="vaultPath"
            label={t("sourceManager.vaultPath")}
            rules={[{ required: true, message: t("sourceManager.vaultPathRequired") }]}
            extra={t("sourceManager.vaultPathHint")}
          >
            <Input placeholder="/absolute/path/to/your/obsidian/vault" />
          </Form.Item>
        )}
        {needsEmbedding && (
          <Form.Item
            name="embeddingProvider"
            label={t("sourceManager.embeddingModel")}
            rules={[{ required: true, message: t("sourceManager.embeddingRequired") }]}
          >
            <EmbeddingModelSelect
              value={form.getFieldValue("embeddingProvider")}
              onChange={(val) => form.setFieldValue("embeddingProvider", val)}
            />
          </Form.Item>
        )}
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
        navigate(`/llm-wiki/${source.id}/graph`);
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

  const configuredCount = useMemo(
    () => knowledgeSources.filter((s) => s.embeddingProvider).length,
    [knowledgeSources],
  );

  // 统计项配置（紧凑横条，不再用大卡片）
  const statsItems = useMemo(
    () => [
      {
        label: t("sourceManager.stats.knowledgeBases"),
        value: bases.length,
        Icon: Database,
        color: token.colorPrimary,
      },
      { label: t("sourceManager.stats.documents"), value: bases.length, Icon: BookOpen, color: token.colorInfo },
      {
        label: t("sourceManager.stats.vectorReady"),
        value: `${configuredCount} / ${knowledgeSources.length}`,
        Icon: Zap,
        color: token.colorSuccess,
      },
    ],
    [t, bases.length, configuredCount, knowledgeSources.length, token],
  );

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
          <div className="flex flex-col gap-3">
            {/* 紧凑统计条 + 标题 + 添加按钮 同一行 */}
            <div className="flex items-center justify-between flex-wrap gap-2">
              <div className="flex items-center gap-4 flex-wrap">
                {statsItems.map((s, i) => (
                  <div key={i} className="flex items-center gap-1.5">
                    <s.Icon size={14} style={{ color: s.color }} />
                    <Text type="secondary" style={{ fontSize: 12 }}>{s.label}</Text>
                    <Text strong style={{ fontSize: 14 }}>{s.value}</Text>
                  </div>
                ))}
              </div>
              <Button
                size="small"
                icon={<Plus size={14} />}
                onClick={() => onCreate?.()}
              >
                {t("settings.knowledge.add")}
              </Button>
            </div>

            <Spin spinning={knowledgeLoading}>
              {knowledgeSources.length === 0
                ? (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description={t("sourceManager.empty")}
                    style={{ padding: 24 }}
                  />
                )
                : (
                  <Row gutter={[10, 10]}>
                    {knowledgeSources.map((source) => (
                      <Col key={source.id} xs={24} sm={12} lg={8} xl={6}>
                        <SourceCard source={source} onViewConfig={onViewConfig} onViewDocument={handleViewDocument} />
                      </Col>
                    ))}
                  </Row>
                )}

              {bases.length > 0 && (
                <>
                  <Divider style={{ margin: `${token.marginMD}px 0` }} />
                  <div
                    className="flex items-center justify-between"
                    style={{ marginBottom: token.marginSM }}
                  >
                    <Text strong style={{ fontSize: 14 }}>
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
                  <Row gutter={[10, 10]}>
                    {bases.slice(0, 6).map((base) => (
                      <Col key={base.id} xs={24} sm={12} lg={8} xl={6}>
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
                                width: 32,
                                height: 32,
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

  const configuredCount = useMemo(
    () => memorySources.filter((s) => s.embeddingProvider).length,
    [memorySources],
  );

  const statsItems = useMemo(
    () => [
      { label: t("sourceManager.stats.namespaces"), value: namespaces.length, Icon: Brain, color: token.colorPrimary },
      {
        label: t("sourceManager.stats.memoryItems"),
        value: namespaces.length,
        Icon: Sparkles,
        color: token.colorPrimary,
      },
      {
        label: t("sourceManager.stats.vectorReady"),
        value: `${configuredCount} / ${memorySources.length}`,
        Icon: Zap,
        color: token.colorSuccess,
      },
    ],
    [t, namespaces.length, configuredCount, memorySources.length, token],
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div className="flex items-center gap-4 flex-wrap">
          {statsItems.map((s, i) => (
            <div key={i} className="flex items-center gap-1.5">
              <s.Icon size={14} style={{ color: s.color }} />
              <Text type="secondary" style={{ fontSize: 12 }}>{s.label}</Text>
              <Text strong style={{ fontSize: 14 }}>{s.value}</Text>
            </div>
          ))}
        </div>
        <Button
          size="small"
          icon={<Plus size={14} />}
          onClick={() => onCreate?.()}
        >
          {t("settings.memory.addNamespace")}
        </Button>
      </div>

      <Spin spinning={memoryLoading}>
        {memorySources.length === 0
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("sourceManager.empty")}
              style={{ padding: 24 }}
            />
          )
          : (
            <Row gutter={[10, 10]}>
              {memorySources.map((source) => (
                <Col key={source.id} xs={24} sm={12} lg={8} xl={6}>
                  <SourceCard source={source} onViewConfig={onViewConfig} />
                </Col>
              ))}
            </Row>
          )}

        {namespaces.length > 0 && (
          <>
            <Divider style={{ margin: `${token.marginMD}px 0` }} />
            <div
              className="flex items-center justify-between"
              style={{ marginBottom: token.marginSM }}
            >
              <Text strong style={{ fontSize: 14 }}>
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
            <Row gutter={[10, 10]}>
              {namespaces.slice(0, 6).map((ns) => (
                <Col key={ns.id} xs={24} sm={12} lg={8} xl={6}>
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
                          width: 32,
                          height: 32,
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
      const wikiIds = new Set(wikis.map((w) => w.id));
      const staleSources = wikiSources.filter((s) => !wikiIds.has(s.id));
      if (staleSources.length > 0) {
        fetchSources();
      }
    }
  }, [wikis, wikiSources, fetchSources]);

  // 统计数据缓存，避免每次 render 都 reduce
  const { totalNotes, totalSources } = useMemo(
    () => ({
      totalNotes: wikis.reduce((sum, w) => sum + (w.noteCount ?? 0), 0),
      totalSources: wikis.reduce((sum, w) => sum + (w.sourceCount ?? 0), 0),
    }),
    [wikis],
  );

  const statsItems = useMemo(
    () => [
      { label: t("sourceManager.stats.wikis"), value: wikis.length, Icon: Network, color: token.colorPrimary },
      { label: t("sourceManager.stats.notes"), value: totalNotes, Icon: BookOpen, color: token.colorPrimary },
      { label: t("sourceManager.stats.wikiSources"), value: totalSources, Icon: FolderPlus, color: token.colorWarning },
    ],
    [t, wikis.length, totalNotes, totalSources, token],
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div className="flex items-center gap-4 flex-wrap">
          {statsItems.map((s, i) => (
            <div key={i} className="flex items-center gap-1.5">
              <s.Icon size={14} style={{ color: s.color }} />
              <Text type="secondary" style={{ fontSize: 12 }}>{s.label}</Text>
              <Text strong style={{ fontSize: 14 }}>{s.value}</Text>
            </div>
          ))}
        </div>
        <Button
          size="small"
          icon={<Plus size={14} />}
          onClick={() => onCreate?.()}
        >
          {t("wiki.llm.createWiki")}
        </Button>
      </div>

      {wikiSources.length === 0 && wikis.length === 0
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("sourceManager.empty")}
            style={{ padding: 24 }}
          />
        )
        : (
          <>
            {wikiSources.length > 0 && (
              <Row
                gutter={[10, 10]}
                style={{
                  marginBottom: wikiSources.length > 0 && wikis.length > 0
                    ? token.marginMD
                    : 0,
                }}
              >
                {wikiSources.map((source) => (
                  <Col key={source.id} xs={24} sm={12} lg={8} xl={6}>
                    <SourceCard source={source} onViewConfig={onViewConfig} />
                  </Col>
                ))}
              </Row>
            )}

            {wikis.length > 0 && (
              <>
                {wikiSources.length > 0 && <Divider style={{ margin: `${token.marginMD}px 0` }} />}
                <div
                  className="flex items-center justify-between"
                  style={{ marginBottom: token.marginSM }}
                >
                  <Text strong style={{ fontSize: 14 }}>
                    {t("sourceManager.wiki.wikiList")}
                  </Text>
                </div>
                <Row gutter={[10, 10]}>
                  {wikis.map((wiki) => (
                    <Col key={wiki.id} xs={24} sm={12} lg={8} xl={6}>
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
                onClick={() => navigate(`/llm-wiki/${wiki.id}/graph`)}
              >
                {t("sourceManager.view")}
              </Button>
              <Button
                size="small"
                type="text"
                icon={<GitGraph size={12} />}
                onClick={() => navigate(`/llm-wiki/${wiki.id}/graph`)}
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

  // 各类型计数 — useMemo 缓存避免每次 render 都 filter
  const { knowledgeCount, memoryCount, wikiCount } = useMemo(
    () => {
      let knowledgeCount = 0;
      let memoryCount = 0;
      let wikiCount = 0;
      for (const s of sources) {
        if (s.containerType === "knowledge") { knowledgeCount++; }
        else if (s.containerType === "memory") { memoryCount++; }
        else if (s.containerType === "wiki") { wikiCount++; }
      }
      return { knowledgeCount, memoryCount, wikiCount };
    },
    [sources],
  );

  // 统计卡片配置
  const statsCards = useMemo(
    () => [
      {
        key: "knowledge",
        count: knowledgeCount,
        label: t("sourceManager.type.knowledge"),
        onClick: () => onNavigateToTab("knowledge"),
        meta: TYPE_META.knowledge,
        Icon: Database,
      },
      {
        key: "memory",
        count: memoryCount,
        label: t("sourceManager.type.memory"),
        onClick: () => onNavigateToTab("memory"),
        meta: TYPE_META.memory,
        Icon: Brain,
      },
      {
        key: "wiki",
        count: wikiCount,
        label: t("sourceManager.type.wiki"),
        onClick: () => onNavigateToTab("wiki"),
        meta: TYPE_META.wiki,
        Icon: Network,
      },
    ],
    [knowledgeCount, memoryCount, wikiCount, t, onNavigateToTab],
  );

  return (
    <div className="flex flex-col gap-3">
      {/* 统计卡片行 + 搜索栏合并：左侧 3 个统计卡 flex 自适应，右侧搜索框 */}
      <div className="flex items-stretch gap-2 flex-wrap">
        {statsCards.map(({ key, count, label, onClick, meta, Icon }) => (
          <Card
            key={key}
            hoverable
            size="small"
            onClick={onClick}
            style={{
              borderRadius: token.borderRadiusLG,
              borderColor: token.colorBorder,
              cursor: "pointer",
              flex: "1 1 200px",
              minWidth: 200,
            }}
            styles={{ body: { padding: `${token.paddingSM}px ${token.padding}px` } }}
          >
            <div className="flex items-center gap-2.5">
              <div
                className="flex items-center justify-center shrink-0"
                style={{
                  width: 32,
                  height: 32,
                  borderRadius: token.borderRadius,
                  backgroundColor: meta.bgColor,
                  color: meta.fgColor,
                }}
              >
                <Icon size={16} />
              </div>
              <div className="min-w-0">
                <Text type="secondary" style={{ fontSize: 11 }}>
                  {label}
                </Text>
                <div>
                  <Text strong style={{ fontSize: 20 }}>
                    {count}
                  </Text>
                </div>
              </div>
            </div>
          </Card>
        ))}
      </div>
      <Row gutter={[8, 8]} align="middle">
        <Col flex="auto" style={{ minWidth: 240 }}>
          <Input
            id="source-manager-input-176"
            prefix={<Search size={14} />}
            placeholder={t("sourceManager.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onPressEnter={handleSearch}
            allowClear
            onClear={() => setSearchResults(null)}
            size="middle"
            style={{ width: "100%" }}
          />
        </Col>
        <Col>
          <Button type="primary" icon={<Search size={14} />} loading={searching} onClick={handleSearch}>
            {t("sourceManager.search")}
          </Button>
        </Col>
        <Col>
          <Button icon={<Plus size={14} />} onClick={onCreateClick}>
            {t("sourceManager.createSource")}
          </Button>
        </Col>
        <Col>
          <Button icon={<FolderPlus size={14} />} onClick={() => onOpenImportModal("create")}>
            {t("sourceManager.importProjectSources")}
          </Button>
        </Col>
        <Col>
          <Button icon={<RefreshCw size={14} />} onClick={() => onOpenImportModal("update")}>
            {t("sourceManager.syncProjectSources")}
          </Button>
        </Col>
      </Row>

      <Spin spinning={loading}>
        {displaySources.length === 0
          ? (
            <Empty
              description={t("sourceManager.empty")}
              style={{ padding: 24 }}
            />
          )
          : (
            <Row gutter={[10, 10]}>
              {displaySources.map((source) => (
                <Col key={source.id} xs={24} sm={12} lg={8} xl={6}>
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

  // tabBar 右侧附加内容：创建按钮，与 Tab 同行节省垂直空间
  const tabBarExtraContent = (
    <Button
      size="small"
      type="primary"
      icon={<Plus size={14} />}
      onClick={() => setCreateOpen(true)}
    >
      {t("sourceManager.createSource")}
    </Button>
  );

  return (
    <div
      className="flex flex-col h-full"
      style={{ minHeight: 0 }}
    >
      {/* Tab 栏（含右侧创建按钮），底部边框分隔 */}
      <div
        className="flex items-center px-4 pt-3 shrink-0"
        data-testid="source-manager-tabs"
        style={{ borderBottom: `1px solid ${token.colorBorderSecondary}` }}
      >
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={tabItems}
          size="small"
          tabBarExtraContent={{ right: tabBarExtraContent }}
          className="source-manager-tabs"
          style={{ marginBottom: 0 }}
        />
      </div>

      {/* 内容区：flex:1 撑满剩余空间，内部滚动 */}
      <div
        className="flex-1"
        data-testid="source-manager-body"
        style={{
          minHeight: 0,
          overflowY: "auto",
          padding: `${token.paddingMD}px ${token.paddingLG}px`,
        }}
      >
        {/* 仅渲染当前激活的 Tab，避免 4 个 Tab 同时 mount 触发 4 路 IPC + 4 棵组件树渲染 */}
        {activeTab === "all" && (
          <AllSourcesTab
            onViewConfig={setConfigSource}
            onNavigateToTab={setActiveTab}
            onCreateClick={() => setCreateOpen(true)}
            onOpenImportModal={openImportModal}
          />
        )}
        {activeTab === "knowledge" && (
          <KnowledgeTab
            onViewConfig={setConfigSource}
            onCreate={() => setCreateOpen(true)}
          />
        )}
        {activeTab === "memory" && (
          <MemoryTab
            onViewConfig={setConfigSource}
            onCreate={() => setCreateOpen(true)}
          />
        )}
        {activeTab === "wiki" && <WikiTab onViewConfig={setConfigSource} onCreate={() => setCreateOpen(true)} />}
      </div>

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
