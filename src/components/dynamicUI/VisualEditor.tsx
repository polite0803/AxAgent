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

import {
  appendChildToNode,
  cloneSchema,
  cloneSchemaWithNewIds,
  detachNodeById,
  findNodeById,
  findParent,
  flattenNodes,
  genId,
  getDefaultProps,
  insertBeforeSibling,
  isDescendant,
  removeNodeById,
  updateNodeById,
} from "@/lib/dynamicUI/utils";
import type {
  ConditionalDisplay,
  ConditionalRule,
  DataSourceConfig,
  DynamicAction,
  DynamicComponentType,
  EventHandler,
  UISchema,
} from "@/types";
import { VALID_DYNAMIC_COMPONENT_TYPES } from "@/types";
import {
  AppstoreAddOutlined,
  CopyOutlined,
  DeleteOutlined,
  EditOutlined,
  ExportOutlined,
  EyeOutlined,
  FullscreenExitOutlined,
  FullscreenOutlined,
  HolderOutlined,
  ImportOutlined,
  MinusCircleOutlined,
  PlusOutlined,
  RedoOutlined,
  ScissorOutlined,
  SettingOutlined,
  SnippetsOutlined,
  UndoOutlined,
} from "@ant-design/icons";
import { App, Button, Collapse, Divider, Empty, Input, Select, Switch, Tag, Tooltip, Upload } from "antd";
import type { UploadProps } from "antd";
import type { CollapseProps } from "antd/es/collapse";
import * as React from "react";
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
  /**
   * 持久化 key。传入时启用 localStorage 自动保存 + 启动恢复：
   *   - 启动时优先从 localStorage 读取，读不到则用 propSchema
   *   - schema 变更后 debounce 800ms 写入 localStorage
   * 省略时仅内存态（与原行为一致）
   */
  persistenceKey?: string;
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

/** 响应式视口类型 */
type Viewport = "desktop" | "tablet" | "mobile";

/** 视口宽度映射（px） */
const VIEWPORT_WIDTH: Record<Viewport, number> = {
  desktop: 1280,
  tablet: 768,
  mobile: 375,
};

/** 容器类型集合 — 这些组件可接收拖入的子节点 */
const CONTAINER_TYPES = new Set<DynamicComponentType>([
  "Container",
  "Row",
  "Column",
  "Grid",
  "Card",
  "Tabs",
  "Accordion",
  "Form",
]);

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

// ── Schema 工具函数（来自 @/lib/dynamicUI/utils，此处仅留本地辅助）──────────────────────────

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

/**
 * 构建编辑器预览用的 mock dataContext。
 * 遍历 schema 树，对每个有 dataSource 的节点注入一条占位数据，
 * 让 Table/Chart/List 等数据组件在编辑器里也能看到效果（而非空渲染）。
 *
 * - static 类型：尝试返回 config.value，否则返回占位数组
 * - store/api/agent-generated：统一返回示例数据，避免编辑时真正发起请求
 */
