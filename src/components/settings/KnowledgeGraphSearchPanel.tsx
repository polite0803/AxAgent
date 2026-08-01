// SPDX-License-Identifier: AGPL-3.0-only

// LightRAG 知识图谱搜索面板
// 对接 useKnowledgeGraphStore，提供实体查询与上下文复制能力

import { List } from "@/components/common/AntdList";
import { message } from "@/lib/toast";
import { useKnowledgeGraphStore, useKnowledgeStore } from "@/stores";
import type { GraphEnhancedContextChunk } from "@/types";
import { Button, Empty, InputNumber, Select, Space, Spin, Switch, Tag, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Paragraph, Text } = Typography;

/** 单个实体卡片 */
function EntityCard({ entity }: { entity: GraphEnhancedContextChunk }) {
  const { t } = useTranslation();
  return (
    <div
      style={{
        border: "1px solid var(--ant-color-border-secondary, #f0f0f0)",
        borderRadius: 8,
        padding: 12,
        marginBottom: 8,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
        <Text strong>{entity.entityName}</Text>
        <Tag color="blue">{entity.entityType}</Tag>
      </div>
      {entity.description && (
        <Paragraph
          type="secondary"
          ellipsis={{ rows: 3, expandable: true, symbol: t("common.expand") }}
          style={{ marginBottom: 8, fontSize: 12 }}
        >
          {entity.description}
        </Paragraph>
      )}
      {entity.relations.length > 0 && (
        <div style={{ marginTop: 6 }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("knowledgeGraph.relations")} ({entity.relations.length})
          </Text>
          <div style={{ marginTop: 4 }}>
            {entity.relations.slice(0, 5).map((rel, idx) => (
              <Tag key={idx} style={{ marginBottom: 4 }}>
                {rel.relationType} → {rel.targetEntityName}
              </Tag>
            ))}
            {entity.relations.length > 5 && <Tag>+{entity.relations.length - 5}</Tag>}
          </div>
        </div>
      )}
    </div>
  );
}

export function KnowledgeGraphSearchPanel() {
  const { t } = useTranslation();
  const { bases, loadBases } = useKnowledgeStore();
  const {
    searchResult,
    loading,
    graphEnhancedSearch,
    clearResults,
    clearError,
  } = useKnowledgeGraphStore();

  const [knowledgeBaseId, setKnowledgeBaseId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [topK, setTopK] = useState<number>(10);
  const [includeNeighbors, setIncludeNeighbors] = useState(true);

  // 首次进入加载知识库列表
  useEffect(() => {
    if (bases.length === 0) {
      loadBases();
    }
  }, [bases.length, loadBases]);

  // 退出时清空结果，避免下次进来看到旧数据
  useEffect(() => {
    return () => {
      clearResults();
    };
  }, [clearResults]);

  const kbOptions = useMemo(
    () =>
      bases
        .filter((b) => b.enabled)
        .map((b) => ({ label: b.name, value: b.id })),
    [bases],
  );

  const handleSearch = useCallback(async () => {
    if (!knowledgeBaseId || !query.trim()) {
      return;
    }
    clearError();
    await graphEnhancedSearch({
      knowledgeBaseId,
      query: query.trim(),
      topK,
      includeNeighbors,
    });
  }, [knowledgeBaseId, query, topK, includeNeighbors, graphEnhancedSearch, clearError]);

  const handleCopyContext = useCallback(async () => {
    if (!searchResult?.contextText) {
      return;
    }
    try {
      await navigator.clipboard.writeText(searchResult.contextText);
      message.success(t("knowledgeGraph.copySuccess"));
    } catch {
      message.error(t("artifact.copyFailed"));
    }
  }, [searchResult, t]);

  const entities = searchResult?.entities ?? [];

  return (
    <div className="knowledge-graph-panel">
      <SettingsGroup title={t("knowledgeGraph.search")}>
        <div style={{ display: "flex", flexDirection: "column", gap: 12, padding: 4 }}>
          <Space orientation="vertical" style={{ width: "100%" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Text style={{ width: 100 }}>{t("knowledgeGraph.title")}</Text>
              <Select
                style={{ flex: 1 }}
                placeholder={t("knowledgeGraph.title")}
                value={knowledgeBaseId ?? undefined}
                onChange={setKnowledgeBaseId}
                options={kbOptions}
                showSearch
                optionFilterProp="label"
              />
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Text style={{ width: 100 }}>{t("knowledgeGraph.searchPlaceholder")}</Text>
              <input
                className="ant-input"
                style={{ flex: 1, padding: "4px 11px" }}
                placeholder={t("knowledgeGraph.searchPlaceholder")}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    handleSearch();
                  }
                }}
              />
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap" }}>
              <Space>
                <Text>{t("knowledgeGraph.topK")}</Text>
                <InputNumber
                  min={1}
                  max={50}
                  value={topK}
                  onChange={(v) => v && setTopK(v)}
                  style={{ width: 80 }}
                />
              </Space>
              <Space>
                <Text>{t("knowledgeGraph.includeNeighbors")}</Text>
                <Switch checked={includeNeighbors} onChange={setIncludeNeighbors} />
              </Space>
              <Button type="primary" onClick={handleSearch} loading={loading}>
                {t("knowledgeGraph.searchButton")}
              </Button>
            </div>
          </Space>
        </div>
      </SettingsGroup>

      <SettingsGroup
        title={t("knowledgeGraph.results")}
        extra={searchResult && searchResult.totalHits > 0
          ? (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("knowledgeGraph.totalHits", { count: searchResult.totalHits })}
            </Text>
          )
          : null}
      >
        <div style={{ padding: 4 }}>
          {loading
            ? (
              <div style={{ display: "flex", justifyContent: "center", padding: 24 }}>
                <Spin />
              </div>
            )
            : entities.length === 0
            ? <Empty description={t("knowledgeGraph.empty")} />
            : (
              <List
                dataSource={entities}
                renderItem={(entity) => <EntityCard entity={entity} />}
                rowKey={(item) => `${item.knowledgeBaseId}:${item.entityName}`}
              />
            )}
          {searchResult?.contextText && (
            <div style={{ marginTop: 12 }}>
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: 4,
                }}
              >
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("knowledgeGraph.contextText")}
                </Text>
                <Button size="small" onClick={handleCopyContext}>
                  {t("knowledgeGraph.copyContext")}
                </Button>
              </div>
              <Paragraph
                style={{
                  fontSize: 12,
                  padding: 8,
                  background: "var(--ant-color-fill-quaternary, #fafafa)",
                  borderRadius: 6,
                  maxHeight: 200,
                  overflow: "auto",
                  whiteSpace: "pre-wrap",
                }}
              >
                {searchResult.contextText}
              </Paragraph>
            </div>
          )}
        </div>
      </SettingsGroup>
    </div>
  );
}
