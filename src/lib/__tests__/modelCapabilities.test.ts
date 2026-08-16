// SPDX-License-Identifier: AGPL-3.0-only

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
  modelId: "gpt-4",
  name: "GPT-4",
  modelType: "Chat",
  providerId: "p-1",
  capabilities: ["Vision", "FunctionCalling", "Reasoning"],
  maxTokens: 4096,
  enabled: true,
  paramOverrides: null,
  ...overrides,
});

const mockProvider = (
  overrides: Partial<ProviderConfig> = {},
): ProviderConfig => ({
  id: "p-1",
  name: "OpenAI",
  providerType: "openai",
  apiHost: "https://api.openai.com",
  apiPath: null,
  enabled: true,
  models: [
    mockModel(),
    mockModel({ modelId: "gpt-3.5", capabilities: ["FunctionCalling"] }),
  ],
  keys: [],
  proxyConfig: null,
  toolAdaptation: null,
  toolAdaptationMarkerPrefix: null,
  customHeaders: null,
  icon: null,
  builtinId: null,
  sortOrder: 0,
  createdAt: 0,
  updatedAt: 0,
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
    expect(getEditableCapabilities(null)).toHaveLength(4);
    expect(getEditableCapabilities(undefined)).toHaveLength(4);
  });
});

describe("sanitizeModelCapabilities", () => {
  it("filters out capabilities not in the allowed set", () => {
    const result = sanitizeModelCapabilities("Chat", [
      "Vision",
      "Unknown" as any,
    ]);
    expect(result).toEqual(["Vision"]);
  });

  it("returns empty when modelType is non-Chat", () => {
    const result = sanitizeModelCapabilities("Embedding", ["Vision"]);
    expect(result).toEqual([]);
  });

  it("keeps all valid capabilities", () => {
    const result = sanitizeModelCapabilities("Chat", [
      "Vision",
      "FunctionCalling",
      "Reasoning",
    ]);
    expect(result).toHaveLength(3);
  });
});

describe("getVisibleModelCapabilities", () => {
  it("returns sanitized capabilities for a chat model", () => {
    const model = mockModel({
      modelType: "Chat",
      capabilities: ["Vision", "Reasoning"],
    });
    const result = getVisibleModelCapabilities(model);
    expect(result).toEqual(["Vision", "Reasoning"]);
  });

  it("returns empty for a non-Chat model", () => {
    const model = mockModel({
      modelType: "Embedding",
      capabilities: ["Vision"],
    });
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
    expect(supportsReasoning(mockModel({ capabilities: ["Reasoning"] }))).toBe(
      true,
    );
  });

  it("returns false when model lacks Reasoning", () => {
    expect(supportsReasoning(mockModel({ capabilities: ["Vision"] }))).toBe(
      false,
    );
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
    expect(model!.modelId).toBe("gpt-4");
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
        models: [mockModel({ modelId: "claude-3", providerId: "p-2" })],
      }),
    ];
    const model = findModelByIds(providers, "p-2", "claude-3");
    expect(model).not.toBeNull();
    expect(model!.modelId).toBe("claude-3");
  });
});
