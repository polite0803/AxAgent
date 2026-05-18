import { GraphData, GraphView } from "@/components/wiki/GraphView";
import { WikiDetailPanel } from "@/components/wiki/WikiDetailPanel";
import { WikiFilePanel } from "@/components/wiki/WikiFilePanel";
import { WikiNodeContextMenu } from "@/components/wiki/WikiNodeContextMenu";
import { invoke } from "@/lib/invoke";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { useWikiStore } from "@/stores/feature/wikiStore";
import { FileAddOutlined, NodeIndexOutlined, ReloadOutlined, SearchOutlined } from "@ant-design/icons";
import { Button, Empty, Input, message, Select, Space, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { Eye, PanelLeft, PanelRight } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";

const { Title, Text } = Typography;
const DEFAULT_VAULT_ID = "default";
const MIN_PANEL_WIDTH = 180;
const MAX_LEFT_PANEL = 400;
const MAX_RIGHT_PANEL = 600;

export function WikiGraphPage() {
  const { token } = theme.useToken();
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { wikiId } = useParams<{ wikiId: string }>();
  const [searchParams] = useSearchParams();
  const wikiIdFromUrl = searchParams.get("wikiId") || wikiId || DEFAULT_VAULT_ID;

  const { wikis, loadWikis } = useLlmWikiStore();
  const {
    notes,
    loading: notesLoading,
    loadNotes,
    createNote,
    deleteNote,
    setSelectedVaultId,
  } = useWikiStore();

  // 图谱数据
  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [graphLoading, setGraphLoading] = useState(true);
  const [communities, setCommunities] = useState<Map<string, number> | null>(null);

  // 选中和高亮
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [highlightedNodeIds, setHighlightedNodeIds] = useState<Set<string>>(new Set());
  const [detailPanelOpen, setDetailPanelOpen] = useState(false);

  // 右键菜单
  const [contextMenu, setContextMenu] = useState<{
    visible: boolean;
    nodeId: string;
    position: { x: number; y: number };
  }>({ visible: false, nodeId: "", position: { x: 0, y: 0 } });

  // 面板宽度拖曳
  const [leftPanelWidth, setLeftPanelWidth] = useState(240);
  const [rightPanelWidth, setRightPanelWidth] = useState(380);
  const [leftPanelVisible, setLeftPanelVisible] = useState(true);
  const [leftAtBoundary, setLeftAtBoundary] = useState<"min" | "max" | null>(null);
  const [rightAtBoundary, setRightAtBoundary] = useState<"min" | "max" | null>(null);
  const resizingRef = useRef<"left" | "right" | null>(null);

  // 搜索
  const [globalSearch, setGlobalSearch] = useState("");

  // 加载 Wiki 列表和图谱
  useEffect(() => {
    loadWikis();
  }, [loadWikis]);

  const loadGraphData = useCallback(async () => {
    setGraphLoading(true);
    try {
      const [data, communityResult] = await Promise.all([
        invoke<GraphData>("get_wiki_graph", { wikiId: wikiIdFromUrl }),
        invoke<{ communities: Record<string, number> }>("wiki_graph_communities", { wikiId: wikiIdFromUrl })
          .catch(() => null),
      ]);
      setGraphData(data);
      if (communityResult?.communities) {
        setCommunities(new Map(Object.entries(communityResult.communities)));
      } else {
        setCommunities(null);
      }
    } catch (e) {
      message.error(t("wiki.graph.loadError", { error: String(e) }));
    }
    setGraphLoading(false);
  }, [wikiIdFromUrl, t]);

  useEffect(() => {
    setSelectedVaultId(wikiIdFromUrl);
    loadNotes(wikiIdFromUrl);
    loadGraphData();
  }, [wikiIdFromUrl, setSelectedVaultId, loadNotes, loadGraphData]);

  const handleReload = () => {
    loadNotes(wikiIdFromUrl);
    loadGraphData();
  };

  // 面板拖曳
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (resizingRef.current === "left") {
        const clamped = Math.max(MIN_PANEL_WIDTH, Math.min(MAX_LEFT_PANEL, e.clientX));
        setLeftPanelWidth(clamped);
        setLeftAtBoundary(
          clamped <= MIN_PANEL_WIDTH ? "min" : clamped >= MAX_LEFT_PANEL ? "max" : null,
        );
      } else if (resizingRef.current === "right") {
        const clamped = Math.max(MIN_PANEL_WIDTH, Math.min(MAX_RIGHT_PANEL, window.innerWidth - e.clientX));
        setRightPanelWidth(clamped);
        setRightAtBoundary(
          clamped <= MIN_PANEL_WIDTH ? "min" : clamped >= MAX_RIGHT_PANEL ? "max" : null,
        );
      }
    };
    const handleMouseUp = () => {
      resizingRef.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      setLeftAtBoundary(null);
      setRightAtBoundary(null);
    };
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

  const handleResizeStart = (side: "left" | "right") => (e: React.MouseEvent) => {
    e.preventDefault();
    resizingRef.current = side;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  // 节点操作
  const handleNodeClick = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
  }, []);

  const handleNodeDoubleClick = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
    setDetailPanelOpen(true);
  }, []);

  const handleContextMenu = useCallback(
    (nodeId: string, position: { x: number; y: number }) => {
      setSelectedNodeId(nodeId);
      setContextMenu({ visible: true, nodeId, position });
      setDetailPanelOpen(true);
    },
    [],
  );

  const handleSearchHighlight = useCallback((nodeIds: Set<string>) => {
    setHighlightedNodeIds(nodeIds);
  }, []);

  const handleNavigateToNote = useCallback(
    (noteId: string) => {
      setSelectedNodeId(noteId);
      setDetailPanelOpen(true);
    },
    [],
  );

  const handleCreateNote = useCallback(async () => {
    const now = Date.now();
    const note = await createNote({
      vaultId: wikiIdFromUrl,
      title: `${t("wiki.newNoteDefault")} ${new Date(now).toLocaleString(i18n.language)}`,
      filePath: `/new-note-${now}.md`,
      content: "",
      author: "user",
    });
    if (note) {
      loadNotes(wikiIdFromUrl);
      loadGraphData();
      setSelectedNodeId(note.id);
      setDetailPanelOpen(true);
    }
  }, [wikiIdFromUrl, createNote, loadNotes, loadGraphData]);

  const handleCreateLinkedNote = useCallback(
    async (sourceNodeId: string) => {
      const sourceNode = graphData?.nodes.find((n) => n.id === sourceNodeId);
      const title = sourceNode
        ? `${t("wiki.linkedPrefix")}: ${sourceNode.title}`
        : t("wiki.linkedNoteTitle");
      const now = Date.now();
      const note = await createNote({
        vaultId: wikiIdFromUrl,
        title,
        filePath: `/linked-note-${now}.md`,
        content: sourceNode ? `${t("wiki.linkedRef")}: [[${sourceNode.title}]]` : "",
        author: "user",
      });
      if (note) {
        loadNotes(wikiIdFromUrl);
        loadGraphData();
        setSelectedNodeId(note.id);
        setDetailPanelOpen(true);
      }
    },
    [wikiIdFromUrl, graphData, createNote, loadNotes, loadGraphData],
  );

  const handleDeleteNote = useCallback(
    async (nodeId: string) => {
      try {
        await deleteNote(nodeId);
        message.success(t("wiki.deleted"));
        if (selectedNodeId === nodeId) {
          setSelectedNodeId(null);
          setDetailPanelOpen(false);
        }
        loadNotes(wikiIdFromUrl);
        loadGraphData();
      } catch (e) {
        message.error(String(e));
      }
    },
    [deleteNote, selectedNodeId, wikiIdFromUrl, loadNotes, loadGraphData, t],
  );

  const handleNoteUpdated = () => {
    loadNotes(wikiIdFromUrl);
    loadGraphData();
  };

  const handleGlobalSearch = (value: string) => {
    setGlobalSearch(value);
    if (!value.trim() || !graphData) {
      setHighlightedNodeIds(new Set());
      return;
    }
    const q = value.toLowerCase();
    const ids = new Set<string>();
    graphData.nodes.forEach((n) => {
      if (
        n.title.toLowerCase().includes(q)
        || n.tags.some((t) => t.toLowerCase().includes(q))
        || n.path.toLowerCase().includes(q)
      ) {
        ids.add(n.id);
      }
    });
    setHighlightedNodeIds(ids);
  };

  const selectedNode = graphData?.nodes.find((n) => n.id === selectedNodeId);

  const contextMenuNode = graphData?.nodes.find((n) => n.id === contextMenu.nodeId);

  // 统计
  const stats = useMemo(() => {
    if (!graphData) { return { nodes: 0, edges: 0, tags: 0 }; }
    const tags = new Set<string>();
    graphData.nodes.forEach((n) => n.tags.forEach((t) => tags.add(t)));
    return { nodes: graphData.nodes.length, edges: graphData.edges.length, tags: tags.size };
  }, [graphData]);

  return (
    <div className="h-full flex flex-col" style={{ overflow: "hidden", backgroundColor: token.colorBgLayout }}>
      {/* 工具栏 — 玻璃态 */}
      <div
        className="flex items-center gap-2 px-4 py-2 shrink-0 backdrop-blur-lg z-10"
        style={{
          borderBottom: `1px solid ${token.colorBorderSecondary}20`,
          backgroundColor: `${token.colorBgContainer}cc`,
          boxShadow: `0 1px 3px ${token.colorBgContainer}40`,
        }}
      >
        <NodeIndexOutlined style={{ color: token.colorPrimary, fontSize: 18 }} />
        <Title level={5} style={{ margin: 0 }}>
          {t("wiki.graph.title")}
        </Title>

        <Select
          size="small"
          value={wikiIdFromUrl}
          onChange={(val) => navigate(`/wiki/${val}`)}
          style={{ minWidth: 160, marginLeft: 8 }}
          options={wikis.map((w) => ({ label: w.name, value: w.id }))}
          placeholder={t("wiki.selectWiki")}
        />

        <div className="flex-1" />

        <Input
          id="wiki-graph-page-input-133"
          size="small"
          prefix={<SearchOutlined />}
          placeholder={t("wiki.searchGraph")}
          value={globalSearch}
          onChange={(e) => handleGlobalSearch(e.target.value)}
          allowClear
          style={{ width: 200 }}
        />

        <Space size={4}>
          <Tag style={{ margin: 0 }}>
            {stats.nodes} {t("wiki.nodes")}
          </Tag>
          <Tag style={{ margin: 0 }}>
            {stats.edges} {t("wiki.edges")}
          </Tag>
        </Space>

        <Tooltip title={leftPanelVisible ? t("wiki.hidePanel") : t("wiki.showPanel")}>
          <Button
            size="small"
            type="text"
            icon={leftPanelVisible ? <PanelLeft size={14} /> : <PanelRight size={14} />}
            onClick={() => setLeftPanelVisible(!leftPanelVisible)}
          />
        </Tooltip>

        {!detailPanelOpen && selectedNodeId && (
          <Button
            size="small"
            type="text"
            icon={<Eye size={14} />}
            onClick={() => setDetailPanelOpen(true)}
          >
            {t("wiki.showDetail")}
          </Button>
        )}

        <Tooltip title={t("wiki.newNote")}>
          <Button size="small" icon={<FileAddOutlined />} onClick={handleCreateNote} />
        </Tooltip>

        <Tooltip title={t("wiki.refresh")}>
          <Button size="small" icon={<ReloadOutlined />} onClick={handleReload} loading={graphLoading} />
        </Tooltip>
      </div>

      {/* 主工作区 */}
      <div className="flex-1 flex overflow-hidden">
        {/* 左侧面板 */}
        {leftPanelVisible && (
          <>
            <div style={{ width: leftPanelWidth, flexShrink: 0, overflow: "hidden" }}>
              <WikiFilePanel
                notes={notes}
                graphData={graphData}
                loading={notesLoading}
                selectedNodeId={selectedNodeId}
                highlightedNodeIds={highlightedNodeIds}
                onSelectNode={handleNavigateToNote}
                onSearchHighlight={handleSearchHighlight}
              />
            </div>
            {/* 左拖曳手柄 */}
            <div
              className="shrink-0 cursor-col-resize select-none transition-all duration-300"
              style={{
                width: leftAtBoundary ? 5 : 3,
                background: leftAtBoundary
                  ? `linear-gradient(to right, ${token.colorWarningBg}60, ${token.colorWarning}80, ${token.colorWarningBg}60)`
                  : `linear-gradient(to right, transparent, ${token.colorBorderSecondary}10, transparent)`,
              }}
              onMouseDown={handleResizeStart("left")}
              onMouseEnter={(e) => {
                if (!leftAtBoundary) {
                  e.currentTarget.style.width = "5px";
                  e.currentTarget.style.background =
                    `linear-gradient(to right, ${token.colorPrimaryBg}40, ${token.colorPrimary}60, ${token.colorPrimaryBg}40)`;
                }
              }}
              onMouseLeave={(e) => {
                if (!leftAtBoundary) {
                  e.currentTarget.style.width = "3px";
                  e.currentTarget.style.background = "";
                }
              }}
            />
          </>
        )}

        {/* 中央图谱 */}
        <div className="flex-1" style={{ minWidth: 0 }}>
          {graphLoading
            ? (
              <div className="h-full flex items-center justify-center">
                <Spin size="large" tip={t("wiki.graph.loading")} />
              </div>
            )
            : !graphData || graphData.nodes.length === 0
            ? (
              <div className="h-full flex items-center justify-center">
                <Empty description={t("wiki.graph.empty")}>
                  <Button type="primary" onClick={handleCreateNote}>
                    {t("wiki.createFirstNote")}
                  </Button>
                </Empty>
              </div>
            )
            : (
              <GraphView
                data={graphData}
                onNodeClick={handleNodeClick}
                onNodeDoubleClick={handleNodeDoubleClick}
                onContextMenu={handleContextMenu}
                onDeleteNode={handleDeleteNote}
                onDeselect={() => {
                  setSelectedNodeId(null);
                  setHighlightedNodeIds(new Set());
                  setDetailPanelOpen(false);
                }}
                selectedNodeId={selectedNodeId}
                highlightedNodeIds={highlightedNodeIds}
                communities={communities ?? undefined}
                showMinimap
              />
            )}
        </div>

        {/* 右侧详情面板 */}
        {detailPanelOpen && (
          <>
            {/* 右拖曳手柄 */}
            <div
              className="shrink-0 cursor-col-resize select-none transition-all duration-300"
              style={{
                width: rightAtBoundary ? 5 : 3,
                background: rightAtBoundary
                  ? `linear-gradient(to right, ${token.colorWarningBg}60, ${token.colorWarning}80, ${token.colorWarningBg}60)`
                  : `linear-gradient(to right, transparent, ${token.colorBorderSecondary}10, transparent)`,
              }}
              onMouseDown={handleResizeStart("right")}
              onMouseEnter={(e) => {
                if (!rightAtBoundary) {
                  e.currentTarget.style.width = "5px";
                  e.currentTarget.style.background =
                    `linear-gradient(to right, ${token.colorPrimaryBg}40, ${token.colorPrimary}60, ${token.colorPrimaryBg}40)`;
                }
              }}
              onMouseLeave={(e) => {
                if (!rightAtBoundary) {
                  e.currentTarget.style.width = "3px";
                  e.currentTarget.style.background = "";
                }
              }}
            />
            <div style={{ width: rightPanelWidth, flexShrink: 0, overflow: "hidden" }}>
              <WikiDetailPanel
                noteId={selectedNodeId}
                graphData={graphData}
                onClose={() => setDetailPanelOpen(false)}
                onNoteUpdated={handleNoteUpdated}
                onNavigateToNote={handleNavigateToNote}
              />
            </div>
          </>
        )}
      </div>

      {/* 底部状态栏 — 玻璃态 */}
      <div
        className="flex items-center gap-3 px-4 py-1.5 shrink-0 backdrop-blur-lg z-10"
        style={{
          borderTop: `1px solid ${token.colorBorderSecondary}20`,
          backgroundColor: `${token.colorBgContainer}dd`,
          fontSize: 11,
        }}
      >
        <Text type="secondary">
          {t("wiki.wiki")}: {wikiIdFromUrl}
        </Text>
        {selectedNodeId && (
          <Text type="secondary">
            {t("wiki.selected")}: {selectedNode?.title || selectedNodeId}
            {selectedNode && ` (→${selectedNode.linkCount} / ←${selectedNode.backlinkCount})`}
          </Text>
        )}
        <div className="flex-1" />
        <Text type="secondary">
          {t("wiki.tips.doubleClick")} · {t("wiki.tips.dragPanel")} · {t("wiki.tips.rightClick")} ·{" "}
          {t("wiki.tips.keyboard")}
        </Text>
      </div>

      {/* 右键菜单 */}
      <WikiNodeContextMenu
        visible={contextMenu.visible}
        position={contextMenu.position}
        nodeId={contextMenu.nodeId}
        nodeTitle={contextMenuNode?.title || ""}
        onClose={() => setContextMenu((c) => ({ ...c, visible: false }))}
        onEdit={(id) => {
          setSelectedNodeId(id);
          setDetailPanelOpen(true);
        }}
        onViewBacklinks={(id) => {
          setSelectedNodeId(id);
          setDetailPanelOpen(true);
        }}
        onFocusLocal={() => {
          if (contextMenu.nodeId && graphData) {
            const neighborIds = new Set<string>();
            graphData.edges.forEach((e) => {
              if (e.source === contextMenu.nodeId) { neighborIds.add(e.target); }
              if (e.target === contextMenu.nodeId) { neighborIds.add(e.source); }
            });
            neighborIds.add(contextMenu.nodeId);
            setHighlightedNodeIds(neighborIds);
          }
        }}
        onCreateLinked={handleCreateLinkedNote}
        onDelete={handleDeleteNote}
      />
    </div>
  );
}
