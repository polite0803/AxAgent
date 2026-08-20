// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Paired — 全局"必须同步的多字段对"类型工具
 *
 * 设计原则：
 * 1. 任何业务上"必须同时存在或同时为 null"的多个字段对，都应使用此类型
 * 2. 通过类型系统保证结构一致性，消除运行时检查的需要
 * 3. 开发期断言确保数据边界的正确性
 *
 * 使用场景：
 * - providerId + modelId 必须同步
 * - userId + userRole 必须同步
 * - configKey + configValue 必须同步
 *
 * ❌ 反模式（天然会产生不一致）：
 * interface Bad { a: string | null; b: string | null; }
 *
 * ✅ 正模式（类型系统保证一致）：
 * type Good = Paired<string, string> | null;
 */

// ==================== 品牌类型 ====================

/**
 * 品牌类型工具：防止不同语义的 string 被混用
 *
 * 使用示例：
 * type ProviderId = Brand<string, "ProviderId">;
 * type UserId = Brand<string, "UserId">;
 *
 * 这样 ProviderId 和 UserId 虽然底层都是 string，但类型系统会拒绝混用。
 */
export type Brand<T, B> = T & { readonly __brand: B };

// ==================== Paired ====================

/**
 * Paired<A, B> — 两个必须同步的值
 *
 * 语义：A 和 B 要么同时存在，要么通过 NullablePaired 变为 null
 * 不可能出现"A 有值但 B 为 null"的状态
 */
export interface Paired<A, B> {
  readonly a: A;
  readonly b: B;
}

/**
 * 可为空的 Paired
 * null = 两个值都未设置
 */
export type NullablePaired<A, B> = Paired<A, B> | null;

// ==================== ModelRef ====================

/**
 * ProviderId 品牌类型
 */
export type ProviderId = Brand<string, "ProviderId">;

/**
 * ModelId 品牌类型
 */
export type ModelId = Brand<string, "ModelId">;

/**
 * ModelRef — 绑定的 provider + model 引用
 *
 * 这是 Paired 的具体实现，用于所有需要 provider+model 绑定的场景。
 * 类型系统保证：不可能出现"providerId 存在但 modelId 缺失"的状态。
 */
export type ModelRef = Paired<ProviderId, ModelId>;

/**
 * 可为空的 ModelRef
 */
export type NullableModelRef = ModelRef | null;

// ==================== 构造函数 ====================

/**
 * Paired 构造工具
 */
export const Paired = {
  /**
   * 创建 Paired
   * @throws 任一参数为 null/undefined/空字符串时
   */
  create<A, B>(a: A, b: B): Paired<A, B> {
    if (a === null || a === undefined) {
      throw new Error("[Paired] a 不能为空");
    }
    if (b === null || b === undefined) {
      throw new Error("[Paired] b 不能为空");
    }
    return { a, b };
  },

  /**
   * 从两个独立值创建 NullablePaired
   * 如果两个值不一致（一个为 null 另一个有值），在开发环境下抛出错误
   *
   * 这是核心的编译期保证点
   */
  fromNullable<A, B>(
    a: A | null | undefined,
    b: B | null | undefined,
  ): NullablePaired<A, B> {
    const hasA = a !== null && a !== undefined;
    const hasB = b !== null && b !== undefined;

    // 开发期断言：两个值必须同时存在或同时为 null
    if (hasA !== hasB) {
      if (import.meta.env?.DEV) {
        throw new Error(
          `[Paired] a 和 b 必须同时设置或同时为 null，`
            + `当前 a=${String(a)}, b=${String(b)}`,
        );
      }
      return null;
    }

    if (!hasA || !hasB) { return null; }
    return Paired.create(a as A, b as B);
  },

  /**
   * 验证 Paired 结构正确性
   */
  isValid<A, B>(value: unknown): value is Paired<A, B> {
    return (
      typeof value === "object"
      && value !== null
      && "a" in value
      && "b" in value
      && (value as Paired<A, B>).a !== null
      && (value as Paired<A, B>).a !== undefined
      && (value as Paired<A, B>).b !== null
      && (value as Paired<A, B>).b !== undefined
    );
  },
};

// ==================== ModelRef 工具 ====================

/**
 * ModelRef 构造工具
 */
