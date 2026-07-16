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

/**
 * 深拷贝 UISchema 并递归重新生成所有节点 id。
 *
 * 用于"复制/粘贴"场景：粘贴时如果保留原 id 会导致树中出现重复 id，
 * 进而使 findNodeById / removeNodeById 等基于 id 的操作产生歧义。
 * 因此粘贴前必须给整棵子树重新分配 id。
 *
 * 新 id 保留原 prefix（取原 id 的第一段），避免污染命名空间。
 */
export function cloneSchemaWithNewIds(node: UISchema): UISchema {
  const cloned = cloneSchema(node);
  regenIdsInPlace(cloned);
  return cloned;
}

/** 递归重新生成节点 id（in-place 修改） */
function regenIdsInPlace(node: UISchema): void {
  const prefix = node.id.split("-")[0] ?? "node";
  node.id = genId(prefix);
  if (node.children) {
    for (const child of node.children) {
      regenIdsInPlace(child);
    }
  }
}

/** 根据 id 递归查找节点（DFS），找不到时返回 null */
export function findNodeById(root: UISchema, id: string): UISchema | null {
  if (root.id === id) {
    return root;
  }
  if (!root.children) {
    return null;
  }
  for (const child of root.children) {
    const found = findNodeById(child, id);
    if (found) {
      return found;
    }
  }
  return null;
}

/** 递归删除指定 id 的节点（返回新树，找不到时返回原树） */
export function removeNodeById(root: UISchema, id: string): UISchema {
  if (!root.children) {
    return root;
  }
  const filtered = root.children.filter((c) => c.id !== id);
  const nextChildren = filtered.map((c) => removeNodeById(c, id));
  if (nextChildren.length !== root.children.length || nextChildren.some((c, i) => c !== filtered[i])) {
    return { ...root, children: nextChildren };
  }
  return root;
}

/**
 * 从树中移除指定 id 的节点（返回新树，找不到时返回原树）。
 * 用于"跨容器移动"场景：先从原位置摘除节点，再 append 到目标容器。
 */
export function detachNodeById(root: UISchema, id: string): { tree: UISchema; detached: UISchema | null } {
  if (root.id === id) {
    // 根节点不能被摘除
    return { tree: root, detached: null };
  }
  if (!root.children) {
    return { tree: root, detached: null };
  }
  const detachedIdx = root.children.findIndex((c) => c.id === id);
  if (detachedIdx >= 0) {
    const detached = root.children[detachedIdx];
    const nextChildren = root.children.filter((_, i) => i !== detachedIdx);
    return { tree: { ...root, children: nextChildren }, detached };
  }
  // 递归在子树中查找
  for (let i = 0; i < root.children.length; i++) {
    const child = root.children[i];
    const result = detachNodeById(child, id);
    if (result.detached) {
      const nextChildren = [...root.children];
      nextChildren[i] = result.tree;
      return { tree: { ...root, children: nextChildren }, detached: result.detached };
    }
  }
  return { tree: root, detached: null };
}

/** 在指定父节点下插入子节点（默认追加到末尾） */
export function appendChildToNode(root: UISchema, parentId: string, child: UISchema): UISchema {
  if (root.id === parentId) {
    return { ...root, children: [...(root.children ?? []), child] };
  }
  if (!root.children) {
    return root;
  }
  let changed = false;
  const nextChildren = root.children.map((c) => {
    const result = appendChildToNode(c, parentId, child);
    if (result !== c) {
      changed = true;
      return result;
    }
    return c;
  });
  return changed ? { ...root, children: nextChildren } : root;
}

/** 在指定父节点下，把 childId 插入到 siblingId 之前（用于精确位置插入） */
export function insertBeforeSibling(
  root: UISchema,
  parentId: string,
  child: UISchema,
  siblingId: string,
): UISchema {
  if (root.id === parentId && root.children) {
    const idx = root.children.findIndex((c) => c.id === siblingId);
    if (idx >= 0) {
      const nextChildren = [...root.children];
      nextChildren.splice(idx, 0, child);
      return { ...root, children: nextChildren };
    }
    // 找不到 sibling，退化为 append
    return { ...root, children: [...root.children, child] };
  }
  if (!root.children) {
    return root;
  }
  let changed = false;
  const nextChildren = root.children.map((c) => {
    const result = insertBeforeSibling(c, parentId, child, siblingId);
    if (result !== c) {
      changed = true;
      return result;
    }
    return c;
  });
  return changed ? { ...root, children: nextChildren } : root;
}

/**
 * 收集整棵树的所有节点，按 DFS 序返回。
 * 用于键盘导航（方向键在节点间切换）。
 */
export function flattenNodes(root: UISchema): UISchema[] {
  const result: UISchema[] = [root];
  if (root.children) {
    for (const child of root.children) {
      result.push(...flattenNodes(child));
    }
  }
  return result;
}

/** 查找直接父节点（找不到时返回 null，root 无父节点） */
export function findParent(root: UISchema, childId: string): UISchema | null {
  if (!root.children) {
    return null;
  }
  if (root.children.some((c) => c.id === childId)) {
    return root;
  }
  for (const child of root.children) {
    const found = findParent(child, childId);
    if (found) {
      return found;
    }
  }
  return null;
}

/**
 * 判断 targetId 是否是 source 的子孙节点。
 * 用于跨容器拖拽时防止把父节点拖入自己的子节点（会形成环）。
 */
export function isDescendant(source: UISchema, targetId: string): boolean {
  if (!source.children) {
    return false;
  }
  for (const child of source.children) {
    if (child.id === targetId) {
      return true;
    }
    if (isDescendant(child, targetId)) {
      return true;
    }
  }
  return false;
}

/**
 * 按 id 查找节点并就地应用 updater（返回新树，不变时返回原树）。
 * 用于属性面板更新某个节点的 props / events / dataSource 等。
 */
export function updateNodeById(
  root: UISchema,
  id: string,
  updater: (node: UISchema) => void,
): UISchema {
  if (root.id === id) {
    const next = cloneSchema(root);
    updater(next);
    return next;
  }
  if (!root.children) {
    return root;
  }
  let changed = false;
  const nextChildren = root.children.map((c) => {
    const result = updateNodeById(c, id, updater);
    if (result !== c) {
      changed = true;
      return result;
    }
    return c;
  });
  return changed ? { ...root, children: nextChildren } : root;
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
