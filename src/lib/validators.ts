/**
 * src/lib/validators.ts
 *
 * 项目级数据验证工具库
 *
 * 目的：在数据边界（特别是前后端 IPC、数据库读写、序列化/反序列化）
 * 建立统一的验证防线，防止 "undefined"/"null" 字面量字符串等脏数据
 * 污染数据流。
 *
 * 使用原则：
 * - 所有涉及 ID（providerId、modelId、conversationId 等）的代码
 *   必须使用 `validateId()` 或 `sanitizeId()` 进行验证
 * - 模板字符串拼接 ID 前必须先通过验证
 * - 从后端/数据库读取的 ID 值必须先通过验证才能使用
 */

/**
 * 判断值是否为有效的 ID 字符串
 *
 * 有效 ID 定义：非空、非 "undefined"、非 "null"、非纯空白的字符串
 *
 * @param value - 待检查的值
 * @returns 如果是有效 ID 返回 true
 */
export function isValidId(value: unknown): value is string {
  if (typeof value !== "string") { return false; }
  const trimmed = value.trim();
  if (trimmed.length === 0) { return false; }
  if (trimmed === "undefined" || trimmed === "null") { return false; }
  return true;
}

/**
 * 清洗 ID 值，将无效值统一转为 null
 *
 * 使用场景：
 * - 从后端 DTO 读取 ID 字段时
 * - 从 localStorage/sessionStorage 读取 ID 时
 * - 从 URL 参数读取 ID 时
 *
 * @param value - 可能为脏数据的 ID 值
 * @returns 有效的 ID 字符串，或 null
 */
export function sanitizeId(value: unknown): string | null {
  return isValidId(value) ? value : null;
}

/**
 * 验证模型引用（providerId + modelId）的完整性
 *
 * 要求：providerId 和 modelId 必须同时有效，或同时为 null
 * 不允许只有一个有效另一个为 null/undefined 的情况
 *
 * @param providerId - 提供商 ID
 * @param modelId - 模型 ID
 * @returns 如果两个都有效返回 { providerId, modelId }，否则返回 null
 */
export function validateModelRef(
  providerId: unknown,
  modelId: unknown,
): { providerId: string; modelId: string } | null {
  const validProvider = sanitizeId(providerId);
  const validModel = sanitizeId(modelId);

  // 两者必须同时有效或同时为 null
  if (validProvider && validModel) {
    return { providerId: validProvider, modelId: validModel };
  }

  if (validProvider || validModel) {
    // 只有一个有效，数据不一致
    console.warn("[validateModelRef] 数据不一致: providerId 和 modelId 必须同时有效或同时为 null", {
      providerId,
      modelId,
    });
    return null;
  }

  return null;
}

/**
 * 安全拼接 ID 字符串，防止 undefined/null 污染
 *
 * 替代 `${a}::${b}` 模板字符串的安全版本。
 * 如果任一参数无效，返回空字符串而非 "undefined::xxx"。
 *
 * @param parts - 需要拼接的 ID 部分
 * @param separator - 分隔符，默认 "::"
 * @returns 安全拼接的字符串，无效部分会被跳过
 */
export function safeJoinIds(parts: unknown[], separator = "::"): string {
  return parts
    .filter((p): p is string => isValidId(p))
    .join(separator);
}

/**
 * 从拼接字符串中安全解析 ID 对
 *
 * 替代 `parseModelValue` 中直接 split 的做法，增加验证层。
 *
 * @param joined - 拼接的 ID 字符串（如 "providerId::modelId"）
 * @param separator - 分隔符，默认 "::"
 * @returns 解析后的对象，或 null
 */
export function safeParseIdPair(
  joined: string | undefined | null,
  separator = "::",
): Record<string, string> | null {
  if (!joined || typeof joined !== "string") { return null; }

  const parts = joined.split(separator);
  if (parts.length !== 2) { return null; }

  const [first, second] = parts;
  if (!isValidId(first) || !isValidId(second)) {
    return null;
  }

  return { first, second };
}
