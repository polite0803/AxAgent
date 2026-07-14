// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicComponentType, UISchema } from "@/types";

/**
 * 使用点号分隔的路径获取嵌套对象值。
 * 如 "user.profile.name" -> obj.user.profile.name
 */
export function getNestedValue(
  obj: Record<string, unknown>,
  path: string,
): unknown {
  const keys = path.split(".");
  let current: unknown = obj;
  for (const key of keys) {
    if (current === null || current === undefined) {
      return undefined;
    }
    if (typeof current !== "object") {
      return undefined;
    }
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}

/** 生成唯一 id */
let idCounter = 0;
export function genId(prefix: string): string {
  idCounter += 1;
  return `${prefix}-${Date.now()}-${idCounter}`;
}

/** 深拷贝 UISchema */
export function cloneSchema(node: UISchema): UISchema {
  return {
    ...node,
    props: { ...node.props },
    children: node.children ? node.children.map(cloneSchema) : undefined,
    events: node.events
      ? node.events.map((e) => ({ ...e, actions: [...e.actions] }))
      : undefined,
    dataSource: node.dataSource
      ? { ...node.dataSource, config: { ...node.dataSource.config } }
      : undefined,
    conditionalDisplay: node.conditionalDisplay
      ? Array.isArray(node.conditionalDisplay)
        ? [...node.conditionalDisplay]
        : { ...node.conditionalDisplay, rules: [...node.conditionalDisplay.rules] }
      : undefined,
    style: node.style ? { ...node.style } : undefined,
  };
}

/** 根据 i18n t 函数生成组件默认 props */
export function getDefaultProps(
  t: (key: string) => string,
): Partial<Record<DynamicComponentType, Record<string, unknown>>> {
  return {
    Input: {
      name: "field",
      label: t("dynamicUIManager.defaults.inputLabel"),
      placeholder: t("dynamicUIManager.defaults.inputPlaceholder"),
    },
    Number: { name: "number", label: t("dynamicUIManager.defaults.numberLabel") },
    Select: { name: "select", label: t("dynamicUIManager.defaults.selectLabel"), options: [] },
    DatePicker: { name: "date", label: t("dynamicUIManager.defaults.dateLabel") },
    Switch: { name: "enabled", label: t("dynamicUIManager.defaults.switchLabel") },
    Checkbox: { name: "checked", label: t("dynamicUIManager.defaults.checkboxLabel") },
    Radio: { name: "option", label: t("dynamicUIManager.defaults.radioLabel"), options: [] },
    Textarea: {
      name: "text",
      label: t("dynamicUIManager.defaults.textareaLabel"),
      placeholder: t("dynamicUIManager.defaults.inputPlaceholder"),
    },
    Button: { text: t("dynamicUIManager.defaults.buttonText") },
    Text: { content: t("dynamicUIManager.defaults.textContent") },
    Form: { layout: "vertical", submitText: t("dynamicUIManager.defaults.formSubmit") },
    Card: { title: t("dynamicUIManager.defaults.cardTitle") },
    Container: {},
    Row: {},
    Column: {},
    Grid: { columns: 3 },
    Tabs: {},
    Accordion: {},
    Table: { columns: [] },
    Chart: { chartType: "line" },
    List: {},
    Dashboard: { items: [] },
    CodeEditor: {},
    FilePreview: {},
    Markdown: { content: "" },
    Image: { src: "" },
    Divider: {},
    Progress: { percent: 0 },
    Tag: { text: t("dynamicUIManager.defaults.tagText") },
    Tree: { treeData: [] },
    Timeline: { items: [] },
  };
}

/**
 * 解析动态数据展示组件（Table / Chart / List / Dashboard / Tree / Timeline）的数组数据。
 *
 * 取值优先级：
 * 1. 显式传入的 props 数组（内联静态数据）；
 * 2. 运行时经 dataSource 解析后注入到 `dataContext[schemaId]` 的数据
 *    （由 DynamicUIRenderer 在处理 dataSource 后写入该路径）。
 *
 * 二者皆缺失或非数组时返回空数组，避免组件因 undefined 数据崩溃。
 */
export function resolveDynamicArray<T = Record<string, unknown>>(
  explicit: T[] | undefined | null,
  dataContext: Record<string, unknown> | undefined,
  schemaId: string,
): T[] {
  if (Array.isArray(explicit)) {
    return explicit;
  }
  const fromContext = dataContext?.[schemaId];
  if (Array.isArray(fromContext)) {
    return fromContext as T[];
  }
  return [];
}
