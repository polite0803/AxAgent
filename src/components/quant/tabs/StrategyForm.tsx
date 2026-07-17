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
    { key: "short_period", label: "quant.strategyParams.shortPeriod", default: 5, min: 2, max: 60, step: 1 },
    { key: "long_period", label: "quant.strategyParams.longPeriod", default: 20, min: 5, max: 250, step: 1 },
  ],
  [BUILTIN_STRATEGY_IDS.Macd]: [
    { key: "fast", label: "quant.strategyParams.fast", default: 12, min: 2, max: 60, step: 1 },
    { key: "slow", label: "quant.strategyParams.slow", default: 26, min: 5, max: 250, step: 1 },
    { key: "signal", label: "quant.strategyParams.signal", default: 9, min: 2, max: 60, step: 1 },
  ],
  [BUILTIN_STRATEGY_IDS.Rsi]: [
    { key: "period", label: "quant.strategyParams.period", default: 6, min: 2, max: 60, step: 1 },
    { key: "overbought", label: "quant.strategyParams.overbought", default: 70, min: 50, max: 95, step: 1 },
    { key: "oversold", label: "quant.strategyParams.oversold", default: 30, min: 5, max: 50, step: 1 },
  ],
  [BUILTIN_STRATEGY_IDS.Boll]: [
    { key: "period", label: "quant.strategyParams.period", default: 20, min: 5, max: 60, step: 1 },
    { key: "stddev", label: "quant.strategyParams.stddev", default: 2, min: 0.5, max: 5, step: 0.1 },
  ],
  [BUILTIN_STRATEGY_IDS.Turtle]: [
    { key: "entry_period", label: "quant.strategyParams.entryPeriod", default: 20, min: 5, max: 60, step: 1 },
    { key: "exit_period", label: "quant.strategyParams.exitPeriod", default: 10, min: 3, max: 60, step: 1 },
    { key: "atr_period", label: "quant.strategyParams.atrPeriod", default: 20, min: 5, max: 60, step: 1 },
    { key: "atr_multiplier", label: "quant.strategyParams.atrMultiplier", default: 2, min: 0.5, max: 5, step: 0.1 },
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

  // 修复 H9: 通过 Form key 强制重挂载同步 initialValues，避免 onChange 展开导致字段重置
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
    <Form key={strategy.id} layout="vertical" size="small" initialValues={initialValues}>
      {fields.map((f) => (
        <Form.Item key={f.key} label={t(f.label)} name={f.key}>
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
