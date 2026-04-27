import { describe, expect, it } from "vitest";
import { getStreamingLoadingState, shouldRenderAssistantMarkdownFromContent } from "../chatStreaming";

describe("chat streaming helpers", () => {
  it("derives bubble and footer loading state from stream progress and content presence", () => {
    expect(getStreamingLoadingState(true, "")).toEqual({
      bubbleLoading: true,
      footerLoading: false,
    });

    expect(getStreamingLoadingState(true, "hello")).toEqual({
      bubbleLoading: false,
      footerLoading: true,
    });

    expect(getStreamingLoadingState(false, "hello")).toEqual({
      bubbleLoading: false,
      footerLoading: false,
    });
  });

  it("keeps streamed assistant messages on the content renderer after completion", () => {
    expect(shouldRenderAssistantMarkdownFromContent(true, false)).toBe(true);
    expect(shouldRenderAssistantMarkdownFromContent(false, true)).toBe(true);
    expect(shouldRenderAssistantMarkdownFromContent(false, false)).toBe(false);
  });
});
