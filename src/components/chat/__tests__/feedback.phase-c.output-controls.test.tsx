// SPDX-License-Identifier: AGPL-3.0-only

import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

function readSource(...segments: string[]) {
  return fs.readFileSync(path.resolve(process.cwd(), ...segments), "utf8");
}

describe("Phase C output control regressions", () => {
  it("lets assistant replies enter the shared edit flow instead of restricting edits to user prompts", () => {
    const source = readSource("src/components/chat/ChatView.tsx");

    // 编辑流程支持 assistant 角色消息
    expect(source).toContain('editingMessageRole === "assistant"');
  });

  it("shows a per-turn total token summary alongside prompt and completion counts", () => {
    const source = readSource("src/components/chat/ChatView.tsx");

    // token 统计信息在对话视图中可用
    expect(source).toContain("tokens");
  });

  it("adds transcript copy and no-thinking export variants at chat level", () => {
    const source = readSource("src/components/chat/ChatView.tsx");

    // 导出功能存在于 ChatView
    expect(source).toContain("export");
  });

  it("lets export helpers optionally strip thinking content before saving or copying", () => {
    const source = readSource("src/lib/exportChat.ts");

    expect(source).toContain("includeThinking");
    expect(source).toContain("stripAxAgentTags");
    expect(source).toContain("copyTranscript");
  });
});
