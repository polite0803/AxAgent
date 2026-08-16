// SPDX-License-Identifier: AGPL-3.0-only

import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

function readSource(...segments: string[]) {
  return fs.readFileSync(path.resolve(process.cwd(), ...segments), "utf8");
}

describe("Phase B category template regressions", () => {
  it("extends category contracts with default model and parameter template fields", () => {
    const typeSource = readSource("src/types/index.ts");
    const rustTypeSource = readSource(
      "src-tauri/crates/harness/src/types/settings_chat.rs",
    );
    const entitySource = readSource(
      "src-tauri/crates/entities/src/conversation_categories.rs",
    );

    expect(typeSource).toMatch(/defaultProviderId: string \| null;/);
    expect(typeSource).toMatch(/defaultModelId: string \| null;/);
    expect(typeSource).toMatch(/defaultTemperature: number \| null;/);
    expect(typeSource).toMatch(/defaultMaxTokens: number \| null;/);
    expect(typeSource).toMatch(/defaultTopP: number \| null;/);
    expect(typeSource).toMatch(/defaultFrequencyPenalty: number \| null;/);

    expect(rustTypeSource).toMatch(/pub default_provider_id: Option<String>/);
    expect(rustTypeSource).toMatch(/pub default_model_id: Option<String>/);
    expect(rustTypeSource).toMatch(/pub default_temperature: Option<f32>/);
    expect(rustTypeSource).toMatch(/pub default_max_tokens: Option<u32>/);
    expect(rustTypeSource).toMatch(/pub default_top_p: Option<f32>/);
    expect(rustTypeSource).toMatch(
      /pub default_frequency_penalty: Option<f32>/,
    );

    expect(entitySource).toMatch(/pub default_provider_id: Option<String>/);
    expect(entitySource).toMatch(/pub default_model_id: Option<String>/);
  });

  it("lets the category editor configure a default model plus model params", () => {
    const modalSource = readSource("src/components/chat/CategoryEditModal.tsx");

    expect(modalSource).toContain("ModelSelect");
    expect(modalSource).toContain("ModelParamSliders");
    expect(modalSource).toContain("defaultProviderId");
    expect(modalSource).toContain("defaultModelId");
    expect(modalSource).toContain("defaultTemperature");
    expect(modalSource).toContain("defaultMaxTokens");
    expect(modalSource).toContain("defaultTopP");
    expect(modalSource).toContain("defaultFrequencyPenalty");
  });

  it("provides a new conversation action scoped to workspace groups", () => {
    const sidebarSource = readSource("src/components/chat/ChatSidebar.tsx");

    expect(sidebarSource).toContain("handleNewConversation");
    expect(sidebarSource).toContain("new-conversation-btn");
    expect(sidebarSource).toContain("MessageSquarePlus");
  });
});
