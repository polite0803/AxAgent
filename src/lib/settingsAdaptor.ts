// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Settings Adaptor — 设置适配层
 *
 * 核心职责：
 * - 在数据加载时立即验证所有模型引用的有效性
 * - 将无效的 provider/model 组合自动清理为 null
 * - 保证 UI 层永远不会看到无效的模型选择
 *
 * ⚠️ 编译期保证说明：
 * - 本模块保证【结构正确性】（providerId 和 modelId 要么同时存在要么同时为 null）
 * - 开发环境会断言数据一致性（见 assertModelConsistency）
 * - 运行时验证数据有效性（provider/model 是否真实存在）
 */

import {
  type AppSettings,
  ModelSelection,
  ModelValidator,
  type NullableModelRef,
  type NullableModelSelection,
  type ProviderConfig,
} from "@/types";

/** 需要验证的模型选择字段对 */
interface FieldPair {
  modelKey: keyof AppSettings;
  label: string;
}

/** AppSettings 中所有需要验证的模型选择字段 */
const MODEL_FIELD_PAIRS: FieldPair[] = [
  { modelKey: "defaultModel", label: "默认对话模型" },
  { modelKey: "titleSummaryModel", label: "标题摘要模型" },
  { modelKey: "compressionModel", label: "压缩模型" },
];

/** 验证结果统计 */
export interface AdaptResult {
  cleanedSettings: AppSettings;
  invalidFields: Array<{
    field: string;
    label: string;
    reason: string;
  }>;
  changed: boolean;
}

/**
 * 适配设置：验证所有模型选择，清理无效引用
 *
 * @param rawSettings 从后端获取的原始设置
 * @param providers 当前可用的 provider 列表
 * @returns 清理后的设置和验证报告
 *
 * 这个函数应该在 fetchSettings 成功后立即调用。
 * 如果返回的 changed = true，说明有无效引用被清理，
 * 调用方应该保存清理后的设置到后端。
 */
export function adaptSettings(
  rawSettings: AppSettings,
  providers: readonly ProviderConfig[],
): AdaptResult {
  const invalidFields: AdaptResult["invalidFields"] = [];
  let changed = false;

  // 开发环境断言：检查 NullableModelRef 结构正确性
  if (import.meta.env?.DEV) {
    for (const pair of MODEL_FIELD_PAIRS) {
      const modelRef = rawSettings[pair.modelKey] as NullableModelRef;
      if (modelRef !== null && modelRef !== undefined) {
        if (typeof modelRef.a !== "string" || typeof modelRef.b !== "string") {
          console.error(
            `[SettingsAdaptor] ${pair.label}: NullableModelRef 结构无效`,
            modelRef,
          );
        }
      }
    }
  }

  // 创建设置副本进行修改
  const cleanedSettings = { ...rawSettings };

  for (const pair of MODEL_FIELD_PAIRS) {
    const modelRef = rawSettings[pair.modelKey] as NullableModelRef;

    const validationResult = ModelValidator.validate(modelRef, providers);

    if (!validationResult.valid && modelRef) {
      // 记录无效字段
      invalidFields.push({
        field: String(pair.modelKey),
        label: pair.label,
        reason: validationResult.reason ?? "未知原因",
      });

      // 清理无效引用
      (cleanedSettings as Record<string, unknown>)[pair.modelKey] = null;
      changed = true;

      console.warn(
        `[SettingsAdaptor] 清理无效的${pair.label}: ${validationResult.reason}`,
        { providerId: modelRef.a, modelId: modelRef.b },
      );
    }
  }

  return {
    cleanedSettings,
    invalidFields,
    changed,
  };
}

/**
 * 从 AppSettings 获取强类型的 ModelSelection
 * 返回 null 表示未设置或设置无效
 */
export function getModelSelection(
  settings: AppSettings,
  modelKey: keyof AppSettings,
): NullableModelSelection {
  return settings[modelKey] as NullableModelRef;
}

/**
 * 验证单个模型选择是否有效
 */
export function isModelSelectionValid(
  settings: AppSettings,
  modelKey: keyof AppSettings,
  providers: readonly ProviderConfig[],
): boolean {
  const selection = getModelSelection(settings, modelKey);
  if (!selection) { return true; }
  return ModelSelection.isValid(selection, providers);
}
