import type { GraphData, GraphNode } from "@/components/wiki/GraphView";
import { invoke } from "@/lib/invoke";
import { useKnowledgeStore } from "@/stores";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { BacklinkInfo, KnowledgeBase, Note, NoteLink } from "@/types";
import { DeleteOutlined, LinkOutlined, SaveOutlined } from "@ant-design/icons";
import {
  Button,
  Empty,
  List,
  message,
  Modal,
  Popconfirm,
  Select,
  Spin,
  Tabs,
  Tag,
  theme,
  Tooltip,
  Typography,
} from "antd";
import { ArrowLeftRight, BookOpen, GitGraph, Network, PenLine, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ReactFlow, { Background, Controls, Edge, MiniMap, Node, useEdgesState, useNodesState } from "reactflow";
import "reactflow/dist/style.css";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface WikiDetailPanelProps {
  noteId: string | null;
  graphData: GraphData | null;
  onClose: () => void;
  onNoteUpdated: () => void;
  onNavigateToNote: (noteId: string) => void;
}

type DetailTab = "edit" | "backlinks" | "outlinks" | "localgraph";

export function WikiDetailPanel({
  noteId,
  graphData,
  onClose,
  onNoteUpdated,
  onNavigateToNote,
}: WikiDetailPanelProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { getNote, updateNote, deleteNote, getNoteLinks, getNoteBacklinks } = useWikiStore();
  const { bases: knowledgeBases, loadBases } = useKnowledgeStore();

  const [note, setNote] = useState<Note | null>(null);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [links, setLinks] = useState<NoteLink[]>([]);
  const [backlinks, setBacklinks] = useState<BacklinkInfo[]>([]);
  const [linksLoading, setLinksLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<DetailTab>("edit");
  const autoSaveRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [syncModalOpen, setSyncModalOpen] = useState(false);
  const [selectedKbId, setSelectedKbId] = useState<string | null>(null);
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    if (!noteId) {
      setNote(null);
      setContent("");
      setTitle("");
      setLinks([]);
      setBacklinks([]);
      return;
    }
    loadNote();
    loadLinks();
  }, [noteId]);

  const loadNote = useCallback(async () => {
    if (!noteId) { return; }
    setLoading(true);
    const n = await getNote(noteId);
    if (n) {
      setNote(n);
      setContent(n.content);
      setTitle(n.title);
    }
    setLoading(false);
  }, [noteId, getNote]);

  const loadLinks = useCallback(async () => {
    if (!noteId) { return; }
    setLinksLoading(true);
    const [l, bl] = await Promise.all([getNoteLinks(noteId), getNoteBacklinks(noteId)]);
    setLinks(l);
    setBacklinks(bl);
    setLinksLoading(false);
  }, [noteId, getNoteLinks, getNoteBacklinks]);

  useEffect(() => {
    if (note) {
      setHasChanges(content !== note.content || title !== note.title);
    }
  }, [content, title, note]);

  // Ctrl+S 保存
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  // 自动保存（3 秒空闲）
  useEffect(() => {
    if (!hasChanges || saving) { return; }
    if (autoSaveRef.current) { clearTimeout(autoSaveRef.current); }
    autoSaveRef.current = setTimeout(() => handleSave(), 3000);
    return () => {
      if (autoSaveRef.current) { clearTimeout(autoSaveRef.current); }
    };
  }, [content, title]);

  const handleSave = async () => {
    if (!note || !hasChanges) { return; }
    setSaving(true);
    try {
      const updated = await updateNote(note.id, { title, content });
      if (updated) {
        setNote(updated);
        setHasChanges(false);
        onNoteUpdated();
      }
    } catch (e) {
      message.error(String(e));
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    if (!note) { return; }
    await deleteNote(note.id);
    message.success(t("wiki.deleted"));
    onNoteUpdated();
    onClose();
  };

  const handleOpenSyncModal = useCallback(() => {
    loadBases();
    setSelectedKbId(null);
    setSyncModalOpen(true);
  }, [loadBases]);

  const handleSyncToKb = useCallback(async () => {
    if (!note || !selectedKbId) { return; }
    setSyncing(true);
    try {
      await invoke("sync_note_to_knowledge_base", { noteId: note.id, knowledgeBaseId: selectedKbId });
      message.success(t("wiki.sync.success"));
      setSyncModalOpen(false);
    } catch (e) {
      message.error(t("wiki.sync.error") + ": " + String(e));
    }
    setSyncing(false);
  }, [note, selectedKbId, t]);

  // 局部图谱：当前节点 + 直接邻居
  const localGraphData = useMemo(() => {
    if (!noteId || !graphData) { return { nodes: [], edges: [] }; }
    const neighborIds = new Set<string>();
    graphData.edges.forEach((e) => {
      if (e.source === noteId) { neighborIds.add(e.target); }
      if (e.target === noteId) { neighborIds.add(e.source); }
    });
    neighborIds.add(noteId);
    return {
      nodes: graphData.nodes.filter((n) => neighborIds.has(n.id)),
      edges: graphData.edges.filter((e) => neighborIds.has(e.source) && neighborIds.has(e.target)),
    };
  }, [noteId, graphData]);

  const noteNode = graphData?.nodes.find((n) => n.id === noteId);
  const noteTitle = noteNode?.title || note?.title || "";

  if (!noteId) {
    return (
      <div className="h-full flex items-center justify-center" style={{ backgroundColor: token.colorBgElevated }}>
        <Empty description={t("wiki.selectNote")} />
      </div>
    );
  }

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center" style={{ backgroundColor: token.colorBgElevated }}>
        <Spin />
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col" style={{ backgroundColor: token.colorBgElevated, overflow: "hidden" }}>
      {/* 标题栏 — 玻璃态 */}
      <div
        className="flex items-center gap-2 px-4 py-2.5 shrink-0 backdrop-blur-lg"
        style={{
          borderBottom: `1px solid ${token.colorBorderSecondary}20`,
          backgroundColor: `${token.colorBgContainer}dd`,
        }}
      >
        <div
          className="w-1.5 h-5 rounded-full"
          style={{ backgroundColor: token.colorPrimary }}
        />
        <Text strong ellipsis className="flex-1 text-sm tracking-tight" title={noteTitle}>
          {noteTitle}
        </Text>
        <Tooltip title={t("wiki.close")}>
          <Button
            icon={<X size={14} />}
            size="small"
            type="text"
            className="opacity-60 hover:opacity-100 transition-opacity"
            onClick={onClose}
          />
        </Tooltip>
      </div>

      <Tabs
        activeKey={activeTab}
        onChange={(k) => setActiveTab(k as DetailTab)}
        className="flex-1 flex flex-col"
        style={{ minHeight: 0 }}
        tabBarStyle={{
          padding: "4px 12px 0",
          marginBottom: 0,
          flexShrink: 0,
          borderBottom: `1px solid ${token.colorBorderSecondary}10`,
        }}
        size="small"
        items={[
          {
            key: "edit",
            label: (
              <span className="flex items-center gap-1">
                <PenLine size={12} />
                {t("wiki.edit")}
              </span>
            ),
            children: (
              <div className="flex flex-col gap-2 p-3" style={{ height: "calc(100% - 46px)" }}>
                {/* 标题 */}
                <input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  className="w-full text-lg font-semibold bg-transparent border-none outline-none tracking-tight placeholder:opacity-40"
                  style={{ color: token.colorText }}
                  placeholder={t("wiki.titlePlaceholder")}
                />
                {/* 工具栏 */}
                <div className="flex items-center gap-2 shrink-0">
                  <Button
                    icon={<SaveOutlined />}
                    size="small"
                    type="primary"
                    onClick={handleSave}
                    loading={saving}
                    disabled={!hasChanges}
                    className="shadow-sm"
                  >
                    {t("wiki.save")}
                  </Button>
                  <Popconfirm
                    title={t("wiki.confirmDelete")}
                    onConfirm={handleDelete}
                  >
                    <Button
                      icon={<DeleteOutlined />}
                      size="small"
                      danger
                      type="text"
                      className="opacity-50 hover:opacity-100"
                    />
                  </Popconfirm>
                  <Tooltip title={t("wiki.sync.toKnowledgeBase")}>
                    <Button
                      icon={<BookOpen size={14} />}
                      size="small"
                      type="text"
                      className="opacity-50 hover:opacity-100"
                      onClick={handleOpenSyncModal}
                    />
                  </Tooltip>
                  {hasChanges && (
                    <span
                      className="text-xs px-1.5 py-0.5 rounded-full animate-pulse"
                      style={{ backgroundColor: `${token.colorWarningBg}`, color: token.colorWarningText }}
                    >
                      {t("wiki.unsaved")}
                    </span>
                  )}
                  {note?.author === "llm" && (
                    <span
                      className="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
                      style={{ backgroundColor: `${token.colorPrimary}18`, color: token.colorPrimary }}
                    >
                      AI
                    </span>
                  )}
                </div>
                {/* 编辑器 — 卡片风格 */}
                <div
                  className="flex-1 overflow-hidden rounded-xl transition-shadow duration-200"
                  style={{
                    backgroundColor: token.colorBgContainer,
                    border: `1px solid ${token.colorBorderSecondary}60`,
                    boxShadow: `0 1px 2px ${token.colorBgLayout}40`,
                  }}
                >
                  <textarea
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    className="w-full h-full p-4 resize-none outline-none bg-transparent text-sm leading-relaxed font-mono placeholder:opacity-30"
                    style={{ color: token.colorText }}
                    placeholder={t("wiki.contentPlaceholder")}
                    spellCheck={false}
                  />
                </div>
                {/* 快速操作 */}
                <div className="shrink-0 flex gap-1.5">
                  <Button
                    size="small"
                    className="text-xs opacity-70 hover:opacity-100 transition-opacity"
                    onClick={() => setContent((c) => c + "[[新笔记]]")}
                  >
                    <LinkOutlined /> [[link]]
                  </Button>
                  <Button
                    size="small"
                    className="text-xs opacity-70 hover:opacity-100 transition-opacity"
                    onClick={() => setActiveTab("backlinks")}
                  >
                    <ArrowLeftRight size={12} />
                    <span className="ml-1">{backlinks.length}</span>
                  </Button>
                  <Button
                    size="small"
                    className="text-xs opacity-70 hover:opacity-100 transition-opacity"
                    onClick={() => setActiveTab("outlinks")}
                  >
                    <ArrowLeftRight size={12} />
                    <span className="ml-1">{links.length}</span>
                  </Button>
                </div>
              </div>
            ),
          },
          {
            key: "backlinks",
            label: (
              <span className="flex items-center gap-1">
                <ArrowLeftRight size={12} />
                {t("wiki.backlinks")}
                {backlinks.length > 0 && (
                  <Tag color="blue" style={{ fontSize: 10, margin: 0, padding: "0 4px" }}>
                    {backlinks.length}
                  </Tag>
                )}
              </span>
            ),
            children: (
              <BacklinkList
                backlinks={backlinks}
                loading={linksLoading}
                currentNoteTitle={noteTitle}
                onNavigate={onNavigateToNote}
                emptyText={t("wiki.noBacklinks")}
                token={token}
              />
            ),
          },
          {
            key: "outlinks",
            label: (
              <span className="flex items-center gap-1">
                <ArrowLeftRight size={12} />
                {t("wiki.outlinks")}
                {links.length > 0 && (
                  <Tag color="green" style={{ fontSize: 10, margin: 0, padding: "0 4px" }}>
                    {links.length}
                  </Tag>
                )}
              </span>
            ),
            children: (
              <LinkList
                links={links}
                loading={linksLoading}
                graphData={graphData}
                onNavigate={onNavigateToNote}
                emptyText={t("wiki.noOutlinks")}
                token={token}
              />
            ),
          },
          {
            key: "localgraph",
            label: (
              <span className="flex items-center gap-1">
                <GitGraph size={12} />
                {t("wiki.localGraph")}
              </span>
            ),
            children: <LocalGraphView data={localGraphData} token={token} onNodeClick={onNavigateToNote} />,
          },
        ]}
      />

      <Modal
        title={t("wiki.sync.toKnowledgeBaseTitle")}
        open={syncModalOpen}
        onOk={handleSyncToKb}
        onCancel={() => setSyncModalOpen(false)}
        okButtonProps={{ loading: syncing, disabled: !selectedKbId }}
        okText={t("wiki.sync.toKnowledgeBase")}
        width={420}
      >
        <div className="py-4">
          <div className="text-sm font-medium mb-2">
            {t("wiki.sync.selectKnowledgeBase")}
          </div>
          <Select
            value={selectedKbId ?? undefined}
            onChange={setSelectedKbId}
            placeholder={t("wiki.sync.selectKnowledgeBase")}
            style={{ width: "100%" }}
            options={knowledgeBases.map((kb: KnowledgeBase) => ({
              value: kb.id,
              label: kb.name,
            }))}
          />
        </div>
      </Modal>
    </div>
  );
}

