import React, { useCallback, useEffect, useMemo } from 'react';
import { ReactFlow, Background, Controls, MiniMap, type Node, type Edge, useNodesState, useEdgesState, Connection, Panel, useReactFlow } from 'reactflow';
import 'reactflow/dist/style.css';
import { useTranslation } from 'react-i18next';
import { Spin, message } from 'antd';
import { useWorkflowEditorStore } from '@/stores';
import { EditorHeader } from './Header/EditorHeader';
import { LeftPanel } from './Panels/LeftPanel';
import { RightPanel } from './Panels/RightPanel';
import { StatusBar } from './StatusBar/EditorStatusBar';
import {
  BaseNode,
  TriggerNode,
  AgentNode,
  LLMNode,
  ConditionNode,
  ParallelNode,
  LoopNode,
  MergeNode,
  DelayNode,
  ToolNode,
  CodeNode,
  AtomicSkillNode,
  SubWorkflowNode,
  DocumentParserNode,
  VectorRetrieveNode,
  EndNode,
} from './Nodes';
import { BaseEdge } from './Edges/BaseEdge';
import { NODE_TYPE_MAP, type WorkflowNode, type WorkflowEdge, type AtomicSkillInfo } from './types';
import { SkillUpgradeModal } from './SkillUpgradeModal';

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
  atomicSkill: AtomicSkillNode,
  subWorkflow: SubWorkflowNode,
  documentParser: DocumentParserNode,
  vectorRetrieve: VectorRetrieveNode,
  end: EndNode,
};

const edgeTypes = {
  base: BaseEdge,
};

const defaultEdgeOptions = {
  type: 'base',
  animated: false,
};

interface WorkflowEditorProps {
  templateId?: string;
  onClose?: () => void;
}