function buildPreviewDataContext(root: UISchema): Record<string, unknown> {
  const ctx: Record<string, unknown> = {};
  const walk = (node: UISchema): void => {
    if (node.dataSource) {
      const ds = node.dataSource;
      let mock: unknown;
      if (ds.type === "static") {
        const val = (ds.config as Record<string, unknown>).value;
        mock = Array.isArray(val) ? val : [val];
      } else {
        // store/api/agent-generated — 编辑时不真实拉取，给一行示例
        mock = [{ id: 1, name: "Sample", value: 100 }];
      }
      ctx[node.id] = mock;
    }
    if (node.children) {
      for (const child of node.children) { walk(child); }
    }
  };
  walk(root);
  return ctx;
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

// ── 子组件：画布中的可排序列表项（支持递归嵌套容器） ────────────────────

interface SortableCanvasItemProps {
  node: UISchema;
  isSelected: boolean;
  onSelectNode: (id: string) => void;
  selectedId: string | null;
  t: (key: string) => string;
  /** 嵌套深度，用于缩进显示 */
  depth?: number;
}

function SortableCanvasItem({
  node,
  isSelected,
  onSelectNode,
  selectedId,
  t,
  depth = 0,
}: SortableCanvasItemProps) {
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

  // 容器节点自身挂 droppable，用于接收拖入子节点
  const isContainer = CONTAINER_TYPES.has(node.type);
  const { setNodeRef: dropRef, isOver } = useDroppable({
    id: `drop-${node.id}`,
    data: { type: "container-drop", nodeId: node.id },
  });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
    zIndex: isDragging ? 999 : undefined,
  };

  const children = node.children ?? [];
  const hasChildren = children.length > 0;

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      onClick={(e) => {
        e.stopPropagation();
        onSelectNode(node.id);
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
        {!hasChildren && (
          <Tooltip title={`[${node.type}] ${t("visualEditor.canvas.leafNode")}`}>
            <span className="text-[10px] text-gray-400">◻</span>
          </Tooltip>
        )}
        {/* 容器子节点数 */}
        {hasChildren && (
          <Tooltip title={`${children.length} ${t("visualEditor.canvas.childCount")}`}>
            <Tag className="text-[9px] leading-none px-1 py-0 m-0" color="processing">
              {children.length}
            </Tag>
          </Tooltip>
        )}
      </div>

      {/* 容器节点的嵌套子节点 + 拖入区域 */}
      {isContainer && (
        <div
          ref={dropRef}
          className={`
            mt-2 ml-3 pl-3 border-l-2 border-dashed rounded-md transition-colors
            ${
            isOver
              ? "border-blue-500 bg-blue-50/70 dark:bg-blue-900/20"
              : "border-gray-200 dark:border-gray-600 bg-gray-50/30 dark:bg-gray-800/30"
          }
          `}
        >
          <div className="text-[10px] text-gray-400 dark:text-gray-500 py-1 px-2 select-none">
            {isOver ? t("visualEditor.canvas.dropHere") : t("visualEditor.canvas.childArea")}
          </div>
          <div className="space-y-2 pb-2">
            {children.map((child) => (
              <SortableCanvasItem
                key={child.id}
                node={child}
                isSelected={selectedId === child.id}
                onSelectNode={onSelectNode}
                selectedId={selectedId}
                t={t}
                depth={depth + 1}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ── 子组件：画布区域 (Canvas) ──────────────────────────────────────────────

function VisualCanvas({
  schema,
  selectedId,
  onSelectNode,
  isPreviewMode,
  viewport,
  onAction,
  t,
}: {
  schema: UISchema;
  selectedId: string | null;
  onSelectNode: (id: string) => void;
  isPreviewMode: boolean;
  viewport: Viewport;
  onAction?: (action: DynamicAction) => void;
  t: (key: string) => string;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: "canvas-root-drop",
    data: { type: "canvas-root" },
    disabled: isPreviewMode,
  });

  const children = schema.children ?? [];

  // 预览模式：只渲染真实效果，不显示结构/选中/拖拽
  // 根据视口宽度限制画布宽，模拟不同设备尺寸
  if (isPreviewMode) {
    const previewWidth = VIEWPORT_WIDTH[viewport];
    return (
      <div className="flex-1 h-full overflow-y-auto p-4 bg-gray-100 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 flex flex-col items-center">
        {/* 视口宽度标识 */}
        <div className="text-[10px] text-gray-400 dark:text-gray-500 mb-2 font-mono">
          {viewport.toUpperCase()} · {previewWidth}px
        </div>

        {children.length === 0
          ? (
            <div className="flex flex-col items-center justify-center h-full text-gray-400 dark:text-gray-500">
              <EyeOutlined className="text-4xl mb-3 opacity-40" />
              <p className="text-sm">{t("visualEditor.canvas.previewEmpty")}</p>
            </div>
          )
          : (
            <div
              className="border rounded-lg p-3 bg-white dark:bg-gray-800 min-h-50 shadow-md transition-all"
              style={{ width: `${previewWidth}px`, maxWidth: "100%" }}
            >
              <DynamicUIRenderer
                schema={schema}
                dataContext={buildPreviewDataContext(schema)}
                onAction={onAction}
              />
            </div>
          )}
      </div>
    );
  }

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

      {/* 子节点列表（可排序 + 支持嵌套容器） */}
      <div className="space-y-2 mb-4">
        {children.map((child) => (
          <SortableCanvasItem
            key={child.id}
            node={child}
            isSelected={selectedId === child.id}
            onSelectNode={onSelectNode}
            selectedId={selectedId}
            t={t}
          />
        ))}
      </div>

      {/* 实时可视化预览 — 渲染实际组件效果，注入 mock dataContext 让数据组件可见 */}
      {children.length > 0 && (
        <div
          className="border-t border-gray-200 dark:border-gray-700 pt-3 mt-auto"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="text-[11px] text-gray-400 mb-2">
            {t("visualEditor.canvas.livePreview")}
          </div>
          <div className="border rounded-lg p-3 bg-white dark:bg-gray-800 min-h-30 overflow-auto">
            <DynamicUIRenderer schema={schema} dataContext={buildPreviewDataContext(schema)} />
          </div>
        </div>
      )}
    </div>
  );
}

// ── 子组件：属性面板 (PropertyPanel) ───────────────────────────────────────

/**
 * 每个组件类型的专属 props 字段配置。
 * 渲染时按 control 类型生成对应表单控件，避免用户手写 JSON 字符串。
 */
type PropControlType =
  | "text"
  | "textarea"
  | "number"
  | "select"
  | "switch"
  | "options"
  | "json";

interface PropFieldDef {
  prop: string;
  labelKey: string;
  control: PropControlType;
  options?: Array<{ labelKey: string; value: string }>;
  placeholderKey?: string;
}

const PROP_FIELD_CONFIG: Partial<Record<DynamicComponentType, PropFieldDef[]>> = {
  Input: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "placeholder", labelKey: "visualEditor.property.placeholderLabel", control: "text" },
    {
      prop: "type",
      labelKey: "visualEditor.property.type",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.inputType.text", value: "text" },
        { labelKey: "visualEditor.property.inputType.password", value: "password" },
        { labelKey: "visualEditor.property.inputType.textarea", value: "textarea" },
      ],
    },
    { prop: "required", labelKey: "visualEditor.property.required", control: "switch" },
    { prop: "maxLength", labelKey: "visualEditor.property.maxLength", control: "number" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Textarea: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "placeholder", labelKey: "visualEditor.property.placeholderLabel", control: "text" },
    { prop: "rows", labelKey: "visualEditor.property.rows", control: "number" },
    { prop: "required", labelKey: "visualEditor.property.required", control: "switch" },
    { prop: "maxLength", labelKey: "visualEditor.property.maxLength", control: "number" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Number: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "placeholder", labelKey: "visualEditor.property.placeholderLabel", control: "text" },
    { prop: "min", labelKey: "visualEditor.property.min", control: "number" },
    { prop: "max", labelKey: "visualEditor.property.max", control: "number" },
    { prop: "step", labelKey: "visualEditor.property.step", control: "number" },
    { prop: "required", labelKey: "visualEditor.property.required", control: "switch" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Select: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "placeholder", labelKey: "visualEditor.property.placeholderLabel", control: "text" },
    {
      prop: "mode",
      labelKey: "visualEditor.property.selectModeLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.selectMode.default", value: "default" },
        { labelKey: "visualEditor.property.selectMode.multiple", value: "multiple" },
        { labelKey: "visualEditor.property.selectMode.tags", value: "tags" },
      ],
    },
    { prop: "options", labelKey: "visualEditor.property.options", control: "options" },
    { prop: "required", labelKey: "visualEditor.property.required", control: "switch" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Radio: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "options", labelKey: "visualEditor.property.options", control: "options" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Checkbox: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "options", labelKey: "visualEditor.property.options", control: "options" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Switch: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "defaultChecked", labelKey: "visualEditor.property.defaultChecked", control: "switch" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  DatePicker: [
    { prop: "name", labelKey: "visualEditor.property.name", control: "text" },
    { prop: "label", labelKey: "visualEditor.property.label", control: "text" },
    { prop: "format", labelKey: "visualEditor.property.format", control: "text" },
    { prop: "showTime", labelKey: "visualEditor.property.showTime", control: "switch" },
    { prop: "required", labelKey: "visualEditor.property.required", control: "switch" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Button: [
    { prop: "text", labelKey: "visualEditor.property.text", control: "text" },
    {
      prop: "type",
      labelKey: "visualEditor.property.type",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.btnType.default", value: "default" },
        { labelKey: "visualEditor.property.btnType.primary", value: "primary" },
        { labelKey: "visualEditor.property.btnType.dashed", value: "dashed" },
        { labelKey: "visualEditor.property.btnType.link", value: "link" },
      ],
    },
    { prop: "icon", labelKey: "visualEditor.property.icon", control: "text" },
    { prop: "disabled", labelKey: "visualEditor.property.disabled", control: "switch" },
  ],
  Text: [
    { prop: "content", labelKey: "visualEditor.property.content", control: "textarea" },
    { prop: "strong", labelKey: "visualEditor.property.strong", control: "switch" },
  ],
  Tag: [
    { prop: "text", labelKey: "visualEditor.property.text", control: "text" },
    {
      prop: "color",
      labelKey: "visualEditor.property.colorLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.color.blue", value: "blue" },
        { labelKey: "visualEditor.property.color.green", value: "green" },
        { labelKey: "visualEditor.property.color.orange", value: "orange" },
        { labelKey: "visualEditor.property.color.red", value: "red" },
        { labelKey: "visualEditor.property.color.purple", value: "purple" },
      ],
    },
  ],
  Progress: [
    { prop: "percent", labelKey: "visualEditor.property.percent", control: "number" },
    {
      prop: "type",
      labelKey: "visualEditor.property.type",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.progressType.line", value: "line" },
        { labelKey: "visualEditor.property.progressType.circle", value: "circle" },
        { labelKey: "visualEditor.property.progressType.dashboard", value: "dashboard" },
      ],
    },
  ],
  Image: [
    { prop: "src", labelKey: "visualEditor.property.src", control: "text" },
    { prop: "alt", labelKey: "visualEditor.property.alt", control: "text" },
    { prop: "preview", labelKey: "visualEditor.property.preview", control: "switch" },
    { prop: "width", labelKey: "visualEditor.property.width", control: "text" },
    { prop: "height", labelKey: "visualEditor.property.height", control: "text" },
  ],
  Markdown: [
    { prop: "content", labelKey: "visualEditor.property.content", control: "textarea" },
  ],
  CodeEditor: [
    {
      prop: "language",
      labelKey: "visualEditor.property.language",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.lang.plaintext", value: "plaintext" },
        { labelKey: "visualEditor.property.lang.javascript", value: "javascript" },
        { labelKey: "visualEditor.property.lang.typescript", value: "typescript" },
        { labelKey: "visualEditor.property.lang.json", value: "json" },
        { labelKey: "visualEditor.property.lang.rust", value: "rust" },
        { labelKey: "visualEditor.property.lang.python", value: "python" },
        { labelKey: "visualEditor.property.lang.markdown", value: "markdown" },
      ],
    },
    { prop: "readOnly", labelKey: "visualEditor.property.readOnly", control: "switch" },
    { prop: "height", labelKey: "visualEditor.property.height", control: "text" },
  ],
  Table: [
    { prop: "columns", labelKey: "visualEditor.property.columns", control: "json" },
    { prop: "showHeader", labelKey: "visualEditor.property.showHeader", control: "switch" },
    {
      prop: "size",
      labelKey: "visualEditor.property.sizeLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.size.small", value: "small" },
        { labelKey: "visualEditor.property.size.middle", value: "middle" },
        { labelKey: "visualEditor.property.size.large", value: "large" },
      ],
    },
    { prop: "dataSource", labelKey: "visualEditor.property.data", control: "json" },
  ],
  Chart: [
    {
      prop: "chartType",
      labelKey: "visualEditor.property.chartTypeLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.chartType.line", value: "line" },
        { labelKey: "visualEditor.property.chartType.bar", value: "bar" },
        { labelKey: "visualEditor.property.chartType.pie", value: "pie" },
        { labelKey: "visualEditor.property.chartType.scatter", value: "scatter" },
        { labelKey: "visualEditor.property.chartType.area", value: "area" },
      ],
    },
    { prop: "xKey", labelKey: "visualEditor.property.xKey", control: "text" },
    { prop: "yKey", labelKey: "visualEditor.property.yKey", control: "text" },
    { prop: "seriesKey", labelKey: "visualEditor.property.seriesKey", control: "text" },
    { prop: "data", labelKey: "visualEditor.property.data", control: "json" },
  ],
  Dashboard: [
    { prop: "columns", labelKey: "visualEditor.property.columns", control: "number" },
    { prop: "gap", labelKey: "visualEditor.property.gap", control: "number" },
    { prop: "items", labelKey: "visualEditor.property.items", control: "json" },
  ],
  List: [
    {
      prop: "itemLayout",
      labelKey: "visualEditor.property.itemLayoutLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.itemLayout.vertical", value: "vertical" },
        { labelKey: "visualEditor.property.itemLayout.horizontal", value: "horizontal" },
      ],
    },
    {
      prop: "size",
      labelKey: "visualEditor.property.sizeLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.size.small", value: "small" },
        { labelKey: "visualEditor.property.size.default", value: "default" },
        { labelKey: "visualEditor.property.size.large", value: "large" },
      ],
    },
    { prop: "dataSource", labelKey: "visualEditor.property.data", control: "json" },
  ],
  Tree: [
    { prop: "checkable", labelKey: "visualEditor.property.checkable", control: "switch" },
    { prop: "showLine", labelKey: "visualEditor.property.showLine", control: "switch" },
    { prop: "treeData", labelKey: "visualEditor.property.treeData", control: "json" },
  ],
  Timeline: [
    { prop: "items", labelKey: "visualEditor.property.items", control: "json" },
  ],
  Grid: [
    { prop: "columns", labelKey: "visualEditor.property.columns", control: "number" },
    { prop: "gap", labelKey: "visualEditor.property.gap", control: "number" },
  ],
  Row: [
    { prop: "gap", labelKey: "visualEditor.property.gap", control: "number" },
    {
      prop: "align",
      labelKey: "visualEditor.property.alignLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.align.start", value: "start" },
        { labelKey: "visualEditor.property.align.center", value: "center" },
        { labelKey: "visualEditor.property.align.end", value: "end" },
      ],
    },
  ],
  Column: [
    { prop: "gap", labelKey: "visualEditor.property.gap", control: "number" },
  ],
  Card: [
    { prop: "title", labelKey: "visualEditor.property.title", control: "text" },
    { prop: "bordered", labelKey: "visualEditor.property.bordered", control: "switch" },
  ],
  Tabs: [
    {
      prop: "tabPosition",
      labelKey: "visualEditor.property.tabPositionLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.tabPosition.top", value: "top" },
        { labelKey: "visualEditor.property.tabPosition.bottom", value: "bottom" },
        { labelKey: "visualEditor.property.tabPosition.left", value: "left" },
        { labelKey: "visualEditor.property.tabPosition.right", value: "right" },
      ],
    },
  ],
  Accordion: [
    { prop: "accordion", labelKey: "visualEditor.property.accordion", control: "switch" },
  ],
  Form: [
    {
      prop: "layout",
      labelKey: "visualEditor.property.layoutLabel",
      control: "select",
      options: [
        { labelKey: "visualEditor.property.layout.vertical", value: "vertical" },
        { labelKey: "visualEditor.property.layout.horizontal", value: "horizontal" },
        { labelKey: "visualEditor.property.layout.inline", value: "inline" },
      ],
    },
    { prop: "submitText", labelKey: "visualEditor.property.submitText", control: "text" },
  ],
};

/** 把任意值转成输入框可显示的字符串 */
function propToString(v: unknown): string {
  if (v === undefined || v === null) { return ""; }
  if (typeof v === "string") { return v; }
  if (typeof v === "number" || typeof v === "boolean") { return String(v); }
  return JSON.stringify(v);
}

