// SPDX-License-Identifier: AGPL-3.0-only

import { createContext, useContext } from "react";

/**
 * 提供当前渲染的 DynamicUI Schema 在数据库中的记录 id。
 *
 * 用于表单数据持久化等需要关联到具体 schema 记录的场景：
 * FormRenderer 通过此 context 获取 schemaId，从而把表单填写结果
 * 保存到后端（save_dynamic_ui_form_data）并在打开时回填。
 *
 * 页面层（DynamicPageViewer / DynamicUIManagerPage / DynamicPagesSettings）
 * 在渲染 DynamicUIRenderer 时用 Provider 注入 schemaId。
 */
export interface SchemaIdContextValue {
  /** 数据库中的 schema 记录 id；非持久化场景（如可视化编辑器预览）为 null */
  schemaId: string | null;
}

export const SchemaIdContext = createContext<SchemaIdContextValue>({ schemaId: null });

export function useSchemaId(): SchemaIdContextValue {
  return useContext(SchemaIdContext);
}
