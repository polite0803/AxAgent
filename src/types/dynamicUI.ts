// SPDX-License-Identifier: AGPL-3.0-only

import type React from "react";

// ── 组件类型枚举 ──
export type DynamicComponentType =
  | "Container"
  | "Row"
  | "Column"
  | "Grid"
  | "Card"
  | "Tabs"
  | "Accordion"
  | "Form"
  | "Input"
  | "Select"
  | "DatePicker"
  | "Switch"
  | "Textarea"
  | "Table"
  | "Chart"
  | "List"
  | "Dashboard"
  | "CodeEditor"
  | "FilePreview"
  | "Markdown"
  | "Image"
  | "Button"
  | "Text"
  | "Divider"
  | "Progress"
  | "Tag"
  | "Tree"
  | "Timeline";

// ── 数据源配置 ──
export interface DataSourceConfig {
  /** 数据源类型 */
  type: "store" | "api" | "static" | "agent-generated";
  /** 各类型对应的配置 */
  config: Record<string, unknown>;
  /** 轮询间隔 ms，0 表示不轮询 */
  polling?: number;
}

// ── 事件处理器 ──
export interface EventHandler {
  /** 触发时机 */
  trigger: "onClick" | "onChange" | "onSubmit" | "onMount" | "onUnmount";
  /** 触发后执行的动作列表 */
  actions: DynamicAction[];
}

// ── 动态动作（兼容 ActionRouter 体系） ──
export interface DynamicAction {
  type:
    | "invoke"
    | "navigate"
    | "emit"
    | "store"
    | "function"
    | "chain"
    | "update-schema";
  config: Record<string, unknown>;
}

// ── 条件渲染规则 ──
export interface ConditionalRule {
  /** 数据上下文字段名 */
  field: string;
  /** 比较操作符 */
  operator:
    | "eq"
    | "neq"
    | "gt"
    | "gte"
    | "lt"
    | "lte"
    | "in"
    | "contains";
  /** 比较值 */
  value: unknown;
}

// ── UI Schema 顶层结构 ──
export interface UISchema {
  /** Schema 版本号 */
  version: string;
  /** 组件唯一标识 */
  id: string;
  /** 组件类型 */
  type: DynamicComponentType;
  /** 组件属性 */
  props: Record<string, unknown>;
  /** 子组件 */
  children?: UISchema[];
  /** 数据源配置 */
  dataSource?: DataSourceConfig;
  /** 事件处理器 */
  events?: EventHandler[];
  /** 条件显示规则（全部满足才渲染） */
  conditionalDisplay?: ConditionalRule[];
  /** 样式覆盖 */
  style?: Record<string, string | number>;
}

// ── 组件注册表项 ──
export interface ComponentRegistryEntry {
  /** 组件类型 */
  type: DynamicComponentType;
  /** React 组件 */
  component: React.ComponentType<DynamicUIProps>;
  /** 组件分类 */
  category: "container" | "data-display" | "form" | "media" | "misc";
  /** 显示标签 */
  label: string;
  /** 默认属性 */
  defaultProps?: Record<string, unknown>;
}

// ── DynamicUIRenderer 接收的 props ──
export interface DynamicUIProps {
  /** UI Schema */
  schema: UISchema;
  /** 外部注入的数据上下文 */
  dataContext?: Record<string, unknown>;
  /** 动作回调 */
  onAction?: (action: DynamicAction) => void;
  /** 子节点 */
  children?: React.ReactNode;
}

// ── Schema 校验结果 ──
export interface SchemaValidationResult {
  /** 是否通过校验 */
  valid: boolean;
  /** 校验错误列表 */
  errors: SchemaValidationError[];
}

export interface SchemaValidationError {
  /** 错误路径（如 root.children[0].props） */
  path: string;
  /** 错误描述 */
  message: string;
}

// ── 有效的 DynamicComponentType 集合 ──
export const VALID_DYNAMIC_COMPONENT_TYPES: ReadonlySet<string> = new Set<DynamicComponentType>([
  "Container",
  "Row",
  "Column",
  "Grid",
  "Card",
  "Tabs",
  "Accordion",
  "Form",
  "Input",
  "Select",
  "DatePicker",
  "Switch",
  "Textarea",
  "Table",
  "Chart",
  "List",
  "Dashboard",
  "CodeEditor",
  "FilePreview",
  "Markdown",
  "Image",
  "Button",
  "Text",
  "Divider",
  "Progress",
  "Tag",
  "Tree",
  "Timeline",
]);

// ── 组件类型 → 必填 props 映射（用于 Schema 校验） ──
export const COMPONENT_REQUIRED_PROPS: Readonly<
  Record<DynamicComponentType, string[]>
> = {
  Container: [],
  Row: [],
  Column: [],
  Grid: ["columns"],
  Card: [],
  Tabs: [],
  Accordion: [],
  Form: [],
  Input: [],
  Select: [],
  DatePicker: [],
  Switch: [],
  Textarea: [],
  Table: ["columns"],
  Chart: ["chartType"],
  List: [],
  Dashboard: ["items"],
  CodeEditor: [],
  FilePreview: [],
  Markdown: [],
  Image: [],
  Button: [],
  Text: [],
  Divider: [],
  Progress: [],
  Tag: [],
  Tree: ["treeData"],
  Timeline: ["items"],
};
