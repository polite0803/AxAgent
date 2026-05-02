import { IngestPanel } from "@/components/wiki/IngestPanel";
import { LintReport } from "@/components/wiki/LintReport";
import { OperationTimeline } from "@/components/wiki/OperationTimeline";
import { WikiSidebar } from "@/components/wiki/WikiSidebar";
import { useLlmWikiStore, Wiki, WikiSource } from "@/stores/feature/llmWikiStore";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { NoteSearchResult } from "@/types";
import {
  DeleteOutlined,
  EyeOutlined,
  FileTextOutlined,
  FolderOutlined,
  HistoryOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  SyncOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import {
  Button,
  Card,
  Col,
  Descriptions,
  Empty,
  Form,
  Input,
  List,
  message,
  Modal,
  Popconfirm,
  Row,
  Space,
  Statistic,
  Table,
  Tabs,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { ArrowLeft } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { WikiEditorPage } from "./WikiEditorPage";

const { Title } = Typography;

export function LlmWikiPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const wikiIdFromUrl = searchParams.get("wikiId");

  const {
    wikis,
    selectedWikiId,
    sources,
    operations,
    loading,
    error,
    loadWikis,
    selectWiki,
    createWiki,
    deleteWiki,
    compileWiki,
    loadOperations,
  } = useLlmWikiStore();

  const {
    notes,
    selectedNoteId,
    loading: notesLoading,
    loadNotes,
    searchNotes,
    createNote,
    setSelectedNoteId,
  } = useWikiStore();

  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isIngestModalOpen, setIsIngestModalOpen] = useState(false);
  const [isCompiling, setIsCompiling] = useState(false);
  const [activeTab, setActiveTab] = useState("overview");
  const [selectedSourceIds, setSelectedSourceIds] = useState<string[]>([]);
  const [form] = Form.useForm();
  const [notesSearchQuery, setNotesSearchQuery] = useState("");
  const [notesSearchResults, setNotesSearchResults] = useState<NoteSearchResult[]>([]);
  const [isNotesSearching, setIsNotesSearching] = useState(false);

  useEffect(() => {
    loadWikis();
  }, [loadWikis]);

  useEffect(() => {
    if (wikiIdFromUrl && wikiIdFromUrl !== selectedWikiId) {
      selectWiki(wikiIdFromUrl);
    }
  }, [wikiIdFromUrl, selectedWikiId, selectWiki]);

  useEffect(() => {
    if (selectedWikiId) {
      loadOperations(selectedWikiId);
      loadNotes(selectedWikiId);
    }
  }, [selectedWikiId, loadOperations, loadNotes]);

  useEffect(() => {
    if (notesSearchQuery.trim() && selectedWikiId) {
      setIsNotesSearching(true);
      const timer = setTimeout(async () => {
        const results = await searchNotes(selectedWikiId, notesSearchQuery);
        setNotesSearchResults(results);
        setIsNotesSearching(false);
      }, 300);
      return () => clearTimeout(timer);
    } else {
      setNotesSearchResults([]);
      setIsNotesSearching(false);
    }
  }, [notesSearchQuery, selectedWikiId, searchNotes]);

  const displayNotes = notesSearchQuery.trim() ? notesSearchResults.map((r) => r.note) : notes;

  const handleSelectNote = (noteId: string) => {
    setSelectedNoteId(noteId);
  };

  const handleCreateNote = () => {
    if (!selectedWikiId) { return; }
    const now = Date.now();
    createNote({
      vaultId: selectedWikiId,
      title: `Untitled ${new Date(now).toLocaleString()}`,
      filePath: `/untitled-${now}.md`,
      content: "",
      author: "user",
    });
  };

  const handleBackFromNote = () => {
    setSelectedNoteId(null);
    setNotesSearchQuery("");
  };

  const selectedWiki = wikis.find((w) => w.id === selectedWikiId);

  const handleCreateWiki = async (values: { name: string; rootPath: string; description?: string }) => {
    const wiki = await createWiki(values.name, values.rootPath, values.description);
    if (wiki) {
      message.success(t("wiki.llm.createSuccess"));
      setIsCreateModalOpen(false);
      form.resetFields();
      selectWiki(wiki.id);
    }
  };

  const handleDeleteWiki = async (wikiId: string) => {
    await deleteWiki(wikiId);
    message.success(t("wiki.llm.deleteSuccess"));
  };

  const handleCompile = async () => {
    if (!selectedWikiId || selectedSourceIds.length === 0) {
      message.warning(t("wiki.llm.selectSourcesFirst"));
      return;
    }

    setIsCompiling(true);
    try {
      const result = await compileWiki(selectedWikiId, selectedSourceIds);
      if (result) {
        if (result.errors.length > 0) {
          message.error(t("wiki.llm.compileErrors", { count: result.errors.length }));
        } else {
          message.success(
            t("wiki.llm.compileSuccess", {
              newCount: result.new_pages.length,
              updatedCount: result.updated_pages.length,
            }),
          );
        }
        loadOperations(selectedWikiId);
      }
    } finally {
      setIsCompiling(false);
    }
  };

  const sourceColumns = [
    { title: t("wiki.source.title"), dataIndex: "title", key: "title" },
    {
      title: t("wiki.source.type"),
      dataIndex: "sourceType",
      key: "sourceType",
      render: (type: string) => <Tag>{type}</Tag>,
    },
    { title: t("wiki.source.path"), dataIndex: "sourcePath", key: "sourcePath", ellipsis: true },
    {
      title: t("wiki.common.actions"),
      key: "actions",
      render: (_: unknown, record: WikiSource) => (
        <Space>
          <Tooltip title={t("wiki.llm.viewSource")}>
            <Button
              size="small"
              icon={<EyeOutlined />}
              onClick={() => navigate(`/llm-wiki/${record.wikiId}/source/${record.id}`)}
            />
          </Tooltip>
        </Space>
      ),
    },
  ];

  const rowSelection = {
    selectedRowKeys: selectedSourceIds,
    onChange: (keys: React.Key[]) => setSelectedSourceIds(keys as string[]),
  };

  const renderOverview = () => (
    <Row gutter={16} style={{ marginBottom: 24 }}>
      <Col span={6}>
        <Card>
          <Statistic
            title={t("wiki.llm.stats.totalWikis")}
            value={wikis.length}
            prefix={<FolderOutlined />}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title={t("wiki.llm.stats.sources")}
            value={sources.length}
            prefix={<FileTextOutlined />}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title={t("wiki.llm.stats.operations")}
            value={operations.length}
            prefix={<HistoryOutlined />}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title={t("wiki.llm.stats.lastCompile")}
            value={operations.filter((o) => o.operationType === "compile").length}
            prefix={<SyncOutlined spin={isCompiling} />}
          />
        </Card>
      </Col>
    </Row>
  );

  const renderWikiList = () => (
    <Card
      title={t("wiki.llm.wikiList")}
      extra={
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setIsCreateModalOpen(true)}>
          {t("wiki.llm.createWiki")}
        </Button>
      }
    >
      <Table
        dataSource={wikis}
        rowKey="id"
        loading={loading}
        columns={[
          { title: t("wiki.wiki.name"), dataIndex: "name", key: "name" },
          { title: t("wiki.wiki.rootPath"), dataIndex: "rootPath", key: "rootPath", ellipsis: true },
          {
            title: t("wiki.wiki.schemaVersion"),
            dataIndex: "schemaVersion",
            key: "schemaVersion",
            render: (v: string) => <Tag color="blue">v{v}</Tag>,
          },
          {
            title: t("wiki.common.actions"),
            key: "actions",
            render: (_: unknown, record: Wiki) => (
              <Space>
                <Button size="small" type="primary" onClick={() => selectWiki(record.id)}>
                  {t("wiki.llm.select")}
                </Button>
                <Button
                  size="small"
                  icon={<EyeOutlined />}
                  onClick={() => navigate(`/llm-wiki/${record.id}`)}
                />
                <Popconfirm
                  title={t("wiki.llm.confirmDelete")}
                  onConfirm={() => handleDeleteWiki(record.id)}
                >
                  <Button size="small" danger icon={<DeleteOutlined />} />
                </Popconfirm>
              </Space>
            ),
          },
        ]}
      />
    </Card>
  );

  const renderSourcePanel = () => (
    <Card
      title={t("wiki.llm.sources")}
      extra={
        <Space>
          <Button icon={<UploadOutlined />} onClick={() => setIsIngestModalOpen(true)}>
            {t("wiki.llm.ingestSource")}
          </Button>
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={isCompiling}
            disabled={selectedSourceIds.length === 0}
            onClick={handleCompile}
          >
            {t("wiki.llm.compile")}
          </Button>
        </Space>
      }
    >
      <Table
        rowSelection={rowSelection}
        dataSource={sources}
        rowKey="id"
        columns={sourceColumns}
        pagination={{ pageSize: 10 }}
      />
    </Card>
  );

  const renderIngestPanel = () => (
    <Card title={t("wiki.llm.ingestSource")}>
      <IngestPanel wikiId={selectedWikiId || ""} onClose={() => setIsIngestModalOpen(false)} />
    </Card>
  );

  const renderLintPanel = () => (
    <Card title={t("wiki.llm.lintReport")}>
      <LintReport wikiId={selectedWikiId || ""} />
    </Card>
  );

  const renderOperationsPanel = () => (
    <Card title={t("wiki.llm.operations")}>
      <OperationTimeline operations={operations} />
    </Card>
  );

  const renderNotesPanel = () => (
    <div className="flex h-full" style={{ overflow: "hidden" }}>
      <WikiSidebar
        notes={displayNotes}
        selectedNoteId={selectedNoteId}
        onSelectNote={handleSelectNote}
        onCreateNote={handleCreateNote}
        loading={notesLoading}
      />
      <div
        className="flex-1 flex flex-col overflow-hidden border-l"
        style={{ borderColor: "var(--color-border-secondary)" }}
      >
        <div className="p-4 border-b" style={{ borderColor: "var(--color-border-secondary)" }}>
          <Space className="w-full" direction="vertical" size="small">
            <Input.Search
              placeholder={t("wiki.searchPlaceholder", "Search notes...")}
              value={notesSearchQuery}
              onChange={(e) => setNotesSearchQuery(e.target.value)}
              loading={isNotesSearching}
              allowClear
              className="flex-1"
            />
          </Space>
        </div>
        <div className="flex-1 overflow-y-auto p-4">
          {displayNotes.length === 0 ? <Empty description={t("wiki.emptyNotes", "No notes yet")} /> : (
            <List
              dataSource={displayNotes}
              renderItem={(note) => (
                <List.Item
                  onClick={() => handleSelectNote(note.id)}
                  className="cursor-pointer hover:bg-black/5 px-3 py-2 rounded"
                >
                  <List.Item.Meta
                    title={note.title}
                    description={
                      <span className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
                        {note.author === "llm" ? t("wiki.llmNote", "LLM") : t("wiki.userNote", "User")} •{" "}
                        {note.filePath}
                      </span>
                    }
                  />
                </List.Item>
              )}
            />
          )}
        </div>
      </div>
    </div>
  );

  if (!selectedWikiId) {
    return (
      <div style={{ padding: 24 }}>
        <Title level={4}>{t("wiki.llm.title")}</Title>
        {error && (
          <div className="mb-3 p-3 text-sm text-red-600 bg-red-50 border border-red-200 rounded">
            {error}
          </div>
        )}
        {renderWikiList()}

        <Modal
          title={t("wiki.llm.createWiki")}
          open={isCreateModalOpen}
          onCancel={() => setIsCreateModalOpen(false)}
          footer={null}
        >
          <Form form={form} layout="vertical" onFinish={handleCreateWiki}>
            <Form.Item
              name="name"
              label={t("wiki.wiki.name")}
              rules={[{ required: true, message: t("wiki.llm.nameRequired") }]}
            >
              <Input placeholder={t("wiki.llm.namePlaceholder")} />
            </Form.Item>
            <Form.Item
              name="rootPath"
              label={t("wiki.wiki.rootPath")}
              rules={[{ required: true, message: t("wiki.llm.pathRequired") }]}
            >
              <Input placeholder={t("wiki.llm.pathPlaceholder")} />
            </Form.Item>
            <Form.Item name="description" label={t("wiki.wiki.description")}>
              <Input.TextArea placeholder={t("wiki.llm.descriptionPlaceholder")} />
            </Form.Item>
            <Button type="primary" htmlType="submit" loading={loading} block>
              {t("wiki.llm.create")}
            </Button>
          </Form>
        </Modal>
      </div>
    );
  }

  return (
    <div style={{ padding: 24 }}>
      <Card style={{ marginBottom: 16 }}>
        <Descriptions
          title={
            <Space>
              <FolderOutlined />
              <span>{selectedWiki?.name}</span>
              <Tag color="blue">v{selectedWiki?.schemaVersion}</Tag>
            </Space>
          }
          extra={
            <Space>
              <Button icon={<HistoryOutlined />} onClick={() => navigate(`/llm-wiki/${selectedWikiId}/graph`)}>
                {t("wiki.graph.title")}
              </Button>
              <Button onClick={() => selectWiki(null)}>{t("wiki.llm.backToList")}</Button>
            </Space>
          }
        >
          <Descriptions.Item label={t("wiki.wiki.rootPath")}>{selectedWiki?.rootPath}</Descriptions.Item>
          <Descriptions.Item label={t("wiki.wiki.description")}>
            {selectedWiki?.description || "-"}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      {selectedNoteId
        ? (
          <div>
            <Button icon={<ArrowLeft />} onClick={handleBackFromNote} className="mb-2">
              {t("wiki.backToNotes", "Back to Notes")}
            </Button>
            <WikiEditorPage noteId={selectedNoteId} onBack={handleBackFromNote} />
          </div>
        )
        : (
          <Tabs
            activeKey={activeTab}
            onChange={setActiveTab}
            items={[
              { key: "overview", label: t("wiki.common.overview"), children: renderOverview() },
              { key: "notes", label: t("wiki.notes", "Notes"), children: renderNotesPanel() },
              { key: "sources", label: t("wiki.llm.sources"), children: renderSourcePanel() },
              { key: "ingest", label: t("wiki.llm.ingestSource"), children: renderIngestPanel() },
              { key: "lint", label: t("wiki.llm.lintReport"), children: renderLintPanel() },
              { key: "operations", label: t("wiki.llm.operations"), children: renderOperationsPanel() },
            ]}
          />
        )}

      <Modal
        title={t("wiki.llm.ingestSource")}
        open={isIngestModalOpen}
        onCancel={() => setIsIngestModalOpen(false)}
        footer={null}
        width={600}
      >
        <IngestPanel wikiId={selectedWikiId} onClose={() => setIsIngestModalOpen(false)} />
      </Modal>
    </div>
  );
}
