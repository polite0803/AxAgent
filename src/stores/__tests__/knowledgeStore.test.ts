// SPDX-License-Identifier: AGPL-3.0-only

import type { ImportDirectoryResult, KnowledgeBase, KnowledgeDocument } from "@/types";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
  logIpcError: vi.fn(() => vi.fn()),
}));

import { useKnowledgeStore } from "@/stores/feature/knowledgeStore";

describe("knowledgeStore - importDirectory", () => {
  const BASE_ID = "kb-1";
  const DIR_PATH = "/tmp/test-docs";
  const DOCS: KnowledgeDocument[] = [];

  function makeResult(overrides?: Partial<ImportDirectoryResult>): ImportDirectoryResult {
    return {
      baseId: BASE_ID,
      importedCount: 3,
      skippedCount: 1,
      errorCount: 0,
      imported: [],
      skipped: [".gitkeep"],
      errors: [],
      ...overrides,
    };
  }

  beforeEach(() => {
    vi.clearAllMocks();
    useKnowledgeStore.setState({
      documents: [],
      error: null,
      loading: false,
    });
  });

  it("invokes import_knowledge_directory with correct arguments", async () => {
    const result = makeResult();
    invokeMock.mockResolvedValueOnce(result); // import_knowledge_directory
    invokeMock.mockResolvedValueOnce(DOCS); // list_knowledge_documents (from loadDocuments)

    const res = await useKnowledgeStore.getState().importDirectory(
      BASE_ID,
      DIR_PATH,
      true,
      ["md", "txt"],
    );

    // 第一个 invoke 调用：import_knowledge_directory
    expect(invokeMock).toHaveBeenNthCalledWith(
      1,
      "import_knowledge_directory",
      {
        baseId: BASE_ID,
        directoryPath: DIR_PATH,
        recursive: true,
        extensions: ["md", "txt"],
      },
    );

    // 第二个 invoke 调用：reload 文档
    expect(invokeMock).toHaveBeenNthCalledWith(
      2,
      "list_knowledge_documents",
      { baseId: BASE_ID },
    );

    expect(res).toEqual(result);
    expect(useKnowledgeStore.getState().error).toBeNull();
  });

  it("passes undefined extensions when no filter given", async () => {
    invokeMock.mockResolvedValueOnce(makeResult());
    invokeMock.mockResolvedValueOnce(DOCS);

    await useKnowledgeStore.getState().importDirectory(
      BASE_ID,
      DIR_PATH,
      false,
    );

    expect(invokeMock).toHaveBeenNthCalledWith(
      1,
      "import_knowledge_directory",
      {
        baseId: BASE_ID,
        directoryPath: DIR_PATH,
        recursive: false,
        extensions: undefined,
      },
    );
  });

  it("sets error and re-throws on failure", async () => {
    invokeMock.mockRejectedValueOnce(new Error("DB error"));

    await expect(
      useKnowledgeStore.getState().importDirectory(BASE_ID, DIR_PATH),
    ).rejects.toThrow("DB error");

    expect(useKnowledgeStore.getState().error).toBe("DB error");
  });

  it("returns result with zero counts for empty directory", async () => {
    const emptyResult = makeResult({
      importedCount: 0,
      skippedCount: 0,
      errorCount: 0,
      imported: [],
      skipped: [],
      errors: [],
    });
    invokeMock.mockResolvedValueOnce(emptyResult);
    invokeMock.mockResolvedValueOnce(DOCS);

    const res = await useKnowledgeStore.getState().importDirectory(
      BASE_ID,
      "/tmp/empty-dir",
    );

    expect(res.importedCount).toBe(0);
    expect(res.skippedCount).toBe(0);
    expect(res.errorCount).toBe(0);
    expect(res.imported).toHaveLength(0);
    expect(res.skipped).toHaveLength(0);
    expect(res.errors).toHaveLength(0);
  });

  it("refreshes documents state after a successful import", async () => {
    const refreshed: KnowledgeDocument[] = [
      {
        id: "doc-1",
        knowledgeBaseId: BASE_ID,
        title: "a.md",
        sourcePath: "/tmp/test-docs/a.md",
        mimeType: "text/markdown",
        sizeBytes: 10,
        indexingStatus: "pending",
        docType: "markdown",
      },
    ];
    invokeMock.mockResolvedValueOnce(makeResult()); // import_knowledge_directory
    invokeMock.mockResolvedValueOnce(refreshed); // loadDocuments 刷新

    await useKnowledgeStore.getState().importDirectory(BASE_ID, DIR_PATH);

    // 断言 documents 被刷新（原审计 L17：成功用例未断言此点）
    expect(useKnowledgeStore.getState().documents).toEqual(refreshed);
    expect(useKnowledgeStore.getState().loading).toBe(false);
  });
});

