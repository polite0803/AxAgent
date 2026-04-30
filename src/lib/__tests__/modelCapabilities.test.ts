import { describe, expect, it } from "vitest";

import {
  findModelByIds,
  getEditableCapabilities,
  getVisibleModelCapabilities,
  modelHasCapability,
  sanitizeModelCapabilities,
  supportsReasoning,
} from "../modelCapabilities";

import type { Model, ProviderConfig } from "@/types";

const mockModel = (overrides: Partial<Model> = {}): Model => ({
  model_id: "gpt-4",
  name: "GPT-4",
  model_type: "Chat",
  provider_id: "p-1",
  capabilities: ["Vision", "FunctionCalling", "Reasoning"],
  max_tokens: 4096,
  enabled: true,
  param_overrides: null,
  ...overrides,
});

const mockProvider = (overrides: Partial<ProviderConfig> = {}): ProviderConfig => ({
  id: "p-1",
  name: "OpenAI",
  provider_type: "openai",
  api_host: "https://api.openai.com",
  api_path: null,
  enabled: true,
  models: [mockModel(), mockModel({ model_id: "gpt-3.5", capabilities: ["FunctionCalling"] })],
  keys: [],
  proxy_config: null,
  custom_headers: null,
  icon: null,
  builtin_id: null,
  sort_order: 0,
  created_at: 0,
  updated_at: 0,
  ...overrides,
});

describe("getEditableCapabilities", () => {
  it("returns CHAT_MODEL_CAPABILITIES for Chat type", () => {
    const caps = getEditableCapabilities("Chat");
    expect(caps).toContain("Vision");
    expect(caps).toContain("FunctionCalling");
    expect(caps).toContain("Reasoning");
  });

  it("returns empty array for non-Chat types", () => {
    expect(getEditableCapabilities("Embedding")).toEqual([]);
    expect(getEditableCapabilities("Voice")).toEqual([]);
  });

  it("returns CHAT_MODEL_CAPABILITIES when type is null/undefined", () => {
    expect(getEditableCapabilities(null)).toHaveLength(3);
    expect(getEditableCapabilities(undefined)).toHaveLength(3);
  });
});

describe("sanitizeModelCapabilities", () => {
  it("filters out capabilities not in the allowed set", () => {
    const result = sanitizeModelCapabilities("Chat", ["Vision", "Unknown" as any]);
    expect(result).toEqual(["Vision"]);
  });

  it("returns empty when modelType is non-Chat", () => {
    const result = sanitizeModelCapabilities("Embedding", ["Vision"]);
    expect(result).toEqual([]);
  });

  it("keeps all valid capabilities", () => {
    const result = sanitizeModelCapabilities("Chat", ["Vision", "FunctionCalling", "Reasoning"]);
    expect(result).toHaveLength(3);
  });
});

describe("getVisibleModelCapabilities", () => {
  it("returns sanitized capabilities for a chat model", () => {
    const model = mockModel({ model_type: "Chat", capabilities: ["Vision", "Reasoning"] });
    const result = getVisibleModelCapabilities(model);
    expect(result).toEqual(["Vision", "Reasoning"]);
  });

  it("returns empty for a non-Chat model", () => {
    const model = mockModel({ model_type: "Embedding", capabilities: ["Vision"] });
    const result = getVisibleModelCapabilities(model);
    expect(result).toEqual([]);
  });
});

describe("modelHasCapability", () => {
  it("returns true when model has capability", () => {
    const model = mockModel({ capabilities: ["Vision", "Reasoning"] });
    expect(modelHasCapability(model, "Vision")).toBe(true);
    expect(modelHasCapability(model, "Reasoning")).toBe(true);
  });

  it("returns false when model lacks capability", () => {
    const model = mockModel({ capabilities: ["Vision"] });
    expect(modelHasCapability(model, "Reasoning")).toBe(false);
  });

  it("returns false for null model", () => {
    expect(modelHasCapability(null, "Vision")).toBe(false);
    expect(modelHasCapability(undefined, "Vision")).toBe(false);
  });
});

describe("supportsReasoning", () => {
  it("returns true when model has Reasoning capability", () => {
    expect(supportsReasoning(mockModel({ capabilities: ["Reasoning"] }))).toBe(true);
  });

  it("returns false when model lacks Reasoning", () => {
    expect(supportsReasoning(mockModel({ capabilities: ["Vision"] }))).toBe(false);
  });

  it("returns false for null model", () => {
    expect(supportsReasoning(null)).toBe(false);
  });
});

describe("findModelByIds", () => {
  it("finds model by provider and model IDs", () => {
    const providers = [mockProvider()];
    const model = findModelByIds(providers, "p-1", "gpt-4");
    expect(model).not.toBeNull();
    expect(model!.model_id).toBe("gpt-4");
  });

  it("returns null for missing provider", () => {
    const providers = [mockProvider()];
    expect(findModelByIds(providers, "p-none", "gpt-4")).toBeNull();
  });

  it("returns null for missing model", () => {
    const providers = [mockProvider()];
    expect(findModelByIds(providers, "p-1", "nonexistent")).toBeNull();
  });

  it("returns null when providerId is null", () => {
    const providers = [mockProvider()];
    expect(findModelByIds(providers, null, "gpt-4")).toBeNull();
  });

  it("returns null when model_id is null", () => {
    const providers = [mockProvider()];
    expect(findModelByIds(providers, "p-1", null)).toBeNull();
  });

  it("finds model across multiple providers", () => {
    const providers = [
      mockProvider(),
      mockProvider({
        id: "p-2",
        name: "Anthropic",
        models: [mockModel({ model_id: "claude-3", provider_id: "p-2" })],
      }),
    ];
    const model = findModelByIds(providers, "p-2", "claude-3");
    expect(model).not.toBeNull();
    expect(model!.model_id).toBe("claude-3");
  });
});
