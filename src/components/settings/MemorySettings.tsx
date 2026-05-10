import { EmbeddingModelSelect } from "@/components/shared/EmbeddingModelSelect";
import { IconEditor } from "@/components/shared/IconEditor";
import { NamespaceIcon } from "@/components/shared/NamespaceIcon";
import { invoke } from "@/lib/invoke";
import { listen } from "@/lib/invoke";
import { formatImportance, getNatureLabel, getTierColor, getTierLabel, TIER_COLORS } from "@/lib/memoryUtils";
import type { MemoryNature, MemoryTier as MemoryTierType } from "@/lib/memoryUtils";
import { useMemoryStore } from "@/stores";
import type { MemoryItem, MemoryNamespace, MemorySource } from "@/types";
import { closestCenter, DndContext, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  Badge,
  Button,
  Collapse,
  Divider,
  Dropdown,
  Empty,
  Form,
  Input,
  InputNumber,
  message,
  Modal,
  Popconfirm,
  Progress,
  Spin,
  Table,
  Tag,
  theme,
  Tooltip,
  Typography,
} from "antd";
import type { MenuProps } from "antd";
import {
  ArrowDown,
  ArrowRightLeft,
  ArrowUp,
  Brain,
  Clock,
  GitMerge,
  GripVertical,
  MoreHorizontal,
  Network,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Settings,
  ThumbsDown,
  ThumbsUp,
  Trash,
  Trash2,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface VectorSearchResult {
  id: string;
  document_id: string;
  chunk_index: number;
  content: string;
  score: number;
}

interface WorkingMemoryEntry {
  id: string;
  content: string;
  tier: MemoryTierType;
  importance: number;
  access_count: number;
  nature: MemoryNature;
  tags: string[];
  created_at: number;
  provenance: {
    conversation_id: string | null;
    message_id: string | null;
    extraction_method: string;
  } | null;
}

interface MemoryCluster {
  ids: string[];
  contents: string[];
  avg_importance: number;
  best_tier_priority: number;
}

interface KnowledgeEntity {
  id: string;
  name: string;
  entity_type: string;
  aliases: string[];
  mention_count: number;
  confidence: number;
}

interface KnowledgeRelationship {
  id: string;
  source_id: string;
  target_id: string;
  relation_type: string;
  weight: number;
}

interface TierStats {
  memory_count: number;
  user_count: number;
  total_tokens: number;
  tier_counts: Record<string, number>;
}

interface SearchExplanation {
  matched_keywords: string[];
  relevance_score: number;
  effective_score: number;
  recency_score: number;
  total_score: number;
  reason: string;
}

interface ExplainedSearchResult {
  entry: WorkingMemoryEntry;
  explanation: SearchExplanation;
}

interface TimeGroupedMemories {
  today: WorkingMemoryEntry[];
  this_week: WorkingMemoryEntry[];
  this_month: WorkingMemoryEntry[];
  older: WorkingMemoryEntry[];
}

const SOURCE_TAG_COLOR: Record<MemorySource, string> = {
  manual: "blue",
  auto_extract: "green",
};

const INDEX_STATUS_CONFIG: Record<string, { color: string; labelKey: string }> = {
  pending: { color: "default", labelKey: "settings.indexStatus.pending" },
  indexing: { color: "processing", labelKey: "settings.indexStatus.indexing" },
  ready: { color: "success", labelKey: "settings.indexStatus.indexed" },
  failed: { color: "error", labelKey: "settings.indexStatus.failed" },
  skipped: { color: "warning", labelKey: "settings.indexStatus.notConfigured" },
};

// ── Sortable Namespace Item ──────────────────────────────

function SortableNamespaceItem({
  ns,
  isSelected,
  onSelect,
  onDelete,
}: {
  ns: MemoryNamespace;
  isSelected: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: ns.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
    borderRadius: token.borderRadius,
    backgroundColor: isSelected ? token.colorPrimaryBg : undefined,
  };

  const menuItems: MenuProps["items"] = [
    {
      key: "delete",
      label: t("settings.memory.deleteNamespace"),
      icon: <Trash2 size={14} />,
      danger: true,
      onClick: (e) => {
        e.domEvent.stopPropagation();
        Modal.confirm({
          title: t("settings.memory.deleteConfirm"),
          okButtonProps: { danger: true },
          onOk: onDelete,
        });
      },
    },
  ];

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="flex items-center cursor-pointer px-3 py-2.5 transition-colors"
      onClick={onSelect}
      onMouseEnter={(e) => {
        if (!isSelected) { e.currentTarget.style.backgroundColor = token.colorFillQuaternary; }
      }}
      onMouseLeave={(e) => {
        if (!isSelected) { e.currentTarget.style.backgroundColor = isSelected ? token.colorPrimaryBg : ""; }
      }}
    >
      <div
        {...attributes}
        {...listeners}
        className="flex items-center mr-2 cursor-grab"
        onClick={(e) => e.stopPropagation()}
      >
        <GripVertical size={14} style={{ color: token.colorTextQuaternary }} />
      </div>
      <div style={{ marginRight: 8 }}>
        <NamespaceIcon ns={ns} size={16} />
      </div>
      <div className="min-w-0 flex-1">
        <span style={{ color: isSelected ? token.colorPrimary : undefined }}>{ns.name}</span>
      </div>
      <Tag
        color={ns.embeddingProvider ? "green" : "default"}
        style={{ marginRight: 4, fontSize: 11 }}
      >
        {ns.embeddingProvider ? t("settings.memory.vectorReady") : t("settings.memory.vectorNotConfigured")}
      </Tag>
      <Dropdown menu={{ items: menuItems }} trigger={["click"]}>
        <Button
          type="text"
          size="small"
          icon={<MoreHorizontal size={14} />}
          onClick={(e) => e.stopPropagation()}
        />
      </Dropdown>
    </div>
  );
}

// ── Left Sidebar: Namespace List ──────────────────────────