// ── 知识库 CRUD ────────────────────────────────────────────
describe("knowledgeStore - base CRUD", () => {
  function makeBase(overrides?: Partial<KnowledgeBase>): KnowledgeBase {
    return {
      id: "kb-1",
      name: "Base",
      enabled: true,
      sortOrder: 0,
      ...overrides,
    };
  }

  beforeEach(() => {
    vi.clearAllMocks();
    useKnowledgeStore.setState({ bases: [], documents: [], error: null, loading: false });
  });

  it("loadBases populates bases and clears error", async () => {
    const bases = [makeBase(), makeBase({ id: "kb-2", name: "B2" })];
    invokeMock.mockResolvedValueOnce(bases);

    await useKnowledgeStore.getState().loadBases();

    expect(invokeMock).toHaveBeenCalledWith("list_knowledge_bases");
    expect(useKnowledgeStore.getState().bases).toEqual(bases);
    expect(useKnowledgeStore.getState().error).toBeNull();
    expect(useKnowledgeStore.getState().loading).toBe(false);
  });

  it("loadBases coerces non-array result to empty array", async () => {
    invokeMock.mockResolvedValueOnce(null);

    await useKnowledgeStore.getState().loadBases();

    expect(useKnowledgeStore.getState().bases).toEqual([]);
  });

  it("createBase appends returned base to state", async () => {
    const created = makeBase({ id: "kb-new", name: "New" });
    invokeMock.mockResolvedValueOnce(created);

    const res = await useKnowledgeStore.getState().createBase({ name: "New" });

    expect(invokeMock).toHaveBeenCalledWith("create_knowledge_base", { input: { name: "New" } });
    expect(res).toEqual(created);
    expect(useKnowledgeStore.getState().bases).toContainEqual(created);
  });

  it("createBase returns null and sets error on failure", async () => {
    invokeMock.mockRejectedValueOnce(new Error("boom"));

    const res = await useKnowledgeStore.getState().createBase({ name: "X" });

    expect(res).toBeNull();
    expect(useKnowledgeStore.getState().error).toBe("boom");
  });

  it("updateBase replaces the matching base", async () => {
    const original = makeBase();
    useKnowledgeStore.setState({ bases: [original] });
    const updated = makeBase({ name: "Renamed" });
    invokeMock.mockResolvedValueOnce(updated);

    await useKnowledgeStore.getState().updateBase("kb-1", { name: "Renamed" });

    expect(useKnowledgeStore.getState().bases[0]).toEqual(updated);
  });

  it("deleteBase removes the matching base", async () => {
    useKnowledgeStore.setState({ bases: [makeBase(), makeBase({ id: "kb-2" })] });
    invokeMock.mockResolvedValueOnce(undefined);

    await useKnowledgeStore.getState().deleteBase("kb-1");

    expect(useKnowledgeStore.getState().bases.map((b) => b.id)).toEqual(["kb-2"]);
  });
});

// ── 文档操作 ───────────────────────────────────────────────
describe("knowledgeStore - documents", () => {
  const BASE_ID = "kb-1";

  beforeEach(() => {
    vi.clearAllMocks();
    useKnowledgeStore.setState({ bases: [], documents: [], error: null, loading: false });
  });

  it("loadDocuments sets documents from backend", async () => {
    const docs: KnowledgeDocument[] = [
      {
        id: "d1",
        knowledgeBaseId: BASE_ID,
        title: "t",
        sourcePath: "/p",
        mimeType: "text/plain",
        sizeBytes: 1,
        indexingStatus: "ready",
        docType: "text",
      },
    ];
    invokeMock.mockResolvedValueOnce(docs);

    await useKnowledgeStore.getState().loadDocuments(BASE_ID);

    expect(invokeMock).toHaveBeenCalledWith("list_knowledge_documents", { baseId: BASE_ID });
    expect(useKnowledgeStore.getState().documents).toEqual(docs);
  });

  it("addDocument reloads documents after add", async () => {
    invokeMock.mockResolvedValueOnce(undefined); // add_knowledge_document
    invokeMock.mockResolvedValueOnce([]); // list_knowledge_documents

    await useKnowledgeStore.getState().addDocument(BASE_ID, "t", "/p", "text/plain");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "add_knowledge_document", {
      baseId: BASE_ID,
      title: "t",
      sourcePath: "/p",
      mimeType: "text/plain",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "list_knowledge_documents", { baseId: BASE_ID });
  });

  it("deleteDocument passes baseId/id and reloads", async () => {
    invokeMock.mockResolvedValueOnce(undefined); // delete_knowledge_document
    invokeMock.mockResolvedValueOnce([]); // list_knowledge_documents

    await useKnowledgeStore.getState().deleteDocument(BASE_ID, "doc-9");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "delete_knowledge_document", {
      baseId: BASE_ID,
      id: "doc-9",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "list_knowledge_documents", { baseId: BASE_ID });
  });

  it("deleteDocument sets error and re-throws on failure", async () => {
    invokeMock.mockRejectedValueOnce(new Error("nope"));

    await expect(
      useKnowledgeStore.getState().deleteDocument(BASE_ID, "doc-9"),
    ).rejects.toThrow("nope");
    expect(useKnowledgeStore.getState().error).toBe("nope");
  });
});
