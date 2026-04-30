import { describe, expect, it } from "vitest";

import type { SearchResultItem } from "@/types";
import { buildSearchTag, formatSearchContent, parseSearchContent } from "../searchUtils";

const mockResult: SearchResultItem = {
  title: "Test Result",
  url: "https://example.com",
  content: "This is a test search result.",
};

describe("formatSearchContent", () => {
  it("prepends search marker with metadata", () => {
    const content = formatSearchContent([mockResult], "user question");
    expect(content).toContain("<!-- search:");
    expect(content).toContain("Test Result");
    expect(content).toContain("https://example.com");
    expect(content).toContain("user question");
  });

  it("includes separator between search results and user content", () => {
    const content = formatSearchContent([mockResult], "hello");
    expect(content).toContain("\n---\n");
  });

  it("handles empty results array", () => {
    const content = formatSearchContent([], "just a question");
    expect(content).toContain("<!-- search:");
    expect(content).toContain("just a question");
    expect(content).toContain("[]");
  });

  it("formats multiple results with numbering", () => {
    const results: SearchResultItem[] = [
      { title: "First", url: "https://a.com", content: "Content A" },
      { title: "Second", url: "https://b.com", content: "Content B" },
    ];
    const content = formatSearchContent(results, "question");
    expect(content).toContain("1. **First** - https://a.com");
    expect(content).toContain("2. **Second** - https://b.com");
  });
});

describe("buildSearchTag", () => {
  it("returns searching tag for 'searching' status", () => {
    const tag = buildSearchTag("searching");
    expect(tag).toContain('<web-search status="searching"');
    expect(tag).toContain("data-axagent=\"1\"");
  });

  it("returns error tag for 'error' status", () => {
    const tag = buildSearchTag("error");
    expect(tag).toContain('<web-search status="error"');
  });

  it("returns done tag with JSON results for 'done' status", () => {
    const tag = buildSearchTag("done", [mockResult]);
    expect(tag).toContain('<web-search status="done"');
    expect(tag).toContain("Test Result");
    expect(tag).toContain("https://example.com");
  });

  it("handles done status with empty results", () => {
    const tag = buildSearchTag("done", []);
    expect(tag).toContain('<web-search status="done"');
    expect(tag).toContain("[]");
  });
});

describe("parseSearchContent", () => {
  it("detects content without search marker", () => {
    const result = parseSearchContent("Plain user message");
    expect(result.hasSearch).toBe(false);
    expect(result.sources).toEqual([]);
    expect(result.userContent).toBe("Plain user message");
  });

  it("parses content with search marker and sources", () => {
    const enriched = formatSearchContent([mockResult], "user message");
    const result = parseSearchContent(enriched);
    expect(result.hasSearch).toBe(true);
    expect(result.sources).toHaveLength(1);
    expect(result.sources[0].title).toBe("Test Result");
    expect(result.sources[0].url).toBe("https://example.com");
    expect(result.userContent).toBe("user message");
  });

  it("handles corrupted search marker gracefully", () => {
    const corrupted = "<!-- search:{invalid json} -->\nresults\n\n---\n\nuser msg";
    const result = parseSearchContent(corrupted);
    expect(result.hasSearch).toBe(true);
    expect(result.sources).toEqual([]);
  });

  it("handles missing separator — falls back to content after marker", () => {
    const content = "<!-- search:{\"sources\":[]} -->\nsome content";
    const result = parseSearchContent(content);
    expect(result.hasSearch).toBe(true);
    // Falls back to content after marker end — includes leading \n from original
    expect(result.userContent).toBe("\nsome content");
  });
});
