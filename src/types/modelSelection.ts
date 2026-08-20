// SPDX-License-Identifier: AGPL-3.0-only

/**
 * ModelSelection — 绑定 providerId + modelId 的强类型
 *
 * ⚠️ 注意：本模块已迁移到使用 paired.ts 中的类型。
 * ModelSelection 现在是 ModelRef 的别名，保持向后兼容。
 *
 * 编译期保证：
 * 1. 结构一致性：providerId 和 modelId 要么同时存在，要么同时为 null
 * 2. 强制空值检查：所有访问 ModelSelection 的代码必须处理 null 情况
 * 3. 防止分离访问：禁止独立处理 providerId 和 modelId（通过工具函数强制）
 */

import type { ProviderConfig } from "./index";
import { ModelRef, type NullableModelRef } from "./paired";

export { ModelRef as ModelSelection };
export type { NullableModelRef as NullableModelSelection };

/**
 * 验证结果
 */
export interface ValidationResult {
  valid: boolean;
  reason?: string;
}

/**
 * 批量验证工具
 */
export const ModelValidator = {
  /** 验证单个 ModelSelection */
  validate(
    sel: NullableModelRef,
    providers: readonly ProviderConfig[],
  ): ValidationResult {
    if (!sel) { return { valid: true }; }

    const provider = providers.find((p) => p.id === sel.a);
    if (!provider) {
      return { valid: false, reason: `Provider "${sel.a}" 不存在` };
    }
    if (!provider.enabled) {
      return { valid: false, reason: `Provider "${sel.a}" 已禁用` };
    }
    const model = provider.models.find((m) => m.modelId === sel.b);
    if (!model) {
      return { valid: false, reason: `Model "${sel.b}" 不存在` };
    }
    if (!model.enabled) {
      return { valid: false, reason: `Model "${sel.b}" 已禁用` };
    }
    return { valid: true };
  },

  /**
   * 验证并返回清理后的模型选择
   * 如果无效返回 null
   */
  sanitize(
    sel: NullableModelRef,
    providers: readonly ProviderConfig[],
  ): NullableModelRef {
    const result = ModelValidator.validate(sel, providers);
    return result.valid ? sel : null;
  },
};
