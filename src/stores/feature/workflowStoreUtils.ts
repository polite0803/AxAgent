// SPDX-License-Identifier: AGPL-3.0-only

/** Dev-only mock module type */
type MockModule = typeof import("./__mocks__/workflowStoreMocks");
let _mockModule: MockModule | null = null;

export async function getMocks(): Promise<MockModule | null> {
  if (import.meta.env.PROD) { return null; }
  if (!_mockModule) {
    _mockModule = await import("./__mocks__/workflowStoreMocks");
  }
  return _mockModule;
}

export function makeId(): string {
  return `wf_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}