function highlightWikilink(snippet: string, linkText: string) {
  const linkPattern = `[[${linkText}]]`;
  const parts = snippet.split(linkPattern);
  if (parts.length === 1) { return <span>{snippet}</span>; }

  return (
    <span>
      {parts.map((part, i) => (
        // 静态文本分割列表，基于索引的 key 安全
        <span key={i}>
          {part}
          {i < parts.length - 1 && (
            <Text
              strong
              style={{
                backgroundColor: "rgba(22,119,255,0.12)",
                borderRadius: 3,
                padding: "0 2px",
              }}
            >
              {linkPattern}
            </Text>
          )}
        </span>
      ))}
    </span>
  );
}

function BacklinkList({
  backlinks,
  loading,
  currentNoteTitle,
  onNavigate,
  emptyText,
  token,
}: {
  backlinks: BacklinkInfo[];
  loading: boolean;
  currentNoteTitle: string;
  onNavigate: (nodeId: string) => void;
  emptyText: string;
  token: ReturnType<typeof theme.useToken>["token"];
}) {
  if (loading) {
    return <Spin className="flex justify-center mt-8" />;
  }

  if (backlinks.length === 0) {
    return <Empty description={emptyText} className="mt-8" />;
  }

  return (
    <List
      dataSource={backlinks}
      renderItem={(bl) => (
        <List.Item
          className="cursor-pointer px-4 py-3 mx-2 my-0.5 rounded-xl transition-all duration-200 hover:shadow-sm"
          style={{ border: "none" }}
          onClick={() => onNavigate(bl.noteId)}
        >
          <div className="flex items-start gap-3 w-full">
            <div
              className="size-8 rounded-lg flex items-center justify-center shrink-0 mt-0.5"
              style={{ backgroundColor: `${token.colorPrimary}10` }}
            >
              <Network size={14} style={{ color: token.colorPrimary }} />
            </div>
            <div className="flex-1 min-w-0">
              <Text className="text-sm font-medium truncate block" style={{ color: token.colorPrimary }}>
                {bl.title}
              </Text>
              {bl.snippets.map((snippet, si) => (
                <Typography.Paragraph
                  key={si}
                  className="!mb-1 text-xs leading-relaxed !mt-1"
                  style={{ color: token.colorTextSecondary }}
                  ellipsis={{ rows: 2, expandable: false }}
                >
                  {highlightWikilink(snippet, currentNoteTitle)}
                </Typography.Paragraph>
              ))}
            </div>
          </div>
        </List.Item>
      )}
    />
  );
}

