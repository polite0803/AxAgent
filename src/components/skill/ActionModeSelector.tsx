import type { AgenticAction, DeclarativeActionType, SkillCommandAction } from "@/types";
import { Form, Input, Radio, Select, Switch } from "antd";

interface ActionModeSelectorProps {
  value: SkillCommandAction;
  availableHandlers: string[];
  onChange: (action: SkillCommandAction) => void;
}

const DECLARATIVE_TYPE_OPTIONS = [
  { value: "invoke", label: "调用后端 (invoke)" },
  { value: "navigate", label: "页面跳转 (navigate)" },
  { value: "emit", label: "发送事件 (emit)" },
  { value: "store", label: "读写 Store (store)" },
  { value: "function", label: "自定义函数 (function)" },
  { value: "handler", label: "引用 Handler (handler)" },
  { value: "chain", label: "嵌套子链 (chain)" },
];

export function ActionModeSelector({ value, availableHandlers, onChange }: ActionModeSelectorProps) {
  const mode = value.mode;

  return (
    <div style={{ border: "1px solid var(--color-border)", borderRadius: 8, padding: 12 }}>
      <Form.Item label="执行模式" style={{ marginBottom: 12 }}>
        <Radio.Group
          value={mode}
          optionType="button"
          size="small"
          onChange={(e) => {
            const newMode = e.target.value as "declarative" | "agentic";
            if (newMode === "declarative") {
              onChange({ mode: "declarative", action: { type: "invoke", command: "" } });
            } else {
              onChange({ mode: "agentic", prompt: "", skillName: "" });
            }
          }}
        >
          <Radio.Button value="declarative">声明式（毫秒响应）</Radio.Button>
          <Radio.Button value="agentic">Agent 智能（LLM 驱动）</Radio.Button>
        </Radio.Group>
      </Form.Item>

      {mode === "declarative" && (
        <DeclarativeEditor
          action={(value as { mode: "declarative"; action: DeclarativeActionType }).action}
          availableHandlers={availableHandlers}
          onChange={(action) => onChange({ mode: "declarative", action })}
        />
      )}

      {mode === "agentic" && (
        <AgenticEditor
          action={value as AgenticAction}
          onChange={(a) => onChange(a)}
        />
      )}
    </div>
  );
}

