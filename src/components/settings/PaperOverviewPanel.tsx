// SPDX-License-Identifier: AGPL-3.0-only

// 论文概览面板
// 对接 usePaperStore，提供概览列表查看、详情展示与删除、Prompt 生成能力

import { message } from "@/lib/toast";
import { useKnowledgeStore, usePaperStore } from "@/stores";
import type { PaperOverview } from "@/types";
import {
  App as AntdApp,
  Button,
  Card,
  Empty,
  List,
  Popconfirm,
  Select,
  Space,
  Spin,
  Tag,
  Timeline,
  Typography,
} from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Paragraph, Text } = Typography;

/** 概览详情卡片 */
function OverviewDetail({ overview }: { overview: PaperOverview }) {
  const { t } = useTranslation();
  const deleteOverview = usePaperStore((s) => s.deleteOverview);
  const generatePrompt = usePaperStore((s) => s.generateOverviewPrompt);
  const [promptLoading, setPromptLoading] = useState(false);

  const handleDelete = useCallback(async () => {
    try {
      await deleteOverview(overview.id);
      message.success(t("paper.deleteSuccess"));
    } catch (e) {
      message.error(String(e));
    }
  }, [overview.id, deleteOverview, t]);

  const handleGeneratePrompt = useCallback(async () => {
    setPromptLoading(true);
    try {
      const prompt = await generatePrompt(overview.documentId);
      // 复制到剪贴板
      await navigator.clipboard.writeText(prompt);
      message.success(t("knowledgeGraph.copySuccess"));
    } catch (e) {
      message.error(String(e));
    } finally {
      setPromptLoading(false);
    }
  }, [overview.documentId, generatePrompt, t]);

  return (
    <Card
      size="small"
      title={
        <Space>
          <Text strong>{t("paper.overview")}</Text>
          <Tag color="blue">{overview.overviewType}</Tag>
          {overview.generatedBy && <Tag>{overview.generatedBy}</Tag>}
        </Space>
      }
      extra={
        <Space>
          <Button size="small" loading={promptLoading} onClick={handleGeneratePrompt}>
            {t("paper.generatePrompt")}
          </Button>
          <Popconfirm
            title={t("paper.deleteConfirm")}
            okText={t("common.confirm")}
            cancelText={t("common.cancel")}
            onConfirm={handleDelete}
          >
            <Button size="small" danger>
              {t("paper.delete")}
            </Button>
          </Popconfirm>
        </Space>
      }
    >
      {overview.tlDr && (
        <div style={{ marginBottom: 12 }}>
          <Text strong>{t("paper.tlDr")}:</Text>
          <Paragraph style={{ display: "inline", marginBottom: 0 }}>
            {overview.tlDr}
          </Paragraph>
        </div>
      )}
      {overview.abstractText && (
        <div style={{ marginBottom: 12 }}>
          <Text strong>{t("paper.abstract")}</Text>
          <Paragraph
            type="secondary"
            ellipsis={{ rows: 4, expandable: true, symbol: t("common.expand") }}
          >
            {overview.abstractText}
          </Paragraph>
        </div>
      )}
      {overview.keyConcepts.length > 0 && (
        <div style={{ marginBottom: 12 }}>
          <Text strong>{t("paper.keyConcepts")}</Text>
          <div style={{ marginTop: 4 }}>
            {overview.keyConcepts.map((c, i) => (
              <Tag key={i} color="geekblue" style={{ marginBottom: 4 }}>
                {c}
              </Tag>
            ))}
          </div>
        </div>
      )}
      {overview.methods.length > 0 && (
        <div style={{ marginBottom: 12 }}>
          <Text strong>{t("paper.methods")}</Text>
          <ul style={{ margin: "4px 0 0 20px" }}>
            {overview.methods.map((m, i) => (
              <li key={i}>
                <Text type="secondary">{m}</Text>
              </li>
            ))}
          </ul>
        </div>
      )}
      {overview.contributions.length > 0 && (
        <div style={{ marginBottom: 12 }}>
          <Text strong>{t("paper.contributions")}</Text>
          <ul style={{ margin: "4px 0 0 20px" }}>
            {overview.contributions.map((c, i) => (
              <li key={i}>
                <Text type="secondary">{c}</Text>
              </li>
            ))}
          </ul>
        </div>
      )}
      {overview.limitations.length > 0 && (
        <div style={{ marginBottom: 12 }}>
          <Text strong>{t("paper.limitations")}</Text>
          <ul style={{ margin: "4px 0 0 20px" }}>
            {overview.limitations.map((l, i) => (
              <li key={i}>
                <Text type="secondary">{l}</Text>
              </li>
            ))}
          </ul>
        </div>
      )}
      {overview.sections.length > 0 && (
        <div style={{ marginBottom: 12 }}>
          <Text strong>{t("paper.sections")}</Text>
          <Timeline
            style={{ marginTop: 8 }}
            items={overview.sections.map((s) => ({
              children: (
                <div>
                  <Text strong>{s.title}</Text>
                  <br />
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {s.summary}
                  </Text>
                </div>
              ),
            }))}
          />
        </div>
      )}
      <div style={{ marginTop: 8, fontSize: 12, color: "var(--ant-color-text-tertiary)" }}>
        <Space split={<span>·</span>}>
          <span>
            {t("paper.createdAt")}: {new Date(overview.createdAt).toLocaleString()}
          </span>
          <span>
            {t("paper.updatedAt")}: {new Date(overview.updatedAt).toLocaleString()}
          </span>
        </Space>
      </div>
    </Card>
  );
}