function NamespaceList({
  namespaces,
  selectedId,
  onSelect,
  onAdd,
}: {
  namespaces: MemoryNamespace[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
}) {
  const { t } = useTranslation();
  const reorderNamespaces = useMemoryStore((s) => s.reorderNamespaces);
  const deleteNamespace = useMemoryStore((s) => s.deleteNamespace);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) { return; }
    const oldIndex = namespaces.findIndex((n) => n.id === active.id);
    const newIndex = namespaces.findIndex((n) => n.id === over.id);
    if (oldIndex === -1 || newIndex === -1) { return; }
    const newOrder = [...namespaces];
    const [moved] = newOrder.splice(oldIndex, 1);
    newOrder.splice(newIndex, 0, moved);
    reorderNamespaces(newOrder.map((n) => n.id));
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex-1 overflow-y-auto p-2 flex flex-col gap-1">
        {namespaces.length === 0
          ? (
            <div className="flex-1 flex items-center justify-center">
              <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("settings.memory.empty")} />
            </div>
          )
          : (
            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
              <SortableContext items={namespaces.map((n) => n.id)} strategy={verticalListSortingStrategy}>
                {namespaces.map((ns) => (
                  <SortableNamespaceItem
                    key={ns.id}
                    ns={ns}
                    isSelected={selectedId === ns.id}
                    onSelect={() => onSelect(ns.id)}
                    onDelete={() => deleteNamespace(ns.id)}
                  />
                ))}
              </SortableContext>
            </DndContext>
          )}
      </div>
      <div className="shrink-0 p-2 pt-0">
        <Button
          type="dashed"
          block
          icon={<Plus size={14} />}
          onClick={onAdd}
        >
          {t("settings.memory.addNamespace")}
        </Button>
      </div>
    </div>
  );
}

// ── Right Panel: Memory Items ─────────────────────────────

