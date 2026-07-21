import { Button, DatePicker, Space } from "antd";
import dayjs, { Dayjs } from "dayjs";
import { useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * AsOfDatePicker — as_of_date 选择器
 *
 * 约束（与后端 AsOfContext::new 一致）：
 *   - 不允许选择未来日期（disabledDate 阻止）
 *   - 不允许选择今天（"今天"未收盘的数据在严格模式下不能算封闭世界）
 *   - 不允许选择空值
 *
 * 提交时把 DatePicker 值转 YYYY-MM-DD 字符串传给 onPick。
 */
export interface AsOfDatePickerProps {
  onPick: (date: string) => void;
  onCancel?: () => void;
}

export function AsOfDatePicker({ onPick, onCancel }: AsOfDatePickerProps) {
  const { t } = useTranslation();
  const [val, setVal] = useState<Dayjs | null>(null);

  const today = dayjs();
  // 严格：不可选 >= today（必须昨天或更早）
  const disabledDate = (d: Dayjs) => d.isSame(today) || d.isAfter(today);

  const ok = () => {
    if (!val) { return; }
    onPick(val.format("YYYY-MM-DD"));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, minWidth: 240 }}>
      <div style={{ fontSize: 12, color: "var(--ax-text-secondary, #6b7280)" }}>
        {t("timeTravel.datePicker.hint")}
      </div>
      <DatePicker
        value={val}
        onChange={(d) => setVal(d)}
        disabledDate={disabledDate}
        format="YYYY-MM-DD"
        allowClear={false}
        placeholder={t("timeTravel.datePicker.placeholder")}
        style={{ width: "100%" }}
        data-testid="asof-date-picker"
      />
      <Space>
        <Button type="primary" onClick={ok} disabled={!val} size="small">
          {t("timeTravel.datePicker.ok")}
        </Button>
        {onCancel && (
          <Button onClick={onCancel} size="small">
            {t("timeTravel.datePicker.cancel")}
          </Button>
        )}
      </Space>
    </div>
  );
}
