// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { UISchema } from "@/types";
import { generateUIFromNaturalLanguage } from "./nl2ui";

/**
 * AI 驱动的 schema 编辑：基于自然语言指令修改已有 schema。
 *
 * 优先调用后端 AI 进行精准编辑；后端不可用时降级为本地重新生成。
 *
 * @param existingSchema - 当前待编辑的 UISchema
 * @param prompt - 自然语言编辑指令
 * @returns 修改后的 schema 与操作描述
 */
export async function editUIFromNL(
  existingSchema: UISchema,
  prompt: string,
): Promise<{ schema: UISchema; description: string }> {
  try {
    const result = await invoke<{ schema: string; description: string }>(
      "edit_dynamic_ui_schema_nl",
      {
        existingSchema: JSON.stringify(existingSchema),
        prompt,
      },
    );

    let parsed: UISchema;
    if (typeof result.schema === "string") {
      parsed = JSON.parse(result.schema) as UISchema;
    } else {
      parsed = result.schema as unknown as UISchema;
    }

    return {
      schema: parsed,
      description: result.description ?? `根据指令"${prompt.slice(0, 50)}"编辑完成`,
    };
  } catch {
    // 降级：将 prompt 作为全新生成请求
    const fallback = generateUIFromNaturalLanguage(prompt);
    return {
      schema: fallback.schema,
      description: `（AI 后端不可用，由本地引擎根据指令重新生成）：${fallback.description}`,
    };
  }
}

/**
 * AI 驱动的自然语言创建：基于自然语言描述生成完整 UI Schema。
 *
 * 优先调用后端 AI 进行生成；后端不可用时降级为本地规则生成。
 *
 * @param prompt - 自然语言描述
 * @returns 生成的 schema、推断标题与描述
 */
export async function generateUIFromNLBackend(
  prompt: string,
): Promise<{ schema: UISchema; title: string; description: string }> {
  try {
    const result = await invoke<{ schema: string; title: string; description: string }>(
      "generate_dynamic_ui_schema_nl",
      { prompt },
    );

    const schema = JSON.parse(result.schema) as UISchema;
    return {
      schema,
      title: result.title || "动态UI",
      description: result.description
        || `由自然语言生成：${prompt.slice(0, 50)}${prompt.length > 50 ? "..." : ""}`,
    };
  } catch {
    // 降级：本地规则生成
    const fallback = generateUIFromNaturalLanguage(prompt);
    return {
      ...fallback,
      description: `（AI 后端不可用，由本地引擎生成）：${fallback.description}`,
    };
  }
}
