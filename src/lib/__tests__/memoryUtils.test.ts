import { describe, expect, it } from "vitest";

import { buildKnowledgeTag, buildMemoryTag } from "../memoryUtils";

describe("buildKnowledgeTag", () => {
  it("returns searching tag", () => {
    const tag = buildKnowledgeTag("searching");
    expect(tag).toContain('<knowledge-retrieval status="searching"');
    expect(tag).toContain("data-axagent=\"1\"");
  });

  it("returns error tag", () => {
    const tag = buildKnowledgeTag("error");
    expect(tag).toContain('<knowledge-retrieval status="error"');
  });

  it("returns done tag with empty sources", () => {
    const tag = buildKnowledgeTag("done", []);
    expect(tag).toContain('<knowledge-retrieval status="done"');
    expect(tag).toContain("[]");
  });

  it("returns done tag with sources", () => {
    const sources = [
      {
        source_type: "knowledge" as const,
        container_id: "kb-1",
        items: [
          {
            content: "Some knowledge content",
            score: 0.95,
            document_id: "doc-1",
            id: "chunk-1",
            document_name: "My Document",
          },
        ],
      },
    ];
    const tag = buildKnowledgeTag("done", sources);
    expect(tag).toContain('<knowledge-retrieval status="done"');
    expect(tag).toContain("Some knowledge content");
    expect(tag).toContain("My Document");
    expect(tag).toContain("doc-1");
  });

  it("handles done status with undefined sources (defaults to empty array)", () => {
    const tag = buildKnowledgeTag("done");
    expect(tag).toContain("[]");
  });
});

describe("buildMemoryTag", () => {
  it("returns searching tag", () => {
    const tag = buildMemoryTag("searching");
    expect(tag).toContain('<memory-retrieval status="searching"');
    expect(tag).toContain("data-axagent=\"1\"");
  });

  it("returns error tag", () => {
    const tag = buildMemoryTag("error");
    expect(tag).toContain('<memory-retrieval status="error"');
  });

  it("returns done tag with empty sources", () => {
    const tag = buildMemoryTag("done", []);
    expect(tag).toContain('<memory-retrieval status="done"');
    expect(tag).toContain("[]");
  });

  it("returns done tag with memory sources", () => {
    const sources = [
      {
        source_type: "memory" as const,
        container_id: "mem-1",
        items: [
          {
            content: "User prefers dark mode",
            score: 0.88,
            document_id: "mem-doc-1",
            id: "mem-chunk-1",
          },
        ],
      },
    ];
    const tag = buildMemoryTag("done", sources);
    expect(tag).toContain('<memory-retrieval status="done"');
    expect(tag).toContain("User prefers dark mode");
    expect(tag).toContain("mem-doc-1");
  });

  it("handles done status with undefined sources (defaults to empty array)", () => {
    const tag = buildMemoryTag("done");
    expect(tag).toContain("[]");
  });

  it("produces distinct tags from buildKnowledgeTag", () => {
    const kTag = buildKnowledgeTag("searching");
    const mTag = buildMemoryTag("searching");
    expect(kTag).not.toBe(mTag);
    expect(kTag).toContain("knowledge-retrieval");
    expect(mTag).toContain("memory-retrieval");
  });
});
