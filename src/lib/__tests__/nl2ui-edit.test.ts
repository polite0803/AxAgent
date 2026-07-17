// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

import type { UISchema } from "@/types";

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn(),
}));

vi.mock("../dynamicUI/nl2ui", () => ({
  generateUIFromNaturalLanguage: vi.fn(() => ({
    schema: { version: "1.0", id: "fallback", type: "Column", props: {} },
    title: "回退",
    description: "本地生成",
  })),
}));

describe("nl2ui-edit", () => {
  describe("editUIFromNL", () => {
    it("后端不可用时应降级到本地生成", async () => {
      const { invoke } = await import("@/lib/invoke");
      vi.mocked(invoke).mockRejectedValue(new Error("Backend unavailable"));

      const { editUIFromNL } = await import("../dynamicUI/nl2ui-edit");
      const existing: UISchema = {
        version: "1.0",
        id: "existing",
        type: "Container",
        props: {},
      };

      const result = await editUIFromNL(existing, "添加一个按钮");
      expect(result.schema).toBeDefined();
      expect(result.schema.id).toBe("fallback");
      expect(result.description).toContain("AI 后端不可用");
    });

    it("后端返回合法 schema 时应直接使用", async () => {
      const { invoke } = await import("@/lib/invoke");
      const backendSchema: UISchema = {
        version: "1.0",
        id: "backend-result",
        type: "Container",
        props: {},
      };
      vi.mocked(invoke).mockResolvedValue({
        schema: JSON.stringify(backendSchema),
        description: "后端生成",
      });

      const { editUIFromNL } = await import("../dynamicUI/nl2ui-edit");
      const existing: UISchema = {
        version: "1.0",
        id: "existing",
        type: "Container",
        props: {},
      };

      const result = await editUIFromNL(existing, "添加一个按钮");
      expect(result.schema.id).toBe("backend-result");
      expect(result.description).toBe("后端生成");
    });
  });

  describe("generateUIFromNLBackend", () => {
    it("后端不可用时应降级到本地生成", async () => {
      const { invoke } = await import("@/lib/invoke");
      vi.mocked(invoke).mockRejectedValue(new Error("Backend unavailable"));

      const { generateUIFromNLBackend } = await import("../dynamicUI/nl2ui-edit");

      const result = await generateUIFromNLBackend("创建一个表单");
      expect(result.schema).toBeDefined();
      expect(result.schema.id).toBe("fallback");
      expect(result.title).toBe("回退");
      expect(result.description).toContain("AI 后端不可用");
    });

    it("后端返回合法结果时应使用", async () => {
      const { invoke } = await import("@/lib/invoke");
      const backendSchema: UISchema = {
        version: "1.0",
        id: "backend-gen",
        type: "Column",
        props: {},
      };
      vi.mocked(invoke).mockResolvedValue({
        schema: JSON.stringify(backendSchema),
        title: "后端标题",
        description: "后端生成描述",
      });

      const { generateUIFromNLBackend } = await import("../dynamicUI/nl2ui-edit");

      const result = await generateUIFromNLBackend("创建一个表单");
      expect(result.schema.id).toBe("backend-gen");
      expect(result.title).toBe("后端标题");
      expect(result.description).toBe("后端生成描述");
    });
  });
});
