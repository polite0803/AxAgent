// StrategyForm — 策略参数编辑表单
// 内置策略用字段,RhIA 策略不需

import { Form, Input, InputNumber } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { StrategyMeta } from "@/types";
import { BUILTIN_STRATEGY_IDS, DEFAULT_STRATEGY_PARAMS } from "@/types";

interface StrategyFormProps {
  strategy: StrategyMeta;
  onChange: (params: Record<string, number | string>) => void;
}

interface ParamField {
  key: string;
  label: string;
  default: number;
  min?: number;
  max?: number;
  step?: number;
}

const BUILTIN_FIELDS: Record<string, ParamField[]> = {
  [BUILTIN_STRATEGY_IDS.MaCross]: [
    { key: "short_period", label: "短周期", default: 5, min: 2, max: 60, step: 1 },
    { key: "long_period", label: "长周期", default: 20, min: 5, max: 250, step: 1 },
  ],
  [BUILTIN_STRATEGY_IDS.Macd]: [
    { key: "fast", label: "快线", default: 12, min: 2, max: 60, step: 1 },
    { key: "slow", label: "慢线", default: 26, min: 5, max: 250, step: 1 },
    { key: "signal", label: "信号线", default: 9, min: 2, max: 60, step: 1 },
  ],
  [BUILTIN_STRATEGY_IDS.Rsi]: [
    { key: "period", label: "周期", default: 6, min: 2, max: 60, step: 1 },
    { key: "overbought", label: "超买", default: 70, min: 50, max: 95, step: 1 },
    { key: "oversold", label: "超卖", default: 30, min: 5, max: 50, step: 1 },
  ],
  [BUILTIN_STRATEGY_IDS.Boll]: [
    { key: "period", label: "周期", default: 20, min: 5, max: 60, step: 1 },
    { key: "stddev", label: "标准差倍数", default: 2, min: 0.5, max: 5, step: 0.1 },
  ],
  [BUILTIN_STRATEGY_IDS.Turtle]: [
    { key: "entry_period", label: "入场周期", default: 20, min: 5, max: 60, step: 1 },
    { key: "exit_period", label: "离场周期", default: 10, min: 3, max: 60, step: 1 },
    { key: "atr_period", label: "ATR 周期", default: 20, min: 5, max: 60, step: 1 },
    { key: "atr_multiplier", label: "ATR 倍数", default: 2, min: 0.5, max: 5, step: 0.1 },
  ],
};

export function StrategyForm({ strategy, onChange }: StrategyFormProps) {
  const { t } = useTranslation();
  const fields = BUILTIN_FIELDS[strategy.id];

  const initialValues = useMemo(() => {
    if (fields) {
      const defaults = DEFAULT_STRATEGY_PARAMS[strategy.id] ?? {};
      const merged: Record<string, number> = {};
      for (const f of fields) {
        merged[f.key] = (strategy.params[f.key] as number) ?? defaults[f.key] ?? f.default;
      }
      return merged;
    }
    return strategy.params as Record<string, number>;
  }, [strategy, fields]);

  if (!fields) {
    // Rhai 策略：参数以 JSON 字符串形式编辑（M1 简化）
    return (
      <Form layout="vertical" size="small">
        <Form.Item label={t("quant.strategyForm.paramsJson")}>
          <Input.TextArea
            rows={4}
            defaultValue={JSON.stringify(strategy.params, null, 2)}
            onChange={(e) => {
              try {
                const v = JSON.parse(e.target.value) as Record<string, number>;
                onChange(v as Record<string, number | string>);
              } catch {
                /* ignore */
              }
            }}
          />
        </Form.Item>
      </Form>
    );
  }

  return (
    <Form layout="vertical" size="small" initialValues={initialValues}>
      {fields.map((f) => (
        <Form.Item key={f.key} label={f.label} name={f.key}>
          <InputNumber
            min={f.min}
            max={f.max}
            step={f.step}
            style={{ width: "100%" }}
            onChange={(v) => {
              if (typeof v === "number") {
                onChange({ ...initialValues, [f.key]: v });
              }
            }}
          />
        </Form.Item>
      ))}
    </Form>
  );
}
