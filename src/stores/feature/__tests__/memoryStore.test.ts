// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  isTauri: () => false,
}));

import { useMemoryStore } from "@/stores/feature/memoryStore";
import type { MemoryItem, MemoryNamespace } from "@/types";

const NS_ID = "ns-1";
const ITEM_ID = "item-1";

function makeNamespace(overrides?: Partial<MemoryNamespace>): MemoryNamespace {
  return {
    id: NS_ID,
    name: "Test Namespace",
    scope: "project",
    sortOrder: 0,
    ...overrides,
  };
}

function makeItem(overrides?: Partial<MemoryItem>): MemoryItem {
  return {
    id: ITEM_ID,
    namespaceId: NS_ID,
    title: "Test Item",
    content: "Test content",
    source: "manual",
    indexStatus: "indexed",
    updatedAt: "2025-01-01T00:00:00Z",
    tier: "working",
    importance: 0.5,
    accessCount: 0,
    decayRate: 0.02,
    memoryNature: "semantic",
    tags: [],
    applicabilityTags: [],
    confirmed: 0,
    ...overrides,
  };
}

describe("memoryStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useMemoryStore.setState({
      namespaces: [],
      items: [],
      loading: false,
      error: null,
      selectedNamespaceId: null,
    });
  });

  describe("loadNamespaces", () => {
    it("loads namespaces from backend", async () => {
      const namespaces = [makeNamespace(), makeNamespace({ id: "ns-2", name: "Namespace 2" })];
      invokeMock.mockResolvedValueOnce(namespaces);

      await useMemoryStore.getState().loadNamespaces();

      expect(invokeMock).toHaveBeenCalledWith("list_memory_namespaces");
      expect(useMemoryStore.getState().namespaces).toEqual(namespaces);
      expect(useMemoryStore.getState().loading).toBe(false);
    });

    it("handles non-array response gracefully", async () => {
      invokeMock.mockResolvedValueOnce(null);

      await useMemoryStore.getState().loadNamespaces();

      expect(useMemoryStore.getState().namespaces).toEqual([]);
    });

    it("sets error on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Network error"));

      await useMemoryStore.getState().loadNamespaces();

      expect(useMemoryStore.getState().error).toBe("Network error");
      expect(useMemoryStore.getState().loading).toBe(false);
    });
  });

  describe("createNamespace", () => {
    it("creates a namespace and adds to store", async () => {
      const ns = makeNamespace();
      invokeMock.mockResolvedValueOnce(ns);

      const result = await useMemoryStore.getState().createNamespace("New NS", "team");

      expect(invokeMock).toHaveBeenCalledWith("create_memory_namespace", {
        input: { name: "New NS", scope: "team", embeddingProvider: undefined },
      });
      expect(result).toEqual(ns);
      expect(useMemoryStore.getState().namespaces).toContainEqual(ns);
      expect(useMemoryStore.getState().error).toBeNull();
    });

    it("returns null and sets error on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Create failed"));

      const result = await useMemoryStore.getState().createNamespace("New NS", "team");

      expect(result).toBeNull();
      expect(useMemoryStore.getState().error).toBe("Create failed");
    });
  });

  describe("deleteNamespace", () => {
    it("deletes a namespace and removes from store", async () => {
      const ns = makeNamespace();
      useMemoryStore.setState({ namespaces: [ns], selectedNamespaceId: NS_ID });

      invokeMock.mockResolvedValueOnce(undefined);

      await useMemoryStore.getState().deleteNamespace(NS_ID);

      expect(invokeMock).toHaveBeenCalledWith("delete_memory_namespace", { id: NS_ID });
      expect(useMemoryStore.getState().namespaces).toHaveLength(0);
      expect(useMemoryStore.getState().selectedNamespaceId).toBeNull();
    });

    it("clears selectedNamespaceId and items when deleting selected namespace", async () => {
      const ns = makeNamespace();
      useMemoryStore.setState({
        namespaces: [ns],
        items: [makeItem()],
        selectedNamespaceId: NS_ID,
      });

      invokeMock.mockResolvedValueOnce(undefined);

      await useMemoryStore.getState().deleteNamespace(NS_ID);

      expect(useMemoryStore.getState().items).toEqual([]);
      expect(useMemoryStore.getState().selectedNamespaceId).toBeNull();
    });
  });

  describe("updateNamespace", () => {
    it("updates a namespace in store", async () => {
      const ns = makeNamespace();
      const updated = makeNamespace({ name: "Updated NS" });
      useMemoryStore.setState({ namespaces: [ns] });

      invokeMock.mockResolvedValueOnce(updated);

      await useMemoryStore.getState().updateNamespace(NS_ID, { name: "Updated NS" });

      expect(invokeMock).toHaveBeenCalledWith("update_memory_namespace", {
        id: NS_ID,
        input: { name: "Updated NS" },
      });
      expect(useMemoryStore.getState().namespaces[0].name).toBe("Updated NS");
    });
  });

  describe("loadItems", () => {
    it("loads items for a namespace", async () => {
      const items = [makeItem(), makeItem({ id: "item-2", title: "Item 2" })];
      invokeMock.mockResolvedValueOnce(items);

      await useMemoryStore.getState().loadItems(NS_ID);

      expect(invokeMock).toHaveBeenCalledWith("list_memory_items", { namespaceId: NS_ID });
      expect(useMemoryStore.getState().items).toEqual(items);
      expect(useMemoryStore.getState().loading).toBe(false);
    });
  });

  describe("addItem", () => {
    it("adds an item and reloads", async () => {
      invokeMock.mockResolvedValueOnce(undefined); // add_memory_item
      invokeMock.mockResolvedValueOnce([makeItem()]); // list_memory_items (reload)

      await useMemoryStore.getState().addItem(NS_ID, "New Item", "Content");

      expect(invokeMock).toHaveBeenCalledWith("add_memory_item", {
        input: { namespace_id: NS_ID, title: "New Item", content: "Content" },
      });
      expect(useMemoryStore.getState().items).toHaveLength(1);
    });
  });

  describe("deleteItem", () => {
    it("deletes an item and reloads", async () => {
      invokeMock.mockResolvedValueOnce(undefined); // delete_memory_item
      invokeMock.mockResolvedValueOnce([]); // list_memory_items (reload)

      await useMemoryStore.getState().deleteItem(NS_ID, ITEM_ID);

      expect(invokeMock).toHaveBeenCalledWith("delete_memory_item", { namespaceId: NS_ID, id: ITEM_ID });
      expect(useMemoryStore.getState().items).toHaveLength(0);
    });
  });

  describe("updateItem", () => {
    it("updates an item and reloads", async () => {
      invokeMock.mockResolvedValueOnce(undefined); // update_memory_item
      invokeMock.mockResolvedValueOnce([makeItem({ title: "Updated" })]); // reload

      await useMemoryStore.getState().updateItem(NS_ID, ITEM_ID, { title: "Updated" });

      expect(invokeMock).toHaveBeenCalledWith("update_memory_item", {
        namespaceId: NS_ID,
        id: ITEM_ID,
        input: { title: "Updated" },
      });
      expect(useMemoryStore.getState().items[0].title).toBe("Updated");
    });
  });

  describe("setSelectedNamespaceId", () => {
    it("sets the selected namespace ID", () => {
      useMemoryStore.getState().setSelectedNamespaceId(NS_ID);
      expect(useMemoryStore.getState().selectedNamespaceId).toBe(NS_ID);

      useMemoryStore.getState().setSelectedNamespaceId(null);
      expect(useMemoryStore.getState().selectedNamespaceId).toBeNull();
    });
  });

  describe("reorderNamespaces", () => {
    it("reorders namespaces by ID list", async () => {
      const ns1 = makeNamespace({ id: "ns-1", name: "First" });
      const ns2 = makeNamespace({ id: "ns-2", name: "Second" });
      const ns3 = makeNamespace({ id: "ns-3", name: "Third" });
      useMemoryStore.setState({ namespaces: [ns1, ns2, ns3] });

      invokeMock.mockResolvedValueOnce(undefined);

      await useMemoryStore.getState().reorderNamespaces(["ns-3", "ns-1", "ns-2"]);

      expect(invokeMock).toHaveBeenCalledWith("reorder_memory_namespaces", {
        namespaceIds: ["ns-3", "ns-1", "ns-2"],
      });

      const namespaces = useMemoryStore.getState().namespaces;
      expect(namespaces[0].id).toBe("ns-3");
      expect(namespaces[0].sortOrder).toBe(0);
      expect(namespaces[1].id).toBe("ns-1");
      expect(namespaces[1].sortOrder).toBe(1);
      expect(namespaces[2].id).toBe("ns-2");
      expect(namespaces[2].sortOrder).toBe(2);
    });
  });
});
