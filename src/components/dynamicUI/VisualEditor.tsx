// SPDX-License-Identifier: AGPL-3.0-only

/**
 * VisualEditor — 可视化拖拽式 UI 编辑器
 *
 * 三栏布局：
 *   左侧 ComponentPalette  — 组件面板，按分类展示可拖拽组件
 *   中间 VisualCanvas      — 画布区域，实时预览 + 拖放目标
 *   右侧 PropertyPanel     — 属性编辑面板
 *
 * 当 schema 为 null 时自动初始化一个空 root Container。
 */

import { cloneSchema, genId, getDefaultProps } from "@/lib/dynamicUI/utils";
import type { DynamicComponentType, UISchema } from "@/types";
import { COMPONENT_REQUIRED_PROPS, VALID_DYNAMIC_COMPONENT_TYPES } from "@/types";
import {
  AppstoreAddOutlined,
  DeleteOutlined,
  FullscreenExitOutlined,
  FullscreenOutlined,
  HolderOutlined,
  MinusCircleOutlined,
  PlusOutlined,
  SettingOutlined,
} from "@ant-design/icons";
import { Button, Collapse, Divider, Empty, Input, Select, Tag, Tooltip } from "antd";
import type { CollapseProps } from "antd/es/collapse";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { DynamicUIRenderer } from "./DynamicUIRenderer";

