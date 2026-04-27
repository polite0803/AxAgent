import { Select } from "antd";
import React from "react";

interface EntryTypeSelectorProps {
  value?: string;
  onChange?: (value: string) => void;
}

const ENTRY_TYPE_OPTIONS = [
  { label: "内置函数 (builtin)", value: "builtin" },
  { label: "MCP工具 (mcp)", value: "mcp" },
  { label: "本地工具 (local)", value: "local" },
  { label: "插件工具 (plugin)", value: "plugin" },
];

export const EntryTypeSelector: React.FC<EntryTypeSelectorProps> = ({ value, onChange }) => {
  return (
    <Select
      value={value}
      onChange={onChange}
      options={ENTRY_TYPE_OPTIONS}
      placeholder="选择执行入口类型"
      style={{ width: "100%" }}
    />
  );
};
