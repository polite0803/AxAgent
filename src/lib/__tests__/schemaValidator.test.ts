// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { UISchema } from "@/types";
import { validateSchema } from "../dynamicUI/SchemaValidator";

function makeValidNode(overrides: Partial<UISchema> = {}): UISchema {
  return {
    version: "1.0",
    id: "test-node",
    type: "Container",
    props: {},
    ...overrides,
  } as UISchema;
}

describe("SchemaValidator", () => {
  describe("validateSchema", () => {
    describe("基本校验", () => {
      it("合法 schema 应通过校验", () => {
        const result = validateSchema(makeValidNode());
        expect(result.valid).toBe(true);
        expect(result.errors).toHaveLength(0);
      });

      it("null 值应返回错误", () => {
        const result = validateSchema(null);
        expect(result.valid).toBe(false);
        expect(result.errors[0].message).toContain("对象类型");
      });

      it("非对象值应返回错误", () => {
        const result = validateSchema("string");
        expect(result.valid).toBe(false);
        expect(result.errors[0].message).toContain("对象类型");
      });
    });

    describe("必填字段", () => {
      it("缺少 id 应返回错误", () => {
        const result = validateSchema({ version: "1.0", type: "Container", props: {} });
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("id"))).toBe(true);
      });

      it("缺少 version 应返回错误", () => {
        const result = validateSchema({ id: "node", type: "Container", props: {} });
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("version"))).toBe(true);
      });

      it("缺少 type 应返回错误", () => {
        const result = validateSchema({ id: "node", version: "1.0", props: {} });
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("type"))).toBe(true);
      });

      it("空字符串 id 应返回错误", () => {
        const result = validateSchema({ id: "", version: "1.0", type: "Container", props: {} });
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("id"))).toBe(true);
      });
    });

    describe("组件类型校验", () => {
      it("未知组件类型应返回错误", () => {
        const result = validateSchema(makeValidNode({ type: "UnknownComponent" as "Container" }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("未知组件类型"))).toBe(true);
      });

      it("已知组件类型应通过校验", () => {
        const validTypes = ["Container", "Row", "Column", "Card", "Form", "Input", "Button", "Text"];
        for (const type of validTypes) {
          const result = validateSchema(makeValidNode({ type: type as "Container" }));
          expect(result.valid).toBe(true);
        }
      });
    });

    describe("props 校验", () => {
      it("props 非对象时应返回错误", () => {
        const result = validateSchema(makeValidNode({ props: "invalid" as unknown as Record<string, unknown> }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("props"))).toBe(true);
      });

      it("Grid 缺少 columns 应返回错误", () => {
        const result = validateSchema(makeValidNode({ type: "Grid" as "Container", props: {} }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("columns"))).toBe(true);
      });

      it("Table 缺少 columns 应返回错误", () => {
        const result = validateSchema(makeValidNode({ type: "Table" as "Container", props: {} }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("columns"))).toBe(true);
      });
    });

    describe("props 形状校验", () => {
      it("Table columns 非数组应返回错误", () => {
        const result = validateSchema(makeValidNode({
          type: "Table" as "Container",
          props: { columns: "not-array" },
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("columns 必须为数组"))).toBe(true);
      });

      it("Chart chartType 非法值应返回错误", () => {
        const result = validateSchema(makeValidNode({
          type: "Chart" as "Container",
          props: { chartType: "invalid" },
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("chartType"))).toBe(true);
      });

      it("Chart chartType 合法值应通过", () => {
        const validTypes = ["line", "bar", "pie", "scatter", "area"];
        for (const chartType of validTypes) {
          const result = validateSchema(makeValidNode({
            type: "Chart" as "Container",
            props: { chartType },
          }));
          expect(result.valid).toBe(true);
        }
      });

      it("Grid columns 非数字应返回错误", () => {
        const result = validateSchema(makeValidNode({
          type: "Grid" as "Container",
          props: { columns: "three" },
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("columns 必须为数字"))).toBe(true);
      });
    });

    describe("children 递归校验", () => {
      it("应递归校验子节点", () => {
        const result = validateSchema(makeValidNode({
          children: [makeValidNode({ id: "child", type: "UnknownType" as "Container" })],
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.path.includes("child"))).toBe(true);
      });

      it("合法子节点应通过校验", () => {
        const result = validateSchema(makeValidNode({
          children: [makeValidNode({ id: "child" })],
        }));
        expect(result.valid).toBe(true);
      });
    });

    describe("深度限制", () => {
      it("超过最大深度时应返回错误", () => {
        // 构造深度 52 的嵌套树（MAX_NESTING_DEPTH = 50，所以 depth=51 > 50 应触发）
        let deep: Record<string, unknown> = {
          id: "deep-51",
          version: "1.0",
          type: "Container",
          props: {},
        };
        for (let i = 50; i >= 0; i--) {
          deep = {
            id: `deep-${i}`,
            version: "1.0",
            type: "Container",
            props: {},
            children: [deep],
          };
        }
        const result = validateSchema(deep);
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("嵌套深度超过上限"))).toBe(true);
      });
    });

    describe("dataSource 校验", () => {
      it("dataSource 非对象时应返回错误", () => {
        const result = validateSchema(makeValidNode({ dataSource: "invalid" as unknown as undefined }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("dataSource"))).toBe(true);
      });

      it("无效的 dataSource type 应返回错误", () => {
        const result = validateSchema(makeValidNode({
          dataSource: { type: "invalid" as "store", config: {} },
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("数据源类型"))).toBe(true);
      });

      it("缺少 config 应返回错误", () => {
        const result = validateSchema(makeValidNode({
          dataSource: { type: "static" } as unknown as UISchema["dataSource"],
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("config"))).toBe(true);
      });

      it("合法 dataSource 应通过校验", () => {
        const result = validateSchema(makeValidNode({
          dataSource: { type: "store", config: { storeName: "preference" } },
        }));
        expect(result.valid).toBe(true);
      });
    });

    describe("events 校验", () => {
      it("events 非数组时应返回错误", () => {
        const result = validateSchema(makeValidNode({ events: "invalid" as unknown as UISchema["events"] }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("events"))).toBe(true);
      });

      it("无效的 trigger 应返回错误", () => {
        const result = validateSchema(makeValidNode({
          events: [{ trigger: "onInvalid" as "onClick", actions: [] }],
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("触发器"))).toBe(true);
      });

      it("actions 非数组时应返回错误", () => {
        const result = validateSchema(makeValidNode({
          events: [{ trigger: "onClick" as const, actions: "invalid" as unknown as [] }],
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("actions"))).toBe(true);
      });

      it("合法 events 应通过校验", () => {
        const result = validateSchema(makeValidNode({
          events: [{ trigger: "onClick" as const, actions: [] }],
        }));
        expect(result.valid).toBe(true);
      });
    });

    describe("conditionalDisplay 校验", () => {
      it("conditionalDisplay 非数组/对象应返回错误", () => {
        const result = validateSchema(makeValidNode({
          conditionalDisplay: "invalid" as unknown as undefined,
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("conditionalDisplay"))).toBe(true);
      });

      it("空数组应返回错误", () => {
        const result = validateSchema(makeValidNode({
          conditionalDisplay: [] as unknown as undefined,
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("不能为空"))).toBe(true);
      });

      it("无效的 logic 应返回错误", () => {
        const result = validateSchema(makeValidNode({
          conditionalDisplay: {
            logic: "xor" as "and",
            rules: [[{ field: "x", operator: "eq", value: 1 }]],
          },
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("logic"))).toBe(true);
      });

      it("无效的 operator 应返回错误", () => {
        const result = validateSchema(makeValidNode({
          conditionalDisplay: [
            { field: "x", operator: "invalid" as "eq", value: 1 },
          ],
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("操作符"))).toBe(true);
      });

      it("缺少 field 应返回错误", () => {
        const result = validateSchema(makeValidNode({
          conditionalDisplay: [
            { field: "", operator: "eq", value: 1 },
          ],
        }));
        expect(result.valid).toBe(false);
        expect(result.errors.some((e) => e.message.includes("field"))).toBe(true);
      });
    });
  });
});
