import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

function readSource(...segments: string[]) {
  return fs.readFileSync(path.resolve(process.cwd(), ...segments), 'utf8');
}

describe('Phase B category template regressions', () => {
  it('extends category contracts with default model and parameter template fields', () => {
    const typeSource = readSource('src/types/index.ts');
    const rustTypeSource = readSource('src-tauri/crates/core/src/types.rs');
    const entitySource = readSource('src-tauri/crates/core/src/entity/conversation_categories.rs');

    expect(typeSource).toMatch(/default_provider_id: string \| null;/);
    expect(typeSource).toMatch(/default_model_id: string \| null;/);
    expect(typeSource).toMatch(/default_temperature: number \| null;/);
    expect(typeSource).toMatch(/default_max_tokens: number \| null;/);
    expect(typeSource).toMatch(/default_top_p: number \| null;/);
    expect(typeSource).toMatch(/default_frequency_penalty: number \| null;/);

    expect(rustTypeSource).toMatch(/pub default_provider_id: Option<String>/);
    expect(rustTypeSource).toMatch(/pub default_model_id: Option<String>/);
    expect(rustTypeSource).toMatch(/pub default_temperature: Option<f64>/);
    expect(rustTypeSource).toMatch(/pub default_max_tokens: Option<i64>/);
    expect(rustTypeSource).toMatch(/pub default_top_p: Option<f64>/);
    expect(rustTypeSource).toMatch(/pub default_frequency_penalty: Option<f64>/);

    expect(entitySource).toMatch(/pub default_provider_id: Option<String>/);
    expect(entitySource).toMatch(/pub default_model_id: Option<String>/);
  });

  it('lets the category editor configure a default model plus model params', () => {
    const modalSource = readSource('src/components/chat/CategoryEditModal.tsx');

    expect(modalSource).toContain('ModelSelect');
    expect(modalSource).toContain('ModelParamSliders');
    expect(modalSource).toContain('default_provider_id');
    expect(modalSource).toContain('default_model_id');
    expect(modalSource).toContain('default_temperature');
    expect(modalSource).toContain('default_max_tokens');
    expect(modalSource).toContain('default_top_p');
    expect(modalSource).toContain('default_frequency_penalty');
  });

  it('adds a category-scoped new conversation action instead of forcing users to create then move', () => {
    const sidebarSource = readSource('src/components/chat/ChatSidebar.tsx');

    expect(sidebarSource).toContain('onCreateConversation');
    expect(sidebarSource).toContain("key: 'new'");
    expect(sidebarSource).toContain('handleNewConversation(cat.id)');
  });
});
