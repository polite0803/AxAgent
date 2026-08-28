// SPDX-License-Identifier: AGPL-3.0-only

import {
  Background,
  BackgroundVariant,
  Connection,
  ConnectionLineType,
  Controls,
  type Edge,
  type EdgeChange,
  MiniMap,
  type Node,
  type NodeChange,
  Panel,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
  useUpdateNodeInternals,
} from "@xyflow/react";
import html2canvas from "html2canvas";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import "@xyflow/react/dist/style.css";
import { invoke, isTauri, logIpcError } from "@/lib/invoke";
import {
  autoLayout,
  autoLayoutWorkflow,
  type AutoNode,
  findSafePosition,
  getNodeSize,
  type NodePositionLike,
  toAbsolutePosition,
  toRelativePosition,
  validateWorkflow,
  wouldCreateCycle,
} from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";

import { showBackendError } from "@/lib/errorI18n";
import { message } from "@/lib/toast";
import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { Button, Modal, Spin, theme } from "antd";
import { useTranslation } from "react-i18next";
import { AIPanel } from "./AIPanel/AIPanel";
import { DebugPanel } from "./DebugPanel";
import { DiagnosticDrawer } from "./Diagnostic";
import { clearDragPayload, getDragPayload } from "./dndState";
import { BaseEdge } from "./Edges/BaseEdge";
import { EdgeMarkers } from "./Edges/EdgeMarkers";
import { EditorHeader } from "./Header/EditorHeader";
import { useFlowNodes, useKeyboardShortcuts } from "./Hooks";
import {
  useWorkflowAutoSave,
  useWorkflowDragPosition,
  useWorkflowLayout,
  useWorkflowPanelState,
  useWorkflowValidation,
} from "./Hooks";
import {
  AgentNode,
  AggregatorNode,
  ApprovalNode,
  BaseNode,
  CodeNode,
  ConditionNode,
  DatabaseQueryNode,
  DataTransformerNode,
  DebateNode,
  DelayNode,
  DocumentParserNode,
  EmailNode,
  EndNode,
  FileOperationNode,
  GroupFrameNode,
  HttpRequestNode,
  LlmClassifierNode,
  LLMNode,
  LoggingNode,
  LoopNode,
  MergeNode,
  MultiAgentNode,
  NotificationNode,
  ParallelNode,
  PhaseSeparatorNode,
  StorageNode,
  SubWorkflowNode,
  SwarmNode,
  SwitchNode,
  ToolNode,
  TriggerNode,
  ValidationNode,
  VectorRetrieveNode,
  WebhookSendNode,
  WorkflowRefNode,
} from "./Nodes";
import { BatchEditPanel } from "./Panels/BatchEditPanel";
import { LeftPanel } from "./Panels/LeftPanel";
import { RightPanel } from "./Panels/RightPanel";
import { SemanticCheckModal } from "./SemanticCheckModal";
import { StatusBar } from "./StatusBar/EditorStatusBar";
import { ImportExportModal } from "./Templates/ImportExportModal";
import { VersionHistoryModal } from "./Templates/VersionHistoryModal";
import { WorkflowToolsPanel } from "./Tools/WorkflowToolsPanel";
import { NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "./types";
import { buildNodesWithParent, getCleanedEdges } from "./utils";
import { WorkflowLegend } from "./WorkflowLegend";

const nodeTypes = {
  base: BaseNode,
  trigger: TriggerNode,
  agent: AgentNode,
  llm: LLMNode,
  condition: ConditionNode,
  parallel: ParallelNode,
  loop: LoopNode,
  merge: MergeNode,
  multiAgent: MultiAgentNode,
  delay: DelayNode,
  tool: ToolNode,
  code: CodeNode,
  subWorkflow: SubWorkflowNode,
  workflowRef: WorkflowRefNode,
  documentParser: DocumentParserNode,
  vectorRetrieve: VectorRetrieveNode,
  validation: ValidationNode,
  end: EndNode,
  httpRequest: HttpRequestNode,
  debate: DebateNode,
  swarm: SwarmNode,
  storage: StorageNode,
  switch: SwitchNode,
  databaseQuery: DatabaseQueryNode,
  notification: NotificationNode,
  approval: ApprovalNode,
  fileOperation: FileOperationNode,
  dataTransformer: DataTransformerNode,
  webhookSend: WebhookSendNode,
  logging: LoggingNode,
  llmClassifier: LlmClassifierNode,
  aggregator: AggregatorNode,
  email: EmailNode,
  _phaseSeparator: PhaseSeparatorNode,
  groupFrame: GroupFrameNode,
};

const edgeTypes = {
  base: BaseEdge,
};

const defaultEdgeOptions = {
  type: "smoothstep",
  animated: false,
  style: { stroke: "#666", strokeWidth: 1.5, borderRadius: 4 },
};

interface WorkflowEditorProps {
  templateId?: string;
  /** 是否为系统模板（认知编排器等）：加载时透传 include_system=true */
  isSystemTemplate?: boolean;
  onClose?: () => void;
}

export const WorkflowEditor: React.FC<WorkflowEditorProps> = ({
  templateId,
  isSystemTemplate,
  onClose,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const {
    currentTemplate,
    nodes,
    edges,
    parentRefs,
    setParentRef,
    isLoading,
    isSaving,
    isDirty,
    validationResult,
    loadTemplate,
    initNewTemplate,
    updateNode,
    deleteNode,
    deleteEdge,
    addEdge: storeAddEdge,
    setSelectedNode,
    setSelectedEdge,
    selectedNodeId,
    selectedEdgeId,
    updateTemplate,
    updateTemplateMetadata,
    createTemplate,
    validateTemplate,
    error,
    undo,
    redo,
    canUndo,
    canRedo,
    addNode,
    semanticCheckResult,
    clearSemanticCheckResult,
    applySkillReplacement,
    collapsedContainers,
    runWorkflowDiagnose,
    diagnoseLoading,
    diagnoseDrawerVisible,
    setDiagnoseDrawerVisible,
  } = useWorkflowEditorStore();

  // ── 并发安全控制器（新增 Hooks） ──────────────────────────
  const dragCtrl = useWorkflowDragPosition();
  const layoutCtrl = useWorkflowLayout();
  const panelCtrl = useWorkflowPanelState();
  const { issues: frontendValidation, msgMap: validationMsgMap } = useWorkflowValidation(
    nodes,
    edges,
    t,
  );
  const autoSaveCtrl = useWorkflowAutoSave();

  // ── 兼容旧代码：将新 Hooks 的控制器映射到旧变量名 ──────────
  const hasAutoLaidOutRef = layoutCtrl.hasAutoLaidOutRef;
  const autoLayoutTimerRef = { current: null as ReturnType<typeof setTimeout> | null };
  const skipPositionWriteRef = layoutCtrl.skipPositionWriteRef;
  const isDraggingRef = dragCtrl.isDraggingRef;
  const suppressRebuildRef = dragCtrl.suppressRebuildRef;
  const autoSaveTimerRef = { current: null as ReturnType<typeof setTimeout> | null };
  const leftPanelCollapsed = panelCtrl.leftPanelCollapsed;
  const rightPanelCollapsed = panelCtrl.rightPanelCollapsed;
  const leftPanelWidth = panelCtrl.leftPanelWidth;
  const rightPanelWidth = panelCtrl.rightPanelWidth;
  const setLeftPanelCollapsed = panelCtrl.setLeftPanelCollapsed;
  const setRightPanelCollapsed = panelCtrl.setRightPanelCollapsed;
  const resizing = panelCtrl.resizing;
  const setResizing = panelCtrl.setResizing;

  // ── React Flow 状态（保持不变） ──────────────────────────
  const [reactFlowNodes, setRNodes, onNodesChange] = useNodesState<Node>([]);
  const [reactFlowEdges, setREdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [isInitialized, setIsInitialized] = React.useState(false);
  const canvasContainerRef = React.useRef<HTMLDivElement>(null);
  const clipboardRef = React.useRef<WorkflowNode[]>([]);
  const edgesRef = React.useRef(edges);
  useEffect(() => {
    edgesRef.current = edges;
  }, [edges]);
  const removeIdsRef = React.useRef<Set<string>>(new Set());
  const [, setDragStopVersion] = useState(0);

  // ── UI 状态（保留，面板状态已由 panelCtrl 接管） ──────────
  const [aiPanelVisible, setAiPanelVisible] = useState(false);
  const [aiPanelHeight, setAiPanelHeight] = useState(300);
  const [debugPanelVisible, setDebugPanelVisible] = useState(false);
  const [importExportModalVisible, setImportExportModalVisible] = useState(false);
  const [versionHistoryVisible, setVersionHistoryVisible] = useState(false);
  const [toolsPanelVisible, setToolsPanelVisible] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; nodeId: string } | null>(null);
  const [searchVisible, setSearchVisible] = useState(false);
  const [selectedNodeIds, setSelectedNodeIds] = useState<Set<string>>(new Set());
  const [batchEditVisible, setBatchEditVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchIdx, setSearchIdx] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [dndDropTargetId, setDndDropTargetId] = useState<string | null>(null);

  const { flowNodes: computedFlowNodes, flowEdges: computedFlowEdges, expectedParentByNode } = useFlowNodes({
    nodes,
    edges,
    parentRefs,
    collapsedContainers,
    validationResult,
    frontendValidation,
    validationMsgMap,
    token,
  });

  const handleSaveRef = React.useRef<() => void>(() => {});
  useKeyboardShortcuts({
    undo,
    redo,
    canUndo,
    canRedo,
    selectedNodeId,
    deleteNode,
    nodes,
    addNode,
    setSelectedNode,
    setParentRef,
    updateNode,
    clipboardRef,
    handleSaveRef,
    setSearchVisible,
  });

  const {
    isDecompositionTemplate,
    saveSkillWorkflowFromLlm,
    generateWorkflowFromPrompt,
    optimizeAgentPrompt,
    recommendNodes,
    applyOptimizedPromptToNode,
    aiChatMessages,
    aiChatStreaming,
    aiChatSend,
    aiChatCancel,
    aiChatClear,
    // applyAiChatAction,
    exportTemplate,
    importTemplate,
    loadTemplates,
    templates,
  } = useWorkflowEditorStore();

  useEffect(() => {
    hasAutoLaidOutRef.current = false;
    console.log("[WorkflowEditor.useEffect] 触发 loadTemplate", {
      templateId,
      isSystemTemplate,
    });
    if (templateId) {
      loadTemplate(templateId, isSystemTemplate)
        .catch(logIpcError("WorkflowEditor: loadTemplate"));
    } else {
      initNewTemplate();
    }
  }, [templateId, isSystemTemplate, loadTemplate, initNewTemplate]);

  // 初始化 workEngine 事件监听器：实时接收 node-status-changed / execution-completed 事件
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let cancelled = false;
    useWorkEngineStore.getState().setupEventListeners().then((fn) => {
      if (cancelled) { fn(); }
      else { cleanup = fn; }
    });
    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, []);

  /** 自动保存逻辑已迁移至 useWorkflowAutoSave Hook */

  const updateNodeInternals = useUpdateNodeInternals();

  useEffect(() => {
    if (isDraggingRef.current || suppressRebuildRef.current) { return; }

    setRNodes(computedFlowNodes);
    setREdges(computedFlowEdges);
    setIsInitialized(true);

    // 折叠/展开后容器节点的 width/height 发生变化，需要在 setRNodes 完成后
    // 强制 React Flow 重新测量 internals，否则 handleBounds 缓存仍是旧尺寸，
    // edge 锚点错位导致连线断开。
    const remeasureIds = containerIdsNeedingRemeasureRef.current;
    if (remeasureIds.length > 0) {
      containerIdsNeedingRemeasureRef.current = [];
      requestAnimationFrame(() => {
        updateNodeInternals(remeasureIds);
      });
    }

    for (const [childId, expectedParent] of Object.entries(expectedParentByNode)) {
      if (parentRefs[childId] !== expectedParent) {
        setParentRef(childId, expectedParent);
      }
    }

    /** ⚠️ 关键：当 expectedParentByNode 有 pending 的 parentRefs 同步时，不可在此轮
     *  调度 autoLayout。因为 autoLayout 的回调闭包捕获了当前轮次的 computedFlowNodes，
     *  此时子节点尚未获得 parentId（parentRefs 还没同步），autoLayout 会把容器内子节点
     *  当作独立节点重新布局，产生错误的位置，然后通过 updateNode 写入 store 造成数据污染。
     *
     *  setParentRef 会触发同步状态更新 + 新一轮渲染，autoLayout 将在新一轮渲染的
     *  useEffect 中重新评估。 */
    const pendingParentSync = Object.keys(expectedParentByNode).length > 0
      && Object.entries(expectedParentByNode).some(([cid, pid]) => parentRefs[cid] !== pid);

    if (pendingParentSync) {
      // 关键：不能在这里把 hasAutoLaidOutRef 设为 true。
      // 否则下一轮 useEffect 因为 ref 已经是 true 而跳过 autoLayout，
      // 整个工作流就停留在初始错乱位置。
      // 应该等下一轮 useEffect 重新评估（setParentRef 已同步 parentRefs），
      // pendingParentSync 此时变为 false，autoLayout 才会被调用。
      // 用 scheduleAutoLayoutNextTick 标记"等下一轮再排"
    }

    if (!hasAutoLaidOutRef.current && nodes.length >= 2) {
      const hasOverlap = (() => {
        const posMap = new Map<string, number>();
        for (const n of nodes) {
          const key = `${Math.round(n.position.x / 10)},${Math.round(n.position.y / 10)}`;
          posMap.set(key, (posMap.get(key) || 0) + 1);
        }
        return Array.from(posMap.values()).some((count) => count > 1);
      })();

      const hasReasonablePositions = nodes.every((n) => n.position.x >= 50 || n.position.y >= 50);
      const skipAutoLayout = hasReasonablePositions && !hasOverlap;

      if (!skipAutoLayout) {
        hasAutoLaidOutRef.current = true;
        autoLayoutTimerRef.current = setTimeout(() => {
          const { nodes: layouted, edges: layoutedE } = autoLayoutWorkflow(
            computedFlowNodes,
            computedFlowEdges,
            parentRefs,
          );
          // 关键：React Flow 的 parent/child 模式要求父节点在 nodes 数组中
          // 排在子节点前面，否则会警告 "Parent node xxx not found" 且
          // 子节点会跑出容器外。这里在 setRNodes 前排序保证顺序正确。
          const sortedLayouted = [...layouted].sort((a, b) => {
            const aPid = parentRefs[a.id];
            const bPid = parentRefs[b.id];
            // a 的父是 b → a 在 b 后面
            if (aPid === b.id) { return 1; }
            // b 的父是 a → a 在 b 前面
            if (bPid === a.id) { return -1; }
            return 0;
          });
          skipPositionWriteRef.current = true;
          setRNodes(sortedLayouted);
          setREdges(layoutedE);
          requestAnimationFrame(() => {
            skipPositionWriteRef.current = false;
          });
          for (const ln of layouted) {
            const pid = parentRefs[ln.id];
            if (pid) {
              const parentLn = layouted.find((n) => n.id === pid);
              if (parentLn) {
                const absPos = toAbsolutePosition(
                  ln.id,
                  ln.position,
                  parentRefs,
                  layouted.map((n) => ({ id: n.id, position: n.position })) as NodePositionLike[],
                );
                updateNode(ln.id, {
                  position: absPos,
                } as Partial<WorkflowNode>);
                continue;
              }
            }
            updateNode(ln.id, { position: ln.position } as Partial<WorkflowNode>);
          }
        }, 100);
      } else {
        hasAutoLaidOutRef.current = true;
      }
    }
    return () => {
      if (autoLayoutTimerRef.current) {
        clearTimeout(autoLayoutTimerRef.current);
        autoLayoutTimerRef.current = null;
      }
    };
  }, [
    computedFlowNodes,
    computedFlowEdges,
    expectedParentByNode,
    parentRefs,
    nodes,
    setParentRef,
    updateNode,
    setRNodes,
    setREdges,
    updateNodeInternals,
  ]);

  const onConnect = useCallback(
    (params: Connection) => {
      if (!params.source || !params.target) { return; }
      // 禁止自循环
      if (params.source === params.target) {
        message.warning(t("workflow.selfLoopNotAllowed"));
        return;
      }
      // 禁止连接到装饰容器或从装饰容器出发
      const srcNode = nodes.find((n) => n.id === params.source);
      const tgtNode = nodes.find((n) => n.id === params.target);
      const srcCfg = (srcNode?.config ?? {}) as Record<string, unknown>;
      const tgtCfg = (tgtNode?.config ?? {}) as Record<string, unknown>;
      if (srcCfg?.kind === "decorative") {
        message.warning(t("workflow.decorativeContainerNoEdges"));
        return;
      }
      if (tgtCfg?.kind === "decorative") {
        message.warning(t("workflow.decorativeContainerNoEdges"));
        return;
      }
      // 禁止重复边（通过 ref 读取避免 onConnect 依赖 edges 频繁重建）
      const exists = edgesRef.current.some(
        (e) =>
          e.source === params.source
          && e.target === params.target
          && (e.sourceHandle ?? undefined) === (params.sourceHandle ?? undefined),
      );
      if (exists) {
        message.warning(t("workflow.edgeAlreadyExists"));
        return;
      }
      // 环检测：对所有新建边检测是否会产生有向环。
      // - loopBack 边若会形成环则拒绝（会在 rt-workflow 引擎中触发无限循环）
      // - 普通边若会形成环则给出警告（由校验系统标记 cycle_no_exit）
      const sourceHandle = (params.sourceHandle ?? undefined) as
        | string
        | undefined;
      const currentEdges = edgesRef.current.map((e) => ({ source: e.source, target: e.target }));
      currentEdges.push({ source: params.source, target: params.target });
      const pendingEdges = useWorkflowEditorStore.getState().edges
        .filter((e) => !edgesRef.current.some((er) => er.id === e.id))
        .map((e) => ({ source: e.source, target: e.target }));
      const allEdges = [...currentEdges, ...pendingEdges];
      const wouldCycle = wouldCreateCycle(
        allEdges,
        params.source,
        params.target,
      );
      if (wouldCycle) {
        // loopBack 边天然就是环（Loop 节点的回边），不应拒绝。
        // 非 loopBack 的环直接拒绝加边，避免前端状态与后端引擎不一致。
        if (sourceHandle !== "loopBack") {
          message.warning(
            t("workflow.cycleDetectedOnConnect", {
              defaultValue: "This edge creates a cycle without a loopBack marker — the workflow engine may reject it.",
            }),
          );
          return;
        }
      }
      // Determine edge type based on sourceHandle
      let edgeType: WorkflowEdge["edgeType"] = "direct";
      if (sourceHandle === "true") {
        edgeType = "conditionTrue";
      } else if (sourceHandle === "false") {
        edgeType = "conditionFalse";
      } else if (sourceHandle === "loopBack") {
        edgeType = "loopBack";
      } else if (sourceHandle?.startsWith("branch-")) {
        edgeType = "parallelBranch";
      } else if (sourceHandle === "fail") {
        edgeType = "error";
      }

      const newEdge: WorkflowEdge = {
        id: `edge-${crypto.randomUUID()}`,
        source: params.source,
        sourceHandle: sourceHandle ?? undefined,
        target: params.target,
        targetHandle: params.targetHandle ?? undefined,
        edgeType: edgeType,
      };
      storeAddEdge(newEdge);
      // FE-I4 修复：连边后立即同步 edgesRef，避免连续快速连边时
      // 重复边检测基于陈旧引用（该 ref 在 effect 中延迟同步）。
      edgesRef.current = [...edgesRef.current, newEdge];
    },
    [storeAddEdge, t, nodes],
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNode(node.id);
      // 点击单节点时清空多选区，避免与 shift+click 多选冲突
      setSelectedNodeIds(new Set([node.id]));
    },
    [setSelectedNode, setSelectedNodeIds],
  );

  const onEdgeClick = useCallback(
    (_: React.MouseEvent, edge: Edge) => {
      setSelectedEdge(edge.id);
    },
    [setSelectedEdge],
  );

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
  }, [setSelectedNode, setSelectedEdge]);

  const reactFlowInstance = useReactFlow();

  // 折叠/展开容器后，容器 handle 位置变化但 React Flow 缓存了旧的节点
  // internals（含 handle 位置），导致 edge 锚点沿用旧尺寸、连线错位
  // （折叠后下方连线断开）。在 setRNodes 完成后（下一帧）强制重新测量
  // 相关容器节点，确保 handleBounds 与真实 DOM 尺寸同步。
  const collapsedContainersRef = React.useRef<Record<string, boolean>>({});
  const containerIdsNeedingRemeasureRef = React.useRef<string[]>([]);
  useEffect(() => {
    const prev = collapsedContainersRef.current;
    collapsedContainersRef.current = collapsedContainers;
    const changedIds = new Set<string>();
    const allIds = new Set([...Object.keys(prev), ...Object.keys(collapsedContainers)]);
    for (const id of allIds) {
      if (prev[id] !== collapsedContainers[id]) { changedIds.add(id); }
    }
    if (changedIds.size > 0) {
      containerIdsNeedingRemeasureRef.current = [...changedIds];
    }
  }, [collapsedContainers]);

  const onMoveEnd = useCallback(() => {
    setZoom(reactFlowInstance.getZoom());
  }, [reactFlowInstance]);

  const handleFitView = useCallback(() => {
    reactFlowInstance.fitView({ padding: 0.2 });
  }, [reactFlowInstance]);

  const handleResetZoom = useCallback(() => {
    reactFlowInstance.zoomTo(1);
    setZoom(1);
  }, [reactFlowInstance]);

  // Custom DnD: handle mouse-up on the canvas to place a node.
  // We listen on the window so the drop works even if the cursor
  // is slightly outside the ReactFlow pane.
  useEffect(() => {
    const handleGlobalMouseUp = (e: MouseEvent) => {
      const payload = getDragPayload();
      if (!payload) {
        return;
      }

      try {
        const typeInfo = NODE_TYPE_MAP[payload.type] || {
          labelKey: "",
          color: token.colorTextQuaternary,
        };

        // Check if the mouse is within the canvas area
        const canvasEl = document.querySelector(".react-flow");
        if (!canvasEl) {
          return;
        }

        const rect = canvasEl.getBoundingClientRect();
        if (
          e.clientX < rect.left
          || e.clientX > rect.right
          || e.clientY < rect.top
          || e.clientY > rect.bottom
        ) {
          return;
        }

        const position = reactFlowInstance.screenToFlowPosition({
          x: e.clientX,
          y: e.clientY,
        });

        // 容器 hit-test：落点在某个容器节点的 bbox 内时，自动挂入该容器。
        // 嵌套容器场景下选最内层（嵌套深度最大的）；跳过折叠态容器。
        const existingNodes = useWorkflowEditorStore.getState().nodes;
        const existingCollapsed = useWorkflowEditorStore.getState().collapsedContainers;
        const existingParentRefs = useWorkflowEditorStore.getState().parentRefs;
        let hitContainerId: string | null = null;
        let bestHitDepth = -1;
        for (const n of existingNodes) {
          if (!NODE_TYPE_MAP[n.type]?.isContainer) { continue; }
          if (existingCollapsed[n.id]) { continue; } // 折叠态容器不接收拖入
          const rfNode = reactFlowInstance?.getNodes().find((rfn) => rfn.id === n.id);
          const w = rfNode?.measured?.width ?? getNodeSize(n.type).width;
          const h = rfNode?.measured?.height ?? getNodeSize(n.type).height;
          if (
            position.x >= n.position.x
            && position.x <= n.position.x + w
            && position.y >= n.position.y
            && position.y <= n.position.y + h
          ) {
            // 计算嵌套深度：越深优先级越高
            let depth = 0;
            let p = existingParentRefs[n.id];
            while (p) {
              depth++;
              p = existingParentRefs[p];
            }
            if (depth > bestHitDepth) {
              bestHitDepth = depth;
              hitContainerId = n.id;
            }
          }
        }

        const id = `node-${crypto.randomUUID()}`;
        const actualNodeType = NODE_TYPE_MAP[payload.type]
          ? payload.type
          : "base";

        const storePosition = position;

        const rfPosition = hitContainerId
          ? toRelativePosition(
            id,
            position,
            { [id]: hitContainerId },
            useWorkflowEditorStore.getState().nodes as NodePositionLike[],
          )
          : position;

        const newNode: Node = {
          id,
          type: actualNodeType,
          position: rfPosition,
          ...(hitContainerId ? { parentId: hitContainerId, extent: "parent" as const } : {}),
          data: {
            id,
            type: payload.type,
            title: t("workflow.newNode", {
              type: typeInfo.labelKey ? t(typeInfo.labelKey) : payload.type,
            }),
            description: "",
            color: typeInfo.color,
            nodeType: payload.type,
            enabled: true,
            ...getDefaultNodeConfig(payload.type),
          },
        };

        setRNodes((nds) => [...nds, newNode]);

        const workflowNode = createWorkflowNode(
          id,
          payload.type,
          storePosition,
          t("workflow.newNode", {
            type: typeInfo.labelKey ? t(typeInfo.labelKey) : payload.type,
          }),
          hitContainerId ?? undefined,
        );
        useWorkflowEditorStore.getState().addNode(workflowNode);

        if (hitContainerId) {
          useWorkflowEditorStore.getState().setParentRef(id, hitContainerId, true);
        }
      } catch (error) {
        message.error(t("workflow.nodeDropFailed", { error: String(error) }));
      } finally {
        clearDragPayload();
      }
    };

    window.addEventListener("mouseup", handleGlobalMouseUp);
    return () => window.removeEventListener("mouseup", handleGlobalMouseUp);
  }, [reactFlowInstance, setRNodes, t, token]);

  // DnD 拖拽入容器高亮反馈
  useEffect(() => {
    let rafId: number | null = null;
    const handleMouseMove = (e: MouseEvent) => {
      if (rafId != null) { return; }
      rafId = requestAnimationFrame(() => {
        rafId = null;
        const payload = getDragPayload();
        if (!payload) {
          setDndDropTargetId(null);
          return;
        }
        const canvasEl = document.querySelector(".react-flow");
        if (!canvasEl) { return; }
        const rect = canvasEl.getBoundingClientRect();
        if (e.clientX < rect.left || e.clientX > rect.right || e.clientY < rect.top || e.clientY > rect.bottom) {
          setDndDropTargetId(null);
          return;
        }
        const position = reactFlowInstance.screenToFlowPosition({ x: e.clientX, y: e.clientY });
        const existingNodes = useWorkflowEditorStore.getState().nodes;
        const mvCollapsed = useWorkflowEditorStore.getState().collapsedContainers;
        const mvParentRefs = useWorkflowEditorStore.getState().parentRefs;
        let hitId: string | null = null;
        let bestMvDepth = -1;
        for (const n of existingNodes) {
          if (!NODE_TYPE_MAP[n.type]?.isContainer) { continue; }
          if (mvCollapsed[n.id]) { continue; } // 折叠态容器不高亮
          const rfNode = reactFlowInstance?.getNodes().find((rfn) => rfn.id === n.id);
          const w = rfNode?.measured?.width ?? getNodeSize(n.type).width;
          const h = rfNode?.measured?.height ?? getNodeSize(n.type).height;
          if (
            position.x >= n.position.x && position.x <= n.position.x + w
            && position.y >= n.position.y && position.y <= n.position.y + h
          ) {
            let depth = 0;
            let p = mvParentRefs[n.id];
            while (p) {
              depth++;
              p = mvParentRefs[p];
            }
            if (depth > bestMvDepth) {
              bestMvDepth = depth;
              hitId = n.id;
            }
          }
        }
        setDndDropTargetId(hitId);
      });
    };
    window.addEventListener("mousemove", handleMouseMove, { passive: true });
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      if (rafId != null) { cancelAnimationFrame(rafId); }
    };
  }, [reactFlowInstance]);

  // DnD 拖拽入容器高亮 — 直接 DOM 操作避免全量重建
  useEffect(() => {
    if (!dndDropTargetId) { return; }
    const el = document.querySelector(`.react-flow__node[data-id="${dndDropTargetId}"]`);
    if (!el) { return; }
    el.classList.add("workflow-dnd-drop-target");
    return () => {
      el.classList.remove("workflow-dnd-drop-target");
    };
  }, [dndDropTargetId]);

  const handleSave = useCallback(async () => {
    if (!currentTemplate || isSaving) {
      return;
    }

    if (autoSaveTimerRef.current) {
      clearTimeout(autoSaveTimerRef.current);
      autoSaveTimerRef.current = null;
    }

    if (isDecompositionTemplate) {
      try {
        await saveSkillWorkflowFromLlm(
          currentTemplate.name,
          currentTemplate.description,
        );
        message.success(t("workflow.decompositionSaved"));
        onClose?.();
      } catch (e) {
        showBackendError(message, e);
      }
      return;
    }

    // 自动清理引用不存在节点的无效边
    const nodeIdSet = new Set(nodes.map((n) => n.id));
    const invalidEdges = edges.filter(
      (e) => !nodeIdSet.has(e.source) || !nodeIdSet.has(e.target),
    );
    const cleanedEdges = getCleanedEdges(nodes, edges);
    if (invalidEdges.length > 0) {
      const { setEdges: storeSetEdges } = useWorkflowEditorStore.getState();
      storeSetEdges(cleanedEdges);
      message.warning(
        t("workflow.invalidEdgesCleaned", {
          count: invalidEdges.length,
          details: invalidEdges
            .map(
              (e) =>
                t("workflow.invalidEdgeDetail", {
                  edgeId: e.id,
                  missingNodeId: !nodeIdSet.has(e.source) ? e.source : e.target,
                }),
            )
            .join("\n"),
        }),
        6,
      );
    }

    // 前端结构校验：error 级别阻塞保存，warning 级别仅提示
    const frontendIssues = validateWorkflow(nodes, cleanedEdges, t);
    const frontendErrors = frontendIssues.issues.filter((i) => i.severity === "error");
    const frontendWarnings = frontendIssues.issues.filter((i) => i.severity === "warning");
    if (frontendErrors.length > 0) {
      message.error(
        t("workflow.validationFailed", { count: frontendErrors.length })
          + "\n" + frontendErrors.map((i) => i.message).join("\n"),
      );
      return;
    }
    if (frontendWarnings.length > 0) {
      message.warning(
        t("workflow.validationWarnings", {
          count: frontendWarnings.length,
          details: frontendWarnings.map((i) => i.message).join("\n"),
        }),
      );
      // warning 不阻塞保存
    }

    const validation = await validateTemplate();
    if (validation && !validation.isValid) {
      const errorDetails = validation.errors
        .map((e) => {
          let detail = e.message;
          if (e.nodeId) {
            detail += t("workflow.validationNodeInfo", { nodeId: e.nodeId });
          }
          if (e.suggestion) {
            detail += t("workflow.validationSuggestion", { suggestion: e.suggestion });
          }
          return detail;
        })
        .join("\n\n");
      message.error(
        t("workflow.validationFailed", { count: validation.errors.length })
          + "\n\n"
          + errorDetails,
        8,
      );
      return;
    }

    // 注入 parentRefs 到节点，与 auto-save 逻辑一致，确保容器父子关系持久化
    // Store 始终存绝对坐标，保存时也保持绝对坐标。
    // 加载时 rebuildParentRefsFromNodes 恢复 parentRefs，useEffect 再将绝对坐标转为相对坐标给 ReactFlow。
    const nodesWithParent = buildNodesWithParent(nodes, parentRefs);

    const input = {
      name: currentTemplate.name,
      description: currentTemplate.description,
      icon: currentTemplate.icon,
      tags: currentTemplate.tags,
      triggerConfig: currentTemplate.triggerConfig,
      nodes: nodesWithParent,
      edges: cleanedEdges,
      inputSchema: currentTemplate.inputSchema,
      outputSchema: currentTemplate.outputSchema,
      variables: currentTemplate.variables,
      errorConfig: currentTemplate.errorConfig,
    };

    if (currentTemplate.id) {
      try {
        const ok = await updateTemplate(currentTemplate.id, input);
        if (ok) {
          message.success(t("workflow.saved"));
        } else {
          message.error(t("workflow.saveFailed"));
        }
      } catch (e) {
        showBackendError(message, e);
      }
    } else {
      try {
        const newId = await createTemplate(input);
        if (newId) {
          await loadTemplate(newId);
          message.success(t("workflow.saved"));
        } else {
          message.error(t("workflow.saveFailed"));
        }
      } catch (e) {
        showBackendError(message, e);
      }
    }
  }, [
    currentTemplate,
    nodes,
    edges,
    parentRefs,
    createTemplate,
    updateTemplate,
    validateTemplate,
    t,
    onClose,
    isDecompositionTemplate,
    saveSkillWorkflowFromLlm,
    loadTemplate,
    isSaving,
  ]);
  useEffect(() => {
    handleSaveRef.current = handleSave;
  }, [handleSave]);

  const handleSaveAsImage = useCallback(async () => {
    if (!reactFlowInstance) { return; }

    let container: HTMLDivElement | null = null;

    try {
      // 1. 注入隐藏 UI 元素的 CSS（仅注入一次）
      const STYLE_ID = "workflow-export-hide-styles";
      if (!document.getElementById(STYLE_ID)) {
        const style = document.createElement("style");
        style.id = STYLE_ID;
        style.textContent = `
          .workflow-exporting .react-flow__controls,
          .workflow-exporting .react-flow__minimap,
          .workflow-exporting .react-flow__panel,
          .workflow-exporting .react-flow__background {
            display: none !important;
          }
        `;
        document.head.appendChild(style);
      }

      // 3. 手动计算所有节点的包围盒（容器节点按 NODE_TYPE_MAP 真实尺寸计算）
      const nodes = reactFlowInstance.getNodes();
      if (nodes.length === 0) {
        message.info(t("workflow.exportEmpty"));
        return;
      }
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      const nodeMap = new Map<string, Node>();
      nodes.forEach((n) => nodeMap.set(n.id, n));
      const getAbsolutePosition = (node: Node): { x: number; y: number } => {
        if (!node.parentId) { return node.position; }
        const parent = nodeMap.get(node.parentId);
        if (!parent) { return node.position; }
        const parentAbs = getAbsolutePosition(parent);
        return { x: node.position.x + parentAbs.x, y: node.position.y + parentAbs.y };
      };
      nodes.forEach((node) => {
        const nodeType = (node.data?.type as string) || node.type || "";
        const fallback = NODE_TYPE_MAP[nodeType]?.isContainer
          ? getNodeSize(nodeType)
          : null;
        const w = node.measured?.width ?? fallback?.width ?? 200;
        const h = node.measured?.height ?? fallback?.height ?? 100;
        const absPos = getAbsolutePosition(node);
        minX = Math.min(minX, absPos.x);
        minY = Math.min(minY, absPos.y);
        maxX = Math.max(maxX, absPos.x + w);
        maxY = Math.max(maxY, absPos.y + h);
      });
      const padding = 80;

      // 4. 创建离屏容器
      container = document.createElement("div");
      container.style.position = "fixed";
      container.style.left = "-99999px";
      container.style.top = "0";
      container.style.background = "#1a1a2e";
      container.style.overflow = "visible";

      const totalW = Math.max(320, Math.ceil(maxX - minX) + padding * 2);
      const totalH = Math.max(240, Math.ceil(maxY - minY) + padding * 2);
      container.style.width = totalW + "px";
      container.style.height = totalH + "px";

      // 5. 克隆 .react-flow 到离屏容器
      const element = canvasContainerRef.current;
      if (!element) {
        message.error(t("workflow.exportNotFoundOrFailed"));
        return;
      }

      const flowEl = element.querySelector(".react-flow") as HTMLElement | null;
      if (!flowEl) { throw new Error("React Flow element not found"); }

      const flowClone = flowEl.cloneNode(true) as HTMLElement;
      flowClone.classList.add("workflow-exporting");
      flowClone.style.position = "relative";
      flowClone.style.transform = "none";
      flowClone.style.overflow = "visible";
      flowClone.style.width = totalW + "px";
      flowClone.style.height = totalH + "px";

      // 6. 重置克隆体中的 viewport transform，以 zoom=1 显示全部节点
      const viewportClone = flowClone.querySelector(".react-flow__viewport") as HTMLElement | null;
      if (viewportClone) {
        viewportClone.style.transform = `translate(${padding - minX}px, ${padding - minY}px) scale(1)`;
        viewportClone.style.transformOrigin = "0 0";
      }

      // 7. 保险：把克隆体内所有 SVG edge 的描边转成具体颜色
      try {
        const edgePaths = flowClone.querySelectorAll<SVGPathElement>(".react-flow__edge-path");
        edgePaths.forEach((path) => {
          const computed = window.getComputedStyle(path).stroke;
          if (computed && computed !== "none" && !computed.startsWith("var(")) {
            path.style.stroke = computed;
          } else {
            const edgeEl = path.closest(".react-flow__edge");
            const isSelected = edgeEl?.classList.contains("selected");
            path.style.stroke = isSelected ? "#888" : "#b1b1b7";
            path.style.strokeWidth = isSelected ? "2" : "1";
          }
        });
        const allElements = flowClone.querySelectorAll<HTMLElement>("*");
        allElements.forEach((el) => {
          const style = el.style;
          for (let i = 0; i < style.length; i++) {
            const prop = style[i];
            const val = style.getPropertyValue(prop);
            if (val && val.startsWith("var(")) {
              const computed = window.getComputedStyle(el).getPropertyValue(prop);
              if (computed && !computed.startsWith("var(")) {
                style.setProperty(prop, computed);
              }
            }
          }
        });
      } catch {
        // ignore
      }

      container.appendChild(flowClone);
      document.body.appendChild(container);

      // 8. 等一帧确保 DOM 渲染完成
      await new Promise<void>((resolve) => {
        requestAnimationFrame(() => {
          resolve();
        });
      });

      // 9. 导出：scale=2 超采样保证高清
      const defaultName = `${currentTemplate?.name || "workflow"}.png`;

      if (isTauri()) {
        const canvas = await html2canvas(container, {
          backgroundColor: "#1a1a2e",
          scale: 2,
        });
        const blob = await new Promise<Blob>((resolve) => {
          canvas.toBlob((b) => resolve(b!), "image/png");
        });
        if (!blob) {
          message.error(t("workflow.exportFailed"));
          return;
        }
        const { save } = await import("@tauri-apps/plugin-dialog");
        const { writeFile } = await import("@tauri-apps/plugin-fs");
        const filePath = await save({
          defaultPath: defaultName,
          filters: [{ name: "PNG Image", extensions: ["png"] }],
        });
        if (!filePath) { return; }
        await writeFile(filePath, new Uint8Array(await blob.arrayBuffer()));
      } else {
        const canvas = await html2canvas(container, {
          backgroundColor: "#1a1a2e",
          scale: 2,
        });
        const dataUrl = canvas.toDataURL("image/png");
        const link = document.createElement("a");
        link.download = defaultName;
        link.href = dataUrl;
        link.click();
      }

      message.success(t("workflow.exportSuccess"));
    } catch (error) {
      console.error("[saveAsImage]", error);
      message.error(`${t("workflow.exportFailed")}: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      // 10. 清理离屏容器
      if (container && container.parentNode) {
        container.parentNode.removeChild(container);
      }
      // FE-S2 修复：移除导出期间注入的全局 style，避免常驻 DOM。
      const exportedStyle = document.getElementById("workflow-export-hide-styles");
      if (exportedStyle && exportedStyle.parentNode) {
        exportedStyle.parentNode.removeChild(exportedStyle);
      }
    }
  }, [reactFlowInstance, currentTemplate, t]);

  const handleExportYaml = useCallback(async (): Promise<string | null> => {
    try {
      const state = useWorkflowEditorStore.getState();
      const { nodes, edges, parentRefs, currentTemplate: tmpl } = state;
      const nodesWithParent = buildNodesWithParent(nodes, parentRefs);
      const workflowInput = {
        name: tmpl?.name || "Unnamed Workflow",
        description: tmpl?.description,
        icon: tmpl?.icon || "Bot",
        tags: tmpl?.tags || [],
        triggerConfig: tmpl?.triggerConfig,
        nodes: nodesWithParent,
        edges,
        inputSchema: tmpl?.inputSchema,
        outputSchema: tmpl?.outputSchema,
        variables: tmpl?.variables || [],
        errorConfig: tmpl?.errorConfig,
      };
      const yaml = await invoke<string>("export_workflow_yaml", { workflowJson: JSON.stringify(workflowInput) });
      return yaml || null;
    } catch (e) {
      console.error("[exportYaml]", e);
      message.error(t("workflow.importExport.yamlExportFailed"));
      return null;
    }
  }, [t]);

  const handleImportYaml = useCallback(async (yaml: string): Promise<boolean> => {
    try {
      // Tauri auto-maps snake_case → camelCase: the Rust parameter yaml_str becomes yamlStr
      const resultStr = await invoke<string>("import_workflow_yaml", { yamlStr: yaml });
      const result = JSON.parse(resultStr) as { workflow: { id: string }; metadata: Record<string, unknown> };
      if (result?.workflow?.id) {
        await loadTemplate(result.workflow.id);
        return true;
      }
      message.error(t("workflow.importExport.yamlImportFailed"));
      return false;
    } catch (e) {
      console.error("[importYaml]", e);
      message.error(t("workflow.importExport.yamlImportFailed"));
      return false;
    }
  }, [t, loadTemplate]);

  const handleNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: Node) => {
      event.preventDefault();
      setContextMenu({ x: event.clientX, y: event.clientY, nodeId: node.id });
    },
    [],
  );

  // 关闭右键菜单
  useEffect(() => {
    if (!contextMenu) { return; }
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  // 节点搜索结果
  const searchResults = useMemo(() => {
    if (!searchQuery) { return []; }
    const q = searchQuery.toLowerCase();
    return nodes.filter((n) =>
      n.title.toLowerCase().includes(q) || n.type.toLowerCase().includes(q) || n.id.toLowerCase().includes(q)
    );
  }, [searchQuery, nodes]);

  const navigateSearch = useCallback((dir: 1 | -1) => {
    if (searchResults.length === 0) { return; }
    const nextIdx = (searchIdx + dir + searchResults.length) % searchResults.length;
    setSearchIdx(nextIdx);
    const target = searchResults[nextIdx];
    setSelectedNode(target.id);
    const absPos = toAbsolutePosition(
      target.id,
      target.position,
      useWorkflowEditorStore.getState().parentRefs,
      useWorkflowEditorStore.getState().nodes as NodePositionLike[],
    );
    reactFlowInstance?.setCenter(absPos.x + 100, absPos.y + 50, { zoom: 1.5, duration: 300 });
  }, [searchResults, searchIdx, reactFlowInstance, setSelectedNode]);

  // 卸载时清理所有 timer 和 RAF，防止内存泄漏与卸载后回调
  useEffect(() => () => {
    if (autoSaveTimerRef.current) { clearTimeout(autoSaveTimerRef.current); }
    if (autoLayoutTimerRef.current) { clearTimeout(autoLayoutTimerRef.current); }
    dragCtrl.clearPending();
    // 确保 autoSaveCtrl 的 timer 也被清理
    autoSaveCtrl.resetRetryCount();
  }, []);

  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const clonedChanges = changes.map((c) => {
        const result: NodeChange = c;
        if (c.type === "position" && c.position) {
          return { ...result, position: { ...c.position } };
        }
        if (c.type === "dimensions" && c.dimensions) {
          return { ...result, dimensions: { ...c.dimensions } };
        }
        if (c.type === "add" && c.item?.position) {
          return {
            ...result,
            item: { ...c.item, position: { ...c.item.position } },
          };
        }
        return result;
      });
      onNodesChange(clonedChanges);

      const hasSelectionChange = changes.some((c) => c.type === "select");
      if (hasSelectionChange) {
        const flowInstance = reactFlowInstance;
        if (flowInstance) {
          const selected = flowInstance.getNodes().filter((n) => n.selected);
          setSelectedNodeIds(new Set(selected.map((n) => n.id)));
        }
      }

      changes.forEach((change) => {
        if (
          change.type === "position" && change.position && currentTemplate && !isDraggingRef.current
          && !skipPositionWriteRef.current
        ) {
          // FE-I1 修复：统一走 dragCtrl.queuePositionUpdate() 的 RAF 批处理，
          // 删除本地 pendingPositionsRef/posRafRef 双机制（本地机制无卸载清理，
          // 卸载后仍会对已卸载 store 写入）。hook 内部 clearPending 负责 cancel。
          dragCtrl.queuePositionUpdate(change.id, change.position);
        }
        if (change.type === "remove" && change.id) {
          // Collect remove IDs first, then delete only non-cascaded nodes
          // to avoid double-pushing undo history when deleteNode cascades
          removeIdsRef.current.add(change.id);
        }
      });
      // Batch delete: collect all remove IDs, then delete only non-cascaded nodes
      if (removeIdsRef.current.size > 0) {
        const idsToDelete = [...removeIdsRef.current];
        removeIdsRef.current.clear();
        // Find which IDs would be cascade-deleted (children of deleted containers)
        const cascadeIds = new Set<string>();
        for (const id of idsToDelete) {
          const nodeType = useWorkflowEditorStore.getState().nodes.find((n) => n.id === id)?.type;
          if (nodeType && NODE_TYPE_MAP[nodeType]?.isContainer) {
            for (const [cid, pid] of Object.entries(useWorkflowEditorStore.getState().parentRefs)) {
              if (pid === id) { cascadeIds.add(cid); }
            }
          }
        }
        // Only delete nodes that aren't cascade children (they'll be deleted by the parent's deleteNode)
        for (const id of idsToDelete) {
          if (!cascadeIds.has(id)) {
            deleteNode(id);
          }
        }
      }
    },
    //
    [onNodesChange, currentTemplate, updateNode, deleteNode, parentRefs, reactFlowInstance],
  );

  const handleNodeDragStart = useCallback((_event: unknown, node: Node) => {
    isDraggingRef.current = true;

    // 关键修复：拖拽开始时移除非容器子节点的 extent 限制
    // 原因：extent: "parent" 会把子节点限制在父容器内，但容器尺寸
    // 在拖拽过程中不会实时更新，导致子节点被锁死在初始边界内。
    // 解决方案：拖拽时允许子节点自由移动，拖拽结束时通过 hit-test
    // 重新计算其 parent 和 extent。
    const latestParentRefs = useWorkflowEditorStore.getState().parentRefs;
    const draggedNodeParentId = latestParentRefs[node.id];
    const nodeType = (node.data?.type as string) || node.type || "";
    const isContainerNode = NODE_TYPE_MAP[nodeType]?.isContainer === true;

    if (draggedNodeParentId && !isContainerNode) {
      // 子节点被拖拽：临时移除 extent 限制
      reactFlowInstance?.setNodes((nds) =>
        nds.map((n) =>
          n.id === node.id
            ? { ...n, extent: undefined as unknown as "parent" }
            : n
        )
      );
    }
  }, [reactFlowInstance]);

  /** 拖拽过程中实时吸附到 grid（ReactFlow 内置 snapToGrid 已处理视觉吸附） */
  const handleNodeDrag = useCallback(
    (_event: unknown, _node: Node) => {
      // ReactFlow 的 snapToGrid 已在渲染层面完成网格吸附；
      // onNodeDrag 在此预留，可用于未来添加 ghost position overlay
    },
    [],
  );

  const handleNodeDragStop = useCallback(
    (_event: unknown, node: Node) => {
      isDraggingRef.current = false;
      suppressRebuildRef.current = true;

      if (!node?.position) { return; }

      useWorkflowEditorStore.getState().recordUndoSnapshot();
      const rfNodes = reactFlowInstance?.getNodes() || [];
      const latestNodes = useWorkflowEditorStore.getState().nodes;
      const latestParentRefs = useWorkflowEditorStore.getState().parentRefs;
      const latestCollapsed = useWorkflowEditorStore.getState().collapsedContainers;

      const draggedNodeParentId = latestParentRefs[node.id];
      const nodeType = (node.data?.type as string) || node.type || "";
      const isContainerNode = NODE_TYPE_MAP[nodeType]?.isContainer === true;

      // ── 计算绝对落点坐标 ──────────────────────────────
      // 关键修复：由于 handleNodeDragStart 中已移除子节点的 extent 限制，
      // node.position 现在是绝对坐标（相对于画布原点）。
      // 但对于容器节点（仍有 dragHandle 限制），需要考虑其自身的 parent 关系。
      const absDropPos = { x: node.position.x, y: node.position.y };

      // ── 容器 hit-test：检测拖拽落点是否在某个容器内 ──
      // 排除：自身、自身后代（避免环）、折叠态容器
      const descendantsOfDragged = new Set<string>();
      if (isContainerNode) {
        const collectDescendants = (parentId: string) => {
          for (const [cid, pid] of Object.entries(latestParentRefs)) {
            if (pid === parentId && !descendantsOfDragged.has(cid)) {
              descendantsOfDragged.add(cid);
              collectDescendants(cid);
            }
          }
        };
        collectDescendants(node.id);
      }

      let hitContainerId: string | null = null;
      let bestDepth = -1;
      for (const n of latestNodes) {
        if (!NODE_TYPE_MAP[n.type]?.isContainer) { continue; }
        if (n.id === node.id) { continue; }
        if (descendantsOfDragged.has(n.id)) { continue; }
        if (latestCollapsed[n.id]) { continue; }
        // 关键修复：使用 useFlowNodes 计算的容器样式尺寸，而不是默认尺寸
        // 从 rfNodes 中获取实际渲染的容器样式
        const rfNode = rfNodes.find((rfn) => rfn.id === n.id);
        let w = rfNode?.style?.width as number | undefined;
        let h = rfNode?.style?.height as number | undefined;
        if (w === undefined || h === undefined) {
          // fallback：使用 measured 或默认尺寸
          w = rfNode?.measured?.width ?? getNodeSize(n.type).width;
          h = rfNode?.measured?.height ?? getNodeSize(n.type).height;
        }
        if (
          absDropPos.x >= n.position.x && absDropPos.x <= n.position.x + w
          && absDropPos.y >= n.position.y && absDropPos.y <= n.position.y + h
        ) {
          let depth = 0;
          let p = latestParentRefs[n.id];
          while (p) {
            depth++;
            p = latestParentRefs[p];
          }
          if (depth > bestDepth) {
            bestDepth = depth;
            hitContainerId = n.id;
          }
        }
      }

      // ── 判断 parent 是否需要切换 ──────────────────────
      const parentChanged = (hitContainerId !== null && hitContainerId !== draggedNodeParentId)
        || (hitContainerId === null && draggedNodeParentId != null);

      let newParentId: string | undefined = hitContainerId ?? undefined;
      let storePos: { x: number; y: number };
      let rfPos: { x: number; y: number };

      if (parentChanged) {
        // parent 切换：更新 parentRefs 并计算坐标
        if (newParentId) {
          // 移入新容器：store 存绝对坐标，ReactFlow 用相对坐标
          storePos = absDropPos;
          rfPos = {
            x: absDropPos.x - latestNodes.find((n) => n.id === newParentId)!.position.x,
            y: absDropPos.y - latestNodes.find((n) => n.id === newParentId)!.position.y,
          };
          useWorkflowEditorStore.getState().setParentRef(node.id, newParentId, true);
          // 多选场景：其他选中的节点也移入同一容器，计算其绝对坐标
          const selectedNodes = rfNodes.filter((rfn) => rfn.selected && rfn.id !== node.id);
          const deltaX = absDropPos.x - node.position.x;
          const deltaY = absDropPos.y - node.position.y;
          for (const selNode of selectedNodes) {
            const newAbsX = selNode.position.x + deltaX;
            const newAbsY = selNode.position.y + deltaY;
            useWorkflowEditorStore.getState().setParentRef(selNode.id, newParentId, true);
            updateNode(selNode.id, { position: { x: newAbsX, y: newAbsY } } as Partial<WorkflowNode>);
          }
        } else {
          // 移出容器：坐标保持绝对
          storePos = absDropPos;
          rfPos = absDropPos;
          useWorkflowEditorStore.getState().setParentRef(node.id, null, true);
          // 多选场景：其他选中的节点也移出容器，计算其绝对坐标
          const selectedNodes = rfNodes.filter((rfn) => rfn.selected && rfn.id !== node.id);
          const deltaX = absDropPos.x - node.position.x;
          const deltaY = absDropPos.y - node.position.y;
          for (const selNode of selectedNodes) {
            const newAbsX = selNode.position.x + deltaX;
            const newAbsY = selNode.position.y + deltaY;
            useWorkflowEditorStore.getState().setParentRef(selNode.id, null, true);
            updateNode(selNode.id, { position: { x: newAbsX, y: newAbsY } } as Partial<WorkflowNode>);
          }
        }
      } else if (draggedNodeParentId) {
        // parent 未变，仍在原容器内
        // 关键修复：由于 handleNodeDragStart 中已移除 extent，
        // node.position 现在是绝对坐标，所以直接使用 absDropPos
        storePos = absDropPos;
        // 计算相对原容器的坐标
        const originalContainer = latestNodes.find((n) => n.id === draggedNodeParentId);
        if (originalContainer) {
          rfPos = {
            x: absDropPos.x - originalContainer.position.x,
            y: absDropPos.y - originalContainer.position.y,
          };
        } else {
          rfPos = absDropPos;
        }
        newParentId = draggedNodeParentId;
      } else {
        // 顶层节点：碰撞避免（仅与同层级顶层节点比较）
        const selectedIds = new Set(rfNodes.filter((n) => n.selected).map((n) => n.id));
        const siblings = rfNodes
          .filter((n) => n.id !== node.id && !selectedIds.has(n.id) && !n.parentId)
          .map((n) => ({
            id: n.id,
            x: n.position.x,
            y: n.position.y,
            type: (n.data?.type as string) || n.type || "",
          }));
        const safePos = findSafePosition({ x: node.position.x, y: node.position.y }, nodeType, siblings);
        storePos = safePos;
        rfPos = safePos;
        newParentId = undefined;
      }

      const oldNode = latestNodes.find((n) => n.id === node.id);
      const dx = oldNode ? storePos.x - oldNode.position.x : 0;
      const dy = oldNode ? storePos.y - oldNode.position.y : 0;
      updateNode(node.id, { position: storePos } as Partial<WorkflowNode>);

      // 被拖的是容器 → 子节点在 store 中存绝对坐标，需同步偏移量
      const isContainer = oldNode ? NODE_TYPE_MAP[oldNode.type]?.isContainer === true : false;
      if (isContainer && (dx !== 0 || dy !== 0)) {
        for (const [childId, pid] of Object.entries(latestParentRefs)) {
          if (pid === node.id) {
            const childNode = latestNodes.find((n2) => n2.id === childId);
            if (childNode) {
              updateNode(childId, {
                position: { x: childNode.position.x + dx, y: childNode.position.y + dy },
              } as Partial<WorkflowNode>);
            }
          }
        }
      }

      // ── 统一更新 ReactFlow 节点（消除竞态：单次 setNodes 包含所有更新）──
      // 关键修复：容器子节点的相对坐标需要基于更新后的绝对坐标重新计算
      const updatedNodes = rfNodes.map((n) => {
        if (n.id === node.id) {
          return {
            ...n,
            position: rfPos,
            ...(newParentId
              ? { parentId: newParentId, extent: "parent" as const }
              : { parentId: undefined, extent: undefined }),
          };
        }
        // 容器子节点同步：如果被拖的是容器，需要更新其子节点的相对坐标
        // 关键修复：使用 updateNode 更新后的绝对坐标来计算相对坐标
        if (isContainer && latestParentRefs[n.id] === node.id) {
          // 从最新的 store 状态获取子节点的绝对坐标
          const latestState = useWorkflowEditorStore.getState();
          const childNode = latestState.nodes.find((sn) => sn.id === n.id);
          const containerNode = latestState.nodes.find((cn) => cn.id === node.id);
          if (childNode && containerNode) {
            return {
              ...n,
              position: {
                x: childNode.position.x - containerNode.position.x,
                y: childNode.position.y - containerNode.position.y,
              },
            };
          }
        }
        // 选中的其他节点：需要同步更新其 parentId 和 extent
        if (n.selected && n.id !== node.id && n.position) {
          // 从最新的 store 状态获取节点的绝对坐标
          const latestState = useWorkflowEditorStore.getState();
          const selStoreNode = latestState.nodes.find((sn) => sn.id === n.id);
          if (selStoreNode) {
            // 如果拖拽改变了 parent，需要更新选中节点的 parentId 和 extent
            if (newParentId) {
              // 移入新容器：重新计算相对坐标
              const newContainer = latestState.nodes.find((cn) => cn.id === newParentId);
              if (newContainer) {
                return {
                  ...n,
                  position: {
                    x: selStoreNode.position.x - newContainer.position.x,
                    y: selStoreNode.position.y - newContainer.position.y,
                  },
                  parentId: newParentId,
                  extent: "parent" as const,
                };
              }
            } else if (!draggedNodeParentId) {
              // 移出容器或顶层节点：移除 parentId 和 extent
              return {
                ...n,
                position: { ...selStoreNode.position },
                parentId: undefined,
                extent: undefined,
              };
            }
            // 仍在原容器或无 parent：返回原节点（不再调用 updateNode）
          }
          return n;
        }
        return n;
      });
      reactFlowInstance?.setNodes(updatedNodes);

      // 强制触发容器尺寸重算 + 统一清理时序
      // 先 RAF 确保容器重算，再解除 suppressRebuildRef 避免 useEffect 覆盖
      const triggerParent = newParentId || (isContainer ? node.id : undefined);
      if (triggerParent) {
        requestAnimationFrame(() => {
          setDragStopVersion((v) => v + 1);
          // 下一帧再开放 useEffect 重建，确保重算已生效
          requestAnimationFrame(() => {
            suppressRebuildRef.current = false;
          });
        });
      } else {
        // 无容器操作，直接开放
        suppressRebuildRef.current = false;
      }
    },
    [updateNode, reactFlowInstance, setDragStopVersion],
  );

  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      onEdgesChange(changes);

      changes.forEach((change) => {
        if (change.type === "remove" && change.id) {
          deleteEdge(change.id);
        }
      });
    },
    [onEdgesChange, deleteEdge],
  );

  const handleNameChange = useCallback(
    (name: string) => {
      updateTemplateMetadata({ name });
    },
    [updateTemplateMetadata],
  );

  const handleImportedTemplate = useCallback(
    (id: string) => {
      loadTemplate(id);
    },
    [loadTemplate],
  );

  const handleAutoLayout = useCallback(async () => {
    // 过滤分组边：不参与自动布局
    const layoutEdges = reactFlowEdges.filter(
      (e) => (e.data as { edgeType?: string } | undefined)?.edgeType !== "grouping",
    );
    // 使用新的 autoLayout（按 type 分层 + Barycenter 启发式）
    const layoutedNodes = autoLayout(
      reactFlowNodes as unknown as AutoNode[], // SAFE: ReactFlow Node to AutoNode for layout engine — both carry position/id/type
      layoutEdges,
      parentRefs,
    );
    // autoLayout 返回值：所有节点 position = 绝对坐标
    // ReactFlow setRNodes 需要子节点为相对坐标，但 autoLayout 返回绝对坐标
    // 所以需要将子节点转为相对坐标给 ReactFlow，同时存绝对坐标到 store
    skipPositionWriteRef.current = true;
    const rfNodes = layoutedNodes.map((n) => {
      const pid = parentRefs[n.id];
      if (pid) {
        const relPos = toRelativePosition(
          n.id,
          n.position,
          parentRefs,
          layoutedNodes.map((ln) => ({ id: ln.id, position: ln.position })) as NodePositionLike[],
        );
        return { ...n, position: relPos };
      }
      return n;
    });
    setRNodes(rfNodes);
    requestAnimationFrame(() => {
      skipPositionWriteRef.current = false;
    });

    // store 存绝对坐标
    for (const ln of layoutedNodes) {
      updateNode(ln.id, { position: ln.position } as Partial<WorkflowNode>);
    }

    // 递归处理子工作流内部节点
    const subWorkflowNodes = reactFlowNodes.filter(
      (n) => (n.data?.type || n.type) === "subWorkflow",
    );
    if (subWorkflowNodes.length > 0) {
      const { invoke } = await import("@/lib/invoke");
      let subCount = 0;
      for (const subNode of subWorkflowNodes) {
        const subId = subNode.data?.subWorkflowId;
        if (!subId) { continue; }
        try {
          const tmpl: Record<string, unknown> = await invoke("get_workflow_template", { id: subId });
          const rawNodes = tmpl.nodes as Array<Record<string, unknown>> | undefined;
          if (!rawNodes) { continue; }
          const subNodes: Array<Record<string, unknown>> = rawNodes;
          const subEdges = (tmpl.edges || []) as Array<Record<string, unknown>>;
          const rfSubNodes: AutoNode[] = subNodes.map((n) => {
            const base = n.base as Record<string, unknown> | undefined;
            const nType = (n.type || base?.type || "agent") as string;
            return {
              id: (n.id || base?.id || "") as string,
              type: nType,
              position: (n.position || base?.position || { x: 0, y: 0 }) as { x: number; y: number },
              data: { ...n, type: nType },
            };
          });
          const rfSubEdges = subEdges.map((e, i) => ({
            id: (e.id || `sub_e_${i}`) as string,
            source: e.source as string,
            target: e.target as string,
            sourceHandle: e.sourceHandle as string | undefined,
            targetHandle: e.targetHandle as string | undefined,
          }));
          const subLayouted = autoLayout(rfSubNodes, rfSubEdges);
          const updatedSubNodes: Array<Record<string, unknown>> = subNodes.map((n) => {
            const nodeId = (n.id || (n.base as Record<string, unknown> | undefined)?.id || "") as string;
            const laid = subLayouted.find((ln) => ln.id === nodeId);
            if (!laid) { return n; }
            if (n.base) {
              return { ...n, base: { ...(n.base as Record<string, unknown>), position: laid.position } };
            }
            return { ...n, position: laid.position };
          });
          const input = {
            name: tmpl.name || "",
            icon: tmpl.icon || "",
            tags: tmpl.tags || [],
            nodes: updatedSubNodes,
            edges: subEdges,
            variables: tmpl.variables || [],
            inputSchema: tmpl.inputSchema || undefined,
            outputSchema: tmpl.outputSchema || undefined,
            errorConfig: tmpl.errorConfig || undefined,
            triggerConfig: tmpl.triggerConfig || undefined,
            description: tmpl.description || undefined,
          };
          await invoke("update_workflow_template", { id: subId, input });
          subCount++;
        } catch {
          // 子工作流加载/保存失败，跳过继续
        }
      }
      if (subCount > 0) {
        message.success(
          t("workflow.autoLayoutWithSubs", { count: subCount }),
        );
        return;
      }
    }

    // 修复9：自动布局完成后调用 fitView，把整个图适配到视口
    requestAnimationFrame(() => {
      reactFlowInstance.fitView({ padding: 0.15, duration: 400, maxZoom: 1.2 });
    });

    message.success(t("workflow.autoLayout"));
  }, [reactFlowNodes, reactFlowEdges, parentRefs, setRNodes, updateNode, t, reactFlowInstance]);

  const handleClose = useCallback(() => {
    if (isDirty) {
      Modal.confirm({
        title: t("workflow.unsavedTitle"),
        content: t("workflow.unsavedContent"),
        okText: t("workflow.discard"),
        cancelText: t("workflow.keepEditing"),
        onOk: () => {
          onClose?.();
        },
      });
    } else {
      onClose?.();
    }
  }, [isDirty, t, onClose]);

  const selectedEdge = useMemo(() => {
    if (!selectedEdgeId) {
      return null;
    }
    return edges.find((e) => e.id === selectedEdgeId) || null;
  }, [selectedEdgeId, edges]);

  if (isLoading) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
        }}
      >
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        minHeight: 0,
        background: token.colorBgContainer,
      }}
    >
      <EditorHeader
        templateName={currentTemplate?.name || t("workflow.newWorkflow")}
        isDirty={isDirty}
        isSaving={isSaving}
        onSave={handleSave}
        onNameChange={handleNameChange}
        onClose={handleClose}
        onToggleAIPanel={() => setAiPanelVisible(!aiPanelVisible)}
        onToggleDebugPanel={() => setDebugPanelVisible(!debugPanelVisible)}
        onRunDiagnostic={async () => {
          try {
            await runWorkflowDiagnose();
            setDiagnoseDrawerVisible(true);
          } catch {
            message.error(t("workflow.diagnostic.error"));
          }
        }}
        diagnosticLoading={diagnoseLoading}
        onToggleLeftPanel={() => setLeftPanelCollapsed(!leftPanelCollapsed)}
        onToggleRightPanel={() => setRightPanelCollapsed(!rightPanelCollapsed)}
        leftPanelCollapsed={leftPanelCollapsed}
        rightPanelCollapsed={rightPanelCollapsed}
        onOpenImportExport={() => setImportExportModalVisible(true)}
        onOpenVersionHistory={() => setVersionHistoryVisible(true)}
        onOpenTools={() => setToolsPanelVisible(true)}
        onUndo={() => {
          if (canUndo()) {
            undo();
          }
        }}
        onRedo={() => {
          if (canRedo()) {
            redo();
          }
        }}
        onAutoLayout={handleAutoLayout}
        selectedNodeIds={selectedNodeIds}
        onBatchEdit={() => setBatchEditVisible(!batchEditVisible)}
        batchEditVisible={batchEditVisible}
        canUndo={canUndo()}
        canRedo={canRedo()}
        aiPanelVisible={aiPanelVisible}
        debugPanelVisible={debugPanelVisible}
        onSaveAsImage={handleSaveAsImage}
        onRun={() => setDebugPanelVisible(true)}
        onSettings={() => setRightPanelCollapsed(false)}
      />

      {searchVisible && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "4px 12px",
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
            background: token.colorBgElevated,
          }}
        >
          <input
            autoFocus
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value);
              setSearchIdx(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") { navigateSearch(1); }
              if (e.key === "Escape") { setSearchVisible(false); }
            }}
            placeholder={t("workflow.searchNodes")}
            style={{
              flex: 1,
              padding: "3px 8px",
              fontSize: 12,
              borderRadius: 4,
              border: `1px solid ${token.colorBorderSecondary}`,
              background: token.colorBgContainer,
              color: token.colorText,
            }}
          />
          <span style={{ fontSize: 11, color: token.colorTextQuaternary }} aria-live="polite">
            {searchResults.length > 0 ? `${searchIdx + 1}/${searchResults.length}` : "0"}
          </span>
          <Button
            size="small"
            onClick={() => navigateSearch(-1)}
            disabled={searchResults.length === 0}
            aria-label={t("workflow.search.prev", { defaultValue: "Previous match" })}
            aria-keyshortcuts="Shift+Enter"
          >
            ▲
          </Button>
          <Button
            size="small"
            onClick={() => navigateSearch(1)}
            disabled={searchResults.length === 0}
            aria-label={t("workflow.search.next", { defaultValue: "Next match" })}
            aria-keyshortcuts="Enter"
          >
            ▼
          </Button>
          <Button
            size="small"
            onClick={() => setSearchVisible(false)}
            aria-label={t("workflow.search.close", { defaultValue: "Close search" })}
            aria-keyshortcuts="Escape"
          >
            ✕
          </Button>
        </div>
      )}

      <div style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
        {!leftPanelCollapsed && <LeftPanel width={leftPanelWidth} />}
        {!leftPanelCollapsed && (
          <div
            onMouseDown={() => setResizing("left")}
            style={{
              width: 4,
              cursor: "col-resize",
              background: resizing === "left" ? token.colorPrimary : "transparent",
              flexShrink: 0,
              transition: "background 0.15s",
            }}
            onMouseEnter={(e) => {
              if (resizing !== "left") { e.currentTarget.style.background = token.colorBorderSecondary; }
            }}
            onMouseLeave={(e) => {
              if (resizing !== "left") { e.currentTarget.style.background = "transparent"; }
            }}
          />
        )}

        <div ref={canvasContainerRef} style={{ flex: 1, position: "relative" }}>
          {isInitialized
            ? (
              <ReactFlow
                nodes={reactFlowNodes}
                edges={reactFlowEdges}
                onNodesChange={handleNodesChange}
                onEdgesChange={handleEdgesChange}
                onConnect={onConnect}
                onNodeClick={onNodeClick}
                onEdgeClick={onEdgeClick}
                onPaneClick={onPaneClick}
                onNodeContextMenu={handleNodeContextMenu}
                onMoveEnd={onMoveEnd}
                onNodeDragStart={handleNodeDragStart}
                onNodeDrag={handleNodeDrag}
                onNodeDragStop={handleNodeDragStop}
                nodeTypes={nodeTypes}
                edgeTypes={edgeTypes}
                defaultEdgeOptions={defaultEdgeOptions}
                fitView
                // 修复9：优化 fitView 选项——增加 padding 避免贴边；maxZoom 限制避免过大节点
                fitViewOptions={{ padding: 0.15, includeHiddenNodes: false, maxZoom: 1.2, duration: 400 }}
                snapToGrid
                snapGrid={[20, 20]}
                selectionOnDrag
                connectionLineStyle={{
                  stroke: token.colorPrimary,
                  strokeWidth: 2,
                  strokeDasharray: "6 3",
                }}
                connectionLineType={ConnectionLineType.SmoothStep}
                multiSelectionKeyCode="Shift"
              >
                <EdgeMarkers />
                <Background
                  variant={BackgroundVariant.Dots}
                  color={token.colorBorderSecondary}
                  gap={20}
                  size={1}
                  style={{ opacity: 0.5 }}
                />
                <Controls style={{ borderRadius: 8 }} />
                <MiniMap
                  nodeColor={(node: Node) => (node.data as { color?: string })?.color || token.colorTextQuaternary}
                  maskColor={token.colorBgMask}
                  pannable
                  zoomable
                  nodeBorderRadius={4}
                  style={{
                    width: 140,
                    height: 90,
                    borderRadius: 8,
                    border: `1px solid ${token.colorBorderSecondary}`,
                    boxShadow: "0 2px 8px rgba(0,0,0,0.12)",
                  }}
                />
                {nodes.length === 0 && (
                  <Panel
                    position="top-center"
                    style={{
                      textAlign: "center",
                      color: token.colorTextSecondary,
                    }}
                  >
                    {t("workflow.dragToStart")}
                  </Panel>
                )}
                {nodes.length >= 2 && (
                  <Panel position="top-right">
                    <WorkflowLegend />
                  </Panel>
                )}
                {selectedNodeIds.size >= 2 && batchEditVisible && (
                  <BatchEditPanel
                    selectedNodeIds={selectedNodeIds}
                    onClose={() => setBatchEditVisible(false)}
                  />
                )}
              </ReactFlow>
            )
            : (
              <div
                className="react-flow"
                style={{
                  width: "100%",
                  height: "100%",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  background: token.colorBgContainer,
                  color: token.colorTextSecondary,
                }}
              >
                <Spin />
              </div>
            )}
        </div>

        {!rightPanelCollapsed && (
          <div
            onMouseDown={() => setResizing("right")}
            style={{
              width: 4,
              cursor: "col-resize",
              background: resizing === "right" ? token.colorPrimary : "transparent",
              flexShrink: 0,
              transition: "background 0.15s",
            }}
            onMouseEnter={(e) => {
              if (resizing !== "right") { e.currentTarget.style.background = token.colorBorderSecondary; }
            }}
            onMouseLeave={(e) => {
              if (resizing !== "right") { e.currentTarget.style.background = "transparent"; }
            }}
          />
        )}
        {!rightPanelCollapsed && (
          <RightPanel width={rightPanelWidth} selectedNodeId={selectedNodeId} selectedEdge={selectedEdge} />
        )}
      </div>

      {aiPanelVisible && (
        <div
          style={{
            background: token.colorBgElevated,
            borderTop: `1px solid ${token.colorBorderSecondary}`,
            display: "flex",
            flexDirection: "column",
            flexShrink: 0,
          }}
        >
          <div
            style={{
              height: 4,
              cursor: "ns-resize",
              background: token.colorBorderSecondary,
              transition: "background 0.2s",
            }}
            onMouseDown={(e) => {
              e.preventDefault();
              const startY = e.clientY;
              const startHeight = aiPanelHeight;
              const onMouseMove = (moveEvent: MouseEvent) => {
                const delta = startY - moveEvent.clientY;
                const newHeight = Math.max(200, Math.min(600, startHeight + delta));
                setAiPanelHeight(newHeight);
              };
              const onMouseUp = () => {
                document.removeEventListener("mousemove", onMouseMove);
                document.removeEventListener("mouseup", onMouseUp);
              };
              document.addEventListener("mousemove", onMouseMove);
              document.addEventListener("mouseup", onMouseUp);
            }}
          />
          <div style={{ height: aiPanelHeight, overflow: "auto" }}>
            <AIPanel
              onGenerateWorkflow={generateWorkflowFromPrompt}
              onOptimizePrompt={optimizeAgentPrompt}
              onRecommendNodes={recommendNodes}
              onClose={() => setAiPanelVisible(false)}
              selectedNodeId={selectedNodeId}
              selectedNodePrompt={selectedNodeId
                ? (nodes.find(n => n.id === selectedNodeId) as unknown as { config?: { systemPrompt?: string } }) // SAFE: accessing config.systemPrompt on WorkflowNode union
                  ?.config?.systemPrompt ?? null
                : null}
              onApplyPromptToNode={applyOptimizedPromptToNode}
              chatMessages={aiChatMessages}
              chatStreaming={aiChatStreaming}
              onChatSend={aiChatSend}
              onChatCancel={aiChatCancel}
              onChatClear={aiChatClear}
            />
          </div>
        </div>
      )}

      {debugPanelVisible && (
        <div
          style={{
            height: 300,
            background: token.colorBgElevated,
            borderTop: `1px solid ${token.colorBorderSecondary}`,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          <DebugPanel workflowId={templateId} />
        </div>
      )}

      <StatusBar
        nodeCount={nodes.length}
        edgeCount={edges.length}
        validationResult={validationResult}
        isDirty={isDirty}
        zoom={zoom}
        onFitView={handleFitView}
        onResetZoom={handleResetZoom}
      />

      {error && (
        <div
          style={{
            position: "fixed",
            bottom: 60,
            left: "50%",
            transform: "translateX(-50%)",
            color: token.colorError,
          }}
        >
          {error}
        </div>
      )}

      <ImportExportModal
        open={importExportModalVisible}
        onClose={() => setImportExportModalVisible(false)}
        onExport={exportTemplate}
        onImport={importTemplate}
        templates={templates}
        onImportComplete={() => {
          loadTemplates();
        }}
        onImportedTemplate={handleImportedTemplate}
        onExportYaml={handleExportYaml}
        onImportYaml={handleImportYaml}
      />

      <VersionHistoryModal
        visible={versionHistoryVisible}
        template={currentTemplate}
        onClose={() => setVersionHistoryVisible(false)}
        onLoadVersion={(tmpl) => {
          setVersionHistoryVisible(false);
          loadTemplate(tmpl.id);
        }}
      />

      <WorkflowToolsPanel
        workflowId={currentTemplate?.id ?? templateId ?? ""}
        workflowName={currentTemplate?.name ?? t("workflow.editor.untitled")}
        open={toolsPanelVisible}
        onClose={() => setToolsPanelVisible(false)}
      />

      <DiagnosticDrawer
        open={diagnoseDrawerVisible}
        onClose={() => setDiagnoseDrawerVisible(false)}
        onJumpToNode={(nodeId) => {
          setSelectedNode(nodeId);
          setDiagnoseDrawerVisible(false);
        }}
      />

      {/* Context menu */}
      {contextMenu && (
        <div
          role="menu"
          aria-label={t("workflow.contextMenu.label", { defaultValue: "Node actions" })}
          style={{
            position: "fixed",
            left: contextMenu.x,
            top: contextMenu.y,
            zIndex: 1000,
            background: token.colorBgElevated,
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: 8,
            boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
            minWidth: 160,
            padding: 4,
          }}
        >
          {["edit", "toggleBreakpoint", "copyNode", "deleteNode"].map((action) => (
            <div
              key={action}
              role="menuitem"
              tabIndex={0}
              style={{
                padding: "6px 10px",
                fontSize: 12,
                cursor: "pointer",
                borderRadius: 4,
                color: action === "deleteNode" ? token.colorError : undefined,
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = token.colorFillQuaternary)}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  e.currentTarget.click();
                }
              }}
              onClick={() => {
                if (action === "edit") { setSelectedNode(contextMenu.nodeId); }
                else if (action === "copyNode") {
                  const foundNode = nodes.find((n) => n.id === contextMenu.nodeId);
                  if (foundNode) {
                    clipboardRef.current = [foundNode];
                  }
                } else if (action === "deleteNode") {
                  deleteNode(contextMenu.nodeId);
                  setSelectedNode(null);
                } else if (action === "toggleBreakpoint") {
                  const engineStore = useWorkEngineStore.getState();
                  engineStore.toggleBreakpoint(contextMenu.nodeId);
                }
                setContextMenu(null);
              }}
            >
              <span aria-hidden="true" style={{ display: "inline-block", width: 14, textAlign: "center" }}>
                {action === "edit" ? "✎" : action === "toggleBreakpoint" ? "●" : action === "copyNode" ? "⎘" : "✕"}
              </span>
              <span>{t(`workflow.${action}`)}</span>
            </div>
          ))}
        </div>
      )}

      <SemanticCheckModal
        open={semanticCheckResult !== null}
        onClose={() => clearSemanticCheckResult()}
        matches={semanticCheckResult?.matches ?? []}
        onApplyReplacement={(nodeId, existingSkillId, action) => {
          applySkillReplacement(nodeId, existingSkillId, action);
        }}
      />
    </div>
  );
};

function getDefaultNodeConfig(nodeType: string): Record<string, unknown> {
  switch (nodeType) {
    case "trigger":
      return { type: "manual", config: {} };
    case "agent":
      return {
        systemPrompt: "",
        tools: [],
        contextSources: [],
        agentProfileId: undefined,
        outputMode: "text",
        model: undefined,
      };
    case "multiAgent":
      return {
        task: "",
        role: undefined,
        model: undefined,
        outputVar: "",
        mode: "auto",
        maxRounds: 3,
      };
    case "llm":
      return { model: "", prompt: "", temperature: 0.7, maxTokens: 2048 };
    case "condition":
      return { conditions: [], logicalOp: "and" };
    case "parallel":
      return { branches: [], waitForAll: true, aggregation: undefined, kind: "executable" };
    case "loop":
      return {
        loopType: "forEach",
        maxIterations: 100,
        continueOnError: false,
        bodySteps: [],
      };
    case "tool":
      return { toolName: "", inputMapping: {}, outputVar: "" };
    case "code":
      return { language: "javascript", code: "", outputVar: "" };
    case "merge":
      return { mergeType: "all", inputs: [] };
    case "delay":
      return { delayType: "seconds", seconds: 5 };
    case "subWorkflow":
      return {
        subWorkflowId: "",
        inputMapping: {},
        outputVar: "",
        isAsync: false,
      };
    case "workflowRef":
      return {
        targetWorkflowId: "",
        inputMapping: {},
        outputVar: "",
        contextMode: "inherit",
      };
    case "documentParser":
      return { inputVar: "", parserType: "", outputVar: "" };
    case "vectorRetrieve":
      return { query: "", knowledgeBaseId: "", topK: 5, outputVar: "" };
    case "end":
      return {};
    case "validation":
      return { assertions: [], onFail: "stop" as const, maxRetries: 0 };
    case "_phaseSeparator":
      return { label: "", width: 800 };
    case "groupFrame":
      return { borderColor: "", collapsed: false };
    default:
      return {};
  }
}

function createWorkflowNode(
  id: string,
  type: string,
  position: { x: number; y: number },
  title: string,
  parentId?: string,
): WorkflowNode {
  const baseNode = {
    id,
    title,
    description: "",
    position,
    retry: {
      enabled: false,
      maxRetries: 3,
      backoffType: "Exponential" as const,
      baseDelayMs: 1000,
      maxDelayMs: 60000,
    },
    timeout: undefined,
    enabled: true,
    parentId,
  };

  switch (type) {
    case "trigger":
      return {
        ...baseNode,
        type: "trigger",
        config: { type: "manual", config: {} },
      };
    case "agent":
      return {
        ...baseNode,
        type: "agent",
        config: {
          systemPrompt: "",
          contextSources: [],
          outputVar: "",
          tools: [],
          exposedTools: [],
          outputMode: "text",
          agentProfileId: undefined,
          maxToolRounds: undefined,
          executionMode: undefined,
          ragSourceIds: [],
        },
      };
    case "multiAgent":
      return {
        ...baseNode,
        type: "multiAgent",
        config: {
          task: "",
          role: undefined,
          model: undefined,
          outputVar: "",
          mode: "auto",
          maxRounds: 3,
        },
      };
    case "llm":
      return {
        ...baseNode,
        type: "llm",
        config: { model: "", prompt: "", temperature: 0.7, maxTokens: 2048 },
      };
    case "condition":
      return {
        ...baseNode,
        type: "condition",
        config: { conditions: [], logicalOp: "and" },
      };
    case "parallel":
      return {
        ...baseNode,
        type: "parallel",
        config: { branches: [], waitForAll: true, aggregation: undefined, kind: "executable" },
      };
    case "loop":
      return {
        ...baseNode,
        type: "loop",
        config: {
          loopType: "forEach",
          maxIterations: 100,
          continueOnError: false,
          bodySteps: [],
        },
      };
    case "merge":
      return {
        ...baseNode,
        type: "merge",
        config: { mergeType: "all", inputs: [] },
      };
    case "delay":
      return {
        ...baseNode,
        type: "delay",
        config: { delayType: "seconds", seconds: 5 },
      };
    case "tool":
      return {
        ...baseNode,
        type: "tool",
        config: { toolName: "", inputMapping: {}, outputVar: "" },
      };
    case "code":
      return {
        ...baseNode,
        type: "code",
        config: { language: "javascript", code: "", outputVar: "" },
      };
    case "subWorkflow":
      return {
        ...baseNode,
        type: "subWorkflow",
        config: {
          subWorkflowId: "",
          inputMapping: {},
          outputVar: "",
          isAsync: false,
        },
      };
    case "workflowRef":
      return {
        ...baseNode,
        type: "workflowRef",
        config: {
          targetWorkflowId: "",
          inputMapping: {},
          outputVar: "",
          contextMode: "inherit",
        },
      };
    case "documentParser":
      return {
        ...baseNode,
        type: "documentParser",
        config: { inputVar: "", parserType: "", outputVar: "" },
      };
    case "vectorRetrieve":
      return {
        ...baseNode,
        type: "vectorRetrieve",
        config: { query: "", knowledgeBaseId: "", topK: 5, outputVar: "" },
      };
    case "end":
      return { ...baseNode, type: "end", config: {} };
    case "validation":
      return {
        ...baseNode,
        type: "validation",
        config: { assertions: [], onFail: "stop" as const, maxRetries: 0 },
      };
    case "httpRequest":
      return {
        ...baseNode,
        type: "httpRequest",
        config: {
          url: "",
          method: "GET",
          headers: {},
          bodyType: "none",
          timeoutSecs: 30,
          outputVar: "",
        },
      };
    case "switch":
      return {
        ...baseNode,
        type: "switch",
        config: {
          inputVar: "",
          cases: [],
          matchMode: "exact",
          outputVar: "",
        },
      };
    case "databaseQuery":
      return {
        ...baseNode,
        type: "databaseQuery",
        config: {
          query: "",
          params: [],
          timeoutSecs: 30,
          outputVar: "",
        },
      };
    case "notification":
      return {
        ...baseNode,
        type: "notification",
        config: {
          channel: "webhook",
          message: "",
          recipients: [],
          enabled: true,
          outputVar: "",
        },
      };
    case "approval":
      return {
        ...baseNode,
        type: "approval",
        config: {
          message: "",
          timeoutSecs: 3600,
          timeoutAction: "reject",
          outputVar: "",
        },
      };
    case "fileOperation":
      return {
        ...baseNode,
        type: "fileOperation",
        config: { operation: "read", filePath: "", outputVar: "" },
      };
    case "dataTransformer":
      return {
        ...baseNode,
        type: "dataTransformer",
        config: { inputVar: "", expression: "", outputVar: "" },
      };
    case "webhookSend":
      return {
        ...baseNode,
        type: "webhookSend",
        config: {
          url: "",
          method: "POST",
          headers: {},
          outputVar: "",
        },
      };
    case "logging":
      return {
        ...baseNode,
        type: "logging",
        config: { level: "info", message: "", outputVar: "" },
      };
    case "llmClassifier":
      return {
        ...baseNode,
        type: "llmClassifier",
        config: {
          categories: [],
          prompt: "",
          inputVar: "",
          outputVar: "",
        },
      };
    case "aggregator":
      return {
        ...baseNode,
        type: "aggregator",
        config: { strategy: "concat", inputSources: [], outputVar: "" },
      };
    case "email":
      return {
        ...baseNode,
        type: "email",
        config: {
          to: [],
          subject: "",
          body: "",
          outputVar: "",
        },
      };
    case "debate":
      return {
        ...baseNode,
        type: "debate",
        config: {
          debaterSteps: [],
          maxRounds: 3,
          topicVar: "",
          outputVar: "",
        },
      };
    case "storage":
      return {
        ...baseNode,
        type: "storage",
        config: {
          backend: "sqlite",
          operation: "insert",
          inputVar: "",
          collection: "",
          keyVar: undefined,
          outputVar: "",
        },
      };
    case "swarm":
      return {
        ...baseNode,
        type: "swarm",
        config: {
          agentSteps: [],
          maxRounds: 3,
          topicVar: "",
          outputVar: "",
        },
      };
    default:
      console.warn(`[createWorkflowNode] Unknown node type "${type}", falling back to agent`);
      return {
        ...baseNode,
        type: "agent" as const,
        config: {},
      } as unknown as WorkflowNode; // SAFE: fallback agent node constructed from base with correct type discriminator
  }
}
