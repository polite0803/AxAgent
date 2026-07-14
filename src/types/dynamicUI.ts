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
  | "Number"
  | "Select"
  | "DatePicker"
  | "Switch"
  | "Checkbox"
  | "Radio"
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
  field: string;
  operator:
    | "eq"
    | "neq"
    | "gt"
    | "gte"
    | "lt"
    | "lte"
    | "in"
    | "contains"
    | "exists"
    | "empty";
  value?: unknown;
}

export type ConditionalDisplay =
  | ConditionalRule[]
  | {
    logic: "and" | "or";
    rules: ConditionalDisplay[];
    not?: boolean;
  };

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
  /** 条件显示规则（支持AND/OR/NOT逻辑组合） */
  conditionalDisplay?: ConditionalDisplay;
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
  "Number",
  "Select",
  "DatePicker",
  "Switch",
  "Checkbox",
  "Radio",
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
  Number: [],
  Select: [],
  DatePicker: [],
  Switch: [],
  Checkbox: [],
  Radio: [],
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

// ── 持久化 Schema 相关类型 ──

export interface DynamicUISchemaRecord {
  id: string;
  title: string;
  description: string;
  schema_json: string;
  category: string;
  tags: string[];
  version: string;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
}

export interface DynamicUIFormDataRecord {
  id: string;
  schema_id: string;
  form_data_json: string;
  instance_key: string;
  updated_at: string;
}

export interface CreateDynamicUISchemaParams {
  title: string;
  description: string;
  schema_json: string;
  category: string;
  tags: string[];
}

export interface UpdateDynamicUISchemaParams {
  title?: string;
  description?: string;
  schema_json?: string;
  category?: string;
  tags?: string[];
  /** 语义化版本号（可选），不传则自动递增 patch */
  version?: string;
  /** 变更说明 */
  change_log?: string;
}

export interface SaveDynamicUIFormDataParams {
  schema_id: string;
  form_data_json: string;
  instance_key?: string;
}

// ── 导航钉入配置类型 ──

export interface DynamicUIPinRecord {
  schema_id: string;
  title: string;
  group_name: string;
  position: number;
  created_at: string;
  updated_at: string;
}

export interface PinDynamicUISchemaParams {
  schema_id: string;
  title: string;
  group_name: string;
  position?: number;
}

export interface UpdateDynamicUIPinParams {
  title?: string;
  group_name?: string;
  position?: number;
}

// ── 版本管理类型 ──

export interface DynamicUISchemaVersion {
  id: number;
  schema_id: string;
  version: string;
  title: string;
  description: string;
  schema_json: string;
  category: string;
  tags: string[];
  change_log: string;
  created_at: number;
}

export interface ListVersionsResponse {
  versions: DynamicUISchemaVersion[];
  current_version: string;
}