/** 单个专属字段渲染器 */
function PropFieldEditor({
  def,
  value,
  onChange,
  t,
}: {
  def: PropFieldDef;
  value: unknown;
  onChange: (v: unknown) => void;
  t: (key: string) => string;
}) {
  const label = <label className="block text-xs text-gray-500 mb-1">{t(def.labelKey)}</label>;

  switch (def.control) {
    case "text":
      return (
        <div>
          {label}
          <Input
            size="small"
            className="w-full"
            value={typeof value === "string" ? value : propToString(value)}
            placeholder={def.placeholderKey ? t(def.placeholderKey) : undefined}
            onChange={(e) => onChange(e.target.value)}
          />
        </div>
      );
    case "textarea":
      return (
        <div>
          {label}
          <Input.TextArea
            size="small"
            className="w-full"
            rows={3}
            value={typeof value === "string" ? value : propToString(value)}
            onChange={(e) => onChange(e.target.value)}
          />
        </div>
      );
    case "number": {
      const numVal = typeof value === "number"
        ? value
        : (typeof value === "string" && value !== "" && !Number.isNaN(Number(value)))
        ? Number(value)
        : undefined;
      return (
        <div>
          {label}
          <Input
            size="small"
            type="number"
            className="w-full"
            value={numVal ?? ""}
            onChange={(e) => {
              const v = e.target.value;
              onChange(v === "" ? undefined : Number(v));
            }}
          />
        </div>
      );
    }
    case "select":
      return (
        <div>
          {label}
          <Select
            size="small"
            className="w-full"
            value={typeof value === "string" ? value : propToString(value)}
            options={(def.options ?? []).map((o) => ({ label: t(o.labelKey), value: o.value }))}
            onChange={(v) => onChange(v)}
            allowClear
          />
        </div>
      );
    case "switch":
      return (
        <div className="flex items-center justify-between">
          <span className="text-xs text-gray-500">{t(def.labelKey)}</span>
          <Switch
            size="small"
            checked={value === true || value === "true"}
            onChange={(checked) => onChange(checked)}
          />
        </div>
      );
    case "options": {
      const arr = Array.isArray(value) ? value as Array<{ label?: string; value?: string }> : [];
      return (
        <div>
          <div className="flex items-center justify-between mb-1">
            <span className="text-xs text-gray-500">{t(def.labelKey)}</span>
            <Button
              size="small"
              type="dashed"
              icon={<PlusOutlined />}
              onClick={() => onChange([...arr, { label: "", value: "" }])}
            >
              {t("visualEditor.property.addOption")}
            </Button>
          </div>
          <div className="space-y-1">
            {arr.map((opt, idx) => (
              <div key={idx} className="flex gap-1">
                <Input
                  size="small"
                  className="flex-1"
                  placeholder={t("visualEditor.property.label")}
                  value={opt.label ?? ""}
                  onChange={(e) => {
                    const next = [...arr];
                    next[idx] = { ...next[idx], label: e.target.value };
                    onChange(next);
                  }}
                />
                <Input
                  size="small"
                  className="flex-1"
                  placeholder={t("visualEditor.property.value")}
                  value={opt.value ?? ""}
                  onChange={(e) => {
                    const next = [...arr];
                    next[idx] = { ...next[idx], value: e.target.value };
                    onChange(next);
                  }}
                />
                <Button
                  size="small"
                  type="text"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={() => {
                    const next = [...arr];
                    next.splice(idx, 1);
                    onChange(next);
                  }}
                />
              </div>
            ))}
            {arr.length === 0 && (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={t("visualEditor.property.noOptions")}
                className="my-2"
              />
            )}
          </div>
        </div>
      );
    }
    case "json": {
      const str = typeof value === "string" ? value : propToString(value);
      return (
        <div>
          {label}
          <Input.TextArea
            size="small"
            className="w-full font-mono text-[11px]"
            rows={4}
            value={str}
            placeholder={t("visualEditor.property.jsonPlaceholder")}
            onChange={(e) => {
              const raw = e.target.value;
              try {
                const parsed = raw.trim() ? JSON.parse(raw) : undefined;
                onChange(parsed);
              } catch {
                // 解析失败时暂存为字符串，由父组件在 onBlur 时再尝试解析
                // 这里维持原值，避免输入未完成时覆盖
              }
            }}
            onBlur={(e) => {
              const raw = e.target.value;
              if (!raw.trim()) {
                onChange(undefined);
                return;
              }
              try {
                onChange(JSON.parse(raw));
              } catch {
                // 保留原值
              }
            }}
          />
        </div>
      );
    }
    default:
      return null;
  }
}

