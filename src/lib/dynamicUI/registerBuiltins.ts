// SPDX-License-Identifier: AGPL-3.0-only

import { componentRegistry } from "./ComponentRegistry";
import type { ComponentRegistryEntry } from "@/types";

import { RowContainer } from "@/components/dynamicUI/containers/RowContainer";
import { ColumnContainer } from "@/components/dynamicUI/containers/ColumnContainer";
import { GridContainer } from "@/components/dynamicUI/containers/GridContainer";
import { CardContainer } from "@/components/dynamicUI/containers/CardContainer";
import { TabsContainer } from "@/components/dynamicUI/containers/TabsContainer";
import { AccordionContainer } from "@/components/dynamicUI/containers/AccordionContainer";

import { DataTable } from "@/components/dynamicUI/data/DataTable";
import { ChartRenderer } from "@/components/dynamicUI/data/ChartRenderer";
import { Dashboard } from "@/components/dynamicUI/data/Dashboard";
import { ListView } from "@/components/dynamicUI/data/ListView";
import { TreeView } from "@/components/dynamicUI/data/TreeView";
import { TimelineView } from "@/components/dynamicUI/data/TimelineView";

import { FormRenderer } from "@/components/dynamicUI/form/FormRenderer";
import {
  InputField,
  SelectField,
  DatePickerField,
  SwitchField,
  TextareaField,
} from "@/components/dynamicUI/form/FormFields";

import { CodeEditorView } from "@/components/dynamicUI/media/CodeEditorView";
import { FilePreviewView } from "@/components/dynamicUI/media/FilePreviewView";
import { MarkdownView } from "@/components/dynamicUI/media/MarkdownView";

import {
  DynamicButton,
  DynamicText,
  DynamicDivider,
  DynamicProgress,
  DynamicTag,
  DynamicImage,
} from "@/components/dynamicUI/misc/MiscComponents";

/**
 * 内置组件注册配置。
 * 在应用启动时调用 registerAllBuiltins() 注册所有内置组件。
 */
const BUILTIN_COMPONENTS: ComponentRegistryEntry[] = [
  // ── Container Components ──
  {
    type: "Container",
    component: RowContainer,
    category: "container",
    label: "容器",
    defaultProps: {},
  },
  {
    type: "Row",
    component: RowContainer,
    category: "container",
    label: "行布局",
    defaultProps: { gap: 8, align: "center" },
  },
  {
    type: "Column",
    component: ColumnContainer,
    category: "container",
    label: "列布局",
    defaultProps: { gap: 8 },
  },
  {
    type: "Grid",
    component: GridContainer,
    category: "container",
    label: "网格布局",
    defaultProps: { columns: 2, gap: 16 },
  },
  {
    type: "Card",
    component: CardContainer,
    category: "container",
    label: "卡片",
    defaultProps: { bordered: true },
  },
  {
    type: "Tabs",
    component: TabsContainer,
    category: "container",
    label: "标签页",
    defaultProps: { tabPosition: "top" },
  },
  {
    type: "Accordion",
    component: AccordionContainer,
    category: "container",
    label: "手风琴",
    defaultProps: { accordion: true },
  },

  // ── Data Display Components ──
  {
    type: "Table",
    component: DataTable,
    category: "data-display",
    label: "数据表格",
    defaultProps: { columns: [], showHeader: true, size: "middle" },
  },
  {
    type: "Chart",
    component: ChartRenderer,
    category: "data-display",
    label: "图表",
    defaultProps: { chartType: "bar" },
  },
  {
    type: "Dashboard",
    component: Dashboard,
    category: "data-display",
    label: "仪表盘",
    defaultProps: { columns: 3, gap: 16 },
  },
  {
    type: "List",
    component: ListView,
    category: "data-display",
    label: "列表",
    defaultProps: { itemLayout: "vertical", size: "default" },
  },
  {
    type: "Tree",
    component: TreeView,
    category: "data-display",
    label: "树形控件",
    defaultProps: { checkable: false, showLine: false },
  },
  {
    type: "Timeline",
    component: TimelineView,
    category: "data-display",
    label: "时间线",
    defaultProps: {},
  },

  // ── Form Components ──
  {
    type: "Form",
    component: FormRenderer,
    category: "form",
    label: "表单",
    defaultProps: { layout: "vertical", submitText: "提交" },
  },
  {
    type: "Input",
    component: InputField,
    category: "form",
    label: "输入框",
    defaultProps: { type: "text" },
  },
  {
    type: "Select",
    component: SelectField,
    category: "form",
    label: "下拉选择",
    defaultProps: { options: [] },
  },
  {
    type: "DatePicker",
    component: DatePickerField,
    category: "form",
    label: "日期选择器",
    defaultProps: {},
  },
  {
    type: "Switch",
    component: SwitchField,
    category: "form",
    label: "开关",
    defaultProps: {},
  },
  {
    type: "Textarea",
    component: TextareaField,
    category: "form",
    label: "文本域",
    defaultProps: { rows: 4 },
  },

  // ── Media Components ──
  {
    type: "CodeEditor",
    component: CodeEditorView,
    category: "media",
    label: "代码编辑器",
    defaultProps: { language: "plaintext", readOnly: false, height: "300px" },
  },
  {
    type: "FilePreview",
    component: FilePreviewView,
    category: "media",
    label: "文件预览",
    defaultProps: {},
  },
  {
    type: "Markdown",
    component: MarkdownView,
    category: "media",
    label: "Markdown",
    defaultProps: { content: "" },
  },
  {
    type: "Image",
    component: DynamicImage,
    category: "media",
    label: "图片",
    defaultProps: { preview: true },
  },

  // ── Misc Components ──
  {
    type: "Button",
    component: DynamicButton,
    category: "misc",
    label: "按钮",
    defaultProps: { text: "按钮", type: "default" },
  },
  {
    type: "Text",
    component: DynamicText,
    category: "misc",
    label: "文本",
    defaultProps: { content: "" },
  },
  {
    type: "Divider",
    component: DynamicDivider,
    category: "misc",
    label: "分割线",
    defaultProps: {},
  },
  {
    type: "Progress",
    component: DynamicProgress,
    category: "misc",
    label: "进度条",
    defaultProps: { percent: 0, type: "line" },
  },
  {
    type: "Tag",
    component: DynamicTag,
    category: "misc",
    label: "标签",
    defaultProps: { text: "", color: "blue" },
  },
];

/**
 * 注册所有内置组件到全局注册表。
 * 应在应用启动时（如 main.tsx 或 App.tsx 初始化）调用一次。
 */
export function registerAllBuiltins(): void {
  componentRegistry.registerBatch(BUILTIN_COMPONENTS);
}