function LinkList({
  links,
  loading,
  graphData,
  onNavigate,
  emptyText,
  token,
}: {
  links: NoteLink[];
  loading: boolean;
  graphData: GraphData | null;
  onNavigate: (nodeId: string) => void;
  emptyText: string;
  token: ReturnType<typeof theme.useToken>["token"];
}) {
  if (loading) {
    return <Spin className="flex justify-center mt-8" />;
  }

  if (links.length === 0) {
    return <Empty description={emptyText} className="mt-8" />;
  }

  const findNode = (id: string) => graphData?.nodes.find((n) => n.id === id);

  return (
    <List
      dataSource={links}
      renderItem={(link) => {
        const sourceNode = findNode(link.sourceNoteId);
        const targetNode = findNode(link.targetNoteId);
        return (
          <List.Item
            className="cursor-pointer px-4 py-3 mx-2 my-0.5 rounded-xl transition-all duration-200 hover:shadow-sm"
            style={{ border: "none" }}
            onClick={() => onNavigate(link.sourceNoteId !== "" ? link.sourceNoteId : link.targetNoteId)}
          >
            <div className="flex items-center gap-3 w-full">
              <div
                className="size-8 rounded-lg flex items-center justify-center shrink-0"
                style={{ backgroundColor: `${token.colorPrimary}10` }}
              >
                <Network size={14} style={{ color: token.colorPrimary }} />
              </div>
              <div className="flex-1 min-w-0">
                <Text className="text-sm font-medium truncate block">
                  {sourceNode?.title || link.sourceNoteId}
                </Text>
                <Text
                  type="secondary"
                  className="text-xs block mt-0.5"
                  style={{ color: token.colorTextSecondary }}
                >
                  → {targetNode?.title || link.targetNoteId}
                  <span
                    className="ml-2 px-1 py-0.5 rounded text-[10px]"
                    style={{ backgroundColor: `${token.colorBorderSecondary}30` }}
                  >
                    {link.linkType}
                  </span>
                </Text>
              </div>
            </div>
          </List.Item>
        );
      }}
    />
  );
}