import {
  closestCenter,
  DndContext,
  type DragEndEvent,
  DragOverlay,
  type DragStartEvent,
  KeyboardSensor,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import { arrayMove, sortableKeyboardCoordinates, useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

// ── Types ──

interface VisualEditorProps {
  /** 当前 schema，可为 null（内部自动初始化空 Container） */
  schema: UISchema | null;
  /** schema 变更回调 */
  onChange: (schema: UISchema) => void;
}

interface PaletteItemData {
  type: DynamicComponentType;
  category: ComponentCategory;
  labelKey: string; // i18n key for the label
  icon?: string;
}

type ComponentCategory = "container" | "data-display" | "form" | "media" | "misc";

/** 拖拽数据类型 */
const DRAG_TYPE_PALETTE = "palette-item";
const DRAG_TYPE_CANVAS = "canvas-item";

// ── 组件面板数据（按分类组织）─────────────────────────────────────────────

const PALETTE_ITEMS: PaletteItemData[] = [
  // Layout / Containers
  { type: "Container", category: "container", labelKey: "visualEditor.components.Container" },
  { type: "Row", category: "container", labelKey: "visualEditor.components.Row" },
  { type: "Column", category: "container", labelKey: "visualEditor.components.Column" },
  { type: "Grid", category: "container", labelKey: "visualEditor.components.Grid" },
  { type: "Card", category: "container", labelKey: "visualEditor.components.Card" },
  { type: "Tabs", category: "container", labelKey: "visualEditor.components.Tabs" },
  { type: "Accordion", category: "container", labelKey: "visualEditor.components.Accordion" },

  // Data Display
  { type: "Table", category: "data-display", labelKey: "visualEditor.components.Table" },
  { type: "Chart", category: "data-display", labelKey: "visualEditor.components.Chart" },
  { type: "Dashboard", category: "data-display", labelKey: "visualEditor.components.Dashboard" },
  { type: "List", category: "data-display", labelKey: "visualEditor.components.List" },
  { type: "Tree", category: "data-display", labelKey: "visualEditor.components.Tree" },
  { type: "Timeline", category: "data-display", labelKey: "visualEditor.components.Timeline" },

  // Form
  { type: "Form", category: "form", labelKey: "visualEditor.components.Form" },
  { type: "Input", category: "form", labelKey: "visualEditor.components.Input" },
  { type: "Textarea", category: "form", labelKey: "visualEditor.components.Textarea" },
  { type: "Number", category: "form", labelKey: "visualEditor.components.Number" },
  { type: "Select", category: "form", labelKey: "visualEditor.components.Select" },
  { type: "DatePicker", category: "form", labelKey: "visualEditor.components.DatePicker" },
  { type: "Switch", category: "form", labelKey: "visualEditor.components.Switch" },
  { type: "Checkbox", category: "form", labelKey: "visualEditor.components.Checkbox" },
  { type: "Radio", category: "form", labelKey: "visualEditor.components.Radio" },

  // Media
  { type: "CodeEditor", category: "media", labelKey: "visualEditor.components.CodeEditor" },
  { type: "FilePreview", category: "media", labelKey: "visualEditor.components.FilePreview" },
  { type: "Markdown", category: "media", labelKey: "visualEditor.components.Markdown" },
  { type: "Image", category: "media", labelKey: "visualEditor.components.Image" },

  // Misc
  { type: "Button", category: "misc", labelKey: "visualEditor.components.Button" },
  { type: "Text", category: "misc", labelKey: "visualEditor.components.Text" },
  { type: "Divider", category: "misc", labelKey: "visualEditor.components.Divider" },
  { type: "Progress", category: "misc", labelKey: "visualEditor.components.Progress" },
  { type: "Tag", category: "misc", labelKey: "visualEditor.components.Tag" },
];

const CATEGORY_ORDER: ComponentCategory[] = ["container", "data-display", "form", "media", "misc"];

const CATEGORY_ICON_MAP: Record<ComponentCategory, string> = {
  container: "📦",
  "data-display": "📊",
  form: "📝",
  media: "🎬",
  misc: "✨",
};

// ── Schema 工具函数（共享逻辑来自 @/lib/dynamicUI/utils）──────────────────────────

function findNodeById(root: UISchema, id: string): UISchema | null {
  if (root.id === id) { return root; }
  if (!root.children) { return null; }
  for (const child of root.children) {
    const found = findNodeById(child, id);
    if (found) { return found; }
  }
  return null;
}

/** 创建空的 root schema */
function createEmptyRoot(): UISchema {
  return {
    version: "1.0",
    id: genId("root"),
    type: "Container",
    props: {},
    children: [],
  };
}

function parsePropValue(v: string): unknown {
  if (v === "true") { return true; }
  if (v === "false") { return false; }
  const num = Number(v);
  if (!Number.isNaN(num) && v.trim() !== "") { return num; }
  if ((v.startsWith("{") && v.endsWith("}")) || (v.startsWith("[") && v.endsWith("]"))) {
    try {
      return JSON.parse(v);
    } catch { /* fall */ }
  }
  return v;
}

// ── 子组件：组件面板 (Palette) ─────────────────────────────────────────────

function DraggablePaletteItem({ item, t }: { item: PaletteItemData; t: (key: string) => string }) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `palette-${item.type}`,
    data: { type: DRAG_TYPE_PALETTE, itemType: item.type, label: t(item.labelKey) },
  });

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      className={`
        flex items-center gap-2 px-3 py-2 mb-1 rounded-md border cursor-grab
        border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700
        hover:border-blue-400 hover:bg-blue-50 dark:hover:border-blue-500 dark:hover:bg-gray-600
        transition-colors select-none
        ${isDragging ? "opacity-50 scale-95" : ""}
      `}
    >
      <span className="text-sm">{CATEGORY_ICON_MAP[item.category]}</span>
      <span className="text-xs font-medium">{t(item.labelKey)}</span>
      <span className="ml-auto text-[10px] text-gray-400 dark:text-gray-500 font-mono">{item.type}</span>
    </div>
  );
}

function ComponentPalette({ t }: { t: (key: string) => string }) {
  const grouped = useMemo(() => {
    const map = new Map<ComponentCategory, PaletteItemData[]>();
    for (const cat of CATEGORY_ORDER) { map.set(cat, []); }
    for (const item of PALETTE_ITEMS) {
      const arr = map.get(item.category);
      if (arr) { arr.push(item); }
    }
    return map;
  }, []);

  const collapseItems: CollapseProps["items"] = useMemo(
    () =>
      CATEGORY_ORDER.map((cat) => ({
        key: cat,
        label: (
          <span className="flex items-center gap-2">
            <span>{CATEGORY_ICON_MAP[cat]}</span>
            <span className="font-medium text-sm">{t(`visualEditor.categories.${cat}`)}</span>
            <span className="ml-auto text-[10px] text-gray-400 bg-gray-100 dark:bg-gray-600 px-1.5 rounded-full">
              {(grouped.get(cat) ?? []).length}
            </span>
          </span>
        ),
        children: (
          <div className="-mx-2">
            {(grouped.get(cat) ?? []).map((item) => <DraggablePaletteItem key={item.type} item={item} t={t} />)}
          </div>
        ),
      })),
    [grouped, t],
  );

  return (
    <div className="h-full overflow-y-auto p-2">
      <Collapse defaultActiveKey={CATEGORY_ORDER} ghost size="small" items={collapseItems} />
    </div>
  );
}