/** Basic Tab 内容：组件类型 + ID + Version + 专属字段表单 + 通用 k-v 兜底 */
function BasicTab({
  selectedNode,
  isRoot,
  onUpdateNode,
  defaultPropsMap,
  t,
}: {
  selectedNode: UISchema;
  isRoot: boolean;
  onUpdateNode: (updater: (node: UISchema) => void) => void;
  defaultPropsMap: Partial<Record<DynamicComponentType, Record<string, unknown>>>;
  t: (key: string) => string;
}) {
  const fieldDefs = PROP_FIELD_CONFIG[selectedNode.type] ?? [];
  const configuredKeys = new Set(fieldDefs.map((f) => f.prop));

  // 未被专属表单覆盖的 props（兜底 k-v 编辑器）
  const remainingEntries = Object.entries(selectedNode.props)
    .filter(([k]) => !configuredKeys.has(k));

  const handleTypeChange = (newType: DynamicComponentType) => {
    onUpdateNode((node) => {
      node.type = newType;
      node.props = { ...defaultPropsMap[newType], ...node.props };
    });
  };

  const handleFieldChange = (prop: string, v: unknown) => {
    onUpdateNode((node) => {
      node.props = { ...node.props, [prop]: v };
    });
  };

  const handleAddProp = () => {
    onUpdateNode((node) => {
      node.props = { ...node.props, "": "" };
    });
  };

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

  const handleRemoveProp = (index: number) => {
    onUpdateNode((node) => {
      const entries = Object.entries(node.props);
      entries.splice(index, 1);
      node.props = Object.fromEntries(entries);
    });
  };

  return (
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
          <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.nodeId")}</label>
          <Input size="small" value={selectedNode.id} disabled />
        </div>
        <div className="flex-1">
          <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.version")}</label>
          <Input size="small" value={selectedNode.version} disabled />
        </div>
      </div>

      <Divider className="my-2" />

      {/* 专属字段表单 */}
      {fieldDefs.length > 0 && (
        <div className="space-y-2">
          <div className="text-[11px] text-gray-400 uppercase tracking-wide">
            {t("visualEditor.property.specificFields")}
          </div>
          {fieldDefs.map((def) => (
            <PropFieldEditor
              key={def.prop}
              def={def}
              value={selectedNode.props[def.prop]}
              onChange={(v) => handleFieldChange(def.prop, v)}
              t={t}
            />
          ))}
        </div>
      )}

      {fieldDefs.length > 0 && <Divider className="my-2" />}

      {/* 通用 props 兜底（k-v 字符串） */}
      <div>
        <div className="flex items-center justify-between mb-1">
          <label className="text-xs text-gray-500">{t("visualEditor.property.advancedProps")}</label>
          <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={handleAddProp}>
            {t("dynamicUIManager.addProp")}
          </Button>
        </div>

        <div className="space-y-1 max-h-50 overflow-auto">
          {remainingEntries.length === 0 && (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("dynamicUIManager.noProps")}
              className="my-2"
            />
          )}
          {remainingEntries.map((entry, idx) => {
            const actualIdx = Object.keys(selectedNode.props).indexOf(entry[0]);
            return (
              <div key={`${entry[0]}-${idx}`} className="flex gap-1">
                <Input
                  size="small"
                  className="w-[38%] font-mono text-[11px]"
                  placeholder={t("visualEditor.property.placeholder.propKey")}
                  value={entry[0]}
                  onChange={(e) => handlePropChange(actualIdx, "key", e.target.value)}
                />
                <Input
                  size="small"
                  className="flex-1 font-mono text-[11px]"
                  placeholder={t("visualEditor.property.placeholder.propValue")}
                  value={typeof entry[1] === "string" ? entry[1] : propToString(entry[1])}
                  onChange={(e) => handlePropChange(actualIdx, "value", e.target.value)}
                />
                <Button
                  size="small"
                  type="text"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={() => handleRemoveProp(actualIdx)}
                />
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}

/** Data Tab 内容：可视化编辑 DataSourceConfig + polling */
function DataTab({
  selectedNode,
  onUpdateNode,
  t,
}: {
  selectedNode: UISchema;
  onUpdateNode: (updater: (node: UISchema) => void) => void;
  t: (key: string) => string;
}) {
  const ds = selectedNode.dataSource;

  const setDataSource = (next: UISchema["dataSource"]) => {
    onUpdateNode((node) => {
      node.dataSource = next;
    });
  };

  const updateType = (type: DataSourceConfig["type"]) => {
    setDataSource({
      type,
      config: {},
      polling: ds?.polling ?? 0,
    });
  };

  const updateConfigField = (key: string, value: unknown) => {
    setDataSource({
      type: ds?.type ?? "static",
      config: { ...(ds?.config), [key]: value },
      polling: ds?.polling ?? 0,
    });
  };

  const updatePolling = (v: number | undefined) => {
    if (!ds) { return; }
    setDataSource({ ...ds, polling: v ?? 0 });
  };

  const cfg = (ds?.config ?? {}) as Record<string, unknown>;

  return (
    <>
      {!ds
        ? (
          <Button
            block
            type="dashed"
            icon={<PlusOutlined />}
            onClick={() => setDataSource({ type: "static", config: { value: [] }, polling: 0 })}
          >
            {t("visualEditor.property.addDataSource")}
          </Button>
        )
        : (
          <>
            <div>
              <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.dataSourceType")}</label>
              <Select
                size="small"
                className="w-full"
                value={ds.type}
                onChange={updateType}
                options={[
                  { label: t("visualEditor.property.dsType.static"), value: "static" },
                  { label: t("visualEditor.property.dsType.store"), value: "store" },
                  { label: t("visualEditor.property.dsType.api"), value: "api" },
                  { label: t("visualEditor.property.dsType.agent"), value: "agent-generated" },
                ]}
              />
            </div>

            <Divider className="my-2" />

            {ds.type === "static" && (
              <div>
                <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.staticValue")}</label>
                <Input.TextArea
                  size="small"
                  className="font-mono text-[11px]"
                  rows={6}
                  value={typeof cfg.value === "string" ? cfg.value : propToString(cfg.value)}
                  placeholder={t("visualEditor.property.jsonPlaceholder")}
                  onChange={(e) => {
                    const raw = e.target.value;
                    try {
                      updateConfigField("value", raw.trim() ? JSON.parse(raw) : undefined);
                    } catch {
                      // 解析失败暂不写入
                    }
                  }}
                  onBlur={(e) => {
                    const raw = e.target.value;
                    if (!raw.trim()) {
                      updateConfigField("value", undefined);
                      return;
                    }
                    try {
                      updateConfigField("value", JSON.parse(raw));
                    } catch {
                      // 保留原值
                    }
                  }}
                />
              </div>
            )}

            {ds.type === "store" && (
              <>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.storeName")}</label>
                  <Input
                    size="small"
                    className="w-full"
                    value={typeof cfg.storeName === "string" ? cfg.storeName : ""}
                    placeholder={t("visualEditor.property.placeholder.storeName")}
                    onChange={(e) => updateConfigField("storeName", e.target.value)}
                  />
                </div>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.selector")}</label>
                  <Input
                    size="small"
                    className="w-full font-mono text-[11px]"
                    value={typeof cfg.selector === "string" ? cfg.selector : ""}
                    placeholder={t("visualEditor.property.placeholder.storeSelector")}
                    onChange={(e) => updateConfigField("selector", e.target.value)}
                  />
                </div>
              </>
            )}

            {ds.type === "api" && (
              <>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.apiEndpoint")}</label>
                  <Input
                    size="small"
                    className="w-full"
                    value={typeof cfg.endpoint === "string" ? cfg.endpoint : ""}
                    placeholder={t("visualEditor.property.placeholder.apiEndpoint")}
                    onChange={(e) => updateConfigField("endpoint", e.target.value)}
                  />
                </div>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.apiMethod")}</label>
                  <Select
                    size="small"
                    className="w-full"
                    value={typeof cfg.method === "string" ? cfg.method : "invoke"}
                    onChange={(v) => updateConfigField("method", v)}
                    options={[
                      { label: "invoke", value: "invoke" },
                      { label: "fetch", value: "fetch" },
                    ]}
                  />
                </div>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.apiParams")}</label>
                  <Input.TextArea
                    size="small"
                    className="font-mono text-[11px]"
                    rows={4}
                    value={typeof cfg.params === "string" ? cfg.params : propToString(cfg.params)}
                    placeholder={t("visualEditor.property.jsonPlaceholder")}
                    onChange={(e) => {
                      const raw = e.target.value;
                      try {
                        updateConfigField("params", raw.trim() ? JSON.parse(raw) : undefined);
                      } catch {
                        // 解析失败暂不写入
                      }
                    }}
                    onBlur={(e) => {
                      const raw = e.target.value;
                      if (!raw.trim()) {
                        updateConfigField("params", undefined);
                        return;
                      }
                      try {
                        updateConfigField("params", JSON.parse(raw));
                      } catch {
                        // 保留原值
                      }
                    }}
                  />
                </div>
              </>
            )}

            {ds.type === "agent-generated" && (
              <>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.generationId")}</label>
                  <Input
                    size="small"
                    className="w-full font-mono text-[11px]"
                    value={typeof cfg.generationId === "string" ? cfg.generationId : ""}
                    placeholder={t("visualEditor.property.placeholder.generationId")}
                    onChange={(e) => updateConfigField("generationId", e.target.value)}
                  />
                </div>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.selector")}</label>
                  <Input
                    size="small"
                    className="w-full font-mono text-[11px]"
                    value={typeof cfg.selector === "string" ? cfg.selector : ""}
                    placeholder={t("visualEditor.property.placeholder.agentSelector")}
                    onChange={(e) => updateConfigField("selector", e.target.value)}
                  />
                </div>
              </>
            )}

            <Divider className="my-2" />

            <div>
              <label className="block text-xs text-gray-500 mb-1">{t("visualEditor.property.polling")}</label>
              <Input
                size="small"
                type="number"
                className="w-full"
                value={ds.polling ?? 0}
                onChange={(e) => {
                  const v = e.target.value;
                  updatePolling(v === "" ? 0 : Number(v));
                }}
              />
              <div className="text-[10px] text-gray-400 mt-1">{t("visualEditor.property.pollingHint")}</div>
            </div>

            <Button
              block
              type="text"
              danger
              icon={<DeleteOutlined />}
              onClick={() => setDataSource(undefined)}
            >
              {t("visualEditor.property.removeDataSource")}
            </Button>
          </>
        )}
    </>
  );
}

/** Events Tab 内容：可视化编辑事件 + actions */
function EventsTab({
  selectedNode,
  onUpdateNode,
  t,
}: {
  selectedNode: UISchema;
  onUpdateNode: (updater: (node: UISchema) => void) => void;
  t: (key: string) => string;
}) {
  const events = selectedNode.events ?? [];

  const updateEvents = (next: EventHandler[]) => {
    onUpdateNode((node) => {
      node.events = next.length > 0 ? next : undefined;
    });
  };

  const addEvent = () => {
    updateEvents([
      ...events,
      {
        trigger: "onClick",
        actions: [{ type: "invoke", config: { command: "", args: {} } }],
      },
    ]);
  };

  const updateEvent = (idx: number, patch: Partial<EventHandler>) => {
    const next = [...events];
    next[idx] = { ...next[idx], ...patch };
    updateEvents(next);
  };

  const removeEvent = (idx: number) => {
    const next = [...events];
    next.splice(idx, 1);
    updateEvents(next);
  };

  const addAction = (eventIdx: number) => {
    const next = [...events];
    next[eventIdx] = {
      ...next[eventIdx],
      actions: [...next[eventIdx].actions, { type: "invoke", config: {} }],
    };
    updateEvents(next);
  };

  const updateAction = (eventIdx: number, actionIdx: number, patch: Partial<DynamicAction>) => {
    const next = [...events];
    const actions = [...next[eventIdx].actions];
    actions[actionIdx] = { ...actions[actionIdx], ...patch };
    next[eventIdx] = { ...next[eventIdx], actions };
    updateEvents(next);
  };

  const removeAction = (eventIdx: number, actionIdx: number) => {
    const next = [...events];
    const actions = [...next[eventIdx].actions];
    actions.splice(actionIdx, 1);
    next[eventIdx] = { ...next[eventIdx], actions };
    updateEvents(next);
  };

  return (
    <>
      {events.length === 0 && (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("visualEditor.property.noEvents")}
          className="my-4"
        />
      )}

      <div className="space-y-3">
        {events.map((evt, eIdx) => (
          <div key={eIdx} className="border border-gray-200 dark:border-gray-700 rounded p-2 space-y-2">
            <div className="flex items-center gap-2">
              <Select
                size="small"
                className="flex-1"
                value={evt.trigger}
                onChange={(v) =>
                  updateEvent(eIdx, { trigger: v })}
                options={[
                  { label: "onClick", value: "onClick" },
                  { label: "onChange", value: "onChange" },
                  { label: "onSubmit", value: "onSubmit" },
                  { label: "onMount", value: "onMount" },
                  { label: "onUnmount", value: "onUnmount" },
                ]}
              />
              <Button
                size="small"
                type="text"
                danger
                icon={<DeleteOutlined />}
                onClick={() =>
                  removeEvent(eIdx)}
              />
            </div>

            <div className="ml-2 space-y-1 border-l-2 border-gray-100 dark:border-gray-700 pl-2">
              {evt.actions.map((act, aIdx) => (
                <div key={aIdx} className="border border-gray-100 dark:border-gray-700 rounded p-2 space-y-1">
                  <div className="flex items-center gap-2">
                    <Select
                      size="small"
                      className="flex-1"
                      value={act.type}
                      onChange={(v) => {
                        // 切换 type 时重置 config
                        updateAction(eIdx, aIdx, {
                          type: v as DynamicAction["type"],
                          config: {},
                        });
                      }}
                      options={[
                        { label: "invoke", value: "invoke" },
                        { label: "navigate", value: "navigate" },
                        { label: "emit", value: "emit" },
                        { label: "store", value: "store" },
                        { label: "update-schema", value: "update-schema" },
                        { label: "chain", value: "chain" },
                      ]}
                    />
                    <Button
                      size="small"
                      type="text"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={() =>
                        removeAction(eIdx, aIdx)}
                    />
                  </div>

                  {/* invoke */}
                  {act.type === "invoke" && (
                    <div className="flex gap-1">
                      <Input
                        size="small"
                        className="flex-1"
                        placeholder={t("visualEditor.property.command")}
                        value={typeof (act.config as Record<string, unknown>).command === "string"
                          ? (act.config as Record<string, unknown>).command as string
                          : ""}
                        onChange={(e) =>
                          updateAction(eIdx, aIdx, {
                            config: { ...act.config, command: e.target.value },
                          })}
                      />
                    </div>
                  )}

                  {/* navigate */}
                  {act.type === "navigate" && (
                    <Input
                      size="small"
                      className="w-full"
                      placeholder={t("visualEditor.property.placeholder.navigatePath")}
                      value={typeof (act.config as Record<string, unknown>).path === "string"
                        ? (act.config as Record<string, unknown>).path as string
                        : ""}
                      onChange={(e) =>
                        updateAction(eIdx, aIdx, {
                          config: { ...act.config, path: e.target.value },
                        })}
                    />
                  )}

                  {/* emit */}
                  {act.type === "emit" && (
                    <>
                      <Input
                        size="small"
                        className="w-full"
                        placeholder={t("visualEditor.property.placeholder.emitEvent")}
                        value={typeof (act.config as Record<string, unknown>).event === "string"
                          ? (act.config as Record<string, unknown>).event as string
                          : ""}
                        onChange={(e) =>
                          updateAction(eIdx, aIdx, {
                            config: { ...act.config, event: e.target.value },
                          })}
                      />
                      <Input.TextArea
                        size="small"
                        className="font-mono text-[11px]"
                        rows={2}
                        placeholder={t("visualEditor.property.jsonPlaceholder")}
                        value={propToString((act.config as Record<string, unknown>).payload)}
                        onChange={(e) => {
                          const raw = e.target.value;
                          try {
                            updateAction(eIdx, aIdx, {
                              config: { ...act.config, payload: raw.trim() ? JSON.parse(raw) : undefined },
                            });
                          } catch {
                            // 解析失败暂不写入
                          }
                        }}
                      />
                    </>
                  )}

                  {/* store */}
                  {act.type === "store" && (
                    <>
                      <Input
                        size="small"
                        className="w-full"
                        placeholder={t("visualEditor.property.placeholder.actionStoreName")}
                        value={typeof (act.config as Record<string, unknown>).storeName === "string"
                          ? (act.config as Record<string, unknown>).storeName as string
                          : ""}
                        onChange={(e) =>
                          updateAction(eIdx, aIdx, {
                            config: { ...act.config, storeName: e.target.value },
                          })}
                      />
                      <Select
                        size="small"
                        className="w-full"
                        value={typeof (act.config as Record<string, unknown>).operation === "string"
                          ? (act.config as Record<string, unknown>).operation as string
                          : "get"}
                        onChange={(v) =>
                          updateAction(eIdx, aIdx, {
                            config: { ...act.config, operation: v },
                          })}
                        options={[
                          { label: "get", value: "get" },
                          { label: "set", value: "set" },
                          { label: "update", value: "update" },
                        ]}
                      />
                      <Input.TextArea
                        size="small"
                        className="font-mono text-[11px]"
                        rows={2}
                        placeholder={t("visualEditor.property.jsonPlaceholder")}
                        value={propToString((act.config as Record<string, unknown>).payload)}
                        onChange={(e) => {
                          const raw = e.target.value;
                          try {
                            updateAction(eIdx, aIdx, {
                              config: { ...act.config, payload: raw.trim() ? JSON.parse(raw) : undefined },
                            });
                          } catch {
                            // 解析失败暂不写入
                          }
                        }}
                      />
                    </>
                  )}

                  {/* update-schema */}
                  {act.type === "update-schema" && (
                    <>
                      <Input
                        size="small"
                        className="w-full"
                        placeholder={t("visualEditor.property.placeholder.schemaId")}
                        value={typeof (act.config as Record<string, unknown>).schemaId === "string"
                          ? (act.config as Record<string, unknown>).schemaId as string
                          : ""}
                        onChange={(e) =>
                          updateAction(eIdx, aIdx, {
                            config: { ...act.config, schemaId: e.target.value },
                          })}
                      />
                      <Select
                        size="small"
                        className="w-full"
                        value={typeof (act.config as Record<string, unknown>).operation === "string"
                          ? (act.config as Record<string, unknown>).operation as string
                          : "replace"}
                        onChange={(v) =>
                          updateAction(eIdx, aIdx, {
                            config: { ...act.config, operation: v },
                          })}
                        options={[
                          { label: "replace", value: "replace" },
                          { label: "append", value: "append" },
                          { label: "remove", value: "remove" },
                        ]}
                      />
                    </>
                  )}
                </div>
              ))}
              <Button
                size="small"
                type="dashed"
                block
                icon={<PlusOutlined />}
                onClick={() => addAction(eIdx)}
              >
                {t("visualEditor.property.addAction")}
              </Button>
            </div>
          </div>
        ))}
      </div>

      <Button
        block
        type="dashed"
        icon={<PlusOutlined />}
        onClick={addEvent}
        className="mt-2"
      >
        {t("visualEditor.property.addEvent")}
      </Button>
    </>
  );
}

// ── ConditionalTab：条件渲染可视化编辑 ──────────────────────────────────────

/**
 * 可视化编辑 conditionalDisplay 规则。
 *
 * 支持两种形式：
 *   1. 规则数组（隐式 AND）：[rule1, rule2, ...]
 *   2. 逻辑组合：{ logic: "and"|"or", rules: [...], not?: boolean }
 *
 * UI 上用嵌套结构展示，每条规则可编辑 field / operator / value。
 * 顶部按钮切换 AND / OR 逻辑组合（会把规则数组转成 logic 形式）。
 */
function ConditionalTab({
  selectedNode,
  onUpdateNode,
  t,
}: {
  selectedNode: UISchema;
  onUpdateNode: (updater: (node: UISchema) => void) => void;
  t: (key: string) => string;
}) {
  const OPERATORS = [
    { value: "eq", label: "==" },
    { value: "neq", label: "!=" },
    { value: "gt", label: ">" },
    { value: "gte", label: ">=" },
    { value: "lt", label: "<" },
    { value: "lte", label: "<=" },
    { value: "in", label: "in" },
    { value: "contains", label: "contains" },
    { value: "exists", label: "exists" },
    { value: "empty", label: "empty" },
  ] as const;

  // 标准化为逻辑组合形式以便 UI 编辑
  const cd = selectedNode.conditionalDisplay;
  const isLogicForm = cd !== undefined && !Array.isArray(cd);
  const rules = isLogicForm
    ? (cd as { rules: ConditionalDisplay[] }).rules
    : (cd as ConditionalRule[] | undefined) ?? [];
  const logic = isLogicForm ? (cd as { logic: "and" | "or" }).logic : "and";
  const notFlag = isLogicForm ? (cd as { not?: boolean }).not ?? false : false;

  // 拍平 rules 数组为 ConditionalRule（嵌套 logic 形式暂不深编辑，只支持一层）
  const flatRules: ConditionalRule[] = rules.map((r) => {
    if (Array.isArray(r)) { return r[0] ?? { field: "", operator: "eq", value: "" }; }
    if ((r as { rules?: unknown }).rules) {
      // 嵌套逻辑组合 — 简化展示为占位
      return { field: "(nested)", operator: "exists", value: "" };
    }
    return r as ConditionalRule;
  });

  const updateConditional = (next: ConditionalDisplay | undefined) => {
    onUpdateNode((node) => {
      if (next === undefined) {
        node.conditionalDisplay = undefined;
      } else {
        node.conditionalDisplay = next;
      }
    });
  };

  const setLogic = (newLogic: "and" | "or") => {
    updateConditional({
      logic: newLogic,
      rules: flatRules.map((r) => ({ ...r })) as unknown as ConditionalDisplay[],
      not: notFlag,
    } as ConditionalDisplay);
  };

  const toggleNot = () => {
    updateConditional({
      logic,
      rules: flatRules.map((r) => ({ ...r })) as unknown as ConditionalDisplay[],
      not: !notFlag,
    } as ConditionalDisplay);
  };

  const addRule = () => {
    const newRule: ConditionalRule = { field: "field", operator: "eq", value: "" };
    updateConditional({
      logic,
      rules: [...flatRules, newRule] as unknown as ConditionalDisplay[],
      not: notFlag,
    } as ConditionalDisplay);
  };

  const updateRule = (idx: number, patch: Partial<ConditionalRule>) => {
    const nextRules = flatRules.map((r, i) => i === idx ? { ...r, ...patch } : r);
    updateConditional({
      logic,
      rules: nextRules as unknown as ConditionalDisplay[],
      not: notFlag,
    } as ConditionalDisplay);
  };

  const removeRule = (idx: number) => {
    const nextRules = flatRules.filter((_, i) => i !== idx);
    if (nextRules.length === 0) {
      updateConditional(undefined);
    } else {
      updateConditional({
        logic,
        rules: nextRules as unknown as ConditionalDisplay[],
        not: notFlag,
      } as ConditionalDisplay);
    }
  };

  const clearAll = () => {
    updateConditional(undefined);
  };

  return (
    <div className="space-y-3">
      {/* 顶部说明 */}
      <div className="text-xs text-gray-500 dark:text-gray-400 leading-relaxed">
        {t("visualEditor.property.conditional.description")}
      </div>

      {/* 逻辑组合切换 + NOT + 清空 */}
      {flatRules.length > 0 && (
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-0.5 bg-gray-200 dark:bg-gray-700 rounded p-0.5">
            <button
              type="button"
              onClick={() => setLogic("and")}
              className={`text-[10px] px-2 py-0.5 rounded ${
                logic === "and" ? "bg-white dark:bg-gray-900 shadow text-blue-600 dark:text-blue-400" : "text-gray-500"
              }`}
            >
              AND
            </button>
            <button
              type="button"
              onClick={() => setLogic("or")}
              className={`text-[10px] px-2 py-0.5 rounded ${
                logic === "or" ? "bg-white dark:bg-gray-900 shadow text-blue-600 dark:text-blue-400" : "text-gray-500"
              }`}
            >
              OR
            </button>
          </div>

          <Tooltip title={t("visualEditor.property.conditional.notHint")}>
            <button
              type="button"
              onClick={toggleNot}
              className={`text-[10px] px-2 py-0.5 rounded border ${
                notFlag
                  ? "bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 border-red-300 dark:border-red-700"
                  : "border-gray-300 dark:border-gray-600 text-gray-500"
              }`}
            >
              NOT
            </button>
          </Tooltip>

          <Button size="small" type="text" danger onClick={clearAll} className="ml-auto text-[10px]">
            {t("visualEditor.property.conditional.clear")}
          </Button>
        </div>
      )}

      {/* 规则列表 */}
      {flatRules.length === 0
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("visualEditor.property.conditional.empty")}
            className="my-4"
          />
        )
        : (
          <div className="space-y-2">
            {flatRules.map((rule, idx) => (
              <div
                key={idx}
                className="border border-gray-200 dark:border-gray-700 rounded p-2 bg-white dark:bg-gray-800 space-y-1.5"
              >
                <div className="flex items-center gap-1">
                  <Input
                    size="small"
                    value={rule.field}
                    placeholder={t("visualEditor.property.conditional.fieldPlaceholder")}
                    onChange={(e) => updateRule(idx, { field: e.target.value })}
                    className="flex-1 text-xs font-mono"
                  />
                  <Button
                    size="small"
                    type="text"
                    danger
                    icon={<MinusCircleOutlined />}
                    onClick={() => removeRule(idx)}
                  />
                </div>
                <div className="flex items-center gap-1">
                  <Select
                    size="small"
                    value={rule.operator}
                    onChange={(v) => updateRule(idx, { operator: v as ConditionalRule["operator"] })}
                    options={OPERATORS.map((op) => ({ value: op.value, label: op.label }))}
                    className="w-24 text-xs"
                  />
                  {(rule.operator !== "exists" && rule.operator !== "empty") && (
                    <Input
                      size="small"
                      value={typeof rule.value === "string" ? rule.value : JSON.stringify(rule.value)}
                      placeholder={t("visualEditor.property.conditional.valuePlaceholder")}
                      onChange={(e) => {
                        const v = e.target.value;
                        // 尝试解析数字/布尔
                        let parsed: unknown = v;
                        if (v === "true") { parsed = true; }
                        else if (v === "false") { parsed = false; }
                        else if (/^-?\d+(\.\d+)?$/.test(v)) { parsed = Number(v); }
                        updateRule(idx, { value: parsed });
                      }}
                      className="flex-1 text-xs"
                    />
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

      <Button
        size="small"
        type="dashed"
        icon={<PlusOutlined />}
        onClick={addRule}
        className="w-full"
      >
        {t("visualEditor.property.conditional.addRule")}
      </Button>
    </div>
  );
}

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
  const [activeTab, setActiveTab] = useState<"basic" | "data" | "events" | "conditional" | "style">("basic");

  if (!selectedNode) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-gray-400 dark:text-gray-500 p-4">
        <SettingOutlined className="text-3xl mb-2 opacity-40" />
        <p className="text-xs text-center">{t("visualEditor.property.selectHint")}</p>
      </div>
    );
  }

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
    { key: "data" as const, label: t("visualEditor.property.tabs.data"), icon: "📊" },
    { key: "events" as const, label: t("visualEditor.property.tabs.events"), icon: "⚡" },
    { key: "conditional" as const, label: t("visualEditor.property.tabs.conditional"), icon: "🔀" },
    { key: "style" as const, label: t("visualEditor.property.tabs.style"), icon: "🎨" },
  ];

  return (
    <div className="h-full overflow-y-auto p-3 bg-gray-50 dark:bg-gray-800 space-y-4">
      {/* Tab 切换 */}
      <div className="grid grid-cols-5 gap-1">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`
              text-xs px-1 py-1 rounded transition-all
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
        <BasicTab
          selectedNode={selectedNode}
          isRoot={isRoot}
          onUpdateNode={onUpdateNode}
          defaultPropsMap={defaultPropsMap}
          t={t}
        />
      )}

      {activeTab === "data" && (
        <DataTab
          selectedNode={selectedNode}
          onUpdateNode={onUpdateNode}
          t={t}
        />
      )}

      {activeTab === "events" && (
        <EventsTab
          selectedNode={selectedNode}
          onUpdateNode={onUpdateNode}
          t={t}
        />
      )}

      {activeTab === "conditional" && (
        <ConditionalTab
          selectedNode={selectedNode}
          onUpdateNode={onUpdateNode}
          t={t}
        />
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

// ── 错误边界：属性面板输入错误时不影响整个编辑器 ────────────────────────────

interface EditorErrorBoundaryState {
  error: Error | null;
}

class EditorErrorBoundary extends React.Component<
  { children: React.ReactNode; t: (key: string) => string },
  EditorErrorBoundaryState
> {
  state: EditorErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): EditorErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo): void {
    // eslint-disable-next-line no-console
    console.error("[VisualEditor] ErrorBoundary caught:", error, info);
  }

  handleReset = (): void => {
    this.setState({ error: null });
  };

  render(): React.ReactNode {
    if (this.state.error) {
      return (
        <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded m-2">
          <div className="text-xs font-semibold text-red-600 dark:text-red-400 mb-1">
            {this.props.t("visualEditor.error.title")}
          </div>
          <div className="text-[10px] text-red-500 dark:text-red-500 font-mono break-all mb-2">
            {this.state.error.message}
          </div>
          <button
            type="button"
            onClick={this.handleReset}
            className="text-[10px] px-2 py-0.5 bg-red-100 dark:bg-red-900/50 hover:bg-red-200 dark:hover:bg-red-800 text-red-600 dark:text-red-400 rounded border border-red-300 dark:border-red-700"
          >
            {this.props.t("visualEditor.error.reset")}
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

// ── localStorage 持久化辅助 ────────────────────────────────────────────────

const STORAGE_PREFIX = "axagent:dynamic-ui:";

/** 从 localStorage 读取 schema，解析失败或不存在时返回 null */
function loadSchemaFromStorage(key: string): UISchema | null {
  try {
    const raw = localStorage.getItem(STORAGE_PREFIX + key);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as UISchema;
    // 基础校验
    if (!parsed.type || !parsed.id || !parsed.version) {
      return null;
    }
    return parsed;
  } catch {
    // localStorage 不可用或 JSON 解析失败，降级为内存态
    return null;
  }
}

/** 把 schema 写入 localStorage，失败时静默降级（如配额超限） */
function saveSchemaToStorage(key: string, schema: UISchema): void {
  try {
    localStorage.setItem(STORAGE_PREFIX + key, JSON.stringify(schema));
  } catch {
    // 静默失败：配额超限、隐私模式、localStorage 被禁用等
  }
}

// ── 子组件：工具栏 ──────────────────────────────────────────────────────────

interface EditorToolbarProps {
  canUndo: boolean;
  canRedo: boolean;
  canCopy: boolean;
  canPaste: boolean;
  isPreviewMode: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onCopy: () => void;
  onCut: () => void;
  onPaste: () => void;
  onExport: () => void;
  onImport: (file: File) => void;
  onTogglePreview: () => void;
  viewport: Viewport;
  onViewportChange: (v: Viewport) => void;
  t: (key: string) => string;
}

function EditorToolbar({
  canUndo,
  canRedo,
  canCopy,
  canPaste,
  isPreviewMode,
  onUndo,
  onRedo,
  onCopy,
  onCut,
  onPaste,
  onExport,
  onImport,
  onTogglePreview,
  viewport,
  onViewportChange,
  t,
}: EditorToolbarProps) {
  // Upload 组件配置：单文件、不上传到服务器，直接拿到 File 对象
  const uploadProps: UploadProps = {
    accept: ".json,application/json",
    showUploadList: false,
    beforeUpload: (file) => {
      onImport(file);
      return false; // 阻止 antd 自动上传
    },
  };

  const btnClass = "flex items-center gap-1 text-xs px-2 py-1 rounded transition-colors";
  const enabledClass = "hover:bg-blue-50 dark:hover:bg-blue-900/30 text-gray-700 dark:text-gray-300";
  const disabledClass = "opacity-40 cursor-not-allowed text-gray-400 dark:text-gray-600";

  return (
    <div
      className="flex items-center gap-1 px-3 py-1.5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 flex-wrap"
      onClick={(e) => e.stopPropagation()}
    >
      {/* 撤销 / 重做 */}
      <Tooltip title={`${t("visualEditor.toolbar.undo")} (Ctrl+Z)`}>
        <button
          type="button"
          disabled={!canUndo}
          onClick={onUndo}
          className={`${btnClass} ${canUndo ? enabledClass : disabledClass}`}
        >
          <UndoOutlined />
        </button>
      </Tooltip>
      <Tooltip title={`${t("visualEditor.toolbar.redo")} (Ctrl+Y)`}>
        <button
          type="button"
          disabled={!canRedo}
          onClick={onRedo}
          className={`${btnClass} ${canRedo ? enabledClass : disabledClass}`}
        >
          <RedoOutlined />
        </button>
      </Tooltip>

      <Divider type="vertical" className="mx-1 !h-4" />

      {/* 复制 / 剪切 / 粘贴 */}
      <Tooltip title={`${t("visualEditor.toolbar.copy")} (Ctrl+C)`}>
        <button
          type="button"
          disabled={!canCopy}
          onClick={onCopy}
          className={`${btnClass} ${canCopy ? enabledClass : disabledClass}`}
        >
          <CopyOutlined />
        </button>
      </Tooltip>
      <Tooltip title={`${t("visualEditor.toolbar.cut")} (Ctrl+X)`}>
        <button
          type="button"
          disabled={!canCopy}
          onClick={onCut}
          className={`${btnClass} ${canCopy ? enabledClass : disabledClass}`}
        >
          <ScissorOutlined />
        </button>
      </Tooltip>
      <Tooltip title={`${t("visualEditor.toolbar.paste")} (Ctrl+V)`}>
        <button
          type="button"
          disabled={!canPaste}
          onClick={onPaste}
          className={`${btnClass} ${canPaste ? enabledClass : disabledClass}`}
        >
          <SnippetsOutlined />
        </button>
      </Tooltip>

      <Divider type="vertical" className="mx-1 !h-4" />

      {/* 导入 / 导出 */}
      <Tooltip title={t("visualEditor.toolbar.import")}>
        <Upload {...uploadProps}>
          <button type="button" className={`${btnClass} ${enabledClass}`}>
            <ImportOutlined />
          </button>
        </Upload>
      </Tooltip>
      <Tooltip title={t("visualEditor.toolbar.export")}>
        <button type="button" onClick={onExport} className={`${btnClass} ${enabledClass}`}>
          <ExportOutlined />
        </button>
      </Tooltip>

      <Divider type="vertical" className="mx-1 !h-4" />

      {/* 预览 / 编辑 模式切换 */}
      <Tooltip title={isPreviewMode ? t("visualEditor.toolbar.editMode") : t("visualEditor.toolbar.previewMode")}>
        <button
          type="button"
          onClick={onTogglePreview}
          className={`
            ${btnClass}
            ${
            isPreviewMode
              ? "bg-blue-500 text-white hover:bg-blue-600"
              : enabledClass
          }
          `}
        >
          {isPreviewMode ? <EditOutlined /> : <EyeOutlined />}
          <span>{isPreviewMode ? t("visualEditor.toolbar.editMode") : t("visualEditor.toolbar.previewMode")}</span>
        </button>
      </Tooltip>

      {/* 视口切换 — 仅在预览模式显示，让用户能真实看到不同尺寸下的渲染效果 */}
      {isPreviewMode && (
        <>
          <Divider type="vertical" className="mx-1 !h-4" />
          <div className="flex items-center gap-0.5 bg-gray-200 dark:bg-gray-700 rounded p-0.5">
            {(["desktop", "tablet", "mobile"] as const).map((vp) => (
              <Tooltip
                key={vp}
                title={`${t(`visualEditor.toolbar.viewport.${vp}`)} (${VIEWPORT_WIDTH[vp]}px)`}
              >
                <button
                  type="button"
                  onClick={() => onViewportChange(vp)}
                  className={`
                    flex items-center justify-center w-7 h-6 rounded text-[10px] transition-colors
                    ${
                    viewport === vp
                      ? "bg-white dark:bg-gray-900 shadow text-blue-600 dark:text-blue-400"
                      : "text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                  }
                  `}
                >
                  {/* 简单的 SVG 图标：桌面/平板/手机 */}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    {vp === "desktop" && (
                      <>
                        <rect x="2" y="3" width="20" height="14" rx="2" />
                        <line x1="8" y1="21" x2="16" y2="21" />
                        <line x1="12" y1="17" x2="12" y2="21" />
                      </>
                    )}
                    {vp === "tablet" && (
                      <>
                        <rect x="5" y="2" width="14" height="20" rx="2" />
                        <line x1="12" y1="18" x2="12" y2="18" />
                      </>
                    )}
                    {vp === "mobile" && (
                      <>
                        <rect x="7" y="2" width="10" height="20" rx="2" />
                        <line x1="12" y1="18" x2="12" y2="18" />
                      </>
                    )}
                  </svg>
                </button>
              </Tooltip>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

// ── 子组件：组件树大纲视图（Outline）─────────────────────────────────────────

interface OutlinePanelProps {
  root: UISchema;
  selectedId: string | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  onDuplicate: (id: string) => void;
  t: (key: string) => string;
}

function OutlinePanel({ root, selectedId, onSelect, onDelete, onDuplicate, t }: OutlinePanelProps) {
  // 递归渲染树节点
  const renderNode = (node: UISchema, depth: number): React.ReactNode => {
    const isSelected = node.id === selectedId;
    const children = node.children ?? [];
    const isContainer = CONTAINER_TYPES.has(node.type);

    return (
      <div key={node.id}>
        <div
          onClick={(e) => {
            e.stopPropagation();
            onSelect(node.id);
          }}
          className={`
            flex items-center gap-1 px-1.5 py-1 rounded cursor-pointer transition-colors text-xs
            ${
            isSelected
              ? "bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300"
              : "hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300"
          }
          `}
          style={{ paddingLeft: `${depth * 12 + 6}px` }}
        >
          {/* 展开/折叠指示（容器节点有子节点时显示箭头） */}
          <span className="w-3 text-gray-400">
            {isContainer && children.length > 0 ? "▾" : ""}
          </span>

          {/* 类型图标 */}
          <span className="text-[10px]">
            {isContainer ? "📦" : CATEGORY_ICON_MAP_BY_TYPE[node.type] ?? "▫"}
          </span>

          {/* 类型标签 */}
          <span className="font-mono truncate flex-1">{node.type}</span>

          {/* 子节点计数 */}
          {children.length > 0 && (
            <span className="text-[9px] text-gray-400 dark:text-gray-500 bg-gray-100 dark:bg-gray-700 px-1 rounded-full">
              {children.length}
            </span>
          )}

          {/* 操作按钮（hover 显示） */}
          <div className="hidden group-hover:flex items-center gap-0.5">
            <Tooltip title={t("visualEditor.outline.duplicate")}>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onDuplicate(node.id);
                }}
                className="p-0.5 hover:text-blue-500"
              >
                <CopyOutlined className="text-[10px]" />
              </button>
            </Tooltip>
            <Tooltip title={t("visualEditor.outline.delete")}>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(node.id);
                }}
                className="p-0.5 hover:text-red-500"
                disabled={node.id === root.id}
              >
                <DeleteOutlined className="text-[10px]" />
              </button>
            </Tooltip>
          </div>
        </div>

        {/* 递归渲染子节点 */}
        {children.length > 0 && (
          <div className="border-l border-gray-200 dark:border-gray-700 ml-2">
            {children.map((child) => renderNode(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="h-full overflow-y-auto p-2">
      <div className="text-[10px] uppercase tracking-wide text-gray-400 dark:text-gray-500 mb-2 px-1">
        {t("visualEditor.outline.title")}
      </div>
      {renderNode(root, 0)}
    </div>
  );
}

// 类型 → emoji 图标映射（Outline 用）
const CATEGORY_ICON_MAP_BY_TYPE: Partial<Record<DynamicComponentType, string>> = {
  Table: "📊",
  Chart: "📈",
  List: "📋",
  Tree: "🌲",
  Timeline: "⏰",
  Input: "📝",
  Textarea: "📝",
  Number: "🔢",
  Select: "📋",
  DatePicker: "📅",
  Switch: "🔘",
  Checkbox: "☑",
  Radio: "📻",
  Button: "🔺",
  Text: "🔤",
  Divider: "─",
  Progress: "📊",
  Tag: "🏷",
  Image: "🖼",
  Markdown: "📄",
  CodeEditor: "💻",
  FilePreview: "📁",
};

// ── 主组件 ──────────────────────────────────────────────────────────────────

export function VisualEditor({ schema: propSchema, onChange, persistenceKey }: VisualEditorProps) {
  const { t } = useTranslation();
  const { message: messageApi } = App.useApp();
  const editorRef = useRef<HTMLDivElement>(null);

  // 内部 schema（启用持久化时优先从 localStorage 读取，否则用 propSchema）
  const [internalSchema, setInternalSchema] = useState<UISchema>(() => {
    if (persistenceKey) {
      const saved = loadSchemaFromStorage(persistenceKey);
      if (saved) {
        return saved;
      }
    }
    return propSchema ? cloneSchema(propSchema) : createEmptyRoot();
  });

  // 选中的节点 ID
  const [selectedId, setSelectedId] = useState<string | null>(null);
  // 全屏模式
  const [isFullscreen, setIsFullscreen] = useState(false);
  // 预览模式（隐藏编辑器壳，仅渲染真实效果）
  const [isPreviewMode, setIsPreviewMode] = useState(false);
  // 响应式视口：仅预览模式生效
  const [viewport, setViewport] = useState<Viewport>("desktop");
  // 剪贴板（复制/剪切时缓存节点子树）
  const [clipboard, setClipboard] = useState<UISchema | null>(null);

  // ── 撤销/重做历史栈 ──
  // 使用 ref 保存避免每次渲染重建；用 forceHistoryTick 触发按钮 disabled 状态更新
  const pastRef = useRef<UISchema[]>([]);
  const futureRef = useRef<UISchema[]>([]);
  const [historyTick, setHistoryTick] = useState(0);
  const HISTORY_LIMIT = 50;

  // onChange 也用 ref，避免 commit / undo / redo 闭包里捕获旧 onChange
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  /**
   * 统一的 schema 变更入口：
   *   1. 调用 updater 计算新 schema
   *   2. 把旧 schema push 到 past 栈，清空 future
   *   3. 通知外部 onChange
   *
   * 所有结构性变更（拖拽、属性修改、删除、粘贴、导入）都应走此函数，
   * 才能被撤销栈正确捕获。
   */
  const commit = useCallback((updater: (prev: UISchema) => UISchema) => {
    setInternalSchema((prev) => {
      const next = updater(prev);
      if (next === prev) { return prev; }
      pastRef.current.push(prev);
      if (pastRef.current.length > HISTORY_LIMIT) {
        pastRef.current.shift();
      }
      futureRef.current = [];
      onChangeRef.current(next);
      setHistoryTick((v) => v + 1);
      return next;
    });
  }, []);

  const undo = useCallback(() => {
    if (pastRef.current.length === 0) { return; }
    setInternalSchema((prev) => {
      const last = pastRef.current.pop()!;
      futureRef.current.push(prev);
      onChangeRef.current(last);
      setHistoryTick((v) => v + 1);
      return last;
    });
  }, []);

  const redo = useCallback(() => {
    if (futureRef.current.length === 0) { return; }
    setInternalSchema((prev) => {
      const next = futureRef.current.pop()!;
      pastRef.current.push(prev);
      onChangeRef.current(next);
      setHistoryTick((v) => v + 1);
      return next;
    });
  }, []);

  // ── localStorage 自动保存（debounce 800ms） ──
  // 仅当传入 persistenceKey 时启用；避免每次属性微调都触发写入
  useEffect(() => {
    if (!persistenceKey) {
      return;
    }
    const timer = setTimeout(() => {
      saveSchemaToStorage(persistenceKey, internalSchema);
    }, 800);
    return () => clearTimeout(timer);
  }, [internalSchema, persistenceKey]);

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
  const canCopy = !isRootSelected && !!selectedNode && !isPreviewMode;
  const canPaste = clipboard !== null && !isPreviewMode;
  const canUndo = pastRef.current.length > 0;
  const canRedo = futureRef.current.length > 0;
  // historyTick 仅用于触发重渲染，让 canUndo/canRedo 重新计算
  void historyTick;

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

      // 从组件面板拖入容器节点 — append 到该容器 children
      if (activeData.type === DRAG_TYPE_PALETTE && overData.type === "container-drop") {
        const itemType = activeData.itemType as DynamicComponentType;
        const targetNodeId = overData.nodeId as string;
        commit((prev) => {
          const next = cloneSchema(prev);
          const target = findNodeById(next, targetNodeId);
          if (!target) { return prev; }
          const newChild: UISchema = {
            version: "1.0",
            id: genId(itemType.toLowerCase()),
            type: itemType,
            props: { ...defaultPropsMap[itemType] },
          };
          target.children = [...(target.children ?? []), newChild];
          return next;
        });
        return;
      }

      // 从组件面板拖到画布根 / 任意 canvas 节点上 — 兼容旧行为，append 到 root
      if (
        activeData.type === DRAG_TYPE_PALETTE && (overData.type === "canvas-root" || overData.type === DRAG_TYPE_CANVAS)
      ) {
        const itemType = activeData.itemType as DynamicComponentType;
        commit((prev) => {
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

      // 画布内同级重排序（仅 root 一级）
      if (activeData.type === DRAG_TYPE_CANVAS && overData.type === "canvas-root") {
        commit((prev) => {
          const oldIndex = prev.children?.findIndex((c) => c.id === active.id) ?? -1;
          const newIndex = prev.children?.findIndex((c) => c.id === over.id) ?? -1;
          if (oldIndex >= 0 && newIndex >= 0 && oldIndex !== newIndex) {
            const next = cloneSchema(prev);
            next.children = arrayMove(next.children ?? [], oldIndex, newIndex);
            return next;
          }
          return prev;
        });
        return;
      }

      // 跨容器移动：把画布节点拖到另一个容器节点的 drop 区
      // 从原位置摘除 → append 到目标容器
      if (activeData.type === DRAG_TYPE_CANVAS && overData.type === "container-drop") {
        const sourceNode = activeData.node as UISchema;
        const targetNodeId = overData.nodeId as string;
        // 不允许拖入自己或自己的子孙（会形成环）
        if (sourceNode.id === targetNodeId || isDescendant(sourceNode, targetNodeId)) {
          return;
        }
        commit((prev) => {
          // 1. 从原位置摘除
          const { tree: detached, detached: node } = detachNodeById(prev, sourceNode.id);
          if (!node) {
            return prev;
          }
          // 2. append 到目标容器
          return appendChildToNode(detached, targetNodeId, node);
        });
        return;
      }

      // 跨容器移动到画布根（从容器里拖出到 root）
      if (activeData.type === DRAG_TYPE_CANVAS && overData.type === DRAG_TYPE_CANVAS) {
        const sourceNode = activeData.node as UISchema;
        const overNodeId = overData.node as UISchema;
        if (sourceNode.id === overNodeId.id) {
          return; // 拖到自己身上
        }
        commit((prev) => {
          // 1. 从原位置摘除
          const { tree: detached, detached: node } = detachNodeById(prev, sourceNode.id);
          if (!node) {
            return prev;
          }
          // 2. 在 root 下定位 over 节点并插到它前面；找不到则 append
          const rootChildren = detached.children ?? [];
          const insertIdx = rootChildren.findIndex((c) => c.id === overNodeId.id);
          if (insertIdx >= 0) {
            return insertBeforeSibling(detached, detached.id, node, overNodeId.id);
          }
          return appendChildToNode(detached, detached.id, node);
        });
        return;
      }
    },
    [commit, defaultPropsMap],
  );

  // ── 操作方法 ──
  const handleSelectNode = useCallback((id: string) => {
    setSelectedId(id);
  }, []);

  const handleUpdateSelectedNode = useCallback(
    (updater: (node: UISchema) => void) => {
      commit((prev) => {
        const next = cloneSchema(prev);
        let target: UISchema;
        if (!selectedId || selectedId === prev.id) {
          target = next;
        } else {
          target = findNodeById(next, selectedId)!;
          if (!target) { return prev; }
        }
        updater(target);
        return next;
      });
    },
    [commit, selectedId],
  );

  // ── 复制 / 剪切 / 粘贴 ──
  const handleCopy = useCallback(() => {
    if (!selectedNode || isRootSelected) { return; }
    setClipboard(cloneSchema(selectedNode));
    messageApi.success(t("visualEditor.toolbar.copied"));
  }, [selectedNode, isRootSelected, t]);

  const handleCut = useCallback(() => {
    if (!selectedNode || isRootSelected || !selectedId) { return; }
    setClipboard(cloneSchema(selectedNode));
    commit((prev) => removeNodeById(prev, selectedId));
    setSelectedId(null);
    messageApi.success(t("visualEditor.toolbar.cutDone"));
  }, [selectedNode, isRootSelected, selectedId, commit, t]);

  const handlePaste = useCallback(() => {
    if (!clipboard) { return; }
    const newSubtree = cloneSchemaWithNewIds(clipboard);
    commit((prev) => {
      const next = cloneSchema(prev);
      // 优先粘贴到当前选中的容器节点；否则粘贴到 root
      let target: UISchema = next;
      if (selectedId && selectedId !== next.id) {
        const found = findNodeById(next, selectedId);
        if (found && CONTAINER_TYPES.has(found.type)) {
          target = found;
        }
      }
      target.children = [...(target.children ?? []), newSubtree];
      return next;
    });
    // 选中新粘贴的根节点，便于继续操作
    setSelectedId(newSubtree.id);
    messageApi.success(t("visualEditor.toolbar.pasted"));
  }, [clipboard, selectedId, commit, t]);

  // ── 导入 / 导出 JSON ──
  const handleExport = useCallback(() => {
    const json = JSON.stringify(internalSchema, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `dynamic-ui-${Date.now()}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    messageApi.success(t("visualEditor.toolbar.exported"));
  }, [internalSchema, t]);

  const handleImport = useCallback((file: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const text = e.target?.result as string;
        const parsed = JSON.parse(text) as UISchema;
        // 基础校验
        if (!parsed.type || !parsed.id || !parsed.version) {
          messageApi.error(t("visualEditor.toolbar.invalidJson"));
          return;
        }
        commit(() => parsed);
        setSelectedId(null);
        messageApi.success(t("visualEditor.toolbar.imported"));
      } catch {
        messageApi.error(t("visualEditor.toolbar.invalidJson"));
      }
    };
    reader.onerror = () => {
      messageApi.error(t("visualEditor.toolbar.invalidJson"));
    };
    reader.readAsText(file);
  }, [commit, t]);

  // ── 事件预览测试 ──
  // 在预览模式下点击组件触发 action 时，通过 messageApi.info 展示触发的 action 详情
  // 让用户验证事件绑定是否正确，无需发布即可调试
  const handlePreviewAction = useCallback((action: DynamicAction) => {
    const detail = JSON.stringify(action.config, null, 2);
    messageApi.info({
      content: `${t("visualEditor.preview.actionTriggered")}: ${action.type}\n${detail}`,
      duration: 3,
    });
  }, [t]);

  // ── 键盘快捷键 ──
  // 仅在编辑器内响应；input/textarea 中输入时不拦截复制/粘贴（让浏览器原生行为生效）
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      if (!ctrl) { return; }

      // 编辑器外不响应
      const editorEl = editorRef.current;
      if (!editorEl) { return; }
      const active = document.activeElement;
      if (!active || !editorEl.contains(active)) {
        // 也允许在 body 焦点时响应（无 input 聚焦）
        if (active !== null && active !== document.body) { return; }
      }

      const key = e.key.toLowerCase();
      const target = e.target as HTMLElement;
      const inFormField = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;

      switch (key) {
        case "z": {
          e.preventDefault();
          if (e.shiftKey) { redo(); }
          else { undo(); }
          break;
        }
        case "y": {
          e.preventDefault();
          redo();
          break;
        }
        case "c": {
          // 复制节点：仅在选中非根节点、且焦点不在输入框时拦截
          if (!inFormField && canCopy) {
            e.preventDefault();
            handleCopy();
          }
          break;
        }
        case "x": {
          if (!inFormField && canCopy) {
            e.preventDefault();
            handleCut();
          }
          break;
        }
        case "v": {
          if (!inFormField && canPaste) {
            e.preventDefault();
            handlePaste();
          }
          break;
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [undo, redo, handleCopy, handleCut, handlePaste, canCopy, canPaste]);

  // ── 键盘导航（Delete/方向键/Tab） ──
  // 非组合键，仅在编辑器内、非输入框焦点时生效
  useEffect(() => {
    if (isPreviewMode) { return; }
    const handler = (e: KeyboardEvent) => {
      // 编辑器外不响应
      const editorEl = editorRef.current;
      if (!editorEl) { return; }
      const active = document.activeElement;
      if (active && active !== document.body && !editorEl.contains(active)) {
        return;
      }

      const target = e.target as HTMLElement;
      const inFormField = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;
      if (inFormField) { return; }

      // 拍平树用于方向键导航
      const flat = flattenNodes(internalSchema);

      switch (e.key) {
        case "Delete":
        case "Backspace": {
          if (!selectedId || selectedId === internalSchema.id) { return; }
          e.preventDefault();
          commit((prev) => removeNodeById(prev, selectedId));
          // 选中前一个节点
          const idx = flat.findIndex((n) => n.id === selectedId);
          const next = idx > 0 ? flat[idx - 1] : internalSchema;
          setSelectedId(next.id);
          break;
        }
        case "ArrowUp": {
          e.preventDefault();
          const idx = flat.findIndex((n) => n.id === (selectedId ?? internalSchema.id));
          if (idx > 0) {
            setSelectedId(flat[idx - 1].id);
          }
          break;
        }
        case "ArrowDown": {
          e.preventDefault();
          const idx = flat.findIndex((n) => n.id === (selectedId ?? internalSchema.id));
          if (idx >= 0 && idx < flat.length - 1) {
            setSelectedId(flat[idx + 1].id);
          }
          break;
        }
        case "Tab": {
          // Tab: 选中第一个子节点（进入子层）
          // Shift+Tab: 选中父节点（回到父层）
          e.preventDefault();
          if (e.shiftKey) {
            const currentId = selectedId ?? internalSchema.id;
            const parent = findParent(internalSchema, currentId);
            if (parent) {
              setSelectedId(parent.id);
            }
          } else {
            const current = selectedId
              ? findNodeById(internalSchema, selectedId)
              : internalSchema;
            if (current?.children && current.children.length > 0) {
              setSelectedId(current.children[0].id);
            }
          }
          break;
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [internalSchema, selectedId, isPreviewMode, commit]);

  // 预览模式切换
  const handleTogglePreview = useCallback(() => {
    setIsPreviewMode((v) => !v);
    // 进入预览时清空选中，避免隐藏的属性面板仍写入不可见节点
    setSelectedId(null);
  }, []);

  // 删除当前选中节点（属性面板顶部的删除按钮）
  const handleDeleteSelected = useCallback(() => {
    if (!selectedId || isRootSelected) { return; }
    commit((prev) => removeNodeById(prev, selectedId));
    setSelectedId(null);
  }, [commit, selectedId, isRootSelected]);

  // Outline 面板的删除：直接按 id 删除（可删除非选中节点）
  const handleOutlineDelete = useCallback((id: string) => {
    if (id === internalSchema.id) { return; } // 根节点不可删
    commit((prev) => removeNodeById(prev, id));
    if (selectedId === id) {
      setSelectedId(null);
    }
  }, [commit, internalSchema.id, selectedId]);

  // Outline 面板的复制：克隆节点 + 重生成 id，插入到原节点之后
  const handleOutlineDuplicate = useCallback((id: string) => {
    if (id === internalSchema.id) { return; } // 根节点不可复制
    commit((prev) => {
      const source = findNodeById(prev, id);
      if (!source) { return prev; }
      const cloned = cloneSchemaWithNewIds(source);
      const parent = findParent(prev, id);
      if (!parent) { return prev; }
      const idx = parent.children?.findIndex((c) => c.id === id) ?? -1;
      if (idx >= 0 && parent.children) {
        const nextChildren = [...parent.children];
        nextChildren.splice(idx + 1, 0, cloned);
        // 更新父节点
        return updateNodeById(prev, parent.id, (n) => {
          n.children = nextChildren;
        });
      }
      return prev;
    });
  }, [commit, internalSchema.id]);

  // 预览模式切换

  return (
    <div
      ref={editorRef}
      className={`relative flex flex-col border rounded-lg overflow-hidden transition-all duration-300 ${
        isFullscreen ? "fixed inset-0 z-50 bg-white dark:bg-gray-900 h-auto w-auto rounded-none border-0 p-4" : "h-130"
      }`}
    >
      {/* 工具栏 */}
      <EditorToolbar
        canUndo={canUndo}
        canRedo={canRedo}
        canCopy={canCopy}
        canPaste={canPaste}
        isPreviewMode={isPreviewMode}
        onUndo={undo}
        onRedo={redo}
        onCopy={handleCopy}
        onCut={handleCut}
        onPaste={handlePaste}
        onExport={handleExport}
        onImport={handleImport}
        onTogglePreview={handleTogglePreview}
        viewport={viewport}
        onViewportChange={setViewport}
        t={t}
      />

      {/* 全屏切换按钮 */}
      <button
        type="button"
        onClick={() => setIsFullscreen((v) => !v)}
        className="absolute top-12 right-2 z-20 flex items-center gap-1 text-xs px-2.5 py-1.5 rounded
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
        <div className="flex flex-1 overflow-hidden">
          {/* 左侧：组件面板 + 大纲视图（预览模式隐藏） */}
          {!isPreviewMode && (
            <div className="w-50 border-r border-gray-200 dark:border-gray-700 shrink-0 bg-gray-50 dark:bg-gray-800 overflow-y-auto flex flex-col">
              <div className="px-3 py-2 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-gray-50 dark:bg-gray-800 z-10">
                <h4 className="text-xs font-semibold text-gray-600 dark:text-gray-300 flex items-center gap-1.5">
                  <AppstoreAddOutlined size={12} />
                  {t("visualEditor.palette.title")}
                </h4>
              </div>
              <div className="flex-1 overflow-y-auto">
                <ComponentPalette t={t} />
              </div>
              {/* Outline 视图：固定在底部，展示完整 schema 树 */}
              <div className="border-t border-gray-200 dark:border-gray-700 max-h-60 shrink-0 flex flex-col">
                <div className="px-3 py-1.5 border-b border-gray-200 dark:border-gray-700 text-[10px] uppercase tracking-wide text-gray-400 dark:text-gray-500">
                  {t("visualEditor.outline.title")}
                </div>
                <div className="flex-1 overflow-y-auto">
                  <OutlinePanel
                    root={internalSchema}
                    selectedId={selectedId}
                    onSelect={handleSelectNode}
                    onDelete={handleOutlineDelete}
                    onDuplicate={handleOutlineDuplicate}
                    t={t}
                  />
                </div>
              </div>
            </div>
          )}

          {/* 中间：画布 */}
          <VisualCanvas
            schema={internalSchema}
            selectedId={isRootSelected ? null : selectedId}
            onSelectNode={handleSelectNode}
            isPreviewMode={isPreviewMode}
            viewport={viewport}
            onAction={isPreviewMode ? handlePreviewAction : undefined}
            t={t}
          />

          {/* 右侧：属性面板（预览模式隐藏） */}
          {!isPreviewMode && (
            <div className="w-65 border-l border-gray-200 dark:border-gray-700 shrink-0 overflow-hidden flex flex-col">
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
                    onClick={handleDeleteSelected}
                  >
                    {t("visualEditor.property.delete")}
                  </Button>
                )}
              </div>
              <div className="flex-1 overflow-y-auto">
                <EditorErrorBoundary t={t}>
                  <PropertyPanel
                    selectedNode={selectedNode ?? null}
                    isRoot={isRootSelected}
                    onUpdateNode={handleUpdateSelectedNode}
                    defaultPropsMap={defaultPropsMap}
                    t={t}
                  />
                </EditorErrorBoundary>
              </div>
            </div>
          )}
        </div>

        {/* Drag Overlay（预览模式禁用拖拽时不会触发） */}
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
