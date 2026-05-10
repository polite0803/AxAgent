import { useSourceStore } from "@/stores";
import type { SourceConfig, UnifiedSource } from "@/stores/feature/sourceStore";
import { Button, Col, Descriptions, Empty, Input, List, Modal, Row, Spin, Tabs, Tag, theme, Typography } from "antd";
import { BookOpen, Brain, Network, Search, Settings } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const TYPE_META: Record<string, { color: string; icon: React.ReactNode; labelKey: string }> = {
  knowledge: { color: "blue", icon: <BookOpen size={14} />, labelKey: "sourceManager.type.knowledge" },
  memory: { color: "purple", icon: <Brain size={14} />, labelKey: "sourceManager.type.memory" },
  wiki: { color: "green", icon: <Network size={14} />, labelKey: "sourceManager.type.wiki" },
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
      title={
        <span>
          <Settings size={16} style={{ marginRight: token.marginXS, verticalAlign: "middle" }} />
          {source?.name ?? ""} — {t("sourceManager.configTitle")}
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

function SourceListItem({
  source,
  onViewConfig,
}: {
  source: UnifiedSource;
  onViewConfig: (s: UnifiedSource) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <List.Item
      style={{ padding: `${token.paddingSM}px ${token.padding}px` }}
      actions={[
        <Button
          key="config"
          type="text"
          size="small"
          icon={<Settings size={14} />}
          onClick={() => onViewConfig(source)}
        >
          {t("sourceManager.viewConfig")}
        </Button>,
      ]}
    >
      <List.Item.Meta
        title={
          <span>
            {source.name}
            {!source.enabled && (
              <Tag color="default" style={{ marginLeft: token.marginXS }}>
                {t("sourceManager.disabled")}
              </Tag>
            )}
          </span>
        }
        description={
          <span>
            <TypeBadge containerType={source.containerType} />
            {source.description && (
              <Text type="secondary" style={{ marginLeft: token.marginXS }}>
                {source.description}
              </Text>
            )}
          </span>
        }
      />
      {source.embeddingProvider && (
        <div style={{ textAlign: "right" }}>
          <Text type="secondary" style={{ fontSize: token.fontSizeSM }}>
            {source.embeddingProvider}
            {source.embeddingDimensions ? ` · ${source.embeddingDimensions}d` : ""}
          </Text>
        </div>
      )}
    </List.Item>
  );
}

function SourceManager() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { sources, loading, fetchSources, searchAllSources } = useSourceStore();

  const [activeTab, setActiveTab] = useState("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<UnifiedSource[] | null>(null);
  const [configSource, setConfigSource] = useState<UnifiedSource | null>(null);

  useEffect(() => {
    fetchSources();
  }, [fetchSources]);

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

  const filteredSources = (() => {
    const list = searchResults ?? sources;
    if (activeTab === "all") { return list; }
    return list.filter((s) => s.containerType === activeTab);
  })();

  const tabItems = [
    { key: "all", label: t("sourceManager.tab.all"), children: undefined as React.ReactNode | undefined },
    { key: "knowledge", label: t("sourceManager.tab.knowledge"), children: undefined },
    { key: "memory", label: t("sourceManager.tab.memory"), children: undefined },
    { key: "wiki", label: t("sourceManager.tab.wiki"), children: undefined },
  ];

  return (
    <div style={{ padding: token.paddingLG }}>
      <Row gutter={[12, 12]} style={{ marginBottom: token.marginMD }}>
        <Col flex="auto">
          <Input
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

      <Tabs
        activeKey={activeTab}
        onChange={(key) => {
          setActiveTab(key);
          setSearchResults(null);
          setSearchQuery("");
        }}
        items={tabItems.map((tab) => ({
          ...tab,
          children: (
            <Spin spinning={loading}>
              {filteredSources.length === 0 ? <Empty description={t("sourceManager.empty")} /> : (
                <List
                  bordered
                  dataSource={filteredSources}
                  renderItem={(source) => <SourceListItem source={source} onViewConfig={setConfigSource} />}
                />
              )}
            </Spin>
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

export default SourceManager;
