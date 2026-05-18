import { EmbeddingModelSelect } from "@/components/shared/EmbeddingModelSelect";
import { useKnowledgeStore } from "@/stores";
import { useSourceStore } from "@/stores";
import { useLlmWikiStore, type Wiki } from "@/stores/feature/llmWikiStore";
import { useMemoryStore } from "@/stores/feature/memoryStore";
import type { SourceConfig, UnifiedSource } from "@/stores/feature/sourceStore";
import {
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
  Row,
  Spin,
  Statistic,
  Tabs,
  Tag,
  theme,
  Typography,
} from "antd";
import {
  BookOpen,
  Brain,
  Database,
  Eye,
  FolderPlus,
  GitGraph,
  Layers,
  Network,
  Plus,
  Search,
  Settings,
  Sparkles,
  Trash2,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Text, Paragraph } = Typography;

const TYPE_META: Record<
  string,
  { color: string; icon: React.ReactNode; labelKey: string; descKey: string; bgColor: string; fgColor: string }
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
  if (!meta) { return <Tag>{containerType}</Tag>; }
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
  const [config, setConfig] = useState<SourceConfig | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open || !source) {
      setConfig(null);
      return;
    }
    setLoading(true);
    getSourceConfig(source.containerType, source.id)
      .then(setConfig)
      .catch(() => setConfig(null))
      .finally(() => setLoading(false));
  }, [open, source, getSourceConfig]);

  return (
    <Modal
      open={open}
      onCancel={onClose}
      footer={null}
      width={520}
      title={
        <span>
          <Settings size={16} style={{ marginRight: token.marginXS, verticalAlign: "middle" }} />
          {source?.name ?? ""}: {t("sourceManager.configTitle")}
        </span>
      }
    >
      <Spin spinning={loading}>
        {config
          ? (
            <Descriptions column={1} size="small" bordered>
              <Descriptions.Item label={t("sourceManager.config.provider")}>
                {config.embeddingProvider ?? "—"}
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
          )
          : (
            !loading && <Empty description={t("sourceManager.noConfig")} />
          )}
      </Spin>
    </Modal>
  );
}

function CreateKnowledgeBaseModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const createBase = useKnowledgeStore((s) => s.createBase);
  const [form] = Form.useForm();
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    try {
      const values = await form.validateFields();
      setCreating(true);
      await createBase(values);
      form.resetFields();
      onClose();
    } catch {
      // validation
    } finally {
      setCreating(false);
    }
  };

  return (
    <Modal
      title={t("settings.knowledge.add")}
      open={open}
      onOk={handleCreate}
      onCancel={() => {
        form.resetFields();
        onClose();
      }}
      confirmLoading={creating}
    >
      <Form form={form} layout="vertical">
        <Form.Item name="name" label={t("settings.knowledge.name")} rules={[{ required: true }]}>
          <Input name="name" />
        </Form.Item>
        <Form.Item
          name="embeddingProvider"
          label={t("settings.knowledge.embeddingModel")}
          rules={[{ required: true, message: t("settings.knowledge.embeddingModelPlaceholder") }]}
        >
          <EmbeddingModelSelect
            value={form.getFieldValue("embeddingProvider")}
            onChange={(val) => form.setFieldValue("embeddingProvider", val)}
            placeholder={t("settings.knowledge.embeddingModelPlaceholder")}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
}

function CreateMemoryNamespaceModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const createNamespace = useMemoryStore((s) => s.createNamespace);
  const [form] = Form.useForm();
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    try {
      const values = await form.validateFields();
      setCreating(true);
      await createNamespace(values.name, "global", values.embeddingProvider);
      form.resetFields();
      onClose();
    } catch {
      // validation
    } finally {
      setCreating(false);
    }
  };

  return (
    <Modal
      title={t("settings.memory.addNamespace")}
      open={open}
      onOk={handleCreate}
      onCancel={() => {
        form.resetFields();
        onClose();
      }}
      confirmLoading={creating}
    >
      <Form form={form} layout="vertical">
        <Form.Item name="name" label={t("settings.memory.namespaceName")} rules={[{ required: true }]}>
          <Input name="name" />
        </Form.Item>
        <Form.Item
          name="embeddingProvider"
          label={t("settings.memory.embeddingModel")}
          rules={[{ required: true, message: t("settings.memory.embeddingModelPlaceholder") }]}
        >
          <EmbeddingModelSelect
            value={form.getFieldValue("embeddingProvider")}
            onChange={(val) => form.setFieldValue("embeddingProvider", val)}
            placeholder={t("settings.memory.embeddingModelPlaceholder")}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
}

function CreateWikiModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const createWiki = useLlmWikiStore((s) => s.createWiki);
  const [form] = Form.useForm();
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    try {
      const values = await form.validateFields();
      setCreating(true);
      await createWiki(values.name, values.rootPath, values.description);
      form.resetFields();
      onClose();
    } catch {
      // validation
    } finally {
      setCreating(false);
    }
  };

  return (
    <Modal
      title={t("wiki.llm.createWiki")}
      open={open}
      onOk={handleCreate}
      onCancel={() => {
        form.resetFields();
        onClose();
      }}
      confirmLoading={creating}
    >
      <Form form={form} layout="vertical">
        <Form.Item
          name="name"
          label={t("wiki.wiki.name")}
          rules={[{ required: true, message: t("wiki.llm.nameRequired") }]}
        >
          <Input name="name" placeholder={t("wiki.llm.namePlaceholder")} />
        </Form.Item>
        <Form.Item
          name="rootPath"
          label={t("wiki.wiki.rootPath")}
          rules={[{ required: true, message: t("wiki.llm.pathRequired") }]}
        >
          <Input name="rootPath" placeholder={t("wiki.llm.pathPlaceholder")} />
        </Form.Item>
        <Form.Item name="description" label={t("wiki.wiki.description")}>
          <Input.TextArea name="description" placeholder={t("wiki.llm.descriptionPlaceholder")} />
        </Form.Item>
      </Form>
    </Modal>
  );
}

function SourceCard({
  source,
  onViewConfig,
}: {
  source: UnifiedSource;
  onViewConfig: (s: UnifiedSource) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const meta = TYPE_META[source.containerType];

  const handleView = useCallback(() => {
    switch (source.containerType) {
      case "wiki":
        navigate(`/wiki/${source.id}`);
        break;
      case "knowledge":
        navigate(`/knowledge`);
        break;
      case "memory":
        navigate(`/knowledge`);
        break;
      default:
        break;
    }
  }, [source.containerType, source.id, navigate]);

  return (
    <Card
      hoverable
      size="small"
      style={{ borderRadius: token.borderRadiusLG, overflow: "hidden" }}
      styles={{
        body: { padding: `${token.paddingSM}px ${token.padding}px` },
      }}
    >
      <div className="flex items-start gap-3">
        <div
          className="shrink-0 flex items-center justify-center"
          style={{
            width: 40,
            height: 40,
            borderRadius: token.borderRadius,
            backgroundColor: meta ? `${token[`${meta.color}6` as keyof typeof token]}` : token.colorFillQuaternary,
            color: meta ? token[`${meta.color}1` as keyof typeof token] as string : token.colorTextSecondary,
          }}
        >
          {meta?.icon ?? <Layers size={16} />}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <Text strong ellipsis style={{ fontSize: 14, flex: 1 }}>{source.name}</Text>
            {!source.enabled && <Tag color="default" style={{ fontSize: 10 }}>{t("sourceManager.disabled")}</Tag>}
          </div>
          <div className="flex items-center gap-2 mb-2">
            <TypeBadge containerType={source.containerType} />
            {source.embeddingProvider && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {source.embeddingProvider}
                {source.embeddingDimensions ? ` · ${source.embeddingDimensions}d` : ""}
              </Text>
            )}
          </div>
          {source.description && (
            <Paragraph type="secondary" ellipsis={{ rows: 2 }} style={{ fontSize: 12, marginBottom: 8 }}>
              {source.description}
            </Paragraph>
          )}
          <div className="flex items-center gap-1">
            <Button size="small" type="primary" ghost icon={<Eye size={12} />} onClick={handleView}>
              {t("sourceManager.view")}
            </Button>
            <Button size="small" type="text" icon={<Settings size={12} />} onClick={() => onViewConfig(source)}>
              {t("sourceManager.viewConfig")}
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}

function KnowledgeTab({
  onViewConfig,
}: {
  onViewConfig: (s: UnifiedSource) => void;
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
  const [createOpen, setCreateOpen] = useState(false);

  useEffect(() => {
    loadBases();
  }, [loadBases]);

  const configuredCount = knowledgeSources.filter((s) => s.embeddingProvider).length;

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: token.marginLG }}>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.knowledgeBases")}
              value={bases.length}
              prefix={<Database size={16} style={{ color: token.colorPrimary }} />}
              valueStyle={{ fontSize: 24 }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.documents")}
              value={bases.length}
              prefix={<BookOpen size={16} style={{ color: token.colorInfo }} />}
              valueStyle={{ fontSize: 24 }}
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
              valueStyle={{ fontSize: 24 }}
            />
          </Card>
        </Col>
      </Row>

      <div className="flex items-center justify-between" style={{ marginBottom: token.marginMD }}>
        <Text strong style={{ fontSize: 15 }}>{t("sourceManager.knowledge.title")}</Text>
        <div className="flex items-center gap-2">
          <Button size="small" icon={<Plus size={14} />} onClick={() => setCreateOpen(true)}>
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
            >
              <Button type="primary" icon={<Plus size={14} />} onClick={() => setCreateOpen(true)}>
                {t("settings.knowledge.add")}
              </Button>
            </Empty>
          )
          : (
            <Row gutter={[12, 12]}>
              {knowledgeSources.map((source) => (
                <Col key={source.id} xs={24} sm={12} lg={8}>
                  <SourceCard source={source} onViewConfig={onViewConfig} />
                </Col>
              ))}
            </Row>
          )}

        {bases.length > 0 && (
          <>
            <Divider style={{ margin: `${token.marginLG}px 0` }} />
            <div className="flex items-center justify-between" style={{ marginBottom: token.marginMD }}>
              <Text strong style={{ fontSize: 15 }}>{t("sourceManager.knowledge.recentBases")}</Text>
              <Button size="small" type="link" onClick={() => navigate("/knowledge")}>
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
                          backgroundColor: TYPE_META.knowledge.bgColor,
                          color: TYPE_META.knowledge.fgColor,
                        }}
                      >
                        <Database size={16} />
                      </div>
                      <div className="flex-1 min-w-0">
                        <Text strong ellipsis style={{ fontSize: 13 }}>{base.name}</Text>
                        <div className="flex items-center gap-2 mt-1">
                          <Tag color={base.embeddingProvider ? "green" : "default"} style={{ fontSize: 10, margin: 0 }}>
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

        <CreateKnowledgeBaseModal open={createOpen} onClose={() => setCreateOpen(false)} />
      </Spin>
    </div>
  );
}

