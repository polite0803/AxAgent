import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

function readSource(...segments: string[]) {
  return fs.readFileSync(path.resolve(process.cwd(), ...segments), 'utf8');
}

describe('Phase D reasoning control regressions', () => {
  it('preserves explicit zero thinking budgets for Gemini requests so the provider sees a disable signal', () => {
    const source = readSource('src-tauri/crates/providers/src/gemini.rs');

    expect(source).toContain('request.thinking_budget.map(|b| GeminiThinkingConfig { thinking_budget: b })');
  });

  it('suppresses returned thinking blocks when the user explicitly disables reasoning', () => {
    const source = readSource('src-tauri/src/commands/conversations.rs');

    expect(source).toContain('let suppress_thinking = thinking_budget == Some(0);');
    expect(source).toContain('strip_disabled_thinking_delta');
    expect(source).toContain('strip_disabled_thinking_content');
    expect(source).toContain('suppress_thinking,');
  });
});
