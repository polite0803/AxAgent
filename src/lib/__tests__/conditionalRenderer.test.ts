// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { ConditionalDisplay } from "@/types";
import { evaluateConditions } from "../dynamicUI/ConditionalRenderer";

describe("evaluateConditions", () => {
  describe("undefined 条件", () => {
    it("条件为 undefined 时应返回 true", () => {
      expect(evaluateConditions(undefined, {})).toBe(true);
    });
  });

  describe("数组形式 (AND)", () => {
    it("所有规则都满足时应返回 true", () => {
      const condition: ConditionalDisplay = [
        { field: "name", operator: "eq", value: "Alice" },
        { field: "age", operator: "gte", value: 18 },
      ];
      expect(evaluateConditions(condition, { name: "Alice", age: 25 })).toBe(true);
    });

    it("任一规则不满足时应返回 false", () => {
      const condition: ConditionalDisplay = [
        { field: "name", operator: "eq", value: "Alice" },
        { field: "age", operator: "gte", value: 18 },
      ];
      expect(evaluateConditions(condition, { name: "Bob", age: 25 })).toBe(false);
    });
  });

  describe("对象形式", () => {
    describe("logic: and", () => {
      it("所有规则都满足时应返回 true", () => {
        const condition: ConditionalDisplay = {
          logic: "and",
          rules: [
            [
              { field: "name", operator: "eq", value: "Alice" },
              { field: "age", operator: "gte", value: 18 },
            ],
          ],
        };
        expect(evaluateConditions(condition, { name: "Alice", age: 25 })).toBe(true);
      });

      it("任一规则不满足时应返回 false", () => {
        const condition: ConditionalDisplay = {
          logic: "and",
          rules: [
            [
              { field: "name", operator: "eq", value: "Alice" },
              { field: "age", operator: "gte", value: 18 },
            ],
          ],
        };
        expect(evaluateConditions(condition, { name: "Alice", age: 15 })).toBe(false);
      });
    });

    describe("logic: or", () => {
      it("任一规则满足时应返回 true", () => {
        const condition: ConditionalDisplay = {
          logic: "or",
          rules: [
            [{ field: "name", operator: "eq", value: "Alice" }],
            [{ field: "age", operator: "gte", value: 18 }],
          ],
        };
        expect(evaluateConditions(condition, { name: "Alice", age: 15 })).toBe(true);
      });

      it("所有规则都不满足时应返回 false", () => {
        const condition: ConditionalDisplay = {
          logic: "or",
          rules: [
            [{ field: "name", operator: "eq", value: "Alice" }],
            [{ field: "age", operator: "gte", value: 18 }],
          ],
        };
        expect(evaluateConditions(condition, { name: "Bob", age: 15 })).toBe(false);
      });
    });

    describe("not", () => {
      it("not: true 应取反", () => {
        const condition: ConditionalDisplay = {
          logic: "and",
          rules: [[{ field: "name", operator: "eq", value: "Alice" }]],
          not: true,
        };
        expect(evaluateConditions(condition, { name: "Alice" })).toBe(false);
        expect(evaluateConditions(condition, { name: "Bob" })).toBe(true);
      });
    });
  });

  describe("operator: eq", () => {
    it("值相等时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "eq", value: 1 }], { x: 1 })).toBe(true);
    });

    it("值不相等时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "eq", value: 1 }], { x: 2 })).toBe(false);
    });
  });

  describe("operator: neq", () => {
    it("值不相等时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "neq", value: 1 }], { x: 2 })).toBe(true);
    });
  });

  describe("operator: gt", () => {
    it("大于时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "gt", value: 5 }], { x: 10 })).toBe(true);
    });

    it("不大于时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "gt", value: 10 }], { x: 5 })).toBe(false);
    });

    it("非数字时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "gt", value: "a" }], { x: "b" })).toBe(false);
    });
  });

  describe("operator: gte", () => {
    it("大于等于时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "gte", value: 5 }], { x: 5 })).toBe(true);
    });
  });

  describe("operator: lt", () => {
    it("小于时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "lt", value: 10 }], { x: 5 })).toBe(true);
    });
  });

  describe("operator: lte", () => {
    it("小于等于时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "lte", value: 5 }], { x: 5 })).toBe(true);
    });
  });

  describe("operator: in", () => {
    it("值在数组中时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "in", value: [1, 2, 3] }], { x: 2 })).toBe(true);
    });

    it("值不在数组中时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "in", value: [1, 2, 3] }], { x: 4 })).toBe(false);
    });

    it("value 非数组时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "in", value: "not-array" }], { x: 1 })).toBe(false);
    });
  });

  describe("operator: contains", () => {
    it("字符串包含时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "contains", value: "hello" }], { x: "hello world" })).toBe(
        true,
      );
    });

    it("数组包含时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "contains", value: 2 }], { x: [1, 2, 3] })).toBe(true);
    });

    it("字符串不包含时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "contains", value: "xyz" }], { x: "hello" })).toBe(false);
    });
  });

  describe("operator: exists", () => {
    it("字段存在且非 null 时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "exists", value: "" }], { x: "value" })).toBe(true);
    });

    it("字段为 null 时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "exists", value: "" }], { x: null })).toBe(false);
    });

    it("字段不存在时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "exists", value: "" }], {})).toBe(false);
    });
  });

  describe("operator: empty", () => {
    it("字段为 undefined 时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "empty", value: "" }], {})).toBe(true);
    });

    it("字段为 null 时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "empty", value: "" }], { x: null })).toBe(true);
    });

    it("字段为空字符串时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "empty", value: "" }], { x: "" })).toBe(true);
    });

    it("字段为空数组时应返回 true", () => {
      expect(evaluateConditions([{ field: "x", operator: "empty", value: "" }], { x: [] })).toBe(true);
    });

    it("字段有值时应返回 false", () => {
      expect(evaluateConditions([{ field: "x", operator: "empty", value: "" }], { x: "value" })).toBe(false);
    });
  });

  describe("嵌套条件", () => {
    it("应支持嵌套 and/or", () => {
      const condition: ConditionalDisplay = {
        logic: "and",
        rules: [
          [{ field: "name", operator: "eq", value: "Alice" }],
          {
            logic: "or",
            rules: [
              [{ field: "age", operator: "gte", value: 18 }],
              [{ field: "vip", operator: "eq", value: true }],
            ],
          },
        ],
      };
      expect(evaluateConditions(condition, { name: "Alice", age: 15, vip: true })).toBe(true);
      expect(evaluateConditions(condition, { name: "Alice", age: 15, vip: false })).toBe(false);
    });
  });
});
