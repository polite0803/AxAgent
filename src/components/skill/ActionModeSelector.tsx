import type { AgenticAction, DeclarativeActionType, SkillCommandAction } from "@/types";
import { Form, Input, Radio, Select, Switch } from "antd";
import { useTranslation } from "react-i18next";

interface ActionModeSelectorProps {
  value: SkillCommandAction;
  availableHandlers: string[];
  onChange: (action: SkillCommandAction) => void;
}

export function ActionModeSelector({ value, availableHandlers, onChange }: ActionModeSelectorProps) {
  const { t } = useTranslation();
  const mode = value.mode;

  return (
    <div style={{ border: "1px solid var(--color-border)", borderRadius: 8, padding: 12 }}>
      <Form.Item label={t("skillEditor.actionMode")} style={{ marginBottom: 12 }}>
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
          <Radio.Button value="declarative">{t("skillEditor.declarative")}</Radio.Button>
          <Radio.Button value="agentic">{t("skillEditor.agentic")}</Radio.Button>
        </Radio.Group>
      </Form.Item>

      {mode === "declarative" && (
        <DeclarativeEditor
          action={(value as { mode: "declarative"; action: DeclarativeActionType }).action}
          availableHandlers={availableHandlers}
          onChange={(action) => onChange({ mode: "declarative", action })}
          t={t}
        />
      )}

      {mode === "agentic" && <AgenticEditor action={value as AgenticAction} onChange={(a) => onChange(a)} t={t} />}
    </div>
  );
}

function DeclarativeEditor({ action, availableHandlers, onChange, t }: {
  action: DeclarativeActionType;
  availableHandlers: string[];
  onChange: (a: DeclarativeActionType) => void;
  t: (key: string) => string;
}) {
  const currentType = action.type;

  return (
    <div>
      <Form.Item label={t("skillEditor.actionType")} style={{ marginBottom: 8 }}>
        <Select
          size="small"
          value={currentType}
          options={[
            { value: "invoke", label: t("skillEditor.invoke") },
            { value: "navigate", label: t("skillEditor.navigate") },
            { value: "emit", label: t("skillEditor.emit") },
            { value: "store", label: t("skillEditor.store") },
            { value: "function", label: t("skillEditor.func") },
            { value: "handler", label: t("skillEditor.handler") },
            { value: "chain", label: t("skillEditor.actionChain") },
          ]}
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
          <Form.Item label={t("skillEditor.invokeCmd")} style={{ marginBottom: 8 }}>
            <Input
              size="small"
              value={action.command || ""}
              onChange={(e) => onChange({ ...action, command: e.target.value })}
            />
          </Form.Item>
          <Form.Item label={t("skillEditor.invokeArgs")} style={{ marginBottom: 8 }}>
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
        <Form.Item label={t("skillEditor.navigatePath")} style={{ marginBottom: 8 }}>
          <Input
            size="small"
            value={action.path || "/"}
            onChange={(e) => onChange({ ...action, path: e.target.value })}
          />
        </Form.Item>
      )}

      {currentType === "emit" && (
        <>
          <Form.Item label={t("skillEditor.emitEvent")} style={{ marginBottom: 8 }}>
            <Input
              size="small"
              value={action.event || ""}
              onChange={(e) => onChange({ ...action, event: e.target.value })}
            />
          </Form.Item>
          <Form.Item label={t("skillEditor.emitPayload")} style={{ marginBottom: 0 }}>
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
          <Form.Item label={t("skillEditor.storeName")} style={{ marginBottom: 8 }}>
            <Input
              size="small"
              value={action.storeName || ""}
              onChange={(e) => onChange({ ...action, storeName: e.target.value })}
            />
          </Form.Item>
          <Form.Item label={t("skillEditor.storeOp")} style={{ marginBottom: 8 }}>
            <Select
              size="small"
              value={action.operation}
              options={[{ value: "get", label: "get" }, { value: "set", label: "set" }, {
                value: "update",
                label: "update",
              }]}
              onChange={(v) => onChange({ ...action, operation: v })}
              style={{ width: 120 }}
            />
          </Form.Item>
          <Form.Item label={t("skillEditor.storePayload")} style={{ marginBottom: 0 }}>
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
        <Form.Item label={t("skillEditor.funcName")} style={{ marginBottom: 0 }}>
          <Input
            size="small"
            value={action.name || ""}
            onChange={(e) => onChange({ ...action, name: e.target.value })}
          />
        </Form.Item>
      )}

      {currentType === "handler" && (
        <Form.Item label={t("skillEditor.handlerName")} style={{ marginBottom: 0 }}>
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

function AgenticEditor({ action, onChange, t }: {
  action: AgenticAction;
  onChange: (a: AgenticAction) => void;
  t: (key: string) => string;
}) {
  return (
    <div>
      <Form.Item label={t("skillEditor.agenticPrompt")} style={{ marginBottom: 8 }}>
        <Input.TextArea
          size="small"
          rows={2}
          value={action.prompt || ""}
          onChange={(e) => onChange({ ...action, prompt: e.target.value })}
          placeholder={t("skillEditor.agenticPromptHint")}
        />
      </Form.Item>
      <Form.Item label={t("skillEditor.agenticSkill")} style={{ marginBottom: 8 }}>
        <Input
          size="small"
          value={action.skillName || ""}
          onChange={(e) => onChange({ ...action, skillName: e.target.value })}
        />
      </Form.Item>
      <Form.Item label={t("skillEditor.agenticOptions")} style={{ marginBottom: 0 }}>
        <div style={{ display: "flex", gap: 16 }}>
          <span style={{ fontSize: 12 }}>
            <Switch
              size="small"
              checked={action.context?.includeConversation ?? true}
              onChange={(v) => onChange({ ...action, context: { ...action.context, includeConversation: v } })}
            />{" "}
            {t("skillEditor.includeConv")}
          </span>
          <span style={{ fontSize: 12 }}>
            <Switch
              size="small"
              checked={action.context?.includeFiles ?? false}
              onChange={(v) => onChange({ ...action, context: { ...action.context, includeFiles: v } })}
            />{" "}
            {t("skillEditor.includeFiles")}
          </span>
          <span style={{ fontSize: 12 }}>
            <Switch
              size="small"
              checked={action.context?.includeSelection ?? false}
              onChange={(v) => onChange({ ...action, context: { ...action.context, includeSelection: v } })}
            />{" "}
            {t("skillEditor.includeSelection")}
          </span>
        </div>
      </Form.Item>
    </div>
  );
}
