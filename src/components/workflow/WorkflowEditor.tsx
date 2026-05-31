import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  Background,
  Connection,
  Controls,
  type Edge,
  MiniMap,
  type Node,
  Panel,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "reactflow";
import "reactflow/dist/style.css";
import { autoLayoutWorkflow } from "@/lib/workflowLayout";
import { useAgentProfileStore, useWorkflowEditorStore } from "@/stores";
import { useExpertStore } from "@/stores/feature/expertStore";
import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { Button, message, Modal, Spin, theme } from "antd";
import { useTranslation } from "react-i18next";
import { AIPanel } from "./AIPanel/AIPanel";
import { DebugPanel } from "./DebugPanel";
import { clearDragPayload, getDragPayload } from "./dndState";
import { BaseEdge } from "./Edges/BaseEdge";
import { EditorHeader } from "./Header/EditorHeader";
import {
  AgentNode,
  BaseNode,
  type BaseNodeData,
  CodeNode,
  ConditionNode,
  DelayNode,
  DocumentParserNode,
  EndNode,
  LLMNode,
  LoopNode,
  MergeNode,
  ParallelNode,
  SubWorkflowNode,
  ToolNode,
  TriggerNode,
  ValidationNode,
  VectorRetrieveNode,
} from "./Nodes";
import { LeftPanel } from "./Panels/LeftPanel";
import { RightPanel } from "./Panels/RightPanel";
import { SemanticCheckModal } from "./SemanticCheckModal";
import { StatusBar } from "./StatusBar/EditorStatusBar";
import { ImportExportModal } from "./Templates/ImportExportModal";
import { type AgentNode as AgentNodeType, NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "./types";

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
  documentParser: DocumentParserNode,
  vectorRetrieve: VectorRetrieveNode,
  validation: ValidationNode,
  end: EndNode,
};

const edgeTypes = {
  base: BaseEdge,
};

