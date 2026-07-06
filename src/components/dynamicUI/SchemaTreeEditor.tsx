// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicComponentType, UISchema } from "@/types";
import { COMPONENT_REQUIRED_PROPS, VALID_DYNAMIC_COMPONENT_TYPES } from "@/types";
import { ArrowDownOutlined, ArrowUpOutlined, DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Divider, Empty, Input, Select, Space, Tag, Tree } from "antd";
import type { DataNode } from "antd/es/tree";
import { useCallback, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface SchemaTreeEditorProps {
  schema: UISchema;
  onChange: (newSchema: UISchema) => void;
}

const COMPONENT_TYPE_OPTIONS = [...VALID_DYNAMIC_COMPONENT_TYPES].sort().map(
  (t) => ({ label: t, value: t }),
);

/** 递归深拷贝 UISchema，避免直接修改原对象 */
function cloneSchema(node: UISchema): UISchema {
  return {
    ...node,
    props: { ...node.props },
    children: node.children ? node.children.map(cloneSchema) : undefined,
    events: node.events ? node.events.map((e) => ({ ...e, actions: [...e.actions] })) : undefined,
    dataSource: node.dataSource ? { ...node.dataSource, config: { ...node.dataSource.config } } : undefined,
    conditionalDisplay: node.conditionalDisplay
      ? (Array.isArray(node.conditionalDisplay)
        ? [...node.conditionalDisplay]
        : { ...node.conditionalDisplay, rules: [...node.conditionalDisplay.rules] })
      : undefined,
    style: node.style ? { ...node.style } : undefined,
  };
}

/** 通过路径查找节点（路径为 id 数组） */
function findNodeByPath(root: UISchema, path: string[]): UISchema | null {
  if (path.length === 0) { return root; }
  if (!root.children) { return null; }
  for (const child of root.children) {
    if (child.id === path[0]) {
      return findNodeByPath(child, path.slice(1));
    }
  }
  return null;
}

/** 生成唯一 id */
let _idCounter = 0;
function genId(prefix: string): string {
  _idCounter += 1;
  return `${prefix}-${Date.now()}-${_idCounter}`;
}

/** 根据 i18n t 函数生成组件默认 props */
function getDefaultProps(t: (key: string) => string): Partial<Record<DynamicComponentType, Record<string, unknown>>> {
  return {
    Input: {
      name: "field",
      label: t("dynamicUIManager.defaults.inputLabel"),
      placeholder: t("dynamicUIManager.defaults.inputPlaceholder"),
    },
    Number: { name: "number", label: t("dynamicUIManager.defaults.numberLabel") },
    Select: { name: "select", label: t("dynamicUIManager.defaults.selectLabel"), options: [] },
    DatePicker: { name: "date", label: t("dynamicUIManager.defaults.dateLabel") },
    Switch: { name: "enabled", label: t("dynamicUIManager.defaults.switchLabel") },
    Checkbox: { name: "checked", label: t("dynamicUIManager.defaults.checkboxLabel") },
    Radio: { name: "option", label: t("dynamicUIManager.defaults.radioLabel"), options: [] },
    Textarea: {
      name: "text",
      label: t("dynamicUIManager.defaults.textareaLabel"),
      placeholder: t("dynamicUIManager.defaults.inputPlaceholder"),
    },
    Button: { text: t("dynamicUIManager.defaults.buttonText") },
    Text: { content: t("dynamicUIManager.defaults.textContent") },
    Form: { layout: "vertical", submitText: t("dynamicUIManager.defaults.formSubmit") },
    Card: { title: t("dynamicUIManager.defaults.cardTitle") },
    Container: {},
    Row: {},
    Column: {},
    Grid: { columns: 3 },
    Tabs: {},
    Accordion: {},
    Table: { columns: [] },
    Chart: { chartType: "line" },
    List: {},
    Dashboard: { items: [] },
    CodeEditor: {},
    FilePreview: {},
    Markdown: { content: "" },
    Image: { src: "" },
    Divider: {},
    Progress: { percent: 0 },
    Tag: { text: t("dynamicUIManager.defaults.tagText") },
    Tree: { treeData: [] },
    Timeline: { items: [] },
  };
}

export function SchemaTreeEditor({ schema, onChange }: SchemaTreeEditorProps) {
  const { t } = useTranslation();
  const defaultPropsMap = useMemo(() => getDefaultProps(t), [t]);

  // 用 id 路径定位选中节点
  const [selectedPath, setSelectedPath] = useState<string[]>([]);
  const [addChildType, setAddChildType] = useState<DynamicComponentType | null>(null);

  const root = useMemo(() => cloneSchema(schema), [schema]);

  // 递归构建 Tree DataNode（用 function 声明而非 const + useCallback，
  // 否则递归引用 buildTreeData 时会触发 react-hooks/immutability 违规）
  function buildTreeData(node: UISchema, parentPath: string[] = []): DataNode {
    const currentPath = [...parentPath, node.id];
    return {
      key: currentPath.join("."),
      title: (
        <span className="flex items-center gap-1">
          <Tag color="geekblue" className="text-[10px] leading-none px-1 py-0">
            {node.type}
          </Tag>
          <span className="text-xs truncate max-w-[140px]">{node.id}</span>
        </span>
      ),
      children: node.children?.map((c) => buildTreeData(c, currentPath)),
      isLeaf: !node.children || node.children.length === 0,
    };
  }

  const buildTreeDataRef = useRef(buildTreeData);
  buildTreeDataRef.current = buildTreeData;

  const treeData = useMemo(() => [buildTreeDataRef.current(root)], [root]);

  // 选中的节点
  const selectedNode = useMemo(() => {
    if (selectedPath.length === 0) { return root; }
    return findNodeByPath(root, selectedPath);
  }, [root, selectedPath]);

  const selectedKeys = useMemo(() => {
    if (selectedPath.length === 0) { return [root.id]; }
    return [selectedPath.join(".")];
  }, [root.id, selectedPath]);

  const handleTreeSelect = useCallback(
    (keys: React.Key[]) => {
      if (keys.length === 0) { return; }
      const key = String(keys[0]);
      const path = key === root.id ? [] : key.split(".");
      setSelectedPath(path);
    },
    [root.id],
  );

  // 修改选中节点
  const updateSelectedNode = useCallback(
    (updater: (node: UISchema) => void) => {
      const cloned = cloneSchema(root);
      let target: UISchema;
      if (selectedPath.length === 0) {
        target = cloned;
      } else {
        target = findNodeByPath(cloned, selectedPath)!;
        if (!target) { return; }
      }
      updater(target);
      onChange(cloned);
    },
    [root, selectedPath, onChange],
  );

  // Props 编辑
  const propEntries = useMemo(() => {
    if (!selectedNode) { return []; }
    return Object.entries(selectedNode.props).map(([key, value]) => ({
      key,
      value: typeof value === "string" ? value : JSON.stringify(value),
    }));
  }, [selectedNode]);

  const handlePropChange = useCallback(
    (index: number, field: "key" | "value", newVal: string) => {
      updateSelectedNode((node) => {
        const entries = Object.entries(node.props);
        if (field === "key") {
          entries[index][0] = newVal;
        } else {
          entries[index][1] = parsePropValue(newVal);
        }
        node.props = Object.fromEntries(entries);
      });
    },
    [updateSelectedNode],
  );

  const handleAddProp = useCallback(() => {
    updateSelectedNode((node) => {
      node.props = { ...node.props, "": "" };
    });
  }, [updateSelectedNode]);

  const handleRemoveProp = useCallback(
    (index: number) => {
      updateSelectedNode((node) => {
        const entries = Object.entries(node.props);
        entries.splice(index, 1);
        node.props = Object.fromEntries(entries);
      });
    },
    [updateSelectedNode],
  );

  // 类型变更
  const handleTypeChange = useCallback(
    (newType: DynamicComponentType) => {
      updateSelectedNode((node) => {
        node.type = newType;
        // 保留已有 props 中合法项，叠加新类型的默认 props
        node.props = {
          ...(defaultPropsMap[newType] ?? {}),
          ...node.props,
        };
      });
    },
    [updateSelectedNode, defaultPropsMap],
  );

  // Children 管理
  const handleRemoveChild = useCallback(
    (childIndex: number) => {
      updateSelectedNode((node) => {
        if (node.children) { node.children = node.children.filter((_, i) => i !== childIndex); }
      });
    },
    [updateSelectedNode],
  );

  const handleMoveChild = useCallback(
    (childIndex: number, direction: "up" | "down") => {
      updateSelectedNode((node) => {
        if (!node.children) { return; }
        const arr = [...node.children];
        const targetIdx = direction === "up" ? childIndex - 1 : childIndex + 1;
        if (targetIdx < 0 || targetIdx >= arr.length) { return; }
        [arr[childIndex], arr[targetIdx]] = [arr[targetIdx], arr[childIndex]];
        node.children = arr;
      });
    },
    [updateSelectedNode],
  );

  const handleAddChild = useCallback(() => {
    if (!addChildType) { return; }
    updateSelectedNode((node) => {
      const newChild: UISchema = {
        version: "1.0",
        id: genId(addChildType.toLowerCase()),
        type: addChildType,
        props: defaultPropsMap[addChildType] ?? {},
      };
      node.children = [...(node.children ?? []), newChild];
    });
    setAddChildType(null);
  }, [addChildType, updateSelectedNode, defaultPropsMap]);

  const requiredProps = selectedNode
    ? COMPONENT_REQUIRED_PROPS[selectedNode.type as DynamicComponentType] ?? []
    : [];

  return (
    <div className="flex h-[500px] border rounded-lg overflow-hidden">
      {/* 左侧树面板 */}
      <div className="w-[280px] border-r overflow-auto p-2 bg-gray-50 dark:bg-gray-800 flex-shrink-0">
        <Tree
          treeData={treeData}
          selectedKeys={selectedKeys}
          onSelect={handleTreeSelect}
          defaultExpandAll
          blockNode
          showIcon={false}
        />
      </div>

      {/* 右侧属性面板 */}
      <div className="flex-1 overflow-auto p-4">
        {!selectedNode ? <Empty description={t("dynamicUIManager.selectToPreview")} /> : (
          <div className="space-y-4">
            {/* 基本信息 */}
            <div>
              <label className="block text-xs text-gray-500 mb-1">
                {t("dynamicUIManager.componentType")}
              </label>
              <Select
                value={selectedNode.type}
                onChange={handleTypeChange}
                options={COMPONENT_TYPE_OPTIONS}
                className="w-full"
                size="small"
                disabled={selectedPath.length === 0 && selectedNode.id === root.id}
              />
            </div>

            <div className="flex gap-2">
              <div className="flex-1">
                <label className="block text-xs text-gray-500 mb-1">ID</label>
                <Input
                  size="small"
                  value={selectedNode.id}
                  disabled
                />
              </div>
              <div className="flex-1">
                <label className="block text-xs text-gray-500 mb-1">Version</label>
                <Input
                  size="small"
                  value={selectedNode.version}
                  disabled
                />
              </div>
            </div>

            <Divider className="my-2" />

            {/* Props 编辑器 */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="text-xs text-gray-500">Props</label>
                <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={handleAddProp}>
                  {t("dynamicUIManager.addProp")}
                </Button>
              </div>

              {requiredProps.length > 0 && (
                <div className="mb-2">
                  <span className="text-[11px] text-orange-500">
                    {t("dynamicUIManager.requiredProps")}: {requiredProps.join(", ")}
                  </span>
                </div>
              )}

              <div className="space-y-1 max-h-[200px] overflow-auto">
                {propEntries.length === 0 && (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description={t("dynamicUIManager.noProps")}
                    className="my-2"
                  />
                )}
                {propEntries.map((entry, idx) => (
                  <div key={idx} className="flex gap-1">
                    <Input
                      size="small"
                      className="w-[40%] font-mono text-xs"
                      placeholder="key"
                      value={entry.key}
                      onChange={(e) => handlePropChange(idx, "key", e.target.value)}
                      status={requiredProps.includes(entry.key) && !entry.value ? "warning" : undefined}
                    />
                    <Input
                      size="small"
                      className="flex-1 font-mono text-xs"
                      placeholder="value"
                      value={entry.value}
                      onChange={(e) => handlePropChange(idx, "value", e.target.value)}
                    />
                    <Button
                      size="small"
                      type="text"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={() => handleRemoveProp(idx)}
                    />
                  </div>
                ))}
              </div>
            </div>

            <Divider className="my-2" />

            {/* Children 管理 */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="text-xs text-gray-500">
                  Children ({selectedNode.children?.length ?? 0})
                </label>
                <Space size={4}>
                  <Select
                    size="small"
                    placeholder={t("dynamicUIManager.addChild")}
                    value={addChildType ?? undefined}
                    onChange={(v) => setAddChildType(v as DynamicComponentType)}
                    options={COMPONENT_TYPE_OPTIONS}
                    style={{ width: 140 }}
                  />
                  <Button
                    size="small"
                    type="dashed"
                    icon={<PlusOutlined />}
                    onClick={handleAddChild}
                    disabled={!addChildType}
                  />
                </Space>
              </div>

              <div className="space-y-1 max-h-[150px] overflow-auto">
                {(!selectedNode.children || selectedNode.children.length === 0) && (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description={t("dynamicUIManager.noChildren")}
                    className="my-2"
                  />
                )}
                {selectedNode.children?.map((child, idx) => (
                  <div
                    key={child.id}
                    className="flex items-center gap-1 bg-gray-50 dark:bg-gray-700 rounded px-2 py-1"
                  >
                    <Tag color="geekblue" className="text-[10px] leading-none px-1 py-0">
                      {child.type}
                    </Tag>
                    <span className="text-xs truncate flex-1">{child.id}</span>
                    <Button
                      size="small"
                      type="text"
                      icon={<ArrowUpOutlined />}
                      disabled={idx === 0}
                      onClick={() => handleMoveChild(idx, "up")}
                    />
                    <Button
                      size="small"
                      type="text"
                      icon={<ArrowDownOutlined />}
                      disabled={idx === (selectedNode.children?.length ?? 0) - 1}
                      onClick={() => handleMoveChild(idx, "down")}
                    />
                    <Button
                      size="small"
                      type="text"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={() => handleRemoveChild(idx)}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/** 尝试将字符串 value 解析为合适的类型 */
function parsePropValue(v: string): unknown {
  if (v === "true") { return true; }
  if (v === "false") { return false; }
  const num = Number(v);
  if (!Number.isNaN(num) && v.trim() !== "") { return num; }
  if ((v.startsWith("{") && v.endsWith("}")) || (v.startsWith("[") && v.endsWith("]"))) {
    try {
      return JSON.parse(v);
    } catch {
      // fall through
    }
  }
  return v;
}
