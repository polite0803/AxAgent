// SPDX-License-Identifier: AGPL-3.0-only

import { IngestPanel } from "@/components/wiki/IngestPanel";
import { logIpcError } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import type { WikiSource } from "@/types";
import {
  DeleteOutlined,
  FileTextOutlined,
  FolderOutlined,
  HistoryOutlined,
  LeftOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import { Button, Modal, Select, Space, Spin, Table, Tabs, Tag, theme, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";

const { Title, Text } = Typography;

export function IngestPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const { wikiId: wikiIdFromUrl } = useParams<{ wikiId: string }>();

  const {
    wikis,
    selectedWikiId,
    sources,
    loading,
    error,
    loadWikis,
    selectWiki,
    deleteSource,
  } = useLlmWikiStore();

  const [activeTab, setActiveTab] = useState("upload");
  const [selectedWikiIdState, setSelectedWikiIdState] = useState<string | null>(
    null,
  );

  useEffect(() => {
    loadWikis().catch(logIpcError("IngestPage: loadWikis"));
  }, [loadWikis]);

  useEffect(() => {
    if (wikiIdFromUrl) {
      setTimeout(() => setSelectedWikiIdState(wikiIdFromUrl), 0);
      selectWiki(wikiIdFromUrl);
    } else if (wikis.length > 0 && !selectedWikiIdState) {
      setTimeout(() => setSelectedWikiIdState(wikis[0].id), 0);
      selectWiki(wikis[0].id);
    }
  }, [wikiIdFromUrl, wikis, selectedWikiIdState, selectWiki, setSelectedWikiIdState]);

  const handleBack = () => {
    navigate(-1);
  };

  const formatBytes = (bytes: number): string => {
    if (!bytes || bytes <= 0) { return "0 B"; }
    const units = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    const value = bytes / Math.pow(1024, i);
    return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  };

  const getSourceTypeIcon = (sourceType: string) => {
    switch (sourceType.toLowerCase()) {
      case "pdf":
        return <FileTextOutlined />;
      case "docx":
        return <FileTextOutlined />;
      case "folder":
        return <FolderOutlined />;
      default:
        return <FileTextOutlined />;
    }
  };

  const handleDeleteSource = (record: WikiSource) => {
    Modal.confirm({
      title: t("wiki.ingestSource.deleteConfirmTitle"),
      content: t("wiki.ingestSource.deleteConfirmContent", { title: record.title }),
      okText: t("common.confirm"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        const ok = await deleteSource(record.id);
        if (ok) {
          message.success(t("wiki.ingestSource.deleteSuccess"));
        } else {
          message.error(t("wiki.ingestSource.deleteFailed"));
        }
      },
    });
  };

  const columns = [
    {
      title: t("wiki.ingestSource.title"),
      dataIndex: "title",
      key: "title",
    },
    {
      title: t("wiki.ingestSource.type"),
      dataIndex: "sourceType",
      key: "sourceType",
      render: (type: string) => <Tag icon={getSourceTypeIcon(type)}>{type.toUpperCase()}</Tag>,
    },
    {
      title: t("wiki.ingestSource.mimeType"),
      dataIndex: "mimeType",
      key: "mimeType",
      render: (mime: string) => <Tag>{mime || "—"}</Tag>,
    },
    {
      title: t("wiki.ingestSource.size"),
      dataIndex: "sizeBytes",
      key: "sizeBytes",
      render: (size: number) => formatBytes(size),
    },
    {
      title: t("wiki.ingestSource.path"),
      dataIndex: "sourcePath",
      key: "sourcePath",
      ellipsis: true,
    },
    {
      title: t("common.actions"),
      key: "actions",
      render: (_: unknown, record: WikiSource) => (
        <Space>
          <Button
            type="text"
            danger
            icon={<DeleteOutlined />}
            onClick={() => handleDeleteSource(record)}
          />
        </Space>
      ),
    },
  ];

  if (loading && wikis.length === 0) {
    return (
      <div className="h-full flex items-center justify-center">
        <Spin size="large" />
      </div>
    );
  }

  const displayWikiId = selectedWikiIdState || selectedWikiId;

  return (
    <div className="h-full flex flex-col" style={{ overflow: "hidden" }}>
      {error && (
        <div
          className="mx-4 mt-3 p-3 text-sm rounded border"
          style={{
            color: token.colorError,
            backgroundColor: token.colorErrorBg,
            borderColor: token.colorErrorBorder,
          }}
        >
          {error}
        </div>
      )}
      <div className="flex items-center gap-2 px-4 py-2 border-b shrink-0">
        <Button icon={<LeftOutlined />} onClick={handleBack} type="text" size="small" />
        <Title level={5} className="m-0 flex-1">
          {t("wiki.ingest.title")}
        </Title>
        <Select
          size="small"
          value={displayWikiId}
          onChange={(value) => {
            setSelectedWikiIdState(value);
            selectWiki(value);
          }}
          style={{ width: 180 }}
          placeholder={t("wiki.selectWiki")}
          options={wikis.map((w) => ({ label: w.name, value: w.id }))}
        />
      </div>

      {displayWikiId
        ? (
          <Tabs
            activeKey={activeTab}
            onChange={setActiveTab}
            className="ax-fill-tabs px-3 pt-2"
            size="small"
            items={[
              {
                key: "upload",
                label: (
                  <span>
                    <UploadOutlined />
                    {t("wiki.ingest.upload")}
                  </span>
                ),
                children: <IngestPanel wikiId={displayWikiId} />,
              },
              {
                key: "history",
                label: (
                  <span>
                    <HistoryOutlined />
                    {t("wiki.ingest.history")}
                  </span>
                ),
                children: (
                  <Table
                    dataSource={sources.filter((s) => s.wikiId === displayWikiId)}
                    columns={columns}
                    rowKey="id"
                    pagination={{ pageSize: 20 }}
                    loading={loading}
                  />
                ),
              },
            ]}
          />
        )
        : (
          <div className="flex-1 flex items-center justify-center">
            <Text type="secondary">{t("wiki.selectWikiPrompt")}</Text>
          </div>
        )}
    </div>
  );
}
