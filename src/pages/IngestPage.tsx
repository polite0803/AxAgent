import { IngestPanel } from "@/components/wiki/IngestPanel";
import { useLlmWikiStore, WikiSource } from "@/stores/feature/llmWikiStore";
import {
  DeleteOutlined,
  FileTextOutlined,
  FolderOutlined,
  HistoryOutlined,
  LeftOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import { Button, Card, message, Select, Space, Spin, Table, Tabs, Tag, theme, Typography } from "antd";
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
  } = useLlmWikiStore();

  const [activeTab, setActiveTab] = useState("upload");
  const [selectedWikiIdState, setSelectedWikiIdState] = useState<string | null>(
    null,
  );

  useEffect(() => {
    loadWikis();
  }, [loadWikis]);

  useEffect(() => {
    if (wikiIdFromUrl) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setSelectedWikiIdState(wikiIdFromUrl);
      selectWiki(wikiIdFromUrl);
    } else if (wikis.length > 0 && !selectedWikiIdState) {
      setSelectedWikiIdState(wikis[0].id);
      selectWiki(wikis[0].id);
    }
  }, [wikiIdFromUrl, wikis, selectedWikiIdState, selectWiki]);

  const handleBack = () => {
    navigate(-1);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "success";
      case "processing":
        return "processing";
      case "pending":
        return "default";
      case "failed":
        return "error";
      default:
        return "default";
    }
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
      title: t("wiki.ingestSource.status"),
      dataIndex: "status",
      key: "status",
      render: (status: string) => <Tag color={getStatusColor(status)}>{status.toUpperCase()}</Tag>,
    },
    {
      title: t("wiki.ingestSource.chunks"),
      dataIndex: "chunkCount",
      key: "chunkCount",
      render: (count: number) => count || 0,
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
      render: (_: unknown, _record: WikiSource) => (
        <Space>
          <Button
            type="text"
            danger
            icon={<DeleteOutlined />}
            onClick={() => {
              message.info(t("wiki.ingestSource.deleteNotImplemented"));
            }}
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
      <div className="flex items-center gap-4 p-4 border-b">
        <Button icon={<LeftOutlined />} onClick={handleBack} type="text" />
        <Title level={3} className="m-0 flex-1">
          {t("wiki.ingest.title")}
        </Title>
        <Select
          value={displayWikiId}
          onChange={(value) => {
            setSelectedWikiIdState(value);
            selectWiki(value);
          }}
          style={{ width: 200 }}
          placeholder={t("wiki.selectWiki")}
        >
          {wikis.map((wiki) => (
            <Select.Option key={wiki.id} value={wiki.id}>
              {wiki.name}
            </Select.Option>
          ))}
        </Select>
      </div>

      {displayWikiId
        ? (
          <Tabs
            activeKey={activeTab}
            onChange={setActiveTab}
            className="flex-1 px-4 pt-4"
            items={[
              {
                key: "upload",
                label: (
                  <span>
                    <UploadOutlined />
                    {t("wiki.ingest.upload")}
                  </span>
                ),
                children: (
                  <Card>
                    <IngestPanel wikiId={displayWikiId} />
                  </Card>
                ),
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