function DeclarativeEditor({ action, availableHandlers, onChange }: {
  action: DeclarativeActionType;
  availableHandlers: string[];
  onChange: (a: DeclarativeActionType) => void;
}) {
  const currentType = action.type;

  return (
    <div>
      <Form.Item label="Action 类型" style={{ marginBottom: 8 }}>
        <Select
          size="small"
          value={currentType}
          options={DECLARATIVE_TYPE_OPTIONS}
          onChange={(v) => {
            const defaults: Record<string, DeclarativeActionType> = {
              invoke: { type: "invoke", command: "" },
              navigate: { type: "navigate", path: "/" },
              emit: { type: "emit", event: "", payload: {} },
              store: { type: "store", operation: "get", storeName: "" },
              function: { type: "function", name: "" },
              handler: { type: "handler", name: "" },
              chain: { type: "chain", actions: [] },
            };
            onChange(defaults[v]);
          }}
          style={{ width: 240 }}
        />
      </Form.Item>

      {currentType === "invoke" && (
        <>
          <Form.Item label="Tauri 命令" style={{ marginBottom: 8 }}>
            <Input
              size="small"
              value={action.command || ""}
              onChange={(e) => onChange({ ...action, command: e.target.value })}
            />
          </Form.Item>
          <Form.Item label="参数 (JSON)" style={{ marginBottom: 8 }}>
            <Input.TextArea
              size="small"
              rows={2}
              value={JSON.stringify((action as { args?: Record<string, unknown> }).args || {}, null, 2)}
              onChange={(e) => {
                try {
                  onChange({ ...action, args: JSON.parse(e.target.value) });
                } catch { /* ignore */ }
              }}
              style={{ fontFamily: "monospace", fontSize: 11 }}
            />
          </Form.Item>
        </>
      )}

      {currentType === "navigate" && (
        <Form.Item label="目标路径" style={{ marginBottom: 8 }}>
          <Input
            size="small"
            value={action.path || "/"}
            onChange={(e) => onChange({ ...action, path: e.target.value })}
          />
        </Form.Item>
      )}

      {currentType === "emit" && (
        <>
          <Form.Item label="事件名" style={{ marginBottom: 8 }}>
            <Input
              size="small"
              value={action.event || ""}
              onChange={(e) => onChange({ ...action, event: e.target.value })}
            />
          </Form.Item>
          <Form.Item label="载荷 (JSON)" style={{ marginBottom: 0 }}>
            <Input.TextArea
              size="small"
              rows={2}
              value={JSON.stringify((action as { payload?: unknown }).payload || {}, null, 2)}
              onChange={(e) => {
                try {
                  onChange({ ...action, payload: JSON.parse(e.target.value) });
                } catch { /* ignore */ }
              }}
              style={{ fontFamily: "monospace", fontSize: 11 }}
            />
          </Form.Item>
        </>
      )}

      {currentType === "store" && (
        <>
          <Form.Item label="Store 名称" style={{ marginBottom: 8 }}>
            <Input
              size="small"
              value={action.storeName || ""}
              onChange={(e) => onChange({ ...action, storeName: e.target.value })}
            />
          </Form.Item>
          <Form.Item label="操作" style={{ marginBottom: 8 }}>
            <Select
              size="small"
              value={action.operation}
              options={[
                { value: "get", label: "get" },
                { value: "set", label: "set" },
                { value: "update", label: "update" },
              ]}
              onChange={(v) => onChange({ ...action, operation: v })}
              style={{ width: 120 }}
            />
          </Form.Item>
          <Form.Item label="载荷 (JSON)" style={{ marginBottom: 0 }}>
            <Input.TextArea
              size="small"
              rows={2}
              value={JSON.stringify((action as { payload?: unknown }).payload || {}, null, 2)}
              onChange={(e) => {
                try {
                  onChange({ ...action, payload: JSON.parse(e.target.value) });
                } catch { /* ignore */ }
              }}
              style={{ fontFamily: "monospace", fontSize: 11 }}
            />
          </Form.Item>
        </>
      )}

      {currentType === "function" && (
        <Form.Item label="函数名" style={{ marginBottom: 0 }}>
          <Input
            size="small"
            value={action.name || ""}
            onChange={(e) => onChange({ ...action, name: e.target.value })}
          />
        </Form.Item>
      )}

      {currentType === "handler" && (
        <Form.Item label="Handler 名称" style={{ marginBottom: 0 }}>
          <Select
            size="small"
            showSearch
            value={action.name || undefined}
            options={availableHandlers.map((h) => ({ value: h, label: h }))}
            onChange={(v) => onChange({ ...action, name: v })}
            style={{ width: 240 }}
          />
        </Form.Item>
      )}
    </div>
  );
}

function AgenticEditor({ action, onChange }: {
  action: AgenticAction;
  onChange: (a: AgenticAction) => void;
}) {
  return (
    <div>
      <Form.Item label="用户意图 Prompt" style={{ marginBottom: 8 }}>
        <Input.TextArea
          size="small"
          rows={2}
          value={action.prompt || ""}
          onChange={(e) => onChange({ ...action, prompt: e.target.value })}
          placeholder="描述需要 Agent 执行的操作"
        />
      </Form.Item>
      <Form.Item label="Skill 名称（可选）" style={{ marginBottom: 8 }}>
        <Input
          size="small"
          value={action.skillName || ""}
          onChange={(e) => onChange({ ...action, skillName: e.target.value })}
        />
      </Form.Item>
      <Form.Item label="附加选项" style={{ marginBottom: 0 }}>
        <div style={{ display: "flex", gap: 16 }}>
          <span style={{ fontSize: 12 }}>
            <Switch
              size="small"
              checked={action.context?.includeConversation ?? true}
              onChange={(v) => onChange({ ...action, context: { ...action.context, includeConversation: v } })}
            />{" "}
            包含会话
          </span>
          <span style={{ fontSize: 12 }}>
            <Switch
              size="small"
              checked={action.context?.includeFiles ?? false}
              onChange={(v) => onChange({ ...action, context: { ...action.context, includeFiles: v } })}
            />{" "}
            包含文件
          </span>
          <span style={{ fontSize: 12 }}>
            <Switch
              size="small"
              checked={action.context?.includeSelection ?? false}
              onChange={(v) => onChange({ ...action, context: { ...action.context, includeSelection: v } })}
            />{" "}
            包含选区
          </span>
        </div>
      </Form.Item>
    </div>
  );
}