const defaultEdgeOptions = {
  type: "base",
  animated: false,
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
  } = useWorkflowEditorStore();

  const [reactFlowNodes, setRNodes, onNodesChange] = useNodesState([]);
  const [reactFlowEdges, setREdges, onEdgesChange] = useEdgesState([]);
  const [isInitialized, setIsInitialized] = React.useState(false);
  const hasAutoLaidOutRef = React.useRef(false);
  const autoLayoutTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const clipboardRef = React.useRef<WorkflowNode[]>([]);
  const edgesRef = React.useRef(edges);
  edgesRef.current = edges;
  // 拖拽时的位置批处理：RAF 合并多次像素级位置变更，只写最后一次到 store
  const pendingPositionsRef = React.useRef<Map<string, { x: number; y: number }>>(new Map());
  const posRafRef = React.useRef<number | null>(null);
  const [aiPanelVisible, setAiPanelVisible] = useState(false);
  const [aiPanelHeight, setAiPanelHeight] = useState(300);
  const [debugPanelVisible, setDebugPanelVisible] = useState(false);
  const [importExportModalVisible, setImportExportModalVisible] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; nodeId: string } | null>(null);
  const [searchVisible, setSearchVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchIdx, setSearchIdx] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [leftPanelCollapsed, setLeftPanelCollapsed] = useState(
    () => localStorage.getItem("workflowEditor.leftPanelCollapsed") === "true",
  );
  const [rightPanelCollapsed, setRightPanelCollapsed] = useState(
    () => localStorage.getItem("workflowEditor.rightPanelCollapsed") === "true",
  );

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
    applyAiChatAction,
    exportTemplate,
    importTemplate,
    loadTemplates,
    templates,
  } = useWorkflowEditorStore();

  useEffect(() => {
    hasAutoLaidOutRef.current = false;
    if (templateId) {
      loadTemplate(templateId);
    } else {
      initNewTemplate();
    }
  }, [templateId]);

  useEffect(() => {
    if (!isDirty || isSaving || isDecompositionTemplate) {
      return;
    }

    autoSaveTimerRef.current = setTimeout(async () => {
      const input = {
        name: currentTemplate?.name || "Unnamed Workflow",
        description: currentTemplate?.description,
        icon: currentTemplate?.icon || "Bot",
        tags: currentTemplate?.tags || [],
        trigger_config: currentTemplate?.trigger_config,
        nodes,
        edges,
        input_schema: currentTemplate?.input_schema,
        output_schema: currentTemplate?.output_schema,
        variables: currentTemplate?.variables || [],
        error_config: currentTemplate?.error_config,
      };

      if (currentTemplate?.id) {
        await updateTemplate(currentTemplate.id, input);
      } else {
        const newId = await createTemplate(input);
        if (newId) {
          await loadTemplate(newId);
        }
      }
    }, 5000);

    return () => {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current);
        autoSaveTimerRef.current = null as unknown as NodeJS.Timeout;
      }
    };
  }, [
    isDirty,
    nodes,
    edges,
    currentTemplate,
    isSaving,
    isDecompositionTemplate,
    updateTemplate,
    createTemplate,
    loadTemplate,
  ]);

  useEffect(() => {
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

        return {
          id: node.id,
          type: nodeType,
          position: node.position,
          data: {
            ...node,
            label: node.title,
            color: typeInfo.color,
            nodeType: node.type,
            ...(validationState ? { validationState } : {}),
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
      setRNodes(flowNodes);

      const flowEdges: Edge[] = edges.map((edge: WorkflowEdge) => ({
        id: edge.id,
        source: edge.source,
        sourceHandle: edge.sourceHandle,
        target: edge.target,
        targetHandle: edge.targetHandle,
        type: "base",
        animated: edge.edge_type === "loopBack",
        label: edge.label,
        data: { edgeType: edge.edge_type },
      }));
      setREdges(flowEdges);
      setIsInitialized(true);

      // 首次加载时若节点全部堆叠在原点附近（未布局过），自动排列
      if (
        !hasAutoLaidOutRef.current
        && nodes.length >= 2
        && nodes.every((n) => n.position.x < 50 && n.position.y < 50)
      ) {
        hasAutoLaidOutRef.current = true;
        autoLayoutTimerRef.current = setTimeout(() => {
          const { nodes: layouted, edges: layoutedE } = autoLayoutWorkflow(
            flowNodes,
            flowEdges,
          );
          setRNodes(layouted);
          setREdges(layoutedE);
          for (const ln of layouted) {
            updateNode(ln.id, { position: ln.position } as Partial<WorkflowNode>);
          }
        }, 100);
      }
      return () => {
        if (autoLayoutTimerRef.current) {
          clearTimeout(autoLayoutTimerRef.current);
          autoLayoutTimerRef.current = null;
        }
      };
    }
  }, [currentTemplate, nodes, edges, validationResult]);

  const onConnect = useCallback(
    (params: Connection) => {
      if (!params.source || !params.target) { return; }
      // 禁止自循环
      if (params.source === params.target) {
        message.warning(t("workflow.selfLoopNotAllowed"));
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
      // Determine edge type based on sourceHandle
      let edgeType: WorkflowEdge["edge_type"] = "direct";
      const sourceHandle = params.sourceHandle;
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
    [storeAddEdge],
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNode(node.id);
    },
    [setSelectedNode],
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

        const id = `node-${crypto.randomUUID()}`;
        const actualNodeType = NODE_TYPE_MAP[payload.type]
          ? payload.type
          : "base";

        const newNode: Node = {
          id,
          type: actualNodeType,
          position,
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
          position,
          t("workflow.newNode", {
            type: typeInfo.labelKey ? t(typeInfo.labelKey) : payload.type,
          }),
        );
        useWorkflowEditorStore.getState().addNode(workflowNode);
      } catch (error) {
        message.error(t("workflow.nodeDropFailed", { error: String(error) }));
      } finally {
        clearDragPayload();
      }
    };

    window.addEventListener("mouseup", handleGlobalMouseUp);
    return () => window.removeEventListener("mouseup", handleGlobalMouseUp);
  }, [reactFlowInstance, setRNodes]);

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
    clipboardRef,
  });
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
    clipboardRef,
  };
  const handleSaveRef = React.useRef(handleSave);
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
        r.canUndo() ? r.undo() : message.info(t("workflow.noUndoAvailable"));
        return;
      }
      if ((isCtrlOrCmd && e.key === "z" && e.shiftKey) || (isCtrlOrCmd && e.key === "y")) {
        e.preventDefault();
        r.canRedo() ? r.redo() : message.info(t("workflow.noRedoAvailable"));
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
          r.addNode({
            ...node,
            id: `node-${crypto.randomUUID()}`,
            position: { x: node.position.x + offset.x, y: node.position.y + offset.y },
          });
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
  const autoSaveTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => () => {
    if (autoSaveTimerRef.current) { clearTimeout(autoSaveTimerRef.current); }
  }, []);

  const handleNodesChange = useCallback(
    (changes: any) => {
      onNodesChange(changes);

      changes.forEach((change: any) => {
        if (change.type === "position" && change.position && currentTemplate) {
          // RAF 批处理：同一次拖拽中只保留最终位置
          pendingPositionsRef.current.set(change.id, change.position);
          if (posRafRef.current == null) {
            posRafRef.current = requestAnimationFrame(() => {
              pendingPositionsRef.current.forEach((pos, nodeId) => {
                updateNode(nodeId, { position: pos } as Partial<WorkflowNode>);
              });
              pendingPositionsRef.current.clear();
              posRafRef.current = null;
            });
          }
        }
        if (change.type === "remove" && change.id) {
          deleteNode(change.id);
        }
      });
    },
    [onNodesChange, currentTemplate, updateNode, deleteNode],
  );

  const handleEdgesChange = useCallback(
    (changes: any) => {
      onEdgesChange(changes);

      changes.forEach((change: any) => {
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
    const { nodes: layoutedNodes, edges: layoutedEdges } = autoLayoutWorkflow(
      reactFlowNodes,
      reactFlowEdges,
    );
    setRNodes(layoutedNodes);
    setREdges(layoutedEdges);

    // 将布局后的位置回写到 store
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
          const tmpl: any = await invoke("get_workflow_template", { id: subId });
          if (!tmpl?.nodes || !Array.isArray(tmpl.nodes)) { continue; }
          const subNodes = tmpl.nodes;
          const subEdges = tmpl.edges || [];
          // 转换为 ReactFlow 格式
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
          const { nodes: subLayouted } = autoLayoutWorkflow(rfSubNodes, rfSubEdges);
          // 回写位置
          const updatedSubNodes = subNodes.map((n: any) => {
            const nodeId = n.id || n.base?.id || "";
            const laid = subLayouted.find((ln) => ln.id === nodeId);
            if (!laid) { return n; }
            if (n.base) {
              return { ...n, base: { ...n.base, position: laid.position } };
            }
            return { ...n, position: laid.position };
          });
          // 用完整模板数据调用 update
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
          t("workflow.autoLayoutWithSubs", { count: subCount })
            || `${t("workflow.autoLayout")}（含 ${subCount} 个子工作流）`,
        );
        return;
      }
    }

    message.success(t("workflow.autoLayout"));
  }, [reactFlowNodes, reactFlowEdges, setRNodes, setREdges, updateNode, t]);

  const handleClose = useCallback(() => {
    if (isDirty) {
      Modal.confirm({
        title: t("wiki.unsavedTitle"),
        content: t("wiki.unsavedContent"),
        okText: t("wiki.discard"),
        cancelText: t("wiki.keepEditing"),
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
        canUndo={canUndo()}
        canRedo={canRedo()}
        aiPanelVisible={aiPanelVisible}
        debugPanelVisible={debugPanelVisible}
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
          <span style={{ fontSize: 11, color: token.colorTextQuaternary }}>
            {searchResults.length > 0 ? `${searchIdx + 1}/${searchResults.length}` : "0"}
          </span>
          <Button size="small" onClick={() => navigateSearch(-1)} disabled={searchResults.length === 0}>▲</Button>
          <Button size="small" onClick={() => navigateSearch(1)} disabled={searchResults.length === 0}>▼</Button>
          <Button size="small" onClick={() => setSearchVisible(false)}>✕</Button>
        </div>
      )}

      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        {!leftPanelCollapsed && <LeftPanel />}

        <div style={{ flex: 1, position: "relative" }}>
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
                nodeTypes={nodeTypes}
                edgeTypes={edgeTypes}
                defaultEdgeOptions={defaultEdgeOptions}
                fitView
                snapToGrid
                snapGrid={[16, 16]}
              >
                <Background color={token.colorBorderSecondary} gap={16} />
                <Controls />
                <MiniMap
                  nodeColor={(node: Node<BaseNodeData>) => node.data?.color || token.colorTextQuaternary}
                  maskColor={token.colorBgMask}
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

        {!rightPanelCollapsed && <RightPanel selectedNodeId={selectedNodeId} selectedEdge={selectedEdge} />}
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
                ? (nodes.find(n => n.id === selectedNodeId) as any)?.config?.system_prompt ?? null
                : null}
              onApplyPromptToNode={applyOptimizedPromptToNode}
              chatMessages={aiChatMessages}
              chatStreaming={aiChatStreaming}
              onChatSend={aiChatSend}
              onChatCancel={aiChatCancel}
              onChatClear={aiChatClear}
              onApplyAction={applyAiChatAction}
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

      {/* Context menu */}
      {contextMenu && (
        <div
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
              style={{
                padding: "6px 10px",
                fontSize: 12,
                cursor: "pointer",
                borderRadius: 4,
                color: action === "deleteNode" ? token.colorError : undefined,
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = token.colorFillQuaternary)}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
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
              {action === "edit" ? "✏️" : action === "toggleBreakpoint" ? "🔴" : action === "copyNode" ? "📋" : "🗑"}
              {" "}
              {t(`workflow.${action}`)}
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
      return { branches: [], wait_for_all: true, aggregation: undefined };
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
    case "documentParser":
      return { input_var: "", parser_type: "", output_var: "" };
    case "vectorRetrieve":
      return { query: "", knowledge_base_id: "", top_k: 5, output_var: "" };
    case "end":
      return {};
    case "validation":
      return { assertions: [], on_fail: "stop" as const, max_retries: 0 };
    default:
      return {};
  }
}

function createWorkflowNode(
  id: string,
  type: string,
  position: { x: number; y: number },
  title: string,
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
        config: { branches: [], wait_for_all: true, aggregation: undefined },
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
    default:
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
        },
      };
  }
}
