// SPDX-License-Identifier: AGPL-3.0-only

import {
  Background,
  BackgroundVariant,
  Connection,
  ConnectionLineType,
  Controls,
  type Edge,
  MiniMap,
  type Node,
  Panel,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "@xyflow/react";
import domtoimage from "dom-to-image-more";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import "@xyflow/react/dist/style.css";
import { invoke, isTauri } from "@/lib/invoke";
import {
  auto_layout,
  autoLayoutWorkflow,
  type AutoNode,
  find_safe_position,
  getNodeSize,
  type NodePositionLike,
  toAbsolutePosition,
  toRelativePosition,
  validate_workflow,
  type ValidateIssue,
  would_create_cycle,
} from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";

import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { Button, message, Modal, Spin, theme } from "antd";
import { useTranslation } from "react-i18next";
import { AIPanel } from "./AIPanel/AIPanel";
import { CanvasTitleBar } from "./CanvasTitleBar";
import { DebugPanel } from "./DebugPanel";
import { DiagnosticDrawer } from "./Diagnostic";
import { clearDragPayload, getDragPayload } from "./dndState";
import { BaseEdge } from "./Edges/BaseEdge";
import { EditorHeader } from "./Header/EditorHeader";
import { useFlowNodes } from "./Hooks/useFlowNodes";
import { useKeyboardShortcuts } from "./Hooks/useKeyboardShortcuts";
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
import { NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "./types";
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
  onClose?: () => void;
}

export const WorkflowEditor: React.FC<WorkflowEditorProps> = ({
  templateId,
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

  const [reactFlowNodes, setRNodes, onNodesChange] = useNodesState<Node>([]);
  const [reactFlowEdges, setREdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [isInitialized, setIsInitialized] = React.useState(false);
  const hasAutoLaidOutRef = React.useRef(false);
  const autoLayoutTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const autoSaveTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const canvasContainerRef = React.useRef<HTMLDivElement>(null);
  const clipboardRef = React.useRef<WorkflowNode[]>([]);
  const edgesRef = React.useRef(edges);
  // eslint-disable-next-line react-hooks/refs
  edgesRef.current = edges;
  // 拖拽时的位置批处理：RAF 合并多次像素级位置变更，只写最后一次到 store
  const pendingPositionsRef = React.useRef<Map<string, { x: number; y: number }>>(new Map());
  const posRafRef = React.useRef<number | null>(null);
  // 拖拽状态标志：拖拽期间抑制 useEffect 全量重建节点和 store 位置写入，防止崩溃
  const isDraggingRef = React.useRef(false);
  const removeIdsRef = React.useRef<Set<string>>(new Set());
  // dragStop 后短暂抑制 useEffect 全量重建，避免覆盖 reactFlowInstance.setNodes 的结果
  const suppressRebuildRef = React.useRef(false);
  // 跳过写入标志：程序化 setRNodes（如 autoLayout）后抑制 onNodesChange 中的重复 updateNode
  const skipPositionWriteRef = React.useRef(false);
  // 拖拽停止版本计数器：每次 dragStop 后 +1，加入 useEffect 依赖确保容器尺寸重算
  const [, setDragStopVersion] = useState(0);
  const [aiPanelVisible, setAiPanelVisible] = useState(false);
  const [aiPanelHeight, setAiPanelHeight] = useState(300);
  const [debugPanelVisible, setDebugPanelVisible] = useState(false);
  const [importExportModalVisible, setImportExportModalVisible] = useState(false);
  const [versionHistoryVisible, setVersionHistoryVisible] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; nodeId: string } | null>(null);
  const [searchVisible, setSearchVisible] = useState(false);
  const [selectedNodeIds, setSelectedNodeIds] = useState<Set<string>>(new Set());
  const [batchEditVisible, setBatchEditVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchIdx, setSearchIdx] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [dndDropTargetId, setDndDropTargetId] = useState<string | null>(null);
  const [leftPanelCollapsed, setLeftPanelCollapsed] = useState(
    () => localStorage.getItem("workflowEditor.leftPanelCollapsed") === "true",
  );
  const [rightPanelCollapsed, setRightPanelCollapsed] = useState(
    () => localStorage.getItem("workflowEditor.rightPanelCollapsed") === "true",
  );
  const [leftPanelWidth, setLeftPanelWidth] = useState(() => {
    const saved = localStorage.getItem("workflowEditor.leftPanelWidth");
    return saved ? Number(saved) : 280;
  });
  const [frontendValidation, setFrontendValidation] = useState<ValidateIssue[]>([]);
  const [validationMsgMap, setValidationMsgMap] = useState<Map<string, string>>(new Map());
  const [rightPanelWidth, setRightPanelWidth] = useState(() => {
    const saved = localStorage.getItem("workflowEditor.rightPanelWidth");
    return saved ? Number(saved) : 320;
  });
  const [resizing, setResizing] = useState<"left" | "right" | null>(null);

  // 面板拖拽调宽
  useEffect(() => {
    if (!resizing) { return; }
    const handleMouseMove = (e: MouseEvent) => {
      if (resizing === "left") {
        setLeftPanelWidth((prev) => {
          const next = Math.max(180, Math.min(600, prev + e.movementX));
          localStorage.setItem("workflowEditor.leftPanelWidth", String(next));
          return next;
        });
      } else {
        setRightPanelWidth((prev) => {
          const next = Math.max(200, Math.min(600, prev - e.movementX));
          localStorage.setItem("workflowEditor.rightPanelWidth", String(next));
          return next;
        });
      }
    };
    const handleMouseUp = () => setResizing(null);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [resizing]);

  // 响应式：窗口过小时自动折叠面板
  useEffect(() => {
    const checkWidth = () => {
      const w = window.innerWidth;
      if (w < 900) {
        if (!leftPanelCollapsed) {
          setLeftPanelCollapsed(true);
          localStorage.setItem("workflowEditor.leftPanelCollapsed", "true");
        }
      }
      if (w < 1100) {
        if (!rightPanelCollapsed) {
          setRightPanelCollapsed(true);
          localStorage.setItem("workflowEditor.rightPanelCollapsed", "true");
        }
      }
    };
    checkWidth();
    window.addEventListener("resize", checkWidth);
    return () => window.removeEventListener("resize", checkWidth);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const validationTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (validationTimerRef.current) { clearTimeout(validationTimerRef.current); }
    validationTimerRef.current = setTimeout(() => {
      const issues = validate_workflow(nodes, edges, t);
      setFrontendValidation(issues.issues);
      const msgMap = new Map<string, string>();
      for (const iss of issues.issues) {
        for (const nid of iss.nodeIds) {
          const prev = msgMap.get(nid);
          msgMap.set(nid, prev ? `${prev}; ${iss.message}` : iss.message);
        }
      }
      setValidationMsgMap(msgMap);
    }, 300);
    return () => {
      if (validationTimerRef.current) { clearTimeout(validationTimerRef.current); }
    };
  }, [nodes, edges, t]);

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

  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    hasAutoLaidOutRef.current = false;
    if (templateId) {
      loadTemplate(templateId);
    } else {
      initNewTemplate();
    }
  }, [templateId]);
  /* eslint-enable react-hooks/exhaustive-deps */

  /** 收集所有容器 subGraph 内的节点 ID，从顶层 nodes 中排除 */
  const autoSaveRetryCountRef = React.useRef(0);
  const MAX_AUTO_SAVE_RETRIES = 3;
  useEffect(() => {
    if (!isDirty || isSaving || isDecompositionTemplate) {
      return;
    }

    autoSaveTimerRef.current = setTimeout(async () => {
      const state = useWorkflowEditorStore.getState();
      if (!state.isDirty || state.isSaving || state.isDecompositionTemplate) {
        return;
      }

      const { nodes, edges, parentRefs, currentTemplate } = state;
      const nodesWithParent: WorkflowNode[] = nodes.map((n) => {
        const pid = parentRefs[n.id];
        if (pid === undefined) { return n; }
        return { ...n, parentId: pid } as WorkflowNode;
      });
      const input = {
        name: currentTemplate?.name || "Unnamed Workflow",
        description: currentTemplate?.description,
        icon: currentTemplate?.icon || "Bot",
        tags: currentTemplate?.tags || [],
        trigger_config: currentTemplate?.trigger_config,
        nodes: nodesWithParent,
        edges,
        input_schema: currentTemplate?.input_schema,
        output_schema: currentTemplate?.output_schema,
        variables: currentTemplate?.variables || [],
        error_config: currentTemplate?.error_config,
      };

      try {
        if (currentTemplate?.id) {
          await invoke<boolean>("update_workflow_template", { id: currentTemplate.id, input });
          useWorkflowEditorStore.setState({ isDirty: false, isSaving: false });
        } else {
          const newId = await invoke<string>("create_workflow_template", { input });
          if (newId) {
            useWorkflowEditorStore.setState({ isDirty: false, isSaving: false });
          }
        }
        autoSaveRetryCountRef.current = 0;
      } catch {
        autoSaveRetryCountRef.current++;
        if (autoSaveRetryCountRef.current >= MAX_AUTO_SAVE_RETRIES) {
          useWorkflowEditorStore.setState({ error: "Auto-save failed after 3 retries" });
          autoSaveRetryCountRef.current = 0;
        }
      }
    }, 5000);

    return () => {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current);
        autoSaveTimerRef.current = null;
      }
    };
  }, [
    isDirty,
    isSaving,
    isDecompositionTemplate,
  ]);

  useEffect(() => {
    if (isDraggingRef.current || suppressRebuildRef.current) { return; }

    setRNodes(computedFlowNodes);
    setREdges(computedFlowEdges);
    setIsInitialized(true);

    for (const [childId, expectedParent] of Object.entries(expectedParentByNode)) {
      if (parentRefs[childId] !== expectedParent) {
        setParentRef(childId, expectedParent);
      }
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
          skipPositionWriteRef.current = true;
          setRNodes(layouted);
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
      const wouldCycle = would_create_cycle(
        allEdges,
        params.source,
        params.target,
      );
      if (wouldCycle) {
        if (sourceHandle === "loopBack") {
          message.warning(t("workflow.loopBackCycleDetected"));
          return;
        }
        message.warning(
          t("workflow.cycleDetectedOnConnect", {
            defaultValue: "This edge creates a cycle without a loopBack marker — the workflow engine may reject it.",
          }),
        );
      }
      // Determine edge type based on sourceHandle
      let edgeType: WorkflowEdge["edge_type"] = "direct";
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
        edge_type: edgeType,
      };
      storeAddEdge(newEdge);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [storeAddEdge],
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
        const existingNodes = useWorkflowEditorStore.getState().nodes;
        let hitContainerId: string | null = null;
        for (const n of existingNodes) {
          if (!NODE_TYPE_MAP[n.type]?.isContainer) { continue; }
          // Use ReactFlow measured dimensions for accurate hit-test
          const rfNode = reactFlowInstance?.getNodes().find((rfn) => rfn.id === n.id);
          const w = rfNode?.measured?.width ?? getNodeSize(n.type).width;
          const h = rfNode?.measured?.height ?? getNodeSize(n.type).height;
          if (
            position.x >= n.position.x
            && position.x <= n.position.x + w
            && position.y >= n.position.y
            && position.y <= n.position.y + h
          ) {
            hitContainerId = n.id;
            break;
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
          useWorkflowEditorStore.getState().setParentRef(id, hitContainerId);
        }
      } catch (error) {
        message.error(t("workflow.nodeDropFailed", { error: String(error) }));
      } finally {
        clearDragPayload();
      }
    };

    window.addEventListener("mouseup", handleGlobalMouseUp);
    return () => window.removeEventListener("mouseup", handleGlobalMouseUp);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reactFlowInstance, setRNodes]);

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
        let hitId: string | null = null;
        for (const n of existingNodes) {
          if (!NODE_TYPE_MAP[n.type]?.isContainer) { continue; }
          const rfNode = reactFlowInstance?.getNodes().find((rfn) => rfn.id === n.id);
          const w = rfNode?.measured?.width ?? getNodeSize(n.type).width;
          const h = rfNode?.measured?.height ?? getNodeSize(n.type).height;
          if (
            position.x >= n.position.x && position.x <= n.position.x + w
            && position.y >= n.position.y && position.y <= n.position.y + h
          ) {
            hitId = n.id;
            break;
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
        message.error(String(e));
      }
      return;
    }

    // 前端结构校验：error 级别阻塞保存，warning 级别仅提示
    const frontendIssues = validate_workflow(nodes, edges, t);
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
    if (validation && !validation.is_valid) {
      message.error(
        t("workflow.validationFailed", { count: validation.errors.length }),
      );
      return;
    }

    // 注入 parentRefs 到节点，与 auto-save 逻辑一致，确保容器父子关系持久化
    const nodesWithParent: WorkflowNode[] = nodes.map((n) => {
      const pid = parentRefs[n.id];
      if (pid === undefined) { return n; }
      // Store 始终存绝对坐标，保存时也保持绝对坐标。
      // 加载时 rebuildParentRefsFromNodes 恢复 parentRefs，useEffect 再将绝对坐标转为相对坐标给 ReactFlow。
      return { ...n, parentId: pid } as WorkflowNode;
    });

    const input = {
      name: currentTemplate.name,
      description: currentTemplate.description,
      icon: currentTemplate.icon,
      tags: currentTemplate.tags,
      trigger_config: currentTemplate.trigger_config,
      nodes: nodesWithParent,
      edges,
      input_schema: currentTemplate.input_schema,
      output_schema: currentTemplate.output_schema,
      variables: currentTemplate.variables,
      error_config: currentTemplate.error_config,
    };

    if (currentTemplate.id) {
      const ok = await updateTemplate(currentTemplate.id, input);
      if (ok) {
        message.success(t("workflow.saved"));
      }
    } else {
      const newId = await createTemplate(input);
      if (newId) {
        await loadTemplate(newId);
        message.success(t("workflow.saved"));
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
  // eslint-disable-next-line react-hooks/refs
  handleSaveRef.current = handleSave;

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
      // Build a map of parent positions for converting child relative coords to absolute
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const nodeMap = new Map<string, any>();
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      nodes.forEach((n: any) => nodeMap.set(n.id, n));
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const getAbsolutePosition = (node: any): { x: number; y: number } => {
        if (!node.parentId) { return node.position; }
        const parent = nodeMap.get(node.parentId);
        if (!parent) { return node.position; }
        const parentAbs = getAbsolutePosition(parent);
        return { x: node.position.x + parentAbs.x, y: node.position.y + parentAbs.y };
      };
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      nodes.forEach((node: any) => {
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
        const blob = await domtoimage.toBlob(container, {
          bgColor: "#1a1a2e",
          scale: 2,
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
        const dataUrl = await domtoimage.toPng(container, {
          bgColor: "#1a1a2e",
          scale: 2,
        });
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
    }
  }, [reactFlowInstance, currentTemplate, t]);

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

  // 卸载时清理 auto-save timeout
  useEffect(() => () => {
    if (autoSaveTimerRef.current) { clearTimeout(autoSaveTimerRef.current); }
  }, []);

  const handleNodesChange = useCallback(
    /* eslint-disable @typescript-eslint/no-explicit-any */
    (changes: any) => {
      // ReactFlow 内部 handleParentExpand 尝试直接修改 node.position，
      // 若 position 对象已被冻结则引发只读属性崩溃。
      // 此处深拷贝 changes 中的 position/dimensions，确保传给 ReactFlow 的都是可写的新对象。
      const clonedChanges = changes.map((c: any) => {
        let result = c;
        if (c.type && c.position) {
          result = { ...result, position: { ...c.position } };
        }
        if (c.dimensions) {
          result = { ...result, dimensions: { ...c.dimensions } };
        }
        // "add" 类型的 change 可能包含 item（节点对象），其 position 也可能冻结
        if (c.type === "add" && c.item?.position) {
          result = {
            ...result,
            item: { ...c.item, position: { ...c.item.position } },
          };
        }
        return result;
      });
      onNodesChange(clonedChanges);

      // Track multi-selection
      const hasSelectionChange = changes.some((c: any) => c.type === "select");
      if (hasSelectionChange) {
        const flowInstance = reactFlowInstance;
        if (flowInstance) {
          const selected = flowInstance.getNodes().filter((n: any) => n.selected);
          setSelectedNodeIds(new Set(selected.map((n: any) => n.id)));
        }
      }

      changes.forEach((change: any) => {
        if (
          change.type === "position" && change.position && currentTemplate && !isDraggingRef.current
          && !skipPositionWriteRef.current
        ) {
          // 方案 B：ReactFlow 在 extent:"parent" 模式下对子节点返回的是相对坐标，
          // 写入 store 时需要转换为画布绝对坐标；顶层节点直接透传。
          const storePos = toAbsolutePosition(
            change.id,
            change.position,
            parentRefs,
            useWorkflowEditorStore.getState().nodes as NodePositionLike[],
          );
          pendingPositionsRef.current.set(change.id, storePos);
          if (posRafRef.current == null) {
            posRafRef.current = requestAnimationFrame(() => {
              posRafRef.current = null;
              if (isDraggingRef.current) {
                return;
              }
              pendingPositionsRef.current.forEach((pos, nodeId) => {
                updateNode(nodeId, { position: pos } as Partial<WorkflowNode>);
              });
              pendingPositionsRef.current.clear();
            });
          }
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
    /* eslint-enable @typescript-eslint/no-explicit-any */
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [onNodesChange, currentTemplate, updateNode, deleteNode, nodes, parentRefs],
  );

  const handleNodeDragStart = useCallback(() => {
    isDraggingRef.current = true;
  }, []);

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
      setTimeout(() => {
        suppressRebuildRef.current = false;
      }, 50);

      if (node?.position) {
        useWorkflowEditorStore.getState().recordUndoSnapshot();
        const rfNodes = reactFlowInstance?.getNodes() || [];
        const latestNodes = useWorkflowEditorStore.getState().nodes;
        const latestParentRefs = useWorkflowEditorStore.getState().parentRefs;

        const draggedNodeParentId = latestParentRefs[node.id];
        let storePos: { x: number; y: number };
        let rfPos: { x: number; y: number };

        if (draggedNodeParentId) {
          storePos = toAbsolutePosition(
            node.id,
            node.position,
            latestParentRefs,
            latestNodes as NodePositionLike[],
          );
          rfPos = { x: node.position.x, y: node.position.y };
        } else {
          // 顶层节点：碰撞避免（仅与同层级顶层节点比较，排除子节点的相对坐标）
          const selectedIds = new Set(
            rfNodes.filter((n) => n.selected).map((n) => n.id),
          );
          const siblings = rfNodes
            .filter((n) => n.id !== node.id && !selectedIds.has(n.id) && !n.parentId)
            .map((n) => ({
              id: n.id,
              x: n.position.x,
              y: n.position.y,
              type: (n.data?.type as string) || n.type || "",
            }));
          const nodeType = (node.data?.type as string) || node.type || "";
          const safePos = find_safe_position(
            { x: node.position.x, y: node.position.y },
            nodeType,
            siblings,
          );
          storePos = safePos;
          rfPos = safePos;
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
          // After updating store positions, also update ReactFlow node positions for children
          requestAnimationFrame(() => {
            const currentRfNodes = reactFlowInstance?.getNodes() || [];
            const latestState = useWorkflowEditorStore.getState();
            reactFlowInstance?.setNodes(currentRfNodes.map((n) => {
              const childPid = latestState.parentRefs[n.id];
              if (childPid === node.id) {
                // Recalculate relative position from updated store absolute positions
                const childStoreNode = latestState.nodes.find((sn) => sn.id === n.id);
                const parentStoreNode = latestState.nodes.find((sn) => sn.id === node.id);
                if (childStoreNode && parentStoreNode) {
                  return {
                    ...n,
                    position: {
                      x: childStoreNode.position.x - parentStoreNode.position.x,
                      y: childStoreNode.position.y - parentStoreNode.position.y,
                    },
                  };
                }
              }
              return n;
            }));
          });
        }

        // 更新 ReactFlow 节点：子节点保留相对坐标，顶层节点用绝对坐标
        const updatedNodes = rfNodes.map((n) => {
          if (n.id === node.id) {
            return { ...n, position: rfPos };
          }
          if (n.selected && n.id !== node.id && n.position) {
            const selectedNodeParent = latestParentRefs[n.id];
            if (selectedNodeParent) {
              const absPos = toAbsolutePosition(
                n.id,
                n.position,
                latestParentRefs,
                latestNodes as NodePositionLike[],
              );
              updateNode(n.id, { position: absPos } as Partial<WorkflowNode>);
              return n;
            }
            // 选中的顶层节点：直接传入 RF 位置
            updateNode(n.id, { position: n.position } as Partial<WorkflowNode>);
          }
          return n;
        });
        reactFlowInstance?.setNodes(updatedNodes);

        // 强制触发容器尺寸重算：拖拽结束后确保 useEffect 重新运行
        const triggerParent = draggedNodeParentId || (isContainer ? node.id : undefined);
        if (triggerParent) {
          requestAnimationFrame(() => {
            setDragStopVersion((v) => v + 1);
          });
        }
      }
    },
    [updateNode, reactFlowInstance, setDragStopVersion],
  );

  const handleEdgesChange = useCallback(
    /* eslint-disable @typescript-eslint/no-explicit-any */
    (changes: any) => {
      onEdgesChange(changes);

      changes.forEach((change: any) => {
        if (change.type === "remove" && change.id) {
          deleteEdge(change.id);
        }
      });
    },
    /* eslint-enable @typescript-eslint/no-explicit-any */
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
    // 使用新的 auto_layout（按 type 分层 + Barycenter 启发式）
    const layoutedNodes = auto_layout(
      reactFlowNodes as unknown as AutoNode[],
      layoutEdges,
      parentRefs,
    );
    // auto_layout 返回值：所有节点 position = 绝对坐标
    // ReactFlow setRNodes 需要子节点为相对坐标，但 auto_layout 返回绝对坐标
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
        const subId = subNode.data?.subWorkflowId || subNode.data?.sub_workflow_id;
        if (!subId) { continue; }
        try {
          /* eslint-disable @typescript-eslint/no-explicit-any */
          const tmpl: any = await invoke("get_workflow_template", { id: subId });
          if (!tmpl?.nodes || !Array.isArray(tmpl.nodes)) { continue; }
          const subNodes = tmpl.nodes;
          const subEdges = tmpl.edges || [];
          // 转换为兼容格式
          const rfSubNodes = subNodes.map((n: any) => ({
            id: n.id || n.base?.id || "",
            type: (n.type || n.base?.type || "agent") as string,
            position: n.position || n.base?.position || { x: 0, y: 0 },
            data: { ...n, type: n.type || n.base?.type || "agent" },
          }));
          const rfSubEdges = subEdges.map((e: any, i: number) => ({
            id: e.id || `sub_e_${i}`,
            source: e.source,
            target: e.target,
            sourceHandle: e.source_handle || e.sourceHandle,
            targetHandle: e.target_handle || e.targetHandle,
          }));
          const subLayouted = auto_layout(rfSubNodes, rfSubEdges);
          // 回写位置
          const updatedSubNodes = subNodes.map((n: any) => {
            const nodeId = n.id || n.base?.id || "";
            const laid = subLayouted.find((ln: any) => ln.id === nodeId);
            if (!laid) { return n; }
            if (n.base) {
              return { ...n, base: { ...n.base, position: laid.position } };
            }
            return { ...n, position: laid.position };
          });
          /* eslint-enable @typescript-eslint/no-explicit-any */
          const input = {
            name: tmpl.name || "",
            icon: tmpl.icon || "",
            tags: tmpl.tags || [],
            nodes: updatedSubNodes,
            edges: subEdges,
            variables: tmpl.variables || [],
            input_schema: tmpl.input_schema || undefined,
            output_schema: tmpl.output_schema || undefined,
            error_config: tmpl.error_config || undefined,
            trigger_config: tmpl.trigger_config || undefined,
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

    message.success(t("workflow.autoLayout"));
  }, [reactFlowNodes, reactFlowEdges, parentRefs, setRNodes, updateNode, t]);

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
        onToggleLeftPanel={() =>
          setLeftPanelCollapsed((v) => {
            const next = !v;
            localStorage.setItem("workflowEditor.leftPanelCollapsed", String(next));
            return next;
          })}
        onToggleRightPanel={() =>
          setRightPanelCollapsed((v) => {
            const next = !v;
            localStorage.setItem("workflowEditor.rightPanelCollapsed", String(next));
            return next;
          })}
        leftPanelCollapsed={leftPanelCollapsed}
        rightPanelCollapsed={rightPanelCollapsed}
        onOpenImportExport={() => setImportExportModalVisible(true)}
        onOpenVersionHistory={() => setVersionHistoryVisible(true)}
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
      />

      {/* 画布顶部名称条：面包屑 + 工作流名称 + 快捷工具栏 */}
      <CanvasTitleBar
        workflowName={currentTemplate?.name || t("workflow.newWorkflow")}
        isDirty={isDirty}
        isSaving={isSaving}
        onNameChange={handleNameChange}
        onSave={handleSave}
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

      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
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
                <Background
                  variant={BackgroundVariant.Lines}
                  color={token.colorBorderSecondary}
                  gap={20}
                  size={1}
                  style={{ opacity: 0.4 }}
                />
                <Controls style={{ borderRadius: 8 }} />
                <MiniMap
                  nodeColor={(node: Node) => (node.data as { color?: string })?.color || token.colorTextQuaternary}
                  maskColor={token.colorBgMask}
                  pannable
                  zoomable
                  nodeBorderRadius={4}
                  style={{
                    width: 180,
                    height: 120,
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
                ? (nodes.find(n => n.id === selectedNodeId) as unknown as { config?: { system_prompt?: string } })
                  ?.config?.system_prompt ?? null
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
    case "llm":
      return { model: "", prompt: "", temperature: 0.7, max_tokens: 2048 };
    case "condition":
      return { conditions: [], logical_op: "and" };
    case "parallel":
      return { branches: [], wait_for_all: true, aggregation: undefined, kind: "executable" };
    case "loop":
      return {
        loop_type: "forEach",
        max_iterations: 100,
        continue_on_error: false,
        body_steps: [],
      };
    case "tool":
      return { tool_name: "", input_mapping: {}, output_var: "" };
    case "code":
      return { language: "javascript", code: "", output_var: "" };
    case "merge":
      return { merge_type: "all", inputs: [] };
    case "delay":
      return { delay_type: "seconds", seconds: 5 };
    case "subWorkflow":
      return {
        sub_workflow_id: "",
        input_mapping: {},
        output_var: "",
        is_async: false,
      };
    case "workflowRef":
      return {
        target_workflow_id: "",
        input_mapping: {},
        output_var: "",
        context_mode: "inherit",
      };
    case "documentParser":
      return { input_var: "", parser_type: "", output_var: "" };
    case "vectorRetrieve":
      return { query: "", knowledge_base_id: "", top_k: 5, output_var: "" };
    case "end":
      return {};
    case "validation":
      return { assertions: [], on_fail: "stop" as const, max_retries: 0 };
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
      max_retries: 3,
      backoff_type: "Exponential" as const,
      base_delay_ms: 1000,
      max_delay_ms: 60000,
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
          system_prompt: "",
          context_sources: [],
          output_var: "",
          tools: [],
          exposed_tools: [],
          output_mode: "text",
          agentProfileId: undefined,
          max_tool_rounds: undefined,
          execution_mode: undefined,
          rag_source_ids: [],
        },
      };
    case "llm":
      return {
        ...baseNode,
        type: "llm",
        config: { model: "", prompt: "", temperature: 0.7, max_tokens: 2048 },
      };
    case "condition":
      return {
        ...baseNode,
        type: "condition",
        config: { conditions: [], logical_op: "and" },
      };
    case "parallel":
      return {
        ...baseNode,
        type: "parallel",
        config: { branches: [], wait_for_all: true, aggregation: undefined, kind: "executable" },
      };
    case "loop":
      return {
        ...baseNode,
        type: "loop",
        config: {
          loop_type: "forEach",
          max_iterations: 100,
          continue_on_error: false,
          body_steps: [],
        },
      };
    case "merge":
      return {
        ...baseNode,
        type: "merge",
        config: { merge_type: "all", inputs: [] },
      };
    case "delay":
      return {
        ...baseNode,
        type: "delay",
        config: { delay_type: "seconds", seconds: 5 },
      };
    case "tool":
      return {
        ...baseNode,
        type: "tool",
        config: { tool_name: "", input_mapping: {}, output_var: "" },
      };
    case "code":
      return {
        ...baseNode,
        type: "code",
        config: { language: "javascript", code: "", output_var: "" },
      };
    case "subWorkflow":
      return {
        ...baseNode,
        type: "subWorkflow",
        config: {
          sub_workflow_id: "",
          input_mapping: {},
          output_var: "",
          is_async: false,
        },
      };
    case "workflowRef":
      return {
        ...baseNode,
        type: "workflowRef",
        config: {
          target_workflow_id: "",
          input_mapping: {},
          output_var: "",
          context_mode: "inherit",
        },
      };
    case "documentParser":
      return {
        ...baseNode,
        type: "documentParser",
        config: { input_var: "", parser_type: "", output_var: "" },
      };
    case "vectorRetrieve":
      return {
        ...baseNode,
        type: "vectorRetrieve",
        config: { query: "", knowledge_base_id: "", top_k: 5, output_var: "" },
      };
    case "end":
      return { ...baseNode, type: "end", config: {} };
    case "validation":
      return {
        ...baseNode,
        type: "validation",
        config: { assertions: [], on_fail: "stop" as const, max_retries: 0 },
      };
    case "httpRequest":
      return {
        ...baseNode,
        type: "httpRequest",
        config: {
          url: "",
          method: "GET",
          headers: {},
          body_type: "none",
          timeout_secs: 30,
          output_var: "",
        },
      };
    case "switch":
      return {
        ...baseNode,
        type: "switch",
        config: {
          input_var: "",
          cases: [],
          match_mode: "exact",
          output_var: "",
        },
      };
    case "databaseQuery":
      return {
        ...baseNode,
        type: "databaseQuery",
        config: {
          query: "",
          params: [],
          timeout_secs: 30,
          output_var: "",
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
          output_var: "",
        },
      };
    case "approval":
      return {
        ...baseNode,
        type: "approval",
        config: {
          message: "",
          timeout_secs: 3600,
          timeout_action: "reject",
          output_var: "",
        },
      };
    case "fileOperation":
      return {
        ...baseNode,
        type: "fileOperation",
        config: { operation: "read", file_path: "", output_var: "" },
      };
    case "dataTransformer":
      return {
        ...baseNode,
        type: "dataTransformer",
        config: { input_var: "", expression: "", output_var: "" },
      };
    case "webhookSend":
      return {
        ...baseNode,
        type: "webhookSend",
        config: {
          url: "",
          method: "POST",
          headers: {},
          output_var: "",
        },
      };
    case "logging":
      return {
        ...baseNode,
        type: "logging",
        config: { level: "info", message: "", output_var: "" },
      };
    case "llmClassifier":
      return {
        ...baseNode,
        type: "llmClassifier",
        config: {
          categories: [],
          prompt: "",
          input_var: "",
          output_var: "",
        },
      };
    case "aggregator":
      return {
        ...baseNode,
        type: "aggregator",
        config: { strategy: "concat", input_sources: [], output_var: "" },
      };
    case "email":
      return {
        ...baseNode,
        type: "email",
        config: {
          to: [],
          subject: "",
          body: "",
          output_var: "",
        },
      };
    case "debate":
      return {
        ...baseNode,
        type: "debate",
        config: {
          debater_steps: [],
          max_rounds: 3,
          topic_var: "",
          output_var: "",
        },
      };
    case "storage":
      return {
        ...baseNode,
        type: "storage",
        config: {
          backend: "sqlite",
          operation: "insert",
          input_var: "",
          collection: "",
          key_var: undefined,
          output_var: "",
        },
      };
    case "swarm":
      return {
        ...baseNode,
        type: "swarm",
        config: {
          agent_steps: [],
          max_rounds: 3,
          topic_var: "",
          output_var: "",
        },
      };
    default:
      console.warn(`[createWorkflowNode] Unknown node type "${type}", falling back to agent`);
      return {
        ...baseNode,
        type: "agent" as const,
        config: {},
      } as unknown as WorkflowNode;
  }
}
