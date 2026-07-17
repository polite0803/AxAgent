// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

const mockRegisterBatch = vi.fn();

vi.mock("../dynamicUI/ComponentRegistry", () => ({
  componentRegistry: {
    registerBatch: mockRegisterBatch,
  },
}));

vi.mock("@/components/dynamicUI/containers/AccordionContainer", () => ({ AccordionContainer: {} }));
vi.mock("@/components/dynamicUI/containers/CardContainer", () => ({ CardContainer: {} }));
vi.mock("@/components/dynamicUI/containers/ColumnContainer", () => ({ ColumnContainer: {} }));
vi.mock("@/components/dynamicUI/containers/Container", () => ({ Container: {} }));
vi.mock("@/components/dynamicUI/containers/GridContainer", () => ({ GridContainer: {} }));
vi.mock("@/components/dynamicUI/containers/RowContainer", () => ({ RowContainer: {} }));
vi.mock("@/components/dynamicUI/containers/TabsContainer", () => ({ TabsContainer: {} }));
vi.mock("@/components/dynamicUI/data/ChartRenderer", () => ({ ChartRenderer: {} }));
vi.mock("@/components/dynamicUI/data/Dashboard", () => ({ Dashboard: {} }));
vi.mock("@/components/dynamicUI/data/DataTable", () => ({ DataTable: {} }));
vi.mock("@/components/dynamicUI/data/ListView", () => ({ ListView: {} }));
vi.mock("@/components/dynamicUI/data/TimelineView", () => ({ TimelineView: {} }));
vi.mock("@/components/dynamicUI/data/TreeView", () => ({ TreeView: {} }));
vi.mock("@/components/dynamicUI/form/FormFields", () => ({
  CheckboxField: {},
  DatePickerField: {},
  InputField: {},
  NumberField: {},
  RadioField: {},
  SelectField: {},
  SwitchField: {},
}));
vi.mock("@/components/dynamicUI/form/FormRenderer", () => ({ FormRenderer: {} }));
vi.mock("@/components/dynamicUI/media/CodeEditorView", () => ({ CodeEditorView: {} }));
vi.mock("@/components/dynamicUI/media/FilePreviewView", () => ({ FilePreviewView: {} }));
vi.mock("@/components/dynamicUI/media/MarkdownView", () => ({ MarkdownView: {} }));
vi.mock("@/components/dynamicUI/misc/MiscComponents", () => ({
  DynamicButton: {},
  DynamicDivider: {},
  DynamicImage: {},
  DynamicProgress: {},
  DynamicTag: {},
  DynamicText: {},
}));

describe("registerBuiltins", () => {
  it("registerAllBuiltins 应调用 componentRegistry.registerBatch", async () => {
    const { registerAllBuiltins } = await import("../dynamicUI/registerBuiltins");
    registerAllBuiltins();
    expect(mockRegisterBatch).toHaveBeenCalledTimes(1);
  }, 15000);

  it("应注册至少 20 个组件", async () => {
    const { registerAllBuiltins } = await import("../dynamicUI/registerBuiltins");
    mockRegisterBatch.mockClear();
    registerAllBuiltins();
    const entries = mockRegisterBatch.mock.calls[0][0];
    expect(entries.length).toBeGreaterThanOrEqual(20);
  });

  it("注册的组件应包含 category 字段", async () => {
    const { registerAllBuiltins } = await import("../dynamicUI/registerBuiltins");
    mockRegisterBatch.mockClear();
    registerAllBuiltins();
    const entries = mockRegisterBatch.mock.calls[0][0];
    for (const entry of entries) {
      expect(entry).toHaveProperty("category");
      expect(entry).toHaveProperty("type");
      expect(entry).toHaveProperty("label");
    }
  });

  it("应包含 container 分类的组件", async () => {
    const { registerAllBuiltins } = await import("../dynamicUI/registerBuiltins");
    mockRegisterBatch.mockClear();
    registerAllBuiltins();
    const entries = mockRegisterBatch.mock.calls[0][0];
    const containers = entries.filter((e: { category: string }) => e.category === "container");
    expect(containers.length).toBeGreaterThanOrEqual(5);
  });

  it("应包含 form 分类的组件", async () => {
    const { registerAllBuiltins } = await import("../dynamicUI/registerBuiltins");
    mockRegisterBatch.mockClear();
    registerAllBuiltins();
    const entries = mockRegisterBatch.mock.calls[0][0];
    const forms = entries.filter((e: { category: string }) => e.category === "form");
    expect(forms.length).toBeGreaterThanOrEqual(5);
  });

  it("应包含 data-display 分类的组件", async () => {
    const { registerAllBuiltins } = await import("../dynamicUI/registerBuiltins");
    mockRegisterBatch.mockClear();
    registerAllBuiltins();
    const entries = mockRegisterBatch.mock.calls[0][0];
    const dataDisplay = entries.filter((e: { category: string }) => e.category === "data-display");
    expect(dataDisplay.length).toBeGreaterThanOrEqual(3);
  });
});