// ── 子组件：画布中的可排序列表项 ─────────────────────────────────────────

interface SortableCanvasItemProps {
  node: UISchema;
  isSelected: boolean;
  onSelect: () => void;
  t: (key: string) => string;
}

function SortableCanvasItem({ node, isSelected, onSelect, t }: SortableCanvasItemProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: node.id,
    data: { type: DRAG_TYPE_CANVAS, node },
  });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
    zIndex: isDragging ? 999 : undefined,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      onClick={(e) => {
        e.stopPropagation();
        onSelect();
      }}
      className={`
        relative group rounded-lg border-2 p-3 cursor-pointer transition-all
        ${
        isSelected
          ? "border-blue-500 ring-2 ring-blue-200 dark:ring-blue-800 bg-blue-50/50 dark:bg-blue-900/20"
          : "border-transparent hover:border-gray-300 dark:hover:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800/60"
      }
      `}
    >
      {/* 排序手柄 */}
      <div
        {...listeners}
        className="absolute left-1 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity"
      >
        <HolderOutlined className="text-gray-400 cursor-grab active:cursor-grabbing text-xs p-1" />
      </div>

      {/* 节点标签 */}
      <div className="flex items-center gap-2 ml-4">
        <Tag color={isSelected ? "blue" : "geekblue"} className="text-[10px] leading-none px-1 py-0 m-0">
          {node.type}
        </Tag>
        <span className="text-xs text-gray-500 dark:text-gray-400 truncate max-w-30 font-mono">
          {node.id}
        </span>

        {/* 叶子节点预览提示 */}
        {!node.children?.length && (
          <Tooltip title={`[${node.type}] ${t("visualEditor.canvas.leafNode")}`}>
            <span className="text-[10px] text-gray-400">◻</span>
          </Tooltip>
        )}
        {/* 容器子节点数 */}
        {node.children && node.children.length > 0 && (
          <Tooltip title={`${node.children.length} ${t("visualEditor.canvas.childCount")}`}>
            <Tag className="text-[9px] leading-none px-1 py-0 m-0" color="processing">
              {node.children.length}
            </Tag>
          </Tooltip>
        )}
      </div>
    </div>
  );
}

// ── 子组件：画布区域 (Canvas) ──────────────────────────────────────────────

function VisualCanvas({
  schema,
  selectedId,
  onSelectNode,
  t,
}: {
  schema: UISchema;
  selectedId: string | null;
  onSelectNode: (id: string) => void;
  t: (key: string) => string;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: "canvas-root-drop",
    data: { type: "canvas-root" },
  });

  const children = schema.children ?? [];

  return (
    <div
      ref={setNodeRef}
      onClick={() => onSelectNode(schema.id)}
      className={`
        flex-1 h-full overflow-y-auto p-4 transition-colors
        ${isOver ? "bg-blue-50/70 dark:bg-blue-900/20" : "bg-white dark:bg-gray-900"}
        border-r border-gray-200 dark:border-gray-700
      `}
    >
      {/* Root 节点信息 */}
      <div className="mb-3 flex items-center gap-2">
        <Tag color="purple" className="m-0">{schema.type}</Tag>
        <span className="text-xs text-gray-400 dark:text-gray-500 font-mono">{schema.id}</span>
        <span className="text-[11px] text-gray-400 dark:text-gray-500 ml-auto">
          {children.length === 0
            ? t("visualEditor.canvas.emptyHint")
            : `${children.length} ${t("visualEditor.canvas.rootHint")}`}
        </span>
      </div>

      {/* 空状态 */}
      {children.length === 0 && (
        <div
          className={`
          flex flex-col items-center justify-center h-75 rounded-lg border-2 border-dashed
          ${isOver ? "border-blue-500 bg-blue-50 dark:bg-blue-900/30" : "border-gray-300 dark:border-gray-600"}
          transition-colors
        `}
        >
          <AppstoreAddOutlined className="text-4xl mb-3 text-gray-300 dark:text-gray-600" />
          <p className="text-sm text-gray-400 dark:text-gray-500 mb-1">{t("visualEditor.canvas.dropPrompt")}</p>
          <p className="text-xs text-gray-300 dark:text-gray-600">{t("visualEditor.canvas.dropHint")}</p>
        </div>
      )}

      {/* 子节点列表（可排序） */}
      <div className="space-y-2 mb-4">
        {children.map((child) => (
          <SortableCanvasItem
            key={child.id}
            node={child}
            isSelected={selectedId === child.id}
            onSelect={() => onSelectNode(child.id)}
            t={t}
          />
        ))}
      </div>

      {/* 实时可视化预览 — 渲染实际组件效果 */}
      {children.length > 0 && (
        <div
          className="border-t border-gray-200 dark:border-gray-700 pt-3 mt-auto"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="text-[11px] text-gray-400 mb-2">
            {t("visualEditor.canvas.livePreview")}
          </div>
          <div className="border rounded-lg p-3 bg-white dark:bg-gray-800 min-h-30 overflow-auto">
            <DynamicUIRenderer schema={schema} />
          </div>
        </div>
      )}
    </div>
  );
}

