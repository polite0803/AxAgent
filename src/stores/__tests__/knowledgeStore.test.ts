// SPDX-License-Identifier: AGPL-3.0-only

import type { ImportDirectoryResult, KnowledgeDocument } from "@/types";
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

    expect(useKnowledgeStore.getState().error).toBe("Error: DB error");
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
});
