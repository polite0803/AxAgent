import type { GraphData, GraphNode } from "@/components/wiki/GraphView";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { Note, NoteLink } from "@/types";
import { DeleteOutlined, LinkOutlined, SaveOutlined } from "@ant-design/icons";
import { Button, Empty, List, message, Popconfirm, Spin, Tabs, Tag, theme, Tooltip, Typography } from "antd";
import { ArrowLeftRight, GitGraph, Network, PenLine, X } from "lucide-react";
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

  const [note, setNote] = useState<Note | null>(null);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [links, setLinks] = useState<NoteLink[]>([]);
  const [backlinks, setBacklinks] = useState<NoteLink[]>([]);
  const [linksLoading, setLinksLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<DetailTab>("edit");
  const autoSaveRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
    message.success(t("wiki.deleted", "Note deleted"));
    onNoteUpdated();
    onClose();
  };

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
        <Empty description={t("wiki.selectNote", "Select a note to view details")} />
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
        <Tooltip title={t("wiki.close", "Close")}>
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
                {t("wiki.edit", "Edit")}
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
                  placeholder={t("wiki.titlePlaceholder", "Note title...")}
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
                    {t("wiki.save", "Save")}
                  </Button>
                  <Popconfirm
                    title={t("wiki.confirmDelete", "Delete this note?")}
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
                  {hasChanges && (
                    <span
                      className="text-xs px-1.5 py-0.5 rounded-full animate-pulse"
                      style={{ backgroundColor: `${token.colorWarningBg}`, color: token.colorWarningText }}
                    >
                      {t("wiki.unsaved", "Unsaved")}
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
                    placeholder={t("wiki.contentPlaceholder", "Start writing...\nUse [[link]] to connect notes")}
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
                {t("wiki.backlinks", "Backlinks")}
                {backlinks.length > 0 && (
                  <Tag color="blue" style={{ fontSize: 10, margin: 0, padding: "0 4px" }}>
                    {backlinks.length}
                  </Tag>
                )}
              </span>
            ),
            children: (
              <LinkList
                links={backlinks}
                loading={linksLoading}
                graphData={graphData}
                onNavigate={onNavigateToNote}
                emptyText={t("wiki.noBacklinks", "No backlinks")}
                token={token}
              />
            ),
          },
          {
            key: "outlinks",
            label: (
              <span className="flex items-center gap-1">
                <ArrowLeftRight size={12} />
                {t("wiki.outlinks", "Outgoing")}
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
                emptyText={t("wiki.noOutlinks", "No outgoing links")}
                token={token}
              />
            ),
          },
          {
            key: "localgraph",
            label: (
              <span className="flex items-center gap-1">
                <GitGraph size={12} />
                {t("wiki.localGraph", "Local Graph")}
              </span>
            ),
            children: <LocalGraphView data={localGraphData} token={token} onNodeClick={onNavigateToNote} />,
          },
        ]}
      />
    </div>
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
                className="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
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
    return <Empty description="No connected nodes" className="mt-8" />;
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
