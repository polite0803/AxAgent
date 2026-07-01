// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Checkbox, DatePicker, Form, Input, InputNumber, Radio, Select, Switch } from "antd";
import type { CheckboxOptionType, RadioGroupProps } from "antd";
import dayjs from "dayjs";
import { useTranslation } from "react-i18next";
import { useInFormContext } from "./FormRenderer";

export const InputField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { t } = useTranslation();
  const {
    name,
    label,
    placeholder,
    type = "text",
    required,
    rules,
    disabled,
    rows,
    maxLength,
  } = schema.props as {
    name?: string;
    label?: string;
    placeholder?: string;
    type?: "text" | "password" | "textarea";
    required?: boolean;
    rules?: Array<{ required?: boolean; message?: string }>;
    disabled?: boolean;
    rows?: number;
    maxLength?: number;
  };

  const inForm = useInFormContext();

  const inputProps = {
    placeholder: placeholder || (label ? t("dynamicUI.pleaseEnter", { label }) : undefined),
    disabled,
    maxLength,
    style: schema.style as React.CSSProperties,
  };

  const inputEl = type === "textarea"
    ? <Input.TextArea rows={rows || 4} {...inputProps} />
    : <Input type={type} {...inputProps} />;

  if (inForm && name) {
    const formRules = [
      ...(rules || []),
      ...(required
        ? [{ required: true, message: t("dynamicUI.fieldRequired", { field: label || name }) }]
        : []),
    ];
    return (
      <FormFieldWrapper name={name} label={label} rules={formRules}>
        {inputEl}
      </FormFieldWrapper>
    );
  }

  return inputEl;
};

export const SelectField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { t } = useTranslation();
  const {
    name,
    label,
    placeholder,
    options,
    required,
    rules,
    disabled,
    mode,
  } = schema.props as {
    name?: string;
    label?: string;
    placeholder?: string;
    options?: Array<{ label: string; value: string | number }>;
    required?: boolean;
    rules?: Array<{ required?: boolean; message?: string }>;
    disabled?: boolean;
    mode?: "multiple" | "tags";
  };

  const inForm = useInFormContext();

  const selectEl = (
    <Select
      placeholder={placeholder || t("dynamicUI.pleaseSelect", { label: label || "" })}
      options={options}
      disabled={disabled}
      mode={mode}
      style={{ width: "100%", ...(schema.style as React.CSSProperties) }}
    />
  );

  if (inForm && name) {
    const formRules = [
      ...(rules || []),
      ...(required
        ? [{ required: true, message: t("dynamicUI.pleaseSelect", { label: label || name }) }]
        : []),
    ];
    return (
      <FormFieldWrapper name={name} label={label} rules={formRules}>
        {selectEl}
      </FormFieldWrapper>
    );
  }

  return selectEl;
};

export const NumberField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { t } = useTranslation();
  const {
    name,
    label,
    placeholder,
    min,
    max,
    step,
    required,
    rules,
    disabled,
  } = schema.props as {
    name?: string;
    label?: string;
    placeholder?: string;
    min?: number;
    max?: number;
    step?: number;
    required?: boolean;
    rules?: Array<{ required?: boolean; message?: string }>;
    disabled?: boolean;
  };

  const inForm = useInFormContext();

  const numberEl = (
    <InputNumber
      placeholder={placeholder || (label ? t("dynamicUI.pleaseEnter", { label }) : undefined)}
      min={min}
      max={max}
      step={step}
      disabled={disabled}
      style={{ width: "100%", ...(schema.style as React.CSSProperties) }}
    />
  );

  if (inForm && name) {
    const formRules = [
      ...(rules || []),
      ...(required
        ? [{ required: true, message: t("dynamicUI.fieldRequired", { field: label || name }) }]
        : []),
    ];
    return (
      <FormFieldWrapper name={name} label={label} rules={formRules}>
        {numberEl}
      </FormFieldWrapper>
    );
  }

  return numberEl;
};

