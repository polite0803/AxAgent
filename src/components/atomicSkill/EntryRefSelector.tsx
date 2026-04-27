import { invoke } from "@tauri-apps/api/core";
import { Input, Select, Spin } from "antd";
import React, { useEffect, useState } from "react";

interface EntryRefSelectorProps {
  entryType?: string;
  value?: string;
  onChange?: (value: string) => void;
}

interface ToolOption {
  name: string;
  description?: string;
}

export const EntryRefSelector: React.FC<EntryRefSelectorProps> = ({ entryType, value, onChange }) => {
  const [options, setOptions] = useState<ToolOption[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!entryType) {
      setOptions([]);
      return;
    }

    setLoading(true);
    (async () => {
      try {
        if (entryType === "builtin" || entryType === "local") {
          // Fetch local/builtin tools from the registry
          const tools: ToolOption[] = await invoke("list_local_tools");
          setOptions(tools);
        } else if (entryType === "mcp") {
          // Fetch MCP tools
          const tools: ToolOption[] = await invoke("list_mcp_tools");
          setOptions(tools);
        } else if (entryType === "plugin") {
          // Fetch plugin tools
          const tools: ToolOption[] = await invoke("list_plugin_tools");
          setOptions(tools);
        } else {
          setOptions([]);
        }
      } catch {
        setOptions([]);
      } finally {
        setLoading(false);
      }
    })();
  }, [entryType]);

  if (loading) { return <Spin size="small" />; }

  if (!entryType) {
    return <Input placeholder="请先选择入口类型" disabled />;
  }

  return (
    <Select
      value={value}
      onChange={onChange}
      options={options.map((t) => ({
        value: t.name,
        label: t.description ? `${t.name} - ${t.description}` : t.name,
      }))}
      placeholder={`选择${entryType}工具...`}
      style={{ width: "100%" }}
      showSearch
      allowClear
    />
  );
};