// ── 子组件：属性面板 (PropertyPanel) ───────────────────────────────────────

function PropertyPanel({
  selectedNode,
  isRoot,
  onUpdateNode,
  defaultPropsMap,
  t,
}: {
  selectedNode: UISchema | null;
  isRoot: boolean;
  onUpdateNode: (updater: (node: UISchema) => void) => void;
  defaultPropsMap: Partial<Record<DynamicComponentType, Record<string, unknown>>>;
  t: (key: string) => string;
}) {
  const [activeTab, setActiveTab] = useState<"basic" | "style" | "events" | "advanced">("basic");

  if (!selectedNode) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-gray-400 dark:text-gray-500 p-4">
        <SettingOutlined className="text-3xl mb-2 opacity-40" />
        <p className="text-xs text-center">{t("visualEditor.property.selectHint")}</p>
      </div>
    );
  }

  const propEntries = Object.entries(selectedNode.props).map(([key, value]) => ({
    key,
    value: typeof value === "string" ? value : JSON.stringify(value),
  }));

  const requiredProps = COMPONENT_REQUIRED_PROPS[selectedNode.type as DynamicComponentType] ?? [];

  const handlePropChange = (index: number, field: "key" | "value", newVal: string) => {
    onUpdateNode((node) => {
      const entries = Object.entries(node.props);
      if (field === "key") {
        entries[index][0] = newVal;
      } else {
        entries[index][1] = parsePropValue(newVal);
      }
      node.props = Object.fromEntries(entries);
    });
  };

  const handleAddProp = () => {
    onUpdateNode((node) => {
      node.props = { ...node.props, "": "" };
    });
  };

  const handleRemoveProp = (index: number) => {
    onUpdateNode((node) => {
      const entries = Object.entries(node.props);
      entries.splice(index, 1);
      node.props = Object.fromEntries(entries);
    });
  };

  const handleTypeChange = (newType: DynamicComponentType) => {
    onUpdateNode((node) => {
      node.type = newType;
      node.props = { ...defaultPropsMap[newType], ...node.props };
    });
  };

  const handleStyleChange = (key: string, val: string) => {
    onUpdateNode((node) => {
      if (!node.style) { node.style = {}; }
      if (val === "") {
        delete node.style[key];
      } else {
        node.style[key] = /^\d+$/.test(val) ? Number(val) : val;
      }
      if (Object.keys(node.style).length === 0) { node.style = undefined; }
    });
  };

  const styleEntries = selectedNode.style ? Object.entries(selectedNode.style) : [];

  const TABS = [
    { key: "basic" as const, label: t("visualEditor.property.tabs.basic"), icon: "⚙️" },
    { key: "style" as const, label: t("visualEditor.property.tabs.style"), icon: "🎨" },
  ];

  return (
    <div className="h-full overflow-y-auto p-3 bg-gray-50 dark:bg-gray-800 space-y-4">
      {/* Tab 切换 */}
      <div className="flex gap-1">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`
              flex-1 text-xs px-2 py-1 rounded transition-all
              ${
              activeTab === tab.key
                ? "bg-blue-500 text-white shadow-sm"
                : "bg-gray-200 dark:bg-gray-600 text-gray-600 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-500"
            }
            `}
          >
            {tab.icon} {tab.label}
          </button>
        ))}
      </div>

      {activeTab === "basic" && (
        <>
          <div>
            <label className="block text-xs text-gray-500 mb-1">{t("dynamicUIManager.componentType")}</label>
            <Select
              value={selectedNode.type}
              onChange={handleTypeChange}
              options={[...VALID_DYNAMIC_COMPONENT_TYPES].sort().map((ct) => ({ label: ct, value: ct }))}
              className="w-full"
              size="small"
              disabled={isRoot}
            />
          </div>

          <div className="flex gap-2">
            <div className="flex-1">
              <label className="block text-xs text-gray-500 mb-1">ID</label>
              <Input size="small" value={selectedNode.id} disabled />
            </div>
            <div className="flex-1">
              <label className="block text-xs text-gray-500 mb-1">Version</label>
              <Input size="small" value={selectedNode.version} disabled />
            </div>
          </div>

          <Divider className="my-2" />

          {/* Props 编辑器 */}
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs text-gray-500">Props</label>
              <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={handleAddProp}>
                {t("dynamicUIManager.addProp")}
              </Button>
            </div>

            {requiredProps.length > 0 && (
              <div className="mb-2">
                <span className="text-[11px] text-orange-500">
                  {t("dynamicUIManager.requiredProps")}: {requiredProps.join(", ")}
                </span>
              </div>
            )}

            <div className="space-y-1 max-h-50 overflow-auto">
              {propEntries.length === 0 && (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={t("dynamicUIManager.noProps")}
                  className="my-2"
                />
              )}
              {propEntries.map((entry, idx) => (
                <div key={`${entry.key}-${idx}`} className="flex gap-1">
                  <Input
                    size="small"
                    className="w-[38%] font-mono text-[11px]"
                    placeholder={t("visualEditor.property.placeholder.propKey")}
                    value={entry.key}
                    onChange={(e) => handlePropChange(idx, "key", e.target.value)}
                    status={requiredProps.includes(entry.key) && !entry.value ? "warning" : undefined}
                  />
                  <Input
                    size="small"
                    className="flex-1 font-mono text-[11px]"
                    placeholder={t("visualEditor.property.placeholder.propValue")}
                    value={entry.value}
                    onChange={(e) => handlePropChange(idx, "value", e.target.value)}
                  />
                  <Button
                    size="small"
                    type="text"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={() => handleRemoveProp(idx)}
                  />
                </div>
              ))}
            </div>
          </div>
        </>
      )}

      {activeTab === "style" && (
        <>
          <div>
            <label className="text-xs text-gray-500 mb-1 block">{t("visualEditor.property.style.label")}</label>
            <div className="space-y-1 max-h-50 overflow-auto">
              {styleEntries.length === 0 && (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={t("visualEditor.property.style.empty")}
                  className="my-2"
                />
              )}
              {styleEntries.map(([key, val], idx) => (
                <div key={idx} className="flex gap-1">
                  <Input
                    size="small"
                    className="w-[38%] font-mono text-[11px]"
                    placeholder={t("visualEditor.property.style.placeholder.propKey")}
                    value={key}
                    onChange={(e) => {
                      const newKey = e.target.value;
                      if (key !== "" && newKey !== key) {
                        // 删除旧 key，添加新 key
                        handleStyleChange(key, "");
                        if (newKey) { handleStyleChange(newKey, String(val)); }
                      }
                    }}
                  />
                  <Input
                    size="small"
                    className="flex-1 font-mono text-[11px]"
                    placeholder={t("visualEditor.property.style.placeholder.propValue")}
                    value={String(val)}
                    onChange={(e) => handleStyleChange(key, e.target.value)}
                  />
                  <Button
                    size="small"
                    type="text"
                    danger
                    icon={<MinusCircleOutlined />}
                    onClick={() => handleStyleChange(key, "")}
                  />
                </div>
              ))}
            </div>
            <Button
              size="small"
              type="dashed"
              icon={<PlusOutlined />}
              onClick={() => handleStyleChange(`_new_${Date.now()}`, "")}
              className="mt-2 w-full"
            >
              {t("visualEditor.property.style.add")}
            </Button>
          </div>

          {/* 快捷样式 */}
          <Divider className="my-2" />
          <div className="space-y-2">
            <label className="block text-xs text-gray-500">{t("visualEditor.property.style.quickStyles")}</label>
            {(["padding", "margin", "backgroundColor", "color"] as const).map((prop) => (
              <div key={prop} className="flex gap-2 items-center">
                <span className="text-[11px] text-gray-500 w-22.5 font-mono shrink-0">{prop}</span>
                <Input
                  size="small"
                  className="flex-1 font-mono text-[11px]"
                  placeholder="--"
                  value={selectedNode.style ? String(selectedNode.style[prop] ?? "") : ""}
                  onChange={(e) => {
                    const v = e.target.value.trim();
                    if (v) {
                      handleStyleChange(prop, v);
                    } else if (selectedNode.style?.[prop]) {
                      handleStyleChange(prop, "");
                    }
                  }}
                />
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

export function VisualEditor({ schema: propSchema, onChange }: VisualEditorProps) {
  const { t } = useTranslation();
  const editorRef = useRef<HTMLDivElement>(null);

  // 内部 schema（null 时自动创建 root）
  const [internalSchema, setInternalSchema] = useState<UISchema>(() =>
    propSchema ? cloneSchema(propSchema) : createEmptyRoot()
  );

  // 选中的节点 ID
  const [selectedId, setSelectedId] = useState<string | null>(null);
  // 全屏模式
  const [isFullscreen, setIsFullscreen] = useState(false);
  // Escape 键退出全屏
  useEffect(() => {
    if (!isFullscreen) { return; }
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        setIsFullscreen(false);
      }
    };
    // 使用 capture 确保在 Antd Modal 的 Escape 处理之前拦截
    window.addEventListener("keydown", handler, { capture: true });
    return () => window.removeEventListener("keydown", handler, { capture: true });
  }, [isFullscreen]);

  // 默认 props 映射
  const defaultPropsMap = useMemo(() => getDefaultProps(t), [t]);

  // 当前选中的节点
  const selectedNode = useMemo(
    () => (selectedId ? findNodeById(internalSchema, selectedId) : internalSchema),
    [internalSchema, selectedId],
  );

  const isRootSelected = selectedId === null || selectedId === internalSchema.id;

  // ── dnd-kit sensors ──
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  // ── 拖拽状态 ──
  const [activeDragId, setActiveDragId] = useState<string | null>(null);

  const handleDragStart = useCallback((event: DragStartEvent) => {
    setActiveDragId(String(event.active.id));
  }, []);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      setActiveDragId(null);

      if (!over) { return; }

      const activeData = active.data.current;
      const overData = over.data.current;

      if (!activeData || !overData) { return; }

      // 从组件面板拖入画布（根容器 或 任何子节点）
      if (
        activeData.type === DRAG_TYPE_PALETTE && (overData.type === "canvas-root" || overData.type === DRAG_TYPE_CANVAS)
      ) {
        const itemType = activeData.itemType as DynamicComponentType;
        setInternalSchema((prev) => {
          const next = cloneSchema(prev);
          const newChild: UISchema = {
            version: "1.0",
            id: genId(itemType.toLowerCase()),
            type: itemType,
            props: { ...defaultPropsMap[itemType] },
          };
          next.children = [...(next.children ?? []), newChild];
          return next;
        });
        return;
      }

      // 画布内重排序
      if (activeData.type === DRAG_TYPE_CANVAS && overData.type === "canvas-root") {
        const oldIndex = internalSchema.children?.findIndex((c) => c.id === active.id) ?? -1;
        const newIndex = internalSchema.children?.findIndex((c) => c.id === over.id) ?? -1;
        if (oldIndex >= 0 && newIndex >= 0 && oldIndex !== newIndex) {
          setInternalSchema((prev) => {
            const next = cloneSchema(prev);
            next.children = arrayMove(next.children ?? [], oldIndex, newIndex);
            return next;
          });
        }
        return;
      }
    },
    [defaultPropsMap, internalSchema.children],
  );

  // ── 操作方法 ──
  const handleSelectNode = useCallback((id: string) => {
    setSelectedId(id);
  }, []);

  const handleUpdateSelectedNode = useCallback(
    (updater: (node: UISchema) => void) => {
      setInternalSchema((prev) => {
        const next = cloneSchema(prev);
        let target: UISchema;
        if (!selectedId || selectedId === prev.id) {
          target = next;
        } else {
          target = findNodeById(next, selectedId)!;
          if (!target) { return prev; }
        }
        updater(target);
        onChange(next);
        return next;
      });
    },
    [onChange, selectedId],
  );

  return (
    <div
      ref={editorRef}
      className={`relative flex border rounded-lg overflow-hidden transition-all duration-300 ${
        isFullscreen ? "fixed inset-0 z-50 bg-white dark:bg-gray-900 h-auto w-auto rounded-none border-0 p-4" : "h-130"
      }`}
    >
      {/* 全屏切换按钮 */}
      <button
        type="button"
        onClick={() => setIsFullscreen((v) => !v)}
        className="absolute top-2 right-2 z-20 flex items-center gap-1 text-xs px-2.5 py-1.5 rounded
                   bg-white/90 dark:bg-gray-700/90 hover:bg-gray-100 dark:hover:bg-gray-600
                   border border-gray-300 dark:border-gray-500 shadow-md transition-colors"
      >
        {isFullscreen ? <FullscreenExitOutlined /> : <FullscreenOutlined />}
        <span>
          {isFullscreen ? t("visualEditor.exitFullscreen") : t("visualEditor.fullscreen")}
        </span>
      </button>

      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div
          ref={editorRef}
          className={`flex border rounded-lg overflow-hidden transition-all duration-300 ${
            isFullscreen
              ? "fixed inset-0 z-50 bg-white dark:bg-gray-900 h-auto w-auto rounded-none border-0 p-4"
              : "h-130"
          }`}
        >
          {/* 左侧：组件面板 */}
          <div className="w-50 border-r border-gray-200 dark:border-gray-700 shrink-0 bg-gray-50 dark:bg-gray-800">
            <div className="px-3 py-2 border-b border-gray-200 dark:border-gray-700">
              <h4 className="text-xs font-semibold text-gray-600 dark:text-gray-300 flex items-center gap-1.5">
                <AppstoreAddOutlined size={12} />
                {t("visualEditor.palette.title")}
              </h4>
            </div>
            <ComponentPalette t={t} />
          </div>

          {/* 中间：画布 */}
          <VisualCanvas
            schema={internalSchema}
            selectedId={isRootSelected ? null : selectedId}
            onSelectNode={handleSelectNode}
            t={t}
          />

          {/* 右侧：属性面板 */}
          <div className="w-65 border-l border-gray-200 dark:border-gray-700 shrink-0 overflow-hidden">
            <div className="px-3 py-2 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
              <h4 className="text-xs font-semibold text-gray-600 dark:text-gray-300 flex items-center gap-1.5">
                <SettingOutlined size={12} />
                {t("visualEditor.property.title")}
              </h4>
              {selectedNode && !isRootSelected && (
                <Button
                  size="small"
                  type="text"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={() => {
                    setInternalSchema((prev) => {
                      const next = cloneSchema(prev);
                      if (next.children) {
                        next.children = next.children.filter((c) => c.id !== selectedId);
                        if (selectedId) { setSelectedId(null); }
                      }
                      onChange(next);
                      return next;
                    });
                  }}
                >
                  {t("visualEditor.property.delete")}
                </Button>
              )}
            </div>
            <PropertyPanel
              selectedNode={selectedNode ?? null}
              isRoot={isRootSelected}
              onUpdateNode={handleUpdateSelectedNode}
              defaultPropsMap={defaultPropsMap}
              t={t}
            />
          </div>
        </div>

        {/* Drag Overlay */}
        <DragOverlay>
          {activeDragId?.startsWith("palette-")
            ? (
              <div className="bg-blue-500 text-white text-xs px-3 py-2 rounded shadow-lg flex items-center gap-2">
                <AppstoreAddOutlined />
                {activeDragId.replace("palette-", "")}
              </div>
            )
            : activeDragId
            ? (
              <div className="bg-gray-700 dark:bg-gray-600 text-white text-xs px-3 py-2 rounded shadow-lg">
                <HolderOutlined className="mr-1" />
                {activeDragId}
              </div>
            )
            : null}
        </DragOverlay>
      </DndContext>
    </div>
  );
}