export const ModelRef = {
  /**
   * 从 providerId + modelId 创建
   */
  from(providerId: string, modelId: string): ModelRef {
    if (!providerId) {
      throw new Error("[ModelRef] providerId 不能为空");
    }
    if (!modelId) {
      throw new Error("[ModelRef] modelId 不能为空");
    }
    return {
      a: providerId as ProviderId,
      b: modelId as ModelId,
    };
  },

  /**
   * 从两个独立值创建 NullableModelRef
   * 如果两个值不一致（一个为 null 另一个有值），在开发环境下抛出错误
   *
   * 这是核心的编译期保证点
   */
  fromNullable(
    providerId: string | null | undefined,
    modelId: string | null | undefined,
  ): NullableModelRef {
    const hasProvider = providerId !== null && providerId !== undefined && providerId !== "";
    const hasModel = modelId !== null && modelId !== undefined && modelId !== "";

    // 开发期断言：两个值必须同时存在或同时为 null
    if (hasProvider !== hasModel) {
      if (import.meta.env?.DEV) {
        throw new Error(
          `[ModelRef] providerId 和 modelId 必须同时设置或同时为 null，`
            + `当前 providerId=${String(providerId)}, modelId=${String(modelId)}`,
        );
      }
      return null;
    }

    if (!hasProvider || !hasModel) { return null; }

    try {
      return ModelRef.from(providerId, modelId);
    } catch {
      return null;
    }
  },

  /**
   * 从 "providerId::modelId" 格式字符串解析
   */
  parse(value: string | null | undefined): NullableModelRef {
    if (!value) { return null; }
    const idx = value.indexOf("::");
    if (idx < 0) { return null; }
    const providerId = value.slice(0, idx);
    const modelId = value.slice(idx + 2);
    if (!providerId || !modelId) { return null; }
    try {
      return ModelRef.from(providerId, modelId);
    } catch {
      return null;
    }
  },

  /**
   * 序列化为 "providerId::modelId" 字符串
   */
  toValue(ref: ModelRef): string {
    return `${ref.a}::${ref.b}`;
  },

  /**
   * 转换为存储结构
   */
  toStorage(ref: ModelRef): { providerId: string; modelId: string } {
    return { providerId: ref.a, modelId: ref.b };
  },

  /**
   * 从存储结构创建
   */
  fromStorage(storage: { providerId: string | null; modelId: string | null }): NullableModelRef {
    return ModelRef.fromNullable(storage.providerId, storage.modelId);
  },

  /**
   * 转换为字符串对（用于需要分离字段的场景）
   */
  toPair(ref: ModelRef): { providerId: string; modelId: string } {
    return { providerId: ref.a, modelId: ref.b };
  },

  /**
   * 获取 providerId
   */
  providerId(ref: ModelRef): ProviderId {
    return ref.a;
  },

  /**
   * 获取 modelId
   */
  modelId(ref: ModelRef): ModelId {
    return ref.b;
  },

  /**
   * 验证 ModelRef 在 provider 列表中是否存在
   */
  isValid(
    ref: ModelRef,
    providers: ReadonlyArray<
      { id: string; enabled: boolean; models: ReadonlyArray<{ modelId: string; enabled: boolean }> }
    >,
  ): boolean {
    const provider = providers.find((p) => p.id === ref.a);
    if (!provider || !provider.enabled) { return false; }
    const model = provider.models.find((m) => m.modelId === ref.b);
    if (!model || !model.enabled) { return false; }
    return true;
  },
};

// ==================== 开发期断言 ====================

/**
 * 全局开发期断言：检查数据结构的一致性
 *
 * 在数据加载后立即调用，确保进入应用的数据结构正确。
 * 开发环境会抛出错误，生产环境会静默记录警告。
 *
 * 使用方式：
 * ```typescript
 * assertConsistency(settings, [
 *   { fields: ["defaultProviderId", "defaultModelId"], label: "默认模型" },
 *   { fields: ["titleSummaryProviderId", "titleSummaryModelId"], label: "标题摘要模型" },
 * ]);
 * ```
 */
export function assertConsistency(
  data: Record<string, unknown>,
  checks: Array<{ fields: [string, string]; label: string }>,
): void {
  if (!import.meta.env?.DEV) { return; }

  for (const { fields, label } of checks) {
    const [key1, key2] = fields;
    const val1 = data[key1];
    const val2 = data[key2];

    const hasVal1 = val1 !== null && val1 !== undefined && val1 !== "";
    const hasVal2 = val2 !== null && val2 !== undefined && val2 !== "";

    if (hasVal1 !== hasVal2) {
      throw new Error(
        `[Consistency] ${label}: 字段 ${key1} 和 ${key2} 必须同时设置或同时为 null，`
          + `当前 ${key1}=${String(val1)}, ${key2}=${String(val2)}`,
      );
    }
  }
}

// ==================== 类型守卫 ====================

/**
 * 检查值是否为非空的 Paired
 * 用于 TypeScript 类型收窄
 */
export function isPaired<A, B>(value: NullablePaired<A, B>): value is Paired<A, B> {
  return value !== null && value !== undefined;
}

/**
 * 检查值是否为非空的 ModelRef
 */
export function isModelRef(value: NullableModelRef): value is ModelRef {
  return value !== null && value !== undefined;
}