export const WorkflowEditor: React.FC<WorkflowEditorProps> = ({ templateId, onClose }) => {
  const { t } = useTranslation('chat');
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
    addEdge: storeAddEdge,
    setSelectedNode,
    setSelectedEdge,
    selectedNodeId,
    selectedEdgeId,
    updateTemplate,
    createTemplate,
    validateTemplate,
    error,
    isDecompositionTemplate,
    checkSkillSemanticMatches,
    applySemanticAction,
    semanticCheckResult,
  } = useWorkflowEditorStore();

  const [reactFlowNodes, setRNodes, onNodesChange] = useNodesState([]);
  const [reactFlowEdges, setREdges, onEdgesChange] = useEdgesState([]);
  const [isInitialized, setIsInitialized] = React.useState(false);
  const [upgradeModalState, setUpgradeModalState] = React.useState<{
    visible: boolean;
    existingSkill: AtomicSkillInfo | null;
    generatedSkillName: string;
    generatedSkillDescription: string;
    nodeId: string;
  }>({
    visible: false,
    existingSkill: null,
    generatedSkillName: '',
    generatedSkillDescription: '',
    nodeId: '',
  });

  useEffect(() => {
    if (templateId) {
      loadTemplate(templateId);
    } else {
      initNewTemplate();
    }
  }, [templateId]);

  useEffect(() => {
    if (currentTemplate) {
      const flowNodes: Node[] = nodes.map((node: WorkflowNode) => {
        const typeInfo = NODE_TYPE_MAP[node.type] || { label: node.type, color: '#999' };
        const nodeType = NODE_TYPE_MAP[node.type] ? node.type : 'base';

        let semanticMatch = undefined;
        if (semanticCheckResult?.matches && node.type === 'atomicSkill') {
          const match = semanticCheckResult.matches.find((m) => m.node_id === node.id);
          if (match?.matches && match.matches.length > 0) {
            const bestMatch = match.matches[0];
            semanticMatch = {
              existing_skill_id: bestMatch.existing_skill.id,
              existing_skill_name: bestMatch.existing_skill.name,
              similarity_score: bestMatch.similarity_score,
              match_reasons: bestMatch.match_reasons,
            };
          }
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
            semanticMatch,
            onSemanticAction: applySemanticAction,
            onUpgradeRequest: handleUpgradeRequest,
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
        type: 'base',
        animated: edge.edge_type === 'loopBack',
        label: edge.label,
        data: { edgeType: edge.edge_type },
      }));
      setREdges(flowEdges);
      setIsInitialized(true);
    }
  }, [currentTemplate, nodes, edges]);

  useEffect(() => {
    if (isInitialized && isDecompositionTemplate && nodes.length > 0) {
      checkSkillSemanticMatches(nodes);
    }
  }, [isInitialized, isDecompositionTemplate, nodes, checkSkillSemanticMatches]);

  const handleUpgradeRequest = useCallback(
    (nodeId: string, existingSkillId: string, _existingSkillName: string, generatedSkillName: string, generatedSkillDescription: string) => {
      if (!semanticCheckResult) return;

      const match = semanticCheckResult.matches.find((m) => m.node_id === nodeId);
      if (!match || !match.matches || match.matches.length === 0) return;

      const bestMatch = match.matches.find((m) => m.existing_skill.id === existingSkillId);
      if (!bestMatch) return;

      setUpgradeModalState({
        visible: true,
        existingSkill: bestMatch.existing_skill,
        generatedSkillName,
        generatedSkillDescription,
        nodeId,
      });
    },
    [semanticCheckResult]
  );

  const handleUpgradeConfirm = useCallback(
    (suggestion: { name: string; description: string; input_schema: Record<string, unknown> | null; output_schema: Record<string, unknown> | null; reasoning: string }) => {
      const { nodeId, existingSkill } = upgradeModalState;
      if (!existingSkill) return;

      useWorkflowEditorStore.setState((state) => {
        const nodeIndex = state.nodes.findIndex((n) => n.id === nodeId);
        if (nodeIndex !== -1) {
          const node = state.nodes[nodeIndex] as any;
          node.config.skill_id = existingSkill.id;
          node.config.skill_name = suggestion.name;
          node.config.entry_ref = existingSkill.entry_ref;
          node.config.entry_type = existingSkill.entry_type;
          node.config.category = existingSkill.category;
          node.title = suggestion.name;
          node.description = suggestion.description;
          node.data.skillId = existingSkill.id;
          node.data.skillName = suggestion.name;
        }

        state.pendingReplacements.set(nodeId, {
          existingSkillId: existingSkill.id,
          action: 'upgrade_existing',
        });

        const remainingMatches = state.semanticCheckResult?.matches.filter((m) => m.node_id !== nodeId) || [];
        if (remainingMatches.length === 0) {
          state.semanticCheckResult = null;
        }
      });

      setUpgradeModalState((prev) => ({ ...prev, visible: false }));
      message.success(t('workflow.semanticCheckApplied'));
    },
    [upgradeModalState, t]
  );

  const onConnect = useCallback(
    (params: Connection) => {
      if (params.source && params.target) {
        const newEdge: WorkflowEdge = {
          id: `edge-${Date.now()}`,
          source: params.source,
          sourceHandle: params.sourceHandle ?? undefined,
          target: params.target,
          targetHandle: params.targetHandle ?? undefined,
          edge_type: 'direct',
        };
        storeAddEdge(newEdge);
      }
    },
    [storeAddEdge]
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNode(node.id);
    },
    [setSelectedNode]
  );

  const onEdgeClick = useCallback(
    (_: React.MouseEvent, edge: Edge) => {
      setSelectedEdge(edge.id);
    },
    [setSelectedEdge]
  );

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
  }, [setSelectedNode, setSelectedEdge]);

  const reactFlowInstance = useReactFlow();

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const data = event.dataTransfer.getData('application/reactflow');
      if (!data) return;

      try {
        const { type: nodeType } = JSON.parse(data);
        const typeInfo = NODE_TYPE_MAP[nodeType] || { label: nodeType, color: '#999' };

        const position = reactFlowInstance.screenToFlowPosition({
          x: event.clientX,
          y: event.clientY,
        });

        const id = `node-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
        const actualNodeType = NODE_TYPE_MAP[nodeType] ? nodeType : 'base';

        const newNode: Node = {
          id,
          type: actualNodeType,
          position,
          data: {
            id,
            type: nodeType,
            title: `新建 ${typeInfo.label}`,
            description: '',
            color: typeInfo.color,
            nodeType: nodeType,
            enabled: true,
            ...getDefaultNodeConfig(nodeType),
          },
        };

        setRNodes((nds) => [...nds, newNode]);

        const workflowNode = createWorkflowNode(id, nodeType, position, `新建 ${typeInfo.label}`);
        useWorkflowEditorStore.getState().addNode(workflowNode);
      } catch (error) {
        console.error('Failed to drop node:', error);
      }
    },
    [reactFlowInstance, setRNodes]
  );

  const handleNodesChange = useCallback(
    (changes: any) => {
      onNodesChange(changes);

      changes.forEach((change: any) => {
        if (change.type === 'position' && change.position && currentTemplate) {
          const nodeId = change.id;
          const position = change.position;
          updateNode(nodeId, { position } as Partial<WorkflowNode>);
        }
        if (change.type === 'remove' && change.id) {
          deleteNode(change.id);
        }
      });
    },
    [onNodesChange, currentTemplate, updateNode, deleteNode]
  );

  const handleSave = useCallback(async () => {
    if (!currentTemplate) return;

    const { isDecompositionTemplate, saveDecompositionWorkflow: saveDecomposition } = useWorkflowEditorStore();

    if (isDecompositionTemplate) {
      try {
        await saveDecomposition(currentTemplate.name, currentTemplate.description);
        message.success(t('workflow.decompositionSaved'));
        onClose?.();
      } catch (e) {
        message.error(String(e));
      }
      return;
    }

    const validation = await validateTemplate();
    if (validation && !validation.is_valid) {
      message.error(t('workflow.validationFailed', { count: validation.errors.length }));
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
      await updateTemplate(currentTemplate.id, input);
    } else {
      const newId = await createTemplate(input);
      if (newId) {
        useWorkflowEditorStore.setState((state) => {
          if (state.currentTemplate) {
            state.currentTemplate.id = newId;
          }
        });
        message.success(t('workflow.saved'));
      }
    }
  }, [currentTemplate, nodes, edges, createTemplate, updateTemplate, validateTemplate, t, onClose]);

  const selectedNode = useMemo(() => {
    if (!selectedNodeId) return null;
    return nodes.find((n) => n.id === selectedNodeId) || null;
  }, [selectedNodeId, nodes]);

  const selectedEdge = useMemo(() => {
    if (!selectedEdgeId) return null;
    return edges.find((e) => e.id === selectedEdgeId) || null;
  }, [selectedEdgeId, edges]);

  if (isLoading) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: '#1a1a1a' }}>
      <EditorHeader
        templateName={currentTemplate?.name || '新建工作流'}
        isDirty={isDirty}
        isSaving={isSaving}
        onSave={handleSave}
        onClose={onClose}
      />

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <LeftPanel />

        <div style={{ flex: 1, position: 'relative' }}>
          {isInitialized && (
            <ReactFlow
              nodes={reactFlowNodes}
              edges={reactFlowEdges}
              onNodesChange={handleNodesChange}
              onEdgesChange={onEdgesChange}
              onConnect={onConnect}
              onNodeClick={onNodeClick}
              onEdgeClick={onEdgeClick}
              onPaneClick={onPaneClick}
              onDragOver={onDragOver}
              onDrop={onDrop}
              nodeTypes={nodeTypes}
              edgeTypes={edgeTypes}
              defaultEdgeOptions={defaultEdgeOptions}
              fitView
              snapToGrid
              snapGrid={[16, 16]}
            >
              <Background color="#333" gap={16} />
              <Controls />
              <MiniMap
                nodeColor={(node: Node) => (node.data as any)?.color || '#999'}
                maskColor="rgba(0, 0, 0, 0.8)"
              />
              {nodes.length === 0 && (
                <Panel position="top-center" style={{ textAlign: 'center', color: '#666' }}>
                  从左侧面板拖拽节点以开始构建工作流
                </Panel>
              )}
            </ReactFlow>
          )}
        </div>

        <RightPanel selectedNode={selectedNode} selectedEdge={selectedEdge} />
      </div>

      <StatusBar
        nodeCount={nodes.length}
        edgeCount={edges.length}
        validationResult={validationResult}
        isDirty={isDirty}
      />

      {error && (
        <div style={{ position: 'fixed', bottom: 60, left: '50%', transform: 'translateX(-50%)', color: 'red' }}>
          {error}
        </div>
      )}

      {upgradeModalState.visible && upgradeModalState.existingSkill && (
        <SkillUpgradeModal
          open={upgradeModalState.visible}
          onClose={() => setUpgradeModalState((prev) => ({ ...prev, visible: false }))}
          existingSkill={upgradeModalState.existingSkill}
          generatedSkillName={upgradeModalState.generatedSkillName}
          generatedSkillDescription={upgradeModalState.generatedSkillDescription}
          onConfirm={handleUpgradeConfirm}
        />
      )}
    </div>
  );
};

function getDefaultNodeConfig(nodeType: string): Record<string, unknown> {
  switch (nodeType) {
    case 'trigger':
      return { triggerConfig: { type: 'manual', config: {} } };
    case 'agent':
      return { agentRole: 'developer', systemPrompt: '', tools: [], contextSources: [], outputMode: 'text' };
    case 'llm':
      return { model: '', prompt: '', temperature: 0.7, maxTokens: 2048 };
    case 'condition':
      return { conditions: [], logicalOp: 'and' };
    case 'parallel':
      return { branches: [], waitForAll: true };
    case 'loop':
      return { loopType: 'forEach', maxIterations: 100, continueOnError: false, bodySteps: [] };
    case 'tool':
      return { toolName: '', inputMapping: {}, outputVar: '' };
    case 'code':
      return { language: 'javascript', code: '', outputVar: '' };
    case 'atomicSkill':
      return { skillId: '', skillName: '', entryType: 'builtin', inputMapping: {}, outputVar: '' };
    case 'end':
      return { outputVar: '' };
    default:
      return {};
  }
}

function createWorkflowNode(id: string, type: string, position: { x: number; y: number }, title: string): WorkflowNode {
  const baseNode = {
    id,
    title,
    description: '',
    position,
    retry: { enabled: false, max_retries: 3, backoff_type: 'Exponential' as const, base_delay_ms: 1000, max_delay_ms: 60000 },
    timeout: undefined,
    enabled: true,
  };

  switch (type) {
    case 'trigger':
      return { ...baseNode, type: 'trigger', config: { type: 'manual', config: {} } };
    case 'agent':
      return { ...baseNode, type: 'agent', config: { role: 'developer', system_prompt: '', context_sources: [], output_var: '', tools: [], output_mode: 'text' } };
    case 'llm':
      return { ...baseNode, type: 'llm', config: { model: '', prompt: '', temperature: 0.7, max_tokens: 2048 } };
    case 'condition':
      return { ...baseNode, type: 'condition', config: { conditions: [], logical_op: 'and' } };
    case 'parallel':
      return { ...baseNode, type: 'parallel', config: { branches: [], wait_for_all: true } };
    case 'loop':
      return { ...baseNode, type: 'loop', config: { loop_type: 'forEach', max_iterations: 100, continue_on_error: false, body_steps: [] } };
    case 'merge':
      return { ...baseNode, type: 'merge', config: { merge_type: 'all', inputs: [] } };
    case 'delay':
      return { ...baseNode, type: 'delay', config: { delay_type: 'seconds', seconds: 5 } };
    case 'tool':
      return { ...baseNode, type: 'tool', config: { tool_name: '', input_mapping: {}, output_var: '' } };
    case 'code':
      return { ...baseNode, type: 'code', config: { language: 'javascript', code: '', output_var: '' } };
    case 'subWorkflow':
      return { ...baseNode, type: 'subWorkflow', config: { sub_workflow_id: '', input_mapping: {}, output_var: '', is_async: false } };
    case 'documentParser':
      return { ...baseNode, type: 'documentParser', config: { input_var: '', parser_type: '', output_var: '' } };
    case 'vectorRetrieve':
      return { ...baseNode, type: 'vectorRetrieve', config: { query: '', knowledge_base_id: '', top_k: 5, output_var: '' } };
    case 'atomicSkill':
      return { ...baseNode, type: 'atomicSkill', config: { skillId: '', skillName: '', entryType: 'builtin', inputMapping: {}, outputVar: '' } } as unknown as WorkflowNode;
    case 'end':
      return { ...baseNode, type: 'end', config: {} };
    default:
      return { ...baseNode, type: 'agent', config: { role: 'developer', system_prompt: '', context_sources: [], output_var: '', tools: [], output_mode: 'text' } };
  }
}

export default WorkflowEditor;