function MemoryTab({
  onViewConfig,
}: {
  onViewConfig: (s: UnifiedSource) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const { namespaces, loadNamespaces, loading: memoryLoading } = useMemoryStore();
  const allSources = useSourceStore((s) => s.sources);
  const memorySources = useMemo(
    () => allSources.filter((s) => s.containerType === "memory"),
    [allSources],
  );
  const [createOpen, setCreateOpen] = useState(false);

  useEffect(() => {
    loadNamespaces();
  }, [loadNamespaces]);

  const configuredCount = memorySources.filter((s) => s.embeddingProvider).length;

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: token.marginLG }}>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.namespaces")}
              value={namespaces.length}
              prefix={<Brain size={16} style={{ color: token.colorPrimary }} />}
              valueStyle={{ fontSize: 24 }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.memoryItems")}
              value={namespaces.length}
              prefix={<Sparkles size={16} style={{ color: token.colorPrimary }} />}
              valueStyle={{ fontSize: 24 }}
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
              valueStyle={{ fontSize: 24 }}
            />
          </Card>
        </Col>
      </Row>

      <div className="flex items-center justify-between" style={{ marginBottom: token.marginMD }}>
        <Text strong style={{ fontSize: 15 }}>{t("sourceManager.memory.title")}</Text>
        <div className="flex items-center gap-2">
          <Button size="small" icon={<Plus size={14} />} onClick={() => setCreateOpen(true)}>
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
            >
              <Button type="primary" icon={<Plus size={14} />} onClick={() => setCreateOpen(true)}>
                {t("settings.memory.addNamespace")}
              </Button>
            </Empty>
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
            <div className="flex items-center justify-between" style={{ marginBottom: token.marginMD }}>
              <Text strong style={{ fontSize: 15 }}>{t("sourceManager.memory.namespaces")}</Text>
              <Button size="small" type="link" onClick={() => navigate("/knowledge")}>
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
                        <Text strong ellipsis style={{ fontSize: 13 }}>{ns.name}</Text>
                        <div className="flex items-center gap-2 mt-1">
                          <Tag color={ns.embeddingProvider ? "green" : "default"} style={{ fontSize: 10, margin: 0 }}>
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

        <CreateMemoryNamespaceModal open={createOpen} onClose={() => setCreateOpen(false)} />
      </Spin>
    </div>
  );
}