function MemoryItemsPanel({
  namespace,
}: {
  namespace: MemoryNamespace;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { items, loading, loadItems, addItem, deleteItem, updateItem, updateNamespace } = useMemoryStore();
  const [itemModalOpen, setItemModalOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<MemoryItem | null>(null);
  const [itemForm] = Form.useForm();
  const [messageApi, contextHolder] = message.useMessage();

  // Settings modal state
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsForm, setSettingsForm] = useState({
    name: "",
    embeddingProvider: undefined as string | undefined,
    embeddingDimensions: undefined as number | undefined,
    retrievalThreshold: undefined as number | undefined,
    retrievalTopK: undefined as number | undefined,
  });
  const [originalProvider, setOriginalProvider] = useState<string | undefined>(undefined);

  // Pending embedding provider change (for confirmation)
  const [pendingProvider, setPendingProvider] = useState<string | undefined>(undefined);
  const [providerConfirmOpen, setProviderConfirmOpen] = useState(false);

  // Search state
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<VectorSearchResult[] | null>(null);
  const [searching, setSearching] = useState(false);
  const [explainedSearch, setExplainedSearch] = useState(false);
  const [explainedResults, setExplainedResults] = useState<ExplainedSearchResult[]>([]);

  // Index status
  const [rebuildingIndex, setRebuildingIndex] = useState(false);
  const rebuildingRef = useRef(false);

  // Working Memory state
  const [workingMemories, setWorkingMemories] = useState<WorkingMemoryEntry[]>([]);
  const [workingMemoriesLoading, setWorkingMemoriesLoading] = useState(false);

  // Tier Stats state
  const [tierStats, setTierStats] = useState<TierStats | null>(null);

  // Find Duplicates state
  const [duplicatesModalOpen, setDuplicatesModalOpen] = useState(false);
  const [duplicatesLoading, setDuplicatesLoading] = useState(false);
  const [clusters, setClusters] = useState<MemoryCluster[]>([]);
  const [consolidatingIds, setConsolidatingIds] = useState<Set<string>>(new Set());

  // Knowledge Graph state
  const [knowledgeGraphModalOpen, setKnowledgeGraphModalOpen] = useState(false);
  const [knowledgeGraphLoading, setKnowledgeGraphLoading] = useState(false);
  const [entities, setEntities] = useState<KnowledgeEntity[]>([]);
  const [relationships, setRelationships] = useState<KnowledgeRelationship[]>([]);

  // Timeline state
  const [timelineModalOpen, setTimelineModalOpen] = useState(false);
  const [timelineLoading, setTimelineLoading] = useState(false);
  const [timelineData, setTimelineData] = useState<TimeGroupedMemories | null>(null);

  useEffect(() => {
    loadItems(namespace.id);
  }, [namespace.id, loadItems]);

  // Load working memories and tier stats when namespace changes
  useEffect(() => {
    loadWorkingMemories();
    loadTierStats();
  }, [namespace.id]);

  // Listen for indexing events
  useEffect(() => {
    const unlistenIndexed = listen<
      { itemId: string; success: boolean; status?: string; error?: string; isRebuild?: boolean }
    >(
      "memory-item-indexed",
      () => {
        loadItems(namespace.id);
      },
    );
    const unlistenRebuild = listen<{ namespaceId: string }>(
      "memory-rebuild-complete",
      (event) => {
        if (event.payload.namespaceId === namespace.id) {
          setRebuildingIndex(false);
          rebuildingRef.current = false;
          loadItems(namespace.id);
        }
      },
    );
    return () => {
      unlistenIndexed.then((fn) => fn());
      unlistenRebuild.then((fn) => fn());
    };
  }, [namespace.id, loadItems]);

  const loadWorkingMemories = async () => {
    setWorkingMemoriesLoading(true);
    try {
      const result = await invoke<WorkingMemoryEntry[]>("search_working_memories", { query: "", limit: 50 });
      setWorkingMemories(result);
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setWorkingMemoriesLoading(false);
    }
  };

  const loadTierStats = async () => {
    try {
      const result = await invoke<TierStats>("get_memory_tier_stats");
      setTierStats(result);
    } catch {
      // silently fail - stats are supplementary
    }
  };

  const handleAddItem = async () => {
    try {
      const values = await itemForm.validateFields();
      const content: string = values.content;
      await addItem(namespace.id, content.slice(0, 50), content);
      setItemModalOpen(false);
      itemForm.resetFields();
    } catch {
      // validation error
    }
  };

  const handleEditItem = async () => {
    if (!editingItem) { return; }
    try {
      const values = await itemForm.validateFields();
      await updateItem(namespace.id, editingItem.id, {
        content: values.content,
        title: values.content.slice(0, 50),
      });
      setEditingItem(null);
      itemForm.resetFields();
      messageApi.success(t("settings.memory.updateSuccess"));
    } catch {
      // validation error
    }
  };

  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) { return; }
    setSearching(true);
    try {
      if (explainedSearch) {
        const results = await invoke<ExplainedSearchResult[]>("search_memories_explained", {
          query: searchQuery,
          limit: 10,
        });
        setExplainedResults(results);
        setSearchResults(null);
      } else if (namespace.embeddingProvider) {
        const results = await invoke<VectorSearchResult[]>("search_memory", {
          namespaceId: namespace.id,
          query: searchQuery,
          topK: 5,
        });
        setSearchResults([...results].sort((a, b) => a.score - b.score));
        setExplainedResults([]);
      }
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setSearching(false);
    }
  }, [searchQuery, namespace.id, namespace.embeddingProvider, explainedSearch, messageApi]);

  const handlePromoteMemory = async (memoryId: string) => {
    try {
      await invoke("promote_memory_entry", { memoryId });
      messageApi.success(t("settings.memory.promoteSuccess"));
      loadItems(namespace.id);
      loadWorkingMemories();
      loadTierStats();
    } catch (e) {
      messageApi.error(String(e));
    }
  };

  const handleDemoteMemory = async (memoryId: string) => {
    try {
      await invoke("demote_memory_entry", { memoryId });
      messageApi.success(t("settings.memory.demoteSuccess"));
      loadItems(namespace.id);
      loadWorkingMemories();
      loadTierStats();
    } catch (e) {
      messageApi.error(String(e));
    }
  };

  const handleFeedback = async (memoryId: string, feedback: "useful" | "not_useful") => {
    try {
      await invoke("submit_memory_feedback", { memoryId, feedback });
      messageApi.success(t("settings.memory.feedbackSuccess"));
    } catch (e) {
      messageApi.error(String(e));
    }
  };

  const handleFindDuplicates = async () => {
    setDuplicatesLoading(true);
    setDuplicatesModalOpen(true);
    try {
      const result = await invoke<MemoryCluster[]>("find_memory_clusters", { similarityThreshold: 0.7 });
      setClusters(result);
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setDuplicatesLoading(false);
    }
  };

  const handleConsolidate = async (cluster: MemoryCluster) => {
    const key = cluster.ids.join(",");
    setConsolidatingIds((prev) => new Set(prev).add(key));
    try {
      await invoke("consolidate_memory_cluster", { memoryIds: cluster.ids });
      messageApi.success(t("settings.memory.consolidateSuccess"));
      loadItems(namespace.id);
      loadWorkingMemories();
      loadTierStats();
      // Refresh duplicates
      const result = await invoke<MemoryCluster[]>("find_memory_clusters", { similarityThreshold: 0.7 });
      setClusters(result);
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setConsolidatingIds((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }
  };

  const handleKnowledgeGraph = async () => {
    setKnowledgeGraphLoading(true);
    setKnowledgeGraphModalOpen(true);
    try {
      const result = await invoke<{ entities: KnowledgeEntity[]; relationships: KnowledgeRelationship[] }>(
        "list_knowledge_graph",
      );
      setEntities(result.entities);
      setRelationships(result.relationships);
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setKnowledgeGraphLoading(false);
    }
  };

  const handleTimeline = async () => {
    setTimelineLoading(true);
    setTimelineModalOpen(true);
    try {
      const result = await invoke<TimeGroupedMemories>("get_memories_time_grouped");
      setTimelineData(result);
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setTimelineLoading(false);
    }
  };

  const formatAge = (createdAt: number) => {
    const now = Date.now() / 1000;
    const diffSec = now - createdAt;
    if (diffSec < 60) { return t("settings.memory.ageJustNow"); }
    if (diffSec < 3600) { return t("settings.memory.ageMinutes", { count: Math.floor(diffSec / 60) }); }
    if (diffSec < 86400) { return t("settings.memory.ageHours", { count: Math.floor(diffSec / 3600) }); }
    return t("settings.memory.ageDays", { count: Math.floor(diffSec / 86400) });
  };

  const itemColumns = [
    {
      title: t("settings.memory.itemContent"),
      dataIndex: "content",
      key: "content",
      ellipsis: { showTitle: true },
    },
    {
      title: t("settings.memory.indexStatusLabel"),
      dataIndex: "indexStatus",
      key: "indexStatus",
      width: 100,
      render: (status: string, record: MemoryItem) => {
        const cfg = INDEX_STATUS_CONFIG[status] || INDEX_STATUS_CONFIG.pending;
        const tag = (
          <Tag color={cfg.color} style={{ fontSize: 11 }}>
            {status === "indexing" && <Spin size="small" style={{ marginRight: 4 }} />}
            {t(cfg.labelKey)}
          </Tag>
        );
        if (status === "failed" && record.indexError) {
          return <Tooltip title={record.indexError}>{tag}</Tooltip>;
        }
        return tag;
      },
    },
    {
      title: t("settings.memory.source"),
      dataIndex: "source",
      key: "source",
      width: 90,
      render: (source: MemorySource) => (
        <Tag color={SOURCE_TAG_COLOR[source]}>
          {t(`settings.memory.${source === "auto_extract" ? "autoExtract" : "manual"}`)}
        </Tag>
      ),
    },
    {
      title: t("settings.memory.tier"),
      dataIndex: "tier",
      key: "tier",
      width: 90,
      render: (tier: MemoryTierType) => (
        <Tag color={getTierColor(tier)} style={{ fontSize: 11 }}>
          {getTierLabel(tier)}
        </Tag>
      ),
    },
    {
      title: t("settings.memory.importance"),
      dataIndex: "importance",
      key: "importance",
      width: 70,
      render: (importance: number) => (
        <Tooltip title={`${(importance * 100).toFixed(0)}%`}>
          <span
            style={{ fontSize: 12, color: importance >= 0.7 ? "#f59e0b" : importance >= 0.4 ? "#3b82f6" : "#94a3b8" }}
          >
            {formatImportance(importance)}
          </span>
        </Tooltip>
      ),
    },
    {
      title: t("settings.memory.nature"),
      dataIndex: "nature",
      key: "nature",
      width: 80,
      render: (nature: string) => (
        <Tag style={{ fontSize: 11 }}>{getNatureLabel(nature as "episodic" | "semantic")}</Tag>
      ),
    },
    {
      key: "actions",
      width: 200,
      render: (_: unknown, record: MemoryItem) => (
        <div className="flex gap-1">
          <Tooltip title={t("settings.memory.promote")}>
            <Button
              size="small"
              type="text"
              icon={<ArrowUp size={14} />}
              onClick={() => handlePromoteMemory(record.id)}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.demote")}>
            <Button
              size="small"
              type="text"
              icon={<ArrowDown size={14} />}
              onClick={() => handleDemoteMemory(record.id)}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.feedbackUseful")}>
            <Button
              size="small"
              type="text"
              icon={<ThumbsUp size={14} />}
              onClick={() => handleFeedback(record.id, "useful")}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.feedbackNotUseful")}>
            <Button
              size="small"
              type="text"
              icon={<ThumbsDown size={14} />}
              onClick={() => handleFeedback(record.id, "not_useful")}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.editItem")}>
            <Button
              size="small"
              type="text"
              icon={<Pencil size={14} />}
              onClick={() => {
                setEditingItem(record);
                itemForm.setFieldsValue({ content: record.content });
              }}
            />
          </Tooltip>
          <Popconfirm
            title={t("settings.memory.rebuildItemConfirm")}
            placement="bottom"
            onConfirm={async () => {
              await invoke("reindex_memory_item", { namespaceId: namespace.id, itemId: record.id }).catch((e) => {
                messageApi.error(String(e));
              });
              loadItems(namespace.id);
            }}
          >
            <Tooltip title={t("settings.memory.reindexItem")}>
              <Button
                size="small"
                type="text"
                icon={<Zap size={14} />}
                loading={record.indexStatus === "indexing"}
                disabled={!namespace.embeddingProvider}
              />
            </Tooltip>
          </Popconfirm>
          <Popconfirm
            title={t("settings.memory.deleteConfirm")}
            onConfirm={() => deleteItem(namespace.id, record.id)}
          >
            <Button size="small" danger type="text" icon={<Trash2 size={14} />} />
          </Popconfirm>
        </div>
      ),
    },
  ];

  const workingMemoryColumns = [
    {
      title: t("settings.memory.itemContent"),
      dataIndex: "content",
      key: "content",
      ellipsis: { showTitle: true },
      render: (content: string) => (
        <Typography.Paragraph ellipsis={{ rows: 1 }} style={{ margin: 0, fontSize: 12 }}>
          {content}
        </Typography.Paragraph>
      ),
    },
    {
      title: t("settings.memory.tier"),
      dataIndex: "tier",
      key: "tier",
      width: 80,
      render: (tier: MemoryTierType) => (
        <Tag color={getTierColor(tier)} style={{ fontSize: 11 }}>
          {getTierLabel(tier)}
        </Tag>
      ),
    },
    {
      title: t("settings.memory.importance"),
      dataIndex: "importance",
      key: "importance",
      width: 60,
      render: (importance: number) => <span style={{ fontSize: 12 }}>{formatImportance(importance)}</span>,
    },
    {
      title: t("settings.memory.nature"),
      dataIndex: "nature",
      key: "nature",
      width: 70,
      render: (nature: MemoryNature) => <Tag style={{ fontSize: 11 }}>{getNatureLabel(nature)}</Tag>,
    },
    {
      title: t("settings.memory.age"),
      dataIndex: "created_at",
      key: "created_at",
      width: 80,
      render: (createdAt: number) => (
        <span style={{ fontSize: 12, color: token.colorTextSecondary }}>{formatAge(createdAt)}</span>
      ),
    },
    {
      key: "actions",
      width: 120,
      render: (_: unknown, record: WorkingMemoryEntry) => (
        <div className="flex gap-1">
          <Tooltip title={t("settings.memory.promote")}>
            <Button
              size="small"
              type="text"
              icon={<ArrowUp size={12} />}
              onClick={() => handlePromoteMemory(record.id)}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.demote")}>
            <Button
              size="small"
              type="text"
              icon={<ArrowDown size={12} />}
              onClick={() => handleDemoteMemory(record.id)}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.feedbackUseful")}>
            <Button
              size="small"
              type="text"
              icon={<ThumbsUp size={12} />}
              onClick={() => handleFeedback(record.id, "useful")}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.feedbackNotUseful")}>
            <Button
              size="small"
              type="text"
              icon={<ThumbsDown size={12} />}
              onClick={() => handleFeedback(record.id, "not_useful")}
            />
          </Tooltip>
        </div>
      ),
    },
  ];

  return (
    <div className="p-6 pb-12 overflow-y-auto h-full">
      {contextHolder}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <IconEditor
            iconType={namespace.iconType}
            iconValue={namespace.iconValue}
            onChange={(type, value) =>
              updateNamespace(namespace.id, {
                iconType: type ?? undefined,
                iconValue: value ?? undefined,
                updateIcon: true,
              })}
            size={28}
            defaultIcon={<NamespaceIcon ns={namespace} size={28} />}
          />
          <span style={{ fontWeight: 600, fontSize: 16 }}>{namespace.name}</span>
        </div>
        <div className="flex items-center gap-2">
          <Tag
            color={namespace.embeddingProvider ? "green" : "default"}
            style={{ fontSize: 12 }}
          >
            {namespace.embeddingProvider ? t("settings.memory.vectorReady") : t("settings.memory.vectorNotConfigured")}
          </Tag>
          <Tooltip title={t("settings.memory.namespaceSettings")}>
            <Button
              size="small"
              type="text"
              icon={<Settings size={14} />}
              onClick={() => {
                setSettingsForm({
                  name: namespace.name,
                  embeddingProvider: namespace.embeddingProvider ?? undefined,
                  embeddingDimensions: namespace.embeddingDimensions ?? undefined,
                  retrievalThreshold: namespace.retrievalThreshold ?? 0.1,
                  retrievalTopK: namespace.retrievalTopK ?? 5,
                });
                setOriginalProvider(namespace.embeddingProvider ?? undefined);
                setSettingsOpen(true);
              }}
            />
          </Tooltip>
        </div>
      </div>

      {/* Working Memory Panel */}
      <Collapse
        defaultActiveKey={[]}
        style={{ marginBottom: 12 }}
        items={[
          {
            key: "working-memory",
            label: (
              <div className="flex items-center gap-2">
                <Brain size={14} style={{ color: token.colorPrimary }} />
                <span style={{ fontWeight: 500 }}>{t("settings.memory.workingMemory")}</span>
                <Badge
                  count={workingMemories.length}
                  showZero
                  style={{ backgroundColor: token.colorPrimary }}
                  size="small"
                />
              </div>
            ),
            extra: (
              <Button
                size="small"
                type="text"
                icon={<RefreshCw size={12} />}
                loading={workingMemoriesLoading}
                onClick={(e) => {
                  e.stopPropagation();
                  loadWorkingMemories();
                }}
              />
            ),
            children: workingMemories.length === 0
              ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("settings.memory.noWorkingMemory")} />
              : (
                <Table
                  dataSource={workingMemories}
                  columns={workingMemoryColumns}
                  rowKey="id"
                  pagination={false}
                  loading={workingMemoriesLoading}
                  size="small"
                  bordered
                  scroll={{ y: 300 }}
                />
              ),
          },
        ]}
      />

      {/* Tier Statistics */}
      {tierStats && (
        <div
          className="flex items-center gap-3 mb-3 px-3 py-2"
          style={{
            backgroundColor: token.colorFillQuaternary,
            borderRadius: token.borderRadius,
            fontSize: 12,
          }}
        >
          <span style={{ color: token.colorTextSecondary, fontWeight: 500 }}>
            {t("settings.memory.tierStats")}
          </span>
          {(Object.keys(TIER_COLORS) as MemoryTierType[]).map((tier) => (
            <Tag
              key={tier}
              color={getTierColor(tier)}
              style={{ fontSize: 11, margin: 0 }}
            >
              {getTierLabel(tier)}: {tierStats.tier_counts[tier] ?? 0}
            </Tag>
          ))}
          <Divider type="vertical" style={{ margin: "0 4px" }} />
          <span style={{ color: token.colorTextSecondary }}>
            {t("settings.memory.totalMemories")}: {tierStats.memory_count}
          </span>
          <span style={{ color: token.colorTextSecondary }}>
            {t("settings.memory.totalTokens")}: {tierStats.total_tokens}
          </span>
        </div>
      )}

      {/* Settings Modal */}
      <Modal
        title={t("settings.memory.namespaceSettings")}
        open={settingsOpen}
        onOk={async () => {
          const providerChanged = settingsForm.embeddingProvider !== originalProvider;
          if (providerChanged && originalProvider) {
            setPendingProvider(settingsForm.embeddingProvider);
            setProviderConfirmOpen(true);
            return;
          }
          await updateNamespace(namespace.id, {
            name: settingsForm.name,
            embeddingProvider: settingsForm.embeddingProvider,
            updateEmbeddingProvider: providerChanged,
            embeddingDimensions: settingsForm.embeddingDimensions,
            updateEmbeddingDimensions: true,
            retrievalThreshold: settingsForm.retrievalThreshold,
            updateRetrievalThreshold: true,
            retrievalTopK: settingsForm.retrievalTopK,
            updateRetrievalTopK: true,
          });
          setSettingsOpen(false);
        }}
        onCancel={() => setSettingsOpen(false)}
        mask={{ enabled: true, blur: true }}
      >
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between">
            <span>{t("settings.memory.namespaceName")}</span>
            <Input
              value={settingsForm.name}
              onChange={(e) => setSettingsForm(s => ({ ...s, name: e.target.value }))}
              style={{ width: 280 }}
            />
          </div>
          <Divider style={{ margin: 0 }} />
          <div className="flex items-center justify-between">
            <span>{t("settings.memory.embeddingModel")}</span>
            <EmbeddingModelSelect
              value={settingsForm.embeddingProvider}
              onChange={(val) => setSettingsForm(s => ({ ...s, embeddingProvider: val || undefined }))}
              placeholder={t("settings.memory.embeddingModelPlaceholder")}
              style={{ width: 280 }}
            />
          </div>
          <Divider style={{ margin: 0 }} />
          <div className="flex items-center justify-between">
            <span>{t("settings.memory.embeddingDimensions")}</span>
            <InputNumber
              value={settingsForm.embeddingDimensions}
              onChange={(val) => setSettingsForm(s => ({ ...s, embeddingDimensions: val ?? undefined }))}
              placeholder={t("settings.memory.embeddingDimensionsAuto")}
              min={1}
              max={65536}
              style={{ width: 280 }}
            />
          </div>
          <Divider style={{ margin: 0 }} />
          <div className="flex items-center justify-between">
            <span>{t("settings.memory.retrievalThreshold")}</span>
            <InputNumber
              value={settingsForm.retrievalThreshold}
              onChange={(val) => setSettingsForm(s => ({ ...s, retrievalThreshold: val ?? 0.1 }))}
              min={0}
              max={2}
              step={0.01}
              style={{ width: 280 }}
            />
          </div>
          <Divider style={{ margin: 0 }} />
          <div className="flex items-center justify-between">
            <span>{t("settings.memory.retrievalTopK")}</span>
            <InputNumber
              value={settingsForm.retrievalTopK}
              onChange={(val) => setSettingsForm(s => ({ ...s, retrievalTopK: val ?? 5 }))}
              min={1}
              max={100}
              style={{ width: 280 }}
            />
          </div>
        </div>
      </Modal>

      {/* Embedding provider change confirmation */}
      <Modal
        title={t("settings.memory.changeEmbeddingTitle")}
        open={providerConfirmOpen}
        onOk={async () => {
          await updateNamespace(namespace.id, {
            name: settingsForm.name,
            embeddingProvider: pendingProvider,
            updateEmbeddingProvider: true,
            embeddingDimensions: settingsForm.embeddingDimensions,
            updateEmbeddingDimensions: true,
            retrievalThreshold: settingsForm.retrievalThreshold,
            updateRetrievalThreshold: true,
            retrievalTopK: settingsForm.retrievalTopK,
            updateRetrievalTopK: true,
          });
          setProviderConfirmOpen(false);
          setPendingProvider(undefined);
          setSettingsOpen(false);
          if (pendingProvider) {
            setRebuildingIndex(true);
            invoke("rebuild_memory_index", { namespaceId: namespace.id }).catch((e) => {
              setRebuildingIndex(false);
              messageApi.error(String(e));
            });
          }
        }}
        onCancel={() => {
          setProviderConfirmOpen(false);
          setPendingProvider(undefined);
        }}
        okButtonProps={{ danger: true }}
        mask={{ enabled: true, blur: true }}
      >
        <p>{t("settings.memory.changeEmbeddingWarning")}</p>
      </Modal>

      {/* Toolbar: add + rebuild + sync + find duplicates + knowledge graph on left, search + clear on right */}
      <div className="flex items-center justify-between mb-3 gap-3">
        <div className="flex items-center gap-2">
          <Tooltip title={t("settings.memory.addItem")}>
            <Button
              icon={<Plus size={14} />}
              onClick={() => {
                setEditingItem(null);
                itemForm.resetFields();
                setItemModalOpen(true);
              }}
            />
          </Tooltip>
          <Popconfirm
            title={t("settings.memory.rebuildIndexConfirm")}
            placement="bottom"
            onConfirm={async () => {
              setRebuildingIndex(true);
              rebuildingRef.current = true;
              try {
                await invoke("rebuild_memory_index", { namespaceId: namespace.id });
                loadItems(namespace.id);
              } catch (e) {
                setRebuildingIndex(false);
                rebuildingRef.current = false;
                messageApi.error(String(e));
              }
            }}
          >
            <Tooltip title={t("settings.memory.rebuildIndex")}>
              <Button
                icon={<Zap size={14} />}
                loading={rebuildingIndex}
                disabled={!namespace.embeddingProvider}
              />
            </Tooltip>
          </Popconfirm>
          <Popconfirm
            title={t("settings.memory.syncWorkingMemoryConfirm")}
            placement="bottom"
            onConfirm={async () => {
              try {
                const count = await invoke<number>("sync_working_memory_to_namespace", {
                  namespaceId: namespace.id,
                });
                messageApi.success(t("settings.memory.syncWorkingMemorySuccess", { count }));
                loadItems(namespace.id);
              } catch (e) {
                messageApi.error(t("settings.memory.syncWorkingMemoryError", { error: String(e) }));
              }
            }}
          >
            <Tooltip title={t("settings.memory.syncWorkingMemory")}>
              <Button icon={<ArrowRightLeft size={14} />} />
            </Tooltip>
          </Popconfirm>
          <Tooltip title={t("settings.memory.findDuplicates")}>
            <Button
              icon={<GitMerge size={14} />}
              onClick={handleFindDuplicates}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.knowledgeGraph")}>
            <Button
              icon={<Network size={14} />}
              onClick={handleKnowledgeGraph}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.timeline")}>
            <Button
              icon={<Clock size={14} />}
              onClick={handleTimeline}
            />
          </Tooltip>
        </div>
        <div className="flex items-center gap-2">
          <Input
            placeholder={t("settings.memory.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onPressEnter={handleSearch}
            style={{ width: 200 }}
            allowClear
            onClear={() => {
              setSearchResults(null);
              setExplainedResults([]);
            }}
          />
          <Tooltip title={t("settings.memory.explainedSearch")}>
            <Button
              size="small"
              type={explainedSearch ? "primary" : "default"}
              icon={<Brain size={14} />}
              onClick={() => setExplainedSearch(!explainedSearch)}
            />
          </Tooltip>
          <Tooltip title={t("settings.memory.search")}>
            <Button
              icon={<Search size={14} />}
              loading={searching}
              onClick={handleSearch}
              disabled={!searchQuery.trim() || (!explainedSearch && !namespace.embeddingProvider)}
            />
          </Tooltip>
          <Popconfirm
            title={t("settings.memory.clearIndexConfirm")}
            onConfirm={async () => {
              try {
                await invoke("clear_memory_index", { namespaceId: namespace.id });
                loadItems(namespace.id);
                messageApi.success(t("settings.memory.clearSuccess"));
              } catch (e) {
                messageApi.error(String(e));
              }
            }}
          >
            <Tooltip title={t("settings.memory.clearIndex")}>
              <Button
                danger
                icon={<Trash size={14} />}
                disabled={!namespace.embeddingProvider}
              />
            </Tooltip>
          </Popconfirm>
        </div>
      </div>

      {/* Search results */}
      <Modal
        title={`${t("settings.memory.searchResults")} (${searchResults?.length || 0})`}
        open={searchResults !== null}
        onCancel={() => setSearchResults(null)}
        footer={null}
        width={700}
        mask={{ enabled: true, blur: true }}
      >
        {searchResults && searchResults.length === 0
          ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("settings.memory.noResults")} />
          : (
            <Table
              dataSource={searchResults || []}
              rowKey={(_, i) => String(i)}
              pagination={{ pageSize: 10, size: "small" }}
              size="small"
              bordered
              columns={[
                {
                  title: "ID",
                  dataIndex: "document_id",
                  key: "document_id",
                  width: 100,
                  ellipsis: true,
                  render: (id: string) => <span style={{ fontSize: 12 }}>{id.slice(0, 8)}</span>,
                },
                {
                  title: t("settings.memory.itemContent"),
                  dataIndex: "content",
                  key: "content",
                  ellipsis: { showTitle: false },
                  render: (content: string) => (
                    <Typography.Paragraph
                      ellipsis={{ rows: 2 }}
                      style={{ margin: 0, fontSize: 13 }}
                    >
                      {content}
                    </Typography.Paragraph>
                  ),
                },
                {
                  title: t("settings.memory.similarity"),
                  dataIndex: "score",
                  key: "score",
                  width: 90,
                  defaultSortOrder: "ascend" as const,
                  sorter: (a: VectorSearchResult, b: VectorSearchResult) => a.score - b.score,
                  render: (score: number) => (
                    <Tag color="blue" style={{ fontSize: 11 }}>{(1 / (1 + score)).toFixed(4)}</Tag>
                  ),
                },
              ]}
            />
          )}
      </Modal>

      {/* Explained Search Results */}
      <Modal
        title={`${t("settings.memory.explainedSearchResults")} (${explainedResults.length})`}
        open={explainedResults.length > 0}
        onCancel={() => setExplainedResults([])}
        footer={null}
        width={800}
        mask={{ enabled: true, blur: true }}
      >
        <div className="flex flex-col gap-3">
          {explainedResults.map((result, idx) => (
            <div
              key={idx}
              className="p-3"
              style={{
                border: "1px solid var(--border-color)",
                borderRadius: 6,
              }}
            >
              <div className="flex items-start justify-between gap-2 mb-2">
                <Typography.Paragraph
                  ellipsis={{ rows: 2 }}
                  style={{ margin: 0, fontSize: 13, flex: 1 }}
                >
                  {result.entry.content}
                </Typography.Paragraph>
                <Tag color={getTierColor(result.entry.tier)} style={{ fontSize: 11, flexShrink: 0 }}>
                  {getTierLabel(result.entry.tier)}
                </Tag>
              </div>
              <div className="flex items-center gap-3 mb-2">
                <Tooltip title={`Relevance: ${(result.explanation.relevance_score * 100).toFixed(0)}%`}>
                  <Progress
                    percent={Math.round(result.explanation.relevance_score * 100)}
                    size="small"
                    style={{ width: 80, margin: 0 }}
                    strokeColor="#3b82f6"
                  />
                </Tooltip>
                <Tooltip title={`Effective: ${(result.explanation.effective_score * 100).toFixed(0)}%`}>
                  <Progress
                    percent={Math.round(result.explanation.effective_score * 100)}
                    size="small"
                    style={{ width: 80, margin: 0 }}
                    strokeColor="#8b5cf6"
                  />
                </Tooltip>
                <Tooltip title={`Recency: ${(result.explanation.recency_score * 100).toFixed(0)}%`}>
                  <Progress
                    percent={Math.round(result.explanation.recency_score * 100)}
                    size="small"
                    style={{ width: 80, margin: 0 }}
                    strokeColor="#10b981"
                  />
                </Tooltip>
                <Tag color="gold" style={{ fontSize: 11 }}>
                  {(result.explanation.total_score * 100).toFixed(0)}%
                </Tag>
              </div>
              {result.explanation.matched_keywords.length > 0 && (
                <div className="flex items-center gap-1 flex-wrap">
                  {result.explanation.matched_keywords.map((kw, ki) => (
                    <Tag key={ki} style={{ fontSize: 10 }}>{kw}</Tag>
                  ))}
                </div>
              )}
              {result.explanation.reason && (
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  {result.explanation.reason}
                </Typography.Text>
              )}
            </div>
          ))}
        </div>
      </Modal>

      {/* Find Duplicates Modal */}
      <Modal
        title={t("settings.memory.findDuplicates")}
        open={duplicatesModalOpen}
        onCancel={() => {
          setDuplicatesModalOpen(false);
          setClusters([]);
        }}
        footer={null}
        width={750}
        mask={{ enabled: true, blur: true }}
      >
        {duplicatesLoading
          ? (
            <div className="flex items-center justify-center py-8">
              <Spin />
            </div>
          )
          : clusters.length === 0
          ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("settings.memory.noDuplicates")} />
          : (
            <div className="flex flex-col gap-3">
              {clusters.map((cluster, idx) => (
                <div
                  key={idx}
                  className="flex flex-col gap-2 p-3"
                  style={{
                    backgroundColor: token.colorFillQuaternary,
                    borderRadius: token.borderRadius,
                  }}
                >
                  <div className="flex items-center justify-between">
                    <span style={{ fontWeight: 500, fontSize: 13 }}>
                      {t("settings.memory.clusterLabel", { index: idx + 1, count: cluster.ids.length })}
                    </span>
                    <div className="flex items-center gap-2">
                      <Tag style={{ fontSize: 11 }}>
                        {t("settings.memory.avgImportance")}: {(cluster.avg_importance * 100).toFixed(0)}%
                      </Tag>
                      <Button
                        size="small"
                        type="primary"
                        icon={<GitMerge size={12} />}
                        loading={consolidatingIds.has(cluster.ids.join(","))}
                        onClick={() => handleConsolidate(cluster)}
                      >
                        {t("settings.memory.consolidate")}
                      </Button>
                    </div>
                  </div>
                  <div className="flex flex-col gap-1">
                    {cluster.contents.map((content, cIdx) => (
                      <Typography.Paragraph
                        key={cIdx}
                        ellipsis={{ rows: 2 }}
                        style={{
                          margin: 0,
                          fontSize: 12,
                          paddingLeft: 8,
                          borderLeft: `2px solid ${token.colorBorder}`,
                        }}
                      >
                        {content}
                      </Typography.Paragraph>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
      </Modal>

      {/* Knowledge Graph Modal */}
      <Modal
        title={t("settings.memory.knowledgeGraph")}
        open={knowledgeGraphModalOpen}
        onCancel={() => {
          setKnowledgeGraphModalOpen(false);
          setEntities([]);
          setRelationships([]);
        }}
        footer={null}
        width={800}
        mask={{ enabled: true, blur: true }}
      >
        {knowledgeGraphLoading
          ? (
            <div className="flex items-center justify-center py-8">
              <Spin />
            </div>
          )
          : entities.length === 0 && relationships.length === 0
          ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("settings.memory.noKnowledgeGraph")} />
          : (
            <div className="flex flex-col gap-4">
              {/* Entities Section */}
              {entities.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 mb-2">
                    <span style={{ fontWeight: 600, fontSize: 14 }}>{t("settings.memory.entities")}</span>
                    <Badge count={entities.length} style={{ backgroundColor: token.colorPrimary }} />
                  </div>
                  <Table
                    dataSource={entities}
                    rowKey="id"
                    pagination={{ pageSize: 5, size: "small" }}
                    size="small"
                    bordered
                    columns={[
                      {
                        title: t("settings.memory.entityName"),
                        dataIndex: "name",
                        key: "name",
                        render: (name: string) => <span style={{ fontWeight: 500 }}>{name}</span>,
                      },
                      {
                        title: t("settings.memory.entityType"),
                        dataIndex: "entity_type",
                        key: "entity_type",
                        width: 100,
                        render: (entityType: string) => <Tag color="blue" style={{ fontSize: 11 }}>{entityType}</Tag>,
                      },
                      {
                        title: t("settings.memory.mentionCount"),
                        dataIndex: "mention_count",
                        key: "mention_count",
                        width: 90,
                        render: (count: number) => <span style={{ fontSize: 12 }}>{count}</span>,
                      },
                      {
                        title: t("settings.memory.confidence"),
                        dataIndex: "confidence",
                        key: "confidence",
                        width: 120,
                        render: (confidence: number) => (
                          <Progress
                            percent={Math.round(confidence * 100)}
                            size="small"
                            format={(pct) => `${pct}%`}
                          />
                        ),
                      },
                    ]}
                  />
                </div>
              )}
              {/* Relationships Section */}
              {relationships.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 mb-2">
                    <span style={{ fontWeight: 600, fontSize: 14 }}>{t("settings.memory.relationships")}</span>
                    <Badge count={relationships.length} style={{ backgroundColor: token.colorPrimary }} />
                  </div>
                  <Table
                    dataSource={relationships.map((r) => ({
                      ...r,
                      source_name: entities.find((e) => e.id === r.source_id)?.name ?? r.source_id,
                      target_name: entities.find((e) => e.id === r.target_id)?.name ?? r.target_id,
                    }))}
                    rowKey="id"
                    pagination={{ pageSize: 5, size: "small" }}
                    size="small"
                    bordered
                    columns={[
                      {
                        title: t("settings.memory.source"),
                        dataIndex: "source_name",
                        key: "source_name",
                        render: (name: string) => <span style={{ fontWeight: 500 }}>{name}</span>,
                      },
                      {
                        title: t("settings.memory.relationType"),
                        dataIndex: "relation_type",
                        key: "relation_type",
                        width: 120,
                        render: (relationType: string) => (
                          <Tag color="purple" style={{ fontSize: 11 }}>{relationType}</Tag>
                        ),
                      },
                      {
                        title: t("settings.memory.target"),
                        dataIndex: "target_name",
                        key: "target_name",
                        render: (name: string) => <span style={{ fontWeight: 500 }}>{name}</span>,
                      },
                      {
                        title: t("settings.memory.weight"),
                        dataIndex: "weight",
                        key: "weight",
                        width: 80,
                        render: (weight: number) => <span style={{ fontSize: 12 }}>{weight.toFixed(2)}</span>,
                      },
                    ]}
                  />
                </div>
              )}
            </div>
          )}
      </Modal>

      {/* Timeline Modal */}
      <Modal
        title={t("settings.memory.timeline")}
        open={timelineModalOpen}
        onCancel={() => {
          setTimelineModalOpen(false);
          setTimelineData(null);
        }}
        footer={null}
        width={700}
        mask={{ enabled: true, blur: true }}
      >
        {timelineLoading
          ? (
            <div className="flex items-center justify-center py-8">
              <Spin />
            </div>
          )
          : !timelineData
          ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} />
          : (() => {
            const sections: { key: keyof TimeGroupedMemories; label: string; color: string }[] = [
              { key: "today", label: t("settings.memory.timelineToday"), color: "#10b981" },
              { key: "this_week", label: t("settings.memory.timelineThisWeek"), color: "#3b82f6" },
              { key: "this_month", label: t("settings.memory.timelineThisMonth"), color: "#f59e0b" },
              { key: "older", label: t("settings.memory.timelineOlder"), color: "#94a3b8" },
            ];
            const total = sections.reduce((s, sec) => s + timelineData[sec.key].length, 0);
            if (total === 0) {
              return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("settings.memory.noTimelineData")} />;
            }
            return (
              <div className="flex flex-col gap-4">
                {sections.map((sec) => {
                  const entries = timelineData[sec.key];
                  if (entries.length === 0) { return null; }
                  return (
                    <div key={sec.key}>
                      <div className="flex items-center gap-2 mb-2">
                        <div style={{ width: 4, height: 18, borderRadius: 2, backgroundColor: sec.color }} />
                        <span style={{ fontWeight: 600, fontSize: 14 }}>{sec.label}</span>
                        <Badge count={entries.length} style={{ backgroundColor: sec.color }} />
                      </div>
                      <div className="flex flex-col gap-1">
                        {entries.map((entry) => (
                          <div
                            key={entry.id}
                            className="flex items-center gap-2"
                            style={{
                              padding: "6px 8px",
                              borderRadius: 6,
                              backgroundColor: token.colorBgLayout,
                              fontSize: 13,
                            }}
                          >
                            <Tag
                              color={getTierColor(entry.tier as MemoryTierType)}
                              style={{ fontSize: 11, margin: 0, flexShrink: 0 }}
                            >
                              {getTierLabel(entry.tier as MemoryTierType)}
                            </Tag>
                            <span
                              style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                            >
                              {entry.content}
                            </span>
                            <span style={{ fontSize: 11, color: token.colorTextSecondary, flexShrink: 0 }}>
                              {formatImportance(entry.importance)}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  );
                })}
              </div>
            );
          })()}
      </Modal>

      <Table
        dataSource={items}
        columns={itemColumns}
        rowKey="id"
        pagination={false}
        loading={loading}
        size="small"
        bordered
      />

      {/* Add / Edit Modal */}
      <Modal
        title={editingItem ? t("settings.memory.editItem") : t("settings.memory.addItem")}
        open={itemModalOpen || !!editingItem}
        onOk={editingItem ? handleEditItem : handleAddItem}
        onCancel={() => {
          setItemModalOpen(false);
          setEditingItem(null);
          itemForm.resetFields();
        }}
        mask={{ enabled: true, blur: true }}
      >
        <Form form={itemForm} layout="vertical">
          <Form.Item name="content" label={t("settings.memory.itemContent")} rules={[{ required: true }]}>
            <Input.TextArea autoSize={{ minRows: 3, maxRows: 8 }} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

// ── Main Component ────────────────────────────────────────

export default function MemorySettings() {
  const { t } = useTranslation();
  const { namespaces, loadNamespaces, createNamespace, setSelectedNamespaceId } = useMemoryStore();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [nsModalOpen, setNsModalOpen] = useState(false);
  const [nsForm] = Form.useForm();

  useEffect(() => {
    loadNamespaces();
  }, [loadNamespaces]);

  useEffect(() => {
    if (!selectedId && namespaces.length > 0) {
      setSelectedId(namespaces[0].id);
    }
  }, [namespaces, selectedId]);

  useEffect(() => {
    if (selectedId) {
      setSelectedNamespaceId(selectedId);
    }
  }, [selectedId, setSelectedNamespaceId]);

  const selectedNamespace = namespaces.find((ns) => ns.id === selectedId) ?? null;

  const handleAdd = () => {
    nsForm.resetFields();
    setNsModalOpen(true);
  };

  const handleCreate = async () => {
    try {
      const values = await nsForm.validateFields();
      await createNamespace(values.name, "global", values.embeddingProvider);
      setNsModalOpen(false);
      nsForm.resetFields();
    } catch {
      // validation error
    }
  };

  return (
    <div className="flex h-full">
      <div className="w-64 shrink-0 pt-2" style={{ borderRight: "1px solid var(--border-color)" }}>
        <NamespaceList
          namespaces={namespaces}
          selectedId={selectedId}
          onSelect={setSelectedId}
          onAdd={handleAdd}
        />
      </div>
      <div className="min-w-0 flex-1 overflow-y-auto">
        {selectedNamespace
          ? (
            <MemoryItemsPanel
              key={selectedNamespace.id}
              namespace={selectedNamespace}
            />
          )
          : (
            <div className="flex h-full items-center justify-center">
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={t("settings.memory.selectOrAdd")}
              />
            </div>
          )}
      </div>

      <Modal
        title={t("settings.memory.addNamespace")}
        open={nsModalOpen}
        onOk={handleCreate}
        onCancel={() => {
          setNsModalOpen(false);
          nsForm.resetFields();
        }}
        mask={{ enabled: true, blur: true }}
      >
        <Form form={nsForm} layout="vertical">
          <Form.Item name="name" label={t("settings.memory.namespaceName")} rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item
            name="embeddingProvider"
            label={t("settings.memory.embeddingModel")}
            rules={[{ required: true, message: t("settings.memory.embeddingModelPlaceholder") }]}
          >
            <EmbeddingModelSelect
              value={nsForm.getFieldValue("embeddingProvider")}
              onChange={(val) => nsForm.setFieldValue("embeddingProvider", val)}
              placeholder={t("settings.memory.embeddingModelPlaceholder")}
            />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
