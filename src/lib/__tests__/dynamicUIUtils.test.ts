// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { UISchema } from "@/types";
import {
  appendChildToNode,
  cloneSchema,
  cloneSchemaWithNewIds,
  detachNodeById,
  findNodeById,
  findParent,
  flattenNodes,
  genId,
  getDefaultProps,
  getNestedValue,
  insertBeforeSibling,
  isDescendant,
  removeNodeById,
  resolveDynamicArray,
  updateNodeById,
} from "../dynamicUI/utils";

function makeNode(overrides: Partial<UISchema> = {}): UISchema {
  return {
    version: "1.0",
    id: "node-1",
    type: "Container",
    props: {},
    ...overrides,
  } as UISchema;
}

describe("getNestedValue", () => {
  const obj = { user: { profile: { name: "Alice", age: 30 } } };

  it("应获取嵌套值", () => {
    expect(getNestedValue(obj, "user.profile.name")).toBe("Alice");
  });

  it("应获取中间对象", () => {
    expect(getNestedValue(obj, "user.profile")).toEqual({ name: "Alice", age: 30 });
  });

  it("中间值为 null 时应返回 undefined", () => {
    expect(getNestedValue({ user: null }, "user.name")).toBeUndefined();
  });

  it("路径不存在时应返回 undefined", () => {
    expect(getNestedValue(obj, "user.nonexistent.field")).toBeUndefined();
  });

  it("空对象时应返回 undefined", () => {
    expect(getNestedValue({}, "any.path")).toBeUndefined();
  });
});

describe("genId", () => {
  it("应生成带前缀的 id", () => {
    const id = genId("test");
    expect(id).toMatch(/^test-\d+-\d+$/);
  });

  it("每次调用应生成不同的 id", () => {
    const ids = new Set([genId("a"), genId("a"), genId("a")]);
    expect(ids.size).toBe(3);
  });
});

describe("cloneSchema", () => {
  it("应深拷贝节点", () => {
    const original = makeNode({ props: { key: "value" } });
    const cloned = cloneSchema(original);
    expect(cloned).not.toBe(original);
    expect(cloned.props).not.toBe(original.props);
    expect(cloned.props).toEqual(original.props);
  });

  it("应深拷贝 children", () => {
    const original = makeNode({
      children: [makeNode({ id: "child-1", props: { x: 1 } })],
    });
    const cloned = cloneSchema(original);
    expect(cloned.children).not.toBe(original.children);
    expect(cloned.children![0].props).toEqual({ x: 1 });
  });

  it("应深拷贝 events", () => {
    const original = makeNode({
      events: [{ trigger: "onClick" as const, actions: [] }],
    });
    const cloned = cloneSchema(original);
    expect(cloned.events).not.toBe(original.events);
    expect(cloned.events![0].actions).not.toBe(original.events![0].actions);
  });

  it("应深拷贝 dataSource", () => {
    const original = makeNode({
      dataSource: { type: "static", config: { value: "hello" } },
    });
    const cloned = cloneSchema(original);
    expect(cloned.dataSource).not.toBe(original.dataSource);
    expect(cloned.dataSource!.config).not.toBe(original.dataSource!.config);
  });
});

describe("cloneSchemaWithNewIds", () => {
  it("应为整棵树重新生成 id", () => {
    const original = makeNode({
      id: "old-1",
      children: [makeNode({ id: "old-2" })],
    });
    const cloned = cloneSchemaWithNewIds(original);
    expect(cloned.id).not.toBe(original.id);
    expect(cloned.id).toMatch(/^old-\d+-\d+$/);
    expect(cloned.children![0].id).not.toBe(original.children![0].id);
  });
});

describe("findNodeById", () => {
  it("应找到根节点", () => {
    const root = makeNode();
    expect(findNodeById(root, "node-1")).toBe(root);
  });

  it("应找到子节点", () => {
    const child = makeNode({ id: "child-1" });
    const root = makeNode({ children: [child] });
    expect(findNodeById(root, "child-1")).toBe(child);
  });

  it("应找到深层节点", () => {
    const deep = makeNode({ id: "deep" });
    const root = makeNode({
      children: [makeNode({ id: "mid", children: [deep] })],
    });
    expect(findNodeById(root, "deep")).toBe(deep);
  });

  it("找不到时应返回 null", () => {
    expect(findNodeById(makeNode(), "nonexistent")).toBeNull();
  });
});

describe("removeNodeById", () => {
  it("应移除直接子节点", () => {
    const root = makeNode({
      children: [makeNode({ id: "a" }), makeNode({ id: "b" })],
    });
    const result = removeNodeById(root, "a");
    expect(result.children!.length).toBe(1);
    expect(result.children![0].id).toBe("b");
  });

  it("应递归移除深层节点", () => {
    const root = makeNode({
      children: [makeNode({ id: "mid", children: [makeNode({ id: "deep" })] })],
    });
    const result = removeNodeById(root, "deep");
    expect(result.children![0].children!.length).toBe(0);
  });

  it("找不到时应返回原树", () => {
    const root = makeNode();
    const result = removeNodeById(root, "nonexistent");
    expect(result).toBe(root);
  });
});

describe("detachNodeById", () => {
  it("应摘除子节点并返回", () => {
    const child = makeNode({ id: "child" });
    const root = makeNode({ children: [child] });
    const { tree, detached } = detachNodeById(root, "child");
    expect(tree.children!.length).toBe(0);
    expect(detached).toBe(child);
  });

  it("根节点不能摘除", () => {
    const root = makeNode({ id: "root" });
    const { tree, detached } = detachNodeById(root, "root");
    expect(detached).toBeNull();
    expect(tree).toBe(root);
  });

  it("找不到时应返回 null detached", () => {
    const root = makeNode();
    const { detached } = detachNodeById(root, "nonexistent");
    expect(detached).toBeNull();
  });
});

