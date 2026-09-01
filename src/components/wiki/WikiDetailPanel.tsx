// SPDX-License-Identifier: AGPL-3.0-only

import { MonacoEditor } from "@/components/shared/MonacoEditor";
import type { GraphData, GraphNode } from "@/components/wiki/GraphView";
import { highlightWikilink } from "@/components/wiki/wikilinkHighlight";
import { useWikiAutoSave } from "@/hooks/useWikiAutoSave";
import { invoke } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { useKnowledgeStore } from "@/stores";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { BacklinkInfo, KnowledgeBase, Note, NoteLink } from "@/types";
import { DeleteOutlined, LinkOutlined, SaveOutlined } from "@ant-design/icons";
import { Background, Controls, Edge, MiniMap, Node, ReactFlow, useEdgesState, useNodesState } from "@xyflow/react";
import { Button, Empty, Modal, Popconfirm, Select, Spin, Tabs, Tag, theme, Tooltip, Typography } from "antd";
import { ArrowLeftRight, BookOpen, GitGraph, Network, PenLine, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "@xyflow/react/dist/style.css";
import { showBackendError } from "@/lib/errorI18n";
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
  const [loadFailed, setLoadFailed] = useState(false);
  const [saving, setSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [links, setLinks] = useState<NoteLink[]>([]);
  const [backlinks, setBacklinks] = useState<BacklinkInfo[]>([]);
  const [linksLoading, setLinksLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<DetailTab>("edit");
  const isSavingRef = useRef(false);
  const [syncModalOpen, setSyncModalOpen] = useState(false);
  const [selectedKbId, setSelectedKbId] = useState<string | null>(null);
  const [syncing, setSyncing] = useState(false);

  const loadNote = useCallback(async () => {
    if (!noteId) {
      return;
    }
    // 实体节点无对应笔记，显示实体信息面板而非尝试加载
    if (noteId.startsWith("entity:")) {
      setNote(null);
      setContent("");
      setTitle("");
      setLoadFailed(true);
      setLoading(false);
      return;
    }
    setLoading(true);
    setLoadFailed(false);
    const n = await getNote(noteId);
    if (n) {
      setNote(n);
      setContent(n.content);
      setTitle(n.title);
    } else {
      setLoadFailed(true);
      message.error(t("wiki.noteLoadFailed"));
    }
    setLoading(false);
  }, [noteId, getNote, t]);

  const loadLinks = useCallback(async () => {
    if (!noteId) {
      return;
    }
    setLinksLoading(true);
    const [l, bl] = await Promise.all([
      getNoteLinks(noteId),
      getNoteBacklinks(noteId),
    ]);
    setLinks(l);
    setBacklinks(bl);
    setLinksLoading(false);
  }, [noteId, getNoteLinks, getNoteBacklinks]);

  const handleSave = useCallback(async () => {
    if (isSavingRef.current) {
      return;
    }
    if (!note || !hasChanges) {
      return;
    }
    isSavingRef.current = true;
    setSaving(true);
    try {
      const updated = await updateNote(note.id, { title, content });
      if (updated) {
        setNote(updated);
        setHasChanges(false);
        onNoteUpdated();
      }
    } catch (e) {
      showBackendError(message, e);
    } finally {
      setSaving(false);
      isSavingRef.current = false;
    }
  }, [note, hasChanges, updateNote, onNoteUpdated, content, title]);

  useEffect(() => {
    if (!noteId) {
      setTimeout(() => {
        setNote(null);
        setContent("");
        setTitle("");
        setLinks([]);
        setBacklinks([]);
        setLoadFailed(false);
      }, 0);
      return;
    }
    setTimeout(() => loadNote(), 0);
    setTimeout(() => loadLinks(), 0);
  }, [noteId, loadNote, loadLinks, setNote]);

  useEffect(() => {
    if (note) {
      setTimeout(() => setHasChanges(content !== note.content || title !== note.title), 0);
    }
  }, [content, title, note]);

  // Ctrl+S 立即保存 + 3 秒空闲自动保存（F5：共享 hook，与全页编辑器行为一致）
  useWikiAutoSave({
    content,
    title,
    autoSaveEnabled: hasChanges && !saving && !isSavingRef.current,
    handleSave,
  });

  const handleDelete = async () => {
    if (!note) {
      return;
    }
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
    if (!note || !selectedKbId) {
      return;
    }
    setSyncing(true);
    try {
      await invoke("sync_note_to_knowledge_base", {
        noteId: note.id,
        knowledgeBaseId: selectedKbId,
      });
      message.success(t("wiki.sync.success"));
      setSyncModalOpen(false);
    } catch (e) {
      message.error(t("wiki.sync.error") + ": " + String(e));
    }
    setSyncing(false);
  }, [note, selectedKbId, t]);

  // 局部图谱：当前节点 + 直接邻居
  const localGraphData = useMemo(() => {
    if (!noteId || !graphData) {
      return { nodes: [], edges: [] };
    }
    const neighborIds = new Set<string>();
    graphData.edges.forEach((e) => {
      if (e.source === noteId) {
        neighborIds.add(e.target);
      }
      if (e.target === noteId) {
        neighborIds.add(e.source);
      }
    });
    neighborIds.add(noteId);
    return {
      nodes: graphData.nodes.filter((n) => neighborIds.has(n.id)),
      edges: graphData.edges.filter(
        (e) => neighborIds.has(e.source) && neighborIds.has(e.target),
      ),
    };
  }, [noteId, graphData]);

  const noteNode = graphData?.nodes.find((n) => n.id === noteId);
  const noteTitle = noteNode?.title || note?.title || "";
  const isEntityNode = noteId?.startsWith("entity:") ?? false;

  if (!noteId) {
    return (
      <div
        className="h-full flex items-center justify-center"
        style={{ backgroundColor: token.colorBgElevated }}
      >
        <Empty description={t("wiki.selectNote")} />
      </div>
    );
  }

  if (loading) {
    return (
      <div
        className="h-full flex items-center justify-center"
        style={{ backgroundColor: token.colorBgElevated }}
      >
        <Spin />
      </div>
    );
  }

  // 实体节点：显示实体信息而非笔记编辑器
  if (isEntityNode) {
    return (
      <div
        className="h-full flex flex-col"
        style={{ backgroundColor: token.colorBgElevated }}
      >
        <div
          className="flex items-center gap-2 px-4 py-2.5 shrink-0"
          style={{
            borderBottom: `1px solid ${token.colorBorderSecondary}20`,
            backgroundColor: `${token.colorBgContainer}dd`,
          }}
        >
          <div
            className="w-1.5 h-5 rounded-full"
            style={{ backgroundColor: "#FA8C16" }}
          />
          <Text strong ellipsis style={{ flex: 1 }}>
            {noteNode?.title || noteId}
          </Text>
        </div>
        <div className="flex-1 flex items-center justify-center p-6">
          <Empty
            description={
              <div style={{ textAlign: "center" }}>
                <div style={{ marginBottom: 8, fontWeight: 500 }}>
                  {t("wiki.entityNodeInfo")}
                </div>
                <div style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                  {t("wiki.entityNodeDesc")}
                </div>
              </div>
            }
          />
        </div>
      </div>
    );
  }

  // 笔记加载失败/不存在：显示错误态而非空白编辑区
  if (loadFailed && !note) {
    return (
      <div
        className="h-full flex items-center justify-center"
        style={{ backgroundColor: token.colorBgElevated }}
      >
        <Empty description={t("wiki.noteLoadFailed")} />
      </div>
    );
  }

  return (
    <div
      className="h-full flex flex-col"
      style={{ backgroundColor: token.colorBgElevated, overflow: "hidden" }}
    >
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
        <Text
          strong
          ellipsis
          className="flex-1 text-sm tracking-tight"
          title={noteTitle}
        >
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
              <div
                className="flex flex-col gap-2 p-3"
                style={{ height: "calc(100% - 46px)" }}
              >
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
                      style={{
                        backgroundColor: `${token.colorWarningBg}`,
                        color: token.colorWarningText,
                      }}
                    >
                      {t("wiki.unsaved")}
                    </span>
                  )}
                  {note?.author === "llm" && (
                    <span
                      className="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
                      style={{
                        backgroundColor: `${token.colorPrimary}18`,
                        color: token.colorPrimary,
                      }}
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
                  {/* F3: 复用共享 MonacoEditor（loadMonaco 懒加载）替换裸 textarea，提供 markdown 高亮 */}
                  <MonacoEditor
                    value={content}
                    language="markdown"
                    onChange={(v) => setContent(v)}
                    height="100%"
                  />
                </div>
                {/* 快速操作 */}
                <div className="shrink-0 flex gap-1.5">
                  <Button
                    size="small"
                    className="text-xs opacity-70 hover:opacity-100 transition-opacity"
                    onClick={() => setContent((c) => c + `[[${t("wiki.newNote")}]]`)}
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
                  <Tag
                    color="blue"
                    style={{ fontSize: 10, margin: 0, padding: "0 4px" }}
                  >
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
                  <Tag
                    color="green"
                    style={{ fontSize: 10, margin: 0, padding: "0 4px" }}
                  >
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
            children: (
              <LocalGraphView
                data={localGraphData}
                token={token}
                onNodeClick={onNavigateToNote}
              />
            ),
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
    <div className="divide-y divide-gray-100">
      {backlinks.map((bl) => (
        <div
          key={bl.noteId}
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
              <Text
                className="text-sm font-medium truncate block"
                style={{ color: token.colorPrimary }}
              >
                {bl.title}
              </Text>
              {bl.snippets.map((snippet, si) => (
                // FIXME: snippets 是字符串数组，无稳定唯一标识
                <Typography.Paragraph
                  key={`snippet-${si}`}
                  className="!mb-1 text-xs leading-relaxed !mt-1"
                  style={{ color: token.colorTextSecondary }}
                  ellipsis={{ rows: 2, expandable: false }}
                >
                  {highlightWikilink(snippet, currentNoteTitle, token)}
                </Typography.Paragraph>
              ))}
            </div>
          </div>
        </div>
      ))}
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
    <div className="divide-y divide-gray-100">
      {links.map((link) => {
        const sourceNode = findNode(link.sourceNoteId);
        const targetNode = findNode(link.targetNoteId);
        return (
          <div
            key={`${link.sourceNoteId}-${link.targetNoteId}`}
            className="cursor-pointer px-4 py-3 mx-2 my-0.5 rounded-xl transition-all duration-200 hover:shadow-sm"
            style={{ border: "none" }}
            onClick={() =>
              onNavigate(
                link.sourceNoteId !== ""
                  ? link.sourceNoteId
                  : link.targetNoteId,
              )}
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
                    style={{
                      backgroundColor: `${token.colorBorderSecondary}30`,
                    }}
                  >
                    {link.linkType}
                  </span>
                </Text>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function LocalGraphView({
  data,
  token,
  onNodeClick,
}: {
  data: {
    nodes: GraphNode[];
    edges: { source: string; target: string; type: string }[];
  };
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
          x: 200
            + Math.cos((2 * Math.PI * i) / Math.max(data.nodes.length, 1)) * 120,
          y: 200
            + Math.sin((2 * Math.PI * i) / Math.max(data.nodes.length, 1)) * 120,
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
    <div
      style={{
        width: "100%",
        height: "calc(100% - 46px)",
        position: "relative",
      }}
    >
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
