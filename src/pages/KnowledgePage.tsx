import KnowledgeSettings from "@/components/settings/KnowledgeSettings";
import { IngestPanel } from "@/components/wiki/IngestPanel";
import { WikiSidebar } from "@/components/wiki/WikiSidebar";
import { useKnowledgeStore } from "@/stores";
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
import { open } from "@tauri-apps/plugin-dialog";
import { theme } from "antd";
import {
  Button,
  Card,
  Checkbox,
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
import { ArrowLeft, Database, FolderPlus, Library } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { WikiEditorPage } from "./WikiEditorPage";

const { Title } = Typography;

export function KnowledgePage() {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const navigate = useNavigate();

  const [leftPanelTab, setLeftPanelTab] = useState<"ingest" | "sources">("ingest");
  const [rightPanelTab, setRightPanelTab] = useState<"rag" | "wiki">("rag");

  const {
    wikis,
    selectedWikiId,
    sources,
    operations,
    loading: llmWikiLoading,
    error: llmWikiError,
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
    createNote,
    setSelectedNoteId,
  } = useWikiStore();

  const {
    bases,
    selectedBaseId,
  } = useKnowledgeStore();

  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isIngestModalOpen, setIsIngestModalOpen] = useState(false);
  const [isCompiling, setIsCompiling] = useState(false);
  const [activeTab, setActiveTab] = useState("overview");
  const [selectedSourceIds, setSelectedSourceIds] = useState<string[]>([]);
  const [form] = Form.useForm();
  const [notesSearchQuery, setNotesSearchQuery] = useState("");
  const [notesSearchResults] = useState<NoteSearchResult[]>([]);

  const [isImportModalOpen, setIsImportModalOpen] = useState(false);
  const [selectedFolderPath, setSelectedFolderPath] = useState<string>("");
  const [importToRAG, setImportToRAG] = useState(true);
  const [importToWiki, setImportToWiki] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  const [messageApi, contextHolder] = message.useMessage();

  const selectedWiki = wikis.find((w) => w.id === selectedWikiId);

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

  const handleSelectFolder = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) { return; }
      setSelectedFolderPath(selected as string);
      setIsImportModalOpen(true);
    } catch (e) {
      messageApi.error(t("common.error", "操作失败") + ": " + String(e));
    }
  };

  const handleImport = async () => {
    if (!selectedFolderPath) {
      messageApi.warning(t("knowledge.selectFolderFirst", "请先选择文件夹"));
      return;
    }

    if (!importToRAG && !importToWiki) {
      messageApi.warning(t("knowledge.selectDestination", "请选择导入目标"));
      return;
    }

    if (importToRAG && !selectedBaseId) {
      messageApi.warning(t("knowledge.selectKnowledgeBaseFirst", "请先选择一个知识库"));
      return;
    }

    if (importToWiki && !selectedWikiId) {
      messageApi.warning(t("wiki.llm.selectWikiFirst", "请先选择一个Wiki"));
      return;
    }

    setIsImporting(true);

    try {
      let ragSuccess = false;
      let wikiSuccess = false;

      if (importToRAG && selectedBaseId) {
        const { invoke: invokeFn } = await import("@/lib/invoke");
        await invokeFn("add_knowledge_document", {
          baseId: selectedBaseId,
          title: selectedFolderPath.split(/[/\\]/).pop() || "Folder",
          sourcePath: selectedFolderPath,
          mimeType: "text/folder",
        });
        ragSuccess = true;
      }

      if (importToWiki && selectedWikiId) {
        const { ingestSource } = useLlmWikiStore.getState();
        await ingestSource(
          selectedWikiId,
          "folder",
          selectedFolderPath,
          undefined,
          selectedFolderPath.split(/[/\\]/).pop(),
        );
        wikiSuccess = true;
      }

      if (ragSuccess || wikiSuccess) {
        messageApi.success(t("knowledge.importSuccess", "导入成功"));
        setIsImportModalOpen(false);
        setSelectedFolderPath("");
      }
    } catch (e) {
      messageApi.error(t("common.error", "导入失败") + ": " + String(e));
    } finally {
      setIsImporting(false);
    }
  };

  const handleSelectWiki = (wikiId: string) => {
    selectWiki(wikiId);
    setRightPanelTab("wiki");
  };

  const handleCreateWiki = async (values: { name: string; rootPath: string; description?: string }) => {
    const wiki = await createWiki(values.name, values.rootPath, values.description);
    if (wiki) {
      messageApi.success(t("wiki.llm.createSuccess"));
      setIsCreateModalOpen(false);
      form.resetFields();
      selectWiki(wiki.id);
    }
  };

  const handleDeleteWiki = async (wikiId: string) => {
    await deleteWiki(wikiId);
    messageApi.success(t("wiki.llm.deleteSuccess"));
  };

  const handleCompile = async () => {
    if (!selectedWikiId || selectedSourceIds.length === 0) {
      messageApi.warning(t("wiki.llm.selectSourcesFirst"));
      return;
    }

    setIsCompiling(true);
    try {
      const result = await compileWiki(selectedWikiId, selectedSourceIds);
      if (result) {
        if (result.errors.length > 0) {
          messageApi.error(t("wiki.llm.compileErrors", { count: result.errors.length }));
        } else {
          messageApi.success(
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
        loading={llmWikiLoading}
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
                <Button size="small" type="primary" onClick={() => handleSelectWiki(record.id)}>
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
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <div className="p-4 border-b" style={{ borderColor: token.colorBorderSecondary }}>
          <Space className="w-full" direction="vertical" size="small">
            <Input.Search
              placeholder={t("wiki.searchPlaceholder", "Search notes...")}
              value={notesSearchQuery}
              onChange={(e) => setNotesSearchQuery(e.target.value)}
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
                      <span className="text-xs" style={{ color: token.colorTextSecondary }}>
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

  const renderWikiContent = () => {
    if (!selectedWikiId) {
      return (
        <div style={{ padding: 24 }}>
          {llmWikiError && (
            <div className="mb-3 p-3 text-sm text-red-600 bg-red-50 border border-red-200 rounded">
              {llmWikiError}
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
              <Button type="primary" htmlType="submit" loading={llmWikiLoading} block>
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

        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={[
            { key: "overview", label: t("wiki.common.overview"), children: renderOverview() },
            { key: "notes", label: t("wiki.notes", "Notes"), children: renderNotesPanel() },
            { key: "sources", label: t("wiki.llm.sources"), children: renderSourcePanel() },
          ]}
        />

        <Modal
          title={t("wiki.llm.ingestSource")}
          open={isIngestModalOpen}
          onCancel={() => setIsIngestModalOpen(false)}
          footer={null}
          width={600}
        >
          <IngestPanel wikiId={selectedWikiId} onClose={() => setIsIngestModalOpen(false)} />
        </Modal>

        <Modal
          title={t("knowledge.importTitle", "导入文档")}
          open={isImportModalOpen}
          onCancel={() => setIsImportModalOpen(false)}
          footer={null}
          width={500}
        >
          <div className="py-4">
            <div className="mb-4">
              <div className="text-sm font-medium mb-2">{t("knowledge.selectedFolder", "已选择文件夹")}</div>
              <Input value={selectedFolderPath} disabled prefix={<FolderOutlined />} />
            </div>

            <div className="mb-4">
              <div className="text-sm font-medium mb-2">{t("knowledge.importDestination", "导入到")}</div>
              <Checkbox
                checked={importToRAG}
                onChange={(e) => setImportToRAG(e.target.checked)}
                className="block mb-2"
              >
                {t("knowledge.ragSystem", "RAG 知识库")}
                {selectedBaseId && (
                  <span className="text-xs text-gray-500 ml-2">
                    ({bases.find((b) => b.id === selectedBaseId)?.name || selectedBaseId})
                  </span>
                )}
              </Checkbox>
              <Checkbox
                checked={importToWiki}
                onChange={(e) => setImportToWiki(e.target.checked)}
                className="block"
              >
                {t("knowledge.wikiSystem", "Wiki 系统")}
                {selectedWikiId && (
                  <span className="text-xs text-gray-500 ml-2">
                    ({wikis.find((w) => w.id === selectedWikiId)?.name || selectedWikiId})
                  </span>
                )}
              </Checkbox>
            </div>

            <div className="flex justify-end gap-2">
              <Button onClick={() => setIsImportModalOpen(false)}>
                {t("common.cancel", "取消")}
              </Button>
              <Button type="primary" loading={isImporting} onClick={handleImport}>
                {t("knowledge.import", "导入")}
              </Button>
            </div>
          </div>
        </Modal>
      </div>
    );
  };

  const renderRAGContent = () => <KnowledgeSettings />;

  return (
    <div className="h-full flex" style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}>
      {contextHolder}
      <aside
        className="w-64 shrink-0 border-r flex flex-col"
        style={{ backgroundColor: token.colorBgContainer, borderColor: token.colorBorder }}
      >
        <div className="p-4 border-b" style={{ borderColor: token.colorBorder }}>
          <Title level={5} style={{ margin: 0 }}>
            {t("knowledge.commonPanel", "公共面板")}
          </Title>
          <p className="text-xs mt-1" style={{ color: token.colorTextSecondary }}>
            {t("knowledge.commonPanelDesc", "知识库和Wiki的共同操作")}
          </p>
        </div>

        <Tabs
          activeKey={leftPanelTab}
          onChange={(key) => setLeftPanelTab(key as "ingest" | "sources")}
          size="small"
          style={{ flex: 1, display: "flex", flexDirection: "column" }}
          tabBarStyle={{ padding: "0 12px", marginBottom: 0 }}
          items={[
            {
              key: "ingest",
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                  <FolderPlus size={14} />
                  {t("knowledge.loadFiles", "加载文件")}
                </span>
              ),
              children: (
                <div className="p-3 flex-1 overflow-y-auto">
                  <p className="text-xs mb-3" style={{ color: token.colorTextSecondary }}>
                    {t("knowledge.loadFilesDesc", "从文件夹加载文档到知识库或Wiki")}
                  </p>
                  <Button
                    icon={<FolderPlus size={14} />}
                    onClick={handleSelectFolder}
                    block
                    style={{ marginBottom: 12 }}
                  >
                    {t("knowledge.selectFolder", "选择文件夹")}
                  </Button>
                  <Button
                    icon={<UploadOutlined size={14} />}
                    onClick={() => setRightPanelTab("rag")}
                    block
                  >
                    {t("knowledge.loadToRAG", "加载到RAG")}
                  </Button>
                </div>
              ),
            },
            {
              key: "sources",
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                  <Database size={14} />
                  {t("knowledge.sources", "数据源")}
                </span>
              ),
              children: (
                <div className="p-3 flex-1 overflow-y-auto">
                  <p className="text-xs mb-3" style={{ color: token.colorTextSecondary }}>
                    {t("knowledge.sourcesDesc", "管理所有已加载的数据源")}
                  </p>
                  <div className="mb-3">
                    <div className="text-xs font-medium mb-2">{t("knowledge.recentSources", "最近使用")}</div>
                    <List
                      size="small"
                      dataSource={sources.slice(0, 5)}
                      renderItem={(source) => (
                        <List.Item
                          className="cursor-pointer hover:bg-black/5 px-2 py-1 rounded"
                          onClick={() => {
                            setSelectedSourceIds([source.id]);
                            setRightPanelTab("wiki");
                            if (!selectedWikiId) {
                              selectWiki(source.wikiId);
                            }
                          }}
                        >
                          <List.Item.Meta
                            title={<span style={{ fontSize: 12 }}>{source.title}</span>}
                            description={
                              <span style={{ fontSize: 11, color: token.colorTextQuaternary }}>
                                {source.sourceType}
                              </span>
                            }
                          />
                        </List.Item>
                      )}
                    />
                  </div>
                  <Button
                    icon={<UploadOutlined size={14} />}
                    onClick={handleSelectFolder}
                    block
                  >
                    {t("knowledge.addSource", "添加数据源")}
                  </Button>
                </div>
              ),
            },
          ]}
        />
      </aside>

      <main className="flex-1 flex flex-col overflow-hidden">
        <Tabs
          activeKey={rightPanelTab}
          onChange={(key) => setRightPanelTab(key as "rag" | "wiki")}
          style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}
          tabBarStyle={{ padding: "0 24px", marginBottom: 0, flexShrink: 0 }}
          items={[
            {
              key: "rag",
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                  <Database size={14} />
                  {t("knowledge.ragTab", "知识库 RAG")}
                </span>
              ),
              children: (
                <div style={{ flex: 1, overflow: "auto" }}>
                  {selectedNoteId
                    ? (
                      <div>
                        <Button
                          icon={<ArrowLeft />}
                          onClick={handleBackFromNote}
                          className="mb-2"
                          style={{ margin: 16 }}
                        >
                          {t("wiki.backToNotes", "Back to Notes")}
                        </Button>
                        <WikiEditorPage noteId={selectedNoteId} onBack={handleBackFromNote} />
                      </div>
                    )
                    : (
                      renderRAGContent()
                    )}
                </div>
              ),
            },
            {
              key: "wiki",
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                  <Library size={14} />
                  {t("knowledge.wikiTab", "Wiki")}
                </span>
              ),
              children: (
                <div style={{ flex: 1, overflow: "auto" }}>
                  {selectedNoteId
                    ? (
                      <div>
                        <Button
                          icon={<ArrowLeft />}
                          onClick={handleBackFromNote}
                          className="mb-2"
                          style={{ margin: 16 }}
                        >
                          {t("wiki.backToNotes", "Back to Notes")}
                        </Button>
                        <WikiEditorPage noteId={selectedNoteId} onBack={handleBackFromNote} />
                      </div>
                    )
                    : (
                      renderWikiContent()
                    )}
                </div>
              ),
            },
          ]}
        />
      </main>
    </div>
  );
}