describe("appendChildToNode", () => {
  it("应追加子节点到根", () => {
    const root = makeNode();
    const child = makeNode({ id: "new-child" });
    const result = appendChildToNode(root, root.id, child);
    expect(result.children!.length).toBe(1);
    expect(result.children![0].id).toBe("new-child");
  });

  it("应追加到深层父节点", () => {
    const root = makeNode({
      children: [makeNode({ id: "mid", children: [makeNode({ id: "deep" })] })],
    });
    const child = makeNode({ id: "sibling" });
    const result = appendChildToNode(root, "deep", child);
    const mid = result.children![0];
    const deep = mid.children![0];
    expect(deep.children!.length).toBe(1);
    expect(deep.children![0].id).toBe("sibling");
  });
});

describe("insertBeforeSibling", () => {
  it("应在 sibling 前插入", () => {
    const root = makeNode({
      children: [makeNode({ id: "a" }), makeNode({ id: "b" })],
    });
    const child = makeNode({ id: "x" });
    const result = insertBeforeSibling(root, root.id, child, "b");
    expect(result.children!.map((c) => c.id)).toEqual(["a", "x", "b"]);
  });

  it("sibling 不存在时退化为追加", () => {
    const root = makeNode({ children: [makeNode({ id: "a" })] });
    const child = makeNode({ id: "x" });
    const result = insertBeforeSibling(root, root.id, child, "nonexistent");
    expect(result.children!.map((c) => c.id)).toEqual(["a", "x"]);
  });
});

describe("flattenNodes", () => {
  it("应收集所有节点（DFS 序）", () => {
    const root = makeNode({
      id: "root",
      children: [makeNode({ id: "a" }), makeNode({ id: "b" })],
    });
    const nodes = flattenNodes(root);
    expect(nodes.map((n) => n.id)).toEqual(["root", "a", "b"]);
  });

  it("单节点应返回自身", () => {
    const nodes = flattenNodes(makeNode());
    expect(nodes.length).toBe(1);
  });
});

describe("findParent", () => {
  it("应找到直接父节点", () => {
    const child = makeNode({ id: "child" });
    const root = makeNode({ children: [child] });
    expect(findParent(root, "child")).toBe(root);
  });

  it("应找到深层父节点", () => {
    const deep = makeNode({ id: "deep" });
    const mid = makeNode({ id: "mid", children: [deep] });
    const root = makeNode({ children: [mid] });
    expect(findParent(root, "deep")).toBe(mid);
  });

  it("找不到时应返回 null", () => {
    expect(findParent(makeNode(), "nonexistent")).toBeNull();
  });
});

describe("isDescendant", () => {
  it("直接子节点应返回 true", () => {
    const root = makeNode({ children: [makeNode({ id: "child" })] });
    expect(isDescendant(root, "child")).toBe(true);
  });

  it("非子孙节点应返回 false", () => {
    expect(isDescendant(makeNode(), "other")).toBe(false);
  });

  it("深层子孙应返回 true", () => {
    const root = makeNode({
      children: [makeNode({ id: "mid", children: [makeNode({ id: "deep" })] })],
    });
    expect(isDescendant(root, "deep")).toBe(true);
  });
});

describe("updateNodeById", () => {
  it("应更新节点 props", () => {
    const root = makeNode({ props: { old: true } });
    const result = updateNodeById(root, root.id, (node) => {
      node.props.newProp = "value";
    });
    expect(result.props).toEqual({ old: true, newProp: "value" });
  });

  it("应更新子节点", () => {
    const root = makeNode({
      children: [makeNode({ id: "child", props: { x: 1 } })],
    });
    const result = updateNodeById(root, "child", (node) => {
      node.props.x = 2;
    });
    expect(result.children![0].props.x).toBe(2);
  });

  it("找不到时应返回原树", () => {
    const root = makeNode();
    const result = updateNodeById(root, "nonexistent", () => {});
    expect(result).toBe(root);
  });
});

describe("getDefaultProps", () => {
  const t = (key: string) => key;

  it("应返回 Input 的默认 props", () => {
    const defaults = getDefaultProps(t);
    expect(defaults.Input).toEqual({
      name: "field",
      label: "dynamicUIManager.defaults.inputLabel",
      placeholder: "dynamicUIManager.defaults.inputPlaceholder",
    });
  });

  it("应返回 Container 的空 props", () => {
    const defaults = getDefaultProps(t);
    expect(defaults.Container).toEqual({});
  });

  it("应返回 Grid 的 columns", () => {
    const defaults = getDefaultProps(t);
    expect(defaults.Grid).toEqual({ columns: 3 });
  });
});

describe("resolveDynamicArray", () => {
  it("explicit 数组存在时应优先返回", () => {
    const result = resolveDynamicArray([1, 2, 3], { key: "other" }, "key");
    expect(result).toEqual([1, 2, 3]);
  });

  it("无 explicit 时应从 dataContext 获取", () => {
    const result = resolveDynamicArray(null, { key: [4, 5, 6] }, "key");
    expect(result).toEqual([4, 5, 6]);
  });

  it("dataContext 中非数组时应返回空数组", () => {
    const result = resolveDynamicArray(null, { key: "not-array" }, "key");
    expect(result).toEqual([]);
  });

  it("两者皆无时应返回空数组", () => {
    const result = resolveDynamicArray(undefined, undefined, "key");
    expect(result).toEqual([]);
  });
});
