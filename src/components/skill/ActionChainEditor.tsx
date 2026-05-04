import type { SkillCommandAction } from "@/types";
import { Button, Collapse, Empty, Popconfirm } from "antd";
import { GripVertical, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ActionModeSelector } from "./ActionModeSelector";

interface ActionChainEditorProps {
  actions: SkillCommandAction[];
  availableHandlers: string[];
  onChange: (actions: SkillCommandAction[]) => void;
}

export function ActionChainEditor({ actions, availableHandlers, onChange }: ActionChainEditorProps) {
  const { t } = useTranslation();

  const addAction = () => {
    onChange([...actions, { mode: "declarative", action: { type: "invoke", command: "" } }]);
  };

  const removeAction = (index: number) => {
    onChange(actions.filter((_, i) => i !== index));
  };

  const updateAction = (index: number, action: SkillCommandAction) => {
    onChange(actions.map((a, i) => (i === index ? action : a)));
  };

  const moveAction = (index: number, direction: -1 | 1) => {
    const newIndex = index + direction;
    if (newIndex < 0 || newIndex >= actions.length) { return; }
    const copy = [...actions];
    [copy[index], copy[newIndex]] = [copy[newIndex], copy[index]];
    onChange(copy);
  };

  if (actions.length === 0) {
    return (
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={t("skillEditor.actionEmpty")}
        style={{ margin: "8px 0" }}
      >
        <Button type="dashed" size="small" icon={<Plus size={12} />} onClick={addAction}>
          {t("skillEditor.actionAdd")}
        </Button>
      </Empty>
    );
  }

  const getActionLabel = (action: SkillCommandAction, index: number): string => {
    if (action.mode === "agentic") {
      return `#${index + 1} Agentic: ${action.prompt.slice(0, 30) || "(-)"}`;
    }
    const a = action.action;
    switch (a.type) {
      case "invoke":
        return `#${index + 1} invoke: ${a.command || "(-)"}`;
      case "navigate":
        return `#${index + 1} navigate: ${a.path}`;
      case "emit":
        return `#${index + 1} emit: ${a.event || "(-)"}`;
      case "store":
        return `#${index + 1} store: ${a.storeName}.${a.operation}`;
      case "function":
        return `#${index + 1} function: ${a.name || "(-)"}`;
      case "handler":
        return `#${index + 1} handler: ${a.name || "(-)"}`;
      case "chain":
        return `#${index + 1} chain: ${a.actions?.length || 0}`;
    }
  };

  return (
    <div>
      <Collapse
        size="small"
        items={actions.map((action, index) => ({
          key: String(index),
          label: getActionLabel(action, index),
          extra: (
            <div style={{ display: "flex", gap: 2 }} onClick={(e) => e.stopPropagation()}>
              <Button type="text" size="small" disabled={index === 0} onClick={() => moveAction(index, -1)}>
                <GripVertical size={12} style={{ transform: "rotate(90deg)" }} />
              </Button>
              <Button
                type="text"
                size="small"
                disabled={index === actions.length - 1}
                onClick={() => moveAction(index, 1)}
              >
                <GripVertical size={12} style={{ transform: "rotate(-90deg)" }} />
              </Button>
              <Popconfirm
                title={t("skillEditor.actionDelete")}
                onConfirm={() => removeAction(index)}
                okText={t("common.delete")}
                cancelText={t("common.cancel")}
              >
                <Button type="text" size="small" danger icon={<Trash2 size={12} />} />
              </Popconfirm>
            </div>
          ),
          children: (
            <ActionModeSelector
              value={action}
              availableHandlers={availableHandlers}
              onChange={(updated) => updateAction(index, updated)}
            />
          ),
        }))}
      />
      <div style={{ marginTop: 8 }}>
        <Button type="dashed" size="small" icon={<Plus size={12} />} onClick={addAction}>
          {t("skillEditor.actionAdd")}
        </Button>
      </div>
    </div>
  );
}
