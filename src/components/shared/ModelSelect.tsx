// SPDX-License-Identifier: AGPL-3.0-only

import { SmartProviderIcon } from "@/lib/providerIcons";
import { useProviderStore } from "@/stores";
import { ModelIcon } from "@lobehub/icons";
import { Select, theme } from "antd";
import { useCallback, useMemo } from "react";

/** Parse a combined `providerId::modelId` value. */
// eslint-disable-next-line react-refresh/only-export-components
export function parseModelValue(value: string | undefined) {
  if (!value) {
    return null;
  }
  const idx = value.indexOf("::");
  if (idx < 0) {
    return null;
  }
  return { providerId: value.slice(0, idx), modelId: value.slice(idx + 2) };
}

/** Hook: returns grouped Select options (Provider → Models) */
// eslint-disable-next-line react-refresh/only-export-components
export function useGroupedModelOptions() {
  const providers = useProviderStore((s) => s.providers);
  return useMemo(() => {
    return providers.flatMap((p) =>
      p.enabled && p.models.some((m) => m.enabled)
        ? [
          {
            label: (
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 6,
                }}
              >
                <SmartProviderIcon provider={p} size={16} type="avatar" />
                {p.name}
              </span>
            ),
            title: p.name,
            options: p.models.flatMap((m) =>
              m.enabled
                ? [
                  {
                    label: m.name,
                    value: `${p.id}::${m.modelId}`,
                    modelId: m.modelId,
                    providerName: p.name,
                  },
                ]
                : []
            ),
          },
        ]
        : []
    );
  }, [providers]);
}

/** Hook: returns Map<providerId, providerName> */
// eslint-disable-next-line react-refresh/only-export-components
export function useProviderNameMap() {
  const providers = useProviderStore((s) => s.providers);
  return useMemo(() => {
    const map = new Map<string, string>();
    providers.forEach((p) => map.set(p.id, p.name));
    return map;
  }, [providers]);
}

/**
 * Hook: 返回一个把 `${providerId}::${modelId}` 复合 ID 解析为
 * "供应商名 / 模型名" 友好展示的函数。
 *
 * 后端 `embeddingProvider` 字段统一存的是 `providerId::modelId` 复合值，
 * 直接渲染会暴露内部 ID。本 hook 复用 `useProviderNameMap` +
 * `useProviderStore` 把它转成人类可读的标签。
 */
// eslint-disable-next-line react-refresh/only-export-components
export function useEmbeddingProviderLabel(): (
  value: string | undefined | null,
) => string {
  const providerNameMap = useProviderNameMap();
  const providers = useProviderStore((s) => s.providers);

  return useMemo(() => {
    return (value: string | undefined | null): string => {
      if (!value) {
        return "";
      }
      const parsed = parseModelValue(value ?? undefined);
      if (!parsed) {
        // 不带 `::` 分隔符的旧数据，直接返回原值
        return value;
      }
      const providerName = providerNameMap.get(parsed.providerId)
        ?? providers.find((p) => p.id === parsed.providerId)?.name
        ?? parsed.providerId;
      // 模型名优先用 provider.models 里的 name（友好名），找不到就用 modelId
      const model = providers
        .find((p) => p.id === parsed.providerId)
        ?.models.find((m) => m.modelId === parsed.modelId);
      const modelLabel = model?.name ?? parsed.modelId;
      return `${providerName} / ${modelLabel}`;
    };
  }, [providerNameMap, providers]);
}

/**
 * Reusable model selector with provider-grouped options, ModelIcon rendering,
 * and search support. Value format: `providerId::modelId`.
 */
export function ModelSelect({
  value,
  onChange,
  placeholder,
  allowClear = true,
  style,
}: {
  value?: string;
  onChange: (value: string | undefined) => void;
  placeholder?: string;
  allowClear?: boolean;
  style?: React.CSSProperties;
}) {
  const { token } = theme.useToken();
  const groupedOptions = useGroupedModelOptions();
  const providerNameMap = useProviderNameMap();

  const optionRender = useCallback(
    (
      oriOption: { label?: React.ReactNode; value?: string | number },
      _info: { index: number },
    ) => {
      const modelId = String(oriOption.value ?? "").split("::")[1] ?? "";
      return (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <ModelIcon model={modelId} size={18} type="avatar" />
          {oriOption.label}
        </span>
      );
    },
    [],
  );

  const labelRender = useCallback(
    (props: { label?: React.ReactNode; value?: string | number }) => {
      const parsed = parseModelValue(String(props.value ?? ""));
      if (!parsed) {
        return <span>{props.label}</span>;
      }
      const providerName = providerNameMap.get(parsed.providerId) ?? "";
      return (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <ModelIcon model={parsed.modelId} size={18} type="avatar" />
          {props.label}
          <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
            ({providerName})
          </span>
        </span>
      );
    },
    [providerNameMap, token.colorTextSecondary],
  );

  return (
    <Select
      value={value}
      onChange={onChange}
      placeholder={placeholder}
      allowClear={allowClear}
      showSearch
      optionFilterProp="label"
      optionRender={optionRender}
      labelRender={labelRender}
      options={groupedOptions}
      style={style}
    />
  );
}
