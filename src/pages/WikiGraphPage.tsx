// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { QualityScore } from "@/components/llm-wiki/QualityScore";
import { SyncStatus } from "@/components/llm-wiki/SyncStatus";
import { GraphData, GraphView } from "@/components/wiki/GraphView";
import { WikiDetailPanel } from "@/components/wiki/WikiDetailPanel";
import { WikiFilePanel } from "@/components/wiki/WikiFilePanel";
import { WikiNodeContextMenu } from "@/components/wiki/WikiNodeContextMenu";
import { showBackendError } from "@/lib/errorI18n";
import { invoke } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { useWikiStore } from "@/stores/feature/wikiStore";
import { BookOutlined, FileAddOutlined, NodeIndexOutlined, ReloadOutlined, SearchOutlined } from "@ant-design/icons";
import { Button, Empty, Input, Select, Space, Spin, Tag, theme, Typography } from "antd";
import { Eye, PanelLeft, PanelRight } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
const { Title } = Typography;
const MIN_PANEL_WIDTH = 180;
const MAX_LEFT_PANEL = 400;
const MAX_RIGHT_PANEL = 600;

export function WikiGraphPage() {
  const { token } = theme.useToken();
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { wikiId } = useParams<{ wikiId: string }>();
  const [searchParams] = useSearchParams();
  const urlWikiId = searchParams.get("wikiId") || wikiId || null;

  const { wikis, loading: wikisLoading, loadWikis } = useLlmWikiStore();
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
  const [communities, setCommunities] = useState<Map<string, number> | null>(
    null,
  );

  // 选中和高亮
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [highlightedNodeIds, setHighlightedNodeIds] = useState<Set<string>>(
    new Set(),
  );
  const [detailPanelOpen, setDetailPanelOpen] = useState(false);

  // 右键菜单
  const [contextMenu, setContextMenu] = useState<{
    visible: boolean;
    nodeId: string;
    position: { x: number; y: number };
  }>({ visible: false, nodeId: "", position: { x: 0, y: 0 } });

  // 面板宽度拖曳（默认更窄，最大化图谱）
  const [leftPanelWidth, setLeftPanelWidth] = useState(200);
  const [rightPanelWidth, setRightPanelWidth] = useState(340);
  const [leftPanelVisible, setLeftPanelVisible] = useState(true);
  const [leftAtBoundary, setLeftAtBoundary] = useState<"min" | "max" | null>(
    null,
  );
  const [rightAtBoundary, setRightAtBoundary] = useState<"min" | "max" | null>(
    null,
  );
  const resizingRef = useRef<"left" | "right" | null>(null);
  const [resizingSide, setResizingSide] = useState<"left" | "right" | null>(null);
  useEffect(() => {
    resizingRef.current = resizingSide;
  }, [resizingSide]);

  // 搜索
  const [globalSearch, setGlobalSearch] = useState("");

  // wikiIdFromUrl — 在 wiki 列表加载完成后验证有效性，避免硬编码 fallback
  const [wikiIdFromUrl, setWikiIdFromUrl] = useState<string | null>(null);
  const [wikisLoaded, setWikisLoaded] = useState(false);

  // 加载 Wiki 列表
  useEffect(() => {
    loadWikis().then(() => setWikisLoaded(true));
  }, [loadWikis]);

  // wiki 列表加载完成后：验证 urlWikiId 是否有效；无效或不存在时导航到首个可用 wiki
  useEffect(() => {
    if (!wikisLoaded) {
      return;
    }
    if (wikis.length > 0) {
      const valid = urlWikiId && wikis.some((w) => w.id === urlWikiId);
      if (valid) {
        setWikiIdFromUrl(urlWikiId);
      } else {
        navigate(`/llm-wiki/${wikis[0].id}/graph`, { replace: true });
      }
    } else {
      // wikis 为空列表且已加载完毕 → 无可用 wiki
      setWikiIdFromUrl(null);
    }
  }, [wikisLoaded, wikis, urlWikiId, navigate]);

  const loadGraphData = useCallback(async () => {
    if (!wikiIdFromUrl) {
      setGraphData(null);
      setGraphLoading(false);
      return;
    }
    setGraphLoading(true);
    try {
      // 走缓存版命令：10万节点命中缓存 < 10ms，未命中自动计算并写缓存
      const [data, communityResult] = await Promise.all([
        invoke<GraphData>("get_wiki_graph_cached", { wikiId: wikiIdFromUrl }),
        invoke<{ communities: Record<string, number> }>(
          "wiki_graph_communities_cached",
          { wikiId: wikiIdFromUrl },
        ).catch(() => null),
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
    if (!wikiIdFromUrl) {
      return;
    }
    setSelectedVaultId(wikiIdFromUrl);
    loadNotes(wikiIdFromUrl);
    setTimeout(() => loadGraphData(), 0);
  }, [wikiIdFromUrl, setSelectedVaultId, loadNotes, loadGraphData]);

  const handleReload = () => {
    if (!wikiIdFromUrl) {
      return;
    }
    loadNotes(wikiIdFromUrl);
    loadGraphData();
  };

  // 面板拖曳
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (resizingRef.current === "left") {
        const clamped = Math.max(
          MIN_PANEL_WIDTH,
          Math.min(MAX_LEFT_PANEL, e.clientX),
        );
        setLeftPanelWidth(clamped);
        setLeftAtBoundary(
          clamped <= MIN_PANEL_WIDTH
            ? "min"
            : clamped >= MAX_LEFT_PANEL
            ? "max"
            : null,
        );
      } else if (resizingRef.current === "right") {
        const clamped = Math.max(
          MIN_PANEL_WIDTH,
          Math.min(MAX_RIGHT_PANEL, window.innerWidth - e.clientX),
        );
        setRightPanelWidth(clamped);
        setRightAtBoundary(
          clamped <= MIN_PANEL_WIDTH
            ? "min"
            : clamped >= MAX_RIGHT_PANEL
            ? "max"
            : null,
        );
      }
    };
    const handleMouseUp = () => {
      setResizingSide(null);
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
    setResizingSide(side);
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

  const handleDeselect = useCallback(() => {
    setSelectedNodeId(null);
    setHighlightedNodeIds(new Set());
    setDetailPanelOpen(false);
  }, []);

  const handleNavigateToNote = useCallback((noteId: string) => {
    setSelectedNodeId(noteId);
    setDetailPanelOpen(true);
  }, []);

  const handleCreateNote = useCallback(async () => {
    if (!wikiIdFromUrl) {
      return;
    }
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
  }, [wikiIdFromUrl, createNote, loadNotes, loadGraphData, i18n.language, t]);

  const { importKnowledgeMd } = useWikiStore();
  const [importingMd, setImportingMd] = useState(false);

  const handleImportKnowledgeMd = useCallback(async () => {
    if (!wikiIdFromUrl) {
      return;
    }
    setImportingMd(true);
    try {
      const stats = await importKnowledgeMd(wikiIdFromUrl);
      if (stats) {
        message.success(
          t("wiki.importKnowledgeMdResult", {
            imported: stats.imported,
            skipped: stats.skipped,
            failed: stats.failed,
          }),
        );
        loadNotes(wikiIdFromUrl);
        loadGraphData();
      }
    } catch (e) {
      showBackendError(message, e);
    }
    setImportingMd(false);
  }, [wikiIdFromUrl, importKnowledgeMd, loadNotes, loadGraphData, t]);

  const handleCreateLinkedNote = useCallback(
    async (sourceNodeId: string) => {
      if (!wikiIdFromUrl) {
        return;
      }
      const sourceNode = graphData?.nodes.find((n) => n.id === sourceNodeId);
      const title = sourceNode
        ? `${t("wiki.linkedPrefix")}: ${sourceNode.title}`
        : t("wiki.linkedNoteTitle");
      const now = Date.now();
      const note = await createNote({
        vaultId: wikiIdFromUrl,
        title,
        filePath: `/linked-note-${now}.md`,
        content: sourceNode
          ? `${t("wiki.linkedRef")}: [[${sourceNode.title}]]`
          : "",
        author: "user",
      });
      if (note) {
        loadNotes(wikiIdFromUrl);
        loadGraphData();
        setSelectedNodeId(note.id);
        setDetailPanelOpen(true);
      }
    },
    [wikiIdFromUrl, graphData, createNote, loadNotes, loadGraphData, t],
  );

  const handleDeleteNote = useCallback(
    async (nodeId: string) => {
      if (!wikiIdFromUrl) {
        return;
      }
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
        showBackendError(message, e);
      }
    },
    [deleteNote, selectedNodeId, wikiIdFromUrl, loadNotes, loadGraphData, t],
  );

  const handleNoteUpdated = () => {
    if (!wikiIdFromUrl) {
      return;
    }
    loadNotes(wikiIdFromUrl);
    loadGraphData();
  };

  const handleGlobalSearch = useCallback(
    (value: string) => {
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
    },
    [graphData],
  );

  const selectedNode = useMemo(
    () => graphData?.nodes.find((n) => n.id === selectedNodeId),
    [graphData, selectedNodeId],
  );

  const contextMenuNode = useMemo(
    () => graphData?.nodes.find((n) => n.id === contextMenu.nodeId),
    [graphData, contextMenu.nodeId],
  );

  // 统计
  const stats = useMemo(() => {
    if (!graphData) {
      return { nodes: 0, edges: 0, tags: 0 };
    }
    const tags = new Set<string>();
    graphData.nodes.forEach((n) => n.tags.forEach((t) => tags.add(t)));
    return {
      nodes: graphData.nodes.length,
      edges: graphData.edges.length,
      tags: tags.size,
    };
  }, [graphData]);

  return (
    <div
      className="h-full flex flex-col"
      style={{ overflow: "hidden", backgroundColor: token.colorBgLayout }}
    >
      {/* 工具栏 — 极致紧凑，最大化图谱空间 */}
      <div
        className="flex items-center gap-1 px-2 py-1 shrink-0 backdrop-blur-lg z-10"
        style={{
          borderBottom: `1px solid ${token.colorBorderSecondary}30`,
          backgroundColor: `${token.colorBgContainer}ee`,
        }}
      >
        <NodeIndexOutlined
          style={{ color: token.colorPrimary, fontSize: 16 }}
        />
        <Title level={5} style={{ margin: 0, fontSize: 14 }}>
          {t("wiki.graph.title")}
        </Title>

        {wikis.length > 0 && (
          <Select
            size="small"
            value={wikiIdFromUrl ?? undefined}
            onChange={(val) => navigate(`/llm-wiki/${val}/graph`)}
            style={{ minWidth: 130 }}
            options={wikis.map((w) => ({ label: w.name, value: w.id }))}
            placeholder={t("wiki.selectWiki")}
          />
        )}

        <Input
          size="small"
          prefix={<SearchOutlined />}
          placeholder={t("wiki.searchGraph")}
          value={globalSearch}
          onChange={(e) => handleGlobalSearch(e.target.value)}
          allowClear
          style={{ width: 160 }}
        />

        <Space size={2}>
          <Tag style={{ margin: 0, fontSize: 11, lineHeight: "18px" }}>
            {stats.nodes}N
          </Tag>
          <Tag style={{ margin: 0, fontSize: 11, lineHeight: "18px" }}>
            {stats.edges}E
          </Tag>
        </Space>

        {/* 选中节点信息行内展示 */}
        {selectedNodeId && selectedNode && (
          <span
            className="text-xs truncate max-w-[160px]"
            style={{ color: token.colorTextSecondary }}
            title={`${selectedNode.title} (→${selectedNode.linkCount} / ←${selectedNode.backlinkCount})`}
          >
            | {selectedNode.title}
          </span>
        )}

        <div className="flex-1" />

        <Tooltip
          title={leftPanelVisible ? t("wiki.hidePanel") : t("wiki.showPanel")}
        >
          <Button
            size="small"
            type="text"
            icon={leftPanelVisible ? <PanelLeft size={13} /> : <PanelRight size={13} />}
            onClick={() => setLeftPanelVisible(!leftPanelVisible)}
          />
        </Tooltip>

        {!detailPanelOpen && selectedNodeId && (
          <Button
            size="small"
            type="text"
            icon={<Eye size={13} />}
            onClick={() => setDetailPanelOpen(true)}
          />
        )}

        <Tooltip title={t("wiki.newNote")}>
          <Button
            size="small"
            icon={<FileAddOutlined />}
            onClick={handleCreateNote}
          />
        </Tooltip>

        <Tooltip title={t("wiki.importKnowledgeMdDesc")}>
          <Button
            size="small"
            icon={<BookOutlined />}
            onClick={handleImportKnowledgeMd}
            loading={importingMd}
          />
        </Tooltip>

        <Tooltip title={t("wiki.refresh")}>
          <Button
            size="small"
            icon={<ReloadOutlined />}
            onClick={handleReload}
            loading={graphLoading}
          />
        </Tooltip>

        {wikiIdFromUrl && <SyncStatus wikiId={wikiIdFromUrl} compact />}
        {wikiIdFromUrl && <QualityScore wikiId={wikiIdFromUrl} compact />}
      </div>

      {/* 主工作区 */}
      <div className="flex-1 flex overflow-hidden">
        {/* 左侧面板 */}
        {leftPanelVisible && (
          <>
            <div
              style={{
                width: leftPanelWidth,
                flexShrink: 0,
                overflow: "hidden",
              }}
            >
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
              role="separator"
              tabIndex={0}
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
          {wikisLoading
            ? (
              <div className="h-full flex items-center justify-center">
                <Spin size="large" />
              </div>
            )
            : wikiIdFromUrl === null
            ? (
              <div className="h-full flex items-center justify-center">
                <Empty description={t("wiki.selectWikiPrompt")} />
              </div>
            )
            : graphLoading
            ? (
              <div className="h-full flex items-center justify-center">
                <Spin size="large" description={t("wiki.graph.loading")} />
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
                onDeselect={handleDeselect}
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
              role="separator"
              tabIndex={0}
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
            <div
              style={{
                width: rightPanelWidth,
                flexShrink: 0,
                overflow: "hidden",
              }}
            >
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
              if (e.source === contextMenu.nodeId) {
                neighborIds.add(e.target);
              }
              if (e.target === contextMenu.nodeId) {
                neighborIds.add(e.source);
              }
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