function LocalGraphView({
  data,
  token,
  onNodeClick,
}: {
  data: { nodes: GraphNode[]; edges: { source: string; target: string; type: string }[] };
  token: ReturnType<typeof theme.useToken>["token"];
  onNodeClick: (nodeId: string) => void;
}) {
  const { t } = useTranslation();
  const initialNodes: Node[] = useMemo(
    () =>
      data.nodes.map((n, i) => ({
        id: n.id,
        type: "default",
        position: {
          x: 200 + Math.cos((2 * Math.PI * i) / Math.max(data.nodes.length, 1)) * 120,
          y: 200 + Math.sin((2 * Math.PI * i) / Math.max(data.nodes.length, 1)) * 120,
        },
        data: { label: n.title },
        style: {
          background: token.colorBgContainer,
          border: `1px solid ${token.colorPrimary}`,
          borderRadius: 8,
          padding: "8px 12px",
          fontSize: 12,
          maxWidth: 160,
        },
      })),
    [data.nodes, token],
  );

  const initialEdges: Edge[] = useMemo(
    () =>
      data.edges.map((e) => ({
        id: `${e.source}-${e.target}`,
        source: e.source,
        target: e.target,
        type: "smoothstep",
        style: { stroke: token.colorBorderSecondary, strokeWidth: 1 },
      })),
    [data.edges, token],
  );

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  useEffect(() => {
    setNodes(initialNodes);
    setEdges(initialEdges);
  }, [initialNodes, initialEdges, setNodes, setEdges]);

  if (data.nodes.length === 0) {
    return <Empty description={t("wiki.graph.noConnectedNodes")} className="mt-8" />;
  }

  return (
    <div style={{ width: "100%", height: "calc(100% - 46px)", position: "relative" }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={(_, node) => onNodeClick(node.id)}
        fitView
        nodesDraggable
        nodesConnectable={false}
        elementsSelectable
      >
        <Controls />
        <MiniMap style={{ width: 100, height: 80 }} />
        <Background gap={16} color={token.colorBorderSecondary} />
      </ReactFlow>
    </div>
  );
}
