// SPDX-License-Identifier: AGPL-3.0-only

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "antd";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ProviderDetail } from "../ProviderDetail";

const toggleProvider = vi.fn();
const updateProvider = vi.fn();
const deleteProvider = vi.fn();
const addProviderKey = vi.fn();
const deleteProviderKey = vi.fn();
const toggleProviderKey = vi.fn();
const validateProviderKey = vi.fn();
const toggleModel = vi.fn();
const updateModelParams = vi.fn();
const fetchRemoteModels = vi.fn();
const saveModels = vi.fn();

let provider = {
  id: "provider-1",
  name: "OpenAI",
  provider_type: "openai",
  api_host: "https://api.openai.com",
  api_path: "/v1/chat/completions",
  enabled: true,
  models: [
    {
      provider_id: "provider-1",
      model_id: "gpt-5.4",
      name: "GPT 5.4",
      group_name: "gpt-5.4",
      model_type: "Chat",
      capabilities: ["TextChat"],
      max_tokens: null,
      enabled: true,
      param_overrides: null,
    },
  ],
  keys: [],
  proxy_config: null,
  sort_order: 0,
  created_at: 0,
  updated_at: 0,
};

const zh: Record<string, string> = {
  "settings.addModel": "添加模型",
  "settings.addModelToGroup": "添加到当前分组",
  "settings.model_id": "模型标识",
  "settings.modelName": "模型名称",
  "settings.modelGroup": "模型分组",
  "settings.modelType.title": "模型类型",
  "common.confirm": "确认",
  "common.cancel": "取消",
  "common.enabled": "已启用",
  "common.disabled": "已禁用",
  "common.noData": "暂无数据",
  "common.copySuccess": "复制成功",
  "common.collapseAll": "全部收起",
  "common.expandAll": "全部展开",
  "common.errorDetail": "错误详情",
  "common.failed": "失败",
  "error.saveFailed": "保存失败",
  "error.loadFailed": "加载失败",
  "error.unknown": "未知错误",
  "error.keyValidationFailed": "密钥验证失败",
};

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      if (zh[key]) { return zh[key]; }
      if (typeof options === "string") { return options; }
      return key;
    },
    i18n: { language: "zh" },
  }),
}));

vi.mock("@lobehub/icons", () => ({
  ProviderIcon: () => <div>provider-icon</div>,
  ModelIcon: () => <div>model-icon</div>,
  providerMappings: [],
  modelMappings: [],
}));

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count }: { count: number }) => ({
    getTotalSize: () => count * 40,
    getVirtualItems: () =>
      Array.from({ length: count }, (_, i) => ({
        index: i,
        key: `virtual-${i}`,
        start: i * 40,
        size: 40,
      })),
    measureElement: () => {},
  }),
}));

vi.mock("../IconPickerModal", () => ({
  IconPickerModal: () => null,
  default: () => null,
}));

const setSelectedProviderId = vi.fn();

vi.mock("@/stores", () => ({
  useProviderStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      providers: [provider],
      toggleProvider,
      updateProvider,
      deleteProvider,
      addProviderKey,
      deleteProviderKey,
      toggleProviderKey,
      validateProviderKey,
      toggleModel,
      updateModelParams,
      fetchRemoteModels,
      saveModels,
    }),
  useUIStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      setSelectedProviderId,
    }),
  useSettingsStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      settings: { theme_mode: "light" },
      saveSettings: vi.fn(),
    }),
}));

describe("ProviderDetail", () => {
  vi.setConfig({ testTimeout: 20000 });
  beforeEach(() => {
    vi.clearAllMocks();
    provider = {
      id: "provider-1",
      name: "OpenAI",
      provider_type: "openai",
      api_host: "https://api.openai.com",
      api_path: "/v1/chat/completions",
      enabled: true,
      models: [
        {
          provider_id: "provider-1",
          model_id: "gpt-5.4",
          name: "GPT 5.4",
          group_name: "gpt-5.4",
          model_type: "Chat",
          capabilities: ["TextChat"],
          max_tokens: null,
          enabled: true,
          param_overrides: null,
        },
      ],
      keys: [],
      proxy_config: null,
      sort_order: 0,
      created_at: 0,
      updated_at: 0,
    };
    saveModels.mockResolvedValue(undefined);

    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
  });

  it("adds a model from the card-level action and derives the default group from the model id", async () => {
    render(
      <App>
        <ProviderDetail providerId="provider-1" />
      </App>,
    );

    await userEvent.click(screen.getByRole("button", { name: "添加模型" }));

    const dialog = await screen.findByRole("dialog");
    const inputs = within(dialog).getAllByRole("textbox");
    await userEvent.type(inputs[0], "gpt-5.4-think");
    await userEvent.clear(inputs[1]);
    await userEvent.type(inputs[1], "GPT 5.4 Think");

    await userEvent.click(
      within(dialog).getByRole("button", { name: "添加模型" }),
    );

    expect(saveModels).toHaveBeenCalledWith(
      "provider-1",
      expect.arrayContaining([
        expect.objectContaining({
          model_id: "gpt-5.4-think",
          name: "GPT 5.4 Think",
          group_name: "gpt-5.4",
          model_type: "Chat",
        }),
      ]),
    );
  });

  it("prefills the current group when adding a model from a group header", async () => {
    render(
      <App>
        <ProviderDetail providerId="provider-1" />
      </App>,
    );

    await userEvent.click(
      screen.getByRole("button", { name: "添加到当前分组" }),
    );

    const dialog = await screen.findByRole("dialog");
    const comboboxes = within(dialog).getAllByRole("combobox");
    expect(comboboxes[0]).toHaveValue("gpt-5.4");
  });
});