export function PaperOverviewPanel() {
  const { t } = useTranslation();
  const { modal } = AntdApp.useApp();
  const { bases, loadBases } = useKnowledgeStore();
  const { overviews, loading, loadOverviewsByKb, clearCurrent } = usePaperStore();
  const [selectedKbId, setSelectedKbId] = useState<string | null>(null);
  const [selectedOverviewId, setSelectedOverviewId] = useState<string | null>(null);

  // 首次进入加载知识库列表
  useEffect(() => {
    if (bases.length === 0) {
      loadBases();
    }
  }, [bases.length, loadBases]);

  // 退出时清空状态
  useEffect(() => {
    return () => {
      clearCurrent();
    };
  }, [clearCurrent]);

  // 选中知识库后加载概览
  useEffect(() => {
    if (selectedKbId) {
      loadOverviewsByKb(selectedKbId);
      setSelectedOverviewId(null);
    }
  }, [selectedKbId, loadOverviewsByKb]);

  const kbOptions = useMemo(
    () =>
      bases
        .filter((b) => b.enabled)
        .map((b) => ({ label: b.name, value: b.id })),
    [bases],
  );

  const selectedOverview = useMemo(
    () => overviews.find((o) => o.id === selectedOverviewId) ?? null,
    [overviews, selectedOverviewId],
  );

  const handleRegenerate = useCallback(
    (overview: PaperOverview) => {
      modal.confirm({
        title: t("paper.regenerate"),
        content: t("paper.generatePromptConfirm"),
        okText: t("common.confirm"),
        cancelText: t("common.cancel"),
        onOk: async () => {
          try {
            const prompt = await usePaperStore.getState().generateOverviewPrompt(
              overview.documentId,
            );
            await navigator.clipboard.writeText(prompt);
            message.success(t("knowledgeGraph.copySuccess"));
          } catch (e) {
            message.error(String(e));
          }
        },
      });
    },
    [t],
  );

  return (
    <div className="paper-overview-panel">
      <SettingsGroup title={t("paper.title")}>
        <div style={{ padding: 4 }}>
          <Space direction="vertical" style={{ width: "100%" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Text style={{ width: 100 }}>{t("knowledgeGraph.title")}</Text>
              <Select
                style={{ flex: 1 }}
                placeholder={t("knowledgeGraph.title")}
                value={selectedKbId ?? undefined}
                onChange={setSelectedKbId}
                options={kbOptions}
                showSearch
                optionFilterProp="label"
              />
            </div>
          </Space>
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("paper.overview")}>
        <div style={{ padding: 4 }}>
          {!selectedKbId
            ? <Empty description={t("knowledgeGraph.searchPlaceholder")} />
            : loading
            ? (
              <div style={{ display: "flex", justifyContent: "center", padding: 24 }}>
                <Spin />
              </div>
            )
            : overviews.length === 0
            ? <Empty description={t("paper.empty")} />
            : (
              <List
                dataSource={overviews}
                rowKey={(o) => o.id}
                renderItem={(overview) => (
                  <List.Item
                    style={{
                      cursor: "pointer",
                      background: overview.id === selectedOverviewId
                        ? "var(--ant-color-fill-quaternary, #fafafa)"
                        : undefined,
                    }}
                    onClick={() => setSelectedOverviewId(overview.id)}
                  >
                    <List.Item.Meta
                      title={
                        <Space>
                          <Text strong>{overview.documentId}</Text>
                          <Tag>{overview.overviewType}</Tag>
                        </Space>
                      }
                      description={
                        <Space direction="vertical" size={0}>
                          {overview.tlDr && (
                            <Text type="secondary" style={{ fontSize: 12 }}>
                              {overview.tlDr}
                            </Text>
                          )}
                          <Space size={4}>
                            {overview.keyConcepts.slice(0, 3).map((c, i) => (
                              <Tag key={i} style={{ fontSize: 11 }}>
                                {c}
                              </Tag>
                            ))}
                            {overview.keyConcepts.length > 3 && (
                              <Tag style={{ fontSize: 11 }}>
                                +{overview.keyConcepts.length - 3}
                              </Tag>
                            )}
                          </Space>
                          <Text type="secondary" style={{ fontSize: 11 }}>
                            {new Date(overview.updatedAt).toLocaleString()}
                          </Text>
                        </Space>
                      }
                    />
                    <Button
                      size="small"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleRegenerate(overview);
                      }}
                    >
                      {t("paper.regenerate")}
                    </Button>
                  </List.Item>
                )}
              />
            )}
        </div>
      </SettingsGroup>

      {selectedOverview && (
        <SettingsGroup title={t("paper.view")}>
          <div style={{ padding: 4 }}>
            <OverviewDetail overview={selectedOverview} />
          </div>
        </SettingsGroup>
      )}
    </div>
  );
}
