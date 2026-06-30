// SPDX-License-Identifier: AGPL-3.0-only

import { type NodePositionLike, toRelativePosition } from "@/lib/workflowLayout";
import { getNodeSize } from "@/lib/workflowLayout";
import type { ValidateIssue } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { useAgentStore } from "@/stores/feature/agentStore";
import { useExpertStore } from "@/stores/feature/expertStore";
import { type Edge, type Node } from "@xyflow/react";
import { useMemo } from "react";
import { type AgentNode as AgentNodeType, NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "../types";

interface UseFlowNodesParams {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  parentRefs: Record<string, string>;
  collapsedContainers: Record<string, boolean>;
  validationResult: {
    errors: Array<{ node_id?: string }>;
    warnings: Array<{ node_id?: string }>;
  } | null;
  frontendValidation: ValidateIssue[];
  validationMsgMap: Map<string, string>;
  token: {
    colorTextQuaternary: string;
    colorError: string;
    colorWarning: string;
    colorPrimary: string;
  };
}

export function useFlowNodes(params: UseFlowNodesParams) {
  const {
    nodes,
    edges,
    parentRefs,
    collapsedContainers,
    validationResult,
    frontendValidation,
    validationMsgMap,
    token,
  } = params;

  const expandedSubWorkflows = useWorkflowEditorStore((s) => s.expandedSubWorkflows);

  return useMemo(() => {
    const errorNodeIds = new Set<string>();
    const warningNodeIds = new Set<string>();
    if (validationResult) {
      validationResult.errors.forEach((e) => {
        if (e.node_id) { errorNodeIds.add(e.node_id); }
      });
      validationResult.warnings.forEach((w) => {
        if (w.node_id) { warningNodeIds.add(w.node_id); }
      });
    }
    for (const iss of frontendValidation) {
      for (const nid of iss.nodeIds) {
        if (iss.severity === "error") {
          errorNodeIds.add(nid);
        } else {
          warningNodeIds.add(nid);
        }
      }
    }

    const childrenOfParent: Record<string, string[]> = {};
    const nodeById: Record<string, WorkflowNode> = {};
    for (const n of nodes) {
      nodeById[n.id] = n;
    }
    const expandedSWData = expandedSubWorkflows;
    // 标记来自 expandedSubWorkflows 的子节点 ID，这些节点的 position 已是相对坐标
    const subWorkflowChildIds = new Set<string>();
    for (const [, swData] of Object.entries(expandedSWData)) {
      if (!swData || swData.isLoading || !swData.nodes?.length) { continue; }
      for (const subNode of swData.nodes) {
        nodeById[subNode.id] = subNode;
        subWorkflowChildIds.add(subNode.id);
      }
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
        || (rtType === "subWorkflow" && expandedSubWorkflows[node.id] != null);
      const nodeConfig = (node as unknown as Record<string, unknown>).config as Record<string, unknown> | undefined;
      const subGraph = nodeConfig?.subGraph as Record<string, unknown> | undefined;
      const subGraphNodes = subGraph?.nodes;
      let subGraphChildCount = Array.isArray(subGraphNodes) ? (subGraphNodes as unknown[]).length : 0;
      if (subGraphChildCount === 0) {
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
        && collapsedContainers[node.id];
      // 修复1、2、7：与 workflowLayout.ts 保持同步的容器尺寸常量
      const CONTAINER_PADDING = 16;
      const CONTAINER_MIN_W = 240;
      const CONTAINER_MIN_H = 120;
      const CONTAINER_HEADER_H = 36;
      let containerStyle: React.CSSProperties | undefined;
      if (isContainerCollapsed) {
        containerStyle = { width: 160, height: 34 };
      } else if (isContainer) {
        const childIds = childrenOfParent[node.id] ?? [];
        const subGraphChildren = subGraphNodes ?? [];
        let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
        for (const childId of childIds) {
          const child = nodeById[childId];
          if (!child) { continue; }
          const sz = getNodeSize(child.type);
          // expandedSubWorkflows 的子节点 position 已是相对坐标（子工作流内部坐标）；
          // 普通子节点 store 存绝对坐标，需减去父容器绝对坐标转为相对坐标。
          const isSWChild = subWorkflowChildIds.has(childId);
          const relX = isSWChild ? child.position.x : child.position.x - node.position.x;
          const relY = isSWChild ? child.position.y : child.position.y - node.position.y;
          minX = Math.min(minX, relX);
          minY = Math.min(minY, relY);
          maxX = Math.max(maxX, relX + sz.width);
          maxY = Math.max(maxY, relY + sz.height);
        }
        for (const sgChild of subGraphChildren as { type?: string; position: { x: number; y: number } }[]) {
          const sz = getNodeSize(sgChild.type ?? "base");
          // subGraph.nodes 的 position 已是相对坐标（相对于容器），直接使用
          const relX = sgChild.position.x;
          const relY = sgChild.position.y;
          minX = Math.min(minX, relX);
          minY = Math.min(minY, relY);
          maxX = Math.max(maxX, relX + sz.width);
          maxY = Math.max(maxY, relY + sz.height);
        }
        if (minX === Infinity) {
          minX = 0;
          minY = 0;
          maxX = CONTAINER_MIN_W - CONTAINER_PADDING * 2;
          maxY = CONTAINER_MIN_H - CONTAINER_PADDING * 2 - CONTAINER_HEADER_H;
        }
        containerStyle = {
          width: Math.max(CONTAINER_MIN_W, maxX - minX + CONTAINER_PADDING * 2),
          height: Math.max(CONTAINER_MIN_H, maxY - minY + CONTAINER_PADDING * 2 + CONTAINER_HEADER_H),
        };
      }
      const pid = parentRefs[node.id];
      const childIsHidden = pid != null
        && collapsedContainers[pid];

      const relPos = toRelativePosition(node.id, node.position, parentRefs, nodes as NodePositionLike[]);

      return {
        id: node.id,
        type: rtType,
        position: relPos,
        ...(pid ? { parentId: pid, extent: "parent" as const } : {}),
        ...(containerStyle ? { style: containerStyle } : {}),
        ...(isContainer ? { dragHandle: ".workflow-container-drag-handle", zIndex: 0 } : {}),
        ...(!isContainer ? { zIndex: 10 } : {}),
        ...(childIsHidden ? { hidden: true } : {}),
        data: {
          ...node,
          label: node.title,
          color: typeInfo.color,
          nodeType: node.type,
          ...(pid ? { parentId: pid } : {}),
          ...(nodeConfig?.kind ? { kind: nodeConfig.kind as string } : {}),
          ...(isContainer ? { childCount: subGraphChildCount } : {}),
          ...((isContainer && containerStyle)
            ? {
              nodeWidth: (containerStyle as React.CSSProperties).width as number | undefined,
              nodeHeight: (containerStyle as React.CSSProperties).height as number | undefined,
            }
            : {}),
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
          ...(node.type === "subWorkflow"
            ? {
              subWorkflowId: nodeConfig?.sub_workflow_id as string | undefined,
              subWorkflowName: nodeConfig?.sub_workflow_name as string | undefined,
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
                    ?? useAgentStore
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

    const flowNodeIndexMap = new Map<string, number>();
    for (let i = 0; i < flowNodes.length; i++) {
      flowNodeIndexMap.set(flowNodes[i].id, i);
    }

    const hiddenChildIds = new Set<string>();
    const expectedParentByNode: Record<string, string> = {};
    for (const node of nodes) {
      const scopedNode = node as unknown as Record<string, unknown>;
      const scopedConfig = scopedNode.config as Record<string, unknown> | undefined;
      if (node.type === "parallel" && scopedConfig?.branches) {
        const branches = scopedConfig.branches as { steps?: string[] }[];
        for (const branch of branches) {
          for (const stepId of (branch.steps || []) as string[]) {
            const childIdx = flowNodeIndexMap.get(stepId);
            if (childIdx === undefined) { continue; }
            const storedParent = parentRefs[stepId];
            if (storedParent === undefined || storedParent === node.id) {
              const isCollapsedParent = collapsedContainers[node.id];
              if (isCollapsedParent) { hiddenChildIds.add(stepId); }
              flowNodes[childIdx] = {
                ...flowNodes[childIdx],
                hidden: isCollapsedParent ? true : flowNodes[childIdx].hidden,
              };
              expectedParentByNode[stepId] = node.id;
            }
          }
        }
      }
      if (node.type === "merge" && scopedConfig?.auto_inputs_from_branches) {
        const inputs = scopedConfig?.inputs as string[] | undefined;
        if (inputs) {
          for (const inputId of inputs) {
            const childIdx = flowNodeIndexMap.get(inputId);
            if (childIdx === undefined) { continue; }
            const targetParent = parentRefs[inputId];
            if (!targetParent) { continue; }
            const storedMergeParent = parentRefs[node.id];
            if (storedMergeParent === undefined || storedMergeParent === targetParent) {
              const mergeIdx = flowNodeIndexMap.get(node.id);
              if (mergeIdx === undefined) { continue; }
              const isCollapsedParent = collapsedContainers[targetParent];
              if (isCollapsedParent) { hiddenChildIds.add(node.id); }
              flowNodes[mergeIdx] = {
                ...flowNodes[mergeIdx],
                hidden: isCollapsedParent ? true : flowNodes[mergeIdx].hidden,
              };
              expectedParentByNode[node.id] = targetParent;
            }
          }
        }
      }
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
          const childIdx = flowNodeIndexMap.get(stepId);
          if (childIdx === undefined) { continue; }
          const storedParent = parentRefs[stepId];
          if (storedParent === undefined || storedParent === node.id) {
            const isCollapsedParent = collapsedContainers[node.id];
            if (isCollapsedParent) { hiddenChildIds.add(stepId); }
            flowNodes[childIdx] = {
              ...flowNodes[childIdx],
              hidden: isCollapsedParent ? true : flowNodes[childIdx].hidden,
            };
            expectedParentByNode[stepId] = node.id;
          }
        }
      }
    }

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
      const isCollapsedParent = collapsedContainers[containerNode.id];
      if (isCollapsedParent) { continue; }

      for (const subNode of subGraph.nodes) {
        const existingIdx = flowNodeIndexMap.get(subNode.id);
        const subTypeInfo = NODE_TYPE_MAP[subNode.type] || { color: token.colorTextQuaternary };
        const subData = {
          ...subNode,
          label: subNode.title,
          color: subTypeInfo.color,
          nodeType: subNode.type,
          enabled: true,
        };
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
          zIndex: 10,
          data: subData,
        };

        if (existingIdx !== undefined) {
          flowNodes[existingIdx] = {
            ...subFlowNode,
            data: {
              ...flowNodes[existingIdx].data,
              ...subData,
            },
          };
        } else {
          flowNodes.push(subFlowNode);
          flowNodeIndexMap.set(subNode.id, flowNodes.length - 1);
        }
      }
    }

    for (const [swNodeId, swData] of Object.entries(expandedSWData)) {
      if (!swData || swData.isLoading || swData.nodes.length === 0) { continue; }
      const parentFn = flowNodes.find((fn) => fn.id === swNodeId);
      if (!parentFn) { continue; }

      for (const subNode of swData.nodes) {
        flowNodes.push({
          id: subNode.id,
          type: (subNode as unknown as { type?: string }).type || "agent",
          position: { x: subNode.position.x, y: subNode.position.y },
          parentId: swNodeId,
          extent: "parent" as const,
          zIndex: 10,
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

    const nodeHiddenMap = new Map<string, boolean>();
    const hiddenChildToParent = new Map<string, string>();
    for (const fn of flowNodes) {
      nodeHiddenMap.set(fn.id, fn.hidden === true);
      if (fn.hidden) {
        const pid = parentRefs[fn.id];
        if (pid && collapsedContainers[pid]) {
          hiddenChildToParent.set(fn.id, pid);
        }
      }
    }
    const seenEdgeKeys = new Set<string>();

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
      const wasRemapped = remappedSource !== edge.source || remappedTarget !== edge.target;
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
          : edge.sourceHandle,
        target: remappedTarget,
        targetHandle: wasRemapped && remappedTarget !== edge.target ? undefined : edge.targetHandle,
        type: "base",
        animated: edge.edge_type === "loopBack",
        label: edge.label,
        data: { edgeType: edge.edge_type },
        ...((sourceIsOtherHidden || targetIsOtherHidden) ? { hidden: true } : {}),
      });
    }

    const seenSubEdgeIds = new Set(flowEdges.map((e) => e.id));
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
      const isCollapsedParent = collapsedContainers[containerNode.id];
      if (isCollapsedParent) { continue; }
      const subNodeIds = new Set((subGraph.nodes ?? []).map((n: { id: string }) => n.id));
      for (const subEdge of subGraph.edges) {
        if (!subNodeIds.has(subEdge.source) || !subNodeIds.has(subEdge.target)) { continue; }
        if (seenSubEdgeIds.has(subEdge.id)) { continue; }
        seenSubEdgeIds.add(subEdge.id);
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

    // 防御性处理：剔除孤儿 parentId + 拓扑排序保证父节点在子节点前面
    // React Flow 内部 adoptUserNodes 按数组顺序处理节点，遇到带 parentId 的子节点时
    // 必须保证父节点已在 nodeLookup 中，否则触发 "Parent node xxx not found" 警告。
    // 任何带 parentId 指向数组中不存在父节点的节点都视为孤儿，移除其 parentId。
    const flowNodeIdSet = new Set(flowNodes.map((n) => n.id));
    for (const fn of flowNodes) {
      const pid = (fn as { parentId?: string }).parentId;
      if (pid && !flowNodeIdSet.has(pid)) {
        // 孤儿：父节点不在本批次 flowNodes 中，移除 parentId
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        delete (fn as any).parentId;
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        delete (fn as any).extent;
        if (fn.data) {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          delete (fn.data as any).parentId;
        }
      }
    }
    // 拓扑排序：父节点排在子节点前面（Kahn 算法，保证稳定性）
    const inDegree = new Map<string, number>();
    const childrenOf = new Map<string, string[]>();
    for (const fn of flowNodes) {
      inDegree.set(fn.id, inDegree.get(fn.id) ?? 0);
      const pid = (fn as { parentId?: string }).parentId;
      if (pid && flowNodeIdSet.has(pid)) {
        inDegree.set(fn.id, (inDegree.get(fn.id) ?? 0) + 1);
        if (!childrenOf.has(pid)) { childrenOf.set(pid, []); }
        childrenOf.get(pid)!.push(fn.id);
      }
    }
    const queue: string[] = [];
    const flowNodeById = new Map<string, typeof flowNodes[number]>();
    for (const fn of flowNodes) { flowNodeById.set(fn.id, fn); }
    // 记录原始 index，保证拓扑排序中无父子关系的节点保持原顺序
    const originalIndex = new Map<string, number>();
    for (let i = 0; i < flowNodes.length; i++) { originalIndex.set(flowNodes[i].id, i); }
    // 入度为 0 的节点按原始 index 升序入队
    const indegZero = flowNodes.filter((fn) => (inDegree.get(fn.id) ?? 0) === 0);
    indegZero.sort((a, b) => (originalIndex.get(a.id) ?? 0) - (originalIndex.get(b.id) ?? 0));
    for (const fn of indegZero) { queue.push(fn.id); }
    const sorted: typeof flowNodes = [];
    const visited = new Set<string>();
    while (queue.length > 0) {
      const id = queue.shift()!;
      if (visited.has(id)) { continue; }
      visited.add(id);
      sorted.push(flowNodeById.get(id)!);
      const kids = childrenOf.get(id) ?? [];
      // 子节点按原始 index 升序处理
      kids.sort((a, b) => (originalIndex.get(a) ?? 0) - (originalIndex.get(b) ?? 0));
      for (const kid of kids) {
        const d = (inDegree.get(kid) ?? 0) - 1;
        inDegree.set(kid, d);
        if (d === 0 && !visited.has(kid)) { queue.push(kid); }
      }
    }
    // 处理环（理论上不应发生）或未访问的节点
    if (sorted.length < flowNodes.length) {
      for (const fn of flowNodes) {
        if (!visited.has(fn.id)) { sorted.push(fn); }
      }
    }

    return { flowNodes: sorted, flowEdges, expectedParentByNode };
  }, [
    nodes,
    edges,
    parentRefs,
    collapsedContainers,
    validationResult,
    frontendValidation,
    validationMsgMap,
    token,
    expandedSubWorkflows,
  ]);
}
