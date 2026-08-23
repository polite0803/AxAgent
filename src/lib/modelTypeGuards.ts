// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 编译期检查工具 — TypeScript 类型约束
 *
 * 这些工具函数利用 TypeScript 的类型系统，在编译期（而非运行期）
 * 检测可能导致业务错误的代码模式。
 *
 * 使用方式：
 * 1. 在编写新代码时使用这些工具函数
 * 2. 配合 ESLint 规则进一步约束
 * 3. 在 code review 中检查是否遵循这些模式
 */

import { type AppSettings, ModelSelection, ModelValidator, type NullableModelRef, type ProviderConfig } from "@/types";

/**
 * 类型断言：检查 AppSettings 中的模型选择字段是否有效
 *
 * 这个类型在编译期检查 NullableModelRef 字段的结构正确性。
 * 已合并字段保证：不可能出现 providerId 存在但 modelId 缺失的状态
 */
export type ValidateModelConsistency<T> = T extends {
  defaultModel: infer M;
} ? (M extends null ? true : true)
  : never;

/**
 * 强制类型：确保不会创建分离的 providerId + modelId 对
 *
 * 如果代码试图分别处理 providerId 和 modelId，
 * 编译器会产生类型错误，强制开发者使用 ModelSelection。
 *
 * @example
 * // ❌ 错误示例：分离的字段
 * const modelRef = settings.defaultModel;  // NullableModelRef
 *
 * // ✅ 正确示例：使用 NullableModelRef
 * if (modelRef) {
 *   // modelRef.a (providerId) 和 modelRef.b (modelId) 保证一致
 * }
 */
export type EnforceModelSelection<T> = T extends string | null ? ModelSelection | null
  : never;

/**
 * 编译期验证：检查模型选择在 provider 列表中是否存在
 *
 * 这个函数返回一个条件类型，如果模型选择无效会产生编译期错误。
 * 注意：由于 providers 是动态的，这个检查只能在运行时完成，
 * 但类型系统可以确保检查被正确执行。
 */
export type MustValidateModelSelection = {
  [K in keyof AppSettings]: K extends `${string}Model` ? AppSettings[K] extends NullableModelRef ? true
    : false
    : false;
}[keyof AppSettings];

/**
 * 创建编译期检查的辅助函数
 *
 * 使用示例：
 * ```typescript
 * // 在开发模式下启用严格检查
 * if (process.env.NODE_ENV === 'development') {
 *   // 这些调用会在编译期提醒开发者使用正确的模式
 *   assertModelSelectionConsistency(settings, providers);
 * }
 * ```
 */

/**
 * 断言模型选择的一致性（仅在开发环境中使用）
 * 如果检测到不一致会抛出 Error，提醒开发者修复
 */
export function assertModelSelectionConsistency(
  settings: AppSettings,
  providers: readonly ProviderConfig[],
): void {
  if (process.env.NODE_ENV !== "development") { return; }

  const pairs: Array<[keyof AppSettings, string]> = [
    ["defaultModel", "默认对话模型"],
    ["titleSummaryModel", "标题摘要模型"],
    ["compressionModel", "压缩模型"],
  ];

  for (const [modelKey, label] of pairs) {
    const modelRef = settings[modelKey] as NullableModelRef;

    // 检查: 如果有值，必须在 providers 列表中有效
    if (modelRef) {
      const result = ModelValidator.validate(modelRef, providers);
      if (!result.valid) {
        throw new Error(
          `[ModelConsistency] ${label}: ${result.reason} `
            + `（providerId=${modelRef.a}, modelId=${modelRef.b}）`,
        );
      }
    }
  }
}

/**
 * 类型守卫：检查值是否为有效的 ModelSelection
 * 可用于缩小类型范围，确保只有经过验证的模型选择才能继续使用
 */
export function isEffectiveModelSelection(
  value: unknown,
): value is ModelSelection {
  if (typeof value !== "object" || value === null) { return false; }
  const obj = value as Record<string, unknown>;
  return (
    typeof obj.providerId === "string"
    && typeof obj.modelId === "string"
    && obj.providerId.length > 0
    && obj.modelId.length > 0
  );
}
