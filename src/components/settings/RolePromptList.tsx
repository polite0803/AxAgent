/**
 * 股票分析角色 system_prompt 列表编辑器。
 * 从 agent_roles 表加载，支持展开编辑 + 保存。
 */
import { invoke } from "@/lib/invoke";
import { App, Button, Input, Spin, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface RoleRow {
  id: string;
  name: string;
  description?: string | null;
  system_prompt: string;
  source: string;
}

export function RolePromptList() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [roles, setRoles] = useState<RoleRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return invoke<RoleRow[]>("list_agent_roles", { source: "stock-analysis" });
    })
      .then((rows) => {
        if (cancelled) { return; }
        setRoles(Array.isArray(rows) ? rows : []);
      })
      .catch(() => {
        if (!cancelled) { setRoles([]); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const expand = (role: RoleRow) => {
    setExpandedId(role.id);
    setEditText(role.system_prompt);
  };

  const save = async () => {
    if (!expandedId) { return; }
    setSaving(true);
    try {
      await invoke("update_agent_role", { id: expandedId, systemPrompt: editText });
      message.success(t("common.saved"));
      setRoles((prev) => prev.map((r) => (r.id === expandedId ? { ...r, system_prompt: editText } : r)));
      setExpandedId(null);
    } catch {
      message.error(t("common.saveFailed"));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 24, textAlign: "center" }}>
        <Spin />
      </div>
    );
  }

  return (
    <div>
      {roles.length === 0 && (
        <div style={{ color: "var(--muted)", padding: 12, textAlign: "center" }}>
          {t("stockAnalysis.settings.noRoles")}
        </div>
      )}
      {roles.map((role) => (
        <div
          key={role.id}
          style={{
            borderBottom: "1px solid var(--border)",
            padding: "8px 0",
          }}
        >
          <div
            style={{ display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer" }}
            onClick={() => (expandedId === role.id ? setExpandedId(null) : expand(role))}
          >
            <div style={{ flex: 1, minWidth: 0 }}>
              <span style={{ fontSize: 13, fontWeight: 500 }}>{role.name}</span>
              <span style={{ fontSize: 11, color: "var(--muted)", marginLeft: 8 }}>
                {role.description || role.id}
              </span>
            </div>
            <Tag color="blue" style={{ fontSize: 10 }}>{role.source}</Tag>
          </div>
          {expandedId === role.id && (
            <div style={{ marginTop: 8 }}>
              <Input.TextArea
                value={editText}
                onChange={(e) => setEditText(e.target.value)}
                rows={10}
                style={{ fontFamily: "monospace", fontSize: 12 }}
              />
              <div style={{ display: "flex", gap: 8, marginTop: 6, justifyContent: "flex-end" }}>
                <Button size="small" onClick={() => setExpandedId(null)}>{t("common.cancel")}</Button>
                <Button size="small" type="primary" loading={saving} onClick={save}>{t("common.save")}</Button>
              </div>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
