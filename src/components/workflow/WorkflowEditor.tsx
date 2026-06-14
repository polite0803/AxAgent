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
import { isTauri } from "@/lib/invoke";
import {
  auto_layout,
  autoLayoutWorkflow,
  type AutoNode,
  find_safe_position,
  getNodeSize,
  validate_workflow,
  type ValidateIssue,
  would_create_cycle,
} from "@/lib/workflowLayout";
import { useAgentProfileStore, useWorkflowEditorStore } from "@/stores";
import { useExpertStore } from "@/stores/feature/expertStore";
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
} from "./Nodes";
import { BatchEditPanel } from "./Panels/BatchEditPanel";
import { LeftPanel } from "./Panels/LeftPanel";
import { RightPanel } from "./Panels/RightPanel";
import { SemanticCheckModal } from "./SemanticCheckModal";
import { StatusBar } from "./StatusBar/EditorStatusBar";
import { ImportExportModal } from "./Templates/ImportExportModal";
import { VersionHistoryModal } from "./Templates/VersionHistoryModal";
import { type AgentNode as AgentNodeType, NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "./types";
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
  workflowRef: BaseNode,
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
  // dragStop 后短暂抑制 useEffect 全量重建，避免覆盖 reactFlowInstance.setNodes 的结果
  const suppressRebuildRef = React.useRef(false);
  // 跳过写入标志：程序化 setRNodes（如 autoLayout）后抑制 onNodesChange 中的重复 updateNode
  const skipPositionWriteRef = React.useRef(false);
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

  // 前端校验：nodes / edges 变更时自动重新运行 validate_workflow
  useEffect(() => {
    const issues = validate_workflow(nodes, edges, t);
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setFrontendValidation(issues.issues);
    const msgMap = new Map<string, string>();
    for (const iss of issues.issues) {
      for (const nid of iss.nodeIds) {
        const prev = msgMap.get(nid);
        msgMap.set(nid, prev ? `${prev}; ${iss.message}` : iss.message);
      }
    }
    setValidationMsgMap(msgMap);
  }, [nodes, edges, t]);

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
  // Auto-save: 通过 useRef 避免每次 nodes/edges 引用变化重建 timer，
  // 回调内通过 useWorkflowEditorStore.getState() 读取最新 store 数据。
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

      if (currentTemplate?.id) {
        await state.updateTemplate(currentTemplate.id, input);
      } else {
        const newId = await state.createTemplate(input);
        if (newId) {
          await state.loadTemplate(newId);
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
    // 拖拽期间或 dragStop 后短暂窗口内跳过全量重建
    if (isDraggingRef.current || suppressRebuildRef.current) { return; }
    if (currentTemplate) {
      const errorNodeIds = new Set<string>();
      const warningNodeIds = new Set<string>();
      if (validationResult) {
        validationResult.errors.forEach((e) => {
          if (e.node_id) {
            errorNodeIds.add(e.node_id);
          }
        });
        validationResult.warnings.forEach((w) => {
          if (w.node_id) {
            warningNodeIds.add(w.node_id);
          }
        });
      }
      // 合并前端校验结果
      for (const iss of frontendValidation) {
        for (const nid of iss.nodeIds) {
          if (iss.severity === "error") {
            errorNodeIds.add(nid);
          } else {
            warningNodeIds.add(nid);
          }
        }
      }

      // 预构建两张表
      //   childrenOfParent[pid] = [子节点 id 数组]
      //   nodeById[id] = 原始 WorkflowNode 引用（方便容器尺寸计算时查子节点）
      const childrenOfParent: Record<string, string[]> = {};
      const nodeById: Record<string, WorkflowNode> = {};
      for (const n of nodes) {
        nodeById[n.id] = n;
      }
      for (const [childId, pid] of Object.entries(parentRefs)) {
        if (!childrenOfParent[pid]) { childrenOfParent[pid] = []; }
        childrenOfParent[pid].push(childId);
      }

      const flowNodes: Node[] = nodes.map((node: WorkflowNode) => {
        const typeInfo = NODE_TYPE_MAP[node.type] || {
          labelKey: "",
          color: token.colorTextQuaternary,
        };
        const nodeType = NODE_TYPE_MAP[node.type] ? node.type : "base";

        let validationState: "error" | "warning" | undefined;
        if (errorNodeIds.has(node.id)) {
          validationState = "error";
        } else if (warningNodeIds.has(node.id)) {
          validationState = "warning";
        }

        const rtType = nodeType;
        const typeMeta = NODE_TYPE_MAP[rtType];
        const isContainer = typeMeta?.isContainer === true
          || (rtType === "subWorkflow" && useWorkflowEditorStore.getState().expandedSubWorkflows[node.id] != null);
        // 子图节点计数（优先从 config.subGraph.nodes 获取，否则回退到引用的子节点 ID 计数）
        const nodeConfig = (node as unknown as Record<string, unknown>).config as Record<string, unknown> | undefined;
        const subGraph = nodeConfig?.subGraph as Record<string, unknown> | undefined;
        const subGraphNodes = subGraph?.nodes;
        let subGraphChildCount = Array.isArray(subGraphNodes) ? (subGraphNodes as unknown[]).length : 0;
        if (subGraphChildCount === 0) {
          // 向后兼容：从分支/循环体/辩论者引用中统计子节点数
          if (node.type === "parallel") {
            const branches = nodeConfig?.branches as { steps?: unknown[] }[] | undefined;
            if (branches) {
              subGraphChildCount = branches.reduce((sum, b) => sum + (b.steps?.length ?? 0), 0);
            }
          } else if (node.type === "loop") {
            subGraphChildCount = (nodeConfig?.body_steps as string[] | undefined)?.length ?? 0;
          } else if (node.type === "debate") {
            subGraphChildCount = (nodeConfig?.debater_steps as string[] | undefined)?.length ?? 0;
          } else if (node.type === "swarm") {
            subGraphChildCount = (nodeConfig?.agent_steps as string[] | undefined)?.length ?? 0;
          } else if (node.type === "aggregator") {
            subGraphChildCount = (nodeConfig?.input_sources as string[] | undefined)?.length ?? 0;
          }
        }
        const isContainerCollapsed = isContainer
          && useWorkflowEditorStore.getState().collapsedContainers.has(node.id);
        // 折叠态：容器自身缩为紧凑尺寸
        const CONTAINER_PADDING = 40;
        const CONTAINER_HEADER_H = 60;
        const CONTAINER_MIN_W = 400;
        const CONTAINER_MIN_H = 200;
        let containerStyle: React.CSSProperties | undefined;
        if (isContainerCollapsed) {
          containerStyle = { width: 200, height: 60 };
        } else if (isContainer) {
          const childIds = childrenOfParent[node.id] ?? [];
          // 同时考虑子图（subGraph）节点对容器尺寸的影响
          const subGraphChildren = subGraphNodes ?? [];
          let maxX = CONTAINER_MIN_W - CONTAINER_PADDING;
          let maxY = CONTAINER_MIN_H - CONTAINER_PADDING;
          for (const childId of childIds) {
            const child = nodeById[childId];
            if (!child) { continue; }
            const sz = getNodeSize(child.type);
            // 子节点在 Store 中是绝对坐标 —— 转为相对容器的偏移
            const relX = child.position.x - node.position.x;
            const relY = child.position.y - node.position.y;
            const cx = relX + sz.width;
            const cy = relY + sz.height;
            if (cx > maxX) { maxX = cx; }
            if (cy > maxY) { maxY = cy; }
          }
          for (const sgChild of subGraphChildren as { type?: string; position: { x: number; y: number } }[]) {
            const sz = getNodeSize(sgChild.type ?? "base");
            // subGraph 中的 position 始终是相对容器的偏移（种子数据已转换为相对坐标，
            // 保存时也写回相对坐标），无需再减容器绝对坐标。
            const relX = sgChild.position.x;
            const relY = sgChild.position.y;
            const cx = relX + sz.width;
            const cy = relY + sz.height;
            if (cx > maxX) { maxX = cx; }
            if (cy > maxY) { maxY = cy; }
          }
          containerStyle = {
            width: Math.max(CONTAINER_MIN_W, maxX + CONTAINER_PADDING),
            height: Math.max(CONTAINER_MIN_H, maxY + CONTAINER_PADDING + CONTAINER_HEADER_H),
          };
        }
        // 折叠态下：容器内的子节点在画布上隐藏
        const parentId = parentRefs[node.id];
        const childIsHidden = parentId != null
          && useWorkflowEditorStore.getState().collapsedContainers.has(parentId);
        // 方案 A：全顶层节点，position 直接用 Store 中的绝对坐标
        return {
          id: node.id,
          type: rtType,
          position: node.position,
          ...(containerStyle ? { style: containerStyle } : {}),
          ...(isContainer ? { dragHandle: ".workflow-container-drag-handle", zIndex: 0 } : {}),
          ...(!isContainer ? { zIndex: 10 } : {}),
          ...(childIsHidden ? { hidden: true } : {}),
          data: {
            ...node,
            label: node.title,
            color: typeInfo.color,
            nodeType: node.type,
            ...(nodeConfig?.kind ? { kind: nodeConfig.kind as string } : {}),
            ...(isContainer ? { childCount: subGraphChildCount } : {}),
            ...((isContainer && containerStyle)
              ? {
                nodeWidth: (containerStyle as React.CSSProperties).width as number | undefined,
                nodeHeight: (containerStyle as React.CSSProperties).height as number | undefined,
              }
              : {}),
            // 容器特化字段提取：从 config 提取到 data 顶层供容器组件渲染使用
            ...(node.type === "debate"
              ? {
                debaterSteps: (nodeConfig?.debater_steps as string[])
                  ?? (subGraph?.nodes as unknown[] | undefined)?.map((n) => (n as Record<string, string>).id) ?? [],
                maxRounds: (nodeConfig?.max_rounds as number) ?? 2,
                convergencePrompt: nodeConfig?.convergence_prompt as string | undefined,
              }
              : {}),
            ...(node.type === "parallel"
              ? {
                branches: (nodeConfig?.branches as unknown[] | undefined)?.length
                  ?? (subGraph?.nodes as unknown[] | undefined)?.length ?? 0,
                waitStrategy: nodeConfig?.wait_for_all === false ? "any" as const : undefined,
                aggregation: nodeConfig?.aggregation as string | undefined,
                autoInputFromParent: nodeConfig?.auto_input_from_parent as boolean | undefined,
                hasBranchTimeout:
                  (nodeConfig?.branches as { branchTimeoutMs?: number; degradeStrategy?: string }[] | undefined)?.some(
                    (b) => b.branchTimeoutMs != null || (b.degradeStrategy && b.degradeStrategy !== "skip"),
                  ) ?? false,
              }
              : {}),
            ...(node.type === "loop"
              ? {
                loopType: nodeConfig?.loop_type as string | undefined,
                maxIterations: nodeConfig?.max_iterations as number | undefined,
                loopCondition: nodeConfig?.continue_condition as string | undefined,
                collectionVar: (nodeConfig?.iter_input_var ?? nodeConfig?.items_var) as string | undefined,
              }
              : {}),
            ...(node.type === "swarm"
              ? {
                agentSteps: (nodeConfig?.agent_steps as string[])
                  ?? (subGraph?.nodes as unknown[] | undefined)?.map((n) => (n as Record<string, string>).id) ?? [],
                maxRounds: (nodeConfig?.max_rounds as number) ?? 3,
              }
              : {}),
            ...(validationState ? { validationState } : {}),
            ...(validationState ? { validationMessage: validationMsgMap.get(node.id) || "" } : {}),
            ...(node.type === "agent" && (node as AgentNodeType).config
              ? {
                agentProfileId: (node as AgentNodeType).config.agentProfileId,
                systemPrompt: (node as AgentNodeType).config.system_prompt,
                tools: (node as AgentNodeType).config.tools,
                contextSources: (node as AgentNodeType).config
                  .context_sources,
                outputMode: (node as AgentNodeType).config.output_mode,
                model: (node as AgentNodeType).config.model,
                ...(function() {
                  const profileId = (node as AgentNodeType).config
                    .agentProfileId;
                  if (profileId) {
                    const profile = useExpertStore
                      .getState()
                      .getRoleById(profileId)
                      ?? useAgentProfileStore
                        .getState()
                        .getProfileById(profileId);
                    if (profile) {
                      return {
                        agentRole: profile.agentRole || undefined,
                        agentRoleIcon: profile.icon,
                        agentRoleDisplayName: profile.name,
                      };
                    }
                  }
                  return {};
                })(),
              }
              : {}),
          },
        };
      });
      // 将 ParallelNode 的 branches[].steps 和 MergeNode（auto-inputs）中的子节点挂载为容器子节点
      // parentId 权威来源是 store.parentRefs（持久化），其次才回退到本次回填期望值。
      const expectedParentByNode: Record<string, string> = {};
      // 折叠态：收集所有"应隐藏"的子节点 ID（含 branches.steps + merge auto_inputs）
      const hiddenChildIds = new Set<string>();
      for (const node of nodes) {
        const scopedNode = node as unknown as Record<string, unknown>;
        const scopedConfig = scopedNode.config as Record<string, unknown> | undefined;
        if (node.type === "parallel" && scopedConfig?.branches) {
          const branches = scopedConfig.branches as { steps?: string[] }[];
          for (const branch of branches) {
            for (const stepId of (branch.steps || []) as string[]) {
              const childIdx = flowNodes.findIndex((fn) => fn.id === stepId);
              if (childIdx === -1) { continue; }
              const storedParent = parentRefs[stepId];
              if (storedParent === undefined || storedParent === node.id) {
                const childFn = flowNodes[childIdx];
                const isCollapsedParent = collapsedContainers.has(node.id);
                if (isCollapsedParent) { hiddenChildIds.add(stepId); }
                flowNodes[childIdx] = {
                  ...childFn,
                  hidden: isCollapsedParent ? true : childFn.hidden,
                };
                expectedParentByNode[stepId] = node.id;
              }
            }
          }
        }
        // 将 MergeNode（auto-inputs）也挂入同一容器
        if (node.type === "merge" && scopedConfig?.auto_inputs_from_branches) {
          const inputs = scopedConfig?.inputs as string[] | undefined;
          if (inputs) {
            for (const inputId of inputs) {
              const childIdx = flowNodes.findIndex((fn) => fn.id === inputId);
              if (childIdx === -1) { continue; }
              const targetParent = parentRefs[inputId];
              if (!targetParent) { continue; }
              const storedMergeParent = parentRefs[node.id];
              if (storedMergeParent === undefined || storedMergeParent === targetParent) {
                const mergeIdx = flowNodes.findIndex((fn) => fn.id === node.id);
                if (mergeIdx === -1) { continue; }
                const mergeFn = flowNodes[mergeIdx];
                const isCollapsedParent = collapsedContainers.has(targetParent);
                if (isCollapsedParent) { hiddenChildIds.add(node.id); }
                flowNodes[mergeIdx] = {
                  ...mergeFn,
                  hidden: isCollapsedParent ? true : mergeFn.hidden,
                };
                expectedParentByNode[node.id] = targetParent;
              }
            }
          }
        }
        // DebateNode / LoopNode / SwarmNode / AggregatorNode 的子节点挂载：只注册 parentRefs，不动 position
        let stepIds: string[] | undefined;
        if (node.type === "debate") {
          stepIds = scopedConfig?.debater_steps as string[] | undefined;
        } else if (node.type === "loop") {
          stepIds = scopedConfig?.body_steps as string[] | undefined;
        } else if (node.type === "swarm") {
          stepIds = scopedConfig?.agent_steps as string[] | undefined;
        } else if (node.type === "aggregator") {
          stepIds = scopedConfig?.input_sources as string[] | undefined;
        }
        if (stepIds && stepIds.length > 0) {
          for (const stepId of stepIds) {
            const childIdx = flowNodes.findIndex((fn) => fn.id === stepId);
            if (childIdx === -1) { continue; }
            const storedParent = parentRefs[stepId];
            if (storedParent === undefined || storedParent === node.id) {
              const childFn = flowNodes[childIdx];
              const isCollapsedParent = collapsedContainers.has(node.id);
              if (isCollapsedParent) { hiddenChildIds.add(stepId); }
              flowNodes[childIdx] = {
                ...childFn,
                hidden: isCollapsedParent ? true : childFn.hidden,
              };
              expectedParentByNode[stepId] = node.id;
            }
          }
        }
      }

      // 把回填期望值持久化到 store：扫描结束后调 setParentRef，让 autosave 能保存到后端。
      // 仅在"登记值与期望不一致"时写入，避免每次渲染都推撤销栈（虽然 setParentRef 本身不进栈）。
      for (const [childId, expectedParent] of Object.entries(expectedParentByNode)) {
        if (parentRefs[childId] !== expectedParent) {
          setParentRef(childId, expectedParent);
        }
      }

      // ── 注入容器的子图（subGraph）内部节点 ──
      // subGraph.nodes 嵌入在容器 config 中，不在主 nodes[] 数组内。
      // 仅当容器未折叠时注入，折叠时由 ContainerNode 组件显示节点计数。
      for (const containerNode of nodes) {
        const typeMeta = NODE_TYPE_MAP[containerNode.type];
        if (!typeMeta?.isContainer) { continue; }
        const cnCfg = (containerNode as unknown as Record<string, unknown>).config as
          | Record<string, unknown>
          | undefined;
        const subGraph = cnCfg?.subGraph as
          | { nodes?: WorkflowNode[]; edges?: WorkflowEdge[] }
          | undefined;
        if (!subGraph?.nodes?.length) { continue; }
        const isCollapsedParent = collapsedContainers.has(containerNode.id);
        if (isCollapsedParent) { continue; } // 折叠态：子图节点隐藏，由 ContainerNode 显示计数

        for (const subNode of subGraph.nodes) {
          const existingIdx = flowNodes.findIndex((fn) => fn.id === subNode.id);
          const subTypeInfo = NODE_TYPE_MAP[subNode.type] || { color: token.colorTextQuaternary };
          const subData = {
            ...subNode,
            label: subNode.title,
            color: subTypeInfo.color,
            nodeType: subNode.type,
            enabled: true,
          };
          // 方案 B：挂载为容器子节点，使用子图相对坐标 + parentId
          // 子图 position 在种子中已转换为相对容器的偏移（adjust_positions_to_relative），
          // ReactFlow 在 extent:"parent" 模式下将相对偏移正确渲染在容器内。
          const subRelPos = {
            x: subNode.position.x,
            y: subNode.position.y,
          };
          const subFlowNode = {
            id: subNode.id,
            type: subNode.type || "agent",
            position: subRelPos,
            parentId: containerNode.id,
            extent: "parent" as const,
            data: subData,
          };

          if (existingIdx !== -1) {
            flowNodes[existingIdx] = {
              ...subFlowNode,
              data: {
                ...flowNodes[existingIdx].data,
                ...subData,
              },
            };
          } else {
            flowNodes.push(subFlowNode);
          }
        }

        // ── 注入子图内部边（subGraph.edges）──
        // 这些边仅在容器展开时可见，连接子图内部的节点。
        // 实际注入在 flowEdges 声明之后的 subGraphEdgeInjection 阶段完成。

        // ── subGraph 子节点 parentRefs 持久化（确保保存时不会丢失父子关系）──
        const cnCfg2 = (containerNode as unknown as Record<string, unknown>).config as
          | Record<string, unknown>
          | undefined;
        const subGraph2 = cnCfg2?.subGraph as
          | { nodes?: WorkflowNode[] }
          | undefined;
        if (subGraph2?.nodes?.length) {
          for (const subNode of subGraph2.nodes) {
            if (parentRefs[subNode.id] !== containerNode.id) {
              setParentRef(subNode.id, containerNode.id);
            }
          }
        }
      }

      // ── 注入展开的子工作流内部节点 ──
      const expandedSWData = useWorkflowEditorStore.getState().expandedSubWorkflows;
      for (const [swNodeId, swData] of Object.entries(expandedSWData)) {
        if (!swData || swData.isLoading || swData.nodes.length === 0) { continue; }
        const parentFn = flowNodes.find((fn) => fn.id === swNodeId);
        if (!parentFn) { continue; }

        for (const subNode of swData.nodes) {
          // subNode.position 相对父节点偏移 → 绝对坐标 = 父位置 + 相对偏移
          flowNodes.push({
            id: subNode.id,
            type: (subNode as unknown as { type?: string }).type || "agent",
            position: { x: parentFn.position.x + subNode.position.x, y: parentFn.position.y + subNode.position.y },
            data: {
              ...subNode,
              label: subNode.title,
              color: "#eb2f96",
              nodeType: subNode.type,
              enabled: true,
            },
          });
        }
      }

      // ── 节点排序：确保父容器在子节点之前（ReactFlow 要求 parentId 的父节点先于子节点）──
      flowNodes.sort((a, b) => {
        const aIsParent = a.parentId == null && flowNodes.some((n) => n.parentId === a.id);
        const bIsParent = b.parentId == null && flowNodes.some((n) => n.parentId === b.id);
        if (aIsParent && !bIsParent) { return -1; }
        if (!aIsParent && bIsParent) { return 1; }
        return 0;
      });

      setRNodes(flowNodes);

      // 折叠态边处理：将连接到隐藏子节点的边重定向到其父容器，内部边仍隐藏。
      const nodeHiddenMap = new Map<string, boolean>();
      // hidden child → 折叠中的父容器 ID
      const hiddenChildToParent = new Map<string, string>();
      for (const fn of flowNodes) {
        nodeHiddenMap.set(fn.id, fn.hidden === true);
        if (fn.hidden) {
          const pid = parentRefs[fn.id];
          if (pid && collapsedContainers.has(pid)) {
            hiddenChildToParent.set(fn.id, pid);
          }
        }
      }
      const seenEdgeKeys = new Set<string>();

      // 构建 parallel 子节点 → 端口号映射（动态分配，最多 6 端口均匀分布）
      const childPortMap = new Map<string, string>();
      for (const node of nodes) {
        if (node.type !== "parallel") { continue; }
        const nodeCfg = (node as unknown as Record<string, unknown>).config as
          | { branches?: Array<{ steps: string[] }> }
          | undefined;
        const cfg = nodeCfg;
        if (!cfg?.branches) { continue; }
        for (let bi = 0; bi < cfg.branches.length; bi++) {
          const portId = `port-${Math.min(bi, 5)}`;
          for (const stepId of cfg.branches[bi].steps || []) {
            // 只登记确认属于该容器的子节点
            if (parentRefs[stepId] === node.id) {
              childPortMap.set(stepId, portId);
            }
          }
        }
      }

      const flowEdges: Edge[] = [];
      for (const edge of edges as WorkflowEdge[]) {
        const remappedSource = hiddenChildToParent.get(edge.source) ?? edge.source;
        const remappedTarget = hiddenChildToParent.get(edge.target) ?? edge.target;
        const bothHidden = nodeHiddenMap.get(edge.source) === true
          && nodeHiddenMap.get(edge.target) === true;
        // 两端都是隐藏子节点（容器内部边） → 直接隐藏，不做重定向
        if (bothHidden) {
          flowEdges.push({
            id: edge.id,
            source: edge.source,
            sourceHandle: edge.sourceHandle,
            target: edge.target,
            targetHandle: edge.targetHandle,
            type: "base",
            animated: edge.edge_type === "loopBack",
            label: edge.label,
            data: { edgeType: edge.edge_type },
            hidden: true,
          });
          continue;
        }
        // 发生了重定向 → 去重（多条边折叠到同一个 source→target 对时只保留一条）
        const wasRemapped = remappedSource !== edge.source || remappedTarget !== edge.target;
        // 任一端是隐藏节点但不是容器折叠子节点（如展开的子工作流内部节点）→ 直接隐藏边
        const sourceIsOtherHidden = nodeHiddenMap.get(edge.source) === true && !hiddenChildToParent.has(edge.source);
        const targetIsOtherHidden = nodeHiddenMap.get(edge.target) === true && !hiddenChildToParent.has(edge.target);
        if (wasRemapped) {
          const key = `${remappedSource}→${remappedTarget}:${edge.edge_type}`;
          if (seenEdgeKeys.has(key)) { continue; }
          seenEdgeKeys.add(key);
        }
        flowEdges.push({
          id: edge.id,
          source: remappedSource,
          sourceHandle: wasRemapped && remappedSource !== edge.source
            ? undefined
            : (childPortMap.get(edge.source) ?? edge.sourceHandle),
          target: remappedTarget,
          targetHandle: wasRemapped && remappedTarget !== edge.target ? undefined : edge.targetHandle,
          type: "base",
          animated: edge.edge_type === "loopBack",
          label: edge.label,
          data: { edgeType: edge.edge_type },
          ...((sourceIsOtherHidden || targetIsOtherHidden) ? { hidden: true } : {}),
        });
      }

      // ── 注入子图内部边（subGraph.edges）──
      // 这些边仅在容器展开时可见，连接子图内部的节点。
      for (const containerNode of nodes) {
        const typeMeta = NODE_TYPE_MAP[containerNode.type];
        if (!typeMeta?.isContainer) { continue; }
        const cnCfg = (containerNode as unknown as Record<string, unknown>).config as
          | Record<string, unknown>
          | undefined;
        const subGraph = cnCfg?.subGraph as
          | { nodes?: WorkflowNode[]; edges?: WorkflowEdge[] }
          | undefined;
        if (!subGraph?.edges?.length) { continue; }
        const isCollapsedParent = collapsedContainers.has(containerNode.id);
        if (isCollapsedParent) { continue; }
        const subNodeIds = new Set((subGraph.nodes ?? []).map((n: { id: string }) => n.id));
        for (const subEdge of subGraph.edges) {
          if (!subNodeIds.has(subEdge.source) || !subNodeIds.has(subEdge.target)) { continue; }
          flowEdges.push({
            id: subEdge.id,
            source: subEdge.source,
            sourceHandle: subEdge.sourceHandle,
            target: subEdge.target,
            targetHandle: subEdge.targetHandle,
            type: "base",
            animated: subEdge.edge_type === "loopBack",
            label: subEdge.label,
            data: { edgeType: subEdge.edge_type },
          });
        }
      }

      setREdges(flowEdges);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setIsInitialized(true);

      // 首次加载时自动布局，确保节点排列合理
      if (!hasAutoLaidOutRef.current && nodes.length >= 2) {
        const hasReasonableLayout = nodes.some(
          (n) => n.position.x >= 50 || n.position.y >= 50,
        );
        // 如果有容器节点或子节点坐标明确，视为已有合理布局，跳过自动布局
        const hasContainerNodes = nodes.some((n) => NODE_TYPE_MAP[n.type]?.isContainer);
        const hasContainerChildren = nodes.some((n) => {
          const pid = parentRefs[n.id];
          return typeof pid === "string" && pid.length > 0 && (n.position.x !== 0 || n.position.y !== 0);
        });
        const skipAutoLayout = hasContainerNodes || hasContainerChildren;

        const hasOverlap = (() => {
          const posMap = new Map<string, number>();
          for (const n of nodes) {
            const key = `${Math.round(n.position.x / 10)},${Math.round(n.position.y / 10)}`;
            posMap.set(key, (posMap.get(key) || 0) + 1);
          }
          return Array.from(posMap.values()).some((count) => count > 1);
        })();

        if (!skipAutoLayout && (!hasReasonableLayout || hasOverlap)) {
          hasAutoLaidOutRef.current = true;
          autoLayoutTimerRef.current = setTimeout(() => {
            const { nodes: layouted, edges: layoutedE } = autoLayoutWorkflow(
              flowNodes,
              flowEdges,
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
                const parentLayouted = layouted.find((n) => n.id === pid);
                if (parentLayouted) {
                  updateNode(ln.id, {
                    position: {
                      x: ln.position.x + parentLayouted.position.x,
                      y: ln.position.y + parentLayouted.position.y,
                    },
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
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentTemplate, nodes, edges, validationResult, collapsedContainers, frontendValidation]);

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
      // 环检测：loopBack 边若会形成环则拒绝（普通直接边允许有向无环图外的边，
      // 但 loopBack 会在 rt-workflow 引擎中触发无限循环，必须前置拦截）。
      const sourceHandle = (params.sourceHandle ?? undefined) as
        | string
        | undefined;
      if (
        sourceHandle === "loopBack" && would_create_cycle(
          edgesRef.current.map((e) => ({ source: e.source, target: e.target })),
          params.source,
          params.target,
        )
      ) {
        message.warning(t("workflow.loopBackCycleDetected"));
        return;
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
          const size = getNodeSize(n.type);
          const nx = n.position.x;
          const ny = n.position.y;
          if (
            position.x >= nx
            && position.x <= nx + size.width
            && position.y >= ny
            && position.y <= ny + size.height
          ) {
            hitContainerId = n.id;
            break;
          }
        }

        const id = `node-${crypto.randomUUID()}`;
        const actualNodeType = NODE_TYPE_MAP[payload.type]
          ? payload.type
          : "base";

        // 方案 A：Store 存绝对坐标，ReactFlow 输出绝对坐标
        const storePosition = position;

        const newNode: Node = {
          id,
          type: actualNodeType,
          position: position,
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
          const size = getNodeSize(n.type);
          if (
            position.x >= n.position.x && position.x <= n.position.x + size.width
            && position.y >= n.position.y && position.y <= n.position.y + size.height
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

    const input = {
      name: currentTemplate.name,
      description: currentTemplate.description,
      icon: currentTemplate.icon,
      tags: currentTemplate.tags,
      trigger_config: currentTemplate.trigger_config,
      nodes,
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
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      nodes.forEach((node: any) => {
        const nodeType = (node.data?.type as string) || node.type || "";
        const fallback = NODE_TYPE_MAP[nodeType]?.isContainer
          ? getNodeSize(nodeType)
          : null;
        const w = node.measured?.width ?? fallback?.width ?? 200;
        const h = node.measured?.height ?? fallback?.height ?? 100;
        minX = Math.min(minX, node.position.x);
        minY = Math.min(minY, node.position.y);
        maxX = Math.max(maxX, node.position.x + w);
        maxY = Math.max(maxY, node.position.y + h);
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

  // 用 ref 保存频繁变化的值，避免键盘事件监听器每次渲染重建
  const keyRef = React.useRef({
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
  });
  // eslint-disable-next-line react-hooks/refs
  keyRef.current = {
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
  };
  const handleSaveRef = React.useRef(handleSave);
  // eslint-disable-next-line react-hooks/refs
  handleSaveRef.current = handleSave;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const r = keyRef.current;
      const isCtrlOrCmd = e.ctrlKey || e.metaKey;
      const isEditing = (e.target as HTMLElement).tagName === "INPUT"
        || (e.target as HTMLElement).tagName === "TEXTAREA"
        || (e.target as HTMLElement).isContentEditable;

      if (isCtrlOrCmd && e.key === "s") {
        e.preventDefault();
        handleSaveRef.current();
        return;
      }
      if (isCtrlOrCmd && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        if (r.canUndo()) { r.undo(); }
        else { message.info(t("workflow.noUndoAvailable")); }
        return;
      }
      if ((isCtrlOrCmd && e.key === "z" && e.shiftKey) || (isCtrlOrCmd && e.key === "y")) {
        e.preventDefault();
        if (r.canRedo()) { r.redo(); }
        else { message.info(t("workflow.noRedoAvailable")); }
        return;
      }
      if ((e.key === "Delete" || e.key === "Backspace") && r.selectedNodeId) {
        if (isEditing) { return; }
        e.preventDefault();
        r.deleteNode(r.selectedNodeId);
        r.setSelectedNode(null);
        return;
      }
      if (isCtrlOrCmd && e.key === "c" && r.selectedNodeId) {
        const nodeToCopy = r.nodes.find((n) => n.id === r.selectedNodeId);
        if (nodeToCopy) {
          r.clipboardRef.current = [nodeToCopy];
          message.success(t("workflow.nodeCopied"));
        }
        return;
      }
      if (isCtrlOrCmd && e.key === "v" && !isEditing) {
        if (r.clipboardRef.current.length === 0) { return; }
        const offset = { x: 50, y: 50 };
        r.clipboardRef.current.forEach((node) => {
          const newId = `node-${crypto.randomUUID()}`;
          r.addNode({
            ...node,
            id: newId,
            position: { x: node.position.x + offset.x, y: node.position.y + offset.y },
          });
          // 复制容器内子节点时同步维护 parentRefs 与父容器的 subGraph
          const originalParentId = useWorkflowEditorStore.getState().parentRefs[node.id];
          if (originalParentId) {
            r.setParentRef(newId, originalParentId);
            const parent = r.nodes.find((n) => n.id === originalParentId);
            if (parent) {
              const parentConfig = (parent as unknown as { config?: Record<string, unknown> }).config ?? {};
              const parentSubGraph = (parentConfig as { subGraph?: { nodes?: unknown[]; edges?: unknown[] } }).subGraph;
              r.updateNode(
                parent.id,
                {
                  ...(parent as object),
                  config: {
                    ...parentConfig,
                    subGraph: {
                      nodes: [
                        ...(parentSubGraph?.nodes ?? []),
                        {
                          ...node,
                          id: newId,
                          position: { x: node.position.x + offset.x, y: node.position.y + offset.y },
                        },
                      ],
                      edges: parentSubGraph?.edges ?? [],
                    },
                  },
                } as unknown as Parameters<typeof r.updateNode>[1],
              );
            }
          }
        });
        message.success(t("workflow.nodesPasted", { count: r.clipboardRef.current.length }));
        return;
      }
      // Ctrl+F: 节点搜索
      if (isCtrlOrCmd && e.key === "f" && !isEditing) {
        e.preventDefault();
        setSearchVisible(true);
        return;
      }
      // Ctrl+Shift+C: 全部折叠容器
      if (isCtrlOrCmd && e.shiftKey && e.key === "C" && !isEditing) {
        e.preventDefault();
        const containers = r.nodes.filter((n) => NODE_TYPE_MAP[n.type]?.isContainer);
        if (containers.length > 0) {
          useWorkflowEditorStore.getState().collapseAllContainers();
          message.success(t("workflow.containersCollapsed", { defaultValue: "All containers collapsed" }));
        }
        return;
      }
      // Ctrl+Shift+E: 全部展开容器
      if (isCtrlOrCmd && e.shiftKey && e.key === "E" && !isEditing) {
        e.preventDefault();
        useWorkflowEditorStore.getState().expandAllContainers();
        message.success(t("workflow.containersExpanded", { defaultValue: "All containers expanded" }));
        return;
      }
      // Ctrl+A: 仅拦截画布全选，输入框内放行
      if (isCtrlOrCmd && e.key === "a" && !isEditing) {
        e.preventDefault();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [t]);

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
    reactFlowInstance?.setCenter(target.position.x + 100, target.position.y + 50, { zoom: 1.5, duration: 300 });
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
      // 此处深拷贝 changes 中的 position，确保传给 ReactFlow 的都是可写的新对象。
      const clonedChanges = changes.map((c: any) => {
        if (c.type && c.position) {
          return { ...c, position: { ...c.position } };
        }
        return c;
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
          let storePos: { x: number; y: number };
          const rfNode = reactFlowInstance?.getNodes().find((n: any) => n.id === change.id);
          if (rfNode?.parentId) {
            const parent = reactFlowInstance?.getNodes().find((n: any) => n.id === rfNode.parentId);
            if (parent) {
              storePos = { x: change.position.x + parent.position.x, y: change.position.y + parent.position.y };
            } else {
              storePos = { x: change.position.x, y: change.position.y };
            }
          } else {
            storePos = { x: change.position.x, y: change.position.y };
          }
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
          deleteNode(change.id);
        }
      });
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
      requestAnimationFrame(() => {
        suppressRebuildRef.current = false;
      });

      if (node?.position) {
        // 碰撞避免：对 drag 主节点计算安全位置
        const rfNodes = reactFlowInstance?.getNodes() || [];
        const selectedIds = new Set(
          rfNodes.filter((n) => n.selected).map((n) => n.id),
        );
        // siblings = 所有非选中节点（同组拖拽节点整体移动，不互相排斥）
        const siblings = rfNodes
          .filter((n) => n.id !== node.id && !selectedIds.has(n.id))
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
          10,
        );

        // 从 store 实时读取最新节点状态，避免闭包中的 nodes 过时
        const latestNodes = useWorkflowEditorStore.getState().nodes;
        const latestParentRefs = useWorkflowEditorStore.getState().parentRefs;
        const oldNode = latestNodes.find((n) => n.id === node.id);
        const dx = oldNode ? safePos.x - oldNode.position.x : 0;
        const dy = oldNode ? safePos.y - oldNode.position.y : 0;
        updateNode(node.id, { position: safePos } as Partial<WorkflowNode>);

        // 被拖的是容器 → 同步位移差到所有子节点
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

        const updatedNodes = rfNodes.map((n) => {
          if (n.id === node.id) {
            return { ...n, position: safePos };
          }
          if (n.selected && n.id !== node.id && n.position) {
            updateNode(n.id, { position: n.position } as Partial<WorkflowNode>);
          }
          return n;
        });
        reactFlowInstance?.setNodes(updatedNodes);
      }
    },
    [updateNode, reactFlowInstance],
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
    // 保留 edges 不变
    skipPositionWriteRef.current = true;
    setRNodes(layoutedNodes);
    requestAnimationFrame(() => {
      skipPositionWriteRef.current = false;
    });

    // auto_layout 返回值：子节点 position = 相对父容器偏移，需转回绝对坐标
    for (const ln of layoutedNodes) {
      const pid = parentRefs[ln.id];
      if (pid) {
        const parentLayouted = layoutedNodes.find((n) => n.id === pid);
        if (parentLayouted) {
          updateNode(ln.id, {
            position: {
              x: ln.position.x + parentLayouted.position.x,
              y: ln.position.y + parentLayouted.position.y,
            },
          } as Partial<WorkflowNode>);
          continue;
        }
      }
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
                  clipboardRef.current = [nodes.find((n) => n.id === contextMenu.nodeId)!];
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
