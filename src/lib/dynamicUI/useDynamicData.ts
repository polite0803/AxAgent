// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Hook: 统一解析动态组件数据。
 *
 * 各数据类组件的数据获取遵循同一三级回退模式：
 *   1. static data（schema.props 中直接写入的数据）
 *   2. dataContext[schemaId]（由 DynamicUIRenderer 通过 DataSource 注入）
 *   3. 空数组 []
 *
 * 本 hook 封装此模式，消除 6 个组件中的重复代码。
 */

export function useDynamicData<T = Record<string, unknown>>(
  staticData: T[] | undefined,
  schemaId: string,
  dataContext?: Record<string, unknown>,
): T[] {
  if (staticData && staticData.length > 0) {
    return staticData;
  }

  const contextData = dataContext?.[schemaId];
  if (Array.isArray(contextData) && contextData.length > 0) {
    return contextData as T[];
  }

  return [];
}