function WikiTab({
  onViewConfig,
}: {
  onViewConfig: (s: UnifiedSource) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { wikis, loadWikis } = useLlmWikiStore();
  const allSources = useSourceStore((s) => s.sources);
  const wikiSources = useMemo(
    () => allSources.filter((s) => s.containerType === "wiki"),
    [allSources],
  );
  const [createOpen, setCreateOpen] = useState(false);

  useEffect(() => {
    loadWikis();
  }, [loadWikis]);

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
              valueStyle={{ fontSize: 24 }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.notes")}
              value={totalNotes}
              prefix={<BookOpen size={16} style={{ color: token.colorPrimary }} />}
              valueStyle={{ fontSize: 24 }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card size="small" style={{ borderRadius: token.borderRadiusLG }}>
            <Statistic
              title={t("sourceManager.stats.wikiSources")}
              value={totalSources}
              prefix={<FolderPlus size={16} style={{ color: token.colorWarning }} />}
              valueStyle={{ fontSize: 24 }}
            />
          </Card>
        </Col>
      </Row>

      <div className="flex items-center justify-between" style={{ marginBottom: token.marginMD }}>
        <Text strong style={{ fontSize: 15 }}>{t("sourceManager.wiki.title")}</Text>
        <div className="flex items-center gap-2">
          <Button size="small" icon={<Plus size={14} />} onClick={() => setCreateOpen(true)}>
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
          >
            <Button type="primary" icon={<Plus size={14} />} onClick={() => setCreateOpen(true)}>
              {t("wiki.llm.createWiki")}
            </Button>
          </Empty>
        )
        : (
          <>
            {wikiSources.length > 0 && (
              <Row
                gutter={[12, 12]}
                style={{ marginBottom: wikiSources.length > 0 && wikis.length > 0 ? token.marginMD : 0 }}
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
                <div className="flex items-center justify-between" style={{ marginBottom: token.marginMD }}>
                  <Text strong style={{ fontSize: 15 }}>{t("sourceManager.wiki.wikiList")}</Text>
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

      <CreateWikiModal open={createOpen} onClose={() => setCreateOpen(false)} />
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
              <Text strong ellipsis style={{ fontSize: 14, flex: 1 }}>{wiki.name}</Text>
              <Tag color="blue" style={{ fontSize: 10 }}>v{wiki.schemaVersion}</Tag>
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
              <Paragraph type="secondary" ellipsis={{ rows: 1 }} style={{ fontSize: 12, marginBottom: 8 }}>
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
                <Button size="small" type="text" danger icon={<Trash2 size={12} />} />
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
}: {
  onViewConfig: (s: UnifiedSource) => void;
  onNavigateToTab: (tab: string) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { sources, loading, searchAllSources } = useSourceStore();
  const [searchQuery, setSearchQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<UnifiedSource[] | null>(null);

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

  const knowledgeCount = sources.filter((s) => s.containerType === "knowledge").length;
  const memoryCount = sources.filter((s) => s.containerType === "memory").length;
  const wikiCount = sources.filter((s) => s.containerType === "wiki").length;

  return (
    <div>
      <Row gutter={[16, 16]} style={{ marginBottom: token.marginLG }}>
        <Col span={8}>
          <Card
            hoverable
            size="small"
            onClick={() => onNavigateToTab("knowledge")}
            style={{ borderRadius: token.borderRadiusLG, borderColor: token.colorBorder, cursor: "pointer" }}
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
                <Text type="secondary" style={{ fontSize: 12 }}>{t("sourceManager.type.knowledge")}</Text>
                <div>
                  <Text strong style={{ fontSize: 22 }}>{knowledgeCount}</Text>
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
            style={{ borderRadius: token.borderRadiusLG, borderColor: token.colorBorder, cursor: "pointer" }}
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
                <Text type="secondary" style={{ fontSize: 12 }}>{t("sourceManager.type.memory")}</Text>
                <div>
                  <Text strong style={{ fontSize: 22 }}>{memoryCount}</Text>
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
            style={{ borderRadius: token.borderRadiusLG, borderColor: token.colorBorder, cursor: "pointer" }}
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
                <Text type="secondary" style={{ fontSize: 12 }}>{t("sourceManager.type.wiki")}</Text>
                <div>
                  <Text strong style={{ fontSize: 22 }}>{wikiCount}</Text>
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
          <Button type="primary" icon={<Search size={14} />} loading={searching} onClick={handleSearch}>
            {t("sourceManager.search")}
          </Button>
        </Col>
      </Row>

      <Spin spinning={loading}>
        {displaySources.length === 0
          ? <Empty description={t("sourceManager.empty")} style={{ padding: 40 }} />
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
  const [activeTab, setActiveTab] = useState("all");
  const [configSource, setConfigSource] = useState<UnifiedSource | null>(null);

  useEffect(() => {
    fetchSources();
  }, [fetchSources]);

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
              {tab.key === "all" && <AllSourcesTab onViewConfig={setConfigSource} onNavigateToTab={setActiveTab} />}
              {tab.key === "knowledge" && <KnowledgeTab onViewConfig={setConfigSource} />}
              {tab.key === "memory" && <MemoryTab onViewConfig={setConfigSource} />}
              {tab.key === "wiki" && <WikiTab onViewConfig={setConfigSource} />}
            </>
          ),
        }))}
      />

      <SourceConfigModal
        source={configSource}
        open={configSource !== null}
        onClose={() => setConfigSource(null)}
      />
    </div>
  );
}

export { SourceManager };
