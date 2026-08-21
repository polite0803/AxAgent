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
  const providerId = value.slice(0, idx);
  const modelId = value.slice(idx + 2);
  // 防御：解析出的 ID 不能是 undefined/null/空字符串字面量
  // 这是为了防止后端传来或中间件生成的脏数据污染前端
  if (
    !providerId
    || !modelId
    || providerId === "undefined"
    || providerId === "null"
    || modelId === "undefined"
    || modelId === "null"
  ) {
    console.warn("[parseModelValue] 检测到无效值", { value, providerId, modelId });
    return null;
  }
  return { providerId, modelId };
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
            options: p.models.flatMap((m) => {
              // 防御：过滤无效的 modelId，防止 "undefined"/"null" 字符串污染
              if (
                !m.modelId
                || m.modelId === "undefined"
                || m.modelId === "null"
                || m.modelId.trim() === ""
              ) {
                console.warn(
                  `[ModelSelect] 跳过无效模型选项: provider=${p.name}, modelId=${String(m.modelId)}`,
                );
                return [];
              }
              return m.enabled
                ? [
                  {
                    label: m.name,
                    value: `${p.id}::${m.modelId}`,
                    modelId: m.modelId,
                    providerName: p.name,
                  },
                ]
                : [];
            }),
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
 *
 * Uses `labelInValue` internally to bypass Ant Design's internal
 * value-to-option mapping, which has issues with grouped options in v6.
 * The external API remains a simple string value.
 *
 * 修复策略：使用 key 属性强制 Select 在 value 或 options 变化时
 * 完全重新创建，避免 Ant Design v6 在 grouped options 下
 * 内部状态与外部 value 不同步的 bug。
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

  // Build a flat map of value → label for reliable lookup
  const valueToLabelMap = useMemo(() => {
    const map = new Map<string, string>();
    groupedOptions.forEach((group) => {
      group.options?.forEach((opt) => {
        if (typeof opt.value === "string") {
          map.set(opt.value, String(opt.label ?? opt.value));
        }
      });
    });
    return map;
  }, [groupedOptions]);

  // Convert external string value to labelInValue format { value, label }
  const internalValue = useMemo(() => {
    if (!value) {
      return undefined;
    }
    const label = valueToLabelMap.get(value);
    if (label === undefined) {
      return undefined;
    }
    return { value, label };
  }, [value, valueToLabelMap]);

  // 关键修复：计算 options 指纹，用于 key 属性
  // 当 options 内容变化时，强制 Select 重新创建，避免内部状态不同步
  const optionsFingerprint = useMemo(() => {
    return groupedOptions
      .map(
        (g) =>
          `${g.options?.length ?? 0}:${
            g.options
              ?.map((o) => String(o.value))
              .join("|") ?? ""
          }`,
      )
      .join("||");
  }, [groupedOptions]);

  // Select 的 key：仅在 options 变化时强制重新创建（不在 value 变化时）
  // 避免每次选择都闪烁
  const selectKey = useMemo(() => {
    return `model-select__${optionsFingerprint}`;
  }, [optionsFingerprint]);

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
      const valueStr = String(props.value ?? "");
      // 关键修复：使用 valueToLabelMap 自己查找正确的 label
      // 不依赖 Ant Design 传入的 props.label，避免 grouped options + labelInValue 下的匹配 bug
      const correctLabel = valueToLabelMap.get(valueStr) ?? String(props.label ?? "");
      const parsed = parseModelValue(valueStr);
      if (!parsed) {
        return <span>{correctLabel}</span>;
      }
      const providerName = providerNameMap.get(parsed.providerId) ?? "";
      return (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <ModelIcon model={parsed.modelId} size={18} type="avatar" />
          {correctLabel}
          <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
            ({providerName})
          </span>
        </span>
      );
    },
    [valueToLabelMap, providerNameMap, token.colorTextSecondary],
  );

  // Convert onChange back to simple string for external API
  const handleChange = useCallback(
    (newValue: unknown) => {
      if (newValue === undefined || newValue === null) {
        onChange(undefined);
        return;
      }
      if (typeof newValue === "string") {
        onChange(newValue);
      } else if (
        typeof newValue === "object"
        && newValue !== null
        && "value" in newValue
      ) {
        onChange(String((newValue as { value: string }).value));
      }
    },
    [onChange],
  );

  return (
    <Select
      key={selectKey}
      value={internalValue}
      onChange={handleChange}
      placeholder={placeholder}
      allowClear={allowClear}
      showSearch
      optionFilterProp="label"
      optionRender={optionRender}
      labelRender={labelRender}
      options={groupedOptions}
      labelInValue
      style={style}
    />
  );
}
