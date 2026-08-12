// SPDX-License-Identifier: AGPL-3.0-only
/**
 * Memory 知识图谱 → GraphView 数据适配器
 *
 * 将后端 Memory 知识图谱（Entity/Relationship）转换为 GraphView 所需的
 * GraphData 格式，实现图谱视图在 Memory 模块中的复用。
 */

import type { GraphData, GraphEdge, GraphEdgeType, GraphNode, GraphNodeType } from "@/components/wiki/GraphView";

// ── 后端数据类型（与 Rust Entity/Relationship 对应） ──

export interface MemoryEntity {
  id: string;
  name: string;
  entity_type: string; // project | user | concept | file | task
  properties?: Record<string, unknown>;
  aliases?: string[];
  first_seen_at?: string;
  last_seen_at?: string;
  mention_count?: number;
  confidence?: number;
  created_at?: string;
  updated_at?: string;
}

export interface MemoryRelationship {
  id: string;
  source_id: string;
  target_id: string;
  relation_type: string; // part_of | related_to | depends_on | ...
  properties?: Record<string, unknown>;
  weight?: number;
  created_at?: string;
}

export interface MemoryGraphResponse {
  entities: MemoryEntity[];
  relationships: MemoryRelationship[];
}

// ── EntityType → GraphNodeType 映射 ──

const entityTypeToGraphNodeType = (entityType: string): GraphNodeType => {
  switch (entityType.toLowerCase()) {
    case "concept":
    case "task":
      return "concept";
    case "project":
      return "note";
    case "user":
    case "file":
      return "entity";
    default:
      return "concept";
  }
};

// ── RelationshipType → GraphEdgeType 映射 ──

const relationshipTypeToGraphEdgeType = (relationType: string): GraphEdgeType => {
  switch (relationType.toLowerCase().replace(/_/g, "")) {
    case "partof":
    case "contains":
    case "implements":
    case "methodof":
    case "performs":
      return "link";
    case "dependson":
    case "defines":
    case "calls":
      return "derived_from";
    case "owns":
      return "mapping";
    case "relatedto":
    case "associatedwith":
      return "reference";
    default:
      return "reference";
  }
};

/**
 * 将 Memory 知识图谱数据转换为 GraphData
 * @param response 后端 list_knowledge_graph 返回的数据
 * @returns GraphView 可直接消费的 GraphData
 */
export function adaptMemoryToGraphData(response: MemoryGraphResponse): GraphData {
  // 防御：IPC 层可能返回空数组或缺失字段（如浏览器 mock 兜底），避免后续 .map 崩溃。
  const entities = Array.isArray(response?.entities) ? response.entities : [];
  const relationships = Array.isArray(response?.relationships) ? response.relationships : [];

  // 构建 Entity 查找表
  const entityMap = new Map<string, MemoryEntity>();
  for (const entity of entities) {
    entityMap.set(entity.id, entity);
  }

  // 转换节点
  const nodes: GraphNode[] = entities.map((entity) => {
    const graphType = entityTypeToGraphNodeType(entity.entity_type);
    const tags = entity.aliases ?? [];
    // 将 properties 中的 key-value 也作为标签
    if (entity.properties) {
      for (const [key, value] of Object.entries(entity.properties)) {
        if (typeof value === "string" && value.length < 30) {
          tags.push(`${key}: ${value}`);
        }
      }
    }

    return {
      id: entity.id,
      title: entity.name,
      type: graphType,
      tags,
      linkCount: entity.mention_count ?? 0,
      backlinkCount: 0,
      path: entity.entity_type,
    };
  });

  // 转换边（过滤掉无效边）
  const edges: GraphEdge[] = [];
  const edgeSet = new Set<string>(); // 去重

  for (const rel of relationships) {
    // 检查源和目标节点是否存在
    if (!entityMap.has(rel.source_id) || !entityMap.has(rel.target_id)) {
      continue;
    }

    // 去重：同一 source-target 对只保留第一条
    const edgeKey = `${rel.source_id}→${rel.target_id}`;
    if (edgeSet.has(edgeKey)) {
      continue;
    }
    edgeSet.add(edgeKey);

    edges.push({
      source: rel.source_id,
      target: rel.target_id,
      type: relationshipTypeToGraphEdgeType(rel.relation_type),
    });
  }

  return { nodes, edges };
}

/**
 * 计算节点的反向链接数（backlinkCount）
 * 根据入度计算每个节点被引用的次数
 */
export function computeBacklinkCounts(
  graphData: GraphData,
): Map<string, number> {
  const backlinkMap = new Map<string, number>();

  // 初始化所有节点的 backlinkCount
  for (const node of graphData.nodes) {
    backlinkMap.set(node.id, 0);
  }

  // 计算入度
  for (const edge of graphData.edges) {
    const count = backlinkMap.get(edge.target) ?? 0;
    backlinkMap.set(edge.target, count + 1);
  }

  return backlinkMap;
}

/**
 * 格式化实体类型为显示标签（返回 i18n key，由调用方使用 t() 翻译）
 */
export function getEntityTypeKey(type: string): string {
  const keyMap: Record<string, string> = {
    project: "memory.graph.entityLabels.project",
    user: "memory.graph.entityLabels.user",
    concept: "memory.graph.entityLabels.concept",
    file: "memory.graph.entityLabels.file",
    task: "memory.graph.entityLabels.task",
  };
  return keyMap[type.toLowerCase()] ?? type;
}

/**
 * 格式化关系类型为显示标签（返回 i18n key，由调用方使用 t() 翻译）
 */
export function getRelationshipTypeKey(type: string): string {
  const keyMap: Record<string, string> = {
    part_of: "memory.graph.relationshipLabels.partOf",
    related_to: "memory.graph.relationshipLabels.relatedTo",
    depends_on: "memory.graph.relationshipLabels.dependsOn",
    owns: "memory.graph.relationshipLabels.owns",
    defines: "memory.graph.relationshipLabels.defines",
    implements: "memory.graph.relationshipLabels.implements",
    contains: "memory.graph.relationshipLabels.contains",
    calls: "memory.graph.relationshipLabels.calls",
    method_of: "memory.graph.relationshipLabels.methodOf",
    performs: "memory.graph.relationshipLabels.performs",
    associated_with: "memory.graph.relationshipLabels.associatedWith",
  };
  return keyMap[type.toLowerCase()] ?? type;
}
