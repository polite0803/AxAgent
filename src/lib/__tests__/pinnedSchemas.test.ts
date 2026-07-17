// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import { getPinnedSchemasByGroup, PIN_GROUPS, type PinnedSchemaConfig, type PinnedSchemaMap } from "../pinned-schemas";

function makePin(schemaId: string, group: string, position: number): PinnedSchemaConfig {
  return { schemaId, title: schemaId, group, position };
}

describe("pinned-schemas", () => {
  describe("PIN_GROUPS", () => {
    it("应包含预置分组", () => {
      expect(PIN_GROUPS.length).toBe(4);
      expect(PIN_GROUPS.map((g) => g.key)).toEqual(["dashboard", "report", "monitor", "other"]);
    });
  });

  describe("getPinnedSchemasByGroup", () => {
    it("应返回空数组", () => {
      expect(getPinnedSchemasByGroup({})).toEqual([]);
    });

    it("应按 PIN_GROUPS 顺序分组", () => {
      const schemas: PinnedSchemaMap = {
        a: makePin("a", "dashboard", 0),
        b: makePin("b", "report", 0),
        c: makePin("c", "monitor", 0),
      };
      const result = getPinnedSchemasByGroup(schemas);
      expect(result.map((g) => g.group)).toEqual(["dashboard", "report", "monitor"]);
    });

    it("组内应按 position 排序", () => {
      const schemas: PinnedSchemaMap = {
        a: makePin("a", "dashboard", 2),
        b: makePin("b", "dashboard", 0),
        c: makePin("c", "dashboard", 1),
      };
      const result = getPinnedSchemasByGroup(schemas);
      expect(result[0].items.map((i) => i.schemaId)).toEqual(["b", "c", "a"]);
    });

    it("未匹配的组应排在最后", () => {
      const schemas: PinnedSchemaMap = {
        a: makePin("a", "custom-app", 0),
        b: makePin("b", "dashboard", 0),
      };
      const result = getPinnedSchemasByGroup(schemas);
      expect(result.map((g) => g.group)).toEqual(["dashboard", "custom-app"]);
    });

    it("多个未匹配组应按字母排序", () => {
      const schemas: PinnedSchemaMap = {
        a: makePin("a", "z-group", 0),
        b: makePin("b", "a-group", 0),
      };
      const result = getPinnedSchemasByGroup(schemas);
      const unmatched = result.filter((g) => !PIN_GROUPS.some((pg) => pg.key === g.group));
      expect(unmatched.map((g) => g.group)).toEqual(["a-group", "z-group"]);
    });
  });
});
