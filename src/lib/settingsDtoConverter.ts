// SPDX-License-Identifier: AGPL-3.0-only

/**
 * SettingsDtoConverter — IPC 边界转换层
 *
 * 职责：
 * 1. 在前端强类型（NullableModelRef）和后端 DTO（分离字段）之间转换
 * 2. 保证数据边界的类型安全
 * 3. 消除业务代码中的手动转换逻辑
 *
 * 设计原则：
 * - 前端内部只使用 NullableModelRef（类型安全）
 * - 后端 DTO 保持分离字段（兼容现有 Rust 代码）
 * - 转换层是唯一允许做这种转换的地方
 */

import type { AppSettings } from "@/types";
import { ModelRef, type NullableModelRef } from "@/types/paired";

/**
 * 后端 DTO 格式的设置（分离字段）
 * 这是与 Rust 后端交互的格式
 */
export interface SettingsDto {
  // 模型选择分离字段（后端格式）
  defaultProviderId: string | null;
  defaultModelId: string | null;
  titleSummaryProviderId: string | null;
  titleSummaryModelId: string | null;
  compressionProviderId: string | null;
  compressionModelId: string | null;
  // 其他所有字段保持原样
  [key: string]: unknown;
}

/**
 * 前端模型选择字段映射
 * 定义哪些字段对需要转换
 */
const MODEL_FIELD_MAP = {
  defaultModel: { providerKey: "defaultProviderId", modelKey: "defaultModelId" },
  titleSummaryModel: { providerKey: "titleSummaryProviderId", modelKey: "titleSummaryModelId" },
  compressionModel: { providerKey: "compressionProviderId", modelKey: "compressionModelId" },
} as const;

/**
 * 后端 DTO → 前端 AppSettings
 *
 * 在数据加载时调用，将分离的 providerId+modelId 字段合并为 NullableModelRef。
 * 这是类型安全的第一道防线。
 */
export function fromDto(dto: Partial<SettingsDto>): AppSettings {
  const settings = { ...dto } as Record<string, unknown>;

  // 验证 ID 是否为有效值的辅助函数
  const isValidId = (id: unknown): id is string => {
    if (typeof id !== "string") { return false; }
    if (id === "undefined" || id === "null" || id.trim() === "") { return false; }
    return true;
  };

  // 将分离字段合并为 NullableModelRef
  for (const [targetKey, { providerKey, modelKey }] of Object.entries(MODEL_FIELD_MAP)) {
    let providerId = dto[providerKey] as string | null | undefined;
    let modelId = dto[modelKey] as string | null | undefined;

    // 防御：清理无效的 ID 值
    // 如果数据库中存的是 "undefined" 或 "null" 字符串，将其视为 null
    if (!isValidId(providerId)) {
      if (providerId !== null && providerId !== undefined && providerId !== "") {
        console.warn(`[fromDto] 清理无效 providerId: ${providerKey}=${String(providerId)}`);
      }
      providerId = null;
    }
    if (!isValidId(modelId)) {
      if (modelId !== null && modelId !== undefined && modelId !== "") {
        console.warn(`[fromDto] 清理无效 modelId: ${modelKey}=${String(modelId)}`);
      }
      modelId = null;
    }

    // 使用 ModelRef.fromNullable 进行结构一致性检查
    const modelRef = ModelRef.fromNullable(providerId, modelId);

    // 设置合并后的字段
    settings[targetKey] = modelRef;

    // 删除旧的分离字段
    delete settings[providerKey];
    delete settings[modelKey];
  }

  return settings as unknown as AppSettings;
}

/**
 * 前端 AppSettings → 后端 DTO
 *
 * 在数据保存时调用，将 NullableModelRef 拆分为分离的 providerId+modelId 字段。
 * 保证后端收到的是合法的分离字段格式。
 */
export function toDto(settings: AppSettings): SettingsDto {
  const dto = { ...settings } as Record<string, unknown>;

  // 将 NullableModelRef 拆分为分离字段
  for (const [sourceKey, { providerKey, modelKey }] of Object.entries(MODEL_FIELD_MAP)) {
    const modelRef = settings[sourceKey as keyof AppSettings] as NullableModelRef;

    if (modelRef) {
      const providerId = modelRef.a;
      const modelId = modelRef.b;

      // 防御：检查 providerId 和 modelId 是否为有效值
      // 排除 "undefined", "null", 空字符串等脏数据
      const isValid = (id: unknown): id is string => {
        if (typeof id !== "string") { return false; }
        if (id === "undefined" || id === "null" || id.trim() === "") { return false; }
        return true;
      };

      if (isValid(providerId) && isValid(modelId)) {
        dto[providerKey] = providerId;
        dto[modelKey] = modelId;
      } else {
        console.warn(
          `[toDto] 检测到无效模型引用 ${sourceKey}`,
          { providerId, modelId },
        );
        dto[providerKey] = null;
        dto[modelKey] = null;
      }
    } else {
      dto[providerKey] = null;
      dto[modelKey] = null;
    }

    // 删除合并后的字段（后端不认识）
    delete dto[sourceKey];
  }

  return dto as SettingsDto;
}

/**
 * 保存单个模型选择字段
 *
 * 用于设置变更时，只更新特定的模型选择字段对。
 * 保证 providerId 和 modelId 同时更新或同时为 null。
 */
export function updateModelField(
  _currentSettings: AppSettings,
  fieldKey: keyof typeof MODEL_FIELD_MAP,
  newValue: NullableModelRef,
): Partial<AppSettings> {
  return {
    [fieldKey]: newValue,
  } as Partial<AppSettings>;
}

/**
 * 获取模型选择的显示标签
 *
 * 从 NullableModelRef 提取 providerId 和 modelId 用于显示。
 * 这是唯一允许"拆开"NullableModelRef 的地方。
 */
export function getModelDisplay(ref: NullableModelRef): {
  providerId: string | null;
  modelId: string | null;
} {
  if (!ref) {
    return { providerId: null, modelId: null };
  }
  return { providerId: ref.a, modelId: ref.b };
}

/**
 * 验证从后端加载的数据结构正确性
 *
 * 开发环境下会断言数据结构一致性。
 * 如果发现不一致（providerId 存在但 modelId 缺失），会抛出错误。
 */
export function validateDtoConsistency(dto: SettingsDto): void {
  if (!import.meta.env?.DEV) { return; }

  for (const { providerKey, modelKey } of Object.values(MODEL_FIELD_MAP)) {
    const providerId = dto[providerKey];
    const modelId = dto[modelKey];

    const hasProvider = providerId !== null && providerId !== undefined && providerId !== "";
    const hasModel = modelId !== null && modelId !== undefined && modelId !== "";

    if (hasProvider !== hasModel) {
      throw new Error(
        `[SettingsDto] ${providerKey} 和 ${modelKey} 必须同时设置或同时为 null，`
          + `当前 ${providerKey}=${String(providerId)}, ${modelKey}=${String(modelId)}`,
      );
    }
  }
}