export const SwitchField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { name, label, defaultChecked, disabled } = schema.props as {
    name?: string;
    label?: string;
    defaultChecked?: boolean;
    disabled?: boolean;
  };

  const inForm = useInFormContext();

  const switchEl = (
    <Switch
      defaultChecked={defaultChecked}
      disabled={disabled}
      style={schema.style as React.CSSProperties}
    />
  );

  if (inForm && name) {
    return (
      <FormFieldWrapper name={name} label={label} valuePropName="checked">
        {switchEl}
      </FormFieldWrapper>
    );
  }

  return (
    <div className="flex items-center gap-2">
      {label ? <span>{label}:</span> : null}
      {switchEl}
    </div>
  );
};

export const CheckboxField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { name, label, options, disabled } = schema.props as {
    name?: string;
    label?: string;
    options?: CheckboxOptionType[];
    disabled?: boolean;
  };

  const inForm = useInFormContext();

  const checkboxEl = options
    ? (
      <Checkbox.Group
        options={options}
        disabled={disabled}
        style={schema.style as React.CSSProperties}
      />
    )
    : <Checkbox disabled={disabled} style={schema.style as React.CSSProperties}>{label}</Checkbox>;

  if (inForm && name) {
    return (
      <FormFieldWrapper name={name} label={options ? label : undefined} valuePropName="checked">
        {checkboxEl}
      </FormFieldWrapper>
    );
  }

  return checkboxEl;
};

export const RadioField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { name, label, options, disabled } = schema.props as {
    name?: string;
    label?: string;
    options?: RadioGroupProps["options"];
    disabled?: boolean;
  };

  const inForm = useInFormContext();

  const radioEl = (
    <Radio.Group
      options={options}
      disabled={disabled}
      style={schema.style as React.CSSProperties}
    />
  );

  if (inForm && name) {
    return (
      <FormFieldWrapper name={name} label={label}>
        {radioEl}
      </FormFieldWrapper>
    );
  }

  return (
    <div>
      {label ? <div className="mb-1">{label}</div> : null}
      {radioEl}
    </div>
  );
};

export const DatePickerField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { t } = useTranslation();
  const {
    name,
    label,
    placeholder,
    required,
    rules,
    disabled,
    format = "YYYY-MM-DD",
    showTime,
  } = schema.props as {
    name?: string;
    label?: string;
    placeholder?: string;
    required?: boolean;
    rules?: Array<{ required?: boolean; message?: string }>;
    disabled?: boolean;
    format?: string;
    showTime?: boolean;
  };

  const inForm = useInFormContext();

  const pickerEl = (
    <DatePicker
      placeholder={placeholder || (label ? t("dynamicUI.pleaseSelect", { label }) : undefined)}
      disabled={disabled}
      format={format}
      showTime={showTime}
      style={{ width: "100%", ...(schema.style as React.CSSProperties) }}
    />
  );

  if (inForm && name) {
    const formRules = [
      ...(rules || []),
      ...(required
        ? [{ required: true, message: t("dynamicUI.fieldRequired", { field: label || name }) }]
        : []),
    ];
    const getValueProps = (val: unknown) => {
      if (!val) {
        return { value: undefined };
      }
      if (dayjs.isDayjs(val)) {
        return { value: val };
      }
      if (typeof val === "string") {
        const d = dayjs(val);
        return { value: d.isValid() ? d : undefined };
      }
      return { value: undefined };
    };
    return (
      <Form.Item
        name={name}
        label={label}
        rules={formRules}
        getValueProps={getValueProps}
        getValueFromEvent={(v) => (dayjs.isDayjs(v) ? v.format(format) : null)}
      >
        {pickerEl}
      </Form.Item>
    );
  }

  return pickerEl;
};

function FormFieldWrapper({
  name,
  label,
  rules,
  valuePropName,
  children,
}: {
  name: string;
  label?: string;
  rules?: Array<{ required?: boolean; message?: string }>;
  valuePropName?: string;
  children: React.ReactNode;
}) {
  return (
    <Form.Item
      name={name}
      label={label}
      rules={rules}
      valuePropName={valuePropName}
    >
      {children}
    </Form.Item>
  );
}
