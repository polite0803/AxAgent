// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { DatePicker, Input, InputNumber, Select, Switch } from "antd";

/**
 * 输入框组件，基于 Ant Design Input。
 */
export const InputField: React.FC<DynamicUIProps> = ({ schema }) => {
  const {
    label,
    name,
    placeholder,
    required,
    disabled,
    type = "text",
    ...rest
  } = schema.props as {
    label?: string;
    name?: string;
    placeholder?: string;
    required?: boolean;
    disabled?: boolean;
    type?: string;
    [key: string]: unknown;
  };

  // 数字输入
  if (type === "number") {
    return (
      <FormFieldWrapper label={label} name={name} required={required}>
        <InputNumber
          placeholder={placeholder}
          disabled={disabled}
          style={{ width: "100%", ...(schema.style as React.CSSProperties) }}
          {...rest}
        />
      </FormFieldWrapper>
    );
  }

  // 密码输入
  if (type === "password") {
    return (
      <FormFieldWrapper label={label} name={name} required={required}>
        <Input.Password
          placeholder={placeholder}
          disabled={disabled}
          style={schema.style as React.CSSProperties}
          {...rest}
        />
      </FormFieldWrapper>
    );
  }

  return (
    <FormFieldWrapper label={label} name={name} required={required}>
      <Input
        placeholder={placeholder}
        disabled={disabled}
        style={schema.style as React.CSSProperties}
        {...rest}
      />
    </FormFieldWrapper>
  );
};

/**
 * 下拉选择组件，基于 Ant Design Select。
 */
export const SelectField: React.FC<DynamicUIProps> = ({ schema }) => {
  const {
    label,
    name,
    placeholder,
    required,
    disabled,
    options = [],
    mode,
    ...rest
  } = schema.props as {
    label?: string;
    name?: string;
    placeholder?: string;
    required?: boolean;
    disabled?: boolean;
    options?: { label: string; value: string | number }[];
    mode?: "multiple" | "tags";
    [key: string]: unknown;
  };

  return (
    <FormFieldWrapper label={label} name={name} required={required}>
      <Select
        placeholder={placeholder}
        disabled={disabled}
        mode={mode}
        options={options}
        style={schema.style as React.CSSProperties}
        {...rest}
      />
    </FormFieldWrapper>
  );
};

/**
 * 日期选择器，基于 Ant Design DatePicker。
 */
export const DatePickerField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { label, name, placeholder, required, disabled, ...rest } = schema.props as {
    label?: string;
    name?: string;
    placeholder?: string;
    required?: boolean;
    disabled?: boolean;
    [key: string]: unknown;
  };

  return (
    <FormFieldWrapper label={label} name={name} required={required}>
      <DatePicker
        placeholder={placeholder}
        disabled={disabled}
        style={{ width: "100%", ...(schema.style as React.CSSProperties) }}
        {...rest}
      />
    </FormFieldWrapper>
  );
};

/**
 * 开关组件，基于 Ant Design Switch。
 */
export const SwitchField: React.FC<DynamicUIProps> = ({ schema }) => {
  const { label, name, required, disabled, ...rest } = schema.props as {
    label?: string;
    name?: string;
    required?: boolean;
    disabled?: boolean;
    [key: string]: unknown;
  };

  return (
    <FormFieldWrapper
      label={label}
      name={name}
      required={required}
      valuePropName="checked"
    >
      <Switch disabled={disabled} {...rest} />
    </FormFieldWrapper>
  );
};

/**
 * 文本域组件，基于 Ant Design Input.TextArea。
 */
export const TextareaField: React.FC<DynamicUIProps> = ({ schema }) => {
  const {
    label,
    name,
    placeholder,
    required,
    disabled,
    rows = 4,
    ...rest
  } = schema.props as {
    label?: string;
    name?: string;
    placeholder?: string;
    required?: boolean;
    disabled?: boolean;
    rows?: number;
    [key: string]: unknown;
  };

  return (
    <FormFieldWrapper label={label} name={name} required={required}>
      <Input.TextArea
        placeholder={placeholder}
        disabled={disabled}
        rows={rows}
        style={schema.style as React.CSSProperties}
        {...rest}
      />
    </FormFieldWrapper>
  );
};

// ── 辅助组件 ──

function FormFieldWrapper({
  label,
  name,
  required,
  valuePropName,
  children,
}: {
  label?: string;
  name?: string;
  required?: boolean;
  valuePropName?: string;
  children: React.ReactNode;
}) {
  // 当在 FormRenderer 内部时使用 Form.Item，否则直接渲染
  // 使用函数调用来代替 try/catch 包裹 JSX
  const renderFormItem = () => {
    try {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const { Form } = require("antd");
      return (
        <Form.Item
          label={label}
          name={name}
          rules={required ? [{ required: true, message: `请输入${label}` }] : []}
          valuePropName={valuePropName}
        >
          {children}
        </Form.Item>
      );
    } catch {
      return null;
    }
  };

  const formItem = renderFormItem();
  if (formItem) {
    return formItem;
  }

  return (
    <div className="mb-4">
      {label
        ? (
          <label className="block mb-1 text-sm font-medium">
            {label}
            {required ? <span className="text-red-500 ml-0.5">*</span> : null}
          </label>
        )
        : null}
      {children}
    </div>
  );
}

export default InputField;
